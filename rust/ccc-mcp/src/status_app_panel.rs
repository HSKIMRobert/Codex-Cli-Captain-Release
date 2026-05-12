use crate::specialist_roles::{
    agent_id_for_role, generated_custom_agent_name, load_role_config_snapshot,
    normalize_dispatch_role_hint, role_for_agent_id, status_display_agent, status_display_role,
};
use crate::text_utils::summarize_text_for_visibility;
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn compact_task_label(value: &Value, field: &str) -> Value {
    value.get(field).cloned().unwrap_or(Value::Null)
}

fn compact_longway_rows(payload: &Value) -> Value {
    payload
        .pointer("/longway/phase_rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    json!({
                        "id": row.get("id").or_else(|| row.get("phase_id")).cloned().unwrap_or(Value::Null),
                        "title": row.get("title").or_else(|| row.get("label")).cloned().unwrap_or(Value::Null),
                        "status": row.get("status").cloned().unwrap_or(Value::Null),
                        "owner_agent": row.get("owner_agent").or_else(|| row.get("assigned_agent_id")).cloned().unwrap_or(Value::Null),
                        "summary": row.get("summary").cloned().unwrap_or(Value::Null),
                        "lifecycle": row.get("lifecycle").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into()
}

fn compact_planned_rows(payload: &Value) -> Value {
    let fallback = planned_row_fallback_routing(payload);
    payload
        .pointer("/longway/planned_rows")
        .or_else(|| payload.pointer("/scheduler/planned_rows/planned_rows"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let routing = planned_row_display_routing(row, fallback.as_ref());
                    json!({
                        "title": row.get("title").cloned().unwrap_or(Value::Null),
                        "status": row.get("status").cloned().unwrap_or(Value::String("planned".to_string())),
                        "planned_agent_id": row.get("planned_agent_id").cloned().unwrap_or(Value::Null),
                        "planned_role": row.get("planned_role").cloned().unwrap_or(Value::Null),
                        "display_agent_id": routing.agent,
                        "display_role": routing.role,
                        "model": routing.model,
                        "variant": routing.variant,
                        "reasoning": routing.reasoning,
                        "agent_source": routing.agent_source,
                        "model_source": routing.model_source,
                        "reasoning_source": routing.reasoning_source,
                        "recovery": row.get("recovery").cloned().unwrap_or(Value::Null),
                        "scope": row.get("scope").cloned().unwrap_or(Value::Null),
                        "acceptance": row.get("acceptance").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into()
}

#[derive(Clone)]
struct PlannedRowRouting {
    task_card_id: Option<String>,
    role: Value,
    agent: Value,
    model: Value,
    variant: Value,
    reasoning: Value,
    agent_source: Value,
    model_source: Value,
    reasoning_source: Value,
}

fn planned_row_fallback_routing(payload: &Value) -> Option<PlannedRowRouting> {
    let task = payload.get("current_task_card")?;
    let task_card_id = task
        .get("task_card_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let role = task.get("assigned_role").cloned().unwrap_or(Value::Null);
    let agent = task
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .map(display_custom_agent_name)
        .map(Value::String)
        .unwrap_or(Value::Null);
    let model = task
        .pointer("/runtime_dispatch/model")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/model"))
        .or_else(|| task.pointer("/delegation_plan/model"))
        .or_else(|| task.pointer("/role_config_snapshot/model"))
        .cloned()
        .unwrap_or(Value::Null);
    let variant = task
        .pointer("/runtime_dispatch/variant")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/variant"))
        .or_else(|| task.pointer("/delegation_plan/variant"))
        .or_else(|| task.pointer("/role_config_snapshot/variant"))
        .cloned()
        .unwrap_or(Value::Null);
    Some(PlannedRowRouting {
        task_card_id,
        role,
        agent,
        model,
        reasoning: variant.clone(),
        variant,
        agent_source: Value::String("current_task_matching_task_card".to_string()),
        model_source: Value::String("current_task_matching_task_card".to_string()),
        reasoning_source: Value::String("current_task_matching_task_card".to_string()),
    })
}

fn planned_row_display_routing(
    row: &Value,
    fallback: Option<&PlannedRowRouting>,
) -> PlannedRowRouting {
    let matching_fallback = fallback.filter(|routing| {
        let Some(row_task_card_id) = text_value(row, "/task_card_id") else {
            return false;
        };
        routing.task_card_id.as_deref() == Some(row_task_card_id)
    });
    let display_role = text_value(row, "/display_role")
        .filter(|value| *value != "unassigned")
        .map(str::to_string);
    let planned_role = display_role.clone().or_else(|| {
        text_value(row, "/planned_role")
            .filter(|value| *value != "unassigned")
            .map(str::to_string)
            .or_else(|| {
                text_value(row, "/role")
                    .filter(|value| *value != "unassigned")
                    .map(str::to_string)
            })
    });
    let display_agent = text_value(row, "/display_agent_id")
        .filter(|value| *value != "unassigned")
        .map(display_custom_agent_name);
    let (planned_agent, agent_source) = if let Some(agent) = display_agent {
        (Some(agent), Value::String("planned_row_input".to_string()))
    } else if let Some(agent) = text_value(row, "/planned_agent_id")
        .filter(|value| *value != "unassigned")
        .map(display_custom_agent_name)
    {
        (Some(agent), Value::String("planned_row_input".to_string()))
    } else if let Some(agent) = planned_role
        .as_deref()
        .and_then(agent_id_for_role)
        .map(display_custom_agent_name)
    {
        (Some(agent), Value::String("role_mapping".to_string()))
    } else {
        (
            None,
            matching_fallback
                .map(|routing| routing.agent_source.clone())
                .unwrap_or(Value::String("unassigned".to_string())),
        )
    };

    let role_value = planned_role
        .clone()
        .map(Value::String)
        .or_else(|| matching_fallback.map(|routing| routing.role.clone()))
        .unwrap_or(Value::Null);
    let snapshot = planned_row_role_config_snapshot(
        planned_role.as_deref(),
        planned_agent.as_deref(),
        role_value.as_str(),
    );
    let agent_value = planned_agent
        .map(Value::String)
        .or_else(|| matching_fallback.map(|routing| routing.agent.clone()))
        .unwrap_or(Value::Null);

    // The source fields make display precedence visible when planned-row input,
    // role config, and current-task fallback disagree.
    let (model, model_source) = if let Some(value) = row.get("model").cloned() {
        (value, Value::String("planned_row_input".to_string()))
    } else if let Some(value) = snapshot.get("model").cloned() {
        (value, Value::String("role_config".to_string()))
    } else if let Some(routing) = matching_fallback {
        (routing.model.clone(), routing.model_source.clone())
    } else {
        (Value::Null, Value::String("unassigned".to_string()))
    };
    let (variant, reasoning_source) =
        if let Some(value) = row.get("variant").or_else(|| row.get("reasoning")).cloned() {
            (value, Value::String("planned_row_input".to_string()))
        } else if let Some(value) = snapshot.get("variant").cloned() {
            (value, Value::String("role_config".to_string()))
        } else if let Some(routing) = matching_fallback {
            (routing.variant.clone(), routing.reasoning_source.clone())
        } else {
            (Value::Null, Value::String("unassigned".to_string()))
        };

    PlannedRowRouting {
        task_card_id: None,
        role: role_value,
        agent: agent_value,
        model,
        reasoning: variant.clone(),
        variant,
        agent_source,
        model_source,
        reasoning_source,
    }
}

fn planned_row_role_config_snapshot(
    planned_role: Option<&str>,
    planned_agent: Option<&str>,
    fallback_role: Option<&str>,
) -> Value {
    let mut role_candidates = Vec::new();
    if let Some(role) = planned_role {
        role_candidates.push(role.to_string());
    }
    if let Some(role) = planned_agent.and_then(role_for_display_agent) {
        role_candidates.push(role.to_string());
    }
    if let Some(role) = fallback_role {
        role_candidates.push(role.to_string());
    }

    role_candidates
        .into_iter()
        .find_map(|role| {
            let normalized = normalize_dispatch_role_hint(Some(&role), &role);
            if normalized.is_empty() || normalized == "unassigned" {
                return None;
            }
            let snapshot = load_role_config_snapshot(&normalized);
            if snapshot.get("model").and_then(Value::as_str).is_some()
                || snapshot.get("variant").and_then(Value::as_str).is_some()
            {
                Some(snapshot)
            } else {
                None
            }
        })
        .unwrap_or(Value::Null)
}

fn role_for_display_agent(agent: &str) -> Option<&'static str> {
    let normalized = agent.trim().trim_start_matches("ccc_");
    role_for_agent_id(normalized).or_else(|| {
        normalized
            .split_once('-')
            .and_then(|(agent_prefix, _)| role_for_agent_id(agent_prefix))
    })
}

fn display_custom_agent_name(agent_id: &str) -> String {
    let trimmed = agent_id.trim();
    if trimmed.is_empty()
        || trimmed == "unassigned"
        || trimmed.starts_with("ccc_")
        || trimmed.contains('-')
    {
        trimmed.to_string()
    } else {
        generated_custom_agent_name(trimmed)
    }
}

fn compact_current_task(payload: &Value) -> Value {
    let task = payload.get("current_task_card").unwrap_or(&Value::Null);
    if !task.is_object() {
        return Value::Null;
    }
    let model = task
        .pointer("/runtime_dispatch/model")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/model"))
        .or_else(|| task.pointer("/delegation_plan/model"))
        .or_else(|| task.pointer("/role_config_snapshot/model"))
        .cloned()
        .unwrap_or(Value::Null);
    let variant = task
        .pointer("/runtime_dispatch/variant")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/variant"))
        .or_else(|| task.pointer("/delegation_plan/variant"))
        .or_else(|| task.pointer("/role_config_snapshot/variant"))
        .cloned()
        .unwrap_or(Value::Null);
    let fast_mode = task
        .pointer("/runtime_dispatch/fast_mode")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/fast_mode"))
        .or_else(|| task.pointer("/delegation_plan/fast_mode"))
        .or_else(|| task.pointer("/role_config_snapshot/fast_mode"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "task_card_id": task
            .get("task_card_id")
            .cloned()
            .or_else(|| payload.get("active_task_card_id").cloned())
            .unwrap_or(Value::Null),
        "title": compact_task_label(task, "title"),
        "task_kind": compact_task_label(task, "task_kind"),
        "scope": compact_task_label(task, "scope"),
        "assigned_role": compact_task_label(task, "assigned_role"),
        "assigned_agent_id": compact_task_label(task, "assigned_agent_id"),
        "model": model,
        "variant": variant,
        "fast_mode": fast_mode,
        "verification_state": compact_task_label(task, "verification_state"),
        "verification_capsule": task.get("verification_capsule").cloned().unwrap_or(Value::Null),
        "delegated_ownership": task.get("delegated_ownership").cloned().unwrap_or(Value::Null),
        "review_pass_count": compact_task_label(task, "review_pass_count"),
        "lifecycle": task
            .get("subagent_lifecycle")
            .cloned()
            .or_else(|| task.get("review_lifecycle").cloned())
            .unwrap_or(Value::Null),
        "fan_in": task
            .get("subagent_fan_in")
            .cloned()
            .or_else(|| task.get("review_fan_in").cloned())
            .unwrap_or(Value::Null),
    })
}

fn compact_parallel_lanes(payload: &Value) -> Value {
    payload
        .pointer("/current_task_card/parallel_fanout/lanes")
        .and_then(Value::as_array)
        .map(|lanes| {
            lanes
                .iter()
                .map(|lane| {
                    json!({
                        "lane_id": lane.get("lane_id").cloned().unwrap_or(Value::Null),
                        "required": lane.get("required").cloned().unwrap_or(Value::Bool(false)),
                        "scope": lane.get("scope").cloned().unwrap_or(Value::Null),
                        "status": lane.pointer("/lifecycle/status").cloned().unwrap_or(Value::Null),
                        "child_agent_id": lane.pointer("/lifecycle/child_agent_id").cloned().unwrap_or(Value::Null),
                        "thread_id": lane.pointer("/lifecycle/thread_id").cloned().unwrap_or(Value::Null),
                        "summary": lane.pointer("/lifecycle/summary").cloned().unwrap_or(Value::Null),
                        "fan_in_status": lane.pointer("/fan_in/status").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into()
}

fn compact_active_subagents(payload: &Value) -> Value {
    payload
        .pointer("/host_subagent_state/active_subagents")
        .cloned()
        .or_else(|| payload.pointer("/active_task_delegations/active").cloned())
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn compact_subagent_activity(payload: &Value) -> Value {
    let current_task = payload.get("current_task_card").unwrap_or(&Value::Null);
    let current_model = current_task
        .pointer("/runtime_dispatch/model")
        .or_else(|| current_task.pointer("/delegation_plan/runtime_dispatch/model"))
        .or_else(|| current_task.pointer("/delegation_plan/model"))
        .or_else(|| current_task.pointer("/role_config_snapshot/model"));
    let current_variant = current_task
        .pointer("/runtime_dispatch/variant")
        .or_else(|| current_task.pointer("/delegation_plan/runtime_dispatch/variant"))
        .or_else(|| current_task.pointer("/delegation_plan/variant"))
        .or_else(|| current_task.pointer("/role_config_snapshot/variant"));
    let current_fast_mode = current_task
        .pointer("/runtime_dispatch/fast_mode")
        .or_else(|| current_task.pointer("/delegation_plan/runtime_dispatch/fast_mode"))
        .or_else(|| current_task.pointer("/delegation_plan/fast_mode"))
        .or_else(|| current_task.pointer("/role_config_snapshot/fast_mode"));
    let current_role = current_task
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let activities = payload
        .pointer("/host_subagent_state/subagent_activity")
        .cloned()
        .or_else(|| {
            payload
                .pointer("/host_subagent_state/active_subagents")
                .cloned()
        })
        .unwrap_or_else(|| Value::Array(Vec::new()));
    activities
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let mut enriched = item.clone();
                    if let Some(object) = enriched.as_object_mut() {
                        let activity_role = object
                            .get("assigned_role")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        let same_task_role = activity_role == current_role;
                        if same_task_role && !object.contains_key("model") {
                            if let Some(model) = current_model {
                                object.insert("model".to_string(), model.clone());
                            }
                        }
                        if same_task_role && !object.contains_key("variant") {
                            if let Some(variant) = current_variant {
                                object.insert("variant".to_string(), variant.clone());
                            }
                        }
                        if same_task_role && !object.contains_key("fast_mode") {
                            if let Some(fast_mode) = current_fast_mode {
                                object.insert("fast_mode".to_string(), fast_mode.clone());
                            }
                        }
                    }
                    enriched
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into()
}

fn compact_blockers(payload: &Value) -> Value {
    let mut blockers = Vec::new();
    if payload
        .pointer("/reclaim_plan/reclaim_needed_worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        blockers.push(json!({
            "kind": "reclaim_needed",
            "summary": payload.pointer("/reclaim_plan/summary").cloned().unwrap_or(Value::String("worker reclaim is needed".to_string())),
        }));
    }
    if let Some(action) = payload
        .pointer("/pending_captain_follow_up/action")
        .and_then(Value::as_str)
    {
        blockers.push(json!({
            "kind": "pending_captain_follow_up",
            "action": action,
            "status": payload.pointer("/pending_captain_follow_up/status").cloned().unwrap_or(Value::String("queued".to_string())),
        }));
    }
    if let Some(action) = payload
        .pointer("/latest_captain_intervention/chosen_next_action")
        .and_then(Value::as_str)
        .filter(|value| *value != "no_action")
    {
        blockers.push(json!({
            "kind": "captain_intervention",
            "action": action,
            "summary": payload.pointer("/latest_captain_intervention/intervention_rationale").cloned().unwrap_or(Value::Null),
        }));
    }
    Value::Array(blockers)
}

fn compact_warnings(payload: &Value) -> Value {
    let mut warnings = Vec::new();
    if payload
        .pointer("/longway/planning_context/workspace_root/confirmation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let retry_command = target_root_retry_command_from_payload(payload);
        warnings.push(json!({
            "kind": "target_root_confirmation_required",
            "status": "confirm_target_path",
            "root_kind": payload.pointer("/longway/planning_context/workspace_root/root_kind").cloned().unwrap_or(Value::Null),
            "root": payload.pointer("/longway/planning_context/workspace_root/root").cloned().unwrap_or(Value::Null),
            "confidence": payload.pointer("/longway/planning_context/workspace_root/confidence").cloned().unwrap_or(Value::Null),
            "reason": payload.pointer("/longway/planning_context/workspace_root/reason").cloned().unwrap_or(Value::Null),
            "candidates": payload.pointer("/longway/planning_context/workspace_root/candidates").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "retry_command": retry_command.map(Value::String).unwrap_or(Value::Null),
            "recommended_action": "Confirm the intended repo or document root by passing cwd/target_paths, or rerun the request with the target path.",
        }));
    }
    if payload
        .pointer("/long_session_mitigation/recommended_action")
        .and_then(Value::as_str)
        .is_some_and(|value| value != "continue")
    {
        warnings.push(json!({
            "kind": "long_session_mitigation",
            "recommended_action": payload.pointer("/long_session_mitigation/recommended_action").cloned().unwrap_or(Value::Null),
            "summary": payload.pointer("/long_session_mitigation/summary").cloned().unwrap_or(Value::Null),
        }));
    }
    if payload
        .pointer("/token_usage_visibility/status")
        .and_then(Value::as_str)
        == Some("unavailable")
    {
        warnings.push(json!({
            "kind": "token_usage_unavailable",
            "reason": payload.pointer("/token_usage_visibility/unavailable_reason").cloned().unwrap_or(Value::Null),
            "reason_code": payload.pointer("/token_usage_visibility/unavailable_reason_code").cloned().unwrap_or(Value::Null),
        }));
    }
    if payload
        .pointer("/code_graph/diagnostic_severity")
        .and_then(Value::as_str)
        == Some("warning")
    {
        warnings.push(json!({
            "kind": "code_graph_warning",
            "blocking": payload.pointer("/code_graph/blocking").cloned().unwrap_or(Value::Bool(false)),
            "reason": payload.pointer("/code_graph/reason").cloned().unwrap_or(Value::Null),
            "recommended_action": payload.pointer("/code_graph/recommended_action").cloned().unwrap_or(Value::Null),
        }));
    }
    Value::Array(warnings)
}

fn target_root_retry_command_from_payload(payload: &Value) -> Option<String> {
    let first_candidate = payload
        .pointer("/longway/planning_context/workspace_root/candidates")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;

    // Structured consumers get the same copyable command as the boxed text path.
    Some(format!(
        "$cap Use target_paths=[\"{first_candidate}\"] and continue this LongWay."
    ))
}

fn compact_target_workspace(payload: &Value) -> Value {
    let Some(root) = payload.pointer("/longway/planning_context/workspace_root") else {
        return Value::Null;
    };
    if !root.is_object() {
        return Value::Null;
    }
    json!({
        "root": root.get("root").cloned().unwrap_or(Value::Null),
        "root_kind": root.get("root_kind").cloned().unwrap_or(Value::Null),
        "confidence": root.get("confidence").cloned().unwrap_or(Value::Null),
        "confirmation_required": root.get("confirmation_required").cloned().unwrap_or(Value::Bool(false)),
        "reason": root.get("reason").cloned().unwrap_or(Value::Null),
        "candidate_count": root
            .get("candidates")
            .and_then(Value::as_array)
            .map(|items| Value::from(items.len()))
            .unwrap_or(Value::from(0)),
        "candidates": root.get("candidates").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    })
}

fn compact_workspace_state(payload: &Value) -> Value {
    let graph = payload
        .pointer("/longway/planning_context/graph")
        .or_else(|| payload.get("code_graph"))
        .cloned()
        .unwrap_or(Value::Null);
    let memory = payload
        .pointer("/longway/planning_context/memory")
        .or_else(|| payload.get("memory"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "graph": compact_graph_state(&graph),
        "graph_context": compact_graph_context_state(payload.get("graph_context").unwrap_or(&Value::Null)),
        "memory": compact_memory_state(&memory),
    })
}

fn compact_graph_context_state(readiness: &Value) -> Value {
    if !readiness.is_object() {
        return Value::Null;
    }
    json!({
        "provider": readiness.get("provider").cloned().unwrap_or(Value::Null),
        "readiness": readiness.get("readiness").cloned().unwrap_or(Value::Null),
        "reason": readiness.get("reason").cloned().unwrap_or(Value::Null),
        "fallback": readiness.get("fallback").cloned().unwrap_or(Value::Null),
        "artifact_state": readiness.get("artifact_state").cloned().unwrap_or(Value::Null),
        "artifacts": {
            "report": readiness.pointer("/artifacts/report/available").cloned().unwrap_or(Value::Bool(false)),
            "graph": readiness.pointer("/artifacts/graph/available").cloned().unwrap_or(Value::Bool(false)),
        },
        "stale": readiness.pointer("/stale/is_stale").cloned().unwrap_or(Value::Bool(false)),
    })
}

fn compact_graph_state(graph: &Value) -> Value {
    if !graph.is_object() {
        return Value::Null;
    }
    json!({
        "available": graph.get("available").cloned().unwrap_or(Value::Bool(false)),
        "repo_root": graph.get("repo_root").cloned().unwrap_or(Value::Null),
        "store_path": graph.get("store_path").cloned().unwrap_or(Value::Null),
        "file_count": graph.get("file_count").cloned().unwrap_or(Value::from(0)),
        "tolaria": compact_tolaria_state(graph.get("tolaria").unwrap_or(&Value::Null)),
    })
}

fn compact_memory_state(memory: &Value) -> Value {
    if !memory.is_object() {
        return Value::Null;
    }
    json!({
        "available": memory.get("available").cloned().unwrap_or(Value::Bool(false)),
        "enabled": memory.get("enabled").cloned().unwrap_or(Value::Bool(false)),
        "workspace": memory.get("workspace").cloned().unwrap_or(Value::Null),
        "entry_count": memory.get("entry_count").cloned().unwrap_or(Value::from(0)),
        "captain_instruction_count": memory.get("captain_instruction_count").cloned().unwrap_or(Value::from(0)),
        "tolaria": compact_tolaria_state(memory.get("tolaria").unwrap_or(&Value::Null)),
    })
}

fn compact_tolaria_state(value: &Value) -> Value {
    if !value.is_object() {
        return Value::Null;
    }
    json!({
        "enabled": value.get("enabled").cloned().unwrap_or(Value::Bool(false)),
        "available": value.get("available").cloned().unwrap_or(Value::Bool(false)),
        "state": value.get("state").cloned().unwrap_or(Value::Null),
        "relative_note_path": value.get("relative_note_path").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn create_codex_app_panel_payload(payload: &Value) -> Value {
    let fan_in_ready = payload
        .pointer("/run_truth_surface/fan_in_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "schema": "ccc.codex_app_panel.v1",
        "surface": "codex_app",
        "render_strategy": {
            "primary": "host_app_status_panel",
            "data_source": "ccc_status_or_ccc_activity",
            "fallback": "transcript_status_text",
            "mcp_apps_component": "available_if_host_renders_mcp_apps_resources",
            "resource_uri": CCC_APP_PANEL_RESOURCE_URI
        },
        "run": {
            "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "status": payload.get("status").cloned().unwrap_or(Value::Null),
            "stage": payload.get("stage").cloned().unwrap_or(Value::Null),
            "next_step": payload.get("next_step").cloned().unwrap_or(Value::Null),
            "can_advance": payload.get("can_advance").cloned().unwrap_or(Value::Null),
            "visibility_signature": payload.get("visibility_signature").cloned().unwrap_or(Value::Null),
        },
        "longway_progress": {
            "completed_phase_count": payload.pointer("/longway/completed_phase_count").cloned().unwrap_or(Value::Null),
            "phase_count": payload.pointer("/longway/phase_count").cloned().unwrap_or(Value::Null),
            "planned_row_count": payload.pointer("/longway/planned_row_count").cloned().unwrap_or(Value::Null),
            "current_item": payload.pointer("/longway/current_item").cloned().unwrap_or(Value::Null),
            "lifecycle_state": payload.pointer("/longway/lifecycle_state").cloned().unwrap_or(Value::Null),
            "rows": compact_longway_rows(payload),
            "planned_rows": compact_planned_rows(payload),
        },
        "scheduler": payload.get("scheduler").cloned().unwrap_or(Value::Null),
        "active_checkpoint": payload.get("active_checkpoint").cloned().unwrap_or(Value::Null),
        "task_session_state": payload.get("task_session_state").cloned().unwrap_or(Value::Null),
        "workflow_loop": payload.get("workflow_loop").cloned().unwrap_or(Value::Null),
        "lifecycle_hooks": payload.get("lifecycle_hooks").cloned().unwrap_or(Value::Null),
        "state_contract": payload.get("state_contract").cloned().unwrap_or(Value::Null),
        "post_fan_in_captain_decision": payload.get("post_fan_in_captain_decision").cloned().unwrap_or(Value::Null),
        "recovery_lane": payload.get("recovery_lane").cloned().unwrap_or(Value::Null),
        "current_task": compact_current_task(payload),
        "specialist_lanes": {
            "parallel_lanes": compact_parallel_lanes(payload),
            "active_subagents": compact_active_subagents(payload),
            "subagent_activity": compact_subagent_activity(payload),
            "host_subagent_state": payload.get("host_subagent_state").cloned().unwrap_or(Value::Null),
        },
        "fan_in": {
            "ready": fan_in_ready,
            "host_subagent_ready": payload.pointer("/host_subagent_state/fan_in_ready").cloned().unwrap_or(Value::Null),
            "worker_total": payload.pointer("/run_truth_surface/worker_total").cloned().unwrap_or(Value::Null),
            "worker_active": payload.pointer("/run_truth_surface/worker_active").cloned().unwrap_or(Value::Null),
        },
        "blockers": compact_blockers(payload),
        "target_workspace": compact_target_workspace(payload),
        "workspace_state": compact_workspace_state(payload),
        "captain_direct_mutation_guard": payload.get("captain_direct_mutation_guard").cloned().unwrap_or(Value::Null),
        "context_health": payload.get("context_health").cloned().unwrap_or(Value::Null),
        "registry_evidence": payload.get("registry_evidence").cloned().unwrap_or(Value::Null),
        "restart_handoff": payload.get("restart_handoff").cloned().unwrap_or(Value::Null),
        "next_captain_action": {
            "precedence": payload.pointer("/post_fan_in_captain_decision/precedence").cloned().unwrap_or(Value::Null),
            "allowed_action": payload.pointer("/captain_action_contract/allowed_action").cloned().unwrap_or(Value::Null),
            "required_action": payload.pointer("/captain_action_contract/required_action").cloned().unwrap_or(Value::Null),
            "direct_file_mutation_policy": payload.pointer("/captain_action_contract/direct_file_mutation_policy").cloned().unwrap_or_else(default_direct_file_mutation_policy),
            "resume_action": payload.pointer("/run_truth_surface/resume_action").cloned().unwrap_or(Value::Null),
            "run_state_next_action": payload.pointer("/run_state/next_action").cloned().unwrap_or(Value::Null),
        },
        "warnings": compact_warnings(payload),
    })
}

fn default_direct_file_mutation_policy() -> Value {
    json!({
        "allowed": false,
        "applies_to": ["apply_patch", "direct_shell_file_mutation", "file_edits", "mutation_commands"],
        "required_route": "specialist_fan_in_then_captain_review_merge",
        "required_action": "spawn_or_record_specialist",
        "requires_recorded_exception": "explicit_terminal_fallback_or_operator_override",
        "merge_gate": "specialist_fan_in_or_explicit_operator_override",
        "operator_override_required": true,
        "reason": "Host captain must not use apply_patch or direct shell mutation for specialist-owned files while a CCC run is active unless an explicit terminal fallback or operator override is recorded.",
    })
}

pub(crate) const CCC_APP_PANEL_RESOURCE_URI: &str = "ui://ccc/app-panel.html";
pub(crate) const CCC_APP_PANEL_MIME_TYPE: &str = "text/html;profile=mcp-app";

pub(crate) fn create_codex_app_panel_resource_html() -> String {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>CCC LongWay Panel</title>
  <style>
    :root {
      color-scheme: dark light;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: transparent;
      color: CanvasText;
    }
    body {
      margin: 0;
      padding: 12px;
      font-size: 13px;
      line-height: 1.45;
    }
    .panel {
      display: grid;
      gap: 10px;
    }
    .title {
      font-size: 14px;
      font-weight: 650;
    }
    .row {
      display: grid;
      grid-template-columns: 92px minmax(0, 1fr);
      gap: 8px;
      align-items: start;
    }
    .label {
      color: color-mix(in srgb, CanvasText 62%, transparent);
    }
    .value {
      min-width: 0;
      overflow-wrap: anywhere;
    }
    .muted {
      color: color-mix(in srgb, CanvasText 62%, transparent);
    }
    ul {
      margin: 0;
      padding-left: 18px;
    }
  </style>
</head>
<body>
  <main class="panel" id="panel">
    <div class="title">CCC LongWay Panel</div>
    <div class="muted">Waiting for CCC app panel data.</div>
  </main>
  <script>
    const bridge = window.openai || {};
    const data = bridge.toolOutput?.app_panel || bridge.toolOutput || bridge.structuredContent?.app_panel || {};
    const text = (value, fallback = "unknown") => value === undefined || value === null || value === "" ? fallback : String(value);
	    const html = (value, fallback = "unknown") => text(value, fallback).replace(/[&<>"']/g, (char) => ({
	      "&": "&amp;",
	      "<": "&lt;",
	      ">": "&gt;",
	      "\"": "&quot;",
	      "'": "&#39;",
	    }[char]));
	    const callsigns = {
	      tactician: "Executor",
	      scout: "Observer",
	      raider: "Marauder",
	      scribe: "Adjutant",
	      arbiter: "Arbiter",
	      sentinel: "Overseer",
	      companion_reader: "Probe",
	      companion_operator: "SCV",
	    };
	    const roleAgents = {
	      way: "tactician",
	      explorer: "scout",
	      "code specialist": "raider",
	      documenter: "scribe",
	      verifier: "arbiter",
	      sentinel: "sentinel",
	      companion_reader: "companion_reader",
	      companion_operator: "companion_operator",
	    };
	    const agentBase = (value) => {
	      const normalized = text(value, "").replace(/^ccc_/, "");
	      if (callsigns[normalized]) return normalized;
	      return normalized.split(/[_-]/)[0];
	    };
	    const displayAgent = (value) => {
	      const raw = text(value, "");
	      if (!raw || raw === "unassigned") return text(value, "unassigned");
	      if (!raw.startsWith("ccc_") && raw.includes("-")) return raw;
	      const base = agentBase(raw);
	      const callsign = callsigns[base];
	      if (!callsign) return raw;
	      return `${callsign}(${raw.startsWith("ccc_") ? raw : `ccc_${base}`})`;
	    };
	    const displayRole = (value) => {
	      const raw = text(value, "");
	      const agent = roleAgents[raw];
	      return agent && callsigns[agent] ? `${callsigns[agent]}(ccc_${agent})/${raw}` : text(value, "unassigned");
	    };
	    const list = (items, key = "kind") => Array.isArray(items) && items.length
	      ? `<ul>${items.slice(0, 6).map((item) => `<li>${html(item[key] || item.lane_id || item.title, "item")}${item.status ? ` (${html(item.status)})` : ""}</li>`).join("")}</ul>`
	      : '<span class="muted">none</span>';
	    const checklist = (items) => Array.isArray(items) && items.length
	      ? `<ul>${items.slice(0, 6).map((item) => `<li>${html(item.title || item.id || item.label, "item")} (${html(item.status)})${item.owner_agent ? ` · owner=${html(displayAgent(item.owner_agent))}` : ""}</li>`).join("")}</ul>`
	      : '<span class="muted">none</span>';
	    const subagents = (items) => Array.isArray(items) && items.length
	      ? `<ul>${items.slice(0, 6).map((item) => `<li>${html(displayAgent(item.child_agent_id), "subagent")} ${html(item.status)} · role=${html(displayRole(item.assigned_role), "unassigned")} · model=${html(item.model)} · variant=${html(item.variant)}${item.lane_id ? ` · lane=${html(item.lane_id)}` : ""} · ${html(item.task_title, "task")}</li>`).join("")}</ul>`
	      : '<span class="muted">none</span>';
    const panel = document.getElementById("panel");
    panel.innerHTML = `
      <div class="title">CCC LongWay Panel</div>
      <div class="row"><div class="label">Run</div><div class="value">${html(data.run?.run_id)} · ${html(data.run?.stage)} · next=${html(data.run?.next_step)}</div></div>
      <div class="row"><div class="label">LongWay</div><div class="value">${html(data.longway_progress?.completed_phase_count, "-")}/${html(data.longway_progress?.phase_count, "-")} current="${html(data.longway_progress?.current_item, "none")}"</div></div>
      <div class="row"><div class="label">Active Gate</div><div class="value">state=${html(data.state_contract?.state)} · gate=${html(data.state_contract?.active_gate)} · requires=${html(data.state_contract?.required_artifact)} · next=${html(data.state_contract?.next_step)}</div></div>
      <div class="row"><div class="label">Recovery</div><div class="value">status=${html(data.recovery_lane?.status, "clear")} · action=${html(data.recovery_lane?.recommended_action, "none")} · reclaim=${html(data.recovery_lane?.reclaim_replan_action, "none")} · targets=${html(data.recovery_lane?.target_count, "0")}</div></div>
      <div class="row"><div class="label">Checklist</div><div class="value">${checklist(data.longway_progress?.rows)}</div></div>
      <div class="row"><div class="label">Workspace</div><div class="value">graph=${html(data.workspace_state?.graph?.available)} files=${html(data.workspace_state?.graph?.file_count, "0")} · graphMirror=${html(data.workspace_state?.graph?.tolaria?.state, "off")} · graphContext=${html(data.workspace_state?.graph_context?.readiness, "unknown")} fallback=${html(data.workspace_state?.graph_context?.fallback, "none")} · memory=${html(data.workspace_state?.memory?.enabled)} entries=${html(data.workspace_state?.memory?.entry_count, "0")} · memoryMirror=${html(data.workspace_state?.memory?.tolaria?.state, "off")}</div></div>
      <div class="row"><div class="label">Mutation Guard</div><div class="value">state=${html(data.captain_direct_mutation_guard?.state, "unknown")} · changed=${html(data.captain_direct_mutation_guard?.changed_path_count, "0")}</div></div>
	      <div class="row"><div class="label">Task</div><div class="value">${html(data.current_task?.title, "none")} · ${html(displayRole(data.current_task?.assigned_role), "unassigned")} · ${html(displayAgent(data.current_task?.assigned_agent_id), "unassigned")} · model=${html(data.current_task?.model)} · variant=${html(data.current_task?.variant)}</div></div>
      <div class="row"><div class="label">Fan-in</div><div class="value">ready=${html(data.fan_in?.ready, "unknown")} · action=${html(data.next_captain_action?.allowed_action)}</div></div>
      <div class="row"><div class="label">Context</div><div class="value">${html(data.context_health?.status)}</div></div>
      <div class="row"><div class="label">Subagents</div><div class="value">${subagents(data.specialist_lanes?.subagent_activity)}</div></div>
      <div class="row"><div class="label">Lanes</div><div class="value">${list(data.specialist_lanes?.parallel_lanes, "lane_id")}</div></div>
      <div class="row"><div class="label">Warnings</div><div class="value">${list(data.warnings, "kind")}</div></div>
    `;
  </script>
</body>
</html>
"#
    .to_string()
}

pub(crate) fn create_codex_app_panel_markdown(app_panel: &Value) -> String {
    let run_id = display_value(app_panel, "/run/run_id", "unknown");
    let status = display_value(app_panel, "/run/status", "unknown");
    let stage = display_value(app_panel, "/run/stage", "unknown");
    let next_step = display_value(app_panel, "/run/next_step", "unknown");
    let completed = count_value(app_panel, "/longway_progress/completed_phase_count");
    let total = count_value(app_panel, "/longway_progress/phase_count");
    let lifecycle = display_value(app_panel, "/longway_progress/lifecycle_state", "unknown");
    let task_title = display_value(app_panel, "/current_task/title", "none");
    let task_role = display_value(app_panel, "/current_task/assigned_role", "unassigned");
    let task_agent = display_value(app_panel, "/current_task/assigned_agent_id", "unassigned");
    let task_role_display = status_display_role(&task_role);
    let task_agent_display = status_display_agent(&task_agent);
    let task_model = display_value(app_panel, "/current_task/model", "unknown");
    let task_variant = display_value(app_panel, "/current_task/variant", "unknown");
    let next_action = display_value(app_panel, "/next_captain_action/allowed_action", "unknown");
    let state_contract = summarize_state_contract(app_panel)
        .map(|line| format!("## Active Gate\n\n```\n{line}\n```\n\n"))
        .unwrap_or_default();
    let active_checkpoint = summarize_active_checkpoint(app_panel)
        .map(|line| format!("## Checkpoint\n\n```\n{line}\n```\n\n"))
        .unwrap_or_default();
    let recovery_lane = summarize_recovery_lane(app_panel)
        .map(|line| format!("## Recovery\n\n```\n{line}\n```\n\n"))
        .unwrap_or_default();
    let workspace_state = markdown_workspace_state(app_panel);
    let checklist = markdown_checklist_rows(app_panel, 12);
    let subagents = markdown_subagent_rows(app_panel, 8);
    let warnings = markdown_warning_rows(app_panel, 8);
    format!(
        "# CCC LongWay Panel\n\n\
         ## Run\n\n\
         - Run: `{run_id}`\n\
         - Status: `{status}` / `{stage}`\n\
         - Next: `{next_step}`\n\n\
         {state_contract}\
         {active_checkpoint}\
         {recovery_lane}\
         ## LongWay\n\n\
         - Progress: `{completed}/{total}`\n\
         - Lifecycle: `{lifecycle}`\n\n\
         ## Workspace State\n\n\
         {workspace_state}\n\n\
         ## Checklist\n\n\
         {checklist}\n\n\
         ## Current Task\n\n\
         - `{task_role_display}` / `{task_agent_display}`\n\
         - Model: `{task_model}` / `{task_variant}`\n\
         - Task: {task_title}\n\
         - Next Captain Action: `{next_action}`\n\n\
         ## Subagents\n\n\
         {subagents}\n\n\
         ## Warnings\n\n\
         {warnings}\n\n\
         ## Compact Status\n\n\
         ```text\n{}\n```\n\n\
         ## JSON Payload\n\n\
         ```json\n{}\n```\n",
        create_codex_app_panel_text(app_panel),
        serde_json::to_string_pretty(app_panel).unwrap_or_else(|_| "{}".to_string())
    )
}

pub(crate) fn write_codex_app_panel_artifact(
    run_directory: &Path,
    app_panel: &Value,
) -> io::Result<Value> {
    let artifact_directory = run_directory.join("temp-artifacts").join("app-panel");
    fs::create_dir_all(&artifact_directory)?;
    let markdown_path = artifact_directory.join("CCC_LONGWAY_PANEL.md");
    let json_path = artifact_directory.join("CCC_LONGWAY_PANEL.json");
    let markdown = create_codex_app_panel_markdown(app_panel);
    let json = serde_json::to_string_pretty(app_panel).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unable to encode app panel JSON artifact: {error}"),
        )
    })?;
    fs::write(&markdown_path, &markdown)?;
    fs::write(&json_path, &json)?;

    let latest_directory = latest_artifact_directory(run_directory);
    fs::create_dir_all(&latest_directory)?;
    let latest_markdown_path = latest_directory.join("CCC_LATEST_PANEL.md");
    let latest_json_path = latest_directory.join("CCC_LATEST_PANEL.json");
    fs::write(&latest_markdown_path, &markdown)?;
    fs::write(&latest_json_path, &json)?;
    Ok(json!({
        "kind": "ccc_app_panel_artifact",
        "markdown_path": normalize_artifact_path(markdown_path),
        "json_path": normalize_artifact_path(json_path),
        "latest_markdown_path": normalize_artifact_path(latest_markdown_path),
        "latest_json_path": normalize_artifact_path(latest_json_path),
    }))
}

fn latest_artifact_directory(run_directory: &Path) -> PathBuf {
    run_directory
        .parent()
        .and_then(Path::parent)
        .map(|workspace_directory| workspace_directory.join("temp-artifacts").join("app-panel"))
        .unwrap_or_else(|| run_directory.join("temp-artifacts").join("app-panel"))
}

fn normalize_artifact_path(path: PathBuf) -> String {
    fs::canonicalize(&path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn text_value<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
}

fn display_value(value: &Value, pointer: &str, fallback: &str) -> String {
    text_value(value, pointer).unwrap_or(fallback).to_string()
}

fn count_value(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .map(|number| number.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn summarize_items(value: &Value, pointer: &str, label_pointer: &str, limit: usize) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(limit)
                .filter_map(|item| {
                    let label = text_value(item, label_pointer)
                        .or_else(|| text_value(item, "/kind"))
                        .or_else(|| text_value(item, "/lane_id"))?;
                    let status = text_value(item, "/status")
                        .or_else(|| text_value(item, "/fan_in_status"))
                        .or_else(|| text_value(item, "/recommended_action"));
                    Some(match status {
                        Some(status) => format!("{label} ({status})"),
                        None => label.to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn create_codex_app_panel_text(app_panel: &Value) -> String {
    let completed_count = app_panel
        .pointer("/longway_progress/completed_phase_count")
        .and_then(Value::as_u64);
    let total_count = app_panel
        .pointer("/longway_progress/phase_count")
        .and_then(Value::as_u64);
    let completed = completed_count
        .map(|number| number.to_string())
        .unwrap_or_else(|| "-".to_string());
    let total = total_count
        .map(|number| number.to_string())
        .unwrap_or_else(|| "-".to_string());
    let mut lines = vec![
        "CCC LongWay".to_string(),
        format!("Progress: {completed}/{total} completed"),
        format!(
            "Gauge: {}",
            progress_gauge(completed_count.unwrap_or(0), total_count.unwrap_or(0), 24)
        ),
    ];

    if let Some(target_root) = summarize_target_root_confirmation(app_panel) {
        lines.push("Target Root:".to_string());
        lines.push(target_root);
        lines.extend(summarize_target_root_candidates(app_panel, 3));
        if let Some(hint) = target_root_follow_up_hint(app_panel) {
            lines.push(hint);
        }
    }

    let workspace_state = summarize_workspace_state(app_panel);
    if !workspace_state.is_empty() {
        lines.push("Workspace State:".to_string());
        lines.extend(workspace_state);
    }
    if let Some(state_contract) = summarize_state_contract(app_panel) {
        lines.push("Active Gate:".to_string());
        lines.push(state_contract);
    }
    if let Some(active_checkpoint) = summarize_active_checkpoint(app_panel) {
        lines.push("Checkpoint:".to_string());
        lines.push(active_checkpoint);
    }
    if let Some(recovery_lane) = summarize_recovery_lane(app_panel) {
        lines.push("Recovery:".to_string());
        lines.push(recovery_lane);
    }
    if let Some(workflow_loop) = summarize_workflow_loop(app_panel) {
        lines.push("Workflow Loop:".to_string());
        lines.push(workflow_loop);
    }
    if let Some(lifecycle_hooks) = summarize_lifecycle_hooks(app_panel) {
        lines.push("Lifecycle Hooks:".to_string());
        lines.push(lifecycle_hooks);
    }
    if let Some(mutation_guard) = summarize_mutation_guard(app_panel) {
        lines.push("Mutation Guard:".to_string());
        lines.push(mutation_guard);
    }

    let checklist = summarize_longway_rows(app_panel, 5);
    if !checklist.is_empty() {
        lines.push("Checklist:".to_string());
        lines.extend(checklist);
    }
    let planned_rows = summarize_planned_rows(app_panel, 8);
    if !planned_rows.is_empty() {
        lines.push("Planned Rows:".to_string());
        lines.extend(planned_rows);
    }

    let subagents = summarize_subagent_activity(app_panel, 4);
    if !subagents.is_empty() {
        lines.push("Subagents:".to_string());
        lines.extend(subagents);
    }

    boxed_text_panel(&lines)
}

fn summarize_state_contract(app_panel: &Value) -> Option<String> {
    let contract = app_panel
        .get("state_contract")
        .filter(|value| value.is_object())?;
    let state = text_value(contract, "/state")?;
    let active_gate = text_value(contract, "/active_gate").unwrap_or("unknown");
    let required_artifact = text_value(contract, "/required_artifact").unwrap_or("unspecified");
    let next_step = text_value(contract, "/next_step").unwrap_or("unknown");
    let allowed = contract
        .get("allowed_next_transitions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(4)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut line = format!(
        "state={} gate={} requires={} next={}",
        state.replace('_', "-"),
        active_gate.replace('_', "-"),
        required_artifact.replace('_', "-"),
        next_step.replace('_', "-")
    );
    if !allowed.is_empty() {
        line.push_str(&format!(" allowed={}", allowed.join(",")));
    }
    Some(line)
}

fn summarize_active_checkpoint(app_panel: &Value) -> Option<String> {
    let checkpoint = app_panel
        .get("active_checkpoint")
        .filter(|value| value.is_object())?;
    let gate = text_value(checkpoint, "/current_gate").unwrap_or("unknown");
    let task = text_value(checkpoint, "/task_card_id").unwrap_or("unknown");
    let role = text_value(checkpoint, "/assigned_role").unwrap_or("unassigned");
    let agent = text_value(checkpoint, "/assigned_agent_id").unwrap_or("unassigned");
    let delegated = text_value(checkpoint, "/delegated_work/summary").unwrap_or("delegated=0");
    let resume = text_value(checkpoint, "/resume_action").unwrap_or("unknown");
    let command =
        text_value(checkpoint, "/continuation_command").unwrap_or("$cap continue <run_id>");
    let late_state = text_value(checkpoint, "/late_output/state").unwrap_or("none");
    let late_count = checkpoint
        .pointer("/late_output/count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Some(format!(
        "gate={} task={} role={} agent={} {} resume={} continue=\"{}\" late={}({})",
        gate.replace('_', "-"),
        task,
        status_display_role(role),
        status_display_agent(agent),
        delegated,
        resume.replace('_', "-"),
        command,
        late_state.replace('_', "-"),
        late_count
    ))
}

fn summarize_recovery_lane(app_panel: &Value) -> Option<String> {
    let recovery_lane = app_panel
        .get("recovery_lane")
        .filter(|value| value.is_object())?;
    let status = text_value(recovery_lane, "/status")?;
    let recommended_action = text_value(recovery_lane, "/recommended_action").unwrap_or("none");
    let reclaim_action = text_value(recovery_lane, "/reclaim_replan_action").unwrap_or("none");
    let attention = recovery_lane
        .get("needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let target_count = recovery_lane
        .get("target_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let summary = text_value(recovery_lane, "/summary")
        .map(|value| format!(" summary=\"{}\"", summarize_text_for_visibility(value, 140)))
        .unwrap_or_default();

    Some(format!(
        "status={} action={} reclaim={} attention={} targets={}{}",
        status.replace('_', "-"),
        recommended_action.replace('_', "-"),
        reclaim_action.replace('_', "-"),
        attention,
        target_count,
        summary
    ))
}

fn summarize_lifecycle_hooks(app_panel: &Value) -> Option<String> {
    let hooks = app_panel
        .get("lifecycle_hooks")
        .filter(|value| value.is_object())?;
    let status = text_value(hooks, "/status")?;
    let active = hooks
        .get("active_tiers")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let failure_count = hooks
        .get("failure_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Some(format!(
        "status={} active={} failures={} internal=true",
        status.replace('_', "-"),
        if active.is_empty() {
            "none".to_string()
        } else {
            active.join(",")
        },
        failure_count
    ))
}

fn summarize_workflow_loop(app_panel: &Value) -> Option<String> {
    let workflow = app_panel
        .get("workflow_loop")
        .filter(|value| value.is_object())?;
    let current_stage = text_value(workflow, "/current_stage")?;
    let status = text_value(workflow, "/status").unwrap_or("active");
    let stages = workflow
        .get("stages")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let label = text_value(item, "/label")?;
                    let status = text_value(item, "/status").unwrap_or("unknown");
                    Some(format!(
                        "{}:{}",
                        label.replace(' ', "-"),
                        status.replace('_', "-")
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(format!(
        "status={} current={} stages={}",
        status.replace('_', "-"),
        current_stage.replace('_', "-"),
        if stages.is_empty() {
            "none".to_string()
        } else {
            stages.join(">")
        }
    ))
}

fn summarize_mutation_guard(app_panel: &Value) -> Option<String> {
    let guard = app_panel
        .get("captain_direct_mutation_guard")
        .filter(|value| value.is_object())?;
    let state = guard
        .get("state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let changed_path_count = guard
        .get("changed_path_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let paths = guard
        .get("changed_paths")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(3)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let suffix = if paths.is_empty() {
        String::new()
    } else {
        format!(" paths={}", paths.join(","))
    };
    Some(format!(
        "- state={} changed_paths={}{}",
        state.replace('_', "-"),
        changed_path_count,
        suffix
    ))
}

fn summarize_workspace_state(app_panel: &Value) -> Vec<String> {
    let graph = summarize_graph_state(app_panel);
    let graph_context = summarize_graph_context_state(app_panel);
    let registry = summarize_registry_state(app_panel);
    let memory = summarize_memory_state(app_panel);
    [graph, graph_context, registry, memory]
        .into_iter()
        .flatten()
        .collect()
}

fn summarize_graph_state(app_panel: &Value) -> Option<String> {
    let available = app_panel
        .pointer("/workspace_state/graph/available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let file_count = app_panel
        .pointer("/workspace_state/graph/file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let tolaria = summarize_tolaria_state(app_panel, "/workspace_state/graph/tolaria");
    if !available && tolaria.is_none() {
        return None;
    }
    Some(format!(
        "Graph: available={} files={}{}",
        available,
        file_count,
        tolaria
            .map(|value| format!(" mirror={value}"))
            .unwrap_or_default()
    ))
}

fn summarize_graph_context_state(app_panel: &Value) -> Option<String> {
    let graph_context = app_panel
        .pointer("/workspace_state/graph_context")
        .filter(|value| value.is_object())?;
    let readiness = graph_context
        .get("readiness")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let provider = graph_context
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("graphify");
    let fallback = graph_context
        .get("fallback")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let artifact_state = graph_context
        .get("artifact_state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Some(format!(
        "Graph Context: provider={provider} readiness={readiness} fallback={fallback} artifacts={artifact_state}"
    ))
}

fn summarize_memory_state(app_panel: &Value) -> Option<String> {
    let available = app_panel
        .pointer("/workspace_state/memory/available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let enabled = app_panel
        .pointer("/workspace_state/memory/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let entry_count = app_panel
        .pointer("/workspace_state/memory/entry_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let tolaria = summarize_tolaria_state(app_panel, "/workspace_state/memory/tolaria");
    if !available && !enabled && tolaria.is_none() {
        return None;
    }
    Some(format!(
        "Memory: enabled={} entries={}{}",
        enabled,
        entry_count,
        tolaria
            .map(|value| format!(" mirror={value}"))
            .unwrap_or_default()
    ))
}

fn summarize_registry_state(app_panel: &Value) -> Option<String> {
    let registry = app_panel.get("registry_evidence")?;
    if registry.is_null() {
        return None;
    }
    let agent = registry
        .get("agent_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("agent");
    let status = registry
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let ssl_status = registry
        .get("manifest_status")
        .and_then(Value::as_str)
        .or_else(|| {
            registry
                .pointer("/skill_ssl_manifest/status")
                .and_then(Value::as_str)
        })
        .unwrap_or("missing");
    Some(format!(
        "Registry: {} status={status} ssl={ssl_status}",
        status_display_agent(agent)
    ))
}

fn summarize_tolaria_state(app_panel: &Value, pointer: &str) -> Option<String> {
    let state = text_value(app_panel, &format!("{pointer}/state"))?;
    let enabled = app_panel
        .pointer(&format!("{pointer}/enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !enabled {
        return None;
    }
    let note = text_value(app_panel, &format!("{pointer}/relative_note_path"))
        .map(|value| format!(" {value}"))
        .unwrap_or_default();
    Some(format!("{state}{note}"))
}

fn markdown_workspace_state(app_panel: &Value) -> String {
    let rows = summarize_workspace_state(app_panel);
    if rows.is_empty() {
        "- _No graph or memory state available._".to_string()
    } else {
        rows.into_iter()
            .map(|row| format!("- {row}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn progress_gauge(completed: u64, total: u64, width: usize) -> String {
    if total == 0 {
        return format!("[{}] --%", "-".repeat(width));
    }
    let clamped_completed = completed.min(total);
    let filled = ((clamped_completed as usize) * width + (total as usize / 2)) / total as usize;
    let percent = (clamped_completed * 100) / total;
    format!(
        "[{}{}] {percent}%",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    )
}

fn summarize_target_root_confirmation(app_panel: &Value) -> Option<String> {
    let confirmation_required = app_panel
        .pointer("/target_workspace/confirmation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirmation_required {
        return None;
    }
    let root_kind = text_value(app_panel, "/target_workspace/root_kind").unwrap_or("unknown");
    let confidence = text_value(app_panel, "/target_workspace/confidence").unwrap_or("unknown");
    let candidate_count = app_panel
        .pointer("/target_workspace/candidate_count")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            app_panel
                .pointer("/target_workspace/candidates")
                .and_then(Value::as_array)
                .map(|items| items.len() as u64)
                .unwrap_or(0)
        });
    Some(format!(
        "[!] Confirm target path ({root_kind}, confidence={confidence}, candidates={candidate_count})"
    ))
}

fn summarize_target_root_candidates(app_panel: &Value, limit: usize) -> Vec<String> {
    if !target_root_confirmation_required(app_panel) {
        return Vec::new();
    }
    app_panel
        .pointer("/target_workspace/candidates")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(limit)
                .enumerate()
                .filter_map(|(index, item)| {
                    let candidate = item.as_str()?;
                    let label = target_root_candidate_label(candidate);
                    Some(format!("    {}. {label} -> {candidate}", index + 1))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn target_root_follow_up_hint(app_panel: &Value) -> Option<String> {
    if !target_root_confirmation_required(app_panel) {
        return None;
    }
    let first_candidate = app_panel
        .pointer("/target_workspace/candidates")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)?;

    // Keep the prompt copyable and host-agnostic: both Codex App and CLI can
    // reuse the same natural-language `$cap` follow-up without needing a native
    // target-root picker.
    Some(format!(
        "Retry: $cap Use target_paths=[\"{first_candidate}\"] and continue this LongWay."
    ))
}

fn target_root_confirmation_required(app_panel: &Value) -> bool {
    app_panel
        .pointer("/target_workspace/confirmation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn target_root_candidate_label(candidate: &str) -> String {
    Path::new(candidate)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(candidate)
        .to_string()
}

fn summarize_longway_rows(value: &Value, limit: usize) -> Vec<String> {
    value
        .pointer("/longway_progress/rows")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(limit)
                .filter_map(|item| {
                    let title = text_value(item, "/title")?;
                    let status = text_value(item, "/status").unwrap_or("unknown");
                    let owner = text_value(item, "/owner_agent");
                    let symbol = status_symbol(status);
                    Some(match owner {
                        Some(owner) => format!(
                            "{symbol} {title} ({status}, owner={})",
                            status_display_agent(owner)
                        ),
                        None => format!("{symbol} {title} ({status})"),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn status_symbol(status: &str) -> &'static str {
    match status {
        "completed" | "passed" | "merged" | "materialized" => "[x]",
        "spawned" | "acknowledged" | "opened" | "running" | "active" | "in_progress"
        | "await_fan_in" => "[~]",
        "failed" | "blocked" | "stalled" | "cancelled" | "reclaimed" => "[!]",
        _ => "[ ]",
    }
}

fn markdown_checklist_rows(app_panel: &Value, limit: usize) -> String {
    let mut rows = summarize_longway_rows(app_panel, limit);
    let planned_rows = summarize_planned_rows(app_panel, limit);
    if !planned_rows.is_empty() {
        rows.push("_Planned rows_".to_string());
        rows.extend(planned_rows);
    }
    if rows.is_empty() {
        "- _No checklist rows available._".to_string()
    } else {
        rows.into_iter()
            .map(|row| format!("- {row}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn markdown_subagent_rows(app_panel: &Value, limit: usize) -> String {
    let rows = summarize_subagent_activity(app_panel, limit);
    if rows.is_empty() {
        "- _No active or recent subagents._".to_string()
    } else {
        rows.into_iter()
            .map(|row| format!("- {row}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn markdown_warning_rows(app_panel: &Value, limit: usize) -> String {
    let rows = summarize_items(app_panel, "/warnings", "/kind", limit);
    if rows.is_empty() {
        "- _No warnings._".to_string()
    } else {
        rows.into_iter()
            .map(|row| format!("- {row}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn summarize_subagent_activity(value: &Value, limit: usize) -> Vec<String> {
    value
        .pointer("/specialist_lanes/subagent_activity")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(limit)
                .filter_map(|item| {
                    let child_agent = text_value(item, "/child_agent_id")?;
                    let role = text_value(item, "/assigned_role").unwrap_or("unassigned");
                    let display_role = status_display_role(role);
                    let status = text_value(item, "/status").unwrap_or("unknown");
                    let task = text_value(item, "/task_title").unwrap_or("task");
                    let model = text_value(item, "/model").unwrap_or("unknown");
                    let variant = text_value(item, "/variant").unwrap_or("unknown");
                    let lane = text_value(item, "/lane_id");
                    let next_action = text_value(item, "/next_action");
                    let next = next_action
                        .map(|value| format!(" next={value}"))
                        .unwrap_or_default();
                    Some(match lane {
                        Some(lane) => format!(
                            "{} {status} role={display_role} model={model} variant={variant} lane={lane} task=\"{task}\"{next}",
                            status_display_agent(child_agent)
                        ),
                        None => format!(
                            "{} {status} role={display_role} model={model} variant={variant} task=\"{task}\"{next}",
                            status_display_agent(child_agent)
                        ),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn summarize_planned_rows(value: &Value, limit: usize) -> Vec<String> {
    value
        .pointer("/longway_progress/planned_rows")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(limit)
                .filter_map(|item| {
                    let title = text_value(item, "/title")?;
                    let status = text_value(item, "/status").unwrap_or("planned");
                    let agent = text_value(item, "/display_agent_id")
                        .or_else(|| text_value(item, "/planned_agent_id"))
                        .filter(|value| *value != "unassigned");
                    let model = text_value(item, "/model").unwrap_or("unknown");
                    let reasoning = text_value(item, "/reasoning")
                        .or_else(|| text_value(item, "/variant"))
                        .unwrap_or("unknown");
                    let source = planned_row_source_summary(item);
                    let recovery = planned_row_recovery_summary(item);
                    let symbol = status_symbol(status);
                    let route = agent
                        .map(|agent| {
                            format!(
                                " -> {} model={model} reasoning={reasoning}{recovery} {source}",
                                status_display_agent(agent)
                            )
                        })
                        .unwrap_or_else(|| {
                            format!(" -> model={model} reasoning={reasoning}{recovery} {source}")
                        });
                    Some(format!("{symbol} {title}{route}"))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn planned_row_recovery_summary(item: &Value) -> String {
    let Some(recovery) = item.get("recovery").filter(|value| value.is_object()) else {
        return String::new();
    };
    let mode = text_value(recovery, "/mode").unwrap_or("fallback");
    let reason = text_value(recovery, "/reason")
        .map(|value| format!(" reason={}", value.replace('_', "-")))
        .unwrap_or_default();
    let primary = text_value(recovery, "/primary_status")
        .map(|value| format!(" primary={}", value.replace('_', "-")))
        .unwrap_or_default();
    format!(" recovered={mode}{reason}{primary}")
}

fn planned_row_source_summary(item: &Value) -> String {
    // Keep the route projection auditable without dumping the full registry payload.
    let agent_source = text_value(item, "/agent_source")
        .or_else(|| text_value(item, "/display_source"))
        .unwrap_or("unknown");
    let model_source = text_value(item, "/model_source").unwrap_or("unknown");
    let reasoning_source = text_value(item, "/reasoning_source").unwrap_or("unknown");
    format!("sources=agent:{agent_source},model:{model_source},reasoning:{reasoning_source}")
}

fn boxed_text_panel(lines: &[String]) -> String {
    let content_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or("CCC LongWay".len())
        .clamp(40, 118);
    let border = format!("+{}+", "-".repeat(content_width + 2));
    let mut output = Vec::with_capacity(lines.len() + 2);
    output.push(border.clone());
    for line in lines {
        let mut current = line.as_str();
        while current.chars().count() > content_width {
            let split_at = split_at_width(current, content_width);
            output.push(format!(
                "| {:width$} |",
                &current[..split_at],
                width = content_width
            ));
            current = current[split_at..].trim_start();
        }
        output.push(format!("| {:width$} |", current, width = content_width));
    }
    output.push(border);
    output.join("\n")
}

fn split_at_width(text: &str, width: usize) -> usize {
    let mut last_space = None;
    let mut end = text.len();
    for (index, (byte_index, character)) in text.char_indices().enumerate() {
        if index >= width {
            end = byte_index;
            break;
        }
        if character.is_whitespace() {
            last_space = Some(byte_index);
        }
    }
    last_space.filter(|space| *space > 0).unwrap_or(end)
}
