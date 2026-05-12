use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("ccc crate should live under rust/ccc-mcp")
        .to_path_buf()
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!("parse {}: {error}", path.display());
    })
}

fn assert_plugin_relative_path(value: &Value, field: &str) {
    let path = value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("manifest should include {field}"));
    assert!(
        path.starts_with("./"),
        "manifest {field} should be plugin-root relative and ./-prefixed"
    );
}

fn is_server_config(value: &Value) -> bool {
    value
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| !command.is_empty())
}

fn mcp_server_map(value: &Value) -> &Map<String, Value> {
    if let Some(map) = value.get("mcpServers").and_then(Value::as_object) {
        return map;
    }

    if let Some(map) = value.get("mcp_servers").and_then(Value::as_object) {
        return map;
    }

    value
        .as_object()
        .filter(|map| !map.is_empty() && map.values().all(is_server_config))
        .expect(".mcp.json should be a direct server map or a wrapped mcpServers/mcp_servers map")
}

fn read_plugin_skill(root: &Path) -> String {
    let skill_path = root.join("skills/ccc/SKILL.md");
    fs::read_to_string(&skill_path).unwrap_or_else(|error| {
        panic!("read {}: {error}", skill_path.display());
    })
}

fn assert_skill_contains(skill: &str, expected: &str) {
    assert!(
        skill.contains(expected),
        "plugin skill should include workflow contract text: {expected}"
    );
}

#[test]
fn ccc_plugin_manifest_points_to_root_relative_package_assets() {
    let root = repo_root();
    let manifest_path = root.join(".codex-plugin/plugin.json");
    let manifest = read_json(&manifest_path);

    assert_eq!(manifest["name"], "ccc");
    assert_eq!(manifest["mcpServers"], "./.mcp.json");
    assert_eq!(manifest["skills"], "./skills/");
    assert_plugin_relative_path(&manifest, "mcpServers");
    assert_plugin_relative_path(&manifest, "skills");

    let codex_plugin_entries = fs::read_dir(root.join(".codex-plugin"))
        .expect("read .codex-plugin")
        .map(|entry| {
            entry
                .expect("read .codex-plugin entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(codex_plugin_entries, vec!["plugin.json"]);
}

#[test]
fn ccc_plugin_mcp_launcher_starts_ccc_stdio_server() {
    let root = repo_root();
    let mcp = read_json(&root.join(".mcp.json"));
    let servers = mcp_server_map(&mcp);
    let ccc = servers
        .get("ccc")
        .expect("ccc MCP server should be declared");

    assert_eq!(ccc["command"], "ccc");
    assert_eq!(ccc["args"], serde_json::json!(["mcp"]));
}

#[test]
fn ccc_plugin_bundled_skill_preserves_cap_entrypoint() {
    let root = repo_root();
    let skill = read_plugin_skill(&root);

    assert!(skill.contains("name: ccc"));
    assert!(skill.contains("$cap"));
    assert!(skill.contains("public CCC entry point"));
    assert!(!skill.contains("name: cap"));
}

#[test]
fn ccc_plugin_bundled_skill_encodes_workflow_loop_contract() {
    let root = repo_root();
    let skill = read_plugin_skill(&root);

    assert_skill_contains(&skill, "start a CCC run");
    assert_skill_contains(&skill, "Create or refresh the CCC plan");
    assert_skill_contains(&skill, "bounded task cards");
    assert_skill_contains(&skill, "CCC role agents");
    assert_skill_contains(&skill, "wait for");
    assert_skill_contains(&skill, "fan-in");
    assert_skill_contains(&skill, "status or projection");
    assert_skill_contains(&skill, "review gate");
    assert_skill_contains(&skill, "bounded retry or replan");
    assert_skill_contains(&skill, "phase, role, and result updates");
    assert_skill_contains(&skill, "final summary");
}

#[test]
fn ccc_plugin_bundled_skill_keeps_plugin_invocation_secondary() {
    let root = repo_root();
    let skill = read_plugin_skill(&root);

    assert_skill_contains(&skill, "Keep plugin UI invocation secondary");
    assert_skill_contains(&skill, "Plugin UI controls may help discovery");
    assert_skill_contains(&skill, "not replacements for");
    assert_skill_contains(&skill, "the `$cap` entry point");
}
