use serde_json::{json, Value};

use crate::specialist_roles::SUBAGENT_FALLBACK_REASON_CODES;
use crate::status_app_panel::CCC_APP_PANEL_RESOURCE_URI;

pub(crate) fn tool_result(
    id: Option<Value>,
    content_text: String,
    structured_content: Value,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": content_text
                }
            ],
            "structuredContent": structured_content
        }
    })
}

pub(crate) fn tool_result_with_meta(
    id: Option<Value>,
    content_text: String,
    structured_content: Value,
    meta: Value,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": content_text
                }
            ],
            "structuredContent": structured_content,
            "_meta": meta
        }
    })
}

pub(crate) fn tool_error(id: Option<Value>, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

fn planned_rows_input_schema() -> Value {
    json!({
        "type": "array",
        "items": {
            "oneOf": [
                { "type": "string" },
                {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "phase_name": { "type": "string" },
                        "name": { "type": "string" },
                        "planned_role": { "type": "string" },
                        "role": { "type": "string" },
                        "planned_agent_id": { "type": "string" },
                        "agent_id": { "type": "string" },
                        "owner_agent": { "type": "string" },
                        "scope": { "type": "string" },
                        "acceptance": { "type": "string" },
                        "status": { "type": "string" },
                        "evidence_links": { "type": "array", "items": { "type": "string" } },
                        "routing_summary": { "type": "string" },
                        "summary": { "type": "string" },
                        "task_card_id": { "type": "string" }
                    },
                    "additionalProperties": true
                }
            ]
        }
    })
}

fn structured_target_mentions_input_schema() -> Value {
    json!({
        "type": "array",
        "items": {
            "oneOf": [
                { "type": "string" },
                {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "file_path": { "type": "string" },
                        "artifact_path": { "type": "string" },
                        "target_path": { "type": "string" },
                        "absolute_path": { "type": "string" },
                        "uri": { "type": "string" },
                        "type": { "type": "string" }
                    },
                    "additionalProperties": true
                }
            ]
        }
    })
}

pub(crate) fn create_tools_list_response(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "ccc_recommend_entry",
                    "description": "Read-only no-mutation preflight for whether a fresh request can proceed directly or should enter Rust CCC control-plane state.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "request": { "type": "string" },
                            "cwd": { "type": "string" }
                        },
                        "required": ["request"],
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_auto_entry",
                    "description": "Deterministic Rust bounded auto-entry for fresh requests with no hydration/reuse wait.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "request": { "type": "string" },
                            "cwd": { "type": "string" },
                            "codex_bin": { "type": "string" }
                        },
                        "required": ["request"],
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_status",
                    "description": "Read-only persisted run visibility from run.json plus optional run-state and LongWay projection.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "run_id": { "type": "string" },
                            "run_ref": { "type": "string" },
                            "run_dir": { "type": "string" },
                            "cwd": { "type": "string" }
                        },
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_activity",
                    "description": "Read-only consolidated activity view over persisted status, orchestration attempt, and active-task delegation artifacts.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "run_id": { "type": "string" },
                            "run_ref": { "type": "string" },
                            "run_dir": { "type": "string" },
                            "cwd": { "type": "string" }
                        },
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_render_app_panel",
                    "description": "Render-oriented CCC LongWay/status panel payload for MCP Apps hosts that support result components.",
                    "_meta": {
                        "ui.resourceUri": CCC_APP_PANEL_RESOURCE_URI,
                        "openai/outputTemplate": CCC_APP_PANEL_RESOURCE_URI
                    },
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "run_id": { "type": "string" },
                            "run_ref": { "type": "string" },
                            "run_dir": { "type": "string" },
                            "cwd": { "type": "string" }
                        },
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_start",
                    "description": "Create a new local CCC bootstrap run without invoking Codex.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "goal": { "type": "string" },
                            "title": { "type": "string" },
                            "intent": { "type": "string" },
                            "scope": { "type": "string" },
                            "acceptance": { "type": "string" },
                            "prompt": { "type": "string" },
                            "task_kind": { "type": "string", "enum": ["execution", "explore", "review", "way"] },
                            "sequence": { "type": "string", "enum": ["PLAN_SEQUENCE", "EXECUTE_SEQUENCE", "plan", "execute"] },
                            "no_longway": { "type": "boolean" },
                            "skip_longway": { "type": "boolean" },
                            "disable_longway": { "type": "boolean" },
                            "planned_rows": planned_rows_input_schema(),
                            "target_paths": structured_target_mentions_input_schema(),
                            "file_paths": structured_target_mentions_input_schema(),
                            "artifact_paths": structured_target_mentions_input_schema(),
                            "mentioned_files": structured_target_mentions_input_schema(),
                            "input_items": structured_target_mentions_input_schema(),
                            "items": structured_target_mentions_input_schema(),
                            "cwd": { "type": "string" }
                        },
                        "required": ["goal", "title", "intent", "scope", "acceptance", "prompt"],
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_run",
                    "description": "Create a new local CCC run and persist the initial Rust execution checkpoint for later orchestration.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "goal": { "type": "string" },
                            "title": { "type": "string" },
                            "intent": { "type": "string" },
                            "scope": { "type": "string" },
                            "acceptance": { "type": "string" },
                            "prompt": { "type": "string" },
                            "task_kind": { "type": "string", "enum": ["execution", "explore", "review", "way"] },
                            "sequence": { "type": "string", "enum": ["PLAN_SEQUENCE", "EXECUTE_SEQUENCE", "plan", "execute"] },
                            "no_longway": { "type": "boolean" },
                            "skip_longway": { "type": "boolean" },
                            "disable_longway": { "type": "boolean" },
                            "planned_rows": planned_rows_input_schema(),
                            "target_paths": structured_target_mentions_input_schema(),
                            "file_paths": structured_target_mentions_input_schema(),
                            "artifact_paths": structured_target_mentions_input_schema(),
                            "mentioned_files": structured_target_mentions_input_schema(),
                            "input_items": structured_target_mentions_input_schema(),
                            "items": structured_target_mentions_input_schema(),
                            "workflow_variant_selection": { "type": "object" },
                            "codex_bin": { "type": "string" },
                            "cwd": { "type": "string" }
                        },
                        "required": ["goal", "title", "intent", "scope", "acceptance", "prompt"],
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_orchestrate",
                    "description": "Persist an explicit Rust orchestration checkpoint for an existing run while the execution bridge is still being ported.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "run_id": { "type": "string" },
                            "run_ref": { "type": "string" },
                            "run_dir": { "type": "string" },
                            "cwd": { "type": "string" },
                            "codex_bin": { "type": "string" },
                            "progression_mode": { "type": "string", "enum": ["single_step", "two_step", "drain_until_boundary"] },
                            "progression_step_count": { "type": "integer", "minimum": 1, "maximum": 2 },
                            "fast_mode": { "type": "boolean" },
                            "max_steps": { "type": "integer", "minimum": 1, "maximum": 12 },
                            "repair_action": { "type": "string" },
                            "replan_prompt": { "type": "string" },
                            "resolve_outcome": { "type": "string" },
                            "resolve_summary": { "type": "string" },
                            "approve_longway": { "type": "boolean" }
                        },
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_subagent_update",
                    "description": "Record host Codex subagent lifecycle, policy-drift checks, and structured fan-in state for an existing run.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "run_id": { "type": "string" },
                            "run_ref": { "type": "string" },
                            "run_dir": { "type": "string" },
                            "cwd": { "type": "string" },
                            "task_card_id": { "type": "string" },
                            "child_agent_id": { "type": "string" },
                            "lane_id": { "type": "string", "enum": ["raider-a", "raider-b", "raider-c", "raider-d", "scout-a", "scout-b", "scout-c", "scout-d"] },
                            "thread_id": { "type": "string" },
                            "status": { "type": "string", "enum": ["spawned", "acknowledged", "running", "stalled", "completed", "failed", "merged", "reclaimed"] },
                            "summary": { "type": "string" },
                            "fan_in_status": { "type": "string" },
                            "review_outcome": { "type": "string", "enum": ["passed", "needs_work", "unsatisfactory", "blocked", "stalled", "reclaimed"] },
                            "intervention_classification": { "type": "string", "enum": ["clarification_only", "bounded_scope_amendment", "direction_or_risk_correction"] },
                            "intervention_rationale": { "type": "string" },
                            "chosen_next_action": { "type": "string", "enum": ["amend_same_worker", "reclaim", "reassign", "close", "clarify", "no_action"] },
                            "budget_snapshot": { "type": "object" },
                            "reassign_target": {
                                "type": "object",
                                "properties": {
                                    "assigned_role": { "type": "string" },
                                    "assigned_agent_id": { "type": "string" },
                                    "scope": { "type": "string" },
                                    "prompt": { "type": "string" }
                                },
                                "required": ["assigned_role", "assigned_agent_id", "prompt"],
                                "additionalProperties": false
                            },
                            "stale_output_policy": { "type": "string" },
                            "stale_output_summary": { "type": "string" },
                            "sentinel_classification": { "type": "string", "enum": ["observe", "warn", "enforce"] },
                            "sentinel_rationale": { "type": "string" },
                            "sentinel_next_action": { "type": "string" },
                            "sentinel_summary": { "type": "string" },
                            "findings": { "type": "array", "items": { "type": "string" } },
                            "evidence_paths": { "type": "array", "items": { "type": "string" } },
                            "next_action": { "type": "string" },
                            "open_questions": { "type": "array", "items": { "type": "string" } },
                            "confidence": {},
                            "observed_model": { "type": "string" },
                            "observed_variant": { "type": "string" },
                            "observed_sandbox_mode": { "type": "string" },
                            "observed_approval_policy": { "type": "string" },
                            "fallback_reason": {
                                "type": "string",
                                "enum": SUBAGENT_FALLBACK_REASON_CODES
                            },
                            "total_token_usage": { "type": "object" },
                            "context_tokens": { "type": "integer", "minimum": 0 },
                            "estimated_context_tokens": { "type": "integer", "minimum": 0 },
                            "event_ref": { "type": "string" },
                            "mode": { "type": "string", "enum": ["full", "compact"] },
                            "compact": { "type": "boolean" }
                        },
                        "required": ["status"],
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_code_graph",
                    "description": "Update or query local repository code graphs for file summaries, dependencies, impact, review context, flow tracing, criticality, architecture overview, full-text search, and multi-repo search.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "cwd": { "type": "string" },
                            "query": { "type": "string", "enum": ["file_summary", "imports", "callers", "callees", "tests", "impact", "blast_radius", "review_context", "flow_trace", "flows", "criticality", "criticality_scores", "communities", "architecture_overview", "full_text_search", "search", "multi_repo_search"] },
                            "paths": { "type": "array", "items": { "type": "string" } },
                            "text": { "type": "string" },
                            "term": { "type": "string" },
                            "search": { "type": "string" },
                            "direction": { "type": "string", "enum": ["both", "callers", "callees", "upstream", "downstream"] },
                            "max_depth": { "type": "integer", "minimum": 1, "maximum": 6 },
                            "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                            "repos": {
                                "type": "array",
                                "items": {
                                    "oneOf": [
                                        { "type": "string" },
                                        {
                                            "type": "object",
                                            "properties": {
                                                "cwd": { "type": "string" },
                                                "store_path": { "type": "string" },
                                                "update": { "type": "boolean" }
                                            },
                                            "additionalProperties": false
                                        }
                                    ]
                                }
                            },
                            "tolaria_enabled": { "type": "boolean" },
                            "tolaria_sync": { "type": "boolean" },
                            "tolaria_vault_path": { "type": "string" },
                            "update": { "type": "boolean" },
                            "store_path": { "type": "string" }
                        },
                        "additionalProperties": false
                    }
                },
                {
                    "name": "ccc_server_identity",
                    "description": "Attached CCC MCP session identity plus early Rust-preview install/config diagnostics.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }
                }
            ]
        }
    })
}

pub(crate) fn tool_call_arguments(message: &Value) -> Value {
    message
        .get("params")
        .and_then(|params| params.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

pub(crate) fn create_start_tool_text(start_payload: &Value, status_payload: &Value) -> String {
    format!(
        "Created run {} for {}",
        start_payload
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown-run"),
        status_payload
            .get("goal")
            .and_then(Value::as_str)
            .unwrap_or("untitled goal")
    )
}

pub(crate) fn create_start_tool_structured_content(
    start_payload: &Value,
    status_payload: &Value,
) -> Value {
    json!({
        "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
        "run_id": start_payload.get("run_id").cloned().unwrap_or(Value::Null),
        "task_card_id": start_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
        "run_directory": start_payload.get("run_directory").cloned().unwrap_or(Value::Null),
        "run_ref": start_payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "status": start_payload.get("status").cloned().unwrap_or(Value::Null),
        "stage": start_payload.get("stage").cloned().unwrap_or(Value::Null),
        "sequence": start_payload.get("sequence").cloned().unwrap_or(Value::Null),
        "approval_state": start_payload.get("approval_state").cloned().unwrap_or(Value::Null),
        "current_task": compact_operator_task(status_payload),
        "current_task_card": compact_operator_task(status_payload),
        "longway": compact_operator_longway(status_payload),
        "captain_guard": compact_operator_captain_guard(status_payload),
        "next_step": start_payload.get("next_step").cloned().unwrap_or(Value::Null),
        "recommended_next_poll_ms": start_payload.get("recommended_next_poll_ms").cloned().unwrap_or(Value::Null),
        "can_advance": start_payload.get("can_advance").cloned().unwrap_or(Value::Null),
        "allowed_next_commands": start_payload.get("allowed_next_commands").cloned().unwrap_or(Value::Null),
        "way_clarification_request": start_payload
            .get("way_clarification_request")
            .cloned()
            .filter(|value| !value.is_null())
            .or_else(|| status_payload.get("way_clarification_request").cloned())
            .unwrap_or(Value::Null),
    })
}

pub(crate) fn create_run_tool_text(run_payload: &Value, status_payload: &Value) -> String {
    format!(
        "Created and checkpointed run {} for {}",
        run_payload
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown-run"),
        status_payload
            .get("goal")
            .and_then(Value::as_str)
            .unwrap_or("untitled goal")
    )
}

pub(crate) fn create_run_tool_structured_content(
    run_payload: &Value,
    status_payload: &Value,
) -> Value {
    json!({
        "cwd": run_payload.get("cwd").cloned().unwrap_or(Value::Null),
        "run_id": run_payload.get("run_id").cloned().unwrap_or(Value::Null),
        "task_card_id": run_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
        "run_directory": run_payload.get("run_directory").cloned().unwrap_or(Value::Null),
        "run_ref": run_payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "entrypoint": run_payload.get("entrypoint").cloned().unwrap_or(Value::Null),
        "status": status_payload.get("status").cloned().unwrap_or(Value::Null),
        "stage": status_payload.get("stage").cloned().unwrap_or(Value::Null),
        "sequence": status_payload.get("sequence").cloned().unwrap_or(Value::Null),
        "approval_state": status_payload.get("approval_state").cloned().unwrap_or(Value::Null),
        "current_task": compact_operator_task(status_payload),
        "current_task_card": compact_operator_task(status_payload),
        "longway": compact_operator_longway(status_payload),
        "captain_guard": compact_operator_captain_guard(status_payload),
        "thread_id": run_payload.get("thread_id").cloned().unwrap_or(Value::Null),
        "next_step": run_payload.get("next_step").cloned().unwrap_or(Value::Null),
        "recommended_next_poll_ms": run_payload.get("recommended_next_poll_ms").cloned().unwrap_or(Value::Null),
        "can_advance": run_payload.get("can_advance").cloned().unwrap_or(Value::Null),
        "advanced": run_payload.get("advanced").cloned().unwrap_or(Value::Null),
        "routing_summary": run_payload.get("routing_summary").cloned().unwrap_or(Value::Null),
        "routing_trace": run_payload.get("routing_trace").cloned().unwrap_or(Value::Null),
        "allowed_next_commands": run_payload.get("allowed_next_commands").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn create_orchestrate_tool_text(orchestrate_payload: &Value) -> String {
    orchestrate_payload
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("Rust ccc_orchestrate persisted an explicit checkpoint.")
        .to_string()
}

pub(crate) fn create_orchestrate_tool_structured_content(
    orchestrate_payload: &Value,
    status_payload: &Value,
) -> Value {
    json!({
        "cwd": orchestrate_payload.get("cwd").cloned().unwrap_or(Value::Null),
        "run_id": orchestrate_payload.get("run_id").cloned().unwrap_or(Value::Null),
        "run_directory": orchestrate_payload.get("run_directory").cloned().unwrap_or(Value::Null),
        "run_ref": orchestrate_payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "attempt_id": orchestrate_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
        "task_card_id": status_payload
            .get("current_task_card")
            .and_then(|value| value.get("task_card_id"))
            .cloned()
            .unwrap_or(Value::Null),
        "status": status_payload.get("status").cloned().unwrap_or(Value::Null),
        "stage": status_payload.get("stage").cloned().unwrap_or(Value::Null),
        "sequence": status_payload.get("sequence").cloned().unwrap_or(Value::Null),
        "approval_state": status_payload.get("approval_state").cloned().unwrap_or(Value::Null),
        "current_task": compact_operator_task(status_payload),
        "current_task_card": compact_operator_task(status_payload),
        "longway": compact_operator_longway(status_payload),
        "captain_guard": compact_operator_captain_guard(status_payload),
        "starting_next_step": orchestrate_payload.get("starting_next_step").cloned().unwrap_or(Value::Null),
        "next_step": orchestrate_payload.get("next_step").cloned().unwrap_or(Value::Null),
        "progression_mode": orchestrate_payload.get("progression_mode").cloned().unwrap_or(Value::Null),
        "can_advance": orchestrate_payload.get("can_advance").cloned().unwrap_or(Value::Null),
        "advanced": orchestrate_payload.get("advanced").cloned().unwrap_or(Value::Null),
        "summary": orchestrate_payload.get("summary").cloned().unwrap_or(Value::Null),
        "approval_transition": orchestrate_payload.get("approval_transition").cloned().unwrap_or(Value::Null),
        "way_clarification_consumption": orchestrate_payload.get("way_clarification_consumption").cloned().unwrap_or(Value::Null),
        "scheduler_decision": orchestrate_payload.get("scheduler_decision").cloned().unwrap_or(Value::Null),
        "post_fan_in_captain_decision": orchestrate_payload.get("post_fan_in_captain_decision").cloned().unwrap_or(Value::Null),
        "launch_result": orchestrate_payload.get("launch_result").cloned().unwrap_or(Value::Null),
        "reclaimed_targets": orchestrate_payload.get("reclaimed_targets").cloned().unwrap_or(Value::Null),
        "collapsed_fan_in": orchestrate_payload.get("collapsed_fan_in").cloned().unwrap_or(Value::Null),
        "consumed_worker_result_envelope": orchestrate_payload.get("consumed_worker_result_envelope").cloned().unwrap_or(Value::Null),
        "consumed_pending_follow_up": orchestrate_payload.get("consumed_pending_follow_up").cloned().unwrap_or(Value::Null),
        "allowed_next_commands": orchestrate_payload.get("allowed_next_commands").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn create_subagent_update_tool_text(update_payload: &Value) -> String {
    update_payload
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("CCC recorded a host subagent lifecycle checkpoint.")
        .to_string()
}

pub(crate) fn create_subagent_update_tool_structured_content(
    update_payload: &Value,
    status_payload: &Value,
) -> Value {
    if update_payload.get("response_mode").and_then(Value::as_str) == Some("compact") {
        return json!({
            "mode": "compact",
            "cwd": update_payload.get("cwd").cloned().unwrap_or(Value::Null),
            "run_id": update_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": update_payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "task_card_id": update_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
            "child_agent_id": update_payload.get("child_agent_id").cloned().unwrap_or(Value::Null),
            "lane_id": update_payload.get("lane_id").cloned().unwrap_or(Value::Null),
            "event_ref": update_payload.get("event_ref").cloned().unwrap_or(Value::Null),
            "subagent_status": update_payload.get("subagent_status").cloned().unwrap_or(Value::Null),
            "summary": update_payload.get("summary").cloned().unwrap_or(Value::Null),
            "fan_in_artifact": update_payload.get("fan_in_artifact").cloned().unwrap_or(Value::Null),
            "review_outcome": update_payload.get("review_outcome").cloned().unwrap_or(Value::Null),
            "fallback_reason": update_payload.get("fallback_reason").cloned().unwrap_or(Value::Null),
            "sentinel_intervention": update_payload.get("sentinel_intervention").cloned().unwrap_or(Value::Null),
            "next_step": status_payload.get("next_step").cloned().unwrap_or(Value::Null),
            "can_advance": status_payload.get("can_advance").cloned().unwrap_or(Value::Null),
            "agents": compact_operator_agents(status_payload),
            "longway": compact_operator_longway(status_payload),
            "captain_guard": compact_operator_captain_guard(status_payload),
        });
    }

    json!({
        "cwd": update_payload.get("cwd").cloned().unwrap_or(Value::Null),
        "run_id": update_payload.get("run_id").cloned().unwrap_or(Value::Null),
        "run_directory": update_payload.get("run_directory").cloned().unwrap_or(Value::Null),
        "run_ref": update_payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "task_card_id": update_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
        "child_agent_id": update_payload.get("child_agent_id").cloned().unwrap_or(Value::Null),
        "lane_id": update_payload.get("lane_id").cloned().unwrap_or(Value::Null),
        "thread_id": update_payload.get("thread_id").cloned().unwrap_or(Value::Null),
        "subagent_status": update_payload.get("subagent_status").cloned().unwrap_or(Value::Null),
        "summary": update_payload.get("summary").cloned().unwrap_or(Value::Null),
            "fan_in": compact_operator_fan_in(update_payload.get("fan_in").unwrap_or(&Value::Null)),
            "captain_intervention": compact_operator_captain_intervention(update_payload),
            "sentinel_intervention": compact_operator_sentinel_intervention(update_payload),
            "next_step": status_payload.get("next_step").cloned().unwrap_or(Value::Null),
        "can_advance": status_payload.get("can_advance").cloned().unwrap_or(Value::Null),
        "current_task": compact_operator_task(status_payload),
        "agents": compact_operator_agents(status_payload),
        "longway": compact_operator_longway(status_payload),
        "captain_guard": compact_operator_captain_guard(status_payload),
    })
}

fn compact_operator_task(status_payload: &Value) -> Value {
    let task = status_payload
        .get("current_task_card")
        .unwrap_or(&Value::Null);
    if !task.is_object() {
        return Value::Null;
    }
    json!({
        "task_card_id": task.get("task_card_id").cloned().unwrap_or(Value::Null),
        "title": task.get("title").cloned().unwrap_or(Value::Null),
        "assigned_role": task.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": task.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "status": task.get("status").cloned().unwrap_or(Value::Null),
        "verification_state": task.get("verification_state").cloned().unwrap_or(Value::Null),
    })
}

fn compact_operator_longway(status_payload: &Value) -> Value {
    let longway = status_payload.get("longway").unwrap_or(&Value::Null);
    json!({
        "completed": longway.get("completed_phase_count").cloned().unwrap_or(Value::Null),
        "total": longway.get("phase_count").cloned().unwrap_or(Value::Null),
        "current_item": longway.get("current_item").cloned().unwrap_or(Value::Null),
        "active_phase_name": longway.get("active_phase_name").cloned().unwrap_or(Value::Null),
        "active_phase_status": longway.get("active_phase_status").cloned().unwrap_or(Value::Null),
        "planned_row_count": longway.get("planned_row_count").cloned().unwrap_or(Value::Null),
    })
}

fn compact_operator_agents(status_payload: &Value) -> Value {
    status_payload
        .pointer("/host_subagent_state/subagent_activity")
        .and_then(Value::as_array)
        .map(|agents| {
            agents
                .iter()
                .take(6)
                .map(|agent| {
                    json!({
                        "child_agent_id": agent.get("child_agent_id").cloned().unwrap_or(Value::Null),
                        "assigned_role": agent.get("assigned_role").cloned().unwrap_or(Value::Null),
                        "status": agent.get("status").cloned().unwrap_or(Value::Null),
                        "task_title": agent.get("task_title").cloned().unwrap_or(Value::Null),
                        "next_action": agent.get("next_action").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into()
}

fn compact_operator_captain_guard(status_payload: &Value) -> Value {
    let guard = status_payload
        .get("captain_action_contract")
        .unwrap_or(&Value::Null);
    if !guard.is_object() {
        return Value::Null;
    }
    json!({
        "allowed_action": guard.get("allowed_action").cloned().unwrap_or(Value::Null),
        "required_action": guard.get("required_action").cloned().unwrap_or(Value::Null),
        "direct_finish_allowed": guard.get("direct_finish_allowed").cloned().unwrap_or(Value::Null),
        "direct_mutation_allowed": guard.get("direct_mutation_allowed").cloned().unwrap_or(Value::Null),
        "direct_file_mutation_policy": compact_direct_file_mutation_policy(guard),
        "denied_action_reason": guard.get("denied_action_reason").cloned().unwrap_or(Value::Null),
        "preflight_guard": guard.get("preflight_guard").cloned().unwrap_or(Value::Null),
        "preferred_operator_transport": guard
            .get("preferred_operator_transport")
            .cloned()
            .unwrap_or(Value::String("ccc_cli_quiet_subcommand".to_string())),
        "mcp_tool_call_policy": guard
            .get("mcp_tool_call_policy")
            .cloned()
            .unwrap_or(Value::String(
                "reserve_for_app_or_structured_inspection_or_cli_unavailable".to_string(),
            )),
    })
}

fn compact_direct_file_mutation_policy(guard: &Value) -> Value {
    guard
        .get("direct_file_mutation_policy")
        .cloned()
        .unwrap_or_else(|| {
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
        })
}

fn compact_operator_fan_in(fan_in: &Value) -> Value {
    if !fan_in.is_object() {
        return Value::Null;
    }
    json!({
        "status": fan_in.get("status").cloned().unwrap_or(Value::Null),
        "summary": fan_in.get("summary").cloned().unwrap_or(Value::Null),
        "next_action": fan_in.get("next_action").cloned().unwrap_or(Value::Null),
        "confidence": fan_in.get("confidence").cloned().unwrap_or(Value::Null),
        "evidence_paths": fan_in.get("evidence_paths").cloned().unwrap_or(Value::Null),
    })
}

fn compact_operator_captain_intervention(update_payload: &Value) -> Value {
    let intervention = update_payload
        .get("captain_intervention")
        .unwrap_or(&Value::Null);
    if !intervention.is_object() {
        return Value::Null;
    }
    json!({
        "classification": intervention.get("classification").cloned().unwrap_or(Value::Null),
        "chosen_next_action": intervention.get("chosen_next_action").cloned().unwrap_or(Value::Null),
        "next_action": intervention.get("next_action").cloned().unwrap_or(Value::Null),
        "summary": intervention.get("summary").cloned().unwrap_or(Value::Null),
    })
}

fn compact_operator_sentinel_intervention(update_payload: &Value) -> Value {
    let intervention = update_payload
        .get("sentinel_intervention")
        .unwrap_or(&Value::Null);
    if !intervention.is_object() {
        return Value::Null;
    }
    json!({
        "classification": intervention.get("classification").cloned().unwrap_or(Value::Null),
        "next_action": intervention.get("next_action").cloned().unwrap_or(Value::Null),
        "summary": intervention.get("summary").cloned().unwrap_or(Value::Null),
    })
}
