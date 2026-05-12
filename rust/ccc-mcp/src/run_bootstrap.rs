use crate::code_graph::create_code_graph_status_payload;
use crate::memory::create_memory_status_payload;
use crate::parallel_fanout::maybe_create_parallel_fanout_payload;
use crate::request_routing::{
    combine_request_text_for_routing, create_routing_trace_payload, infer_mutation_intent,
    infer_request_shape, infer_task_shape,
};
use crate::review_policy::{
    review_policy_for_start_payload, runtime_review_pressure_snapshot_from_start_scan,
    RuntimeReviewPressureSnapshot,
};
use crate::run_locator::{
    create_ccc_run_ref, create_run_directory_from_workspace,
    create_workspace_run_directory_from_workspace, ensure_run_paths_for_start,
    inspect_active_runs_for_workspace, reclaim_prior_active_runs_for_workspace,
    resolve_workspace_path,
};
use crate::skill_registry::load_skill_registry_for_agent;
use crate::specialist_roles::{
    agent_id_for_role, apply_task_expertise_framing, assigned_role_for_task_kind,
    build_task_card_payload_with_role, create_specialist_delegation_plan,
    generated_custom_agent_name, load_role_config_snapshot, phase_name_for_role,
    sandbox_mode_for_role, sandbox_rationale_for_role,
};
use crate::target_workspace::resolve_target_workspace_root;
use crate::text_utils::summarize_text_for_visibility;
use crate::worktree_guard::create_worktree_mutation_baseline;
use crate::{
    acquire_run_mutation_lock, append_run_event, create_ccc_orchestrate_payload,
    generate_uuid_like_id, is_permission_error, read_json_document,
    read_optional_shared_config_document, write_json_document,
};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn phase_name_for_task_kind(task_kind: &str) -> &'static str {
    phase_name_for_role(assigned_role_for_task_kind(task_kind))
}

fn sequence_for_start(parsed: &Value) -> &'static str {
    match parsed.get("sequence").and_then(Value::as_str) {
        Some("PLAN_SEQUENCE") | Some("plan") => "PLAN_SEQUENCE",
        _ => "EXECUTE_SEQUENCE",
    }
}

fn is_plan_sequence(parsed: &Value) -> bool {
    sequence_for_start(parsed) == "PLAN_SEQUENCE"
}

fn effective_task_kind_for_start(parsed: &Value) -> String {
    if is_plan_sequence(parsed) {
        "way".to_string()
    } else {
        parsed
            .get("task_kind")
            .and_then(Value::as_str)
            .unwrap_or("execution")
            .to_string()
    }
}

fn plan_sequence_requires_way_clarification(parsed: &Value) -> bool {
    if !is_plan_sequence(parsed) {
        return false;
    }
    let request = combine_request_text_for_routing(parsed).to_lowercase();
    [
        "across",
        "cross-module",
        "repo-wide",
        "repository-wide",
        "strategy",
        "multiple",
        "multi-step",
        "several",
        "ambiguous",
        "unclear",
        "investigate",
        "diagnose",
        "plan the next",
        "next step",
        "전체",
        "여러",
        "복수",
        "다방면",
        "불명확",
    ]
    .iter()
    .any(|signal| request.contains(signal))
}

fn create_way_clarification_request(
    parsed: &Value,
    task_card_id: &str,
    timestamp: &str,
) -> Option<Value> {
    if !plan_sequence_requires_way_clarification(parsed) {
        return None;
    }
    let request_text = combine_request_text_for_routing(parsed);

    // Persist only high-signal questions. Narrow Way requests still proceed
    // with explicit assumptions and skip this interview state entirely.
    Some(json!({
        "schema": "ccc.way_clarification_request.v1",
        "state": "awaiting_operator",
        "task_card_id": task_card_id,
        "created_at": timestamp,
        "consumed_at": Value::Null,
        "source": "PLAN_SEQUENCE",
        "request_summary": summarize_text_for_visibility(&request_text, 240),
        "risk_triggers": [
            "broad_or_ambiguous_way_request",
            "scope_or_priority_unclear"
        ],
        "scope_assumptions": [
            "Do not materialize executable LongWay rows until the operator answers.",
            "Keep repository mutation blocked while clarification is pending."
        ],
        "questions": [
            {
                "id": "primary_outcome",
                "question": "What final outcome should this 0.0.13 pre-release slice optimize for first?",
                "answer_kind": "single_priority"
            },
            {
                "id": "scope_boundary",
                "question": "Which remaining work is explicitly in scope for this run, and what should stay deferred?",
                "answer_kind": "scope_boundary"
            },
            {
                "id": "risk_gate",
                "question": "What validation or release-readiness gate must block completion?",
                "answer_kind": "acceptance_gate"
            }
        ],
        "expected_answer_shape": "Answer each question briefly; CCC will consume the answer once and regenerate or amend the pending LongWay.",
        "copyable_follow_up": "$cap Answer the pending Way clarification for this run: primary_outcome=...; scope_boundary=...; risk_gate=...",
    }))
}

fn prompt_refinement_enabled_from_config(config: Option<&Value>) -> bool {
    config
        .and_then(|value| value.pointer("/features/prompt_refinement"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn goal_bridge_enabled_from_config(config: Option<&Value>) -> bool {
    let feature_enabled = config
        .and_then(|value| value.pointer("/features/goals"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let bridge_enabled = config
        .and_then(|value| value.pointer("/goal_bridge/enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    feature_enabled && bridge_enabled
}

fn goal_bridge_u64_from_config(config: Option<&Value>, pointer: &str, default_value: u64) -> u64 {
    config
        .and_then(|value| value.pointer(pointer))
        .and_then(Value::as_u64)
        .unwrap_or(default_value)
}

fn create_goal_bridge_record(
    enabled: bool,
    config: Option<&Value>,
    task_card_id: &str,
    timestamp: &str,
) -> Option<Value> {
    if !enabled {
        return None;
    }

    Some(json!({
        "schema": "ccc.goal_bridge.v1",
        "state": "planned",
        "enabled": true,
        "visibility": "internal",
        "execution_mode": "internal_non_executing",
        "owner": "captain",
        "mode": "captain_owned",
        "task_card_id": task_card_id,
        "created_at": timestamp,
        "recorded_at": timestamp,
        "brief_contract": {
            "language": "en",
            "max_lines": goal_bridge_u64_from_config(config, "/goal_bridge/brief_max_lines", 12),
            "require_verifiable_stop": true
        },
        "truth_contract": {
            "host_goal_state_is_truth": false,
            "authoritative_state": [
                "longway",
                "task_cards",
                "fan_in_records",
                "review_decisions",
                "fallback_records",
                "verification_capsules"
            ]
        },
        "specialist_policy": {
            "allow_specialist_goal_context": true,
            "allow_specialist_set_goal": false,
            "allow_specialist_clear_goal": false,
            "allow_specialist_override_goal": false,
            "max_subgoal_lines": goal_bridge_u64_from_config(
                config,
                "/goal_bridge/specialists/max_subgoal_lines",
                8
            ),
            "require_captain_acceptance": true
        },
        "public_api": {
            "public_command": false,
            "public_skill": false,
            "public_entrypoint": false,
            "set_goal_api_guaranteed": false
        }
    }))
}

fn create_prompt_refinement_request(enabled: bool, task_card_id: &str, timestamp: &str) -> Value {
    let state = if enabled { "planned" } else { "disabled" };

    json!({
        "schema": "ccc.prompt_refinement.v1",
        "state": state,
        "enabled": enabled,
        "execution_mode": "internal",
        "owner": "captain",
        "captain_gate": "accept_adjust_reject",
        "longway_materialization_allowed": false,
        "task_card_creation_allowed": false,
        "source": "ccc_promptsmith",
        "task_card_id": task_card_id,
        "created_at": timestamp,
        "consumed_at": Value::Null,
        "recorded_at": timestamp,
    })
}

fn create_prompt_refinement_handoff_decision(
    enabled: bool,
    task_card_id: &str,
    timestamp: &str,
) -> Option<Value> {
    if !enabled {
        return None;
    }

    Some(json!({
        "schema": "ccc.prompt_refinement_handoff_decision.v1",
        "state": "pending_captain_decision",
        "visibility": "internal",
        "execution_mode": "internal_non_executing",
        "owner": "captain",
        "source": "ccc_promptsmith",
        "task_card_id": task_card_id,
        "created_at": timestamp,
        "updated_at": timestamp,
        "handoff": {
            "from": "ghost",
            "to": "captain",
            "stage": "pre_materialization"
        },
        "captain_gate": {
            "state": "pending",
            "allowed_decisions": ["accept", "adjust", "reject"],
            "decision": Value::Null,
            "decided_at": Value::Null
        },
        "brief_contract": {
            "language": "en",
            "refined_brief_persisted": false,
            "status_surface_allowed": false
        },
        "dispatch_allowed": false,
        "longway_materialization_allowed": false,
        "task_card_creation_allowed": false
    }))
}

fn stage_for_sequence(sequence: &str) -> &'static str {
    if sequence == "PLAN_SEQUENCE" {
        "planning"
    } else {
        "execution"
    }
}

fn approval_state_for_sequence(sequence: &str) -> &'static str {
    if sequence == "PLAN_SEQUENCE" {
        "pending_longway_approval"
    } else {
        "approved_for_task_cards"
    }
}

fn next_action_for_sequence(sequence: &str) -> Value {
    if sequence == "PLAN_SEQUENCE" {
        json!({
            "command": "await_longway_approval",
            "reason": "PLAN_SEQUENCE is read-only and cannot dispatch executable task cards."
        })
    } else {
        json!({
            "command": "execute_task"
        })
    }
}

pub(crate) fn create_permission_fallback_run_directory_from_workspace(
    workspace_dir: &Path,
    run_id: &str,
) -> PathBuf {
    create_workspace_run_directory_from_workspace(workspace_dir, run_id)
}

fn create_initial_task_card_payload(
    parsed: &Value,
    run_id: &str,
    task_card_id: &str,
    timestamp: &str,
    runtime_pressure: Option<&RuntimeReviewPressureSnapshot>,
) -> Value {
    let task_kind = effective_task_kind_for_start(parsed);
    let fallback_role = assigned_role_for_task_kind(&task_kind);
    let request_text = combine_request_text_for_routing(parsed);
    let routing_trace = create_routing_trace_payload(&request_text, fallback_role);
    let sequence = sequence_for_start(parsed);
    let request_shape = routing_trace
        .get("request_shape")
        .and_then(Value::as_str)
        .unwrap_or_else(|| infer_request_shape(&request_text));
    let assigned_role = if sequence == "PLAN_SEQUENCE" {
        "way".to_string()
    } else if request_shape == "diagnostic" {
        "explorer".to_string()
    } else {
        routing_trace
            .get("selected_role")
            .and_then(Value::as_str)
            .unwrap_or(fallback_role)
            .to_string()
    };
    let title = parsed
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled task");
    let intent = parsed
        .get("intent")
        .and_then(Value::as_str)
        .unwrap_or("Create the initial bounded task card.");
    let scope = parsed
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("No explicit scope.");
    let prompt = parsed
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("Implement the bounded task.");
    let acceptance = parsed
        .get("acceptance")
        .and_then(Value::as_str)
        .unwrap_or("No explicit acceptance criteria.");
    let workflow_variant_selection = parsed
        .get("workflow_variant_selection")
        .filter(|value| value.is_object());
    let task_shape = infer_task_shape(&request_text, request_shape);
    let mut task_card = build_task_card_payload_with_role(
        run_id,
        task_card_id,
        title,
        intent,
        scope,
        prompt,
        acceptance,
        &assigned_role,
        timestamp,
    );
    if let Some(object) = task_card.as_object_mut() {
        object.insert("sequence".to_string(), Value::String(sequence.to_string()));
        object.insert(
            "routing_summary".to_string(),
            routing_trace.get("summary").cloned().unwrap_or(Value::Null),
        );
        object.insert("routing_trace".to_string(), routing_trace);
        object.insert(
            "review_policy".to_string(),
            review_policy_for_start_payload(parsed, timestamp, runtime_pressure),
        );
        if let Some(completion_discipline) = parsed
            .get("completion_discipline")
            .filter(|value| value.is_object())
        {
            object.insert(
                "completion_discipline".to_string(),
                completion_discipline.clone(),
            );
        }
        if let Some(parallel_fanout) = maybe_create_parallel_fanout_payload(
            &task_kind,
            &assigned_role,
            title,
            intent,
            scope,
            prompt,
            workflow_variant_selection,
            timestamp,
        ) {
            object.insert("parallel_fanout".to_string(), parallel_fanout);
        }
        if sequence == "PLAN_SEQUENCE" {
            let clarification_request =
                create_way_clarification_request(parsed, task_card_id, timestamp);
            object.insert(
                "status".to_string(),
                Value::String(
                    if clarification_request.is_some() {
                        "awaiting_way_clarification"
                    } else {
                        "pending_longway_approval"
                    }
                    .to_string(),
                ),
            );
            object.insert(
                "node_kind".to_string(),
                Value::String("planning".to_string()),
            );
            object.insert("dispatch_allowed".to_string(), Value::Bool(false));
            object.insert(
                "approval_state".to_string(),
                Value::String(
                    if clarification_request.is_some() {
                        "pending_way_clarification"
                    } else {
                        "pending_longway_approval"
                    }
                    .to_string(),
                ),
            );
            object.insert(
                "way_clarification_request".to_string(),
                clarification_request.unwrap_or(Value::Null),
            );
        }
    }
    apply_task_expertise_framing(&mut task_card, &assigned_role, task_shape);
    task_card
}

fn create_initial_run_payload(
    parsed: &Value,
    run_id: &str,
    task_card_id: &str,
    timestamp: &str,
    runtime_pressure: Option<&RuntimeReviewPressureSnapshot>,
    prompt_refinement_enabled: bool,
    goal_bridge_enabled: bool,
    config: Option<&Value>,
) -> Value {
    let request_text = combine_request_text_for_routing(parsed);
    let task_kind = effective_task_kind_for_start(parsed);
    let fallback_role = assigned_role_for_task_kind(&task_kind);
    let routing_trace = create_routing_trace_payload(&request_text, fallback_role);
    let sequence = sequence_for_start(parsed);
    let selected_role = if sequence == "PLAN_SEQUENCE" {
        "way"
    } else {
        routing_trace
            .get("selected_role")
            .and_then(Value::as_str)
            .unwrap_or(fallback_role)
    };
    let initial_role_config_snapshot = load_role_config_snapshot(selected_role);
    let initial_delegation_plan = create_specialist_delegation_plan(
        selected_role,
        &initial_role_config_snapshot,
        sandbox_mode_for_role(selected_role),
        sandbox_rationale_for_role(selected_role),
    );
    let request_shape = routing_trace
        .get("request_shape")
        .and_then(Value::as_str)
        .unwrap_or_else(|| infer_request_shape(&request_text));
    let mutation_intent = routing_trace
        .get("mutation_intent")
        .and_then(Value::as_str)
        .unwrap_or_else(|| infer_mutation_intent(request_shape));
    let task_shape = infer_task_shape(&request_text, request_shape);
    let review_policy = review_policy_for_start_payload(parsed, timestamp, runtime_pressure);
    let way_clarification_request =
        create_way_clarification_request(parsed, task_card_id, timestamp);
    let prompt_refinement =
        create_prompt_refinement_request(prompt_refinement_enabled, task_card_id, timestamp);
    let prompt_refinement_handoff_decision = create_prompt_refinement_handoff_decision(
        prompt_refinement_enabled,
        task_card_id,
        timestamp,
    );
    let goal_bridge =
        create_goal_bridge_record(goal_bridge_enabled, config, task_card_id, timestamp);
    let tool_route = routing_trace
        .get("tool_route")
        .cloned()
        .unwrap_or(Value::Null);
    let mut run_payload = json!({
        "run_id": run_id,
        "goal": parsed.get("goal").cloned().unwrap_or(Value::Null),
        "status": "active",
        "stage": stage_for_sequence(sequence),
        "sequence": sequence,
        "approval_state": if way_clarification_request.is_some() { "pending_way_clarification" } else { approval_state_for_sequence(sequence) },
        "active_role": "orchestrator",
        "active_agent_id": "captain",
        "active_task_card_id": task_card_id,
        "active_thread_id": Value::Null,
        "task_card_ids": [task_card_id],
        "latest_handoff_id": Value::Null,
        "child_agents": [],
        "specialist_executors": [],
        "latest_verified_checkpoint": Value::Null,
        "latest_verification": Value::Null,
        "latest_failure": Value::Null,
        "latest_orchestrator_synthesis": Value::Null,
        "latest_response": Value::Null,
        "routing_summary": routing_trace.get("summary").cloned().unwrap_or(Value::Null),
        "routing_trace": routing_trace.clone(),
        "review_policy": review_policy.clone(),
        "completion_discipline": parsed
            .get("completion_discipline")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or(Value::Null),
        "prompt_refinement": prompt_refinement,
        "latest_entry_trace": {
            "entrypoint": "ccc_start",
            "request_shape": request_shape,
            "mutation_intent": mutation_intent,
            "task_shape": task_shape,
            "review_policy": review_policy,
            "completion_discipline": parsed
                .get("completion_discipline")
                .filter(|value| value.is_object())
                .cloned()
                .unwrap_or(Value::Null),
            "companion_tool_route_class": tool_route.get("route_class").cloned().unwrap_or(Value::String("none".to_string())),
            "companion_tool_names": tool_route.get("tool_names").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "companion_tool_operation": tool_route.get("operation").cloned().unwrap_or(Value::String("none".to_string())),
            "companion_tool_owner_role": tool_route.get("owner_role").cloned().unwrap_or(Value::Null),
            "companion_tool_model": tool_route.get("model").cloned().unwrap_or(Value::Null),
            "companion_tool_variant": tool_route.get("variant").cloned().unwrap_or(Value::Null),
            "companion_tool_fallback_mode": tool_route.get("fallback_mode").cloned().unwrap_or(Value::String("visible_degraded_host_fallback".to_string())),
            "specialist_selected_role": routing_trace.get("selected_role").cloned().unwrap_or(Value::Null),
            "specialist_preferred_execution_mode": initial_delegation_plan.get("preferred_execution_mode").cloned().unwrap_or(Value::Null),
            "specialist_fallback_execution_mode": initial_delegation_plan.get("fallback_execution_mode").cloned().unwrap_or(Value::Null),
            "specialist_preferred_custom_agent_name": initial_delegation_plan.get("preferred_custom_agent_name").cloned().unwrap_or(Value::Null),
            "specialist_preferred_custom_agent_file": initial_delegation_plan.get("preferred_custom_agent_file").cloned().unwrap_or(Value::Null),
            "specialist_selected_category": routing_trace
                .get("specialist_route")
                .and_then(|value| value.get("selected_category"))
                .cloned()
                .unwrap_or(Value::Null),
            "recorded_at": timestamp,
        },
        "way_clarification_request": way_clarification_request,
        "raw_thread_ids": [],
        "created_at": timestamp,
        "updated_at": timestamp,
        "completed_at": Value::Null,
    });
    if let Some(handoff_decision) = prompt_refinement_handoff_decision {
        if let Some(object) = run_payload.as_object_mut() {
            object.insert(
                "prompt_refinement_handoff_decision".to_string(),
                handoff_decision,
            );
        }
    }
    if let Some(goal_bridge) = goal_bridge {
        if let Some(object) = run_payload.as_object_mut() {
            object.insert("goal_bridge".to_string(), goal_bridge);
        }
    }
    run_payload
}

fn create_initial_run_state_payload(
    run_id: &str,
    timestamp: &str,
    task_kind: &str,
    sequence: &str,
    way_clarification_request: Option<&Value>,
) -> Value {
    let phase_name = phase_name_for_task_kind(task_kind);
    let awaiting_way_clarification = way_clarification_request.is_some();
    json!({
        "version": 1,
        "run_id": run_id,
        "sequence": sequence,
        "approval_state": if awaiting_way_clarification { "pending_way_clarification" } else { approval_state_for_sequence(sequence) },
        "updated_at": timestamp,
        "event_count": 1,
        "last_event_id": "event-0001",
        "current_phase_id": "phase-0001",
        "current_phase_name": phase_name,
        "phases": [],
        "next_action": if awaiting_way_clarification {
            json!({
                "command": "await_operator",
                "reason": "Way clarification is required before pending LongWay materialization.",
                "clarification_request": way_clarification_request.cloned().unwrap_or(Value::Null),
            })
        } else {
            next_action_for_sequence(sequence)
        }
    })
}

fn first_non_empty_row_string(row: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        row.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn planned_row_value_is_missing(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "unassigned" | "none" | "unknown" | "tbd"
    )
}

fn normalize_planned_row(row: &Value) -> Option<Value> {
    let title = row
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| first_non_empty_row_string(row, &["title", "phase_name", "name"]))?;
    let mut planned_role = first_non_empty_row_string(row, &["planned_role", "role"])
        .unwrap_or_else(|| "unassigned".to_string());
    let mut planned_agent_id =
        first_non_empty_row_string(row, &["planned_agent_id", "agent_id", "owner_agent"])
            .unwrap_or_else(|| "unassigned".to_string());
    let scope = first_non_empty_row_string(row, &["scope"])
        .unwrap_or_else(|| "No explicit planned-row scope.".to_string());
    let acceptance = first_non_empty_row_string(row, &["acceptance"])
        .unwrap_or_else(|| "No explicit planned-row acceptance.".to_string());
    let mut inferred_role_from_text = false;
    if planned_row_value_is_missing(&planned_role) {
        let row_text = format!("{title}\n{scope}\n{acceptance}");
        if let Some(inferred_role) = planned_row_role_hint_from_text(&row_text) {
            planned_role = inferred_role.to_string();
            inferred_role_from_text = true;
        }
    }
    if planned_row_value_is_missing(&planned_agent_id) {
        if let Some(agent_id) = agent_id_for_role(&planned_role) {
            planned_agent_id = agent_id.to_string();
        }
    }
    let routing_summary = first_non_empty_row_string(row, &["routing_summary", "summary"]);
    let evidence_links = row
        .get("evidence_links")
        .and_then(Value::as_array)
        .map(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Value::String(value.to_string()))
                .collect::<Vec<_>>()
        })
        .filter(|links| !links.is_empty());

    let mut planned_row = serde_json::Map::new();
    planned_row.insert("title".to_string(), Value::String(title));
    planned_row.insert(
        "planned_role".to_string(),
        Value::String(planned_role.clone()),
    );
    planned_row.insert(
        "planned_agent_id".to_string(),
        Value::String(planned_agent_id),
    );
    if inferred_role_from_text {
        let snapshot = load_role_config_snapshot(&planned_role);
        if let Some(model) = snapshot.get("model").cloned() {
            planned_row.insert("model".to_string(), model);
        }
        if let Some(variant) = snapshot.get("variant").cloned() {
            planned_row.insert("variant".to_string(), variant.clone());
            planned_row.insert("reasoning".to_string(), variant);
        }
    }
    planned_row.insert("scope".to_string(), Value::String(scope));
    planned_row.insert("acceptance".to_string(), Value::String(acceptance));
    planned_row.insert("status".to_string(), Value::String("planned".to_string()));
    if let Some(value) = routing_summary {
        planned_row.insert("routing_summary".to_string(), Value::String(value));
    }
    if let Some(value) = evidence_links {
        planned_row.insert("evidence_links".to_string(), Value::Array(value));
    }
    Some(Value::Object(planned_row))
}

fn display_custom_agent_name(agent_id: &str) -> String {
    let trimmed = agent_id.trim();
    if trimmed.is_empty() || trimmed == "unassigned" || trimmed.starts_with("ccc_") {
        trimmed.to_string()
    } else {
        generated_custom_agent_name(trimmed)
    }
}

fn planned_row_text_has_any(text: &str, terms: &[&str]) -> bool {
    let normalized_text = normalized_ascii_search_text(text);
    terms.iter().any(|term| {
        if term.is_ascii() {
            let normalized_term = normalized_ascii_search_text(term);
            !normalized_term.trim().is_empty()
                && normalized_text.contains(&format!(" {} ", normalized_term.trim()))
        } else {
            text.contains(term)
        }
    })
}

fn normalized_ascii_search_text(text: &str) -> String {
    let normalized = text
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    format!(" {normalized} ")
}

fn planned_row_role_hint_from_text(text: &str) -> Option<&'static str> {
    let text = text.to_ascii_lowercase();
    if planned_row_text_has_any(
        &text,
        &[
            "git",
            "gh",
            "commit",
            "push",
            "git tag",
            "release tag",
            "tag release",
            "release upload",
            "release-upload",
            "release command",
            "operator command",
            "bounded tool",
            "\u{cee4}\u{bc0b}",
            "\u{d478}\u{c2dc}",
        ],
    ) {
        Some("companion_operator")
    } else if planned_row_text_has_any(&text, &["document", "docs", "readme", "release note"]) {
        Some("documenter")
    } else if planned_row_text_has_any(
        &text,
        &[
            "read-only",
            "read only",
            "readonly",
            "evidence",
            "scout",
            "inspect",
            "investigate",
            "collect",
            "search",
            "status",
            "check",
        ],
    ) {
        Some("explorer")
    } else if planned_row_text_has_any(&text, &["review", "verify", "validate", "acceptance"]) {
        Some("verifier")
    } else if planned_row_text_has_any(
        &text,
        &[
            "implement",
            "fix",
            "edit",
            "mutate",
            "repair",
            "change",
            "update",
        ],
    ) {
        Some("code specialist")
    } else {
        None
    }
}

fn enrich_planned_row_display_routing(row: &mut Value, parsed: &Value) {
    let Some(object) = row.as_object_mut() else {
        return;
    };
    let role_is_missing = object
        .get("planned_role")
        .and_then(Value::as_str)
        .map(|value| value.trim().is_empty() || value == "unassigned")
        .unwrap_or(true);
    let explicit_role = if role_is_missing {
        None
    } else {
        object
            .get("planned_role")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let agent_is_missing = object
        .get("planned_agent_id")
        .and_then(Value::as_str)
        .map(|value| value.trim().is_empty() || value == "unassigned")
        .unwrap_or(true);
    if !role_is_missing && !agent_is_missing {
        return;
    }

    let title = object
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let scope = object
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let acceptance = object
        .get("acceptance")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let row_text = format!("{title}\n{scope}\n{acceptance}");
    let request_text = format!("{}\n{}", row_text, combine_request_text_for_routing(parsed));
    let original_task_kind = parsed
        .get("task_kind")
        .and_then(Value::as_str)
        .unwrap_or("explore");
    let fallback_role = assigned_role_for_task_kind(original_task_kind);
    let routing_trace = create_routing_trace_payload(&request_text, fallback_role);
    let display_role = explicit_role
        .or_else(|| planned_row_role_hint_from_text(title))
        .unwrap_or_else(|| {
            routing_trace
                .get("selected_role")
                .and_then(Value::as_str)
                .unwrap_or(fallback_role)
        });
    let display_agent = agent_id_for_role(display_role)
        .map(display_custom_agent_name)
        .unwrap_or_else(|| "unassigned".to_string());
    let snapshot = load_role_config_snapshot(display_role);
    object.insert(
        "display_role".to_string(),
        Value::String(display_role.to_string()),
    );
    object.insert("display_agent_id".to_string(), Value::String(display_agent));
    if let Some(model) = snapshot.get("model").cloned() {
        object.insert("model".to_string(), model);
    }
    if let Some(variant) = snapshot.get("variant").cloned() {
        object.insert("variant".to_string(), variant.clone());
        object.insert("reasoning".to_string(), variant);
    }
}

fn normalize_planned_rows(parsed: &Value) -> Vec<Value> {
    parsed
        .get("planned_rows")
        .or_else(|| parsed.pointer("/longway/planned_rows"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let mut planned_row = normalize_planned_row(row)?;
                    enrich_planned_row_display_routing(&mut planned_row, parsed);
                    Some(planned_row)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn role_capability_summary(role: &str) -> Value {
    let snapshot = load_role_config_snapshot(role);
    json!({
        "role": role,
        "model": snapshot.get("model").cloned().unwrap_or(Value::Null),
        "variant": snapshot.get("variant").cloned().unwrap_or(Value::Null),
        "fast_mode": snapshot.get("fast_mode").cloned().unwrap_or(Value::Bool(false)),
        "sandbox_mode": sandbox_mode_for_role(role),
    })
}

fn create_way_structural_context() -> Value {
    let role_config = load_role_config_snapshot("way");
    let registry = load_skill_registry_for_agent("ccc_tactician", &role_config);
    let registry_status = registry
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let structural = registry
        .get("skill_ssl_manifest")
        .and_then(|manifest| manifest.get("structural"))
        .cloned()
        .unwrap_or(Value::Null);
    let scenes = structural
        .get("scenes")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));

    // Way scenes are planning hints only. They shape the pending LongWay, while
    // operator approval still controls whether executable task cards dispatch.
    json!({
        "schema": "ccc.way_structural_context.v1",
        "source": if registry_status == "available" && scenes.is_array() {
            "skill_registry"
        } else {
            "fallback_heuristic"
        },
        "agent_name": "ccc_tactician",
        "status": registry_status,
        "blocking": false,
        "advisory_only": true,
        "scenes": scenes,
    })
}

fn structural_scene_id(planning_context: &Value, index: usize, fallback: &str) -> String {
    planning_context
        .pointer("/way_structural_context/scenes")
        .and_then(Value::as_array)
        .and_then(|scenes| scenes.get(index))
        .and_then(|scene| scene.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn create_way_planning_context(
    workspace_dir: &Path,
    parsed: &Value,
    task_kind: &str,
    planned_row_count: usize,
) -> Value {
    // PLAN_SEQUENCE uses the target workspace, not just cwd, for graph/memory
    // evidence so parent-folder app sessions can still plan against the repo.
    let target_workspace = resolve_target_workspace_root(workspace_dir, parsed);
    let graph = create_code_graph_status_payload(&target_workspace.root);
    let memory = create_memory_status_payload(&target_workspace.root);
    let target_candidates = target_workspace
        .candidates
        .iter()
        .map(|path| Value::String(path.to_string_lossy().to_string()))
        .collect::<Vec<_>>();
    let request_text = combine_request_text_for_routing(parsed);
    let fallback_role = assigned_role_for_task_kind(task_kind);
    let routing_trace = create_routing_trace_payload(&request_text, fallback_role);
    let way_structural_context = create_way_structural_context();
    let selected_role = routing_trace
        .get("selected_role")
        .and_then(Value::as_str)
        .unwrap_or(fallback_role);
    let capability_roles = [
        "way",
        selected_role,
        "code specialist",
        "explorer",
        "documenter",
        "verifier",
        "companion_reader",
        "companion_operator",
    ];
    let mut capabilities = Vec::new();
    for role in capability_roles {
        if !capabilities
            .iter()
            .any(|item: &Value| item.get("role").and_then(Value::as_str) == Some(role))
        {
            capabilities.push(role_capability_summary(role));
        }
    }

    json!({
        "schema": "ccc.way_planning_context.v1",
        "source": "PLAN_SEQUENCE",
        "workspace_root": {
            "root": target_workspace.root.to_string_lossy(),
            "root_kind": target_workspace.root_kind.clone(),
            "confidence": target_workspace.confidence.clone(),
            "confirmation_required": target_workspace.confirmation_required,
            "reason": target_workspace.reason.clone(),
            "candidates": target_candidates,
        },
        "graph": {
            "available": graph.get("available").cloned().unwrap_or(Value::Bool(false)),
            "repo_root": graph.get("repo_root").cloned().unwrap_or(Value::Null),
            "store_path": graph.get("store_path").cloned().unwrap_or(Value::Null),
            "file_count": graph.get("file_count").cloned().unwrap_or(Value::from(0)),
            "evidence_note": graph.get("evidence_note").cloned().unwrap_or(Value::Null),
            "tolaria": graph.get("tolaria").cloned().unwrap_or(Value::Null),
        },
        "memory": {
            "available": memory.get("available").cloned().unwrap_or(Value::Bool(false)),
            "enabled": memory.get("enabled").cloned().unwrap_or(Value::Bool(false)),
            "workspace": memory.get("workspace").cloned().unwrap_or(Value::Null),
            "path": memory.get("path").cloned().unwrap_or(Value::Null),
            "entry_count": memory.get("entry_count").cloned().unwrap_or(Value::from(0)),
            "captain_instruction_count": memory.get("captain_instruction_count").cloned().unwrap_or(Value::from(0)),
            "captain_instruction_status": memory.get("captain_instruction_status").cloned().unwrap_or(Value::Null),
            "stale": memory.get("stale").cloned().unwrap_or(Value::Bool(false)),
            "tolaria": memory.get("tolaria").cloned().unwrap_or(Value::Null),
        },
        "capabilities": capabilities,
        "way_structural_context": way_structural_context,
        "planned_row_count": planned_row_count,
        "evidence_policy": {
            "bounded": true,
            "raw_graph_dump_stored": false,
            "memory_as_run_truth": false,
            "allowed_memory_truth_sources": memory.get("allowed_source_kinds").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        }
    })
}

fn planning_context_text(context: &Value, pointer: &str, fallback: &str) -> String {
    context
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn planning_context_bool(context: &Value, pointer: &str) -> bool {
    context
        .pointer(pointer)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn selected_role_from_planning_context(context: &Value) -> String {
    context
        .get("capabilities")
        .and_then(Value::as_array)
        .and_then(|capabilities| {
            capabilities
                .iter()
                .filter_map(|item| item.get("role").and_then(Value::as_str))
                .find(|role| !matches!(*role, "way" | "explorer" | "documenter" | "verifier"))
        })
        .unwrap_or("code specialist")
        .to_string()
}

fn push_string_if_non_empty(items: &mut Vec<Value>, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        items.push(Value::String(value.to_string()));
    }
}

fn create_graph_informed_planned_rows(
    parsed: &Value,
    planning_context: &Value,
    task_kind: &str,
) -> Vec<Value> {
    let title = parsed
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("approved LongWay task");
    let request_text = combine_request_text_for_routing(parsed);
    let fallback_role = assigned_role_for_task_kind(task_kind);
    let routing_trace = create_routing_trace_payload(&request_text, fallback_role);
    let selected_role = selected_role_from_planning_context(planning_context);
    let graph_available = planning_context_bool(planning_context, "/graph/available");
    let graph_state = if graph_available {
        "graph_available"
    } else {
        "graph_unavailable"
    };
    let memory_count = planning_context
        .pointer("/memory/entry_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let evidence_note = planning_context_text(
        planning_context,
        "/graph/evidence_note/text",
        "No graph evidence note available.",
    );
    let bounded_evidence_note = summarize_text_for_visibility(&evidence_note, 160);
    let routing_summary = summarize_text_for_visibility(
        &format!(
            "Way used bounded planning_context: {graph_state}; memory_entries={memory_count}; selected_role={selected_role}."
        ),
        240,
    );
    let inspect_scene_id = structural_scene_id(planning_context, 0, "frame_goal");
    let execute_scene_id = structural_scene_id(planning_context, 1, "sequence_rows");
    let approval_scene_id = structural_scene_id(planning_context, 2, "surface_approval");
    let mut evidence_links = Vec::new();
    push_string_if_non_empty(
        &mut evidence_links,
        planning_context
            .pointer("/graph/store_path")
            .and_then(Value::as_str),
    );

    let mut inspect_row = serde_json::Map::new();
    inspect_row.insert(
        "title".to_string(),
        Value::String(format!("Inspect graph-informed boundaries for {title}")),
    );
    inspect_row.insert(
        "planned_role".to_string(),
        Value::String("exploration_specialist".to_string()),
    );
    inspect_row.insert(
        "planned_agent_id".to_string(),
        Value::String("scout-a".to_string()),
    );
    inspect_row.insert(
        "scope".to_string(),
        Value::String(format!(
            "Use the bounded planning_context evidence note before mutation: {bounded_evidence_note}"
        )),
    );
    inspect_row.insert(
        "acceptance".to_string(),
        Value::String(
            "Return the concrete files, boundaries, and risks needed before executable work starts."
                .to_string(),
        ),
    );
    inspect_row.insert("status".to_string(), Value::String("planned".to_string()));
    inspect_row.insert(
        "routing_summary".to_string(),
        Value::String(routing_summary.clone()),
    );
    inspect_row.insert(
        "routing_trace".to_string(),
        json!({
            "query": "review_context",
            "terms": ["planning_context", "LongWay", "bounded graph evidence"],
            "reason": "PLAN_SEQUENCE generated this row from bounded graph, memory, and capability context.",
            "structural_scene_id": inspect_scene_id,
            "summary": routing_summary,
        }),
    );
    if !evidence_links.is_empty() {
        inspect_row.insert(
            "evidence_links".to_string(),
            Value::Array(evidence_links.clone()),
        );
    }

    let mut execute_row = serde_json::Map::new();
    execute_row.insert(
        "title".to_string(),
        Value::String(format!("Execute approved LongWay task for {title}")),
    );
    execute_row.insert(
        "planned_role".to_string(),
        Value::String(selected_role.clone()),
    );
    execute_row.insert(
        "planned_agent_id".to_string(),
        Value::String("unassigned".to_string()),
    );
    execute_row.insert(
        "scope".to_string(),
        Value::String(
            "Use the approved LongWay plus the preceding boundary findings; keep execution inside the accepted scope."
                .to_string(),
        ),
    );
    execute_row.insert(
        "acceptance".to_string(),
        Value::String(
            "Implementation, verification evidence, and LongWay checklist updates are ready for captain fan-in."
                .to_string(),
        ),
    );
    execute_row.insert("status".to_string(), Value::String("planned".to_string()));
    execute_row.insert(
        "routing_summary".to_string(),
        Value::String(summarize_text_for_visibility(
            &format!(
                "Way selected {selected_role} from bounded capability context after {graph_state}."
            ),
            240,
        )),
    );
    execute_row.insert(
        "routing_trace".to_string(),
        json!({
            "query": routing_trace.get("request_shape").cloned().unwrap_or_else(|| Value::String("implementation".to_string())),
            "terms": ["approved LongWay", "capability context", selected_role],
            "reason": "PLAN_SEQUENCE selected the executable lane from bounded role capability context.",
            "structural_scene_id": execute_scene_id,
            "approval_scene_id": approval_scene_id,
            "summary": "Execute only after explicit LongWay approval.",
        }),
    );
    if !evidence_links.is_empty() {
        execute_row.insert("evidence_links".to_string(), Value::Array(evidence_links));
    }

    vec![Value::Object(inspect_row), Value::Object(execute_row)]
}

fn set_planning_context_planned_row_count(context: &mut Value, planned_row_count: usize) {
    if let Some(object) = context.as_object_mut() {
        object.insert(
            "planned_row_count".to_string(),
            Value::from(planned_row_count),
        );
        object.insert(
            "decomposition_source".to_string(),
            Value::String("bounded_planning_context".to_string()),
        );
    }
}

fn create_initial_longway_payload(
    parsed: &Value,
    workspace_dir: &Path,
    task_card_id: &str,
    timestamp: &str,
    task_kind: &str,
    title: &str,
) -> Value {
    let sequence = sequence_for_start(parsed);
    let way_clarification_request =
        create_way_clarification_request(parsed, task_card_id, timestamp);
    let phase_name = phase_name_for_task_kind(task_kind);
    let lifecycle_state = if way_clarification_request.is_some() {
        "awaiting_clarification"
    } else if sequence == "PLAN_SEQUENCE" {
        "pending_approval"
    } else {
        "active"
    };
    let phase_status = if way_clarification_request.is_some() {
        "awaiting_way_clarification"
    } else if sequence == "PLAN_SEQUENCE" {
        "pending_longway_approval"
    } else {
        "pending"
    };
    let mut payload = json!({
        "lifecycle_state": lifecycle_state,
        "sequence": sequence,
        "approval_state": if way_clarification_request.is_some() { "pending_way_clarification" } else { approval_state_for_sequence(sequence) },
        "active_phase_name": phase_name,
        "active_phase_status": phase_status,
        "phases": [{
            "task_card_id": task_card_id,
            "phase_name": phase_name,
            "title": title,
            "status": phase_status
        }]
    });
    let mut planned_rows = normalize_planned_rows(parsed);
    let mut planning_context = if sequence == "PLAN_SEQUENCE" {
        Some(create_way_planning_context(
            workspace_dir,
            parsed,
            task_kind,
            planned_rows.len(),
        ))
    } else {
        None
    };
    if sequence == "PLAN_SEQUENCE" && planned_rows.is_empty() && way_clarification_request.is_none()
    {
        if let Some(context) = planning_context.as_ref() {
            planned_rows = create_graph_informed_planned_rows(parsed, context, task_kind);
        }
    }
    if let Some(context) = planning_context.as_mut() {
        set_planning_context_planned_row_count(context, planned_rows.len());
    }
    if !planned_rows.is_empty() {
        if let Some(object) = payload.as_object_mut() {
            object.insert("planned_rows".to_string(), Value::Array(planned_rows));
        }
    }
    if let Some(context) = planning_context {
        if let Some(object) = payload.as_object_mut() {
            object.insert("planning_context".to_string(), context);
        }
    }
    if let Some(clarification_request) = way_clarification_request {
        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "way_clarification_request".to_string(),
                clarification_request,
            );
        }
    }
    payload
}

fn create_initial_orchestrator_state_payload(
    run_id: &str,
    task_card_id: &str,
    sequence: &str,
    way_clarification_request: Option<&Value>,
) -> Value {
    let decision = if let Some(clarification_request) = way_clarification_request {
        json!({
            "next_step": "await_operator",
            "can_advance": false,
            "summary": "PLAN_SEQUENCE is waiting for Way clarification before pending LongWay materialization.",
            "clarification_request": clarification_request,
        })
    } else if sequence == "PLAN_SEQUENCE" {
        json!({
            "next_step": "await_longway_approval",
            "can_advance": false,
            "summary": "PLAN_SEQUENCE created a read-only pending LongWay approval state; executable dispatch is blocked until explicit approval."
        })
    } else {
        json!({
            "next_step": "execute_task",
            "can_advance": true,
            "summary": "CCC start created a new execution-ready run from operator-supplied scope."
        })
    };
    json!({
        "run_id": run_id,
        "task_card_id": task_card_id,
        "sequence": sequence,
        "approval_state": if way_clarification_request.is_some() { "pending_way_clarification" } else { approval_state_for_sequence(sequence) },
        "execution_request": Value::Null,
        "verification_request": Value::Null,
        "decision": decision,
        "orchestration_policy": {
            "autonomous_research": {
                "mode": "disabled"
            }
        }
    })
}

pub(crate) fn create_ccc_start_payload(parsed: &Value) -> io::Result<Value> {
    let shared_config = read_optional_shared_config_document()?.map(|(_, config)| config);
    create_ccc_start_payload_with_config(parsed, shared_config.as_ref())
}

pub(crate) fn create_ccc_start_payload_with_config(
    parsed: &Value,
    config: Option<&Value>,
) -> io::Result<Value> {
    let workspace_dir = resolve_workspace_path(parsed.get("cwd").and_then(Value::as_str))?;
    let prompt_refinement_enabled = prompt_refinement_enabled_from_config(config);
    let goal_bridge_enabled = goal_bridge_enabled_from_config(config);
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let prior_run_cleanup =
        reclaim_prior_active_runs_for_workspace(&workspace_dir, None, &timestamp)?;
    let mut active_run_scan = inspect_active_runs_for_workspace(&workspace_dir, None)?;
    if let Some(scan) = active_run_scan.as_object_mut() {
        scan.insert(
            "prior_run_cleanup_performed".to_string(),
            prior_run_cleanup
                .get("prior_run_cleanup_performed")
                .cloned()
                .unwrap_or(Value::Bool(false)),
        );
        scan.insert(
            "prior_run_cleanup_summary".to_string(),
            prior_run_cleanup
                .get("prior_run_cleanup_summary")
                .cloned()
                .unwrap_or(Value::Null),
        );
        scan.insert(
            "reclaimed_prior_run_count".to_string(),
            prior_run_cleanup
                .get("reclaimed_prior_run_count")
                .cloned()
                .unwrap_or(Value::Null),
        );
        scan.insert(
            "reclaimed_runs".to_string(),
            prior_run_cleanup
                .get("reclaimed_runs")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        );
    }
    let runtime_pressure = runtime_review_pressure_snapshot_from_start_scan(&active_run_scan);
    let worktree_mutation_baseline = create_worktree_mutation_baseline(&workspace_dir, &timestamp);
    let run_id = generate_uuid_like_id();
    let task_card_id = generate_uuid_like_id();
    let preferred_run_directory = create_run_directory_from_workspace(&workspace_dir, &run_id)?;

    let run_directory = match ensure_run_paths_for_start(&workspace_dir, &preferred_run_directory) {
        Ok(()) => preferred_run_directory,
        Err(error) if is_permission_error(&error) => {
            let workspace_run_directory =
                create_permission_fallback_run_directory_from_workspace(&workspace_dir, &run_id);
            ensure_run_paths_for_start(&workspace_dir, &workspace_run_directory)?;
            workspace_run_directory
        }
        Err(error) => return Err(error),
    };
    let _run_lock = acquire_run_mutation_lock(&run_directory, "ccc_start")?;
    let sequence = sequence_for_start(parsed);
    let task_kind = effective_task_kind_for_start(parsed);
    let mut run_payload = create_initial_run_payload(
        parsed,
        &run_id,
        &task_card_id,
        &timestamp,
        runtime_pressure.as_ref(),
        prompt_refinement_enabled,
        goal_bridge_enabled,
        config,
    );
    if let Some(object) = run_payload.as_object_mut() {
        object.insert(
            "worktree_mutation_baseline".to_string(),
            worktree_mutation_baseline,
        );
    }
    let task_card_payload = create_initial_task_card_payload(
        parsed,
        &run_id,
        &task_card_id,
        &timestamp,
        runtime_pressure.as_ref(),
    );
    write_json_document(&run_directory.join("run.json"), &run_payload)?;
    write_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
        &task_card_payload,
    )?;
    let way_clarification_request =
        create_way_clarification_request(parsed, &task_card_id, &timestamp);
    write_json_document(
        &run_directory.join("run-state.json"),
        &create_initial_run_state_payload(
            &run_id,
            &timestamp,
            &task_kind,
            sequence,
            way_clarification_request.as_ref(),
        ),
    )?;
    write_json_document(
        &run_directory.join("longway.json"),
        &create_initial_longway_payload(
            parsed,
            &workspace_dir,
            &task_card_id,
            &timestamp,
            &task_kind,
            task_card_payload
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled task"),
        ),
    )?;
    write_json_document(
        &run_directory.join("orchestrator-state.json"),
        &create_initial_orchestrator_state_payload(
            &run_id,
            &task_card_id,
            sequence,
            way_clarification_request.as_ref(),
        ),
    )?;
    fs::write(
        run_directory.join("events.jsonl"),
        b"{\"event\":\"run_started\"}\n",
    )?;

    Ok(json!({
        "cwd": workspace_dir.to_string_lossy(),
        "run_id": run_id,
        "task_card_id": task_card_id,
        "run_directory": run_directory.to_string_lossy(),
        "run_ref": create_ccc_run_ref(&run_directory),
        "status": "active",
        "stage": stage_for_sequence(sequence),
        "sequence": sequence,
        "approval_state": if way_clarification_request.is_some() { "pending_way_clarification" } else { approval_state_for_sequence(sequence) },
        "next_step": if way_clarification_request.is_some() { "await_operator" } else if sequence == "PLAN_SEQUENCE" { "await_longway_approval" } else { "execute_task" },
        "recommended_next_poll_ms": Value::Null,
        "routing_summary": run_payload.get("routing_summary").cloned().unwrap_or(Value::Null),
        "routing_trace": run_payload.get("routing_trace").cloned().unwrap_or(Value::Null),
        "prompt_refinement": run_payload.get("prompt_refinement").cloned().unwrap_or(Value::Null),
        "can_advance": sequence != "PLAN_SEQUENCE" && way_clarification_request.is_none(),
        "allowed_next_commands": if way_clarification_request.is_some() {
            json!(["answer_way_clarification"])
        } else if sequence == "PLAN_SEQUENCE" {
            json!(["approve_longway"])
        } else {
            json!(["advance"])
        },
        "way_clarification_request": way_clarification_request,
        "run_selection": if active_run_scan.get("reclaimed_prior_run_count").and_then(Value::as_u64).unwrap_or(0) > 0 {
            "new_run_created_after_prior_reclaim"
        } else if active_run_scan.get("fresh_active_run_count").and_then(Value::as_u64).unwrap_or(0) > 0 {
            "new_run_created_with_active_prior_run"
        } else {
            "new_run_created"
        },
        "active_run_scan": active_run_scan,
    }))
}

pub(crate) fn create_ccc_run_payload(parsed: &Value) -> io::Result<Value> {
    let start_payload = create_ccc_start_payload(parsed)?;
    let run_directory = PathBuf::from(
        start_payload
            .get("run_directory")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ccc_run start payload is missing run_directory.",
                )
            })?,
    );
    let run_id = start_payload
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ccc_run start payload is missing run_id.",
            )
        })?;
    if start_payload.get("sequence").and_then(Value::as_str) == Some("PLAN_SEQUENCE") {
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        append_run_event(
            &run_directory,
            json!({
                "event": "run_plan_sequence_checkpointed",
                "entrypoint": "ccc_run",
                "run_id": run_id,
                "task_card_id": start_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
                "timestamp": timestamp,
            }),
        )?;
        return Ok(json!({
            "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
            "run_id": run_id,
            "task_card_id": start_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
            "run_directory": start_payload.get("run_directory").cloned().unwrap_or(Value::Null),
            "run_ref": start_payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "entrypoint": "ccc_run",
            "status": "active",
            "stage": "planning",
            "sequence": "PLAN_SEQUENCE",
            "approval_state": "pending_longway_approval",
            "next_step": "await_longway_approval",
            "recommended_next_poll_ms": Value::Null,
            "can_advance": false,
            "advanced": false,
            "routing_summary": start_payload.get("routing_summary").cloned().unwrap_or(Value::Null),
            "routing_trace": start_payload.get("routing_trace").cloned().unwrap_or(Value::Null),
            "allowed_next_commands": ["approve_longway"],
            "summary": "PLAN_SEQUENCE ccc_run persisted a pending LongWay approval state without executable dispatch.",
        }));
    }
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String(
            "Rust ccc_run created the run and persisted the initial execute_task checkpoint; Codex dispatch is still pending the Rust ccc_orchestrate port."
                .to_string(),
        ),
    );
    run_object.insert("latest_entry_trace".to_string(), {
        let mut latest_entry_trace = run_object
            .get("latest_entry_trace")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Some(object) = latest_entry_trace.as_object_mut() {
            object.insert(
                "entrypoint".to_string(),
                Value::String("ccc_run".to_string()),
            );
            object.insert(
                "codex_bin".to_string(),
                parsed.get("codex_bin").cloned().unwrap_or(Value::Null),
            );
            object.insert(
                "workflow_variant_selection".to_string(),
                parsed
                    .get("workflow_variant_selection")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            object.insert("completed_at".to_string(), Value::String(timestamp.clone()));
        }
        latest_entry_trace
    });
    run_object.insert("updated_at".to_string(), Value::String(timestamp.clone()));
    write_json_document(&run_file, &run_record)?;

    let orchestrator_state_file = run_directory.join("orchestrator-state.json");
    let mut orchestrator_state = read_json_document(&orchestrator_state_file)?;
    let orchestrator_object = orchestrator_state.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "orchestrator-state.json must be an object.",
        )
    })?;
    orchestrator_object.insert(
        "execution_request".to_string(),
        json!({
            "entrypoint": "ccc_run",
            "codex_bin": parsed.get("codex_bin").cloned().unwrap_or(Value::Null),
            "requested_at": timestamp,
            "workflow_variant_selection": parsed.get("workflow_variant_selection").cloned().unwrap_or(Value::Null),
        }),
    );
    orchestrator_object.insert(
        "decision".to_string(),
        json!({
            "next_step": "execute_task",
            "can_advance": true,
            "summary": "Rust ccc_run persisted the initial execution checkpoint and will immediately hand the run to Rust ccc_orchestrate."
        }),
    );
    write_json_document(&orchestrator_state_file, &orchestrator_state)?;

    append_run_event(
        &run_directory,
        json!({
            "event": "run_checkpointed",
            "entrypoint": "ccc_run",
            "run_id": run_id,
            "task_card_id": start_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
            "timestamp": timestamp,
        }),
    )?;

    let orchestrate_payload = create_ccc_orchestrate_payload(&json!({
        "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
        "run_id": run_id,
        "codex_bin": parsed.get("codex_bin").cloned().unwrap_or(Value::Null),
        "progression_mode": "single_step",
    }))?;

    let mut response = orchestrate_payload.as_object().cloned().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "ccc_orchestrate payload must be an object.",
        )
    })?;
    response.insert(
        "entrypoint".to_string(),
        Value::String("ccc_run".to_string()),
    );
    response.insert(
        "task_card_id".to_string(),
        start_payload
            .get("task_card_id")
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok(Value::Object(response))
}
