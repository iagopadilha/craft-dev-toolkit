// SPDX-License-Identifier: MIT
//! `craft standards check [--scope FOLDER] [--skills a,b] [--toolkit DIR] [--files f1,f2] [--dry-run]`: the deterministic rules of the
//! skills, run in code with no model. A skill keeps its rules in `<toolkit>/skills/<skill>/rules.json`:
//!
//!     {"skill": "backend-dev", "rules": [
//!       {"id": "BE-FILE-SIZE", "check": "deterministic", "kind": "max-file-lines", "max": 1500, "globs": ["*.cs"], "text": "..."},
//!       {"id": "BE-NO-ANY", "check": "deterministic", "kind": "forbid-regex", "pattern": ": any\b", "globs": ["*.ts"], "text": "..."},
//!       {"id": "BE-NAMES", "check": "llm", "text": "Names are descriptive, no abbreviations"}]}
//!
//! Only `check: deterministic` rules run here; the `llm` ones are judged by the `quality-gate` agent. Only NEW code is judged: for a
//! tracked file the rule sees the added lines of `git diff` (a file that was already over the size limit fails only when the change
//! makes it longer), for an untracked file all lines. Reads git (diff, ls-files, show), never writes.
//!
//! Output (format of `verify`): `VERDICT: PASS|FAIL|UNVERIFIED`, `RAN:`, `FAIL:` lines `- <rule-id>: <text> @ path:line`.
//! Exit: 0 PASS, 1 FAIL, 3 UNVERIFIED (no deterministic rules, or no changed file), 2 bad arguments.

use crate::args::{self, Opt};
use crate::text;
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const USAGE: &str = "standards check [-h] [--scope SCOPE] [--skills SKILLS] [--toolkit TOOLKIT] [--files FILES] [--dry-run]";
const HELP: &str = "usage: standards check [-h] [--scope SCOPE] [--skills SKILLS] [--toolkit TOOLKIT] [--files FILES] [--dry-run]

Runs the deterministic rules of the skills (rules.json next to each SKILL.md) on the files that changed. Does not edit files.

options:
  -h, --help         show this help message and exit
  --scope SCOPE      folder to check (default: the current folder)
  --skills SKILLS    comma-separated skills (default: backend-dev,frontend-dev)
  --toolkit TOOLKIT  toolkit root (default: the folder above this binary's bin/)
  --files FILES      comma-separated files, relative to the scope, instead of asking git
  --dry-run          only print the rules that would run

Exit codes: 0 PASS, 1 FAIL, 3 UNVERIFIED, 2 bad arguments.
";
const DEFAULT_SKILLS: &str = "backend-dev,frontend-dev";
const SKIP_PARTS: [&str; 7] = ["node_modules", "bin", "obj", "dist", ".git", "migrations", ".specs"];
const MAX_BYTES: u64 = 1_000_000;
const SHOWN_FAILURES: usize = 20;

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub text: String,
    pub kind: Kind,
    pub globs: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Kind {
    MaxFileLines(usize),
    ForbidRegex(Regex),
}

/// A file under judgment: its path, all its lines, and which line numbers (1-based) are new code.
pub struct Subject {
    pub path: String,
    pub lines: Vec<String>,
    pub new_lines: BTreeSet<usize>,
    pub lines_before: Option<usize>,
}

fn glob_regex(glob: &str) -> Regex {
    let mut out = String::from("(?i)(^|/)");
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                out.push_str(".*");
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            other => out.push_str(&regex::escape(&other.to_string())),
        }
    }
    out.push('$');
    Regex::new(&out).expect("glob regex")
}

fn matches_globs(path: &str, globs: &[String]) -> bool {
    globs.is_empty() || globs.iter().any(|g| glob_regex(g).is_match(path))
}

/// The deterministic rules of one rules.json. A rule that cannot be understood is skipped (never a false FAIL).
pub fn parse_rules(raw: &str) -> Vec<Rule> {
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(raw) else { return vec![] };
    let Some(Value::Array(rules)) = doc.get("rules") else { return vec![] };
    rules
        .iter()
        .filter_map(|rule| {
            if rule.get("check").and_then(Value::as_str) != Some("deterministic") {
                return None;
            }
            let id = rule.get("id")?.as_str()?.to_string();
            let text = rule.get("text").and_then(Value::as_str).unwrap_or("").to_string();
            let globs = rule.get("globs").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
            let kind = match rule.get("kind")?.as_str()? {
                "max-file-lines" => Kind::MaxFileLines(rule.get("max")?.as_u64()? as usize),
                "forbid-regex" => Kind::ForbidRegex(Regex::new(rule.get("pattern")?.as_str()?).ok()?),
                _ => return None,
            };
            Some(Rule { id, text, kind, globs })
        })
        .collect()
}

/// One line per violation: `<rule-id>: <text> @ path:line`.
pub fn evaluate(rules: &[Rule], subjects: &[Subject]) -> Vec<String> {
    let mut failures = vec![];
    for subject in subjects {
        for rule in rules.iter().filter(|r| matches_globs(&subject.path, &r.globs)) {
            match &rule.kind {
                Kind::MaxFileLines(max) => {
                    let now = subject.lines.len();
                    let grew = subject.lines_before.is_none_or(|before| now > before);
                    if now > *max && grew {
                        failures.push(format!("{}: {} ({now} lines, limit {max}) @ {}", rule.id, rule.text, subject.path));
                    }
                }
                Kind::ForbidRegex(pattern) => {
                    for (index, line) in subject.lines.iter().enumerate() {
                        if subject.new_lines.contains(&(index + 1)) && pattern.is_match(line) {
                            failures.push(format!("{}: {} @ {}:{}", rule.id, rule.text, subject.path, index + 1));
                        }
                    }
                }
            }
        }
    }
    failures
}

/// Added line numbers of a `git diff -U0` output (the `+c,d` side of every hunk).
pub fn added_lines(diff: &str) -> BTreeSet<usize> {
    let hunk = Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@").expect("hunk regex");
    let mut out = BTreeSet::new();
    for line in diff.lines() {
        if let Some(c) = hunk.captures(line) {
            let start: usize = c[1].parse().unwrap_or(0);
            let count: usize = c.get(2).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            out.extend(start..start + count);
        }
    }
    out
}

fn git(folder: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").current_dir(folder).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn skip(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.split('/').any(|part| SKIP_PARTS.contains(&part)) || lower.contains(".min.")
}

fn changed_files(folder: &Path, explicit: Option<&str>) -> Option<Vec<(String, bool)>> {
    // (path relative to the scope, tracked)
    if let Some(list) = explicit {
        return Some(list.split(',').map(str::trim).filter(|f| !f.is_empty()).map(|f| (f.replace('\\', "/"), false)).collect());
    }
    let tracked = git(folder, &["diff", "--name-only", "--relative", "HEAD", "--", "."])?;
    let untracked = git(folder, &["ls-files", "--others", "--exclude-standard"])?;
    let mut files: Vec<(String, bool)> = tracked.lines().filter(|l| !l.is_empty()).map(|l| (l.to_string(), true)).collect();
    files.extend(untracked.lines().filter(|l| !l.is_empty()).map(|l| (l.to_string(), false)));
    Some(files)
}

fn subject_of(folder: &Path, path: &str, tracked: bool) -> Option<Subject> {
    let full = folder.join(path);
    let meta = std::fs::metadata(&full).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    let content = std::fs::read_to_string(&full).ok()?;
    let lines: Vec<String> = content.lines().map(String::from).collect();
    if !tracked {
        let all = (1..=lines.len()).collect();
        return Some(Subject { path: path.to_string(), lines, new_lines: all, lines_before: None });
    }
    let diff = git(folder, &["diff", "-U0", "HEAD", "--", path]).unwrap_or_default();
    let before = git(folder, &["show", &format!("HEAD:./{path}")]).map(|c| c.lines().count());
    Some(Subject { path: path.to_string(), lines, new_lines: added_lines(&diff), lines_before: before })
}

fn toolkit_root(parsed: &args::Parsed) -> PathBuf {
    if let Some(dir) = parsed.value("toolkit") {
        return PathBuf::from(dir);
    }
    std::env::current_exe().ok().and_then(|exe| exe.parent().and_then(Path::parent).map(PathBuf::from)).unwrap_or_default()
}

pub fn main(argv: &[String]) -> i32 {
    let specs = [
        Opt { long: "scope", takes_value: true },
        Opt { long: "skills", takes_value: true },
        Opt { long: "toolkit", takes_value: true },
        Opt { long: "files", takes_value: true },
        Opt { long: "dry-run", takes_value: false },
    ];
    let parsed = match args::parse_or_exit("standards", USAGE, HELP, &specs, argv) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    if parsed.positionals.first().map(String::as_str) != Some("check") || parsed.positionals.len() > 1 {
        return args::fail("standards", USAGE, "the only command is `check`");
    }
    let folder = parsed.value("scope").map(text::abspath).unwrap_or_else(|| std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| ".".into()));
    let folder_path = Path::new(&folder);
    if !folder_path.is_dir() {
        text::err(&format!("folder not found: {folder}\n"));
        return 2;
    }
    let toolkit = toolkit_root(&parsed);
    let mut rules: Vec<Rule> = vec![];
    for skill in parsed.value("skills").unwrap_or(DEFAULT_SKILLS).split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(raw) = std::fs::read_to_string(toolkit.join("skills").join(skill).join("rules.json")) {
            rules.extend(parse_rules(&raw));
        }
    }
    if parsed.flag("dry-run") {
        text::println_out(&format!("FOLDER: {folder}"));
        for rule in &rules {
            text::println_out(&format!("{} {}", text::pad_right(&rule.id, 24), rule.text));
        }
        if rules.is_empty() {
            text::println_out("(no deterministic rules)");
        }
        return 0;
    }
    if rules.is_empty() {
        text::println_out("VERDICT: UNVERIFIED\nRAN: -\nFAIL: none\nNOTES: no deterministic rules found (rules.json) for the skills");
        return 3;
    }
    let Some(files) = changed_files(folder_path, parsed.value("files")) else {
        text::println_out("VERDICT: UNVERIFIED\nRAN: -\nFAIL: none\nNOTES: not a git repository, pass --files");
        return 3;
    };
    let subjects: Vec<Subject> = files.iter().filter(|(path, _)| !skip(path)).filter_map(|(path, tracked)| subject_of(folder_path, path, *tracked)).collect();
    if subjects.is_empty() {
        text::println_out("VERDICT: UNVERIFIED\nRAN: -\nFAIL: none\nNOTES: no changed source file to check");
        return 3;
    }
    let failures = evaluate(&rules, &subjects);
    text::println_out(&format!("VERDICT: {}", if failures.is_empty() { "PASS" } else { "FAIL" }));
    text::println_out(&format!("RAN: {}", rules.iter().map(|r| r.id.as_str()).collect::<Vec<_>>().join(", ")));
    text::println_out(&format!("FAIL:{}", if failures.is_empty() { " none" } else { "" }));
    for failure in failures.iter().take(SHOWN_FAILURES) {
        text::println_out(&format!("- {failure}"));
    }
    if failures.len() > SHOWN_FAILURES {
        text::println_out(&format!("NOTES: {} more violations not shown", failures.len() - SHOWN_FAILURES));
    }
    i32::from(!failures.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(path: &str, body: &str, new_lines: &[usize], before: Option<usize>) -> Subject {
        Subject { path: path.into(), lines: body.lines().map(String::from).collect(), new_lines: new_lines.iter().copied().collect(), lines_before: before }
    }

    const RULES: &str = r#"{"skill":"x","rules":[
        {"id":"SIZE","check":"deterministic","kind":"max-file-lines","max":3,"globs":["*.cs"],"text":"file too long"},
        {"id":"NOANY","check":"deterministic","kind":"forbid-regex","pattern":": any\\b","globs":["*.ts"],"text":"no any"},
        {"id":"NAMES","check":"llm","text":"names are descriptive"},
        {"id":"BAD","check":"deterministic","kind":"forbid-regex","pattern":"(","text":"broken regex is skipped"}]}"#;

    #[test]
    fn only_deterministic_understood_rules_are_loaded() {
        let ids: Vec<String> = parse_rules(RULES).into_iter().map(|r| r.id).collect();
        assert_eq!(ids, vec!["SIZE", "NOANY"]);
    }

    #[test]
    fn a_forbidden_pattern_is_flagged_only_on_new_lines() {
        let rules = parse_rules(RULES);
        let file = subject("src/a.ts", "let a: any = 1;\nlet b: any = 2;\n", &[2], Some(1));
        let failures = evaluate(&rules, &[file]);
        assert_eq!(failures, vec!["NOANY: no any @ src/a.ts:2"]);
    }

    #[test]
    fn a_file_already_too_long_fails_only_when_it_grows() {
        let rules = parse_rules(RULES);
        let same = subject("A.cs", "1\n2\n3\n4\n", &[], Some(4));
        assert!(evaluate(&rules, std::slice::from_ref(&same)).is_empty());
        let grew = subject("A.cs", "1\n2\n3\n4\n5\n", &[5], Some(4));
        assert_eq!(evaluate(&rules, &[grew]).len(), 1);
        let new_file = subject("B.cs", "1\n2\n3\n4\n", &[1, 2, 3, 4], None);
        assert_eq!(evaluate(&rules, &[new_file]).len(), 1);
    }

    #[test]
    fn globs_decide_which_files_a_rule_sees() {
        let rules = parse_rules(RULES);
        let other = subject("src/a.py", "x: any\n", &[1], None);
        assert!(evaluate(&rules, &[other]).is_empty());
    }

    #[test]
    fn added_lines_come_from_the_new_side_of_every_hunk() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -3,0 +4,2 @@\n+x\n+y\n@@ -10 +12 @@\n-a\n+b\n";
        assert_eq!(added_lines(diff).into_iter().collect::<Vec<_>>(), vec![4, 5, 12]);
    }

    #[test]
    fn build_output_and_vendor_folders_are_skipped() {
        assert!(skip("web/node_modules/x/a.ts") && skip("src/obj/a.cs") && skip("a.min.js") && skip("Data/Migrations/1.cs"));
        assert!(!skip("src/Orders/OrderService.cs"));
    }
}
