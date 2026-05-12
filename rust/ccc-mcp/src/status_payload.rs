use crate::code_graph::create_code_graph_status_payload;
use crate::graph_context::create_graph_context_readiness_payload;
use crate::host_subagent_lifecycle::{
    create_host_subagent_state_payload, task_card_has_explicit_subagent_fallback_reason,
    task_card_has_terminal_or_merged_host_subagent_state, task_card_required_parallel_fan_in_ready,
    task_card_subagent_fallback_ready,
};
use crate::install_check::create_server_identity_payload;
use crate::lifecycle_hooks::create_lifecycle_hook_tiers_payload;
use crate::long_session::create_long_session_mitigation_payload;
use crate::memory::create_memory_status_payload;
use crate::request_routing::create_assignment_quality_payload;
use crate::run_locator::{create_ccc_run_ref, ResolvedRunLocator};
use crate::scheduler_transition::read_latest_scheduler_transition;
use crate::specialist_roles::{
    fallback_specialist_execution_mode, load_output_config, load_output_verbosity,
    preferred_specialist_execution_mode,
};
use crate::status_app_panel::create_codex_app_panel_payload;
use crate::status_cost_routing::create_cost_routing_payload;
use crate::status_render::create_visibility_signature;
use crate::text_utils::summarize_text_for_visibility;
use crate::token_usage::{create_token_usage_payload, create_token_usage_visibility_payload};
use crate::worker_events::resolve_delegation_message_preview;
use crate::worker_lifecycle::{
    create_reclaim_plan_payload, create_worker_lifecycle_view, create_worker_visibility_payload,
    refresh_running_delegation_heartbeat,
};
use crate::worktree_guard::create_captain_direct_mutation_guard;
use crate::{
    load_runtime_config, load_runtime_config_from_path, read_json_document,
    read_optional_json_document, read_optional_toml_document, SessionContext,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn create_current_task_card_payload(
    run_directory: &Path,
    active_task_card_id: Option<&str>,
) -> io::Result<Value> {
    let Some(task_card_id) = active_task_card_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(Value::Null);
    };

    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let Some(task_card) = read_optional_json_document(&task_card_file)? else {
        return Ok(Value::Null);
    };

    let route_enforcement_state = route_enforcement_state_for_task_card(&task_card);
    let assignment_quality = create_assignment_quality_payload(&task_card);
    let verification_capsule = create_verification_capsule_payload(&task_card);
    let delegated_ownership = create_delegated_ownership_payload(&task_card);

    Ok(json!({
        "file": task_card_file.to_string_lossy(),
        "run_id": task_card.get("run_id").cloned().unwrap_or(Value::Null),
        "task_card_id": task_card.get("task_card_id").cloned().unwrap_or(Value::String(task_card_id.to_string())),
        "title": task_card.get("title").cloned().unwrap_or(Value::Null),
        "intent": task_card.get("intent").cloned().unwrap_or(Value::Null),
        "scope": task_card.get("scope").cloned().unwrap_or(Value::Null),
        "acceptance": task_card.get("acceptance").cloned().unwrap_or(Value::Null),
        "execution_prompt": task_card.get("execution_prompt").cloned().unwrap_or(Value::Null),
        "status": task_card.get("status").cloned().unwrap_or(Value::Null),
        "task_kind": task_card.get("task_kind").cloned().unwrap_or(Value::Null),
        "review_of_task_card_ids": task_card.get("review_of_task_card_ids").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "orchestrator_review_gate": task_card.get("orchestrator_review_gate").cloned().unwrap_or(Value::Null),
        "verification_state": task_card.get("verification_state").cloned().unwrap_or(Value::Null),
        "review_pass_count": task_card.get("review_pass_count").cloned().unwrap_or(Value::Null),
        "review_policy": task_card.get("review_policy").cloned().unwrap_or(Value::Null),
        "completion_discipline": task_card.get("completion_discipline").cloned().unwrap_or(Value::Null),
        "assigned_role": task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "sandbox_mode": task_card.get("sandbox_mode").cloned().unwrap_or(Value::Null),
        "sandbox_rationale": task_card.get("sandbox_rationale").cloned().unwrap_or(Value::Null),
        "role_config_snapshot": task_card.get("role_config_snapshot").cloned().unwrap_or(Value::Null),
        "delegation_plan": task_card.get("delegation_plan").cloned().unwrap_or(Value::Null),
        "verification_capsule": verification_capsule,
        "delegated_ownership": delegated_ownership,
        "route_enforcement_state": route_enforcement_state,
        "assignment_quality": assignment_quality,
        "routing_summary": bounded_status_routing_summary(&task_card).unwrap_or(Value::Null),
        "routing_trace": bounded_status_routing_trace(&task_card, "task_card").unwrap_or(Value::Null),
        "evidence_links": task_card.get("evidence_links").cloned().unwrap_or(Value::Null),
        "result_links": task_card.get("result_links").cloned().unwrap_or(Value::Null),
        "subagent_lifecycle": task_card.get("subagent_lifecycle").cloned().unwrap_or(Value::Null),
        "review_lifecycle": task_card.get("review_lifecycle").cloned().unwrap_or(Value::Null),
        "subagent_fan_in": task_card.get("subagent_fan_in").cloned().unwrap_or(Value::Null),
        "worker_result_envelope": task_card.get("subagent_fan_in").cloned().unwrap_or(Value::Null),
        "late_subagent_output": task_card.get("late_subagent_output").cloned().unwrap_or(Value::Null),
        "review_fan_in": task_card.get("review_fan_in").cloned().unwrap_or(Value::Null),
        "captain_intervention": task_card.get("captain_intervention").cloned().unwrap_or(Value::Null),
        "captain_intervention_history": task_card.get("captain_intervention_history").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "sentinel_intervention": task_card.get("sentinel_intervention").cloned().unwrap_or(Value::Null),
        "sentinel_intervention_history": task_card.get("sentinel_intervention_history").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "subagent_policy_drift": task_card.get("subagent_policy_drift").cloned().unwrap_or(Value::Null),
        "subagent_fallback": task_card.get("subagent_fallback").cloned().unwrap_or(Value::Null),
        "parallel_fanout": task_card.get("parallel_fanout").cloned().unwrap_or(Value::Null),
        "planned_longway_row": task_card.get("planned_longway_row").cloned().unwrap_or(Value::Null),
        "latest_model_launch": task_card.get("latest_model_launch").cloned().unwrap_or(Value::Null),
    }))
}

fn route_enforcement_state_for_task_card(task_card: &Value) -> Value {
    if task_card
        .get("subagent_fallback")
        .is_some_and(|value| !value.is_null())
    {
        let fallback_terminal_ready = match task_card_required_parallel_fan_in_ready(task_card) {
            Some(parallel_ready) => parallel_ready,
            None => task_card_has_terminal_or_merged_host_subagent_state(task_card),
        };
        if task_card_has_explicit_subagent_fallback_reason(task_card) && fallback_terminal_ready {
            return Value::String("degraded_direct_host_fallback_recorded".to_string());
        }
        return Value::String(
            "degraded_direct_host_fallback_pending_terminal_specialist_state".to_string(),
        );
    }
    if task_card
        .get("subagent_lifecycle")
        .is_some_and(|value| !value.is_null())
    {
        return Value::String("host_subagent_lifecycle_recorded".to_string());
    }
    if task_card
        .get("delegation_plan")
        .is_some_and(|value| !value.is_null())
    {
        return Value::String("host_subagent_spawn_required".to_string());
    }
    Value::String("no_delegation_contract".to_string())
}

fn bounded_status_routing_summary(value: &Value) -> Option<Value> {
    value
        .get("routing_summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Value::String(summarize_text_for_visibility(value, 240)))
}

fn first_bounded_status_trace_string(
    trace: &serde_json::Map<String, Value>,
    keys: &[&str],
    max_chars: usize,
) -> Option<Value> {
    keys.iter().find_map(|key| {
        trace
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
    })
}

fn bounded_status_trace_strings(
    trace: &serde_json::Map<String, Value>,
    keys: &[&str],
    max_items: usize,
    max_chars: usize,
) -> Option<Value> {
    keys.iter().find_map(|key| {
        let value = trace.get(*key)?;
        let strings = if let Some(items) = value.as_array() {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .take(max_items)
                .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
                .collect::<Vec<_>>()
        } else {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    vec![Value::String(summarize_text_for_visibility(
                        value, max_chars,
                    ))]
                })
                .unwrap_or_default()
        };
        (!strings.is_empty()).then_some(Value::Array(strings))
    })
}

fn bounded_status_trace_bool(trace: &serde_json::Map<String, Value>, key: &str) -> Option<Value> {
    trace.get(key).and_then(Value::as_bool).map(Value::Bool)
}

fn bounded_status_routing_trace(value: &Value, default_source: &str) -> Option<Value> {
    let trace = value.get("routing_trace")?.as_object()?;
    let mut bounded = serde_json::Map::new();
    bounded.insert(
        "source".to_string(),
        first_bounded_status_trace_string(trace, &["source"], 80)
            .unwrap_or_else(|| Value::String(default_source.to_string())),
    );

    for (output_key, input_keys, max_chars) in [
        ("query", &["query_kind", "query", "kind"][..], 80),
        ("request_shape", &["request_shape"][..], 80),
        ("mutation_intent", &["mutation_intent"][..], 80),
        ("selected_category", &["selected_category"][..], 80),
        ("selected_skill_id", &["selected_skill_id"][..], 120),
        ("selected_skill_name", &["selected_skill_name"][..], 120),
        ("risk", &["risk"][..], 40),
        ("evidence_need", &["evidence_need"][..], 120),
        ("verification_need", &["verification_need"][..], 120),
        ("fallback_role", &["fallback_role"][..], 80),
        ("selected_role", &["selected_role"][..], 80),
        ("selected_agent_id", &["selected_agent_id"][..], 80),
        ("reason", &["reason", "rationale"][..], 240),
        ("summary", &["summary"][..], 240),
    ] {
        if let Some(value) = first_bounded_status_trace_string(trace, input_keys, max_chars) {
            bounded.insert(output_key.to_string(), value);
        }
    }

    for (output_key, input_keys, max_items, max_chars) in [
        ("paths", &["paths", "path"][..], 8, 160),
        ("terms", &["terms", "term", "search", "text"][..], 8, 120),
    ] {
        if let Some(value) = bounded_status_trace_strings(trace, input_keys, max_items, max_chars) {
            bounded.insert(output_key.to_string(), value);
        }
    }

    for key in [
        "companion_route_enforced",
        "release_install_script_repair_guard",
    ] {
        if let Some(value) = bounded_status_trace_bool(trace, key) {
            bounded.insert(key.to_string(), value);
        }
    }

    (bounded.len() > 1).then_some(Value::Object(bounded))
}

fn latest_intervention_payload(
    current_task_card: &Value,
    run_record: &Value,
    task_card_key: &str,
    run_record_key: &str,
) -> Value {
    primary_non_null_or_fallback_payload(
        current_task_card,
        task_card_key,
        run_record,
        run_record_key,
    )
}

struct CccStatusTruthFallbackFields {
    way_clarification_request: Value,
    prompt_refinement: Value,
    review_policy: Value,
    completion_discipline: Value,
    latest_captain_intervention: Value,
    latest_sentinel_intervention: Value,
}

fn create_ccc_status_truth_fallback_fields(
    run_record: &Value,
    current_task_card: &Value,
    longway: &Value,
) -> CccStatusTruthFallbackFields {
    CccStatusTruthFallbackFields {
        way_clarification_request: primary_non_null_or_fallback_payload(
            run_record,
            "way_clarification_request",
            longway,
            "way_clarification_request",
        ),
        prompt_refinement: primary_non_null_or_fallback_payload(
            run_record,
            "prompt_refinement",
            longway,
            "prompt_refinement",
        ),
        review_policy: primary_non_null_or_fallback_payload(
            current_task_card,
            "review_policy",
            run_record,
            "review_policy",
        ),
        completion_discipline: primary_non_null_or_fallback_payload(
            current_task_card,
            "completion_discipline",
            run_record,
            "completion_discipline",
        ),
        latest_captain_intervention: latest_intervention_payload(
            current_task_card,
            run_record,
            "captain_intervention",
            "latest_captain_intervention",
        ),
        latest_sentinel_intervention: latest_intervention_payload(
            current_task_card,
            run_record,
            "sentinel_intervention",
            "latest_sentinel_intervention",
        ),
    }
}

fn primary_non_null_or_fallback_payload(
    primary: &Value,
    primary_key: &str,
    fallback: &Value,
    fallback_key: &str,
) -> Value {
    primary
        .get(primary_key)
        .cloned()
        .filter(|value| !value.is_null())
        .or_else(|| fallback.get(fallback_key).cloned())
        .unwrap_or(Value::Null)
}

fn create_latest_delegate_result_payload(
    run_directory: &Path,
    active_task_card_id: Option<&str>,
) -> io::Result<Value> {
    let Some(task_card_id) = active_task_card_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(Value::Null);
    };
    let delegations_directory = run_directory.join("delegations");
    let entries = match fs::read_dir(&delegations_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Value::Null),
        Err(error) => return Err(error),
    };
    let runtime_config = load_runtime_config()?;
    let mut latest_updated_at = String::new();
    let mut latest_payload = Value::Null;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if path.extension().and_then(|value| value.to_str()) != Some("json")
            || file_name.ends_with(".result.json")
        {
            continue;
        }

        let delegation =
            refresh_running_delegation_heartbeat(run_directory, &path, read_json_document(&path)?)?;
        if delegation.get("task_card_id").and_then(Value::as_str) != Some(task_card_id) {
            continue;
        }
        let status = delegation
            .get("child_agent")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if !matches!(status, "completed" | "failed" | "cancelled") {
            continue;
        }

        let updated_at = delegation
            .get("updated_at")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if !latest_updated_at.is_empty() && updated_at <= latest_updated_at {
            continue;
        }

        let assistant_message_preview =
            resolve_delegation_message_preview(run_directory, &delegation);
        latest_updated_at = updated_at.clone();
        latest_payload = json!({
            "delegation_id": delegation.get("delegation_id").cloned().unwrap_or(Value::Null),
            "agent_id": delegation
                .get("child_agent")
                .and_then(|value| value.get("agent_id"))
                .cloned()
                .unwrap_or(Value::Null),
            "role": delegation
                .get("child_agent")
                .and_then(|value| value.get("role"))
                .cloned()
                .unwrap_or(Value::Null),
            "status": status,
            "updated_at": updated_at,
            "result_summary": delegation.get("result_summary").cloned().unwrap_or(Value::Null),
            "assistant_message_preview": assistant_message_preview,
            "worker_lifecycle": create_worker_lifecycle_view(&delegation, &runtime_config),
        });
    }

    Ok(latest_payload)
}

fn create_longway_payload(run_directory: &Path) -> io::Result<Value> {
    let checklist_file = run_directory.join("longway.json");
    let Some(longway) = read_optional_json_document(&checklist_file)? else {
        return Ok(Value::Null);
    };

    let phases = longway
        .get("phases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let planned_rows: Vec<Value> = longway
        .get("planned_rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(sanitize_longway_planned_row)
                .map(|row| sync_planned_row_projection_status(run_directory, row))
                .collect()
        })
        .unwrap_or_default();
    let phase_count = phases.len();
    let planned_row_count = planned_rows.len();
    let completed_phase_count = phases
        .iter()
        .filter(|phase| phase.get("status").and_then(Value::as_str) == Some("completed"))
        .count();
    let active_phase_index = phases
        .iter()
        .position(|phase| phase.get("status").and_then(Value::as_str) != Some("completed"));
    let current_item = active_phase_index
        .map(|index| format!("item-{}", index + 1))
        .unwrap_or_else(|| "none".to_string());
    let mut phase_rows = create_longway_phase_rows(run_directory, &phases);
    attach_planned_rows_to_phase_rows(&mut phase_rows, &planned_rows, active_phase_index);

    Ok(json!({
        "file": checklist_file.to_string_lossy(),
        "lifecycle_state": longway.get("lifecycle_state").cloned().unwrap_or(Value::Null),
        "active_phase_name": longway.get("active_phase_name").cloned().unwrap_or(Value::Null),
        "active_phase_status": longway.get("active_phase_status").cloned().unwrap_or(Value::Null),
        "planning_context": longway.get("planning_context").cloned().unwrap_or(Value::Null),
        "way_clarification_request": longway.get("way_clarification_request").cloned().unwrap_or(Value::Null),
        "phase_count": phase_count,
        "planned_row_count": planned_row_count,
        "completed_phase_count": completed_phase_count,
        "current_item": current_item,
        "phase_rows": phase_rows,
        "planned_rows": planned_rows,
    }))
}

// LongWay remains the source for planned-row intent, while task cards own runtime lifecycle.
// This projection joins both so status/app-panel output does not show completed rows as pending.
fn sync_planned_row_projection_status(run_directory: &Path, mut row: Value) -> Value {
    let Some(task_card_id) = row
        .get("task_card_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    else {
        return row;
    };
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let Some(task_card) = read_optional_json_document(&task_card_file).ok().flatten() else {
        return row;
    };
    let latest_delegate_result =
        create_latest_delegate_result_payload(run_directory, Some(&task_card_id))
            .ok()
            .filter(|value| value.is_object());
    let projected_status =
        projected_planned_row_status_from_task_card(&task_card, latest_delegate_result.as_ref())
            .or_else(|| {
                row.get("status")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
    if let (Some(status), Some(object)) = (projected_status, row.as_object_mut()) {
        object.insert("status".to_string(), Value::String(status));
        object.insert(
            "status_source".to_string(),
            Value::String("task_card_lifecycle_projection".to_string()),
        );
        if task_card
            .get("subagent_fallback")
            .is_some_and(|value| !value.is_null())
            && latest_delegate_result
                .as_ref()
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                == Some("completed")
        {
            object.insert(
                "recovery".to_string(),
                json!({
                    "status": "completed",
                    "mode": "codex_exec",
                    "reason": task_card
                        .pointer("/subagent_fallback/reason")
                        .cloned()
                        .unwrap_or(Value::Null),
                    "primary_status": task_card
                        .pointer("/subagent_lifecycle/status")
                        .or_else(|| task_card.pointer("/review_lifecycle/status"))
                        .cloned()
                        .unwrap_or(Value::Null),
                }),
            );
        }
    }
    row
}

fn projected_planned_row_status_from_task_card(
    task_card: &Value,
    latest_delegate_result: Option<&Value>,
) -> Option<String> {
    let candidates = [
        task_card.pointer("/worker_result_envelope/status"),
        task_card.pointer("/subagent_fan_in/status"),
        task_card.pointer("/review_fan_in/status"),
        latest_delegate_result.and_then(|value| value.get("status")),
        task_card.pointer("/subagent_lifecycle/status"),
        task_card.pointer("/review_lifecycle/status"),
        task_card.get("verification_state"),
        task_card.get("status"),
    ];
    candidates
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find_map(|status| match status {
            "completed" | "passed" | "merged" => Some("completed".to_string()),
            "failed" | "blocked" | "stalled" | "cancelled" | "reclaimed" => {
                Some(status.to_string())
            }
            "spawned" | "acknowledged" | "running" | "active" | "in_progress" => {
                Some("running".to_string())
            }
            _ => None,
        })
}

fn first_bounded_planned_row_trace_string(
    trace: &serde_json::Map<String, Value>,
    keys: &[&str],
    max_chars: usize,
) -> Option<Value> {
    keys.iter().find_map(|key| {
        trace
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
    })
}

fn bounded_planned_row_trace_strings(
    trace: &serde_json::Map<String, Value>,
    keys: &[&str],
    max_items: usize,
    max_chars: usize,
) -> Option<Value> {
    keys.iter().find_map(|key| {
        let value = trace.get(*key)?;
        let strings = if let Some(items) = value.as_array() {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .take(max_items)
                .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
                .collect::<Vec<_>>()
        } else {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    vec![Value::String(summarize_text_for_visibility(
                        value, max_chars,
                    ))]
                })
                .unwrap_or_default()
        };
        (!strings.is_empty()).then_some(Value::Array(strings))
    })
}

fn bounded_planned_row_status_routing_trace(row: &Value) -> Option<Value> {
    let trace = row.get("routing_trace")?.as_object()?;
    let mut bounded = serde_json::Map::new();
    bounded.insert(
        "source".to_string(),
        Value::String("planned_row".to_string()),
    );
    if let Some(value) =
        first_bounded_planned_row_trace_string(trace, &["query_kind", "query", "kind"], 80)
    {
        bounded.insert("query".to_string(), value);
    }
    for (output_key, input_keys, max_chars) in [
        ("selected_category", &["selected_category"][..], 80),
        ("selected_skill_id", &["selected_skill_id"][..], 120),
        ("selected_skill_name", &["selected_skill_name"][..], 120),
        ("risk", &["risk"][..], 40),
        ("mutation_intent", &["mutation_intent"][..], 80),
        ("evidence_need", &["evidence_need"][..], 120),
        ("verification_need", &["verification_need"][..], 120),
        ("selected_role", &["selected_role"][..], 80),
        ("selected_agent_id", &["selected_agent_id"][..], 80),
    ] {
        if let Some(value) = first_bounded_planned_row_trace_string(trace, input_keys, max_chars) {
            bounded.insert(output_key.to_string(), value);
        }
    }
    if let Some(value) = bounded_planned_row_trace_strings(trace, &["paths", "path"], 8, 160) {
        bounded.insert("paths".to_string(), value);
    }
    if let Some(value) =
        bounded_planned_row_trace_strings(trace, &["terms", "term", "search", "text"], 8, 120)
    {
        bounded.insert("terms".to_string(), value);
    }
    if let Some(value) =
        first_bounded_planned_row_trace_string(trace, &["reason", "rationale"], 240)
    {
        bounded.insert("reason".to_string(), value);
    }
    if let Some(value) = first_bounded_planned_row_trace_string(trace, &["summary"], 240) {
        bounded.insert("summary".to_string(), value);
    }

    (bounded.len() > 1).then_some(Value::Object(bounded))
}

fn sanitize_longway_planned_row(row: &Value) -> Value {
    let Some(object) = row.as_object() else {
        return row.clone();
    };
    let mut sanitized = object.clone();

    if let Some(summary) = object
        .get("routing_summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sanitized.insert(
            "routing_summary".to_string(),
            Value::String(summarize_text_for_visibility(summary, 240)),
        );
    }

    let row_value = Value::Object(object.clone());
    if let Some(trace) = bounded_planned_row_status_routing_trace(&row_value) {
        sanitized.insert("routing_trace".to_string(), trace);
    } else {
        sanitized.remove("routing_trace");
    }
    sanitized.insert(
        "planning_detail".to_string(),
        bounded_planned_row_planning_detail(&row_value),
    );

    Value::Object(sanitized)
}

fn bounded_scheduler_planned_row_routing_trace(row: &Value) -> Option<Value> {
    let trace = row.get("routing_trace")?.as_object()?;
    let mut bounded = serde_json::Map::new();
    for (output_key, input_keys, max_chars) in [
        (
            "selected_category",
            &["selected_category", "category"][..],
            80,
        ),
        (
            "selected_skill_id",
            &["selected_skill_id", "skill_id", "skill"][..],
            120,
        ),
        (
            "selected_skill_name",
            &["selected_skill_name", "skill_name"][..],
            120,
        ),
        ("risk", &["risk"][..], 40),
        ("evidence_need", &["evidence_need"][..], 120),
        ("verification_need", &["verification_need"][..], 120),
        ("selected_role", &["selected_role", "role"][..], 80),
        (
            "selected_agent_id",
            &["selected_agent_id", "agent_id", "agent"][..],
            80,
        ),
        ("reason", &["reason", "rationale"][..], 240),
        ("summary", &["summary"][..], 240),
    ] {
        if let Some(value) = first_bounded_planned_row_trace_string(trace, input_keys, max_chars) {
            bounded.insert(output_key.to_string(), value);
        }
    }

    (!bounded.is_empty()).then_some(Value::Object(bounded))
}

pub(crate) fn sanitize_scheduler_selected_planned_row(row: &Value) -> Value {
    let Some(object) = row.as_object() else {
        return row.clone();
    };
    let mut sanitized = object.clone();

    if let Some(summary) = object
        .get("routing_summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sanitized.insert(
            "routing_summary".to_string(),
            Value::String(summarize_text_for_visibility(summary, 240)),
        );
    }

    let row_value = Value::Object(object.clone());
    if let Some(trace) = bounded_scheduler_planned_row_routing_trace(&row_value) {
        sanitized.insert("routing_trace".to_string(), trace);
    } else {
        sanitized.remove("routing_trace");
    }

    Value::Object(sanitized)
}

fn sanitize_scheduler_transition_payload(transition: Value) -> Value {
    let Some(mut object) = transition.as_object().cloned() else {
        return transition;
    };
    if let Some(row) = object.get("selected_planned_row").cloned() {
        object.insert(
            "selected_planned_row".to_string(),
            sanitize_scheduler_selected_planned_row(&row),
        );
    }
    Value::Object(object)
}

fn bounded_planned_row_planning_detail(row: &Value) -> Value {
    let mut detail = serde_json::Map::new();
    detail.insert(
        "lifecycle".to_string(),
        row.get("status")
            .cloned()
            .unwrap_or_else(|| Value::String("planned".to_string())),
    );
    for (source_key, detail_key, max_chars) in [
        ("scope", "scope", 160),
        ("acceptance", "acceptance", 160),
        ("routing_summary", "routing", 140),
    ] {
        if let Some(value) = row
            .get(source_key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            detail.insert(
                detail_key.to_string(),
                Value::String(summarize_text_for_visibility(value, max_chars)),
            );
        }
    }
    if let Some(count) = row
        .get("evidence_links")
        .and_then(Value::as_array)
        .map(Vec::len)
        .filter(|count| *count > 0)
    {
        detail.insert("evidence_count".to_string(), Value::from(count));
    }
    Value::Object(detail)
}

fn create_longway_phase_rows(run_directory: &Path, phases: &[Value]) -> Vec<Value> {
    phases
        .iter()
        .enumerate()
        .filter_map(|(index, phase)| {
            let owner_agent = first_non_empty_string(
                phase,
                &["owner_agent", "assigned_agent_id", "agent_id", "owner"],
            );
            let task_owner_agent =
                phase
                    .get("task_items")
                    .and_then(Value::as_array)
                    .and_then(|items| {
                        items.iter().find_map(|item| {
                            first_non_empty_string(
                                item,
                                &["owner_agent", "assigned_agent_id", "agent_id", "owner"],
                            )
                        })
                    });
            let owner_agent = owner_agent.or(task_owner_agent);
            let label = first_non_empty_string(phase, &["phase_name", "phase_id"])
                .unwrap_or_else(|| format!("item-{}", index + 1));
            let title = first_non_empty_string(phase, &["title", "summary", "intent"]);
            let task_card_id = first_non_empty_string(phase, &["task_card_id", "task_id"]);
            let lifecycle_sync = task_card_id
                .as_deref()
                .map(|value| create_phase_row_lifecycle_sync(run_directory, value))
                .unwrap_or_else(|| {
                    json!({
                        "available": false,
                        "reason": "phase row has no task_card_id"
                    })
                });
            let mut task_unit_labels = phase_row_task_item_unit_labels(phase);
            if task_unit_labels.is_empty() {
                task_unit_labels = phase_row_lifecycle_unit_labels(&lifecycle_sync);
            }
            if owner_agent.is_none() && title.is_none() {
                return None;
            }

            Some(json!({
                "item": format!("item-{}", index + 1),
                "label": label,
                "title": title.map(Value::String).unwrap_or(Value::Null),
                "status": phase.get("status").cloned().unwrap_or(Value::Null),
                "owner_agent": owner_agent.map(Value::String).unwrap_or(Value::Null),
                "task_card_id": task_card_id
                    .as_ref()
                    .map(|value| Value::String(value.clone()))
                    .unwrap_or(Value::Null),
                "task_unit_labels": task_unit_labels,
                "lifecycle_sync": lifecycle_sync,
            }))
        })
        .collect()
}

fn attach_planned_rows_to_phase_rows(
    phase_rows: &mut [Value],
    planned_rows: &[Value],
    active_phase_index: Option<usize>,
) {
    if phase_rows.is_empty() || planned_rows.is_empty() {
        return;
    }

    let fallback_index = active_phase_index
        .filter(|index| *index < phase_rows.len())
        .unwrap_or_else(|| phase_rows.len().saturating_sub(1));
    let mut grouped_rows = vec![Vec::<Value>::new(); phase_rows.len()];

    for row in planned_rows {
        let target_index = row
            .get("task_card_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|task_card_id| !task_card_id.is_empty())
            .and_then(|task_card_id| {
                phase_rows.iter().position(|phase_row| {
                    phase_row
                        .get("task_card_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        == Some(task_card_id)
                })
            })
            .unwrap_or(fallback_index);
        grouped_rows[target_index].push(row.clone());
    }

    for (index, rows) in grouped_rows.into_iter().enumerate() {
        if rows.is_empty() {
            continue;
        }
        if let Some(object) = phase_rows[index].as_object_mut() {
            object.insert("planned_rows".to_string(), Value::Array(rows));
        }
    }
}

fn create_phase_row_lifecycle_sync(run_directory: &Path, task_card_id: &str) -> Value {
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let Some(task_card) = read_optional_json_document(&task_card_file).ok().flatten() else {
        return json!({
            "available": false,
            "task_card_id": task_card_id,
            "reason": "task card not found"
        });
    };

    // The task card remains the lifecycle source of truth; phase rows only carry this compact
    // projection so checklist/status views can show row-local progress without duplicating state.
    let subagent_status = task_card
        .pointer("/subagent_lifecycle/status")
        .and_then(Value::as_str);
    let review_status = task_card
        .pointer("/review_lifecycle/status")
        .and_then(Value::as_str);
    let fan_in_status = task_card
        .pointer("/subagent_fan_in/status")
        .or_else(|| task_card.pointer("/review_fan_in/status"))
        .and_then(Value::as_str);
    let lane_statuses = phase_row_lane_statuses(&task_card);
    let active_lane_count = lane_statuses
        .values()
        .filter(|status| matches!(status.as_str(), "spawned" | "acknowledged" | "running"))
        .count();
    let terminal_lane_count = lane_statuses
        .values()
        .filter(|status| {
            matches!(
                status.as_str(),
                "completed" | "failed" | "stalled" | "reclaimed" | "merged"
            )
        })
        .count();
    let compact_status = review_status
        .or(subagent_status)
        .or(fan_in_status)
        .or_else(|| {
            if active_lane_count > 0 {
                Some("running")
            } else if terminal_lane_count > 0 {
                Some("terminal")
            } else {
                None
            }
        })
        .unwrap_or("not_started");
    let compact_summary = phase_row_lifecycle_summary(
        compact_status,
        active_lane_count,
        terminal_lane_count,
        lane_statuses.len(),
    );
    let task_unit_labels = task_card_lifecycle_unit_labels(&task_card);
    let lifecycle_details = task_card_lifecycle_details(&task_card);

    json!({
        "available": true,
        "source": task_card_file.to_string_lossy(),
        "task_card_id": task_card_id,
        "status": compact_status,
        "summary": compact_summary,
        "subagent_status": subagent_status.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
        "review_status": review_status.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
        "fan_in_status": fan_in_status.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
        "active_lane_count": active_lane_count,
        "terminal_lane_count": terminal_lane_count,
        "lane_statuses": lane_statuses,
        "details": lifecycle_details,
        "task_unit_labels": task_unit_labels,
    })
}

fn phase_row_lane_statuses(task_card: &Value) -> BTreeMap<String, String> {
    task_card
        .pointer("/parallel_fanout/lanes")
        .and_then(Value::as_array)
        .map(|lanes| {
            lanes
                .iter()
                .filter_map(|lane| {
                    let lane_id = lane.get("lane_id").and_then(Value::as_str)?;
                    let status = lane
                        .pointer("/lifecycle/status")
                        .and_then(Value::as_str)
                        .or_else(|| lane.pointer("/fan_in/status").and_then(Value::as_str))?;
                    Some((lane_id.to_string(), status.to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn phase_row_task_item_unit_labels(phase: &Value) -> Vec<String> {
    phase
        .get("task_items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let unit =
                        first_non_empty_string(item, &["task_item_id", "task_card_id", "title"])?;
                    let owner = first_non_empty_string(
                        item,
                        &["owner_agent", "assigned_agent_id", "agent_id", "owner"],
                    )?;
                    Some(compact_task_unit_label(&unit, &owner))
                })
                .fold(Vec::new(), |mut labels, label| {
                    push_unique_label(&mut labels, label);
                    labels
                })
        })
        .unwrap_or_default()
}

fn task_card_lifecycle_details(task_card: &Value) -> Vec<Value> {
    let mut details = Vec::new();
    let lifecycle_owner = task_card_lifecycle_owner(task_card);

    if let Some(lanes) = task_card
        .pointer("/parallel_fanout/lanes")
        .and_then(Value::as_array)
    {
        for lane in lanes {
            let Some(lane_id) = first_non_empty_string(lane, &["lane_id"]) else {
                continue;
            };
            let lifecycle = lane.get("lifecycle").filter(|value| value.is_object());
            let fan_in = lane.get("fan_in").filter(|value| value.is_object());
            if lifecycle.is_none() && fan_in.is_none() {
                continue;
            }
            let status = lifecycle
                .and_then(|value| value.get("status"))
                .or_else(|| fan_in.and_then(|value| value.get("status")))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty());
            let Some(status) = status else {
                continue;
            };
            let child_agent_id = lifecycle
                .and_then(|value| value.get("child_agent_id"))
                .or_else(|| fan_in.and_then(|value| value.get("child_agent_id")))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .or(lifecycle_owner.as_deref());
            let summary = fan_in
                .and_then(|value| value.get("summary"))
                .or_else(|| lifecycle.and_then(|value| value.get("summary")))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let evidence_count = fan_in
                .and_then(|value| value.get("evidence_paths"))
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            let confidence = fan_in
                .and_then(|value| value.get("confidence"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());

            details.push(json!({
                "kind": "lane",
                "label": lane_id,
                "status": status,
                "child_agent_id": child_agent_id.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
                "summary": summary.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
                "evidence_count": evidence_count,
                "confidence": confidence.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
            }));
        }
    }

    if !details.is_empty() {
        return details;
    }

    for (kind, path) in [
        ("subagent", "/subagent_lifecycle"),
        ("review", "/review_lifecycle"),
    ] {
        let Some(lifecycle) = task_card.pointer(path).filter(|value| value.is_object()) else {
            continue;
        };
        let Some(status) = lifecycle
            .get("status")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let child_agent_id = lifecycle
            .get("child_agent_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let label = child_agent_id.unwrap_or(kind);
        let summary = lifecycle
            .get("summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());

        details.push(json!({
            "kind": kind,
            "label": label,
            "status": status,
            "child_agent_id": child_agent_id.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
            "summary": summary.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
            "evidence_count": 0,
            "confidence": Value::Null,
        }));
    }

    details
}

fn phase_row_lifecycle_unit_labels(lifecycle_sync: &Value) -> Vec<String> {
    lifecycle_sync
        .get("task_unit_labels")
        .and_then(Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn task_card_lifecycle_unit_labels(task_card: &Value) -> Vec<String> {
    let mut labels = Vec::new();
    let lifecycle_owner = task_card_lifecycle_owner(task_card);

    if let Some(lanes) = task_card
        .pointer("/parallel_fanout/lanes")
        .and_then(Value::as_array)
    {
        for lane in lanes {
            let Some(lane_id) = first_non_empty_string(lane, &["lane_id"]) else {
                continue;
            };
            let owner = lane
                .pointer("/lifecycle/child_agent_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    lane.pointer("/fan_in/child_agent_id")
                        .and_then(Value::as_str)
                })
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .or(lifecycle_owner.as_deref());
            if let Some(owner) = owner {
                push_unique_label(&mut labels, compact_task_unit_label(&lane_id, owner));
            }
        }
    }

    if !labels.is_empty() {
        return labels;
    }

    for path in ["/subagent_lifecycle", "/review_lifecycle"] {
        let Some(lifecycle) = task_card.pointer(path) else {
            continue;
        };
        let Some(owner) = lifecycle
            .get("child_agent_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let unit = first_non_empty_string(task_card, &["task_card_id", "title"])
            .unwrap_or_else(|| "task".to_string());
        push_unique_label(&mut labels, compact_task_unit_label(&unit, owner));
    }

    labels
}

fn task_card_lifecycle_owner(task_card: &Value) -> Option<String> {
    [
        "/subagent_lifecycle/child_agent_id",
        "/review_lifecycle/child_agent_id",
    ]
    .iter()
    .find_map(|path| {
        task_card
            .pointer(path)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn compact_task_unit_label_part(value: &str, max_chars: usize) -> String {
    summarize_text_for_visibility(value.trim(), max_chars)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

fn compact_task_unit_label(unit: &str, owner: &str) -> String {
    format!(
        "{}:{}",
        compact_task_unit_label_part(unit, 48),
        compact_task_unit_label_part(owner, 48)
    )
}

fn push_unique_label(labels: &mut Vec<String>, label: String) {
    if !labels.iter().any(|existing| existing == &label) {
        labels.push(label);
    }
}

fn phase_row_lifecycle_summary(
    status: &str,
    active_lane_count: usize,
    terminal_lane_count: usize,
    lane_count: usize,
) -> String {
    if lane_count > 0 {
        format!(
            "{status}; lanes active={active_lane_count} terminal={terminal_lane_count}/{lane_count}"
        )
    } else {
        format!("{status}; no parallel lane lifecycle recorded")
    }
}

fn first_non_empty_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn create_run_state_payload(run_directory: &Path) -> io::Result<Value> {
    let run_state_file = run_directory.join("run-state.json");
    let Some(run_state) = read_optional_json_document(&run_state_file)? else {
        return Ok(Value::Null);
    };

    Ok(json!({
        "file": run_state_file.to_string_lossy(),
        "event_log_file": run_directory.join("events.jsonl").to_string_lossy(),
        "updated_at": run_state.get("updated_at").cloned().unwrap_or(Value::Null),
        "event_count": run_state.get("event_count").cloned().unwrap_or(Value::Null),
        "last_event_id": run_state.get("last_event_id").cloned().unwrap_or(Value::Null),
        "sequence": run_state.get("sequence").cloned().unwrap_or(Value::Null),
        "approval_state": run_state.get("approval_state").cloned().unwrap_or(Value::Null),
        "current_phase_name": run_state.get("current_phase_name").cloned().unwrap_or(Value::Null),
        "next_action": run_state.get("next_action").cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn create_post_fan_in_captain_decision_payload(
    run_record: &Value,
    longway: &Value,
    current_task_card: &Value,
    host_subagent_state: &Value,
    next_step: &str,
    can_advance: bool,
    fan_in_ready: bool,
    worker_active: u64,
    host_subagent_active: u64,
) -> Value {
    let task_has_delegation_plan = current_task_card
        .get("delegation_plan")
        .and_then(Value::as_object)
        .is_some();
    let review_policy = current_task_card.get("review_policy");
    let review_needs_captain_decision =
        review_policy.is_some_and(review_policy_requires_captain_decision);
    let review_required = review_policy.is_some_and(review_policy_blocks_direct_captain_action);
    let direct_captain_drift_requires_acceptance =
        subagent_policy_drift_requires_acceptance(current_task_card);
    let run_status = run_record
        .get("status")
        .and_then(Value::as_str)
        .or_else(|| run_record.as_str());
    let terminal = matches!(
        run_status,
        Some("completed" | "failed" | "cancelled" | "blocked")
    ) || matches!(
        next_step,
        "halt_completed" | "halt_failed" | "halt_cancelled"
    );
    let active_specialist = worker_active > 0 || host_subagent_active > 0;
    let approval_state = longway
        .get("approval_state")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task_card
                .get("approval_state")
                .and_then(Value::as_str)
        })
        .unwrap_or_default();
    let review_outcome = current_task_card
        .pointer("/review_fan_in/outcome")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task_card
                .pointer("/review_fan_in/status")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            current_task_card
                .pointer("/review_lifecycle/status")
                .and_then(Value::as_str)
        })
        .unwrap_or_default();
    let review_next_action = current_task_card
        .pointer("/review_fan_in/captain_next_decision")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task_card
                .pointer("/review_fan_in/next_action")
                .and_then(Value::as_str)
        })
        .unwrap_or_default();
    let recovery_action = host_subagent_state
        .pointer("/recovery_recommendation/recommended_action")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let reclaim_action = host_subagent_state
        .pointer("/reclaim_replan_recommendation/recommended_action")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let mutation_evidence_gate = run_record
        .get("mutation_evidence_gate")
        .or_else(|| current_task_card.get("mutation_evidence_gate"));
    let mutation_evidence_blocked = mutation_evidence_gate
        .and_then(|value| value.get("blocked"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let (
        precedence,
        allowed_action,
        required_action,
        denied_action_reason,
        state,
        active_gate,
        required_artifact,
        scheduler_action_kind,
        scheduler_action_reason,
    ) = if terminal {
        (
            "terminal",
            "finish_allowed",
            "report_terminal_status",
            "Run is already terminal; no additional captain mutation is allowed without a new run.",
            "report_ready",
            "report",
            "final_status_summary",
            "terminal",
            "scheduler observed a terminal run state",
        )
    } else if next_step == "await_operator" {
        (
            "operator",
            "blocked",
            "await_operator",
            "Run is blocked on an operator decision.",
            "clarification_pending",
            "planning_clarification",
            "operator_clarification_answer",
            "await_operator",
            "scheduler is blocked until the operator answers the pending clarification",
        )
    } else if next_step == "await_longway_approval" || approval_state == "pending_longway_approval"
    {
        (
            "longway_approval",
            "blocked",
            "await_longway_approval",
            "PLAN_SEQUENCE is waiting for explicit LongWay approval before executable task dispatch.",
            "longway_pending",
            "planning_approval",
            "operator_longway_approval",
            "await_longway_approval",
            "scheduler is blocked until the pending LongWay is approved",
        )
    } else if review_needs_captain_decision {
        (
            "review_pass_cap",
            "captain_decision_required",
            "ccc_orchestrate",
            "Review pass cap reached; captain must decide the next action through CCC orchestration.",
            "review_decision_pending",
            "review_gate",
            "captain_review_decision",
            "review_captain_decision",
            "review pass cap requires an explicit captain decision before merge or close",
        )
    } else if matches!(
        review_outcome,
        "needs_work" | "failed" | "unsatisfactory" | "blocked"
    ) || matches!(
        review_next_action,
        "captain_repair" | "captain_replan" | "captain_request_operator_input"
    ) {
        (
            "review_needs_work",
            "review_repair_required",
            "ccc_orchestrate",
            "Review fan-in requires captain repair, replan, reassignment, or operator input before merge.",
            "review_decision_pending",
            "review_gate",
            "review_fan_in_repair_decision",
            "review_needs_work",
            "review fan-in requires a repair or replan decision before recovery fallback",
        )
    } else if direct_captain_drift_requires_acceptance {
        (
            "review_gate",
            "captain_drift_acceptance_required",
            "spawn_or_merge_review",
            "Direct captain output on a specialist-owned route must be reviewed or explicitly accepted before merge or close.",
            "review_pending",
            "review_gate",
            "review_or_explicit_acceptance",
            "review_required",
            "direct captain drift requires review or explicit acceptance before merge",
        )
    } else if review_required {
        (
            "review_gate",
            "review_required",
            "spawn_or_merge_review",
            "Review policy is active; captain must satisfy the review gate before direct finish or mutation.",
            "review_pending",
            "review_gate",
            "review_fan_in_or_policy_pass",
            "review_required",
            "review policy is active before direct finish or mutation",
        )
    } else if recovery_action == "reclaim" {
        (
            "recovery",
            "reclaim_subagent",
            "ccc_subagent_update",
            "A host subagent appears stalled; wait for fan-in, close completed host agents when available, or record reclaim/replan before degraded fallback.",
            "recovery_pending",
            "recovery",
            "reclaim_replan_or_fallback_decision",
            "reclaim_subagent",
            "host subagent reclaim/replan is required before degraded fallback",
        )
    } else if matches!(recovery_action, "retry" | "reassign") {
        (
            "recovery",
            "recover_subagent",
            "ccc_orchestrate",
            "A host subagent ended without clean fan-in; close terminal host agents when available, then retry or reassign before degraded fallback.",
            "recovery_pending",
            "recovery",
            "retry_or_reassign_decision",
            "recover_subagent",
            "terminal host subagent state requires retry or reassign before degraded fallback",
        )
    } else if matches!(
        reclaim_action,
        "reclaim_or_replan" | "await_fan_in_or_replan"
    ) {
        (
            "recovery",
            "reclaim_subagent",
            "ccc_subagent_update",
            "A host subagent reclaim/replan recommendation is visible; captain must merge, wait, or replan before degraded fallback.",
            "recovery_pending",
            "recovery",
            "reclaim_replan_or_fallback_decision",
            "reclaim_subagent",
            "host subagent reclaim/replan recommendation is visible",
        )
    } else if mutation_evidence_blocked {
        (
            "mutation_evidence",
            "mutation_evidence_blocked",
            "record_fan_in_or_approve_longway",
            "Mutation dispatch is blocked until persisted evidence or approved LongWay scope exists.",
            "blocked",
            "mutation_evidence",
            "persisted_approved_longway_or_fan_in_evidence",
            "blocked",
            "mutation dispatch is blocked by the evidence-before-mutation gate",
        )
    } else if fan_in_ready || can_advance || next_step == "advance" {
        (
            "advance",
            "captain_advance",
            "ccc_orchestrate",
            "Captain should advance through CCC orchestration before direct finish or mutation.",
            "decision_pending",
            "captain_decision",
            "captain_decision_envelope",
            "select_next",
            "scheduler can advance captain selection to the next bounded action",
        )
    } else if next_step == "await_fan_in" || active_specialist {
        (
            "await_fan_in",
            "await_fan_in",
            "ccc_subagent_update",
            "Active specialist or pending fan-in must complete; captain should wait or close completed host agents before direct finish or mutation.",
            "fan_in_pending",
            "fan_in",
            "specialist_fan_in",
            "await_fan_in",
            "scheduler is waiting for compact fan-in before selecting the next task",
        )
    } else if next_step == "execute_task" || task_has_delegation_plan {
        let role_gate = role_gate_family(current_task_card);
        let (role_state, role_required_artifact) = match role_gate {
            "planning" => ("planning_pending", "planning_or_clarification"),
            "mutation" => ("mutation_pending", "mutation_scope_and_evidence"),
            "verification" => ("verification_pending", "verification_scope"),
            _ => ("evidence_pending", "evidence_scope"),
        };
        (
            "dispatch",
            "spawn_subagent",
            "spawn_or_record_specialist",
            "Current task requires specialist execution before direct captain finish or mutation.",
            role_state,
            role_gate,
            role_required_artifact,
            "dispatch_selected_task",
            "scheduler selected the current approved task for specialist dispatch",
        )
    } else {
        (
            "advance",
            "captain_advance",
            "ccc_orchestrate",
            "Captain should use the persisted CCC next step before direct finish or mutation.",
            "decision_pending",
            "captain_decision",
            "captain_decision_envelope",
            "checkpoint",
            "scheduler recorded a non-dispatch checkpoint",
        )
    };

    json!({
        "schema": "ccc.post_fan_in_captain_decision.v1",
        "source": "persisted_ccc_truth",
        "precedence": precedence,
        "state": state,
        "active_gate": active_gate,
        "required_artifact": required_artifact,
        "allowed_action": allowed_action,
        "required_action": required_action,
        "denied_action_reason": denied_action_reason,
        "scheduler_action": {
            "kind": scheduler_action_kind,
            "reason": scheduler_action_reason,
            "next_step": next_step,
            "can_advance": can_advance,
        },
        "inputs": {
            "run_status": run_status,
            "next_step": next_step,
            "can_advance": can_advance,
            "fan_in_ready": fan_in_ready,
            "worker_active": worker_active,
            "host_subagent_active": host_subagent_active,
            "review_outcome": review_outcome,
            "review_next_action": review_next_action,
            "recovery_action": recovery_action,
            "reclaim_replan_action": reclaim_action,
            "approval_state": approval_state,
        },
        "task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "assigned_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
    })
}

fn create_captain_action_contract(
    current_task_card: &Value,
    post_fan_in_captain_decision: &Value,
) -> Value {
    let allowed_action = post_fan_in_captain_decision
        .get("allowed_action")
        .and_then(Value::as_str)
        .unwrap_or("captain_advance");
    let required_action = post_fan_in_captain_decision
        .get("required_action")
        .and_then(Value::as_str)
        .unwrap_or("ccc_orchestrate");
    let denied_action_reason = post_fan_in_captain_decision
        .get("denied_action_reason")
        .and_then(Value::as_str)
        .unwrap_or(
            "Captain should use the persisted CCC next step before direct finish or mutation.",
        );
    let completion_required = current_task_card
        .get("completion_discipline")
        .and_then(|discipline| discipline.get("documented_completion_requested"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_finish_allowed = allowed_action == "finish_allowed" && !completion_required;
    let direct_mutation_allowed = false;
    let subagent_capacity_policy = create_subagent_capacity_policy();
    let direct_file_mutation_policy =
        create_direct_file_mutation_policy(required_action, denied_action_reason);

    json!({
        "source": "ccc_status",
        "preflight_guard": "ccc_recommend_entry",
        "preferred_operator_transport": "ccc_cli_quiet_subcommand",
        "preferred_operator_transport_reason": "Keeps repeated CCC lifecycle mutations visible as compact command runs instead of verbose MCP tool-call payloads.",
        "mcp_tool_call_policy": "reserve_for_app_or_structured_inspection_or_cli_unavailable",
        "allowed_action": allowed_action,
        "required_action": required_action,
        "direct_finish_allowed": direct_finish_allowed,
        "direct_mutation_allowed": direct_mutation_allowed,
        "direct_file_mutation_policy": direct_file_mutation_policy,
        "subagent_capacity_policy": subagent_capacity_policy,
        "denied_action_reason": denied_action_reason,
        "completion_required": completion_required,
        "post_fan_in_captain_decision": post_fan_in_captain_decision,
    })
}

fn create_subagent_capacity_policy() -> Value {
    json!({
        "on_capacity_exhausted": "wait_or_cleanup_before_direct_work",
        "direct_specialist_takeover_allowed": false,
        "required_order": [
            "wait_for_active_fan_in",
            "close_completed_host_agent_threads",
            "record_reclaim_reassign_or_terminal_fallback",
            "retry_specialist_dispatch"
        ],
        "fallback_reason": "host_subagent_thread_limit",
        "summary": "When host subagent capacity is exhausted, captain must wait, close terminal host agents, or record reclaim/reassign/terminal fallback before retrying specialist dispatch; direct specialist-owned mutation remains blocked."
    })
}

fn create_direct_file_mutation_policy(required_action: &str, denied_action_reason: &str) -> Value {
    json!({
        "allowed": false,
        "applies_to": ["apply_patch", "direct_shell_file_mutation", "file_edits", "mutation_commands"],
        "required_route": "specialist_fan_in_then_captain_review_merge",
        "required_action": required_action,
        "requires_recorded_exception": "explicit_terminal_fallback_or_operator_override",
        "merge_gate": "specialist_fan_in_or_explicit_operator_override",
        "operator_override_required": true,
        "reason": denied_action_reason,
    })
}

fn subagent_policy_drift_requires_acceptance(current_task_card: &Value) -> bool {
    current_task_card
        .pointer("/subagent_policy_drift/direct_captain_bypass")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || current_task_card
            .pointer("/subagent_policy_drift/acceptance_gate/state")
            .and_then(Value::as_str)
            == Some("required")
}

fn review_policy_requires_captain_decision(review_policy: &Value) -> bool {
    review_policy.get("state").and_then(Value::as_str) == Some("captain_decision_required")
}

fn review_policy_blocks_direct_captain_action(review_policy: &Value) -> bool {
    let state = review_policy
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(
        state,
        "skipped" | "suppressed" | "passed" | "completed" | "resolved"
    ) {
        return false;
    }

    let decision = review_policy
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or_default();
    matches!(
        decision,
        "require" | "required" | "recommend_single" | "recommended"
    ) || matches!(
        state,
        "required"
            | "recommended"
            | "review_required"
            | "spawn_review"
            | "await_review"
            | "captain_decision_required"
            | "running"
    )
}

fn count_planned_rows_with_status(longway: &Value, status: &str) -> usize {
    longway
        .get("planned_rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("status").and_then(Value::as_str) == Some(status))
                .count()
        })
        .unwrap_or(0)
}

fn count_materialized_or_completed_planned_rows(longway: &Value) -> usize {
    longway
        .get("planned_rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("task_card_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
                        || matches!(
                            row.get("status").and_then(Value::as_str),
                            Some("materialized" | "running" | "completed" | "passed" | "merged")
                        )
                })
                .count()
        })
        .unwrap_or(0)
}

fn next_pending_planned_row(longway: &Value) -> Value {
    longway
        .get("planned_rows")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("status").and_then(Value::as_str) == Some("planned")
                    && row
                        .get("task_card_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none()
            })
        })
        .cloned()
        .unwrap_or(Value::Null)
}

fn scheduler_decision_source(
    current_task_card: &Value,
    next_step: &str,
    fan_in_ready: bool,
) -> &'static str {
    if current_task_card
        .get("planned_longway_row")
        .is_some_and(|value| value.is_object())
    {
        "planned_row_materialization"
    } else if current_task_card
        .get("parallel_fanout")
        .is_some_and(|value| value.is_object())
    {
        "bounded_parallel_fanout"
    } else if fan_in_ready || next_step == "await_fan_in" {
        "fan_in_barrier"
    } else if next_step == "await_longway_approval" {
        "pending_longway_approval"
    } else {
        "approved_longway_task_cards"
    }
}

fn scheduler_state(
    status: &Value,
    next_step: &str,
    can_advance: bool,
    fan_in_ready: bool,
    current_task_card: &Value,
) -> &'static str {
    if matches!(
        status.as_str(),
        Some("completed" | "failed" | "cancelled" | "blocked")
    ) {
        "terminal"
    } else if next_step == "await_longway_approval" {
        "blocked"
    } else if fan_in_ready || next_step == "await_fan_in" {
        "await_fan_in"
    } else if current_task_card
        .get("parallel_fanout")
        .is_some_and(|value| value.is_object())
    {
        "parallel"
    } else if next_step == "execute_task" {
        "selected"
    } else if can_advance || next_step == "advance" {
        "select_next"
    } else {
        "checkpoint"
    }
}

fn scheduler_status_action(
    state: &str,
    decision_source: &str,
    next_step: &str,
    can_advance: bool,
    fan_in_ready: bool,
    current_task_card: &Value,
    post_fan_in_captain_decision: &Value,
) -> Value {
    let canonical_action = post_fan_in_captain_decision
        .get("scheduler_action")
        .filter(|value| value.is_object());
    let canonical_precedence = post_fan_in_captain_decision
        .get("precedence")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let preserve_scheduler_selection = matches!(
        decision_source,
        "bounded_parallel_fanout" | "planned_row_materialization"
    );
    let (kind, reason) = if !preserve_scheduler_selection
        && matches!(
            canonical_precedence,
            "terminal"
                | "operator"
                | "longway_approval"
                | "review_pass_cap"
                | "review_needs_work"
                | "review_gate"
                | "recovery"
                | "mutation_evidence"
        ) {
        if let Some(action) = canonical_action {
            (
                action
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("checkpoint"),
                action
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("scheduler followed the canonical post-fan-in captain decision"),
            )
        } else {
            (
                "checkpoint",
                "scheduler followed the canonical post-fan-in captain decision",
            )
        }
    } else if decision_source == "bounded_parallel_fanout" {
        if fan_in_ready {
            (
                "parallel_fan_in_ready",
                "all required parallel lanes are terminal and compact fan-in can be consumed",
            )
        } else if next_step == "await_fan_in" {
            (
                "await_parallel_fan_in",
                "bounded parallel fan-out is waiting for all required lanes to return fan-in",
            )
        } else {
            (
                "bounded_parallel_fanout",
                "scheduler selected bounded parallel fan-out for the active task",
            )
        }
    } else if decision_source == "planned_row_materialization" {
        (
            "materialized_planned_row",
            "scheduler selected the active task from a materialized LongWay planned row",
        )
    } else if decision_source == "fan_in_barrier" {
        (
            "await_fan_in",
            "scheduler is waiting for compact fan-in before selecting the next task",
        )
    } else if decision_source == "pending_longway_approval" {
        (
            "await_longway_approval",
            "scheduler is blocked until the pending LongWay is approved",
        )
    } else if state == "terminal" {
        ("terminal", "scheduler observed a terminal run state")
    } else if next_step == "execute_task" {
        (
            "dispatch_selected_task",
            "scheduler selected the current approved task for specialist dispatch",
        )
    } else if can_advance || next_step == "advance" {
        (
            "select_next",
            "scheduler can advance captain selection to the next bounded action",
        )
    } else {
        ("checkpoint", "scheduler recorded a non-dispatch checkpoint")
    };

    json!({
        "kind": kind,
        "reason": reason,
        "next_step": next_step,
        "can_advance": can_advance,
        "selected_task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "captain_decision_precedence": post_fan_in_captain_decision.get("precedence").cloned().unwrap_or(Value::Null),
    })
}

fn create_scheduler_status_payload(
    run_directory: &Path,
    run_record: &Value,
    longway: &Value,
    current_task_card: &Value,
    host_subagent_state: &Value,
    captain_action_contract: &Value,
    post_fan_in_captain_decision: &Value,
    next_step: &str,
    can_advance: bool,
    fan_in_ready: bool,
) -> Value {
    let planned_row_count = longway
        .get("planned_rows")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let planned_pending_count = count_planned_rows_with_status(longway, "planned");
    let planned_materialized_count = count_materialized_or_completed_planned_rows(longway);
    let next_planned_row = next_pending_planned_row(longway);
    let parallel_required_lane_ids = current_task_card
        .pointer("/parallel_fanout/required_lane_ids")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let parallel_candidate_count = parallel_required_lane_ids
        .as_array()
        .map(Vec::len)
        .or_else(|| {
            current_task_card
                .pointer("/parallel_fanout/lanes")
                .and_then(Value::as_array)
                .map(Vec::len)
        })
        .unwrap_or(0);
    let state = scheduler_state(
        run_record.get("status").unwrap_or(&Value::Null),
        next_step,
        can_advance,
        fan_in_ready,
        current_task_card,
    );
    let decision_source = scheduler_decision_source(current_task_card, next_step, fan_in_ready);
    let action = scheduler_status_action(
        state,
        decision_source,
        next_step,
        can_advance,
        fan_in_ready,
        current_task_card,
        post_fan_in_captain_decision,
    );
    let blocked_reason = if state == "blocked" {
        captain_action_contract
            .get("denied_action_reason")
            .cloned()
            .unwrap_or_else(|| Value::String("Scheduler is blocked.".to_string()))
    } else {
        Value::Null
    };
    let latest_transition = sanitize_scheduler_transition_payload(
        read_latest_scheduler_transition(run_directory).unwrap_or_else(|_| Value::Null),
    );
    let selected_planned_row = current_task_card
        .get("planned_longway_row")
        .map(sanitize_scheduler_selected_planned_row)
        .unwrap_or(Value::Null);

    json!({
        "schema": "ccc.scheduler.v1",
        "state": state,
        "decision_source": decision_source,
        "reason": match decision_source {
            "planned_row_materialization" => "Scheduler selected the active task from a materialized LongWay planned row.",
            "bounded_parallel_fanout" => "Scheduler selected bounded parallel fan-out for the active task.",
            "fan_in_barrier" => "Scheduler is waiting for compact fan-in before selecting the next task.",
            "pending_longway_approval" => "Scheduler is blocked until the pending LongWay is approved.",
            _ => "Scheduler selected the current task from approved LongWay task cards.",
        },
        "action": action,
        "post_fan_in_captain_decision": post_fan_in_captain_decision,
        "next_step": next_step,
        "can_advance": can_advance,
        "selected_task_card_id": current_task_card.get("task_card_id").cloned().or_else(|| run_record.get("active_task_card_id").cloned()).unwrap_or(Value::Null),
        "selected_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "selected_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "selected_planned_row": selected_planned_row,
        "latest_transition": latest_transition,
        "planned_rows": {
            "total": planned_row_count,
            "planned": planned_pending_count,
            "materialized": planned_materialized_count,
            "next": next_planned_row,
        },
        "parallel": {
            "candidate_count": parallel_candidate_count,
            "mode": current_task_card.pointer("/parallel_fanout/mode").cloned().unwrap_or(Value::Null),
            "required_lane_ids": parallel_required_lane_ids,
            "fan_in_ready": host_subagent_state.pointer("/parallel_lane_state/fan_in_ready").cloned().unwrap_or(Value::Bool(false)),
            "active_lane_count": host_subagent_state.pointer("/parallel_lane_state/active_lane_count").cloned().unwrap_or(Value::from(0)),
            "terminal_lane_count": host_subagent_state.pointer("/parallel_lane_state/terminal_lane_count").cloned().unwrap_or(Value::from(0)),
        },
        "blocked": {
            "blocked": state == "blocked",
            "reason": blocked_reason,
        },
        "owns": {
            "next_task_selection": true,
            "planned_row_materialization": true,
            "bounded_parallel_fanout": true,
            "blocked_work": true,
            "pending_card_updates": true,
        }
    })
}

fn role_gate_family(current_task_card: &Value) -> &'static str {
    let role = current_task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let agent = current_task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(role, "way") || matches!(agent, "tactician" | "ccc_tactician") {
        "planning"
    } else if matches!(role, "code specialist" | "documenter")
        || matches!(agent, "raider" | "ccc_raider" | "scribe" | "ccc_scribe")
    {
        "mutation"
    } else if matches!(role, "verifier")
        || matches!(
            agent,
            "arbiter" | "ccc_arbiter" | "sentinel" | "ccc_sentinel"
        )
    {
        "verification"
    } else {
        "evidence"
    }
}

fn create_state_contract_payload(
    current_task_card: &Value,
    next_step: &str,
    can_advance: bool,
    captain_action_contract: &Value,
    post_fan_in_captain_decision: &Value,
) -> Value {
    let state = post_fan_in_captain_decision
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("decision_pending");
    let active_gate = post_fan_in_captain_decision
        .get("active_gate")
        .and_then(Value::as_str)
        .unwrap_or("captain_decision");
    let required_artifact = post_fan_in_captain_decision
        .get("required_artifact")
        .and_then(Value::as_str)
        .unwrap_or("captain_decision_envelope");
    let mutation_evidence_gate = current_task_card
        .get("mutation_evidence_gate")
        .cloned()
        .unwrap_or(Value::Null);
    let allowed_next_transitions = match post_fan_in_captain_decision
        .get("precedence")
        .and_then(Value::as_str)
        .unwrap_or("advance")
    {
        "terminal" => vec!["report_terminal_status"],
        "operator" => vec!["answer_way_clarification"],
        "longway_approval" => vec!["approve_longway"],
        "review_pass_cap" | "review_needs_work" | "review_gate" => {
            vec!["accept_review", "repair", "reassign", "block"]
        }
        "recovery" => vec!["retry", "reassign", "record_fallback", "close_host_thread"],
        "await_fan_in" => vec!["record_fan_in", "record_fallback"],
        "dispatch" => vec!["dispatch_specialist", "record_fan_in", "record_fallback"],
        _ => vec!["report", "dispatch_next", "retry", "reassign", "block"],
    };

    let allowed_next_commands = allowed_next_transitions
        .into_iter()
        .map(|value| Value::String(value.to_string()))
        .collect::<Vec<_>>();
    json!({
        "schema": "ccc.state_contract.v1",
        "source": "status_payload",
        "state": state,
        "active_gate": active_gate,
        "required_artifact": required_artifact,
        "next_step": next_step,
        "can_advance": can_advance,
        "allowed_next_transitions": allowed_next_commands,
        "captain_required_action": captain_action_contract.get("required_action").cloned().unwrap_or(Value::Null),
        "captain_allowed_action": captain_action_contract.get("allowed_action").cloned().unwrap_or(Value::Null),
        "task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "assigned_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "mutation_evidence_gate": mutation_evidence_gate,
        "post_fan_in_captain_decision": post_fan_in_captain_decision,
    })
}

fn active_conflict_state(host_subagent_state: &Value, current_task_card: &Value) -> Value {
    let reclaim_attention = host_subagent_state
        .pointer("/reclaim_replan_recommendation/needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recovery_attention = host_subagent_state
        .pointer("/recovery_recommendation/needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let active_subagent_count = host_subagent_state
        .get("active_subagent_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let assignment_drift = current_task_card
        .pointer("/assignment_quality/state")
        .and_then(Value::as_str)
        .is_some_and(|state| state != "matched");
    let assignment_drift_severity = current_task_card
        .pointer("/assignment_quality/drift_severity")
        .and_then(Value::as_str)
        .unwrap_or(if assignment_drift { "blocking" } else { "none" });
    let assignment_drift_blocks =
        assignment_drift && !matches!(assignment_drift_severity, "none" | "info" | "non_blocking");
    let policy_drift = current_task_card
        .pointer("/subagent_policy_drift/ok")
        .and_then(Value::as_bool)
        == Some(false);

    json!({
        "active_subagent_count": active_subagent_count,
        "reclaim_needs_attention": reclaim_attention,
        "recovery_needs_attention": recovery_attention,
        "assignment_drift": assignment_drift,
        "assignment_drift_severity": assignment_drift_severity,
        "policy_drift": policy_drift,
        "blocked": reclaim_attention || recovery_attention || assignment_drift_blocks || policy_drift,
    })
}

fn create_recovery_lane_payload(host_subagent_state: &Value) -> Value {
    let recovery = host_subagent_state
        .get("recovery_recommendation")
        .filter(|value| value.is_object())
        .unwrap_or(&Value::Null);
    let reclaim = host_subagent_state
        .get("reclaim_replan_recommendation")
        .filter(|value| value.is_object())
        .unwrap_or(&Value::Null);
    let recovery_action = recovery
        .get("recommended_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("none");
    let reclaim_action = reclaim
        .get("recommended_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("none");
    let recovery_attention = recovery
        .get("needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reclaim_attention = reclaim
        .get("needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = match recovery_action {
        "none" if reclaim_action == "await_fan_in_or_replan" => "watching",
        "none" => "clear",
        "reclaim" => "reclaim_pending",
        "retry" | "reassign" => "recovery_pending",
        _ => "recovery_pending",
    };
    let targets = recovery
        .get("targets")
        .and_then(Value::as_array)
        .or_else(|| reclaim.get("targets").and_then(Value::as_array));
    let target_count = targets.map(Vec::len).unwrap_or(0);
    let bounded_targets = targets
        .map(|values| values.iter().take(4).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let summary = if recovery_action != "none" {
        recovery.get("summary")
    } else {
        reclaim.get("summary").or_else(|| recovery.get("summary"))
    }
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(|value| summarize_text_for_visibility(value, 220))
    .unwrap_or_else(|| "No host subagent recovery action is currently needed.".to_string());

    json!({
        "source": "host_subagent_state",
        "status": status,
        "recommended_action": recovery_action,
        "reclaim_replan_action": reclaim_action,
        "needs_operator_attention": recovery_attention || reclaim_attention,
        "prefer_before_degraded_fallback": recovery
            .get("prefer_before_degraded_fallback")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "target_count": target_count,
        "targets": bounded_targets,
        "summary": summary,
    })
}

fn task_model_value(current_task_card: &Value, field: &str) -> Value {
    current_task_card
        .pointer(&format!("/latest_model_launch/dispatched_{field}"))
        .or_else(|| current_task_card.pointer(&format!("/runtime_dispatch/{field}")))
        .or_else(|| {
            current_task_card.pointer(&format!("/delegation_plan/runtime_dispatch/{field}"))
        })
        .or_else(|| current_task_card.pointer(&format!("/delegation_plan/{field}")))
        .or_else(|| current_task_card.pointer(&format!("/role_config_snapshot/{field}")))
        .cloned()
        .unwrap_or(Value::Null)
}

fn count_string_array(value: Option<&Value>) -> usize {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|item| !item.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}

fn bounded_string_array(value: Option<&Value>, max_items: usize, max_chars: usize) -> Vec<Value> {
    let Some(value) = value else {
        return Vec::new();
    };
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .take(max_items)
            .map(|item| Value::String(summarize_text_for_visibility(item, max_chars)))
            .collect::<Vec<_>>();
    }
    value
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            vec![Value::String(summarize_text_for_visibility(
                item, max_chars,
            ))]
        })
        .unwrap_or_default()
}

fn append_bounded_strings(
    output: &mut Vec<Value>,
    value: Option<&Value>,
    max_items: usize,
    max_chars: usize,
) {
    for item in bounded_string_array(value, max_items, max_chars) {
        if !output.iter().any(|existing| existing == &item) {
            output.push(item);
        }
    }
}

fn create_verification_capsule_payload(task_card: &Value) -> Value {
    if !task_card.is_object() {
        return Value::Null;
    }

    let subagent_fan_in = task_card.get("subagent_fan_in").unwrap_or(&Value::Null);
    let review_fan_in = task_card.get("review_fan_in").unwrap_or(&Value::Null);
    let mut evidence = Vec::new();
    append_bounded_strings(&mut evidence, task_card.get("evidence_links"), 6, 160);
    append_bounded_strings(&mut evidence, subagent_fan_in.get("evidence_paths"), 6, 160);
    append_bounded_strings(&mut evidence, review_fan_in.get("evidence_paths"), 6, 160);

    let mut validation = Vec::new();
    for key in ["validation_commands", "validation_checks", "checks"] {
        append_bounded_strings(&mut validation, subagent_fan_in.get(key), 6, 180);
        append_bounded_strings(&mut validation, review_fan_in.get(key), 6, 180);
    }

    let mut unresolved = Vec::new();
    append_bounded_strings(
        &mut unresolved,
        subagent_fan_in.get("open_questions"),
        6,
        180,
    );
    append_bounded_strings(
        &mut unresolved,
        subagent_fan_in.get("unresolved_risks"),
        6,
        180,
    );
    append_bounded_strings(
        &mut unresolved,
        review_fan_in.get("unresolved_findings"),
        6,
        180,
    );

    // The capsule is a compact closeout index over persisted task-card and fan-in truth;
    // full evidence and review bodies stay on their native structured fields.
    json!({
        "schema": "ccc.verification_capsule.v1",
        "acceptance": summarize_checkpoint_text(task_card.get("acceptance"), 240),
        "evidence": {
            "count": evidence.len(),
            "items": evidence,
        },
        "reviewer_verdict": review_fan_in
            .get("outcome")
            .or_else(|| review_fan_in.get("review_outcome"))
            .or_else(|| review_fan_in.get("status"))
            .cloned()
            .unwrap_or(Value::Null),
        "validation": {
            "count": validation.len(),
            "commands_or_checks": validation,
        },
        "unresolved_risk": {
            "count": review_fan_in
                .get("unresolved_finding_count")
                .and_then(Value::as_u64)
                .or_else(|| subagent_fan_in.get("unresolved_risk_count").and_then(Value::as_u64))
                .unwrap_or(unresolved.len() as u64),
            "items": unresolved,
        },
    })
}

fn create_delegated_ownership_payload(task_card: &Value) -> Value {
    if !task_card.is_object() {
        return Value::Null;
    }

    let routing_trace = task_card.get("routing_trace").unwrap_or(&Value::Null);
    let paths = bounded_string_array(routing_trace.get("paths"), 8, 160);
    let terms = bounded_string_array(routing_trace.get("terms"), 8, 120);
    let mutation_intent = routing_trace
        .get("mutation_intent")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let lifecycle_status = task_card
        .pointer("/subagent_lifecycle/status")
        .or_else(|| task_card.pointer("/review_lifecycle/status"))
        .cloned()
        .unwrap_or(Value::Null);
    let reclaim_recorded = task_card
        .pointer("/captain_intervention/chosen_next_action")
        .and_then(Value::as_str)
        == Some("reclaim")
        || task_card
            .get("subagent_fallback")
            .is_some_and(|value| !value.is_null());
    let stale_output_recorded = task_card
        .get("late_subagent_output")
        .is_some_and(|value| !value.is_null())
        || task_card
            .pointer("/captain_intervention/stale_output_policy")
            .is_some();

    json!({
        "schema": "ccc.delegated_ownership.v1",
        "owner": {
            "task_card_id": task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
            "assigned_role": task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
            "assigned_agent_id": task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
            "child_agent_id": task_card
                .pointer("/subagent_lifecycle/child_agent_id")
                .or_else(|| task_card.pointer("/review_lifecycle/child_agent_id"))
                .cloned()
                .unwrap_or(Value::Null),
            "status": lifecycle_status,
        },
        "search_ownership": {
            "paths": paths,
            "terms": terms,
        },
        "mutation_ownership": {
            "active": mutation_intent.contains("mutation") || mutation_intent.contains("write"),
            "paths": bounded_string_array(routing_trace.get("paths"), 8, 160),
        },
        "repeat_guard": {
            "policy": "do_not_repeat_delegated_search_or_mutation_without_recorded_reason",
            "allowed_reasons": ["reclaim_recorded", "stale_output_recorded", "explicit_reason_recorded"],
            "reclaim_recorded": reclaim_recorded,
            "stale_output_recorded": stale_output_recorded,
        },
    })
}

fn create_task_session_state_payload(
    session_context: &SessionContext,
    current_task_card: &Value,
    state_contract: &Value,
    recovery_lane: &Value,
) -> Value {
    if !current_task_card.is_object() {
        return Value::Null;
    }

    let subagent_lifecycle = current_task_card
        .get("subagent_lifecycle")
        .filter(|value| value.is_object())
        .or_else(|| {
            current_task_card
                .get("review_lifecycle")
                .filter(|value| value.is_object())
        });
    let subagent_fan_in = current_task_card
        .get("subagent_fan_in")
        .unwrap_or(&Value::Null);
    let review_fan_in = current_task_card
        .get("review_fan_in")
        .unwrap_or(&Value::Null);
    let evidence_count = count_string_array(subagent_fan_in.get("evidence_paths"))
        + count_string_array(review_fan_in.get("evidence_paths"));
    let verification_state = current_task_card
        .get("verification_state")
        .cloned()
        .unwrap_or(Value::Null);
    let unresolved_risk_count = review_fan_in
        .get("unresolved_finding_count")
        .and_then(Value::as_u64)
        .or_else(|| {
            subagent_fan_in
                .get("unresolved_risk_count")
                .and_then(Value::as_u64)
        })
        .unwrap_or_else(|| count_string_array(subagent_fan_in.get("open_questions")) as u64);
    let fallback_recorded = current_task_card
        .get("subagent_fallback")
        .is_some_and(|value| !value.is_null());
    let fallback_status = if fallback_recorded {
        "recorded"
    } else {
        recovery_lane
            .get("status")
            .and_then(Value::as_str)
            .filter(|status| *status != "clear")
            .unwrap_or("none")
    };

    // This joins only compact persisted truth needed for watch/tmux status
    // inspection; larger task cards and fan-in bodies remain on their native
    // structured fields.
    json!({
        "schema": "ccc.task_session_state.v1",
        "source": "status_payload",
        "internal_plumbing": true,
        "public_command_path": false,
        "active_task": {
            "task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
            "title": summarize_checkpoint_text(current_task_card.get("title"), 160),
            "status": current_task_card.get("status").cloned().unwrap_or(Value::Null),
            "assigned_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
            "assigned_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        },
        "current_gate": {
            "state": state_contract.get("state").cloned().unwrap_or(Value::Null),
            "active_gate": state_contract.get("active_gate").cloned().unwrap_or(Value::Null),
            "required_artifact": state_contract.get("required_artifact").cloned().unwrap_or(Value::Null),
        },
        "delegated_agent": {
            "child_agent_id": subagent_lifecycle.and_then(|value| value.get("child_agent_id")).cloned().unwrap_or(Value::Null),
            "status": subagent_lifecycle.and_then(|value| value.get("status")).cloned().unwrap_or(Value::Null),
            "model": task_model_value(current_task_card, "model"),
            "variant": task_model_value(current_task_card, "variant"),
        },
        "fallback_state": {
            "status": fallback_status,
            "recorded": fallback_recorded,
            "recommended_action": recovery_lane.get("recommended_action").cloned().unwrap_or(Value::String("none".to_string())),
        },
        "evidence": {
            "count": evidence_count,
            "latest_summary": summarize_checkpoint_text(
                subagent_fan_in
                    .get("summary")
                    .or_else(|| review_fan_in.get("summary")),
                160,
            ),
        },
        "verification_capsule": current_task_card.get("verification_capsule").cloned().unwrap_or_else(|| create_verification_capsule_payload(current_task_card)),
        "delegated_ownership": current_task_card.get("delegated_ownership").cloned().unwrap_or_else(|| create_delegated_ownership_payload(current_task_card)),
        "verification": {
            "state": verification_state,
            "review_outcome": review_fan_in.get("outcome").cloned().unwrap_or(Value::Null),
            "unresolved_risk_count": unresolved_risk_count,
        },
        "internal_session": {
            "session_id": session_context.session_id,
            "process_id": session_context.process_id,
            "started_at": session_context.started_at,
        },
    })
}

fn create_workflow_loop_payload(
    current_task_card: &Value,
    longway: &Value,
    task_session_state: &Value,
    state_contract: &Value,
    run_truth_surface: &Value,
) -> Value {
    if !current_task_card.is_object() && !longway.is_object() {
        return Value::Null;
    }

    let current_stage = workflow_current_stage(
        current_task_card,
        longway,
        task_session_state,
        state_contract,
        run_truth_surface,
    );
    let verification_complete = current_task_card
        .get("verification_state")
        .and_then(Value::as_str)
        .is_some_and(|state| matches!(state, "passed" | "verified" | "completed"));
    let stage_ids = [
        "requirements_understanding",
        "planning",
        "exploration",
        "modification",
        "review",
        "verification",
    ];
    let current_index = stage_ids
        .iter()
        .position(|stage| *stage == current_stage)
        .unwrap_or(0);
    let stages = stage_ids
        .iter()
        .enumerate()
        .map(|(index, stage_id)| {
            let status = if index < current_index
                || (verification_complete && *stage_id == "verification")
            {
                "completed"
            } else if index == current_index {
                "active"
            } else {
                "pending"
            };
            json!({
                "id": stage_id,
                "label": workflow_stage_label(stage_id),
                "status": status,
                "evidence": workflow_stage_evidence(stage_id, current_task_card, longway, task_session_state, state_contract, run_truth_surface),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema": "ccc.workflow_loop.v1",
        "source": "persisted_longway_task_card_truth",
        "operator_visible": true,
        "public_commands": false,
        "current_stage": current_stage,
        "status": if verification_complete { "completed" } else { "active" },
        "summary": "requirements understanding -> planning -> exploration -> modification -> review -> verification",
        "stages": stages,
    })
}

fn workflow_current_stage(
    current_task_card: &Value,
    longway: &Value,
    task_session_state: &Value,
    state_contract: &Value,
    run_truth_surface: &Value,
) -> &'static str {
    let verification_state = current_task_card
        .get("verification_state")
        .and_then(Value::as_str)
        .unwrap_or("pending");
    if matches!(verification_state, "passed" | "verified" | "completed") {
        return "verification";
    }

    let subagent_status = current_task_card
        .pointer("/subagent_lifecycle/status")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task_card
                .pointer("/subagent_fan_in/status")
                .and_then(Value::as_str)
        });
    let assigned_role = current_task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let assigned_agent = current_task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let active_subagent = subagent_status
        .is_some_and(|status| matches!(status, "spawned" | "acknowledged" | "running"));
    if active_subagent && workflow_role_is_exploration(&assigned_role, &assigned_agent) {
        return "exploration";
    }
    if active_subagent && workflow_role_is_modification(&assigned_role, &assigned_agent) {
        return "modification";
    }

    let approval_state = longway
        .get("approval_state")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let lifecycle_state = longway
        .get("lifecycle_state")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let active_phase = longway
        .get("active_phase_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if approval_state == "pending_longway_approval"
        || matches!(lifecycle_state, "planning" | "planned")
        || matches!(active_phase.as_str(), "plan" | "way")
    {
        return "planning";
    }

    let active_gate = state_contract
        .get("active_gate")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let review_status = current_task_card
        .pointer("/review_lifecycle/status")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task_card
                .pointer("/review_fan_in/outcome")
                .and_then(Value::as_str)
        });
    let fan_in_ready = run_truth_surface
        .get("fan_in_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if active_gate.contains("review") || review_status.is_some() || fan_in_ready {
        return "review";
    }

    if workflow_role_is_exploration(&assigned_role, &assigned_agent)
        || matches!(active_phase.as_str(), "inspect" | "explore" | "research")
    {
        return "exploration";
    }
    if workflow_role_is_modification(&assigned_role, &assigned_agent)
        || matches!(active_phase.as_str(), "mutate" | "execute")
    {
        return "modification";
    }
    if task_session_state.is_object()
        || longway
            .get("phase_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    {
        return "planning";
    }

    "requirements_understanding"
}

fn workflow_role_is_exploration(role: &str, agent: &str) -> bool {
    role.contains("read")
        || role.contains("explor")
        || role.contains("scout")
        || agent.contains("scout")
        || agent.contains("reader")
}

fn workflow_role_is_modification(role: &str, agent: &str) -> bool {
    role.contains("code")
        || role.contains("operator")
        || role.contains("scribe")
        || role.contains("implementation")
        || agent.contains("raider")
        || agent.contains("operator")
        || agent.contains("scribe")
}

fn workflow_stage_label(stage_id: &str) -> &'static str {
    match stage_id {
        "requirements_understanding" => "requirements understanding",
        "planning" => "planning",
        "exploration" => "exploration",
        "modification" => "modification",
        "review" => "review",
        "verification" => "verification",
        _ => "unknown",
    }
}

fn workflow_stage_evidence(
    stage_id: &str,
    current_task_card: &Value,
    longway: &Value,
    task_session_state: &Value,
    state_contract: &Value,
    run_truth_surface: &Value,
) -> Value {
    match stage_id {
        "requirements_understanding" => json!({
            "task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
            "title": current_task_card.get("title").map(|value| summarize_checkpoint_text(Some(value), 120)).unwrap_or(Value::Null),
        }),
        "planning" => json!({
            "phase_count": longway.get("phase_count").cloned().unwrap_or(Value::Null),
            "planned_row_count": longway.get("planned_row_count").cloned().unwrap_or(Value::Null),
            "approval_state": longway.get("approval_state").cloned().unwrap_or(Value::Null),
        }),
        "exploration" => json!({
            "evidence_need": current_task_card.pointer("/routing_trace/evidence_need").cloned().unwrap_or(Value::Null),
            "assigned_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        }),
        "modification" => json!({
            "mutation_intent": current_task_card.pointer("/routing_trace/mutation_intent").cloned().unwrap_or(Value::Null),
            "delegated_agent_status": task_session_state.pointer("/delegated_agent/status").cloned().unwrap_or(Value::Null),
        }),
        "review" => json!({
            "active_gate": state_contract.get("active_gate").cloned().unwrap_or(Value::Null),
            "review_outcome": current_task_card.pointer("/review_fan_in/outcome").cloned().unwrap_or(Value::Null),
            "fan_in_ready": run_truth_surface.get("fan_in_ready").cloned().unwrap_or(Value::Null),
        }),
        "verification" => json!({
            "verification_state": current_task_card.get("verification_state").cloned().unwrap_or(Value::Null),
            "validation_count": current_task_card.pointer("/verification_capsule/validation/count").cloned().unwrap_or(Value::Null),
        }),
        _ => Value::Null,
    }
}

fn create_registry_evidence_status_payload(current_task_card: &Value) -> Value {
    let registry = current_task_card
        .pointer("/delegation_plan/skill_registry")
        .filter(|value| value.is_object())
        .or_else(|| {
            current_task_card
                .pointer("/delegation_plan/runtime_dispatch/skill_registry")
                .filter(|value| value.is_object())
        });
    let Some(registry) = registry else {
        return Value::Null;
    };
    let skill_ssl_manifest = registry
        .get("skill_ssl_manifest")
        .or_else(|| current_task_card.pointer("/delegation_plan/skill_ssl_manifest"))
        .unwrap_or(&Value::Null);

    json!({
        "schema": "ccc.registry_evidence_status.v1",
        "source": "skill_registry",
        "agent_name": registry.get("agent_name").cloned().unwrap_or_else(|| current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null)),
        "status": registry.get("status").cloned().unwrap_or(Value::String("missing".to_string())),
        "manifest_status": registry.get("manifest_status").cloned().unwrap_or_else(|| skill_ssl_manifest.get("status").cloned().unwrap_or(Value::Null)),
        "blocking": registry.get("blocking").cloned().unwrap_or(Value::Bool(false)),
        "runtime_truth": registry.get("runtime_truth").cloned().unwrap_or(Value::Bool(false)),
        "advisory_only": registry.get("advisory_only").cloned().unwrap_or(Value::Bool(true)),
        "skill_ssl_manifest": {
            "status": skill_ssl_manifest.get("status").cloned().unwrap_or(Value::Null),
            "path": skill_ssl_manifest.get("path").cloned().unwrap_or(Value::Null),
            "reason": skill_ssl_manifest.get("reason").cloned().unwrap_or(Value::Null),
            "blocking": skill_ssl_manifest.get("blocking").cloned().unwrap_or(Value::Bool(false)),
            "advisory_only": skill_ssl_manifest.get("advisory_only").cloned().unwrap_or(Value::Bool(true)),
        }
    })
}

fn create_context_health_payload(
    run_id: &str,
    next_step: &str,
    can_advance: bool,
    current_task_card: &Value,
    longway: &Value,
    host_subagent_state: &Value,
    long_session_mitigation: &Value,
    captain_action_contract: &Value,
) -> Value {
    let recommended_rollover = long_session_mitigation
        .get("recommended")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_action = long_session_mitigation
        .get("recommended_action")
        .and_then(Value::as_str)
        .unwrap_or("continue");
    let pressure_signals = long_session_mitigation
        .get("signals")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let conflict_state = active_conflict_state(host_subagent_state, current_task_card);
    let conflict_blocked = conflict_state
        .get("blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let safe_action = if recommended_rollover {
        "checkpoint_then_operator_rollover"
    } else if conflict_blocked {
        "resolve_active_conflict_before_dispatch"
    } else if can_advance {
        "captain_advance"
    } else {
        "wait_for_current_step"
    };

    json!({
        "schema": "ccc.context_health.v1",
        "run_id": run_id,
        "status": if recommended_rollover || conflict_blocked { "attention_needed" } else { "ok" },
        "safe_action": safe_action,
        "next_step": next_step,
        "can_advance": can_advance,
        "pressure_signals": pressure_signals,
        "recommended_rollover_action": recommended_action,
        "checkpoint_required": long_session_mitigation.get("checkpoint_required").cloned().unwrap_or(Value::Bool(false)),
        "active_conflict_state": conflict_state,
        "longway_state": longway.get("lifecycle_state").cloned().unwrap_or(Value::Null),
        "next_task": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "captain_allowed_action": captain_action_contract.get("allowed_action").cloned().unwrap_or(Value::Null),
        "operator_warning": if recommended_rollover {
            "checkpoint CCC before using /compact, /new, or /exit; CCC does not execute Codex CLI slash commands automatically"
        } else if conflict_blocked {
            "resolve the active host subagent or routing conflict before dispatching more work"
        } else {
            "context health is within normal bounds"
        }
    })
}

fn create_restart_handoff_payload(
    run_id: &str,
    run_ref: &str,
    current_task_card: &Value,
    longway: &Value,
    context_health: &Value,
    long_session_mitigation: &Value,
) -> Value {
    let restart_needed = context_health.get("status").and_then(Value::as_str)
        == Some("attention_needed")
        && long_session_mitigation
            .get("recommended")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let resume_command = long_session_mitigation
        .get("resume_command")
        .cloned()
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| Value::String(format!("$cap continue {run_id}")));

    json!({
        "schema": "ccc.restart_handoff.v1",
        "restart_needed": restart_needed,
        "automatic_restart": false,
        "run_id": run_id,
        "run_ref": run_ref,
        "resume_command": resume_command,
        "checkpoint_command": long_session_mitigation.get("checkpoint_command").cloned().unwrap_or(Value::Null),
        "current_longway_state": longway.get("lifecycle_state").cloned().unwrap_or(Value::Null),
        "next_task": {
            "task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
            "title": current_task_card.get("title").cloned().unwrap_or(Value::Null),
            "assigned_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
            "assigned_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        },
        "active_conflict_state": context_health.get("active_conflict_state").cloned().unwrap_or(Value::Null),
        "operator_warning": "manual restart or rollover only; CCC records the handoff but does not restart Codex CLI automatically",
    })
}

fn summarize_checkpoint_text(value: Option<&Value>, max_chars: usize) -> Value {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
        .unwrap_or(Value::Null)
}

fn checkpoint_late_output_payload(current_task_card: &Value) -> Value {
    let late_output = current_task_card
        .get("late_subagent_output")
        .unwrap_or(&Value::Null);
    let captain_intervention = current_task_card
        .get("captain_intervention")
        .unwrap_or(&Value::Null);
    let (count, representative) = if let Some(items) = late_output.as_array() {
        (items.len(), items.last().unwrap_or(&Value::Null))
    } else if late_output.is_object() {
        (1, late_output)
    } else {
        (0, late_output)
    };
    let state = representative
        .get("state")
        .or_else(|| representative.get("status"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(if count > 0 { "recorded" } else { "none" });

    json!({
        "state": state,
        "count": count,
        "status": representative.get("status").cloned().unwrap_or(Value::Null),
        "authority": representative
            .get("authority")
            .cloned()
            .or_else(|| captain_intervention.get("stale_output_policy").cloned())
            .unwrap_or(Value::Null),
        "summary": summarize_checkpoint_text(representative.get("summary"), 180),
    })
}

fn create_active_checkpoint_payload(
    run_id: &str,
    run_record: &Value,
    current_task_card: &Value,
    run_truth_surface: &Value,
    host_subagent_state: &Value,
    state_contract: &Value,
    captain_action_contract: &Value,
    next_step: &str,
) -> Value {
    if run_record.get("status").and_then(Value::as_str) != Some("active")
        || !current_task_card.is_object()
    {
        return Value::Null;
    }

    let worker_total = run_truth_surface
        .get("worker_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let worker_active = run_truth_surface
        .get("worker_active")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let host_total = run_truth_surface
        .get("host_subagent_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let host_active = run_truth_surface
        .get("host_subagent_active")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let pending_approval_state = run_record
        .get("approval_state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let approval_pending =
        pending_approval_state.starts_with("pending") || next_step == "await_operator";
    let resume_action = run_truth_surface
        .get("resume_action")
        .cloned()
        .unwrap_or_else(|| Value::String(next_step.to_string()));
    let next_legal_action = captain_action_contract
        .get("required_action")
        .or_else(|| captain_action_contract.get("allowed_action"))
        .cloned()
        .unwrap_or(Value::Null);

    // The checkpoint is intentionally compact: it joins persisted truth without
    // copying full task cards, lane payloads, or intervention history.
    json!({
        "schema": "ccc.active_checkpoint.v1",
        "run_id": run_id,
        "status": run_record.get("status").cloned().unwrap_or(Value::Null),
        "current_gate": state_contract.get("active_gate").cloned().unwrap_or(Value::Null),
        "task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "title": summarize_checkpoint_text(current_task_card.get("title"), 160),
        "assigned_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "active_thread_id": run_record.get("active_thread_id").cloned().unwrap_or(Value::Null),
        "delegated_work": {
            "summary": format!("workers={worker_active}/{worker_total} host_subagents={host_active}/{host_total}"),
            "worker_total": worker_total,
            "worker_active": worker_active,
            "host_subagent_total": host_total,
            "host_subagent_active": host_active,
            "pending_merge_count": host_subagent_state.get("pending_merge_count").cloned().unwrap_or(Value::Null),
            "active_lane_count": host_subagent_state.pointer("/parallel_lane_state/active_lane_count").cloned().unwrap_or(Value::Null),
            "ownership": current_task_card.get("delegated_ownership").cloned().unwrap_or_else(|| create_delegated_ownership_payload(current_task_card)),
        },
        "fan_in_state": {
            "ready": run_truth_surface.get("fan_in_ready").cloned().unwrap_or(Value::Bool(false)),
            "host_subagent_ready": host_subagent_state.get("fan_in_ready").cloned().unwrap_or(Value::Null),
            "worker_active": worker_active,
            "pending_merge_count": host_subagent_state.get("pending_merge_count").cloned().unwrap_or(Value::Null),
            "parallel_lane_state": host_subagent_state.get("parallel_lane_state").cloned().unwrap_or(Value::Null),
        },
        "pending_approval": {
            "pending": approval_pending,
            "approval_state": pending_approval_state,
        },
        "next_legal_action": next_legal_action,
        "resume_action": resume_action,
        "continuation_command": format!("$cap continue {run_id}"),
        "late_output": checkpoint_late_output_payload(current_task_card),
    })
}

fn create_operator_visible_transport_guidance() -> Value {
    json!({
        "preferred_transport": "ccc_cli_quiet_subcommand",
        "transcript_signal": "ran",
        "reason": "Operator-visible CCC lifecycle mutations should be recorded as quiet inline-JSON CLI subcommands instead of MCP tool calls.",
        "lifecycle_mutations": ["start", "orchestrate", "subagent-update", "memory"],
        "default_payload_transport": "inline_json",
        "longway_visibility": "Use CCC_LONGWAY_PROJECTION.md for normal progress visibility; refresh it with ccc status --projection --json '{...}' or ccc checklist --projection --json '{...}'.",
        "preferred_command_shapes": {
            "start": ["ccc start --quiet --json '{...}'"],
            "orchestrate": ["ccc orchestrate --quiet --json '{...}'"],
            "subagent_update": ["ccc subagent-update --quiet --json '{...}'"],
            "memory": ["ccc memory --quiet --json '{...}'"],
            "projection": ["ccc status --projection --json '{...}'"]
        },
        "mcp_reserved_for": ["app surfaces", "structured inspection", "CLI unavailable"],
    })
}

fn create_graph_context_status_payload(config: &Value, workspace_root: &Path) -> Value {
    match create_graph_context_readiness_payload(config, workspace_root) {
        Ok(payload) => normalize_graph_context_status_fallback(payload),
        Err(error) => json!({
            "schema": "ccc.graph_context_readiness.status.v1",
            "provider": "graphify",
            "readiness": "unavailable",
            "reason": "inspection_error",
            "fallback_when_unavailable": "scout_source_evidence",
            "fallback": "scout_source_evidence",
            "artifact_state": "inspection_error",
            "summary": format!("Graphify graph_context readiness could not be inspected: {error}."),
            "routing": {
                "graph_context_enabled": true,
                "graphify_queries_enabled": false,
                "legacy_code_graph_called": false,
                "legacy_fallback_disabled": true,
                "legacy_rebuild_disabled": true,
                "ccc_graph_backend": "graph_context_scout_source_evidence",
                "ccc_code_graph_backend": "graph_context_scout_source_evidence"
            }
        }),
    }
}

fn normalize_graph_context_status_fallback(mut payload: Value) -> Value {
    let readiness = payload
        .get("readiness")
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
        .to_string();
    let report_available = payload
        .pointer("/artifacts/report/available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let graph_available = payload
        .pointer("/artifacts/graph/available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let artifact_state = if report_available && graph_available {
        "present"
    } else if report_available || graph_available {
        "partial"
    } else {
        "missing"
    };
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "schema".to_string(),
            Value::String("ccc.graph_context_readiness.status.v1".to_string()),
        );
        object.insert(
            "artifact_state".to_string(),
            Value::String(artifact_state.to_string()),
        );
        if readiness != "available" && object.get("fallback_when_unavailable").is_none() {
            object.insert(
                "fallback_when_unavailable".to_string(),
                Value::String("scout_source_evidence".to_string()),
            );
        }
        if readiness != "available" && object.get("fallback").is_none() {
            object.insert(
                "fallback".to_string(),
                Value::String("scout_source_evidence".to_string()),
            );
        }
    }
    payload
}

fn operator_language_from_run_record(run_record: &Value) -> &'static str {
    for key in ["prompt", "request", "intent", "goal", "title"] {
        let Some(text) = run_record.get(key).and_then(Value::as_str) else {
            continue;
        };
        if text.chars().any(|ch| {
            ('\u{ac00}'..='\u{d7af}').contains(&ch)
                || ('\u{1100}'..='\u{11ff}').contains(&ch)
                || ('\u{3130}'..='\u{318f}').contains(&ch)
        }) {
            return "ko";
        }
    }
    "en"
}

pub(crate) fn create_ccc_status_payload(
    session_context: &SessionContext,
    locator: &ResolvedRunLocator,
) -> io::Result<Value> {
    let run_file = locator.run_directory.join("run.json");
    let run_record = read_json_document(&run_file)?;
    let orchestrator_state =
        read_optional_json_document(&locator.run_directory.join("orchestrator-state.json"))?;
    let shared_config =
        read_optional_toml_document(Path::new(&session_context.shared_config_path))?
            .unwrap_or(Value::Null);
    let runtime_config =
        load_runtime_config_from_path(Path::new(&session_context.shared_config_path))?;
    let current_task_card = create_current_task_card_payload(
        &locator.run_directory,
        run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
    )?;
    let longway = create_longway_payload(&locator.run_directory)?;
    let run_state = create_run_state_payload(&locator.run_directory)?;
    let worker_visibility = create_worker_visibility_payload(
        &locator.run_directory,
        run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
        &runtime_config,
    )?;
    let reclaim_plan = create_reclaim_plan_payload(&worker_visibility, &runtime_config);
    let token_usage = create_token_usage_payload(&locator.run_directory)?;
    let latest_delegate_result = create_latest_delegate_result_payload(
        &locator.run_directory,
        run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
    )?;
    let host_subagent_state = create_host_subagent_state_payload(
        &run_record,
        &current_task_card,
        run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
        &runtime_config,
    );
    let recovery_lane = create_recovery_lane_payload(&host_subagent_state);
    let token_usage_visibility =
        create_token_usage_visibility_payload(&token_usage, &host_subagent_state);
    let run_id_text = run_record
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or(&locator.run_id);
    let long_session_mitigation =
        create_long_session_mitigation_payload(run_id_text, &token_usage, &host_subagent_state);
    let next_step = run_state
        .get("next_action")
        .and_then(|value| {
            value
                .get("command")
                .or_else(|| value.get("action"))
                .or_else(|| value.get("type"))
        })
        .and_then(Value::as_str);
    let next_step = orchestrator_state
        .as_ref()
        .and_then(|value| {
            value
                .get("decision")
                .or_else(|| value.get("current_decision"))
        })
        .and_then(|value| value.get("next_step"))
        .and_then(Value::as_str)
        .or(next_step)
        .unwrap_or("advance");
    let worker_total = worker_visibility
        .get("total_worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let worker_active = worker_visibility
        .get("active_worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let host_subagent_fan_in_ready = host_subagent_state
        .get("fan_in_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let host_subagent_total = host_subagent_state
        .get("total_subagent_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let host_subagent_active = host_subagent_state
        .get("active_subagent_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_task_delegation_plan = current_task_card
        .get("delegation_plan")
        .cloned()
        .unwrap_or(Value::Null);
    let subagent_available = current_task_delegation_plan
        .get("subagent_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let subagent_fallback_recorded = current_task_card
        .get("subagent_fallback")
        .and_then(Value::as_object)
        .is_some();
    let subagent_fallback_ready = task_card_subagent_fallback_ready(
        &run_record,
        &current_task_card,
        run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
    );
    let fan_in_ready = next_step == "await_fan_in"
        && ((worker_total > 0 && worker_active == 0) || host_subagent_fan_in_ready);
    let decision_can_advance = orchestrator_state
        .as_ref()
        .and_then(|value| {
            value
                .get("decision")
                .or_else(|| value.get("current_decision"))
        })
        .and_then(|value| value.get("can_advance"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let can_advance = decision_can_advance || next_step == "advance" || fan_in_ready;
    let resume_action = if can_advance { "advance" } else { next_step };
    let fallback_execution_mode = fallback_specialist_execution_mode(&runtime_config);
    let codex_exec_fallback_allowed = fallback_execution_mode == "codex_exec"
        && (preferred_specialist_execution_mode(&runtime_config) != "codex_subagent"
            || !subagent_available
            || subagent_fallback_ready);
    let execution_strategy = json!({
        "preferred_specialist_execution_mode": preferred_specialist_execution_mode(&runtime_config),
        "fallback_specialist_execution_mode": fallback_execution_mode,
        "host_subagent_update_mode": "ccc_cli_subcommand",
        "operator_visible_transport": create_operator_visible_transport_guidance(),
        "codex_exec_fallback_allowed": codex_exec_fallback_allowed,
        "subagent_fallback_recorded": subagent_fallback_recorded,
        "subagent_fallback_ready": subagent_fallback_ready,
        "current_task_delegation_plan": current_task_delegation_plan,
    });
    let post_fan_in_captain_decision = create_post_fan_in_captain_decision_payload(
        &run_record,
        &longway,
        &current_task_card,
        &host_subagent_state,
        next_step,
        can_advance,
        fan_in_ready,
        worker_active,
        host_subagent_active,
    );
    let captain_action_contract =
        create_captain_action_contract(&current_task_card, &post_fan_in_captain_decision);
    let captain_direct_mutation_guard = create_captain_direct_mutation_guard(
        &locator.cwd,
        &run_record,
        &current_task_card,
        &captain_action_contract,
        subagent_fallback_ready,
    );
    let mutation_evidence_gate = run_record
        .get("mutation_evidence_gate")
        .or_else(|| {
            orchestrator_state
                .as_ref()
                .and_then(|value| value.get("mutation_evidence_gate"))
        })
        .cloned()
        .unwrap_or(Value::Null);
    let state_contract = create_state_contract_payload(
        &current_task_card,
        next_step,
        can_advance,
        &captain_action_contract,
        &post_fan_in_captain_decision,
    );
    let pending_captain_follow_up = current_task_card
        .pointer("/captain_intervention/pending_follow_up")
        .cloned()
        .filter(|value| value.is_object())
        .unwrap_or(Value::Null);
    let graph_context = create_graph_context_status_payload(&shared_config, &locator.cwd);
    let code_graph = create_code_graph_status_payload(&locator.cwd);
    let memory = create_memory_status_payload(&locator.cwd);
    let registry_evidence = create_registry_evidence_status_payload(&current_task_card);
    let scheduler = create_scheduler_status_payload(
        &locator.run_directory,
        &run_record,
        &longway,
        &current_task_card,
        &host_subagent_state,
        &captain_action_contract,
        &post_fan_in_captain_decision,
        next_step,
        can_advance,
        fan_in_ready,
    );
    let context_health = create_context_health_payload(
        run_id_text,
        next_step,
        can_advance,
        &current_task_card,
        &longway,
        &host_subagent_state,
        &long_session_mitigation,
        &captain_action_contract,
    );
    let run_ref = create_ccc_run_ref(&locator.run_directory);
    let restart_handoff = create_restart_handoff_payload(
        run_id_text,
        &run_ref,
        &current_task_card,
        &longway,
        &context_health,
        &long_session_mitigation,
    );
    let run_truth_surface = json!({
        "resume_action": resume_action,
        "fan_in_ready": fan_in_ready,
        "worker_total": worker_total,
        "worker_active": worker_active,
        "host_subagent_total": host_subagent_total,
        "host_subagent_active": host_subagent_active,
    });
    let active_checkpoint = create_active_checkpoint_payload(
        run_id_text,
        &run_record,
        &current_task_card,
        &run_truth_surface,
        &host_subagent_state,
        &state_contract,
        &captain_action_contract,
        next_step,
    );
    let task_session_state = create_task_session_state_payload(
        session_context,
        &current_task_card,
        &state_contract,
        &recovery_lane,
    );
    let lifecycle_hooks = create_lifecycle_hook_tiers_payload(
        &runtime_config,
        &current_task_card,
        &longway,
        &run_truth_surface,
        &active_checkpoint,
        &recovery_lane,
        &long_session_mitigation,
        &captain_direct_mutation_guard,
        &latest_delegate_result,
    );
    let workflow_loop = create_workflow_loop_payload(
        &current_task_card,
        &longway,
        &task_session_state,
        &state_contract,
        &run_truth_surface,
    );
    let status_truth_fallback =
        create_ccc_status_truth_fallback_fields(&run_record, &current_task_card, &longway);

    let mut payload = json!({
        "cwd": locator.cwd.to_string_lossy(),
        "run_id": run_record.get("run_id").cloned().unwrap_or(Value::String(locator.run_id.clone())),
        "run_file": run_file.to_string_lossy(),
        "run_directory": locator.run_directory.to_string_lossy(),
        "run_ref": run_ref,
        "goal": run_record.get("goal").cloned().unwrap_or(Value::Null),
        "operator_language": operator_language_from_run_record(&run_record),
        "status": run_record.get("status").cloned().unwrap_or(Value::Null),
        "stage": run_record.get("stage").cloned().unwrap_or(Value::Null),
        "sequence": run_record.get("sequence").cloned().unwrap_or(Value::Null),
        "approval_state": run_record.get("approval_state").cloned().unwrap_or(Value::Null),
        "active_role": run_record.get("active_role").cloned().unwrap_or(Value::Null),
        "active_agent_id": run_record.get("active_agent_id").cloned().unwrap_or(Value::Null),
        "active_task_card_id": run_record.get("active_task_card_id").cloned().unwrap_or(Value::Null),
        "active_thread_id": run_record.get("active_thread_id").cloned().unwrap_or(Value::Null),
        "task_card_count": run_record.get("task_card_ids").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        "latest_handoff_id": run_record.get("latest_handoff_id").cloned().unwrap_or(Value::Null),
        "latest_entry_trace": run_record.get("latest_entry_trace").cloned().unwrap_or(Value::Null),
        "way_clarification_request": status_truth_fallback.way_clarification_request,
        "prompt_refinement": status_truth_fallback.prompt_refinement,
        "review_policy": status_truth_fallback.review_policy,
        "completion_discipline": status_truth_fallback.completion_discipline,
        "latest_captain_intervention": status_truth_fallback.latest_captain_intervention,
        "latest_sentinel_intervention": status_truth_fallback.latest_sentinel_intervention,
        "pending_captain_follow_up": pending_captain_follow_up,
        "child_agent_count": run_record.get("child_agents").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        "specialist_executor_count": run_record.get("specialist_executors").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        "created_at": run_record.get("created_at").cloned().unwrap_or(Value::Null),
        "updated_at": run_record.get("updated_at").cloned().unwrap_or(Value::Null),
        "completed_at": run_record.get("completed_at").cloned().unwrap_or(Value::Null),
        "current_task_card": current_task_card,
        "longway": longway,
        "run_state": run_state,
        "next_step": next_step,
        "can_advance": can_advance,
        "run_truth_surface": run_truth_surface,
        "active_checkpoint": active_checkpoint,
        "task_session_state": task_session_state,
        "workflow_loop": workflow_loop,
        "lifecycle_hooks": lifecycle_hooks,
        "scheduler": scheduler,
        "state_contract": state_contract,
        "worker_visibility": worker_visibility,
        "host_subagent_state": host_subagent_state,
        "recovery_lane": recovery_lane,
        "reclaim_plan": reclaim_plan,
        "token_usage": token_usage,
        "token_usage_visibility": token_usage_visibility,
        "long_session_mitigation": long_session_mitigation,
        "context_health": context_health,
        "restart_handoff": restart_handoff,
        "latest_delegate_result": latest_delegate_result,
        "graph_context": graph_context,
        "code_graph": code_graph,
        "memory": memory,
        "registry_evidence": registry_evidence,
        "runtime_config": runtime_config,
        "execution_strategy": execution_strategy,
        "post_fan_in_captain_decision": post_fan_in_captain_decision,
        "captain_action_contract": captain_action_contract,
        "captain_direct_mutation_guard": captain_direct_mutation_guard,
        "mutation_evidence_gate": mutation_evidence_gate,
        "output": load_output_config(),
        "output_verbosity": load_output_verbosity(),
        "server_identity": create_server_identity_payload(session_context),
    });
    let visibility_signature = create_visibility_signature(&payload);
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "visibility_signature".to_string(),
            Value::String(visibility_signature),
        );
    }
    let cost_routing = create_cost_routing_payload(&runtime_config, &payload);
    if let Some(object) = payload.as_object_mut() {
        object.insert("cost_routing".to_string(), cost_routing);
    }
    let app_panel = create_codex_app_panel_payload(&payload);
    if let Some(object) = payload.as_object_mut() {
        object.insert("app_panel".to_string(), app_panel);
    }
    Ok(payload)
}
