// SPDX-License-Identifier: MIT
//! Small text and file helpers shared by the tools (Python behaviours the scripts rely on).

use std::io::Write;
use std::path::Path;

/// Everything printed goes through here so a closed pipe never panics.
pub fn out(text: &str) {
    let _ = std::io::stdout().write_all(text.as_bytes());
}

pub fn err(text: &str) {
    let _ = std::io::stderr().write_all(text.as_bytes());
}

/// `print(...)`.
pub fn println_out(line: &str) {
    out(line);
    out("\n");
}

pub fn flush_all() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// First `n` characters (`text[:n]`).
pub fn head(text: &str, n: usize) -> String {
    text.chars().take(n).collect()
}

/// `"%-Ns" % text` (padding counts characters, like Python).
pub fn pad_right(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - count))
    }
}

/// `open(path, encoding="utf-8", errors="replace").read()` (no newline translation).
pub fn read_lossy(path: &Path) -> std::io::Result<String> {
    Ok(String::from_utf8_lossy(&std::fs::read(path)?).into_owned())
}

/// `open(path, encoding="utf-8-sig").read()`: strict utf-8, a leading BOM dropped.
pub fn read_utf8_sig(path: &Path) -> Result<String, std::io::Error> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text))
}

/// Text mode line iteration: `\r\n` and lone `\r` read as `\n`.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `os.path.abspath` (ntpath rules).
pub fn abspath(path: &str) -> String {
    pycompat::nt_abspath(path)
}

/// `os.path.expanduser` for a leading `~`.
pub fn expanduser(path: &str) -> String {
    if !path.starts_with('~') {
        return path.to_string();
    }
    let rest = &path[1..];
    if !(rest.is_empty() || rest.starts_with('/') || rest.starts_with('\\')) {
        return path.to_string();
    }
    let home = std::env::var("USERPROFILE").ok().filter(|v| !v.is_empty()).or_else(|| {
        let drive = std::env::var("HOMEDRIVE").unwrap_or_default();
        let dir = std::env::var("HOMEPATH").ok()?;
        Some(format!("{drive}{dir}"))
    });
    match home {
        Some(home) => format!("{home}{rest}"),
        None => path.to_string(),
    }
}

/// `os.path.join(a, b)` (ntpath).
pub fn join(a: &str, b: &str) -> String {
    pycompat::nt_join(a, b)
}

/// `os.path.basename` (ntpath: both separators).
pub fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Directory entries in operating-system order (`os.listdir`).
pub fn listdir(path: &Path) -> Vec<String> {
    match std::fs::read_dir(path) {
        Ok(entries) => entries.flatten().filter_map(|e| e.file_name().into_string().ok()).collect(),
        Err(_) => vec![],
    }
}
