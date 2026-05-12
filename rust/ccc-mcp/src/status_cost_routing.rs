use crate::specialist_roles::{
    fallback_specialist_execution_mode, load_role_config_snapshot,
    preferred_specialist_execution_mode,
};
use serde_json::{json, Value};

fn role_model_entry(role: &str) -> Value {
    let snapshot = load_role_config_snapshot(role);
    let model = snapshot.get("model").cloned().unwrap_or(Value::Null);
    let variant = snapshot.get("variant").cloned().unwrap_or(Value::Null);
    let fast_mode = snapshot
        .get("fast_mode")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let uses_low_cost_model = model
        .as_str()
        .map(|value| value.contains("mini"))
        .unwrap_or(false);

    json!({
        "role": role,
        "agent_id": crate::specialist_roles::agent_id_for_role(role).unwrap_or(role),
        "model": model,
        "variant": variant,
        "fast_mode": fast_mode,
        "source": snapshot.get("source").cloned().unwrap_or(Value::Null),
        "uses_low_cost_model": uses_low_cost_model,
    })
}

fn roles_low_cost(roles: &[&str]) -> bool {
    roles.iter().all(|role| {
        role_model_entry(role)
            .get("uses_low_cost_model")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn create_simple_task_route(label: &str, roles: &[&str]) -> Value {
    json!({
        "task": label,
        "roles": roles.iter().map(|role| role_model_entry(role)).collect::<Vec<_>>(),
        "uses_low_cost_model": roles_low_cost(roles),
    })
}

pub(crate) fn create_cost_routing_payload(runtime_config: &Value, payload: &Value) -> Value {
    let preferred_execution_mode = preferred_specialist_execution_mode(runtime_config);
    let fallback_execution_mode = fallback_specialist_execution_mode(runtime_config);
    let subagents_enabled = preferred_execution_mode == "codex_subagent";
    let current_task = payload.get("current_task_card").unwrap_or(&Value::Null);
    let current_role = current_task
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("unassigned");
    let current_role_model = if current_role == "unassigned" {
        Value::Null
    } else {
        role_model_entry(current_role)
    };
    let simple_task_routes = vec![
        create_simple_task_route(
            "simple_search_and_collection",
            &["explorer", "companion_reader"],
        ),
        create_simple_task_route("documentation_update", &["documenter"]),
        create_simple_task_route("git_or_filesystem_lookup", &["companion_reader"]),
        create_simple_task_route("narrow_git_or_release_mutation", &["companion_operator"]),
    ];
    let simple_routes_low_cost = simple_task_routes
        .iter()
        .all(|route| route.get("uses_low_cost_model").and_then(Value::as_bool) == Some(true));
    let status = if subagents_enabled && simple_routes_low_cost {
        "configured"
    } else {
        "needs_attention"
    };

    json!({
        "status": status,
        "summary": if status == "configured" {
            "CCC is configured to prefer host custom subagents, and simple search/collection/docs companion routes use mini-class models."
        } else {
            "CCC cost routing needs attention; inspect preferred execution mode and simple task role models."
        },
        "subagents": {
            "preferred_execution_mode": preferred_execution_mode,
            "fallback_execution_mode": fallback_execution_mode,
            "enabled": subagents_enabled,
            "host_update_mode": "ccc_cli_subcommand",
            "current_task_role": current_role,
            "current_task_agent_id": current_task.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
            "current_task_preferred_custom_agent": current_task.pointer("/delegation_plan/preferred_custom_agent_name").cloned().unwrap_or(Value::Null),
            "current_task_model": current_role_model,
        },
        "simple_task_routes": simple_task_routes,
        "simple_routes_use_low_cost_models": simple_routes_low_cost,
        "token_usage_observation": {
            "status": payload.pointer("/token_usage_visibility/status").cloned().unwrap_or(Value::String("unknown".to_string())),
            "reason": payload.pointer("/token_usage_visibility/unavailable_reason").cloned().unwrap_or(Value::Null),
            "note": "Host custom subagent token usage is visible only when the host supplies raw usage events; launch routing is still recorded in CCC status."
        }
    })
}
