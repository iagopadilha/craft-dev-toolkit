// SPDX-License-Identifier: MIT
//! The network list of a project as `testenv` reads and enforces it: where the list comes from, the judgement with the wording `testenv` prints,
//! the Playwright browser filter and the contract check. The grammar and the matching come from the shared `egress-policy` crate.

use crate::text;
use egress_policy::{
    host_matches, parse_entry, parse_request_url, path_matches, Effective, Entry, ANY_METHOD, NETWORK_LABEL, ORIGIN_LOCAL, ORIGIN_PROJECT, TCP, UNKNOWN_METHOD,
};
use serde_json::{json, Map, Value};
use std::path::Path;

pub const DATA_DIR_VARIABLE: &str = "LOOP_DATA_DIR";
pub const DEFAULT_DATA_DIR_NAME: &str = ".loop-harness";
pub const CONFIG_FILE: &str = "config.json";
const NETWORK_RELATIVE: [&str; 3] = [".specs", "project", "NETWORK.json"];
const BROWSER_FILTER_TEMPLATE: &str = include_str!("browser_filter.js.tmpl");
const POLICY_PLACEHOLDER: &str = "@@POLICY@@";

pub fn data_dir() -> String {
    match std::env::var(DATA_DIR_VARIABLE) {
        Ok(value) if !value.is_empty() => value,
        _ => text::join(&text::expanduser("~"), DEFAULT_DATA_DIR_NAME),
    }
}

/// `(content, error)`: `(None, "")` when the file does not exist; an error class name when it exists but is not usable JSON.
pub fn read_json(path: &Path) -> (Option<Value>, String) {
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(body) => match serde_json::from_str::<Value>(body.strip_prefix('\u{feff}').unwrap_or(&body)) {
                Ok(value) => (Some(value), String::new()),
                Err(_) => (None, "JSONDecodeError".into()),
            },
            Err(_) => (None, "UnicodeDecodeError".into()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, String::new()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => (None, "PermissionError".into()),
        Err(_) => (None, "OSError".into()),
    }
}

fn as_bool(value: Option<&Value>, default: bool) -> bool {
    match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) if matches!(text.trim().to_lowercase().as_str(), "true" | "false") => text.trim().to_lowercase() == "true",
        _ => default,
    }
}

/// Python truthiness of an optional JSON value.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
    }
}

/// Effective list of a project, read from disk. Both list files are optional; one that exists but cannot be used sets `broken` (every non-local
/// request then fails closed).
pub fn effective_network(project: Option<&str>, folder: Option<&str>, defined_names: &[String]) -> Effective {
    let folder = folder.map(String::from).unwrap_or_else(data_dir);
    let mut notes: Vec<String> = vec![];
    let mut broken = String::new();
    let (config, error) = read_json(&Path::new(&folder).join(CONFIG_FILE));
    let config: Map<String, Value> = match (config, error.is_empty()) {
        (Some(Value::Object(map)), true) => map,
        (None, true) => Map::new(),
        (other, _) => {
            broken = CONFIG_FILE.to_string();
            let why = if error.is_empty() {
                if other.is_some() {
                    "not a JSON object".to_string()
                } else {
                    "not a JSON object".to_string()
                }
            } else {
                error
            };
            notes.push(format!("{CONFIG_FILE} is unusable ({why}): every non-local request fails closed"));
            Map::new()
        }
    };
    let mut texts: Vec<Value> = vec![];
    if let Some(project) = project {
        let mut path = Path::new(project).to_path_buf();
        for part in NETWORK_RELATIVE {
            path.push(part);
        }
        let (content, error) = read_json(&path);
        let shape_ok = match &content {
            None => true,
            Some(Value::Object(map)) => matches!(map.get("allow"), None | Some(Value::Array(_))),
            Some(_) => false,
        };
        if !error.is_empty() || !shape_ok {
            if broken.is_empty() {
                broken = NETWORK_LABEL.to_string();
            }
            let why = if error.is_empty() { "need an object with an `allow` list".to_string() } else { error };
            notes.push(format!("{NETWORK_LABEL} is unusable ({why}): every non-local request fails closed"));
        } else if let Some(Value::Object(map)) = content {
            if let Some(Value::Array(items)) = map.get("allow") {
                texts = items.clone();
            }
        }
    }
    let denies: Vec<Value> = match config.get("egress_never") {
        Some(Value::Array(items)) => items.clone(),
        other if !truthy(other) => vec![],
        _ => {
            // losing the denies would fail open
            if broken.is_empty() {
                broken = CONFIG_FILE.to_string();
            }
            notes.push("config.json egress_never must be a list: every non-local request fails closed".into());
            vec![]
        }
    };
    let global = if truthy(config.get("egress_hosts")) { config["egress_hosts"].clone() } else { Value::Array(vec![]) };
    egress_policy::effective_from(&Value::Array(texts), &global, &denies, as_bool(config.get("egress_builtin_read"), true), defined_names, notes, &broken)
}

fn normalise_method(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed == UNKNOWN_METHOD {
        UNKNOWN_METHOD.to_string()
    } else if trimmed.to_lowercase() == TCP {
        TCP.to_string()
    } else {
        trimmed.to_uppercase()
    }
}

fn method_matches(entry: &Entry, method: &str, scheme: &str) -> bool {
    let entry_tcp = entry.methods.contains(TCP);
    let request_tcp = method == TCP || (method == UNKNOWN_METHOD && scheme == TCP);
    if entry_tcp != request_tcp {
        return false;
    }
    if request_tcp || method == UNKNOWN_METHOD {
        return true;
    }
    entry.methods.contains(ANY_METHOD) || entry.methods.contains(method)
}

/// Judges one request against an effective list: `(allowed, reason)`. Order: parse, deny, local hosts, broken list, allow entries. `cred=true`
/// additionally needs a MATCHING entry that carries `cred`. Method `*` = unknown method, `tcp` = non-HTTP connection.
pub fn network_allows(effective: &Effective, method: &str, url: &str, cred: bool, env_names: Option<&[String]>) -> (bool, String) {
    let method = normalise_method(method);
    let (scheme, host, port, path) = match parse_request_url(url) {
        Ok(parts) => parts,
        Err(error) => return (false, format!("cannot judge the url: {error}")),
    };
    let target = format!("{host}{}{path}", port.map(|p| format!(":{p}")).unwrap_or_default());
    for pattern in &effective.deny {
        if host_matches(pattern, &host) {
            return (false, format!("host {host} is denied (egress_never {pattern})"));
        }
    }
    if effective.allow.iter().any(|entry| entry.origin == ORIGIN_LOCAL && host_matches(&entry.host, &host)) {
        return (true, "local host".into());
    }
    if !effective.broken.is_empty() {
        return (false, format!("{} is unreadable: every non-local request is blocked until it is fixed", effective.broken));
    }
    let defined: Vec<&String> = match env_names {
        Some(names) => names.iter().collect(),
        None => effective.env_names.iter().collect(),
    };
    let mut matching: Vec<&Entry> = vec![];
    for entry in &effective.allow {
        if !entry.if_missing.is_empty() && defined.iter().any(|name| **name == entry.if_missing) {
            continue;
        }
        if !host_matches(&entry.host, &host) || entry.port.is_some_and(|p| Some(p) != port) {
            continue;
        }
        if !path_matches(&entry.path, &path) || !method_matches(entry, &method, &scheme) {
            continue;
        }
        matching.push(entry);
    }
    if matching.is_empty() {
        return (false, format!("{method} {target} is not in the network list"));
    }
    if cred {
        let carrying: Vec<&Entry> = matching.iter().copied().filter(|entry| entry.cred).collect();
        if carrying.is_empty() {
            return (false, format!("{method} {target} is listed ({}) but without `cred`: credentials may not be sent there", matching[0].text));
        }
        matching = carrying;
    }
    (true, format!("matches {} ({})", matching[0].text, matching[0].origin))
}

// ---- browser filter ------------------------------------------------------------------------------------------------------------

/// The part of an effective list a browser can enforce: active HTTP entries (host, port, path, methods), the denies, the local hosts.
fn browser_policy(effective: &Effective) -> String {
    let defined = &effective.env_names;
    let mut allow: Vec<String> = vec![];
    for entry in effective.allow.iter().filter(|e| e.origin != ORIGIN_LOCAL && !e.methods.contains(TCP) && !(!e.if_missing.is_empty() && defined.contains(&e.if_missing)))
    {
        // keys sorted, compact separators, ensure_ascii: json.dumps(..., sort_keys=True, separators=(",", ":"))
        let methods: Vec<String> = entry.methods.iter().map(|m| json_string(m)).collect();
        let port = entry.port.map(|p| p.to_string()).unwrap_or_else(|| "null".into());
        allow.push(format!("{{\"host\":{},\"methods\":[{}],\"path\":{},\"port\":{port}}}", json_string(&entry.host), methods.join(","), json_string(&entry.path)));
    }
    let deny: Vec<String> = effective.deny.iter().map(|d| json_string(d)).collect();
    let local: Vec<String> = effective.allow.iter().filter(|e| e.origin == ORIGIN_LOCAL).map(|e| json_string(&e.host)).collect();
    format!("{{\"allow\":[{}],\"broken\":{},\"deny\":[{}],\"local\":[{}]}}", allow.join(","), !effective.broken.is_empty(), deny.join(","), local.join(","))
}

fn json_string(text: &str) -> String {
    pycompat::json::py_json_dumps(&Value::String(text.to_string()), true)
}

/// The `playwright-cli run-code` function that aborts (`blockedbyclient`) every request outside the list. Deterministic for a given list.
pub fn browser_filter_script(effective: &Effective) -> String {
    BROWSER_FILTER_TEMPLATE.replace(POLICY_PLACEHOLDER, &browser_policy(effective))
}

// ---- contract ------------------------------------------------------------------------------------------------------------------

/// Runs the cases of a contract fixture (`cases` and `invalid`) through these functions. Returns `{"passed": N, "failed": [case + "got" (+ "reason")]}`.
pub fn run_contract(fixture: &Value) -> Result<Value, String> {
    let object = fixture.as_object().ok_or("the fixture is not an object")?;
    let mut passed = 0u64;
    let mut failed: Vec<Value> = vec![];
    for case in object.get("cases").and_then(Value::as_array).cloned().unwrap_or_default() {
        let case_map = case.as_object().ok_or("a case is not an object")?;
        let texts: Vec<Value> = match case_map.get("entries") {
            Some(Value::Array(items)) if !items.is_empty() => items.clone(),
            _ => match case_map.get("entry") {
                Some(entry) if truthy(Some(entry)) => vec![entry.clone()],
                _ => vec![],
            },
        };
        let deny: Vec<Value> = case_map.get("deny").and_then(Value::as_array).cloned().unwrap_or_default();
        let builtin_read = case_map.get("builtin_read").is_none_or(|v| truthy(Some(v)));
        let env_names: Option<Vec<String>> =
            case_map.get("env_names").map(|v| v.as_array().map(|a| a.iter().filter_map(|n| n.as_str().map(String::from)).collect()).unwrap_or_default());
        let effective = egress_policy::effective_from(&Value::Array(texts), &Value::Array(vec![]), &deny, builtin_read, env_names.as_deref().unwrap_or(&[]), vec![], "");
        let request = case_map.get("request").ok_or("a case has no request")?;
        let method = request["method"].as_str().ok_or("a request has no method")?;
        let url = request["url"].as_str().ok_or("a request has no url")?;
        let cred = request.get("cred").is_some_and(|v| truthy(Some(v)));
        let (allowed, reason) = network_allows(&effective, method, url, cred, env_names.as_deref());
        let expected = case_map.get("allowed").and_then(Value::as_bool).ok_or("a case has no `allowed`")?;
        if allowed == expected {
            passed += 1;
        } else {
            let mut entry = case_map.clone();
            entry.insert("got".into(), json!(allowed));
            entry.insert("reason".into(), json!(reason));
            failed.push(Value::Object(entry));
        }
    }
    for item in object.get("invalid").and_then(Value::as_array).cloned().unwrap_or_default() {
        let item_map = item.as_object().ok_or("an invalid entry is not an object")?;
        let got = match parse_entry(item_map.get("invalid").ok_or("no `invalid` text")?, ORIGIN_PROJECT) {
            Ok(_) => "accepted".to_string(),
            Err(error) => error,
        };
        let needle = item_map.get("reason_contains").and_then(Value::as_str).ok_or("no `reason_contains`")?;
        if got != "accepted" && got.to_lowercase().contains(&needle.to_lowercase()) {
            passed += 1;
        } else {
            let mut entry = item_map.clone();
            entry.insert("got".into(), json!(got));
            failed.push(Value::Object(entry));
        }
    }
    Ok(json!({"passed": passed, "failed": failed}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../egress-policy/tests/network_contract.json");

    #[test]
    fn the_shared_contract_fixture_passes_with_no_failures() {
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        let result = run_contract(&fixture).unwrap();
        assert_eq!(result["failed"], json!([]), "failed cases: {}", result["failed"]);
        assert_eq!(result["passed"], json!(112 + 35));
    }

    #[test]
    fn judgement_wording_is_testenv_wording() {
        let effective = egress_policy::effective_from(&json!(["api.example.com POST"]), &json!([]), &[], false, &[], vec![], "");
        assert_eq!(
            network_allows(&effective, "GET", "https://api.example.com/x", false, None),
            (false, "GET api.example.com:443/x is not in the network list".to_string())
        );
        assert_eq!(
            network_allows(&effective, "post", "https://api.example.com/x", true, None).1,
            "POST api.example.com:443/x is listed (api.example.com POST) but without `cred`: credentials may not be sent there"
        );
        assert_eq!(network_allows(&effective, "GET", "http://localhost:3000/", true, None), (true, "local host".to_string()));
    }

    #[test]
    fn the_browser_policy_is_compact_sorted_json() {
        let effective = egress_policy::effective_from(&json!(["*.example.com/api GET,POST"]), &json!([]), &[json!("evil.example.com")], false, &[], vec![], "");
        let script = browser_filter_script(&effective);
        assert!(script.contains(r#"const policy = {"allow":[{"host":"*.example.com","methods":["GET","POST"],"path":"/api","port":null}],"broken":false,"deny":["evil.example.com"],"local":["localhost","127.0.0.1","::1","0.0.0.0","*.localhost"]};"#));
        assert!(!script.contains(POLICY_PLACEHOLDER));
    }
}
