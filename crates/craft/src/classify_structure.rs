// SPDX-License-Identifier: MIT
//! `craft classify_structure <root> [--write STANDARD|LEGACY [--note "text"]]`: classifies a project's structure as STANDARD or LEGACY and (once
//! confirmed) writes it to CLAUDE.md.
//!
//! The suggestion is only a suggestion: the user decides. Without a "Conformance" section in the project's CLAUDE.md, the skill must explain why,
//! show this evidence and ASK the user before writing.
//!
//! STANDARD = the skill's structure: back end with Api/Data/Domain (.NET projects or NestJS folders under src/); front end with src/pages +
//!            src/components (+ services/hooks).
//! LEGACY   = anything else. New code imitates the project; old files only get minimal edits.

use crate::text;
use pycompat::Re;
use std::path::Path;

const BEGIN: &str = "<!-- CONFORMANCE:BEGIN -->";
const END: &str = "<!-- CONFORMANCE:END -->";
const SKIP: [&str; 9] = ["node_modules", "bin", "obj", ".git", "dist", "build", ".next", ".specs", ".vs"];
const FLAT_FOLDERS: [&str; 5] = ["Controllers", "Services", "Models", "Utils", "Interfaces"];

const DOC: &str = "Classifies a project's structure as STANDARD or LEGACY and (once confirmed) writes it to CLAUDE.md.

Usage:
  craft classify_structure <root>                      # shows evidence and the SUGGESTION (writes nothing)
  craft classify_structure <root> --write STANDARD|LEGACY [--note \"text\"]   # writes the Conformance section

The suggestion is only a suggestion: the user decides. Without a \"Conformance\" section in the project's CLAUDE.md, the skill
must explain why, show this evidence and ASK the user before writing.

STANDARD = the skill's structure: back end with Api/Data/Domain (.NET projects or NestJS folders under src/);
           front end with src/pages + src/components (+ services/hooks).
LEGACY   = anything else. New code imitates the project; old files only get minimal edits.

";

fn subdirs(path: &Path) -> Vec<String> {
    text::listdir(path).into_iter().filter(|name| path.join(name).is_dir() && !SKIP.contains(&name.as_str())).collect()
}

/// `glob.glob(root/*.ext)`: entries (files or folders) with that extension, hidden names excluded, case-insensitive like Windows.
fn has_glob(root: &Path, extension: &str) -> bool {
    text::listdir(root).iter().any(|name| !name.starts_with('.') && name.to_lowercase().ends_with(extension))
}

pub struct Analysis {
    pub stack: String,
    pub verdict: &'static str,
    pub evidence: Vec<String>,
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()).unwrap_or_default()
}

pub fn analyze(root: &Path) -> Analysis {
    let mut evidence: Vec<String> = vec![];
    let mut stack = "unknown".to_string();
    let mut verdict = "LEGACY";
    let top = subdirs(root);
    let csproj_here = has_glob(root, ".csproj");
    let solutions = has_glob(root, ".sln");
    let package = root.join("package.json");
    if solutions || csproj_here {
        stack = ".NET".into();
        let mut layers: Vec<(&str, Vec<String>)> = vec![];
        for layer in ["api", "data", "domain"] {
            let pattern = Re::ci(&format!(r"(^|\.){layer}(?=\n?\z)"));
            let found: Vec<String> = top.iter().filter(|name| pattern.search(name).is_some()).cloned().collect();
            evidence.push(format!("project/folder {}: {}", capitalize(layer), if found.is_empty() { "NOT FOUND".to_string() } else { found.join(", ") }));
            layers.push((layer, found));
        }
        let all_layers = layers.iter().all(|(_, found)| !found.is_empty());
        if all_layers {
            verdict = "STANDARD";
        }
        if csproj_here && !all_layers {
            evidence.push(".csproj in the root (single project): flat structure".into());
        }
        let flat: Vec<&String> = top.iter().filter(|name| FLAT_FOLDERS.contains(&name.as_str())).collect();
        if !flat.is_empty() {
            evidence.push(format!("flat folders in the root: {}", flat.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }
    } else if package.exists() {
        let src = root.join("src");
        let src_dirs: Vec<String> = subdirs(&src).iter().map(|name| name.to_lowercase()).collect();
        let body = text::read_lossy(&package).unwrap_or_default();
        if body.contains("@nestjs") {
            stack = "NestJS".into();
            for layer in ["api", "data", "domain"] {
                evidence.push(format!("src/{layer}: {}", if src_dirs.iter().any(|d| d == layer) { "ok" } else { "NOT FOUND" }));
            }
            if ["api", "data", "domain"].iter().all(|layer| src_dirs.iter().any(|d| d == layer)) {
                verdict = "STANDARD";
            }
        } else {
            stack = if body.contains("react") && !root.join("app").is_dir() { "React/Vite/TS" } else { "React/Next" }.into();
            for name in ["pages", "components", "services", "hooks"] {
                evidence.push(format!("src/{name}: {}", if src_dirs.iter().any(|d| d == name) { "ok" } else { "NOT FOUND" }));
            }
            let has_pages = src_dirs.iter().any(|d| d == "pages");
            if root.join("app").is_dir() && !has_pages {
                evidence.push("app/ folder in the root (Next app router structure, no src/pages)".into());
            }
            if has_pages && src_dirs.iter().any(|d| d == "components") {
                verdict = "STANDARD";
            }
        }
    } else {
        evidence.push("neither .sln/.csproj nor package.json in the root: confirm the project root".into());
    }
    Analysis { stack, verdict, evidence }
}

fn today() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    format!("{:04}-{:02}-{:02}", now.year(), u8::from(now.month()), now.day())
}

pub fn section(classification: &str, stack: &str, suggestion: &str, note: &str, date: &str) -> String {
    let mut lines: Vec<String> = vec![
        BEGIN.into(),
        "## Conformance".into(),
        format!("Conformance: {classification}"),
        format!("Stack: {stack}"),
        format!("Defined by: user on {date} (script suggestion: {suggestion})"),
    ];
    if !note.is_empty() {
        lines.push(format!("Note: {note}"));
    }
    if classification == "STANDARD" {
        lines.push("Rule: new code follows the skills' standard (back end Api/Data/Domain; front end pages/components). A file the feature touches may be edited and the touched class refactored to the standard (only it), with a regression check offered.".into());
    } else {
        lines.push("Rule: LEGACY. New code follows the pattern the project already has (no new layer). Old files only get minimal edits, no renaming or splitting; refactoring becomes a Point of attention. Never touch anything outside the feature's PATHS.".into());
    }
    lines.push("Scope (always): only the feature's PATHS and only repo code. A new file is fine if it stays in scope; a large deviation = stop and ask.".into());
    lines.push(END.into());
    lines.push(String::new());
    lines.join("\n")
}

/// The CLAUDE.md text with the block replaced in place, or appended after the last non-blank line.
pub fn place_block(text: &str, block: &str) -> String {
    if let Some(begin) = text.find(BEGIN) {
        // a begin marker without its end marker is left as it is (the Python pattern finds nothing to replace)
        let Some(found) = text[begin + BEGIN.len()..].find(END) else { return text.to_string() };
        let mut end_at = begin + BEGIN.len() + found + END.len();
        if text[end_at..].starts_with('\n') {
            end_at += 1;
        }
        return format!("{}{block}{}", &text[..begin], &text[end_at..]);
    }
    format!("{}\n\n{block}", text.trim_end_matches('\n'))
}

fn write_claude_md(root: &str, classification: &str, stack: &str, suggestion: &str, note: &str) -> std::io::Result<String> {
    let path = text::join(root, "CLAUDE.md");
    let block = section(classification, stack, suggestion, note, &today());
    let (mut newline, mut bom) = ("\n", "");
    let mut body: String;
    if Path::new(&path).exists() {
        let raw = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
        body = raw;
        if let Some(rest) = body.strip_prefix('\u{feff}') {
            bom = "\u{feff}";
            body = rest.to_string();
        }
        if body.contains("\r\n") {
            newline = "\r\n";
        }
        body = body.replace("\r\n", "\n");
    } else {
        body = format!("# {}\n\n", text::basename(root));
    }
    let updated = place_block(&body, &block);
    std::fs::write(&path, format!("{bom}{}", updated.replace('\n', newline)).as_bytes())?;
    Ok(path)
}

/// Value after `flag`, or `None` when the flag is absent; a flag with no value (or followed by another flag) is an error.
fn option_value(args: &[String], flag: &str) -> Result<Option<String>, String> {
    let Some(at) = args.iter().position(|a| a == flag) else { return Ok(None) };
    match args.get(at + 1) {
        Some(value) if !value.starts_with("--") => Ok(Some(value.clone())),
        _ => Err(format!("{flag} needs a value")),
    }
}

pub fn main(argv: &[String]) -> i32 {
    if argv.is_empty() || argv[0] == "-h" || argv[0] == "--help" {
        text::out(DOC);
        text::out("\n");
        return if argv.is_empty() { 2 } else { 0 };
    }
    let root = text::abspath(&argv[0]);
    let analysis = analyze(Path::new(&root));
    if argv.iter().any(|a| a == "--write") {
        let parsed = option_value(argv, "--write").and_then(|w| option_value(argv, "--note").map(|n| (w.unwrap_or_default().to_uppercase(), n.unwrap_or_default())));
        let (classification, note) = match parsed {
            Ok(pair) => pair,
            Err(error) => {
                text::err(&format!("error: {error} (use --write STANDARD|LEGACY [--note \"text\"])\n"));
                return 2;
            }
        };
        if classification != "STANDARD" && classification != "LEGACY" {
            text::err("error: --write takes STANDARD or LEGACY\n");
            return 2;
        }
        return match write_claude_md(&root, &classification, &analysis.stack, analysis.verdict, &note) {
            Ok(path) => {
                text::println_out(&format!("written to {path}"));
                0
            }
            Err(error) => {
                text::err(&format!("classify_structure: cannot write CLAUDE.md: {error}\n"));
                1
            }
        };
    }
    text::println_out(&format!("ROOT: {root}"));
    text::println_out(&format!("STACK: {}", analysis.stack));
    text::println_out(&format!("SUGGESTION: {}", analysis.verdict));
    for line in &analysis.evidence {
        text::println_out(&format!(" - {line}"));
    }
    text::println_out("QUESTION FOR THE USER: is this project already in the skills' new standard (STANDARD) or is it legacy (LEGACY)?");
    text::println_out("WHY ASK: in LEGACY the agent only makes minimal edits to old files (without tests, refactoring there causes silent regressions); in STANDARD it may refactor the class it touches.");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_is_appended_then_replaced_in_place() {
        let block = section("LEGACY", ".NET", "STANDARD", "", "2026-01-02");
        let first = place_block("# App\n\nintro\n", &block);
        assert!(first.starts_with("# App\n\nintro\n\n<!-- CONFORMANCE:BEGIN -->\n## Conformance\nConformance: LEGACY\n"));
        let second = place_block(&first, &section("STANDARD", ".NET", "STANDARD", "ok", "2026-01-02"));
        assert!(second.contains("Conformance: STANDARD") && second.contains("Note: ok") && !second.contains("Conformance: LEGACY"));
        assert_eq!(second.matches(BEGIN).count(), 1);
    }

    #[test]
    fn a_dotnet_solution_with_all_layers_is_standard() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["App.sln", "x"] {
            if name.ends_with(".sln") {
                std::fs::write(dir.path().join(name), "").unwrap();
            }
        }
        for folder in ["App.Api", "App.Data", "App.Domain"] {
            std::fs::create_dir(dir.path().join(folder)).unwrap();
        }
        let result = analyze(dir.path());
        assert_eq!((result.stack.as_str(), result.verdict), (".NET", "STANDARD"));
        assert!(result.evidence.iter().any(|l| l.starts_with("project/folder Api: App.Api")));
    }

    #[test]
    fn a_vite_project_without_pages_is_legacy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{\"dependencies\":{\"react\":\"1\"}}").unwrap();
        std::fs::create_dir_all(dir.path().join("src").join("components")).unwrap();
        let result = analyze(dir.path());
        assert_eq!((result.stack.as_str(), result.verdict), ("React/Vite/TS", "LEGACY"));
    }
}
