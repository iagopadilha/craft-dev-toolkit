// SPDX-License-Identifier: MIT
//! `craft regression_diff baseline.json new.json [--ignore regex ...] [--key field ...]`: compares two JSON snapshots (baseline = old code, new =
//! refactored code).
//!
//! Drops volatile keys, replaces GUID-like and ISO-date-like strings by `<GUID>` / `<DATE>`, sorts lists, prints each difference with its path
//! and a count summary. Exit: 0 same, 1 different, 2 bad arguments. A large drop in counts is a warning, not a win.

use crate::text;
use pycompat::json::py_json_dumps;
use pycompat::Re;
use serde_json::{Map, Value};
use std::sync::LazyLock;

const USAGE_TEXT: &str = "Compares two JSON snapshots (baseline = old code, new = refactored code).

Usage: craft regression_diff baseline.json new.json [--ignore regex ...] [--key field ...]
- Drops volatile keys (id, createdAt, updatedAt, timestamp, guid... and those from --ignore).
- Replaces values that look like a GUID or ISO date with <GUID> / <DATE>.
- Sorts lists of objects (by --key if given, otherwise by canonical JSON), so ordering does not cause false alarms.
- Prints each difference with its path (e.g. tables.Order[2].amount) and a count summary.
Exit: 0 same, 1 different. A large drop in counts is a warning, not a win: explain every item that disappeared.

";
const DEFAULT_IGNORE: &str = r"^(id|.*Id|createdAt|updatedAt|created_at|updated_at|timestamp|guid|traceId|requestId)(?=\n?\z)";
const MAX_LINES: usize = 40;

static GUID: LazyLock<Re> = LazyLock::new(|| Re::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}(?=\n?\z)"));
static ISO_DATE: LazyLock<Re> = LazyLock::new(|| Re::new(r"^\d{4}-\d{2}-\d{2}([T ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?)?(?=\n?\z)"));

/// `json.dumps(value, sort_keys=True, ensure_ascii=...)`.
fn dumps_sorted(value: &Value, ensure_ascii: bool) -> String {
    py_json_dumps(&sort_keys(value), ensure_ascii)
}

fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            Value::Object(keys.into_iter().map(|k| (k.clone(), sort_keys(&map[k]))).collect::<Map<String, Value>>())
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

fn normalize(value: &Value, ignore: &Re, keys: &[String]) -> Value {
    match value {
        Value::Object(map) => {
            Value::Object(map.iter().filter(|(name, _)| ignore.search(name).is_none()).map(|(name, inner)| (name.clone(), normalize(inner, ignore, keys))).collect())
        }
        Value::Array(items) => {
            let items: Vec<Value> = items.iter().map(|item| normalize(item, ignore, keys)).collect();
            if keys.is_empty() {
                let mut decorated: Vec<(String, Value)> = items.into_iter().map(|item| (dumps_sorted(&item, false), item)).collect();
                decorated.sort_by(|a, b| a.0.cmp(&b.0));
                Value::Array(decorated.into_iter().map(|(_, item)| item).collect())
            } else {
                let sort_key = |item: &Value| -> Vec<String> {
                    keys.iter().map(|key| if let Value::Object(map) = item { dumps_sorted(map.get(key).unwrap_or(&Value::Null), true) } else { String::new() }).collect()
                };
                let mut decorated: Vec<(Vec<String>, Value)> = items.into_iter().map(|item| (sort_key(&item), item)).collect();
                decorated.sort_by(|a, b| a.0.cmp(&b.0));
                Value::Array(decorated.into_iter().map(|(_, item)| item).collect())
            }
        }
        Value::String(text) => {
            if GUID.is_match(text) {
                Value::String("<GUID>".into())
            } else if ISO_DATE.is_match(text) {
                Value::String("<DATE>".into())
            } else {
                value.clone()
            }
        }
        other => other.clone(),
    }
}

/// Python's `type(x).__name__` for a JSON value.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Object(_) => "dict",
        Value::Array(_) => "list",
        Value::String(_) => "str",
        Value::Bool(_) => "bool",
        Value::Null => "NoneType",
        Value::Number(n) => {
            if n.is_f64() {
                "float"
            } else {
                "int"
            }
        }
    }
}

fn compare(old: &Value, new: &Value, path: &str, out: &mut Vec<String>) {
    if type_name(old) != type_name(new) {
        out.push(format!("{path}: type {} x {}", type_name(old), type_name(new)));
        return;
    }
    match (old, new) {
        (Value::Object(left), Value::Object(right)) => {
            let mut names: Vec<&String> = left.keys().chain(right.keys()).collect();
            names.sort();
            names.dedup();
            for name in names {
                match (left.get(name), right.get(name)) {
                    (Some(l), None) => out.push(format!("{path}.{name}: ONLY IN BASELINE ({})", text::head(&py_json_dumps(l, false), 80))),
                    (None, Some(r)) => out.push(format!("{path}.{name}: ONLY IN NEW ({})", text::head(&py_json_dumps(r, false), 80))),
                    (Some(l), Some(r)) => compare(l, r, &format!("{path}.{name}"), out),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                out.push(format!("{path}: {} items in baseline x {} in new", left.len(), right.len()));
            }
            for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
                compare(l, r, &format!("{path}[{index}]"), out);
            }
        }
        _ => {
            if old != new {
                out.push(format!("{path}: {} x {}", text::head(&py_json_dumps(old, false), 60), text::head(&py_json_dumps(new, false), 60)));
            }
        }
    }
}

fn load(path: &str) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|_| format!("cannot read {path}: not valid UTF-8"))?;
    serde_json::from_str::<Value>(&text).map_err(|e| format!("cannot parse {path}: {e}"))
}

pub fn main(argv: &[String]) -> i32 {
    let mut files: Vec<String> = vec![];
    let mut ignores: Vec<String> = vec![DEFAULT_IGNORE.to_string()];
    let mut keys: Vec<String> = vec![];
    let mut mode = "";
    for arg in argv {
        if arg == "--ignore" {
            mode = "ignore";
        } else if arg == "--key" {
            mode = "key";
        } else if mode == "ignore" {
            ignores.push(arg.clone());
            mode = "";
        } else if mode == "key" {
            keys.push(arg.clone());
            mode = "";
        } else {
            files.push(arg.clone());
        }
    }
    if files.len() != 2 {
        text::out(USAGE_TEXT);
        return 2;
    }
    let combined = ignores.iter().map(|p| format!("(?:{p})")).collect::<Vec<_>>().join("|");
    let ignore = match Re::try_build_msg(&combined, false) {
        Ok(re) => re,
        Err(message) => {
            text::err(&format!("regression_diff: invalid --ignore pattern: {message}\n"));
            return 1;
        }
    };
    let (old, new) = match (load(&files[0]), load(&files[1])) {
        (Ok(old), Ok(new)) => (normalize(&old, &ignore, &keys), normalize(&new, &ignore, &keys)),
        (Err(message), _) | (_, Err(message)) => {
            text::err(&format!("regression_diff: {message}\n"));
            return 1;
        }
    };
    let mut differences = vec![];
    compare(&old, &new, "$", &mut differences);
    if differences.is_empty() {
        text::println_out("REGRESSION: SAME (baseline x new, after normalizing)");
        return 0;
    }
    text::println_out(&format!("REGRESSION: DIFFERENT ({})", differences.len()));
    for line in differences.iter().take(MAX_LINES) {
        text::println_out(&format!("- {line}"));
    }
    if differences.len() > MAX_LINES {
        text::println_out(&format!("- ... {} more", differences.len() - MAX_LINES));
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn same(a: Value, b: Value, keys: &[String]) -> bool {
        let ignore = Re::new(&format!("(?:{DEFAULT_IGNORE})"));
        let mut out = vec![];
        compare(&normalize(&a, &ignore, keys), &normalize(&b, &ignore, keys), "$", &mut out);
        out.is_empty()
    }

    #[test]
    fn volatile_data_and_order_do_not_count() {
        let a = json!({"id": 1, "items": [{"n": "a", "when": "2024-01-02T03:04:05Z"}, {"n": "b", "guid2": "123e4567-e89b-12d3-a456-426614174000"}]});
        let b = json!({"id": 9, "items": [{"n": "b", "guid2": "00000000-0000-0000-0000-000000000000"}, {"n": "a", "when": "2025-05-05"}]});
        assert!(same(a, b, &[]));
    }

    #[test]
    fn a_real_difference_is_reported_with_its_path() {
        let ignore = Re::new(&format!("(?:{DEFAULT_IGNORE})"));
        let mut out = vec![];
        compare(&normalize(&json!({"t": {"amount": 1}}), &ignore, &[]), &normalize(&json!({"t": {"amount": 2.5}}), &ignore, &[]), "$", &mut out);
        assert_eq!(out, vec!["$.t.amount: type int x float"]);
        out.clear();
        compare(&json!({"a": [1, 2]}), &json!({"a": [1]}), "$", &mut out);
        assert_eq!(out, vec!["$.a: 2 items in baseline x 1 in new"]);
    }

    #[test]
    fn keys_sort_lists_of_objects_by_the_given_fields() {
        let keys = vec!["n".to_string()];
        assert!(same(json!([{"n": 2}, {"n": 1}]), json!([{"n": 1}, {"n": 2}]), &keys));
    }
}
