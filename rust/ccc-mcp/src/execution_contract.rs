use serde_json::{json, Value};

use crate::specialist_roles::{
    agent_id_for_role, callsign_for_role, fallback_specialist_execution_mode,
    generated_custom_agent_name, preferred_specialist_execution_mode, sandbox_mode_for_role,
    sandbox_rationale_for_role, SUBAGENT_FALLBACK_REASON_CODES,
};

const CONTRACT_SCHEMA: &str = "ccc.execution_contract.v1";

pub(crate) fn configured_execution_contract_roles() -> [&'static str; 8] {
    [
        "way",
        "explorer",
        "code specialist",
        "documenter",
        "verifier",
        "sentinel",
        "companion_reader",
        "companion_operator",
    ]
}

fn cost_tier_for_variant(variant: Option<&str>) -> &'static str {
    match variant {
        Some("low") => "low_cost",
        Some("high") | Some("xhigh") => "high_tier",
        _ => "standard",
    }
}

fn runtime_config_from_shared_config(config: &Value) -> Value {
    let runtime = config.get("runtime").cloned().unwrap_or(Value::Null);
    json!({
        "preferred_specialist_execution_mode": preferred_specialist_execution_mode(&runtime),
        "fallback_specialist_execution_mode": fallback_specialist_execution_mode(&runtime),
    })
}

fn supported_execution_modes(runtime_config: &Value) -> Vec<Value> {
    let preferred = preferred_specialist_execution_mode(runtime_config);
    let fallback = fallback_specialist_execution_mode(runtime_config);
    if preferred == fallback {
        vec![Value::String(preferred)]
    } else {
        vec![Value::String(preferred), Value::String(fallback)]
    }
}

fn review_capability_for_role(role: &str) -> Value {
    json!({
        "can_review": role == "verifier",
        "can_gate_acceptance": role == "verifier",
        "can_report_findings": true,
        "review_source": if role == "verifier" { "arbiter_role_contract" } else { "fan_in_only" },
    })
}

fn mutation_capability_for_sandbox(sandbox_mode: &str) -> Value {
    let can_mutate = sandbox_mode != "read-only";
    json!({
        "can_mutate_workspace": can_mutate,
        "mutation_boundary": if can_mutate { "workspace-write" } else { "read-only" },
        "runtime_truth": "persisted_task_card_and_host_policy",
    })
}

pub(crate) fn create_execution_contract_for_role(
    role: &str,
    role_config_snapshot: &Value,
    runtime_config: &Value,
    sandbox_mode: &str,
    sandbox_rationale: &str,
) -> Value {
    let agent_id = agent_id_for_role(role);
    let custom_agent_name = agent_id.map(generated_custom_agent_name);
    let model = role_config_snapshot
        .get("model")
        .cloned()
        .unwrap_or(Value::Null);
    let variant = role_config_snapshot.get("variant").and_then(Value::as_str);
    let preferred_execution_mode = preferred_specialist_execution_mode(runtime_config);
    let fallback_execution_mode = fallback_specialist_execution_mode(runtime_config);

    // The registry describes intended boundaries; persisted task-card state
    // remains the source of truth for active execution.
    json!({
        "schema": CONTRACT_SCHEMA,
        "advisory_only": true,
        "runtime_truth": "persisted_run_and_task_card_state",
        "role_identity": {
            "role": role,
            "agent_id": agent_id,
            "custom_agent_name": custom_agent_name,
            "display_name": role_config_snapshot.get("display_name").cloned().unwrap_or_else(|| callsign_for_role(role).map(Value::from).unwrap_or(Value::Null)),
            "callsign": role_config_snapshot.get("callsign").cloned().unwrap_or_else(|| callsign_for_role(role).map(Value::from).unwrap_or(Value::Null)),
            "theme": role_config_snapshot.get("theme").cloned().unwrap_or(Value::Null),
            "inspired_by": role_config_snapshot.get("inspired_by").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        },
        "cost_tier": cost_tier_for_variant(variant),
        "supported_modes": supported_execution_modes(runtime_config),
        "model_policy": {
            "source": role_config_snapshot.get("source").cloned().unwrap_or(Value::String("role_config_snapshot".to_string())),
            "model": model,
            "variant": role_config_snapshot.get("variant").cloned().unwrap_or(Value::Null),
            "fast_mode": role_config_snapshot.get("fast_mode").and_then(Value::as_bool).unwrap_or(false),
            "config_entries": role_config_snapshot.get("config_entries").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "recommended_workflows": role_config_snapshot.get("recommended_workflows").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "lsp_capabilities": role_config_snapshot.get("lsp_capabilities").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        },
        "fallback_policy": {
            "preferred_execution_mode": preferred_execution_mode,
            "fallback_execution_mode": fallback_execution_mode,
            "must_attempt_preferred_first": true,
            "require_explicit_reason": true,
            "reason_codes": SUBAGENT_FALLBACK_REASON_CODES,
        },
        "tool_restrictions": {
            "sandbox_mode": sandbox_mode,
            "sandbox_rationale": sandbox_rationale,
            "avoid_mcp_tool_call_for_fan_in": true,
            "fan_in_transport": "ccc_cli_subcommand",
            "omit_model_override": true,
            "omit_reasoning_effort_override": true,
        },
        "mutation_capability": mutation_capability_for_sandbox(sandbox_mode),
        "review_capability": review_capability_for_role(role),
    })
}

pub(crate) fn create_execution_contract_registry_from_config(config: &Value) -> Value {
    let runtime_config = runtime_config_from_shared_config(config);
    let contracts = configured_execution_contract_roles()
        .into_iter()
        .map(|role| {
            let snapshot =
                crate::specialist_roles::load_role_config_snapshot_from_config(config, role);
            let sandbox_mode = sandbox_mode_for_role(role);
            let sandbox_rationale = sandbox_rationale_for_role(role);
            create_execution_contract_for_role(
                role,
                &snapshot,
                &runtime_config,
                sandbox_mode,
                sandbox_rationale,
            )
        })
        .collect::<Vec<_>>();

    json!({
        "schema": CONTRACT_SCHEMA,
        "status": if contracts.is_empty() { "missing" } else { "available" },
        "advisory_only": true,
        "runtime_truth": "persisted_run_and_task_card_state",
        "role_count": contracts.len(),
        "roles": contracts,
        "summary": if contracts.is_empty() {
            "No configured ccc_* execution contracts were available."
        } else {
            "Configured ccc_* execution contracts are available for delegation and install inspection."
        },
    })
}
