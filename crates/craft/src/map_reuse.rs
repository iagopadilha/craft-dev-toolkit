// SPDX-License-Identifier: MIT
//! `craft map_reuse <project-root> [--max 60]`: a DRAFT of the "Reuse map" section of the project's CLAUDE.md.
//!
//! Scans C# (.cs) and TS/TSX/JS looking for what is usually generic and reusable:
//!   - folders: common, shared, helpers, utils, util, extensions, base, core, lib, hooks, infrastructure
//!   - files/classes: *Helper, *Utils, *Extensions, *Base, Base*, *Factory, *Saver, *Handler, use*.ts(x)
//!
//! Lists symbol + signature (1 line) + path. Reads nothing beyond that, edits no file, uses no network. The result is a draft: curate it by hand.

use crate::args::{self, Opt};
use crate::text;
use pycompat::Re;
use std::path::Path;
use std::sync::LazyLock;

const SKIP_DIRS: [&str; 12] = ["node_modules", "bin", "obj", ".git", "dist", "build", ".next", ".specs", ".vs", "coverage", "wwwroot", "Migrations"];
const SKIP_DIRS_EXTRA: &str = "__pycache__";
const EXTENSIONS: [&str; 4] = [".cs", ".ts", ".tsx", ".js"];
const SKIP_SUFFIXES: [&str; 5] = [".d.ts", ".test.ts", ".test.tsx", ".spec.ts", ".Designer.cs"];
const MAX_SYMBOLS_PER_FILE: usize = 8;
const SYMBOLS_PER_ROW: usize = 3;
const SIGNATURE_LIMIT: usize = 70;
const USAGE: &str = "map_reuse [-h] [--max MAX] root";
const HELP: &str = "usage: map_reuse [-h] [--max MAX] root

Prints a DRAFT of the 'Reuse map' section of a project's CLAUDE.md (generic helpers and top-level folders). Reads source files only; edits nothing; no network.

positional arguments:
  root        project root folder

options:
  -h, --help  show this help message and exit
  --max MAX   maximum candidate files listed (default 60)
";

static GENERIC_DIRS: LazyLock<Re> = LazyLock::new(|| Re::ci(r"(^|[\\/])(common|shared|helpers?|utils?|extensions?|base|core|lib|hooks|infrastructure)([\\/]|$)"));
static GENERIC_NAMES: LazyLock<Re> = LazyLock::new(|| {
    Re::new(
        r"(Helper|Utils?|Extensions?|Base|Factory|Saver|Handler|Formatter|Validator|Mapper)\.(cs|ts|tsx|js)(?=\n?\z)|^Base\w+\.(cs|ts)(?=\n?\z)|^use[A-Z]\w+\.(ts|tsx)(?=\n?\z)",
    )
});
static CS_SYMBOL: LazyLock<Re> =
    LazyLock::new(|| Re::new(r"^\s*public\s+(?:static\s+|abstract\s+|virtual\s+|async\s+|sealed\s+|partial\s+)*([\w<>\[\],?\. ]+?)\s+(\w+)\s*\(([^)]*)\)"));
static CS_TYPE: LazyLock<Re> = LazyLock::new(|| Re::new(r"^\s*public\s+(?:static\s+|abstract\s+|sealed\s+|partial\s+)*(class|interface|record)\s+(\w+)"));
static TS_SYMBOL: LazyLock<Re> = LazyLock::new(|| Re::new(r"^\s*export\s+(?:default\s+)?(?:async\s+)?(function|const|class|interface|type)\s+(\w+)([^{=]*)"));
static LEADING_KEYWORD: LazyLock<Re> = LazyLock::new(|| Re::new(r"^(class|interface|record|function|const|type)\s+"));

fn is_skipped(name: &str) -> bool {
    SKIP_DIRS.contains(&name) || name == SKIP_DIRS_EXTRA
}

fn is_candidate(relative: &str, file_name: &str) -> bool {
    let dirname = match relative.rfind(['/', '\\']) {
        Some(at) => &relative[..at],
        None => "",
    };
    GENERIC_DIRS.is_match(&format!("{dirname}\\")) || GENERIC_NAMES.is_match(file_name)
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Up to eight "kind name(signature)" strings of the file.
pub fn symbols_in(path: &Path, extension: &str) -> Vec<String> {
    let mut found: Vec<String> = vec![];
    let Ok(raw) = text::read_lossy(path) else { return found };
    for line in text::universal_newlines(&raw).split_inclusive('\n') {
        if extension == ".cs" {
            if let Some(m) = CS_TYPE.match_start(line) {
                found.push(format!("{} {}", &m[1], &m[2]));
                continue;
            }
            if let Some(m) = CS_SYMBOL.match_start(line) {
                if &m[2] != "get" && &m[2] != "set" {
                    found.push(format!("{} {}({})", m[1].trim(), &m[2], text::head(&one_line(&m[3]), SIGNATURE_LIMIT)));
                }
            }
        } else if let Some(m) = TS_SYMBOL.match_start(line) {
            found.push(format!("{} {}{}", &m[1], &m[2], text::head(&one_line(&m[3]), SIGNATURE_LIMIT)));
        }
        if found.len() >= MAX_SYMBOLS_PER_FILE {
            break;
        }
    }
    found
}

fn walk(root: &Path, current: &Path, rows: &mut Vec<(String, Vec<String>)>) {
    let Ok(entries) = std::fs::read_dir(current) else { return };
    let mut dirs: Vec<String> = vec![];
    let mut files: Vec<String> = vec![];
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else { continue };
        let kind = entry.file_type();
        match kind {
            Ok(kind) if kind.is_dir() => dirs.push(name),
            Ok(kind) if kind.is_symlink() => {
                if entry.path().is_dir() {
                    dirs.push(name);
                } else {
                    files.push(name);
                }
            }
            _ => files.push(name),
        }
    }
    for file_name in &files {
        let extension = match file_name.rfind('.') {
            Some(at) if at > 0 => &file_name[at..],
            _ => "",
        };
        if !EXTENSIONS.contains(&extension) || SKIP_SUFFIXES.iter().any(|s| file_name.ends_with(s)) {
            continue;
        }
        let full = current.join(file_name);
        let relative = full.strip_prefix(root).unwrap_or(&full).display().to_string().replace('\\', "/");
        if is_candidate(&relative, file_name) {
            let symbols = symbols_in(&full, extension);
            if !symbols.is_empty() {
                rows.push((relative, symbols));
            }
        }
    }
    for name in dirs {
        if is_skipped(&name) {
            continue;
        }
        let path = current.join(&name);
        if std::fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            continue; // like os.walk: a link to a folder is listed but not followed
        }
        walk(root, &path, rows);
    }
}

fn symbol_name(symbol: &str) -> String {
    let stripped = LEADING_KEYWORD.replace_lit(symbol, "");
    let before_paren = stripped.split('(').next().unwrap_or("");
    before_paren.split_whitespace().last().unwrap_or("").to_string()
}

pub fn main(argv: &[String]) -> i32 {
    let specs = [Opt { long: "max", takes_value: true }];
    let parsed = match args::parse_or_exit("map_reuse", USAGE, HELP, &specs, argv) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let limit = match parsed.value("max") {
        None => 60,
        Some(raw) => match args::parse_int(raw) {
            Some(n) => n,
            None => return args::fail("map_reuse", USAGE, &format!("argument --max: invalid int value: '{raw}'")),
        },
    };
    let root_arg = match parsed.positionals.as_slice() {
        [one] => one.clone(),
        [] => return args::fail("map_reuse", USAGE, "the following arguments are required: root"),
        [_, rest @ ..] => return args::fail("map_reuse", USAGE, &format!("unrecognized arguments: {}", rest.join(" "))),
    };
    let root = text::abspath(&root_arg);
    let root_path = Path::new(&root);
    if !root_path.is_dir() {
        text::err(&format!("folder not found: {root}\n"));
        return 2;
    }
    let mut rows: Vec<(String, Vec<String>)> = vec![];
    walk(root_path, root_path, &mut rows);
    rows.sort();
    let limit_rows = if limit < 0 { rows.len().saturating_sub(limit.unsigned_abs() as usize) } else { (limit as usize).min(rows.len()) };
    text::println_out("## Reuse map (DRAFT generated by map_reuse.py; curate by hand)");
    text::println_out("");
    text::println_out("| I need... | Use | Where | Signature / note |");
    text::println_out("|---|---|---|---|");
    for (relative, symbols) in rows.iter().take(limit_rows) {
        for symbol in symbols.iter().take(SYMBOLS_PER_ROW) {
            text::println_out(&format!("| <describe the need> | `{}` | `{relative}` | {} |", symbol_name(symbol), symbol.replace('|', "/")));
        }
    }
    if (rows.len() as i64) > limit {
        text::println_out("");
        text::println_out(&format!("<!-- {} candidate files beyond the {limit} listed; use --max -->", rows.len() as i64 - limit));
    }
    if rows.is_empty() {
        text::println_out("| (no generic helper detected) | | | |");
    }
    text::println_out("");
    text::println_out("## Structure (draft: top-level folders)");
    text::println_out("");
    text::println_out("| Folder | What it holds | Accepts new code? |");
    text::println_out("|---|---|---|");
    let mut names = text::listdir(root_path);
    names.sort();
    for name in names {
        if root_path.join(&name).is_dir() && !is_skipped(&name) && !name.starts_with('.') {
            text::println_out(&format!("| `{name}/` | <describe> | <yes/no> |"));
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_are_generic_folders_or_helper_names() {
        assert!(is_candidate("src/Common/Dates.cs", "Dates.cs"));
        assert!(is_candidate("src/Orders/OrderHelper.cs", "OrderHelper.cs"));
        assert!(is_candidate("src/ui/useCart.ts", "useCart.ts"));
        assert!(!is_candidate("src/Orders/Order.cs", "Order.cs"));
    }

    #[test]
    fn names_come_from_the_symbol_text() {
        assert_eq!(symbol_name("class Foo"), "Foo");
        assert_eq!(symbol_name("Task<int> Compute(string a)"), "Compute");
        assert_eq!(symbol_name("function useCart<T>"), "useCart<T>");
    }

    #[test]
    fn symbols_are_read_from_csharp_and_typescript() {
        let dir = tempfile::tempdir().unwrap();
        let cs = dir.path().join("DateHelper.cs");
        std::fs::write(&cs, "public static class DateHelper\n{\n    public static string Format(DateTime value, string pattern)\n    { }\n}\n").unwrap();
        assert_eq!(symbols_in(&cs, ".cs"), vec!["class DateHelper", "string Format(DateTime value, string pattern)"]);
        let ts = dir.path().join("useCart.ts");
        std::fs::write(&ts, "export function useCart(id: string) {\n}\nexport const x = 1;\n").unwrap();
        assert_eq!(symbols_in(&ts, ".ts"), vec!["function useCart(id: string)", "const x"]);
    }
}
