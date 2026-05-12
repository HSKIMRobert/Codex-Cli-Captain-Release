use serde_json::{json, Value};

use crate::skill_manifest::load_skill_ssl_manifest_for_agent;

const SKILL_REGISTRY_SCHEMA: &str = "ccc.skill_registry.v1";

pub(crate) fn load_skill_registry_for_agent(
    agent_name: &str,
    role_config_snapshot: &Value,
) -> Value {
    let skill_ssl_manifest = load_skill_ssl_manifest_for_agent(agent_name);
    build_skill_registry_payload(agent_name, skill_ssl_manifest, role_config_snapshot)
}

pub(crate) fn build_skill_registry_payload(
    agent_name: &str,
    skill_ssl_manifest: Value,
    role_config_snapshot: &Value,
) -> Value {
    let manifest_status = skill_ssl_manifest
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let registry_status = registry_status_from_manifest(manifest_status);
    let evidence_sources = evidence_sources_for_status(registry_status);

    // Registry evidence is intentionally advisory: runtime state, task cards,
    // lifecycle, and fan-in remain the authoritative truth for completed work.
    json!({
        "schema": SKILL_REGISTRY_SCHEMA,
        "status": registry_status,
        "blocking": false,
        "runtime_truth": false,
        "advisory_only": true,
        "agent_name": agent_name,
        "source": "skill_registry",
        "source_priority": [
            "persisted_run_state",
            "approved_longway_task_cards",
            "ccc_config_toml",
            "skill_ssl_json",
            "skill_md",
        ],
        "evidence_sources": evidence_sources,
        "fallback": "SKILL.md + ccc-config.toml",
        "manifest_status": manifest_status,
        "skill_ssl_manifest": skill_ssl_manifest,
        "role_config": registry_role_config_evidence(role_config_snapshot),
    })
}

fn registry_status_from_manifest(manifest_status: &str) -> &'static str {
    match manifest_status {
        "available" => "available",
        "invalid" => "invalid",
        "stale" => "stale",
        "drift_detected" => "drift_detected",
        _ => "missing",
    }
}

fn evidence_sources_for_status(registry_status: &str) -> Vec<Value> {
    let mut sources = vec![
        json!({
            "source": "ccc_config_toml",
            "available": true,
            "authoritative_for": ["model", "variant", "execution_policy"],
        }),
        json!({
            "source": "skill_md",
            "available": true,
            "authoritative_for": ["human_instructions", "constraints"],
        }),
    ];
    sources.push(json!({
        "source": "skill_ssl_json",
        "available": registry_status == "available",
        "authoritative_for": ["scheduling_evidence", "structural_scenes", "logical_risk_evidence"],
    }));
    sources
}

fn registry_role_config_evidence(role_config_snapshot: &Value) -> Value {
    json!({
        "source": "role_config_snapshot",
        "model": role_config_snapshot.get("model").cloned().unwrap_or(Value::Null),
        "variant": role_config_snapshot.get("variant").cloned().unwrap_or(Value::Null),
        "fast_mode": role_config_snapshot
            .get("fast_mode")
            .cloned()
            .unwrap_or(Value::Bool(false)),
        "summary": role_config_snapshot.get("summary").cloned().unwrap_or(Value::Null),
    })
}
