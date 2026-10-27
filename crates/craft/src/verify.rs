// SPDX-License-Identifier: MIT
//! `craft verify [--scope FOLDER] [--dry-run] [--timeout SECONDS] [--only typecheck,lint,build,test]`: deterministic project verification.
//! Detects and runs build, typecheck, lint and tests.
//!
//! Output (format of the `verifier` agent):
//!     VERDICT: PASS|FAIL|UNVERIFIED
//!     RAN: cmd, cmd
//!     FAIL:
//!     - <cmd>: <first useful error line>
//!
//! Exit: 0 PASS, 1 FAIL, 3 UNVERIFIED (nothing to run), 2 bad arguments. Does not edit files, run git or install dependencies.

use crate::args::{self, Opt};
use crate::text;
use pycompat::Re;
use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const STEP_ORDER: [&str; 4] = ["typecheck", "lint", "build", "test"];
const DOTNET_SKIP: [&str; 7] = ["bin", "obj", "node_modules", ".git", ".specs", ".vs", "packages"];
const MAX_DOTNET_DEPTH: usize = 3;
const DOTNET_EXTENSIONS: [&str; 3] = [".sln", ".slnx", ".csproj"];
const DEFAULT_TIMEOUT_SECONDS: i64 = 600;
const LINE_LIMIT: usize = 220;
const USAGE: &str = "verify [-h] [--scope SCOPE] [--dry-run] [--timeout TIMEOUT] [--only ONLY]";
const HELP: &str = "usage: verify [-h] [--scope SCOPE] [--dry-run] [--timeout TIMEOUT] [--only ONLY]

Detects and runs the project's typecheck, lint, build and test; prints PASS, FAIL or UNVERIFIED. Does not edit files, run git or install dependencies.

options:
  -h, --help         show this help message and exit
  --scope SCOPE      folder to verify (default: the current folder); use it in a monorepo
  --dry-run          only print what would run
  --timeout TIMEOUT  seconds allowed per step (default 600)
  --only ONLY        comma-separated steps to run: typecheck,lint,build,test

Exit codes: 0 PASS, 1 FAIL, 3 UNVERIFIED (nothing detected to run), 2 bad arguments.
";

static ERROR_LINE: LazyLock<Re> = LazyLock::new(|| Re::ci(r"(error|failed|fail\b|cannot find|TS\d{4}|CS\d{4}|✖|✗)"));
static TEST_WORD: LazyLock<Re> = LazyLock::new(|| Re::ci(r"(^|[._\- ])(tests?|unittests?|integrationtests?|specs?)([._\- ]|$)"));

fn has_file(folder: &Path, names: &[&str]) -> bool {
    names.iter().any(|name| folder.join(name).exists())
}

pub type Step = (&'static str, String);

fn step_name(name: &str) -> &'static str {
    STEP_ORDER.iter().find(|s| **s == name).copied().unwrap_or("")
}

pub fn node_steps(folder: &Path) -> Vec<Step> {
    let Ok(raw) = text::read_utf8_sig(&folder.join("package.json")) else { return vec![] };
    let Ok(Value::Object(package)) = serde_json::from_str::<Value>(&raw) else { return vec![] };
    let scripts = match package.get("scripts") {
        None => serde_json::Map::new(),
        Some(Value::Object(map)) => map.clone(),
        Some(_) => return vec![],
    };
    let runner = if has_file(folder, &["pnpm-lock.yaml"]) {
        "pnpm run"
    } else if has_file(folder, &["yarn.lock"]) {
        "yarn"
    } else {
        "npm run"
    };
    let mut steps: Vec<Step> = vec![];
    if scripts.contains_key("typecheck") {
        steps.push((step_name("typecheck"), format!("{runner} typecheck")));
    } else if has_file(folder, &["tsconfig.json", "tsconfig.app.json"]) {
        let project = if has_file(folder, &["tsconfig.app.json"]) { "tsconfig.app.json" } else { "tsconfig.json" };
        steps.push((step_name("typecheck"), format!("npx --no-install tsc --noEmit -p {project}")));
    }
    if scripts.contains_key("lint") {
        steps.push((step_name("lint"), format!("{runner} lint")));
    }
    if scripts.contains_key("build") {
        steps.push((step_name("build"), format!("{runner} build")));
    }
    let test_defined = match scripts.get("test") {
        None | Some(Value::Null) => false,
        Some(Value::String(script)) => !script.is_empty() && !script.contains("no test specified"),
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
    };
    if test_defined {
        steps.push((step_name("test"), format!("{runner} test")));
    }
    steps
}

/// Folders (as component lists) below `base` that `glob`'s `*` would visit: directories whose name does not start with a dot.
fn wildcard_dirs(base: &Path, parts: &[String]) -> Vec<(Vec<String>, std::path::PathBuf)> {
    let mut names = text::listdir(base);
    names.retain(|name| !name.starts_with('.') && base.join(name).is_dir());
    names
        .into_iter()
        .map(|name| {
            let mut next = parts.to_vec();
            next.push(name.clone());
            (next, base.join(name))
        })
        .collect()
}

/// `glob.glob(folder/*/.../*.ext)` for `depth` wildcard folders: the matching names as (components, full path text).
fn glob_hits(folder: &Path, depth: usize, extension: &str) -> Vec<(Vec<String>, String)> {
    let mut level: Vec<(Vec<String>, std::path::PathBuf)> = vec![(vec![], folder.to_path_buf())];
    for _ in 0..depth {
        level = level.iter().flat_map(|(parts, base)| wildcard_dirs(base, parts)).collect();
    }
    let root_text = folder.display().to_string();
    let mut hits = vec![];
    for (parts, base) in level {
        for name in text::listdir(&base) {
            if !name.starts_with('.') && name.to_lowercase().ends_with(extension) {
                let mut components = parts.clone();
                components.push(name);
                let full = components.iter().fold(root_text.clone(), |acc, part| text::join(&acc, part));
                hits.push((components, full));
            }
        }
    }
    hits
}

/// Closest .NET solution or project to the root (up to `max_depth` folders down; .sln before .csproj at the same depth). Returns the relative
/// path with `/`. Covers repos like src/App/App.csproj, not only the root.
pub fn find_dotnet_target(folder: &Path) -> Option<String> {
    for depth in 0..=MAX_DOTNET_DEPTH {
        for extension in DOTNET_EXTENSIONS {
            let mut hits: Vec<(Vec<String>, String)> = glob_hits(folder, depth, extension)
                .into_iter()
                .filter(|(components, _)| !components[..components.len() - 1].iter().any(|part| DOTNET_SKIP.contains(&part.as_str())))
                .collect();
            hits.sort_by(|a, b| a.1.cmp(&b.1));
            if let Some((components, _)) = hits.first() {
                return Some(components.join("/"));
            }
        }
    }
    None
}

/// A .csproj whose file name has `test`/`tests`/`spec` as a whole word (MyApp.Tests, my-app.test), never `Contest` or `Latest`. Skips node_modules,
/// bin, obj and the other heavy folders.
pub fn has_test_project(folder: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(folder) else { return false };
    let mut dirs = vec![];
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else { continue };
        if entry.path().is_dir() {
            dirs.push(name);
            continue;
        }
        if name.to_lowercase().ends_with(".csproj") {
            let stem = match name.rfind('.') {
                Some(at) if at > 0 => &name[..at],
                _ => &name[..],
            };
            if TEST_WORD.search(stem).is_some() {
                return true;
            }
        }
    }
    dirs.into_iter().filter(|name| !DOTNET_SKIP.contains(&name.as_str())).any(|name| has_test_project(&folder.join(name)))
}

pub fn dotnet_steps(folder: &Path) -> Vec<Step> {
    let Some(target) = find_dotnet_target(folder) else { return vec![] };
    let mut steps: Vec<Step> = vec![];
    if has_file(folder, &[".editorconfig"]) {
        // lint only when the project declares its style; without .editorconfig the formatter would fight legacy code
        steps.push((step_name("lint"), format!("dotnet format {target} --verify-no-changes -v q")));
    }
    steps.push((step_name("build"), format!("dotnet build {target} --nologo -v q")));
    if has_test_project(folder) {
        steps.push((step_name("test"), format!("dotnet test {target} --nologo -v q")));
    }
    steps
}

/// The steps to run, in the fixed order. In a folder with both stacks, Node wins per step.
pub fn detect(folder: &Path) -> Vec<Step> {
    let mut steps: Vec<Step> = vec![];
    if has_file(folder, &["package.json"]) {
        steps.extend(node_steps(folder));
    }
    if find_dotnet_target(folder).is_some() {
        for (name, command) in dotnet_steps(folder) {
            if !steps.iter().any(|(existing, _)| *existing == name) {
                steps.push((name, command));
            }
        }
    }
    STEP_ORDER.iter().filter_map(|name| steps.iter().find(|(n, _)| n == name).cloned()).collect()
}

pub fn first_error_line(output: &str) -> String {
    let lines: Vec<String> = pycompat::splitlines(output).into_iter().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    for line in &lines {
        if ERROR_LINE.search(line).is_some() {
            return text::head(line, LINE_LIMIT);
        }
    }
    match lines.last() {
        Some(last) => text::head(last, LINE_LIMIT),
        None => "no output".to_string(),
    }
}

enum Outcome {
    Finished { code: i32, output: String },
    TimedOut,
}

fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill").args(["/F", "/T", "/PID", &pid.to_string()]).stdout(Stdio::null()).stderr(Stdio::null()).status();
}

fn spawn_reader(mut stream: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buffer = vec![];
        let _ = stream.read_to_end(&mut buffer);
        buffer
    })
}

/// `subprocess.run(command, cwd=folder, shell=True, capture_output=True, timeout=...)`.
fn run_step(command: &str, folder: &Path, timeout: Duration) -> std::io::Result<Outcome> {
    #[cfg(windows)]
    let mut process = {
        use std::os::windows::process::CommandExt;
        let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
        let mut process = Command::new(comspec);
        process.raw_arg(format!("/c \"{command}\""));
        process
    };
    #[cfg(not(windows))]
    let mut process = {
        let mut process = Command::new("sh");
        process.arg("-c").arg(command);
        process
    };
    let mut child = process.current_dir(folder).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let stdout = spawn_reader(child.stdout.take().expect("piped"));
    let stderr = spawn_reader(child.stderr.take().expect("piped"));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            kill_tree(child.id());
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let out = stdout.join().unwrap_or_default();
    let err = stderr.join().unwrap_or_default();
    let Some(status) = status else { return Ok(Outcome::TimedOut) };
    let decode = |bytes: Vec<u8>| text::universal_newlines(&String::from_utf8_lossy(&bytes));
    Ok(Outcome::Finished { code: status.code().unwrap_or(1), output: format!("{}\n{}", decode(out), decode(err)) })
}

pub fn main(argv: &[String]) -> i32 {
    let specs = [
        Opt { long: "scope", takes_value: true },
        Opt { long: "dry-run", takes_value: false },
        Opt { long: "timeout", takes_value: true },
        Opt { long: "only", takes_value: true },
    ];
    let parsed = match args::parse_or_exit("verify", USAGE, HELP, &specs, argv) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    if let Some(extra) = parsed.positionals.first() {
        return args::fail(
            "verify",
            USAGE,
            &format!("unrecognized arguments: {}", parsed.positionals.join(" ")).replace(&format!("arguments: {extra}"), &format!("arguments: {extra}")),
        );
    }
    let timeout = match parsed.value("timeout") {
        None => DEFAULT_TIMEOUT_SECONDS,
        Some(raw) => match args::parse_int(raw) {
            Some(n) => n,
            None => return args::fail("verify", USAGE, &format!("argument --timeout: invalid int value: '{raw}'")),
        },
    };
    let folder = match parsed.value("scope") {
        Some(scope) => text::abspath(scope),
        None => std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| ".".into()),
    };
    let only: Option<Vec<String>> = parsed.value("only").map(|raw| raw.split(',').map(str::trim).filter(|n| !n.is_empty()).map(String::from).collect());
    if let Some(names) = &only {
        if !names.is_empty() && !names.iter().all(|n| STEP_ORDER.contains(&n.as_str())) {
            text::err(&format!("--only accepts: {}\n", STEP_ORDER.join(",")));
            return 2;
        }
    }
    let folder_path = Path::new(&folder);
    if !folder_path.is_dir() {
        text::err(&format!("folder not found: {folder}\n"));
        return 2;
    }
    let filter = only.filter(|names| !names.is_empty());
    let steps: Vec<Step> = detect(folder_path).into_iter().filter(|(name, _)| filter.as_ref().is_none_or(|names| names.iter().any(|n| n == name))).collect();
    if parsed.flag("dry-run") {
        text::println_out(&format!("FOLDER: {folder}"));
        for (name, command) in &steps {
            text::println_out(&format!("{} {command}", text::pad_right(name, 9)));
        }
        if steps.is_empty() {
            text::println_out("(nothing detected)");
        }
        return 0;
    }
    if steps.is_empty() {
        text::println_out(&format!("VERDICT: UNVERIFIED\nRAN: -\nFAIL: none\nNOTES: no build/typecheck/lint/test detected in {folder}"));
        return 3;
    }
    let mut failures: Vec<String> = vec![];
    let mut ran: Vec<String> = vec![];
    for (_, command) in &steps {
        ran.push(command.clone());
        match run_step(command, folder_path, Duration::from_secs(timeout.max(0) as u64)) {
            Ok(Outcome::TimedOut) => failures.push(format!("{command}: timeout of {timeout}s")),
            Ok(Outcome::Finished { code, output }) => {
                if code != 0 {
                    failures.push(format!("{command}: {}", first_error_line(&output)));
                }
            }
            Err(error) => failures.push(format!("{command}: cannot start ({error})")),
        }
    }
    text::println_out(&format!("VERDICT: {}", if failures.is_empty() { "PASS" } else { "FAIL" }));
    text::println_out(&format!("RAN: {}", ran.join(", ")));
    text::println_out(&format!("FAIL:{}", if failures.is_empty() { " none" } else { "" }));
    for failure in &failures {
        text::println_out(&format!("- {failure}"));
    }
    if failures.is_empty() {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, relative: &str, body: &str) {
        let path = dir.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn node_scripts_become_steps_in_the_fixed_order() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "package.json", r#"{"scripts":{"build":"x","lint":"y","test":"echo \"Error: no test specified\" && exit 1"}}"#);
        write(dir.path(), "pnpm-lock.yaml", "");
        write(dir.path(), "tsconfig.json", "{}");
        let steps = detect(dir.path());
        assert_eq!(
            steps,
            vec![
                ("typecheck", "npx --no-install tsc --noEmit -p tsconfig.json".to_string()),
                ("lint", "pnpm run lint".to_string()),
                ("build", "pnpm run build".to_string())
            ]
        );
    }

    #[test]
    fn dotnet_targets_are_found_below_the_root_and_skip_heavy_folders() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/App/App.csproj", "");
        write(dir.path(), "node_modules/x/Bad.csproj", "");
        write(dir.path(), ".editorconfig", "");
        write(dir.path(), "tests/App.Tests/App.Tests.csproj", "");
        assert_eq!(find_dotnet_target(dir.path()).as_deref(), Some("src/App/App.csproj"));
        let steps = detect(dir.path());
        let names: Vec<&str> = steps.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, vec!["lint", "build", "test"]);
        assert_eq!(steps[1].1, "dotnet build src/App/App.csproj --nologo -v q");
    }

    #[test]
    fn test_projects_need_a_whole_word() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Contest/Contest.csproj", "");
        assert!(!has_test_project(dir.path()));
        write(dir.path(), "MyApp.Tests/MyApp.Tests.csproj", "");
        assert!(has_test_project(dir.path()));
    }

    #[test]
    fn the_first_useful_error_line_wins() {
        assert_eq!(first_error_line("ok\n  src/a.ts(1,2): error TS2304: x\nmore"), "src/a.ts(1,2): error TS2304: x");
        assert_eq!(first_error_line("a\nlast line"), "last line");
        assert_eq!(first_error_line(" \n"), "no output");
    }
}
