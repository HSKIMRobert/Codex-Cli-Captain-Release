use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SSL_MANIFEST_VERSION: &str = "ccc.skill_ssl_manifest.v1";
const SUPPORTED_SSL_MANIFEST_DOCUMENT_VERSION: &str = "0.1";

pub(crate) fn load_skill_ssl_manifest_for_agent(agent_name: &str) -> Value {
    let Some(path) = resolve_skill_ssl_manifest_path(agent_name) else {
        return missing_manifest(agent_name);
    };
    load_skill_ssl_manifest_from_path(agent_name, &path)
}

pub(crate) fn load_skill_ssl_manifest_from_path(agent_name: &str, path: &Path) -> Value {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            return invalid_manifest(
                agent_name,
                Some(path),
                format!("Unable to read skill SSL manifest: {error}"),
            )
        }
    };
    let manifest: Value = match serde_json::from_str(&raw) {
        Ok(manifest) => manifest,
        Err(error) => {
            return invalid_manifest(
                agent_name,
                Some(path),
                format!("Unable to parse skill SSL manifest JSON: {error}"),
            )
        }
    };
    validate_skill_ssl_manifest(agent_name, Some(path), manifest)
}

fn validate_skill_ssl_manifest(agent_name: &str, path: Option<&Path>, manifest: Value) -> Value {
    let Some(version) = manifest
        .get("version")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return invalid_manifest(
            agent_name,
            path,
            "Skill SSL manifest is missing required string field: version".to_string(),
        );
    };
    if version != SUPPORTED_SSL_MANIFEST_DOCUMENT_VERSION {
        return classified_manifest(
            agent_name,
            path,
            "stale",
            format!(
                "Skill SSL manifest version {version} is not the supported version {SUPPORTED_SSL_MANIFEST_DOCUMENT_VERSION}.",
            ),
        );
    }

    let manifest_skill_id = manifest
        .get("skill_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if manifest_skill_id != agent_name {
        return classified_manifest(
            agent_name,
            path,
            "drift_detected",
            format!(
                "Skill SSL manifest skill_id {manifest_skill_id:?} does not match requested agent {agent_name}.",
            ),
        );
    }

    let missing_sections = ["scheduling", "structural", "logical"]
        .into_iter()
        .filter(|section| !manifest.get(*section).is_some_and(Value::is_object))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !missing_sections.is_empty() {
        return invalid_manifest(
            agent_name,
            path,
            format!(
                "Skill SSL manifest is missing object section(s): {}",
                missing_sections.join(", ")
            ),
        );
    }

    let missing_required_fields = missing_required_manifest_fields(&manifest);
    if !missing_required_fields.is_empty() {
        return invalid_manifest(
            agent_name,
            path,
            format!(
                "Skill SSL manifest is missing required field(s): {}",
                missing_required_fields.join(", ")
            ),
        );
    }

    json!({
        "schema": SSL_MANIFEST_VERSION,
        "status": "available",
        "blocking": false,
        "source": "skill_ssl_manifest",
        "agent_name": agent_name,
        "path": path.map(|value| value.to_string_lossy().to_string()).unwrap_or_default(),
        "runtime_truth": false,
        "advisory_only": true,
        "manifest_version": version,
        "fallback": "SKILL.md + ccc-config.toml",
        "display_name": manifest.get("display_name").cloned().unwrap_or(Value::Null),
        "callsign": manifest.get("callsign").cloned().unwrap_or(Value::Null),
        "theme": manifest.get("theme").cloned().unwrap_or(Value::Null),
        "inspired_by": manifest.get("inspired_by").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "recommended_workflows": manifest.get("recommended_workflows").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "lsp_capabilities": manifest.get("lsp_capabilities").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "scheduling": manifest.get("scheduling").cloned().unwrap_or(Value::Null),
        "structural": manifest.get("structural").cloned().unwrap_or(Value::Null),
        "logical": manifest.get("logical").cloned().unwrap_or(Value::Null),
    })
}

fn missing_required_manifest_fields(manifest: &Value) -> Vec<String> {
    let mut missing = Vec::new();
    require_string(manifest, "skill_id", &mut missing);
    require_string(manifest, "display_name", &mut missing);
    require_string(manifest, "callsign", &mut missing);
    require_string(manifest, "theme", &mut missing);
    require_array(manifest, "inspired_by", &mut missing);
    require_array(manifest, "recommended_workflows", &mut missing);
    require_array(manifest, "lsp_capabilities", &mut missing);
    require_string(manifest, "scheduling.role_family", &mut missing);
    require_string(manifest, "scheduling.display_agent_id", &mut missing);
    require_array(manifest, "scheduling.intent_signatures", &mut missing);
    require_array(manifest, "scheduling.expected_inputs", &mut missing);
    require_array(manifest, "scheduling.expected_outputs", &mut missing);
    require_bool(manifest, "scheduling.mutation_allowed", &mut missing);
    require_array(manifest, "structural.scenes", &mut missing);
    require_array(manifest, "logical.actions", &mut missing);
    require_bool(manifest, "logical.requires_operator_approval", &mut missing);
    require_bool(manifest, "logical.external_side_effects", &mut missing);
    missing
}

fn manifest_field<'a>(manifest: &'a Value, dotted_path: &str) -> Option<&'a Value> {
    dotted_path
        .split('.')
        .try_fold(manifest, |current, key| current.get(key))
}

fn require_string(manifest: &Value, dotted_path: &str, missing: &mut Vec<String>) {
    if !manifest_field(manifest, dotted_path).is_some_and(|value| {
        value
            .as_str()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    }) {
        missing.push(dotted_path.to_string());
    }
}

fn require_array(manifest: &Value, dotted_path: &str, missing: &mut Vec<String>) {
    if !manifest_field(manifest, dotted_path).is_some_and(Value::is_array) {
        missing.push(dotted_path.to_string());
    }
}

fn require_bool(manifest: &Value, dotted_path: &str, missing: &mut Vec<String>) {
    if !manifest_field(manifest, dotted_path).is_some_and(Value::is_boolean) {
        missing.push(dotted_path.to_string());
    }
}

fn resolve_skill_ssl_manifest_path(agent_name: &str) -> Option<PathBuf> {
    skill_ssl_manifest_directories()
        .into_iter()
        .map(|directory| directory.join(format!("{agent_name}.skill.ssl.json")))
        .find(|path| path.exists())
}

fn skill_ssl_manifest_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Ok(directory) = env::var("CCC_SKILL_SSL_MANIFEST_DIR") {
        let trimmed = directory.trim();
        if !trimmed.is_empty() {
            directories.push(PathBuf::from(trimmed));
        }
    }

    // Keep the first slice non-invasive: manifests are optional sidecar files
    // near CCC source/package assets, not a replacement for SKILL.md or config.
    if let Ok(current_dir) = env::current_dir() {
        directories.push(current_dir.join("skills").join("ssl"));
        directories.push(
            current_dir
                .join("Codex-Cli-Captain")
                .join("skills")
                .join("ssl"),
        );
    }
    if let Ok(exe) = env::current_exe() {
        for ancestor in exe.ancestors().take(6) {
            directories.push(ancestor.join("skills").join("ssl"));
        }
    }

    directories
}

fn missing_manifest(agent_name: &str) -> Value {
    json!({
        "schema": SSL_MANIFEST_VERSION,
        "status": "missing",
        "blocking": false,
        "source": "skill_ssl_manifest",
        "agent_name": agent_name,
        "runtime_truth": false,
        "advisory_only": true,
        "fallback": "SKILL.md + ccc-config.toml",
        "reason": "Optional skill SSL manifest was not found; existing routing and risk rules remain authoritative.",
    })
}

fn invalid_manifest(agent_name: &str, path: Option<&Path>, reason: String) -> Value {
    classified_manifest(agent_name, path, "invalid", reason)
}

fn classified_manifest(
    agent_name: &str,
    path: Option<&Path>,
    status: &str,
    reason: String,
) -> Value {
    json!({
        "schema": SSL_MANIFEST_VERSION,
        "status": status,
        "blocking": false,
        "source": "skill_ssl_manifest",
        "agent_name": agent_name,
        "path": path.map(|value| value.to_string_lossy().to_string()).unwrap_or_default(),
        "runtime_truth": false,
        "advisory_only": true,
        "fallback": "SKILL.md + ccc-config.toml",
        "reason": reason,
    })
}
