// SPDX-License-Identifier: MIT
//! `craft testenv`: test credentials for every skill and for an orchestrator that runs them. One convention, values only in `.env` files, never in
//! markdown.
//!
//! ONE SOURCE AT RUN TIME: `<project>/.specs/project/.env.test` (git-ignored; this project's own test values). Nothing else is read when a skill
//! runs: not a real environment variable, not a machine-wide or shared `.env`. Values from machine-wide `.env` files reach the project only through
//! `import`, on purpose, one name at a time.
//!
//! TEST USERS are `;`-separated lists, aligned by position (write `\;` for a literal `;` inside a value): `TEST_USER_EMAILS`,
//! `TEST_USER_PASSWORDS`, `TEST_USER_ROLES` (optional). Each user is also exposed as plain variables (`TEST_USER_1_EMAIL`, ... and
//! `TEST_USER_COUNT`), so a command never needs a value written in a document.
//!
//! NETWORK LIST. Credentials may only travel to hosts the project allows with `cred` (`.specs/project/NETWORK.json` and the data dir's
//! `config.json`: `egress_hosts`, `egress_never`, `egress_builtin_read`). The grammar and the judgement are the `egress-policy` crate's.

use crate::args::{self, Opt};
use crate::network::{self, browser_filter_script, effective_network, network_allows};
use crate::text;
use pycompat::{json::py_json_dumps, json::py_json_dumps_indent, Re};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const PROJECT_ENV_RELATIVE: [&str; 3] = [".specs", "project", ".env.test"];
const ENVIRONMENT_DOC_RELATIVE: [&str; 3] = [".specs", "project", "TEST-ENVIRONMENT.md"];
const LIST_SEPARATOR: char = ';';
const ESCAPED_SEPARATOR: &str = "\\;";
const USER_FIELDS: [(&str, &str); 3] = [("EMAILS", "EMAIL"), ("PASSWORDS", "PASSWORD"), ("ROLES", "ROLE")];
const MIN_SECRET_LENGTH: usize = 4;
const MASK: &str = "***";
const EXEC_TIMEOUT_SECONDS: u64 = 600;
const EXIT_REFUSED: i32 = 3;
const EXIT_TIMEOUT: i32 = 124;
const EXIT_NO_SHELL: i32 = 2;
const IGNORE_LINES: [&str; 2] = [".specs/project/.env.test", ".playwright-cli/"];
const ENV_TEMPLATE: &str = "# Test values for this project. Git-ignored: never commit this file. Names only until you fill them in.
# Format: NAME=value (quote values with spaces). Lists use `;` (see testenv.py).
# The names must match the $NAMES listed in .specs/project/TEST-ENVIRONMENT.md.
TEST_USER_EMAILS=
TEST_USER_PASSWORDS=
TEST_USER_ROLES=
ADMIN_TOKEN=
";
const URL_TRAILING: &str = ")]},.;";
const HTTP_CLIENTS: [&str; 6] = ["curl", "wget", "iwr", "irm", "invoke-webrequest", "invoke-restmethod"];
const BROWSER_CLIENT: &str = "playwright-cli";
const BROWSER_GET_COMMANDS: [&str; 3] = ["open", "goto", "tab-new"];
const METHOD_OPTIONS: [&str; 3] = ["-X", "--request", "-method"];
const HEAD_OPTIONS: [&str; 2] = ["-I", "--head"];
const GET_OPTIONS: [&str; 2] = ["-G", "--get"];
const BODY_OPTIONS: [&str; 16] = [
    "-d",
    "--data",
    "--data-raw",
    "--data-binary",
    "--data-urlencode",
    "--data-ascii",
    "-F",
    "--form",
    "-T",
    "--upload-file",
    "--json",
    "--post-data",
    "--post-file",
    "--body-data",
    "-body",
    "-infile",
];
const URL_OPTIONS: [&str; 2] = ["--url", "-uri"];
const OTHER_VALUE_OPTIONS: [&str; 26] = [
    "-H",
    "--header",
    "-o",
    "--output",
    "-u",
    "--user",
    "-A",
    "--user-agent",
    "-e",
    "--referer",
    "-b",
    "--cookie",
    "-c",
    "--cookie-jar",
    "-w",
    "--write-out",
    "-m",
    "--max-time",
    "--connect-timeout",
    "-x",
    "--proxy",
    "--cacert",
    "--cert",
    "--key",
    "-K",
    "--config",
];
const OTHER_VALUE_OPTIONS_MORE: [&str; 9] =
    ["-headers", "-outfile", "-contenttype", "-credential", "-authentication", "-timeoutsec", "-useragent", "-websession", "-proxy"];
const HEADER_FILE_OPTION: &str = "--header-file";
const USAGE: &str =
    "testenv [-h] [--project PROJECT] [--names NAMES] [--json] [--replace] {init,list,check,exec,sources,import,network-filter,network-contract} [args ...]";
const HELP: &str = "usage: testenv [-h] [--project PROJECT] [--names NAMES] [--json] [--replace]
               {init,list,check,exec,sources,import,network-filter,network-contract}
               [args ...]

Test credentials: the project's .env.test, machine sources, network list, exec with redaction

positional arguments:
  {init,list,check,exec,sources,import,network-filter,network-contract}
  args                  exec: the command line (single quotes); import: the NAMEs; network-contract: the fixture file

options:
  -h, --help            show this help message and exit
  --project PROJECT     project folder (the one whose .specs/project/.env.test is read)
  --names NAMES         check: comma-separated names to require
  --json                sources: JSON output
  --replace             import: overwrite a name the project already defines
";

static SECRET_NAME: LazyLock<Re> = LazyLock::new(|| Re::ci(r"(token|secret|passw|credential|private_key|api_?key|auth|pwd)"));
static NAME_REFERENCE: LazyLock<Re> = LazyLock::new(|| Re::new(r"\$\{?([A-Z][A-Z0-9_]{2,})\}?"));
static ANY_REFERENCE: LazyLock<Re> = LazyLock::new(|| Re::new(r"\$\{(?:env:)?([A-Za-z_]\w*)\}|\$(?:env:)?([A-Za-z_]\w*)|%([A-Za-z_]\w*)%"));
static ENV_NAME: LazyLock<Re> = LazyLock::new(|| Re::new(r"^[A-Za-z_][A-Za-z0-9_]*(?=\n?\z)"));
static SCHEME_PREFIX: LazyLock<Re> = LazyLock::new(|| Re::new(r"^[A-Za-z][A-Za-z0-9+.\-]*://"));
static URL_LITERAL: LazyLock<Re> = LazyLock::new(|| Re::ci(r#"\b(?:https?|wss?|ftp|tcp)://[^\s'"<>`]+"#));
static HOST_LIKE: LazyLock<Re> = LazyLock::new(|| Re::new(r"^(?:(?:[A-Za-z0-9_-]+\.)+[A-Za-z0-9_-]+|localhost|\[[0-9A-Fa-f:.]+\])(?::\d+)?(?:/\S*)?(?=\n?\z)"));
static ASSIGNMENT: LazyLock<Re> = LazyLock::new(|| Re::new(r"^[A-Za-z_]\w*="));
static AUTHORITY_UNSAFE: LazyLock<Re> = LazyLock::new(|| Re::new(r"[$%`\\]"));

type Values = BTreeMap<String, String>;

// ---- the single source: <project>/.specs/project/.env.test ----------------------------------------------------------------------

/// `a;b;c` -> ['a','b','c'] (blank items kept so lists stay aligned; `\;` is a literal `;`).
pub fn split_values(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }
    text.replace(ESCAPED_SEPARATOR, "\0").split(LIST_SEPARATOR).map(|part| part.replace('\0', &LIST_SEPARATOR.to_string()).trim().to_string()).collect()
}

fn join_parts(base: &str, parts: &[&str]) -> PathBuf {
    parts.iter().fold(PathBuf::from(base), |path, part| path.join(part))
}

pub fn env_test_path(project: &str) -> PathBuf {
    join_parts(project, &PROJECT_ENV_RELATIVE)
}

/// `(label, path)` of the only file read at run time: the project's `.env.test` (nothing without a project).
fn layer_files(project: Option<&str>) -> Vec<(&'static str, PathBuf)> {
    project.map(|p| vec![("project .env.test", env_test_path(p))]).unwrap_or_default()
}

/// Parses `KEY=value` lines: skips blank/# lines and lines without `=`, drops `export `, strips matching surrounding quotes. Keys in order of
/// first appearance; a later line of the same key wins.
fn parse_env_text(body: &str) -> Vec<(String, String)> {
    let mut values: Vec<(String, String)> = vec![];
    for raw in pycompat::splitlines(body) {
        let mut line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export ") {
            line = rest.trim_start();
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        let (key, mut value) = (key.trim().to_string(), value.trim().to_string());
        let chars: Vec<char> = value.chars().collect();
        if chars.len() >= 2 && chars[0] == chars[chars.len() - 1] && (chars[0] == '\'' || chars[0] == '"') {
            value = chars[1..chars.len() - 1].iter().collect();
        }
        if !key.is_empty() {
            match values.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => values.push((key, value)),
            }
        }
    }
    values
}

fn read_file(path: &Path) -> Vec<(String, String)> {
    match text::read_utf8_sig(path) {
        Ok(body) => parse_env_text(&body),
        Err(_) => vec![],
    }
}

fn read_file_map(path: &Path) -> Values {
    read_file(path).into_iter().collect()
}

/// Adds TEST_USER_<n>_EMAIL/PASSWORD/ROLE and TEST_USER_COUNT derived from the plural lists.
fn add_test_users(values: &mut Values, sources: &mut BTreeMap<String, String>) {
    let columns: Vec<(&str, Vec<String>)> =
        USER_FIELDS.iter().map(|(plural, single)| (*single, split_values(values.get(&format!("TEST_USER_{plural}")).map(String::as_str).unwrap_or("")))).collect();
    let count = columns.iter().map(|(_, items)| items.len()).max().unwrap_or(0);
    for number in 1..=count {
        for (plural, single) in USER_FIELDS {
            let items = &columns.iter().find(|(s, _)| *s == single).expect("column").1;
            if number <= items.len() && !items[number - 1].is_empty() {
                let name = format!("TEST_USER_{number}_{single}");
                let source = sources.get(&format!("TEST_USER_{plural}")).cloned().unwrap_or_else(|| "derived".into());
                values.insert(name.clone(), items[number - 1].clone());
                sources.insert(name, source);
            }
        }
    }
    values.insert("TEST_USER_COUNT".into(), count.to_string());
    sources.insert("TEST_USER_COUNT".into(), "derived".into());
}

/// `(values, sources)`: every key of the project's `.env.test` with its label. The real environment is NOT consulted.
pub fn load_layers(project: Option<&str>) -> (Values, BTreeMap<String, String>) {
    let (mut values, mut sources) = (Values::new(), BTreeMap::new());
    for (label, path) in layer_files(project) {
        for (key, value) in read_file(&path) {
            if !value.is_empty() {
                // a blank value counts as not defined (same as the app)
                values.insert(key.clone(), value);
                sources.insert(key, label.to_string());
            }
        }
    }
    add_test_users(&mut values, &mut sources);
    (values, sources)
}

pub fn is_secret_name(name: &str) -> bool {
    SECRET_NAME.search(name).is_some()
}

fn secret_values(values: &Values) -> Vec<String> {
    let mut found: Vec<String> =
        values.iter().filter(|(name, value)| is_secret_name(name) && value.chars().count() >= MIN_SECRET_LENGTH).map(|(_, v)| v.clone()).collect();
    found.sort();
    found.dedup();
    found.sort_by_key(|value| std::cmp::Reverse(value.chars().count()));
    found
}

pub fn redact(text: &str, values: &Values) -> String {
    secret_values(values).iter().fold(text.to_string(), |acc, secret| acc.replace(secret.as_str(), MASK))
}

/// Every $NAME the project's TEST-ENVIRONMENT.md mentions.
fn referenced_names(project: &str) -> Vec<String> {
    let Ok(bytes) = std::fs::read(join_parts(project, &ENVIRONMENT_DOC_RELATIVE)) else { return vec![] };
    let Ok(body) = String::from_utf8(bytes) else { return vec![] };
    let mut names: Vec<String> = NAME_REFERENCE.find_all(&body).iter().filter_map(|m| m.group(1).map(String::from)).collect();
    names.sort();
    names.dedup();
    names
}

fn missing_names(project: Option<&str>, names: Option<Vec<String>>) -> Vec<String> {
    let (values, _) = load_layers(project);
    let wanted = names.unwrap_or_else(|| project.map(referenced_names).unwrap_or_default());
    wanted.into_iter().filter(|name| values.get(name).is_none_or(|v| v.is_empty())).collect()
}

/// Names with a non-empty value in the project's `.env.test` (derived TEST_USER_n_* excluded).
fn defined_names(project: Option<&str>) -> Vec<String> {
    let Some(project) = project else { return vec![] };
    let mut names: Vec<String> = read_file(&env_test_path(project)).into_iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| k).collect();
    names.sort();
    names
}

// ---- machine sources and import (the only way a machine-wide value reaches a project) ------------------------------------------------

/// `env_sources` of `<data dir>/config.json` (`~` allowed); default: the `.env` of the data dir. Repeated paths are read once.
fn source_paths() -> Vec<String> {
    let folder = network::data_dir();
    let (config, _) = network::read_json(&Path::new(&folder).join(network::CONFIG_FILE));
    let configured: Vec<String> = match config.as_ref().and_then(|c| c.get("env_sources")) {
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).filter(|s| !s.trim().is_empty()).map(String::from).collect(),
        _ => vec![],
    };
    let entries = if configured.is_empty() { vec![text::join(&folder, ".env")] } else { configured };
    let (mut paths, mut seen) = (vec![], vec![]);
    for item in entries {
        let path = text::expanduser(item.trim());
        let key = pycompat::nt_normcase(&text::abspath(&path));
        if !seen.contains(&key) {
            seen.push(key);
            paths.push(path);
        }
    }
    paths
}

fn source_report() -> Value {
    Value::Array(
        source_paths()
            .into_iter()
            .map(|path| {
                let mut names: Vec<String> = read_file(Path::new(&path)).into_iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| k).collect();
                names.sort();
                json!({"path": path, "exists": Path::new(&path).is_file(), "names": names})
            })
            .collect(),
    )
}

/// The `.env` text of a value, quoted only when its plain form would not read back the same.
fn quote_value(value: &str) -> Result<String, String> {
    let plain = value.is_empty() || (value == value.trim() && !value.chars().any(|c| c.is_whitespace() || c == '#' || c == '\'' || c == '"'));
    if plain {
        return Ok(value.to_string());
    }
    for mark in ['\'', '"'] {
        if !value.contains(mark) {
            return Ok(format!("{mark}{value}{mark}"));
        }
    }
    let chars: Vec<char> = value.chars().collect();
    if chars[0] == chars[chars.len() - 1] && (chars[0] == '\'' || chars[0] == '"') {
        return Err("a value with both quote kinds at its ends cannot be stored in a .env file".into());
    }
    Ok(value.to_string())
}

/// Sets `NAME=value` for each update in a `.env` file: existing lines are replaced in place, new names are appended. Atomic.
fn write_env_values(path: &Path, updates: &[(String, String)]) -> Result<(), String> {
    let existing = text::read_utf8_sig(path).unwrap_or_default();
    let mut pending: Vec<(String, String)> = updates.to_vec();
    let mut result: Vec<String> = vec![];
    for line in pycompat::splitlines(&existing) {
        let stripped = line.trim();
        let body = stripped.strip_prefix("export ").map(str::trim_start).unwrap_or(stripped);
        let name = if body.contains('=') && !body.starts_with('#') { body.split('=').next().unwrap_or("").trim() } else { "" };
        if updates.iter().any(|(n, _)| n == name) {
            if let Some(at) = pending.iter().position(|(n, _)| n == name) {
                let (n, v) = pending.remove(at);
                result.push(format!("{n}={}", quote_value(&v)?));
            }
            continue; // a repeated line of an updated name is dropped
        }
        result.push(line.to_string());
    }
    for (name, value) in pending {
        result.push(format!("{name}={}", quote_value(&value)?));
    }
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let temporary = tempfile::Builder::new().prefix(".env.test.").suffix(".tmp").tempfile_in(folder).map_err(|e| e.to_string())?;
    std::fs::write(temporary.path(), format!("{}\n", result.join("\n")).as_bytes()).map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

// ---- exec: credentials only to hosts that carry `cred` ---------------------------------------------------------------------------

/// Command text -> list of token lists, one per simple command (split on `;`, `|`, `&&`, `||`, a spaced `&` and newlines; quotes are removed, a
/// quoted operator stays inside its token). Best effort for the credential check, not a shell parser.
pub fn split_segments(command: &str) -> Vec<Vec<String>> {
    let chars: Vec<char> = command.chars().collect();
    let mut segments: Vec<Vec<String>> = vec![];
    let mut tokens: Vec<String> = vec![];
    let mut word = String::new();
    let mut has_word = false;
    let mut quote = '\0';
    let at = |i: usize| chars.get(i).copied();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let end_word = |tokens: &mut Vec<String>, word: &mut String, has_word: &mut bool| {
            if !word.is_empty() || *has_word {
                tokens.push(std::mem::take(word));
            }
            *has_word = false;
        };
        if quote != '\0' {
            if c == quote {
                quote = '\0';
            } else if c == '\\' && quote == '"' && at(index + 1).is_some_and(|n| matches!(n, '"' | '\\' | '$' | '`')) {
                index += 1;
                word.push(chars[index]);
            } else {
                word.push(c);
            }
        } else if c == '\'' || c == '"' {
            quote = c;
            has_word = true;
        } else if c == '\\' && matches!(at(index + 1), Some('\n') | Some('\r')) {
            // line continuation
            index += if at(index + 1) == Some('\r') && at(index + 2) == Some('\n') { 2 } else { 1 };
            end_word(&mut tokens, &mut word, &mut has_word);
        } else if c == ' ' || c == '\t' || c == '\r' {
            end_word(&mut tokens, &mut word, &mut has_word);
        } else if c == ';' || c == '|' || c == '\n' || (c == '&' && (at(index + 1) == Some('&') || word.is_empty())) {
            end_word(&mut tokens, &mut word, &mut has_word);
            if !tokens.is_empty() {
                segments.push(std::mem::take(&mut tokens));
            }
        } else {
            word.push(c);
        }
        index += 1;
    }
    if !word.is_empty() || has_word {
        tokens.push(word);
    }
    if !tokens.is_empty() {
        segments.push(tokens);
    }
    segments
}

/// Fills `$NAME`, `${NAME}`, `$env:NAME` and `%NAME%` with the project's values; an undefined name stays as written.
pub fn expand_references(text: &str, values: &Values) -> String {
    ANY_REFERENCE.replace_all(text, |m| {
        let name = (1..=3).find_map(|g| m.group(g)).unwrap_or("");
        values.get(name).cloned().unwrap_or_else(|| m.text().to_string())
    })
}

/// `example.com/x` / `https://example.com/x` -> a URL text, or '' when the token is not a target.
fn candidate_url(text: &str) -> String {
    if SCHEME_PREFIX.is_match(text) {
        return text.to_string();
    }
    if HOST_LIKE.is_match(text) {
        format!("http://{text}")
    } else {
        String::new()
    }
}

fn in_list(list: &[&str], name: &str) -> bool {
    list.contains(&name)
}

/// `[(method, target)]` of one curl/wget/iwr/irm command (options that take a value are skipped, so a header or a body is never a target).
fn client_requests(tokens: &[String]) -> Vec<(String, String)> {
    let (mut method, mut head, mut body, mut get) = (String::new(), false, false, false);
    let mut targets: Vec<String> = vec![];
    let mut index = 1;
    while index < tokens.len() {
        let token = &tokens[index];
        index += 1;
        let key = if token.chars().count() == 2 { token.clone() } else { token.to_lowercase() };
        let (name, equals, inline) = if key.starts_with("--") {
            match key.split_once('=') {
                Some((n, v)) => (n.to_string(), true, v.to_string()),
                None => (key.clone(), false, String::new()),
            }
        } else {
            (key.clone(), false, String::new())
        };
        let mut value: Option<String> = if equals { Some(inline) } else { None };
        if !key.starts_with('-') {
            if !candidate_url(token).is_empty() || token.contains('$') || token.contains('%') {
                targets.push(token.clone());
            }
            continue;
        }
        if token.starts_with("-X") && token.chars().count() > 2 {
            method = token.chars().skip(2).collect::<String>().to_uppercase();
            continue;
        }
        let takes_value = in_list(&URL_OPTIONS, &name)
            || in_list(&METHOD_OPTIONS, &name)
            || in_list(&BODY_OPTIONS, &name)
            || in_list(&OTHER_VALUE_OPTIONS, &name)
            || in_list(&OTHER_VALUE_OPTIONS_MORE, &name)
            || name == HEADER_FILE_OPTION;
        if takes_value && value.is_none() {
            value = Some(tokens.get(index).cloned().unwrap_or_default());
            index += 1;
        }
        if in_list(&URL_OPTIONS, &name) {
            targets.push(value.unwrap_or_default());
        } else if in_list(&METHOD_OPTIONS, &name) {
            method = value.unwrap_or_default().to_uppercase();
        } else if in_list(&BODY_OPTIONS, &name) {
            body = true;
        } else if in_list(&HEAD_OPTIONS, &name) {
            head = true;
        } else if in_list(&GET_OPTIONS, &name) {
            get = true;
        }
    }
    if method.is_empty() {
        method = if head {
            "HEAD"
        } else if body && !get {
            "POST"
        } else {
            "GET"
        }
        .to_string();
    }
    targets.into_iter().map(|target| (method.clone(), target)).collect()
}

/// `[(method, target)]` for every URL the command would reach: what curl/wget/iwr/irm are given, `playwright-cli open|goto|tab-new <url>` (GET)
/// and any URL literal elsewhere (method `*`: unknown). A target is raw text, `$NAME` references not yet filled.
pub fn command_requests(command: &str) -> Vec<(String, String)> {
    let mut requests: Vec<(String, String)> = vec![];
    for tokens in split_segments(command) {
        let mut start = 0;
        while start < tokens.len() && ASSIGNMENT.match_start(&tokens[start]).is_some() {
            start += 1;
        }
        let words = &tokens[start..];
        let mut program = words.first().map(|w| text::basename(w).to_lowercase()).unwrap_or_default();
        if let Some(stripped) = program.strip_suffix(".exe") {
            program = stripped.to_string();
        }
        if in_list(&HTTP_CLIENTS, &program) {
            requests.extend(client_requests(words));
            continue;
        }
        let subcommand = words.iter().skip(1).find(|w| !w.starts_with('-')).cloned().unwrap_or_default();
        let method = if program == BROWSER_CLIENT && in_list(&BROWSER_GET_COMMANDS, &subcommand) { "GET" } else { egress_policy::UNKNOWN_METHOD };
        for token in words.iter().skip(1) {
            for found in URL_LITERAL.find_all(token) {
                requests.push((method.to_string(), found.text().trim_end_matches(|c| URL_TRAILING.contains(c)).to_string()));
            }
        }
    }
    requests
}

/// Names of the secret-looking variables the command mentions (by name only, whether or not they are defined).
fn secret_references(command: &str) -> Vec<String> {
    let mut names: Vec<String> =
        ANY_REFERENCE.find_all(command).iter().filter_map(|m| (1..=3).find_map(|g| m.group(g)).map(String::from)).filter(|n| is_secret_name(n)).collect();
    names.sort();
    names.dedup();
    names
}

/// `None` when the command may carry the project's secrets, else the refusal text (host and hint; never a value).
pub fn check_credential_targets(command: &str, values: &Values, project: Option<&str>) -> Option<String> {
    if secret_references(command).is_empty() {
        return None;
    }
    let mut effective: Option<egress_policy::Effective> = None;
    for (method, raw) in command_requests(command) {
        let url = candidate_url(&expand_references(&raw, values));
        let authority: String = if url.is_empty() {
            String::new()
        } else {
            let after = &url[url.find("://").map(|at| at + 3).unwrap_or(0)..];
            after.split(['/', '?', '#']).next().unwrap_or("").to_string()
        };
        if url.is_empty() || AUTHORITY_UNSAFE.search(&authority).is_some() {
            return Some(format!("cannot tell which host `{}` is (a variable that is not in .env.test, or a sub-shell); write the host out", redact(&raw, values)));
        }
        let list = effective.get_or_insert_with(|| effective_network(project, None, &defined_names(project)));
        let (allowed, reason) = network_allows(list, &method, &url, true, None);
        if !allowed {
            let host = egress_policy::parse_request_url(&url).map(|parts| parts.1).unwrap_or(authority);
            return Some(format!(
                "host {host} may not receive credentials ({}). Run nothing. Ask the user to list it with `cred` in the project's {} (or in `egress_hosts` of the data dir's config.json); never send the secret another way.",
                redact(&reason, values),
                egress_policy::NETWORK_LABEL
            ));
        }
    }
    None
}

// ---- commands ----------------------------------------------------------------------------------------------------------------------

fn which(program: &str) -> Option<PathBuf> {
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into()).split(';').filter(|e| !e.is_empty()).map(|e| e.to_lowercase()).collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        for extension in &extensions {
            let candidate = dir.join(format!("{program}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if cfg!(windows) && dir.join(program).is_file() && program.contains('.') {
            return Some(dir.join(program));
        }
    }
    None
}

/// argv prefix of a POSIX shell, or `None`. On Windows only Git Bash qualifies: `bash.exe` in System32 / WindowsApps is the WSL launcher (another
/// file system, no Windows environment), so it is never picked.
pub fn find_shell() -> Option<Vec<String>> {
    if !cfg!(windows) {
        return ["bash", "sh"].iter().find_map(|name| which(name)).map(|found| vec![found.display().to_string(), "-c".into()]);
    }
    let mut candidates: Vec<PathBuf> = vec![];
    if let Ok(root) = std::env::var("GIT_INSTALL_ROOT") {
        if !root.is_empty() {
            candidates.push(join_parts(&root, &["bin", "bash.exe"]));
        }
    }
    for variable in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Ok(root) = std::env::var(variable) {
            if !root.is_empty() {
                candidates.push(join_parts(&root, &["Git", "bin", "bash.exe"]));
            }
        }
    }
    if let Some(found) = which("bash") {
        candidates.push(found);
    }
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let system32 = format!("{system_root}\\System32").to_lowercase();
    for candidate in candidates {
        let lowered = pycompat::nt_normpath(&candidate.display().to_string()).to_lowercase();
        if candidate.is_file() && !lowered.contains(&system32) && !lowered.contains("\\windowsapps\\") {
            return Some(vec![candidate.display().to_string(), "-c".into()]);
        }
    }
    None
}

/// Appends the missing `lines` to `<project>/.gitignore` (created when absent). Idempotent; returns the lines it added.
pub fn ensure_gitignore(project: &str, lines: &[&str]) -> Vec<String> {
    let path = Path::new(project).join(".gitignore");
    let mut body = String::from_utf8_lossy(&std::fs::read(&path).unwrap_or_default()).into_owned();
    let present: Vec<String> = pycompat::splitlines(&body).iter().map(|l| l.trim().trim_start_matches('/').to_string()).collect();
    let missing: Vec<String> =
        lines.iter().filter(|line| !present.iter().any(|p| p == *line) && !present.iter().any(|p| p == line.trim_end_matches('/'))).map(|s| s.to_string()).collect();
    if !missing.is_empty() {
        let newline = if body.contains("\r\n") { "\r\n" } else { "\n" };
        if !body.is_empty() && !body.ends_with('\n') {
            body.push_str(newline);
        }
        body.push_str(&missing.join(newline));
        body.push_str(newline);
        let _ = std::fs::write(&path, body.as_bytes());
    }
    missing
}

/// Restricts the file to the current user (the Windows counterpart of chmod 600). Best effort: a failure leaves the default permissions.
fn restrict_to_owner(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(user) = std::env::var("USERNAME") {
            let _ = Command::new("icacls")
                .arg(path)
                .args(["/inheritance:r", "/grant:r"])
                .arg(format!("{user}:(F)"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x0800_0000)
                .status();
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

fn command_init(project: Option<&str>) -> i32 {
    let Some(project) = project else {
        text::err("init needs --project\n");
        return 2;
    };
    let target = env_test_path(project);
    if target.exists() {
        text::println_out(&format!("kept: {} (already exists; nothing overwritten)", target.display()));
    } else {
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(&target, ENV_TEMPLATE.as_bytes()) {
            text::err(&format!("testenv: cannot create {}: {error}\n", target.display()));
            return 1;
        }
        restrict_to_owner(&target);
        text::println_out(&format!("created: {} (names only; fill the values yourself)", target.display()));
    }
    let added = ensure_gitignore(project, &IGNORE_LINES);
    text::println_out(&if added.is_empty() { "gitignore: already has the entries".to_string() } else { format!("gitignore: added {}", added.join(", ")) });
    0
}

fn command_list(project: Option<&str>) -> i32 {
    let (values, sources) = load_layers(project);
    if project.is_none() {
        text::println_out("no --project: nothing to read (only the project's .specs/project/.env.test is read)");
    }
    for (label, path) in layer_files(project) {
        text::println_out(&format!("{} {} {}", text::pad_right(label, 20), if path.is_file() { "found  " } else { "missing" }, path.display()));
    }
    text::println_out("");
    for (name, value) in &values {
        let shown = if is_secret_name(name) && !value.is_empty() { MASK } else { value.as_str() };
        text::println_out(&format!("  {} = {} ({})", text::pad_right(name, 28), text::pad_right(shown, 32), sources[name]));
    }
    0
}

fn command_check(project: Option<&str>, names: Option<Vec<String>>) -> i32 {
    let missing = missing_names(project, names);
    for name in &missing {
        text::println_out(&format!("MISSING {name}"));
    }
    if missing.is_empty() {
        text::println_out("ok: every name is defined");
    }
    i32::from(!missing.is_empty())
}

/// `\r\n` and lone `\r` read as `\n` (text mode).
fn decode_output(bytes: &[u8]) -> String {
    text::universal_newlines(&String::from_utf8_lossy(bytes))
}

fn read_all(mut stream: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buffer = vec![];
        let _ = stream.read_to_end(&mut buffer);
        buffer
    })
}

fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill").args(["/F", "/T", "/PID", &pid.to_string()]).stdout(Stdio::null()).stderr(Stdio::null()).status();
}

fn command_exec(project: Option<&str>, command: &str) -> i32 {
    let (values, _) = load_layers(project);
    if let Some(refusal) = check_credential_targets(command, &values, project) {
        text::err(&format!("testenv: refused: {refusal}\n"));
        return EXIT_REFUSED;
    }
    let Some(shell) = find_shell() else {
        text::err("testenv: no POSIX shell found. On Windows install Git for Windows (Git Bash); the WSL bash in System32 is not used.\n");
        return EXIT_NO_SHELL;
    };
    let mut process = Command::new(&shell[0]);
    process.args(&shell[1..]).arg(command).envs(values.iter()).stdin(Stdio::inherit()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(project) = project {
        process.current_dir(project);
    }
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => {
            text::err(&format!("testenv: cannot start {}: {error}\n", shell[0]));
            return EXIT_NO_SHELL;
        }
    };
    let stdout = read_all(child.stdout.take().expect("piped"));
    let stderr = read_all(child.stderr.take().expect("piped"));
    let deadline = Instant::now() + Duration::from_secs(EXEC_TIMEOUT_SECONDS);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if Instant::now() >= deadline {
            kill_tree(child.id());
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (out, err) = (stdout.join().unwrap_or_default(), stderr.join().unwrap_or_default());
    match status {
        Some(done) => {
            text::out(&redact(&decode_output(&out), &values));
            text::err(&redact(&decode_output(&err), &values));
            done.code().unwrap_or(1)
        }
        None => {
            text::out(&redact(&String::from_utf8_lossy(&out), &values));
            text::err(&redact(&String::from_utf8_lossy(&err), &values));
            text::err(&format!("testenv: command killed after {EXEC_TIMEOUT_SECONDS} s (exit {EXIT_TIMEOUT})\n"));
            EXIT_TIMEOUT
        }
    }
}

fn command_sources(as_json: bool) -> i32 {
    let report = source_report();
    if as_json {
        text::println_out(&py_json_dumps_indent(&report, true, 2));
        return 0;
    }
    for source in report.as_array().into_iter().flatten() {
        text::println_out(&format!("{} ({})", source["path"].as_str().unwrap_or(""), if source["exists"].as_bool().unwrap_or(false) { "found" } else { "missing" }));
        for name in source["names"].as_array().into_iter().flatten() {
            text::println_out(&format!("  {}", name.as_str().unwrap_or("")));
        }
    }
    0
}

fn command_import(project: Option<&str>, names: &[String], replace: bool) -> i32 {
    let Some(project) = project else {
        text::err("import needs --project\n");
        return 2;
    };
    let bad: Vec<&String> = names.iter().filter(|name| ENV_NAME.match_start(name).is_none()).collect();
    if !bad.is_empty() || names.is_empty() {
        let list = if bad.is_empty() { String::new() } else { format!(": {}", bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")) };
        text::err(&format!("import needs one or more variable NAMEs (letters, digits, `_`){list}\n"));
        return 2;
    }
    let target = env_test_path(project);
    let existing = read_file_map(&target);
    let sources: Vec<Values> = source_paths().iter().map(|path| read_file_map(Path::new(path))).collect();
    let mut updates: Vec<(String, String)> = vec![];
    let mut missing = false;
    for name in names {
        let value = sources.iter().find_map(|source| source.get(name).filter(|v| !v.is_empty()).cloned()).unwrap_or_default();
        if value.is_empty() {
            text::println_out(&format!("not found: {name}"));
            missing = true;
        } else if existing.get(name).is_some_and(|v| !v.is_empty()) && !replace {
            text::println_out(&format!("kept: {name} (already in .env.test; --replace overwrites)"));
        } else {
            updates.retain(|(n, _)| n != name);
            updates.push((name.clone(), value));
        }
    }
    if !updates.is_empty() {
        if let Err(error) = write_env_values(&target, &updates) {
            text::err(&format!("import failed: {error}\n"));
            return 2;
        }
    }
    for (name, _) in &updates {
        text::println_out(&format!("imported: {name}"));
    }
    let added = ensure_gitignore(project, &IGNORE_LINES);
    if !added.is_empty() {
        text::println_out(&format!("gitignore: added {}", added.join(", ")));
    }
    i32::from(missing)
}

fn command_network_filter(project: Option<&str>) -> i32 {
    let effective = effective_network(project, None, &defined_names(project));
    for note in &effective.notes {
        text::err(&format!("testenv: {note}\n"));
    }
    text::out(&browser_filter_script(&effective));
    0
}

fn command_network_contract(path: &str) -> i32 {
    let run = || -> Result<Value, String> {
        let body = text::read_utf8_sig(Path::new(path)).map_err(|e| e.to_string())?;
        let fixture: Value = serde_json::from_str(&body).map_err(|e| format!("JSONDecodeError: {e}"))?;
        network::run_contract(&fixture)
    };
    match run() {
        Ok(result) => {
            text::println_out(&py_json_dumps(&result, true));
            i32::from(!result["failed"].as_array().is_some_and(Vec::is_empty))
        }
        Err(error) => {
            text::err(&format!("network-contract: cannot run {path} ({error})\n"));
            2
        }
    }
}

pub fn main(argv: &[String]) -> i32 {
    let specs = [
        Opt { long: "project", takes_value: true },
        Opt { long: "names", takes_value: true },
        Opt { long: "json", takes_value: false },
        Opt { long: "replace", takes_value: false },
    ];
    let parsed = match args::parse_or_exit("testenv", USAGE, HELP, &specs, argv) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let Some((action, rest)) = parsed.positionals.split_first() else {
        return args::fail("testenv", USAGE, "the following arguments are required: action");
    };
    const ACTIONS: [&str; 8] = ["init", "list", "check", "exec", "sources", "import", "network-filter", "network-contract"];
    if !ACTIONS.contains(&action.as_str()) {
        return args::fail(
            "testenv",
            USAGE,
            &format!("argument action: invalid choice: '{action}' (choose from {})", ACTIONS.iter().map(|a| format!("'{a}'")).collect::<Vec<_>>().join(", ")),
        );
    }
    let project_text = parsed.value("project").filter(|p| !p.is_empty()).map(text::abspath);
    let project = project_text.as_deref();
    match action.as_str() {
        "init" => command_init(project),
        "list" => command_list(project),
        "check" => {
            let names: Vec<String> = parsed.value("names").unwrap_or("").split(',').map(str::trim).filter(|n| !n.is_empty()).map(String::from).collect();
            command_check(project, if names.is_empty() { None } else { Some(names) })
        }
        "sources" => command_sources(parsed.flag("json")),
        "import" => command_import(project, rest, parsed.flag("replace")),
        "network-filter" => command_network_filter(project),
        "network-contract" => {
            if rest.len() != 1 {
                return args::fail("testenv", USAGE, "network-contract needs the fixture file");
            }
            command_network_contract(&rest[0])
        }
        _ => {
            if rest.len() != 1 {
                return args::fail("testenv", USAGE, "exec needs the command line as ONE quoted argument");
            }
            command_exec(project, &rest[0])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> Values {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn lists_split_on_semicolons_and_keep_blanks_aligned() {
        assert_eq!(split_values("a;;c"), vec!["a", "", "c"]);
        assert_eq!(split_values(r"a\;b;c"), vec!["a;b", "c"]);
        assert!(split_values("").is_empty());
    }

    #[test]
    fn test_users_are_derived_from_the_plural_lists() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().display().to_string();
        let path = env_test_path(&project);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "TEST_USER_EMAILS=a@example.com;b@example.com\nTEST_USER_PASSWORDS=pw-one;pw-two\nADMIN_TOKEN=\n").unwrap();
        let (loaded, _) = load_layers(Some(&project));
        assert_eq!(loaded["TEST_USER_COUNT"], "2");
        assert_eq!(loaded["TEST_USER_2_EMAIL"], "b@example.com");
        assert!(!loaded.contains_key("ADMIN_TOKEN"), "a blank value is not defined");
        assert!(!loaded.contains_key("TEST_USER_1_ROLE"));
    }

    #[test]
    fn secrets_are_masked_longest_first() {
        let v = values(&[("API_TOKEN", "abcd1234"), ("DB_PASSWORD", "abcd"), ("NAME", "plain-value")]);
        assert_eq!(redact("x abcd1234 y abcd z plain-value", &v), "x *** y *** z plain-value");
    }

    #[test]
    fn commands_split_into_segments_with_quotes_removed() {
        assert_eq!(
            split_segments(r#"curl -H "X: $T" https://a.example.com/x; echo 'a;b' && ls"#),
            vec![vec!["curl", "-H", "X: $T", "https://a.example.com/x"], vec!["echo", "a;b"], vec!["ls"]]
        );
    }

    #[test]
    fn requests_come_from_clients_browsers_and_url_literals() {
        assert_eq!(command_requests("curl -X POST -d 'x' https://api.example.com/v1"), vec![("POST".to_string(), "https://api.example.com/v1".to_string())]);
        assert_eq!(command_requests("curl -I example.com/a"), vec![("HEAD".to_string(), "example.com/a".to_string())]);
        assert_eq!(command_requests("playwright-cli open https://app.example.com/x"), vec![("GET".to_string(), "https://app.example.com/x".to_string())]);
        assert_eq!(command_requests("node run.js --target https://svc.example.com/z."), vec![("*".to_string(), "https://svc.example.com/z".to_string())]);
    }

    #[test]
    fn a_secret_may_only_go_to_a_host_listed_with_cred() {
        let dir = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::env::set_var(network::DATA_DIR_VARIABLE, data.path());
        let project = dir.path().display().to_string();
        let v = values(&[("ADMIN_TOKEN", "s3cr3t-value")]);
        let command = r#"curl -H "X-Token: $ADMIN_TOKEN" https://api.example.com/v1"#;
        let refusal = check_credential_targets(command, &v, Some(&project)).expect("refused");
        assert!(refusal.starts_with("host api.example.com may not receive credentials (GET api.example.com:443/v1 is not in the network list)."), "{refusal}");
        let network = Path::new(&project).join(".specs").join("project").join("NETWORK.json");
        std::fs::create_dir_all(network.parent().unwrap()).unwrap();
        std::fs::write(&network, r#"{"version":1,"allow":["api.example.com GET cred"]}"#).unwrap();
        assert_eq!(check_credential_targets(command, &v, Some(&project)), None);
        assert!(check_credential_targets("curl -H \"X: $ADMIN_TOKEN\" https://$HOST/x", &v, Some(&project)).unwrap().starts_with("cannot tell which host"));
    }

    #[test]
    fn env_values_are_quoted_only_when_needed() {
        assert_eq!(quote_value("plain").unwrap(), "plain");
        assert_eq!(quote_value("has space").unwrap(), "'has space'");
        assert_eq!(quote_value("it's").unwrap(), "\"it's\"");
        assert_eq!(quote_value("").unwrap(), "");
    }

    #[test]
    fn writing_env_values_replaces_in_place_and_appends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env.test");
        std::fs::write(&path, "# c\nA=1\nexport B=2\nA=3\n").unwrap();
        write_env_values(&path, &[("A".into(), "x y".into()), ("C".into(), "9".into())]).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# c\nA='x y'\nexport B=2\nC=9\n");
    }

    #[test]
    fn gitignore_entries_are_added_once() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().display().to_string();
        assert_eq!(ensure_gitignore(&project, &IGNORE_LINES).len(), 2);
        assert!(ensure_gitignore(&project, &IGNORE_LINES).is_empty());
    }
}
