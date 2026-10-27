// SPDX-License-Identifier: MIT
//! A small `argparse` look-alike: long options with or without a value (`--opt value`, `--opt=value`), unique-prefix abbreviations,
//! positionals anywhere, `--` ends the options, `-h/--help`. Errors print the usage and the message to stderr and exit 2.

use crate::text;
use std::collections::BTreeMap;

pub struct Opt {
    pub long: &'static str,
    pub takes_value: bool,
}

#[derive(Default, Debug)]
pub struct Parsed {
    pub values: BTreeMap<String, String>,
    pub flags: Vec<String>,
    pub positionals: Vec<String>,
}

impl Parsed {
    pub fn value(&self, long: &str) -> Option<&str> {
        self.values.get(long).map(String::as_str)
    }

    pub fn flag(&self, long: &str) -> bool {
        self.flags.iter().any(|f| f == long)
    }
}

pub enum Stop {
    Help,
    Error(String),
}

fn looks_like_negative_number(token: &str) -> bool {
    let body = token.strip_prefix('-').unwrap_or("");
    !body.is_empty() && body.chars().all(|c| c.is_ascii_digit() || c == '.') && body.chars().any(|c| c.is_ascii_digit())
}

fn is_option_token(token: &str) -> bool {
    token.starts_with('-') && token.len() > 1 && !looks_like_negative_number(token)
}

/// Python `int(text)` for a command-line value.
pub fn parse_int(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        return None;
    }
    let clean: String = digits.chars().filter(|c| *c != '_').collect();
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    clean.parse::<i64>().ok().map(|n| n * sign)
}

pub fn parse(specs: &[Opt], argv: &[String]) -> Result<Parsed, Stop> {
    let mut parsed = Parsed::default();
    let mut index = 0;
    let mut only_positionals = false;
    while index < argv.len() {
        let token = &argv[index];
        index += 1;
        if only_positionals || !is_option_token(token) {
            parsed.positionals.push(token.clone());
            continue;
        }
        if token == "--" {
            only_positionals = true;
            continue;
        }
        if token == "-h" || token == "--help" {
            return Err(Stop::Help);
        }
        let (name, inline) = match token.split_once('=') {
            Some((name, value)) if token.starts_with("--") => (name.to_string(), Some(value.to_string())),
            _ => (token.clone(), None),
        };
        if !name.starts_with("--") {
            return Err(Stop::Error(format!("unrecognized arguments: {token}")));
        }
        let candidates: Vec<&Opt> = specs.iter().filter(|o| o.long == &name[2..]).collect();
        let candidates: Vec<&Opt> = if candidates.is_empty() { specs.iter().filter(|o| o.long.starts_with(&name[2..])).collect() } else { candidates };
        let spec = match candidates.as_slice() {
            [one] => *one,
            [] => return Err(Stop::Error(format!("unrecognized arguments: {token}"))),
            many => {
                let names: Vec<String> = many.iter().map(|o| format!("--{}", o.long)).collect();
                return Err(Stop::Error(format!("ambiguous option: {name} could match {}", names.join(", "))));
            }
        };
        if spec.takes_value {
            let value = match inline {
                Some(value) => value,
                None => match argv.get(index) {
                    Some(next) if !is_option_token(next) => {
                        index += 1;
                        next.clone()
                    }
                    _ => return Err(Stop::Error(format!("argument --{}: expected one argument", spec.long))),
                },
            };
            parsed.values.insert(spec.long.to_string(), value);
        } else {
            if inline.is_some() {
                return Err(Stop::Error(format!("argument --{}: ignored explicit argument", spec.long)));
            }
            if !parsed.flag(spec.long) {
                parsed.flags.push(spec.long.to_string());
            }
        }
    }
    Ok(parsed)
}

/// Prints the usage and the message to stderr; the exit code argparse uses.
pub fn fail(prog: &str, usage: &str, message: &str) -> i32 {
    text::err(&format!("usage: {prog} {usage}\n{prog}: error: {message}\n"));
    2
}

/// Parses and handles `--help` (prints `help_text` to stdout, exit 0) and errors (exit 2): `Err(code)` means "stop with this code".
pub fn parse_or_exit(prog: &str, usage: &str, help_text: &str, specs: &[Opt], argv: &[String]) -> Result<Parsed, i32> {
    match parse(specs, argv) {
        Ok(parsed) => Ok(parsed),
        Err(Stop::Help) => {
            text::out(help_text);
            Err(0)
        }
        Err(Stop::Error(message)) => Err(fail(prog, usage, &message)),
    }
}
