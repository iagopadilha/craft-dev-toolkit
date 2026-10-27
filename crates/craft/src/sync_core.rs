// SPDX-License-Identifier: MIT
//! `craft sync_core [--check] [--root DIR]`: injects `shared/rules/core-dev.md` into `skills/backend-dev/SKILL.md` and
//! `skills/frontend-dev/SKILL.md`.
//!
//! Idempotent: the block sits between `<!-- CORE_DEV:BEGIN -->` and `<!-- CORE_DEV:END -->`. If the markers already exist, the content between
//! them is replaced; otherwise the block goes in before the skill's calibration heading. `--check` only reports whether it is out of date
//! (exit 1 if so). The plugin root is the folder that holds `bin/craft.exe` (or `--root`, used when working in the repository: `--root plugin`).

use crate::text;
use pycompat::Re;
use std::path::{Path, PathBuf};

const BEGIN: &str = "<!-- CORE_DEV:BEGIN -->";
const END: &str = "<!-- CORE_DEV:END -->";
const TARGETS: [(&str, &str); 2] = [("backend-dev", r"(?m)^## 7\. Continuous Calibration"), ("frontend-dev", r"(?m)^## 13\. (?:Continuous Calibration|Calibra)")];

/// The plugin this tool lives in (never an installed copy named by an environment variable): it edits the files beside it.
fn default_root() -> PathBuf {
    std::env::current_exe().ok().and_then(|exe| exe.parent().and_then(Path::parent).map(Path::to_path_buf)).unwrap_or_default()
}

fn read(path: &Path) -> std::io::Result<String> {
    Ok(String::from_utf8_lossy(&std::fs::read(path)?).into_owned())
}

/// The block between the markers replaced, or inserted before `anchor`; `None` when there is no marker and no anchor.
pub fn sync_text(text: &str, block: &str, anchor: &Re) -> Option<String> {
    if let Some(begin) = text.find(BEGIN) {
        let after_begin = begin + BEGIN.len();
        // BEGIN, then the shortest run up to END followed by a newline (the Python pattern: BEGIN .*? END \n, dot matches newlines).
        let mut search_from = after_begin;
        while let Some(found) = text[search_from..].find(END) {
            let end_at = search_from + found + END.len();
            if text[end_at..].starts_with('\n') {
                return Some(format!("{}{block}{}", &text[..begin], &text[end_at + 1..]));
            }
            search_from = end_at;
        }
        return Some(text.to_string());
    }
    let start = anchor.search(text)?.start;
    Some(format!("{}{block}\n---\n\n{}", &text[..start], &text[start..]))
}

pub fn main(argv: &[String]) -> i32 {
    let mut check_only = false;
    let mut root: Option<PathBuf> = None;
    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--check" => check_only = true,
            "--root" => {
                index += 1;
                match argv.get(index) {
                    Some(value) => root = Some(PathBuf::from(value)),
                    None => {
                        text::err("sync_core: --root needs a folder\n");
                        return 2;
                    }
                }
            }
            "-h" | "--help" => {
                text::out("Usage: craft sync_core [--check] [--root DIR]\n  injects shared/rules/core-dev.md into skills/backend-dev and skills/frontend-dev (idempotent)\n  --check  only report whether it is out of date; exit 1 if so\n  --root   the plugin folder (default: the folder above bin/)\n");
                return 0;
            }
            _ => {}
        }
        index += 1;
    }
    let root = root.unwrap_or_else(default_root);
    let source = root.join("shared").join("rules").join("core-dev.md");
    let core = match read(&source) {
        Ok(text) => format!("{}\n", text.replace("\r\n", "\n").trim_matches(|c: char| c.is_whitespace())),
        Err(error) => {
            text::err(&format!("sync_core: cannot read {}: {error}\n", source.display()));
            return 1;
        }
    };
    let block = format!("{BEGIN}\n{core}{END}\n");
    let mut stale = false;
    for (skill, anchor) in TARGETS {
        let path = root.join("skills").join(skill).join("SKILL.md");
        let original = match read(&path) {
            Ok(text) => text,
            Err(error) => {
                text::err(&format!("sync_core: cannot read {}: {error}\n", path.display()));
                return 1;
            }
        };
        let newline = if original.contains("\r\n") { "\r\n" } else { "\n" };
        let normalized = original.replace("\r\n", "\n");
        let Some(updated) = sync_text(&normalized, &block, &Re::new(anchor)) else {
            text::println_out(&format!("ERROR: anchor not found in {skill}"));
            return 2;
        };
        if updated != normalized {
            stale = true;
            if !check_only {
                if let Err(error) = std::fs::write(&path, updated.replace('\n', newline).as_bytes()) {
                    text::err(&format!("sync_core: cannot write {}: {error}\n", path.display()));
                    return 1;
                }
            }
            text::println_out(&format!("{} {skill}", if check_only { "out of date:" } else { "updated:" }));
        } else {
            text::println_out(&format!("ok: {skill}"));
        }
    }
    if stale && check_only {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> String {
        format!("{BEGIN}\ncore\n{END}\n")
    }

    #[test]
    fn inserts_before_the_anchor_then_replaces_in_place() {
        let anchor = Re::new(r"(?m)^## 7\. Continuous Calibration");
        let original = "# Skill\n\ntext\n\n## 7. Continuous Calibration\nmore\n";
        let first = sync_text(original, &block(), &anchor).unwrap();
        assert_eq!(first, format!("# Skill\n\ntext\n\n{}\n---\n\n## 7. Continuous Calibration\nmore\n", block()));
        let again = sync_text(&first, &block(), &anchor).unwrap();
        assert_eq!(again, first, "idempotent");
        let changed = sync_text(&first, &format!("{BEGIN}\nnew core\n{END}\n"), &anchor).unwrap();
        assert!(changed.contains("new core") && !changed.contains("\ncore\n"));
    }

    #[test]
    fn no_marker_and_no_anchor_is_an_error() {
        assert!(sync_text("# nothing\n", &block(), &Re::new(r"(?m)^## 7\.")).is_none());
    }
}
