use crate::parallel_fanout::compact_fan_in_fields;
use crate::preferred_subagent_child_agent_id;
use crate::status_app_panel::create_codex_app_panel_payload;
use crate::status_payload::sanitize_scheduler_selected_planned_row;
use crate::text_utils::summarize_text_for_visibility;
use serde_json::{json, Value};

fn compact_routing_trace(trace: &Value) -> Value {
    let Some(object) = trace.as_object() else {
        return Value::Null;
    };
    let mut compact = serde_json::Map::new();
    for (output_key, input_keys, max_chars) in [
        ("source", &["source"][..], 80),
        ("selected_category", &["selected_category"][..], 80),
        ("selected_skill_id", &["selected_skill_id"][..], 120),
        ("selected_skill_name", &["selected_skill_name"][..], 120),
        ("risk", &["risk"][..], 40),
        ("mutation_intent", &["mutation_intent"][..], 80),
        ("evidence_need", &["evidence_need"][..], 120),
        ("verification_need", &["verification_need"][..], 120),
        ("selected_role", &["selected_role"][..], 80),
        ("selected_agent_id", &["selected_agent_id"][..], 80),
        ("reason", &["reason", "rationale"][..], 240),
        ("summary", &["summary"][..], 240),
    ] {
        if let Some(value) = input_keys.iter().find_map(|key| {
            object
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        }) {
            compact.insert(
                output_key.to_string(),
                Value::String(summarize_text_for_visibility(value, max_chars)),
            );
        }
    }
    (!compact.is_empty())
        .then_some(Value::Object(compact))
        .unwrap_or(Value::Null)
}

fn compact_scheduler_payload(scheduler: &Value) -> Value {
    let Some(object) = scheduler.as_object() else {
        return scheduler.clone();
    };
    let mut compact = object.clone();
    if let Some(row) = object.get("selected_planned_row") {
        compact.insert(
            "selected_planned_row".to_string(),
            sanitize_scheduler_selected_planned_row(row),
        );
    }
    if let Some(latest_transition) = object.get("latest_transition").and_then(Value::as_object) {
        let mut transition = latest_transition.clone();
        if let Some(row) = latest_transition.get("selected_planned_row") {
            transition.insert(
                "selected_planned_row".to_string(),
                sanitize_scheduler_selected_planned_row(row),
            );
        }
        compact.insert("latest_transition".to_string(), Value::Object(transition));
    }
    Value::Object(compact)
}

fn compact_graph_context_readiness(payload: &Value) -> Value {
    let Some(readiness) = payload
        .get("graph_context")
        .filter(|value| value.is_object())
    else {
        return Value::Null;
    };
    json!({
        "schema": readiness.get("schema").cloned().unwrap_or(Value::String("ccc.graph_context_readiness.status.v1".to_string())),
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
        "routing": readiness.get("routing").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn create_ccc_status_compact_payload(payload: &Value) -> Value {
    let current_task = payload.get("current_task_card").unwrap_or(&Value::Null);
    let delegation_plan = current_task.get("delegation_plan").unwrap_or(&Value::Null);
    let run_id = payload.get("run_id").cloned().unwrap_or(Value::Null);
    let run_id_text = run_id.as_str().unwrap_or("<run_id>");
    let operator_transport_contract = json!({
        "preferred": "ccc_cli_quiet_subcommand",
        "display_expectation": "ran",
        "avoid_for_lifecycle_mutations": "mcp_tool_call",
        "reason": "Use inline --json with compact CLI output for repeated CCC lifecycle updates; reserve MCP calls for app/structured inspection or when CLI is unavailable.",
        "default_payload_transport": "inline_json",
        "longway_visibility": "Use CCC_LONGWAY_PROJECTION.md for normal progress visibility; refresh it with ccc status --projection --json '{...}' or ccc checklist --projection --json '{...}'.",
        "commands": [
            "ccc start --quiet --json '{...}'",
            "ccc orchestrate --quiet --json '{...}'",
            "ccc subagent-update --quiet --json '{...}'",
            "ccc memory --quiet --json '{...}'"
        ]
    });
    let task_card_id = current_task
        .get("task_card_id")
        .cloned()
        .or_else(|| payload.get("active_task_card_id").cloned())
        .unwrap_or(Value::Null);
    let child_agent_id = current_task
        .pointer("/subagent_lifecycle/child_agent_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            current_task
                .pointer("/review_lifecycle/child_agent_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| preferred_subagent_child_agent_id(current_task))
        .map(Value::String)
        .unwrap_or(Value::Null);
    let compact_subagent_fan_in = json!({
        "schema": current_task
            .pointer("/subagent_fan_in/schema")
            .cloned()
            .unwrap_or(Value::String("ccc.worker_result_envelope.v1".to_string())),
        "summary": current_task
            .pointer("/subagent_fan_in/summary")
            .cloned()
            .unwrap_or(Value::Null),
        "status": current_task
            .pointer("/subagent_fan_in/status")
            .cloned()
            .unwrap_or(Value::Null),
        "evidence_paths": current_task
            .pointer("/subagent_fan_in/evidence_paths")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "next_action": current_task
            .pointer("/subagent_fan_in/next_action")
            .cloned()
            .unwrap_or(Value::Null),
        "open_questions": current_task
            .pointer("/subagent_fan_in/open_questions")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "confidence": current_task
            .pointer("/subagent_fan_in/confidence")
            .cloned()
            .unwrap_or(Value::Null),
        "risk": current_task
            .pointer("/subagent_fan_in/risk")
            .cloned()
            .unwrap_or(Value::Null),
        "checks": current_task
            .pointer("/subagent_fan_in/checks")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "contract": current_task
            .pointer("/subagent_fan_in/contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "captain_consumes_compact_fan_in": true,
            })),
        "artifact_ref": current_task
            .pointer("/subagent_fan_in/artifact_ref")
            .cloned()
            .or_else(|| current_task.get("subagent_fan_in_artifact").cloned())
            .unwrap_or(Value::Null),
    });
    let parallel_required_lane_ids = current_task
        .pointer("/parallel_fanout/required_lane_ids")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let parallel_launch_mode = current_task
        .pointer("/parallel_fanout/mode")
        .and_then(Value::as_str)
        .map(|value| value == "parallel")
        .unwrap_or(false);
    let launch_lane_ids = if parallel_launch_mode {
        parallel_required_lane_ids.clone()
    } else {
        Value::Array(Vec::new())
    };
    let launch_lane = if parallel_launch_mode {
        parallel_required_lane_ids
            .as_array()
            .and_then(|lane_ids| lane_ids.first())
            .cloned()
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let launch_expected_fan_in = delegation_plan
        .pointer("/fan_in_contract/required_fields")
        .cloned()
        .unwrap_or_else(|| {
            json!([
                "summary",
                "status",
                "evidence_paths",
                "next_action",
                "open_questions",
                "confidence"
            ])
        });
    let spec_surfaces = delegation_plan
        .pointer("/spec_surfaces")
        .cloned()
        .unwrap_or(Value::Null);
    let verify_retry_recap_report_contract = delegation_plan
        .pointer("/verify_retry_recap_report_contract")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "verify": {
                    "field": "verification_state",
                    "states": [
                        "pending",
                        "passed",
                        "needs_work",
                        "blocked"
                    ]
                },
                "retry": {
                    "field": "captain_follow_up",
                    "budget_key": "retry",
                    "states": [
                        "queued",
                        "consumed"
                    ]
                },
                "recap": {
                    "field": "lane_artifact_contract.recap",
                    "source": "parallel_fanout.lanes[].fan_in.summary"
                },
                "report": {
                    "field": "latest_delegate_result.result_summary",
                    "fallback_field": "latest_delegate_result.assistant_message_preview"
                }
            })
        });
    let compact_parallel_fanout = if current_task
        .get("parallel_fanout")
        .and_then(Value::as_object)
        .is_some()
    {
        let compact_lanes = current_task
            .pointer("/parallel_fanout/lanes")
            .and_then(Value::as_array)
            .map(|lanes| {
                lanes
                    .iter()
                    .map(|lane| {
                        json!({
                            "lane_id": lane.get("lane_id").cloned().unwrap_or(Value::Null),
                            "required": lane.get("required").cloned().unwrap_or(Value::Bool(false)),
                            "scope": lane.get("scope").cloned().unwrap_or(Value::Null),
                            "lifecycle": {
                                "status": lane.pointer("/lifecycle/status").cloned().unwrap_or(Value::Null),
                                "child_agent_id": lane.pointer("/lifecycle/child_agent_id").cloned().unwrap_or(Value::Null),
                                "thread_id": lane.pointer("/lifecycle/thread_id").cloned().unwrap_or(Value::Null),
                                "summary": lane.pointer("/lifecycle/summary").cloned().unwrap_or(Value::Null),
                                "updated_at": lane.pointer("/lifecycle/updated_at").cloned().unwrap_or(Value::Null),
                            },
                            "fan_in": compact_fan_in_fields(lane.get("fan_in").unwrap_or(&Value::Null)),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json!({
            "mode": current_task.pointer("/parallel_fanout/mode").cloned().unwrap_or(Value::Null),
            "summary": current_task.pointer("/parallel_fanout/summary").cloned().unwrap_or(Value::Null),
            "required_lane_ids": parallel_required_lane_ids.clone(),
            "aggregate": current_task.pointer("/parallel_fanout/aggregate").cloned().unwrap_or(Value::Null),
            "lanes": compact_lanes,
        })
    } else {
        Value::Null
    };
    let default_lane_id = parallel_required_lane_ids
        .as_array()
        .and_then(|lane_ids| lane_ids.first())
        .cloned()
        .unwrap_or(Value::Null);

    let app_panel = payload
        .get("app_panel")
        .cloned()
        .unwrap_or_else(|| create_codex_app_panel_payload(payload));
    let captain_action_contract = compact_captain_action_contract(payload);
    let command_templates = create_status_command_templates_payload(
        payload,
        current_task,
        operator_transport_contract.clone(),
        run_id_text,
        &child_agent_id,
        default_lane_id,
        parallel_required_lane_ids,
    );

    json!({
        "compact": true,
        "run_id": run_id,
        "run_ref": payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "status": payload.get("status").cloned().unwrap_or(Value::Null),
        "stage": payload.get("stage").cloned().unwrap_or(Value::Null),
        "sequence": payload.get("sequence").cloned().unwrap_or(Value::Null),
        "approval_state": payload.get("approval_state").cloned().unwrap_or(Value::Null),
        "next_step": payload.get("next_step").cloned().unwrap_or(Value::Null),
        "can_advance": payload.get("can_advance").cloned().unwrap_or(Value::Null),
        "run_truth_surface": payload.get("run_truth_surface").cloned().unwrap_or(Value::Null),
        "active_checkpoint": payload.get("active_checkpoint").cloned().unwrap_or(Value::Null),
        "task_session_state": payload.get("task_session_state").cloned().unwrap_or(Value::Null),
        "workflow_loop": payload.get("workflow_loop").cloned().unwrap_or(Value::Null),
        "lifecycle_hooks": payload.get("lifecycle_hooks").cloned().unwrap_or(Value::Null),
        "scheduler": payload.get("scheduler").map(compact_scheduler_payload).unwrap_or(Value::Null),
        "review_policy": payload.get("review_policy").cloned().unwrap_or(Value::Null),
        "completion_discipline": payload.get("completion_discipline").cloned().unwrap_or(Value::Null),
        "longway": {
            "completed_phase_count": payload.pointer("/longway/completed_phase_count").cloned().unwrap_or(Value::Null),
            "phase_count": payload.pointer("/longway/phase_count").cloned().unwrap_or(Value::Null),
            "planned_row_count": payload.pointer("/longway/planned_row_count").cloned().unwrap_or(Value::Null),
            "current_item": payload.pointer("/longway/current_item").cloned().unwrap_or(Value::Null),
            "lifecycle_state": payload.pointer("/longway/lifecycle_state").cloned().unwrap_or(Value::Null),
            "sequence": payload.pointer("/longway/sequence").cloned().unwrap_or(Value::Null),
            "approval_state": payload.pointer("/longway/approval_state").cloned().unwrap_or(Value::Null),
            "planning_context": payload.pointer("/longway/planning_context").cloned().unwrap_or(Value::Null),
            "way_clarification_request": payload.pointer("/longway/way_clarification_request").cloned().unwrap_or(Value::Null),
            "phase_rows": payload.pointer("/longway/phase_rows").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "planned_rows": payload.pointer("/longway/planned_rows").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        },
        "graph_context": compact_graph_context_readiness(payload),
        "code_graph": payload.get("code_graph").cloned().unwrap_or(Value::Null),
        "memory": payload.get("memory").cloned().unwrap_or(Value::Null),
        "registry_evidence": payload.get("registry_evidence").cloned().unwrap_or(Value::Null),
        "current_task_card": {
            "task_card_id": task_card_id,
            "title": current_task.get("title").cloned().unwrap_or(Value::Null),
            "task_kind": current_task.get("task_kind").cloned().unwrap_or(Value::Null),
            "scope": current_task.get("scope").cloned().unwrap_or(Value::Null),
            "review_of_task_card_ids": current_task.get("review_of_task_card_ids").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "orchestrator_review_gate": current_task.get("orchestrator_review_gate").cloned().unwrap_or(Value::Null),
            "verification_state": current_task.get("verification_state").cloned().unwrap_or(Value::Null),
            "review_pass_count": current_task.get("review_pass_count").cloned().unwrap_or(Value::Null),
            "review_policy": current_task.get("review_policy").cloned().unwrap_or(Value::Null),
            "completion_discipline": current_task.get("completion_discipline").cloned().unwrap_or(Value::Null),
            "captain_intervention": current_task.get("captain_intervention").cloned().unwrap_or(Value::Null),
            "sentinel_intervention": current_task.get("sentinel_intervention").cloned().unwrap_or(Value::Null),
            "assigned_role": current_task.get("assigned_role").cloned().unwrap_or(Value::Null),
            "assigned_agent_id": current_task.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
            "verification_capsule": current_task.get("verification_capsule").cloned().unwrap_or(Value::Null),
            "delegated_ownership": current_task.get("delegated_ownership").cloned().unwrap_or(Value::Null),
            "expertise_framing": current_task
                .get("expertise_framing")
                .cloned()
                .or_else(|| delegation_plan.get("expertise_framing").cloned())
                .unwrap_or(Value::Null),
            "launch_visibility": {
                "lane": launch_lane,
                "lane_ids": launch_lane_ids,
                "expected_fan_in": launch_expected_fan_in,
            },
            "spec_surfaces": spec_surfaces,
            "verify_retry_recap_report_contract": verify_retry_recap_report_contract,
            "runtime_dispatch": delegation_plan
                .pointer("/runtime_dispatch")
                .cloned()
                .unwrap_or(Value::Null),
            "lane_artifact_contract": delegation_plan
                .pointer("/lane_artifact_contract")
                .cloned()
                .unwrap_or(Value::Null),
            "execution_prompt": current_task.get("execution_prompt").cloned().unwrap_or(Value::Null),
            "subagent_lifecycle": current_task.get("subagent_lifecycle").cloned().unwrap_or(Value::Null),
            "review_lifecycle": current_task.get("review_lifecycle").cloned().unwrap_or(Value::Null),
            "subagent_fan_in": compact_subagent_fan_in,
            "worker_result_envelope": current_task.get("worker_result_envelope").cloned().unwrap_or_else(|| current_task.get("subagent_fan_in").cloned().unwrap_or(Value::Null)),
            "subagent_fan_in_artifact": current_task.get("subagent_fan_in_artifact").cloned().unwrap_or(Value::Null),
            "late_subagent_output": current_task.get("late_subagent_output").cloned().unwrap_or(Value::Null),
            "review_fan_in": current_task.get("review_fan_in").cloned().unwrap_or(Value::Null),
            "parallel_fanout": compact_parallel_fanout,
            "subagent_fallback": current_task.get("subagent_fallback").cloned().unwrap_or(Value::Null),
            "subagent_policy_drift": current_task.get("subagent_policy_drift").cloned().unwrap_or(Value::Null),
            "route_enforcement_state": current_task.get("route_enforcement_state").cloned().unwrap_or(Value::Null),
            "routing_trace": current_task.get("routing_trace").map(compact_routing_trace).unwrap_or(Value::Null),
            "subagent_contract": {
                "mode": delegation_plan.get("preferred_execution_mode").cloned().unwrap_or(Value::Null),
                "agent": delegation_plan.get("preferred_custom_agent_name").cloned().unwrap_or(Value::Null),
                "file": delegation_plan.get("preferred_custom_agent_file").cloned().unwrap_or(Value::Null),
                "model": delegation_plan.get("model").cloned().unwrap_or(Value::Null),
                "effort": delegation_plan.get("variant").cloned().unwrap_or(Value::Null),
                "sandbox": delegation_plan.get("sandbox_mode").cloned().unwrap_or(Value::Null),
                "expertise_phrase": delegation_plan.pointer("/expertise_framing/expertise_phrase").cloned().unwrap_or(Value::Null),
                "task_stance": delegation_plan.pointer("/expertise_framing/task_stance").cloned().unwrap_or(Value::Null),
                "expected_thinking_mode": delegation_plan.pointer("/expertise_framing/expected_thinking_mode").cloned().unwrap_or(Value::Null),
                "fresh_context": delegation_plan.pointer("/subagent_spawn_contract/prefer_fresh_child_context").cloned().unwrap_or(Value::Bool(true)),
                "omit_overrides": true,
                "update_via": delegation_plan.pointer("/subagent_update_contract/transport").cloned().unwrap_or(Value::String("ccc_cli_subcommand".to_string())),
                "fan_in_fields": delegation_plan.pointer("/fan_in_contract/required_fields").cloned().unwrap_or_else(|| json!(["summary", "status", "evidence_paths", "next_action", "open_questions", "confidence"])),
            },
        },
        "execution_strategy": {
            "preferred_specialist_execution_mode": payload.pointer("/execution_strategy/preferred_specialist_execution_mode").cloned().unwrap_or(Value::Null),
            "fallback_specialist_execution_mode": payload.pointer("/execution_strategy/fallback_specialist_execution_mode").cloned().unwrap_or(Value::Null),
            "host_subagent_update_mode": payload.pointer("/execution_strategy/host_subagent_update_mode").cloned().unwrap_or(Value::Null),
            "operator_visible_transport": payload.pointer("/execution_strategy/operator_visible_transport").cloned().unwrap_or_else(|| json!({
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
                "mcp_reserved_for": ["app surfaces", "structured inspection", "CLI unavailable"]
            })),
            "codex_exec_fallback_allowed": payload.pointer("/execution_strategy/codex_exec_fallback_allowed").cloned().unwrap_or(Value::Null),
            "subagent_fallback_recorded": payload.pointer("/execution_strategy/subagent_fallback_recorded").cloned().unwrap_or(Value::Null),
            "subagent_fallback_ready": payload.pointer("/execution_strategy/subagent_fallback_ready").cloned().unwrap_or(Value::Null),
        },
        "cost_routing": payload.get("cost_routing").cloned().unwrap_or(Value::Null),
        "captain_action_contract": captain_action_contract,
        "captain_direct_mutation_guard": payload.get("captain_direct_mutation_guard").cloned().unwrap_or(Value::Null),
        "operator_transport_contract": operator_transport_contract,
        "host_subagent_state": payload.get("host_subagent_state").cloned().unwrap_or(Value::Null),
        "recovery_lane": payload.get("recovery_lane").cloned().unwrap_or(Value::Null),
        "latest_captain_intervention": payload.get("latest_captain_intervention").cloned().unwrap_or(Value::Null),
        "latest_sentinel_intervention": payload.get("latest_sentinel_intervention").cloned().unwrap_or(Value::Null),
        "pending_captain_follow_up": payload.get("pending_captain_follow_up").cloned().unwrap_or(Value::Null),
        "latest_delegate_result": payload.get("latest_delegate_result").cloned().unwrap_or(Value::Null),
        "token_usage": payload.get("token_usage").cloned().unwrap_or(Value::Null),
        "token_usage_visibility": payload.get("token_usage_visibility").cloned().unwrap_or(Value::Null),
        "long_session_mitigation": payload.get("long_session_mitigation").cloned().unwrap_or(Value::Null),
        "context_health": payload.get("context_health").cloned().unwrap_or(Value::Null),
        "restart_handoff": payload.get("restart_handoff").cloned().unwrap_or(Value::Null),
        "app_panel": app_panel,
        "token_usage_status": payload.pointer("/token_usage_visibility/status").cloned().unwrap_or(Value::Null),
        "token_usage_unavailable_reason": payload.pointer("/token_usage_visibility/unavailable_reason").cloned().unwrap_or(Value::Null),
        "token_usage_unavailable_reason_code": payload.pointer("/token_usage_visibility/unavailable_reason_code").cloned().unwrap_or(Value::Null),
        "visibility_signature": payload.get("visibility_signature").cloned().unwrap_or(Value::Null),
        "command_templates": command_templates
    })
}

fn create_status_command_templates_payload(
    payload: &Value,
    current_task: &Value,
    operator_transport_contract: Value,
    run_id_text: &str,
    child_agent_id: &Value,
    default_lane_id: Value,
    parallel_required_lane_ids: Value,
) -> Value {
    json!({
        "operator_transport": operator_transport_contract,
        "status": {
            "command": format!("ccc status --quiet --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "text_command": format!("ccc status --text --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "projection_command": format!("ccc status --projection --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "longway_text_command": format!("ccc status --projection --json '{{\"run_id\":\"{}\"}}'", run_id_text)
        },
        "graph": {
            "command": "ccc graph --json '{\"paths\":[\"<path>\"],\"query\":\"review_context\",\"update\":false}'",
            "text_command": "ccc graph --text --json '{\"paths\":[\"<path>\"],\"query\":\"impact\",\"update\":false}'",
            "payload": {
                "cwd": payload.get("cwd").cloned().unwrap_or(Value::Null),
                "paths": ["<changed-path>"],
                "query": "review_context",
                "update": false
            }
        },
        "memory": {
            "command": "ccc memory --text --json '{\"action\":\"status\"}'",
            "preview_command": "ccc memory --text --json '{...}'",
            "write_command": "ccc memory --quiet --json '{...}'",
            "payload": {
                "action": "preview",
                "entries": [{
                    "kind": "captain_instruction",
                    "text": "<recurring captain instruction>",
                    "source_kind": "operator_confirmation",
                    "source": "operator"
                }]
            }
        },
        "checklist": {
            "command": format!("ccc checklist --quiet --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "text_command": format!("ccc checklist --text --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "projection_command": format!("ccc checklist --projection --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "longway_text_command": format!("ccc checklist --projection --json '{{\"run_id\":\"{}\"}}'", run_id_text),
            "payload": {
                "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null)
            }
        },
        "start": {
            "command": "ccc start --quiet --json '{...}'",
            "text_command": "ccc start --text --json '{...}'",
            "payload": {
                "goal": "<short goal>",
                "title": "<bounded title>",
                "intent": "<operator intent>",
                "scope": "<bounded scope>",
                "acceptance": "<done when>",
                "prompt": "<request>",
                "task_kind": "<execution|explore|review|way>",
                "compact": true
            }
        },
        "subagent_update": {
            "command": "ccc subagent-update --quiet --json '{...}'",
            "text_command": "ccc subagent-update --text --json '{...}'",
            "payload": {
                "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
                "task_card_id": current_task.get("task_card_id").cloned().unwrap_or(Value::Null),
                "child_agent_id": child_agent_id.clone(),
                "lane_id": default_lane_id,
                "lane_ids": parallel_required_lane_ids,
                "compact": true,
                "mode": "compact",
                "event_ref": "<optional-stable-event-ref>",
                "status": "<spawned|acknowledged|running|stalled|completed|failed|merged|reclaimed>",
                "review_outcome": "<passed|needs_work|unsatisfactory|blocked|stalled|reclaimed>",
                "summary": "<short>",
                "fan_in_fields_on_completed": ["summary", "status", "evidence_paths", "next_action", "open_questions", "confidence"],
                "intervention_fields_when_unsatisfactory": {
                    "intervention_classification": "<clarification_only|bounded_scope_amendment|direction_or_risk_correction>",
                    "intervention_rationale": "<why the captain is intervening>",
                    "chosen_next_action": "<amend_same_worker|reclaim|reassign|close|clarify|no_action>",
                    "budget_snapshot": {
                        "retry": { "limit": 1, "used": 0, "remaining": 1 },
                        "reassign": { "limit": 1, "used": 0, "remaining": 1 }
                    },
                    "stale_output_policy": "<preserve_visible|merge_explicit_only>",
                    "stale_output_summary": "<summary of late/stale output, if relevant>"
                },
                "lane_payload_template": {
                    "lane_id": "<raider-a|raider-b|raider-c|raider-d|scout-a|scout-b|scout-c|scout-d>",
                    "summary": "<short>",
                    "status": "<completed|failed|stalled|reclaimed|merged>",
                    "review_outcome": "<passed|needs_work|unsatisfactory|blocked|stalled|reclaimed>",
                    "artifacts": {
                        "result": "<fan_in>",
                        "log": "<lifecycle>",
                        "recap": "<fan_in.summary>"
                    },
                    "findings": ["finding summary"],
                    "evidence_paths": ["path:line"],
                    "next_action": "<captain_merge|advance>",
                    "open_questions": [],
                    "confidence": "<low|medium|high>"
                }
            }
        },
        "orchestrate": {
            "command": "ccc orchestrate --quiet --json '{...}'",
            "text_command": "ccc orchestrate --text --json '{...}'",
            "payload": {
                "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
                "compact": true,
                "repair_action": "<role for replan>",
                "replan_prompt": "<bounded next task for replan>",
                "resolve_outcome": "completed",
                "resolve_summary": "<final summary for resolve>"
            }
        },
        "session_rollover": {
            "recommended_action": payload.pointer("/long_session_mitigation/recommended_action").cloned().unwrap_or(Value::String("continue".to_string())),
            "checkpoint_command": payload.pointer("/long_session_mitigation/checkpoint_command").cloned().unwrap_or(Value::Null),
            "resume_command": payload.pointer("/long_session_mitigation/resume_command").cloned().unwrap_or(Value::Null),
            "operator_choices": payload.pointer("/long_session_mitigation/choices").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "slash_command_boundary": payload.pointer("/long_session_mitigation/slash_command_boundary").cloned().unwrap_or(Value::Null)
        }
    })
}

fn compact_captain_action_contract(payload: &Value) -> Value {
    let mut contract = payload
        .get("captain_action_contract")
        .cloned()
        .unwrap_or(Value::Null);
    let Some(object) = contract.as_object_mut() else {
        return contract;
    };
    object
        .entry("direct_file_mutation_policy".to_string())
        .or_insert_with(default_direct_file_mutation_policy);
    contract
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
