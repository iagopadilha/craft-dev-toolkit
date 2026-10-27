// SPDX-License-Identifier: MIT
//! One version per repository: the workspace, the plugin manifest and the marketplace entry must agree, and the skills the README promises exist.

use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(relative: &str) -> Value {
    let text = std::fs::read_to_string(root().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

#[test]
fn the_plugin_the_marketplace_and_the_workspace_share_one_version() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(json("plugin/.claude-plugin/plugin.json")["version"], version);
    assert_eq!(json(".claude-plugin/marketplace.json")["plugins"][0]["version"], version);
}

#[test]
fn the_marketplace_points_at_the_plugin_folder_by_name() {
    let marketplace = json(".claude-plugin/marketplace.json");
    assert_eq!(marketplace["plugins"][0]["source"], "./plugin");
    assert_eq!(marketplace["plugins"][0]["name"], json("plugin/.claude-plugin/plugin.json")["name"]);
}

#[test]
fn every_skill_folder_has_a_skill_file_with_a_matching_name() {
    let skills = root().join("plugin").join("skills");
    let mut count = 0;
    for entry in std::fs::read_dir(&skills).expect("skills folder") {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        let text = std::fs::read_to_string(entry.path().join("SKILL.md")).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(text.lines().any(|l| l.trim() == format!("name: {name}")), "{name}: the frontmatter name must equal the folder");
        count += 1;
    }
    assert_eq!(count, 8);
}

#[test]
fn skills_never_call_a_python_script() {
    let mut stack = vec![root().join("plugin")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") && !path.ends_with("README.md") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                assert!(!text.contains("shared/scripts"), "{} still points at shared/scripts", path.display());
            }
        }
    }
}
