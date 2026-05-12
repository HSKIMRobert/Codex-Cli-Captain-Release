use crate::specialist_roles::{
    generated_custom_agent_name, is_managed_custom_agent_name, role_for_agent_id,
    GENERATED_CUSTOM_AGENT_NAME_PREFIX,
};
use serde_json::{json, Map, Value};
use std::io;

pub(crate) struct SubagentFanInCompactInput<'a> {
    pub(crate) prior_fan_in: &'a Map<String, Value>,
    pub(crate) status: &'a str,
    pub(crate) summary: Option<&'a str>,
    pub(crate) incoming_fan_in_status: Value,
    pub(crate) incoming_evidence_paths: Value,
    pub(crate) incoming_next_action: Value,
    pub(crate) incoming_open_questions: Value,
    pub(crate) incoming_confidence: Value,
    pub(crate) incoming_risk: Value,
    pub(crate) incoming_checks: Value,
}

pub(crate) struct SubagentLifecyclePayloadInput<'a> {
    pub(crate) prior_lifecycle: Map<String, Value>,
    pub(crate) status: &'a str,
    pub(crate) child_agent_id: &'a str,
    pub(crate) primary_thread_id: Option<&'a str>,
    pub(crate) summary: Option<&'a str>,
    pub(crate) stale_output_after_reclaim: bool,
    pub(crate) active_reclaim_intervention: bool,
    pub(crate) reported_status: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct SubagentRunStateUpdateInput<'a> {
    pub(crate) timestamp: &'a str,
    pub(crate) next_action: &'a str,
    pub(crate) current_phase_name: String,
}

pub(crate) struct SubagentOrchestratorStateUpdateInput<'a> {
    pub(crate) next_action: &'a str,
    pub(crate) can_advance: bool,
    pub(crate) summary: &'a str,
    pub(crate) child_agent_id: &'a str,
    pub(crate) lane_id: Option<&'a str>,
    pub(crate) thread_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) review_outcome: Option<&'a str>,
    pub(crate) captain_intervention: Option<&'a Value>,
    pub(crate) fallback_reason: Option<&'a str>,
    pub(crate) active_handle_cleanup: &'a Value,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct SubagentRunRecordChildInput<'a> {
    pub(crate) active_task_card_id: &'a str,
    pub(crate) child_agent_id: &'a str,
    pub(crate) lane_id: Option<&'a str>,
    pub(crate) assigned_role: &'a str,
    pub(crate) status: &'a str,
    pub(crate) primary_thread_id: Option<&'a str>,
    pub(crate) stale_output_after_reclaim: bool,
    pub(crate) summary: Option<&'a str>,
    pub(crate) review_outcome: Option<&'a str>,
    pub(crate) observed_model: Option<&'a str>,
    pub(crate) total_token_usage: Option<&'a Value>,
    pub(crate) context_tokens: Option<u64>,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct SubagentRunRecordExecutorInput<'a> {
    pub(crate) active_task_card_id: &'a str,
    pub(crate) child_agent_id: &'a str,
    pub(crate) lane_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) primary_thread_id: Option<&'a str>,
    pub(crate) fallback_reason: Option<&'a str>,
    pub(crate) review_outcome: Option<&'a str>,
    pub(crate) observed_model: Option<&'a str>,
    pub(crate) total_token_usage: Option<&'a Value>,
    pub(crate) context_tokens: Option<u64>,
    pub(crate) timestamp: &'a str,
}

pub(crate) fn preferred_subagent_child_agent_id(task_card: &Value) -> Option<String> {
    task_card
        .pointer("/delegation_plan/preferred_custom_agent_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            task_card
                .get("assigned_agent_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|agent_id| {
                    if agent_id.starts_with(GENERATED_CUSTOM_AGENT_NAME_PREFIX) {
                        agent_id.to_string()
                    } else {
                        generated_custom_agent_name(agent_id)
                    }
                })
        })
}

pub(crate) fn normalize_subagent_update_agent_identity(
    task_card: &Value,
    parsed_child_agent_id: Option<&str>,
    parsed_thread_id: Option<&str>,
) -> (String, Option<String>) {
    let assigned_agent_id = task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let generated_assigned_agent_id = assigned_agent_id.map(generated_custom_agent_name);
    let preferred_child_agent_id = preferred_subagent_child_agent_id(task_card);
    let fallback_child_agent_id = preferred_child_agent_id
        .clone()
        .or_else(|| generated_assigned_agent_id.clone())
        .or_else(|| assigned_agent_id.map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());
    let mut thread_id = parsed_thread_id.map(str::to_string);

    let child_agent_id = match parsed_child_agent_id {
        Some(incoming_child_agent_id) => {
            let matches_preferred =
                preferred_child_agent_id.as_deref() == Some(incoming_child_agent_id);
            let matches_assigned = assigned_agent_id == Some(incoming_child_agent_id);
            let matches_generated =
                generated_assigned_agent_id.as_deref() == Some(incoming_child_agent_id);
            if matches_preferred || matches_assigned || matches_generated {
                preferred_child_agent_id
                    .clone()
                    .or_else(|| generated_assigned_agent_id.clone())
                    .unwrap_or_else(|| incoming_child_agent_id.to_string())
            } else if is_managed_custom_agent_name(incoming_child_agent_id) {
                incoming_child_agent_id.to_string()
            } else if role_for_agent_id(incoming_child_agent_id).is_some() {
                incoming_child_agent_id.to_string()
            } else {
                if thread_id.is_none() {
                    thread_id = Some(incoming_child_agent_id.to_string());
                }
                fallback_child_agent_id
            }
        }
        None => fallback_child_agent_id,
    };

    (child_agent_id, thread_id)
}

pub(crate) fn create_subagent_policy_drift_payload(
    task_card: &Value,
    observed_child_agent_id: Option<&str>,
    observed_model: Option<&str>,
    observed_variant: Option<&str>,
    observed_sandbox_mode: Option<&str>,
    observed_approval_policy: Option<&str>,
    timestamp: &str,
) -> Value {
    let plan = task_card
        .get("delegation_plan")
        .cloned()
        .unwrap_or(Value::Null);
    let expected_model = plan
        .get("model")
        .and_then(Value::as_str)
        .or_else(|| {
            task_card
                .get("role_config_snapshot")
                .and_then(|value| value.get("model"))
                .and_then(Value::as_str)
        })
        .map(str::to_string);
    let expected_variant = plan
        .get("variant")
        .and_then(Value::as_str)
        .or_else(|| {
            task_card
                .get("role_config_snapshot")
                .and_then(|value| value.get("variant"))
                .and_then(Value::as_str)
        })
        .map(str::to_string);
    let expected_sandbox_mode = plan
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .or_else(|| task_card.get("sandbox_mode").and_then(Value::as_str))
        .map(str::to_string);
    let expected_child_agent_id = preferred_subagent_child_agent_id(task_card);
    let expected_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .map(str::to_string);
    let observed_child_agent_id = observed_child_agent_id.map(str::to_string);
    let observed_role = observed_child_agent_id
        .as_deref()
        .and_then(role_for_agent_id)
        .map(str::to_string);
    let direct_captain_bypass = observed_child_agent_id.as_deref() == Some("captain")
        && expected_role
            .as_deref()
            .is_some_and(|role| role != "orchestrator");
    let mut mismatches = Vec::new();

    if let (Some(expected), Some(observed)) = (
        expected_child_agent_id.as_deref(),
        observed_child_agent_id.as_deref(),
    ) {
        if expected != observed {
            mismatches.push(json!({
                "field": "child_agent_id",
                "expected": expected,
                "observed": observed,
            }));
        }
    }
    if let (Some(expected), Some(observed)) = (expected_model.as_deref(), observed_model) {
        if expected != observed {
            mismatches.push(json!({
                "field": "model",
                "expected": expected,
                "observed": observed,
            }));
        }
    }
    if let (Some(expected), Some(observed)) = (expected_variant.as_deref(), observed_variant) {
        if expected != observed {
            mismatches.push(json!({
                "field": "variant",
                "expected": expected,
                "observed": observed,
            }));
        }
    }
    if let (Some(expected), Some(observed)) =
        (expected_sandbox_mode.as_deref(), observed_sandbox_mode)
    {
        if expected != observed {
            mismatches.push(json!({
                "field": "sandbox_mode",
                "expected": expected,
                "observed": observed,
            }));
        }
    }
    let drift_ok = mismatches.is_empty();

    json!({
        "required": task_card
            .get("delegation_plan")
            .and_then(|value| value.get("policy_drift_check_required"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "checked_at": timestamp,
        "expected": {
            "model": expected_model,
            "variant": expected_variant,
            "sandbox_mode": expected_sandbox_mode,
            "child_agent_id": expected_child_agent_id,
            "role": expected_role,
            "approval_policy": Value::Null,
        },
        "observed": {
            "model": observed_model,
            "variant": observed_variant,
            "sandbox_mode": observed_sandbox_mode,
            "child_agent_id": observed_child_agent_id,
            "role": observed_role,
            "approval_policy": observed_approval_policy,
        },
        "mismatches": mismatches,
        "direct_captain_bypass": direct_captain_bypass,
        "acceptance_gate": if direct_captain_bypass {
            json!({
                "state": "required",
                "required_action": "spawn_or_merge_review",
                "summary": "Direct captain output for a specialist-owned task requires review or explicit acceptance before merge or close.",
                "authority": "routing_drift_acceptance",
            })
        } else {
            Value::Null
        },
        "ok": drift_ok,
    })
}

pub(crate) fn create_sentinel_intervention_payload(
    parsed: &Value,
    child_agent_id: &str,
    drift_payload: &Value,
    fan_in_compact: &Value,
    timestamp: &str,
) -> Option<Value> {
    let explicit_classification = parsed
        .get("sentinel_classification")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let sentinel_child_agent_id = generated_custom_agent_name("sentinel");
    let reported_by_sentinel = role_for_agent_id(child_agent_id) == Some("sentinel")
        || child_agent_id == sentinel_child_agent_id;
    let direct_captain_bypass = drift_payload
        .get("direct_captain_bypass")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let policy_drift_ok = drift_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let classification = explicit_classification
        .or_else(|| direct_captain_bypass.then_some("enforce"))
        .or_else(|| (!policy_drift_ok).then_some("warn"))
        .or_else(|| reported_by_sentinel.then_some("observe"))?;
    let default_next_action = match classification {
        "enforce" => "require_acceptance_gate",
        "warn" => "review_policy_drift",
        _ => "record_observation",
    };
    let rationale = parsed
        .get("sentinel_rationale")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if direct_captain_bypass {
                "Sentinel guardrail detected direct captain output on a specialist-owned task."
                    .to_string()
            } else if !policy_drift_ok {
                "Sentinel guardrail detected host subagent policy drift.".to_string()
            } else {
                "Sentinel recorded an ownership or execution-boundary observation.".to_string()
            }
        });
    let summary = parsed
        .get("sentinel_summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| fan_in_compact.get("summary").and_then(Value::as_str))
        .unwrap_or(rationale.as_str());
    let next_action = parsed
        .get("sentinel_next_action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_next_action);

    Some(json!({
        "classification": classification,
        "rationale": rationale,
        "next_action": next_action,
        "summary": summary,
        "source": if reported_by_sentinel { "sentinel_subagent" } else { "policy_drift_guardrail" },
        "child_agent_id": child_agent_id,
        "subagent_status": fan_in_compact.get("status").cloned().unwrap_or(Value::Null),
        "evidence_paths": fan_in_compact.get("evidence_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "open_questions": fan_in_compact.get("open_questions").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "policy_drift": drift_payload,
        "authority": "sentinel_guardrail",
        "recorded_at": timestamp,
    }))
}

pub(crate) fn create_subagent_fan_in_compact(input: SubagentFanInCompactInput<'_>) -> Value {
    let incoming_summary = input
        .summary
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    let preserve_prior_completed_fan_in = input.status == "merged"
        && input.prior_fan_in.get("status").and_then(Value::as_str) == Some("completed");
    let fan_in_summary = if preserve_prior_completed_fan_in {
        input
            .prior_fan_in
            .get("summary")
            .cloned()
            .filter(|value| !is_blank_fan_in_value(value))
            .unwrap_or(incoming_summary)
    } else {
        incoming_summary
    };

    let status_value = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "status",
        input.incoming_fan_in_status.clone(),
        Value::String(input.status.to_string()),
    );
    let evidence_paths = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "evidence_paths",
        input.incoming_evidence_paths.clone(),
        Value::Array(Vec::new()),
    );
    let next_action = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "next_action",
        input.incoming_next_action.clone(),
        Value::Null,
    );
    let open_questions = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "open_questions",
        input.incoming_open_questions.clone(),
        Value::Array(Vec::new()),
    );
    let confidence = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "confidence",
        input.incoming_confidence.clone(),
        Value::Null,
    );
    let risk = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "risk",
        input.incoming_risk,
        Value::Null,
    );
    let checks = merge_fan_in_field(
        input.prior_fan_in,
        preserve_prior_completed_fan_in,
        "checks",
        input.incoming_checks,
        Value::Array(Vec::new()),
    );

    json!({
        "schema": "ccc.worker_result_envelope.v1",
        "source": "subagent_update",
        "summary": fan_in_summary,
        "status": status_value,
        "evidence_paths": evidence_paths,
        "next_action": next_action,
        "open_questions": open_questions,
        "confidence": confidence,
        "risk": risk,
        "checks": checks,
        "contract": {
            "required_fields": [
                "summary",
                "status",
                "evidence_paths",
                "next_action",
                "open_questions",
                "confidence",
                "risk",
                "checks"
            ],
            "captain_consumes_compact_fan_in": true,
        }
    })
}

pub(crate) fn create_subagent_lifecycle_payload(input: SubagentLifecyclePayloadInput<'_>) -> Value {
    let mut lifecycle = input.prior_lifecycle;
    lifecycle.insert(
        "status".to_string(),
        Value::String(input.status.to_string()),
    );
    lifecycle.insert(
        "child_agent_id".to_string(),
        Value::String(input.child_agent_id.to_string()),
    );
    lifecycle.insert(
        "updated_at".to_string(),
        Value::String(input.timestamp.to_string()),
    );
    if let Some(thread_id) = input.primary_thread_id {
        lifecycle.insert(
            "thread_id".to_string(),
            Value::String(thread_id.to_string()),
        );
    }
    let lifecycle_timestamp_key = match input.status {
        "spawned" => Some("spawned_at"),
        "acknowledged" => Some("acknowledged_at"),
        "running" => Some("running_at"),
        "stalled" => Some("stalled_at"),
        "completed" => Some("completed_at"),
        "failed" => Some("failed_at"),
        "merged" => Some("merged_at"),
        "reclaimed" => Some("reclaimed_at"),
        _ => None,
    };
    if let Some(key) = lifecycle_timestamp_key {
        lifecycle
            .entry(key.to_string())
            .or_insert_with(|| Value::String(input.timestamp.to_string()));
    }
    if !input.stale_output_after_reclaim {
        if let Some(summary) = input.summary {
            lifecycle.insert("summary".to_string(), Value::String(summary.to_string()));
        }
    }
    if input.stale_output_after_reclaim {
        lifecycle.insert(
            "late_stale_output_at".to_string(),
            Value::String(input.timestamp.to_string()),
        );
    }
    if input.active_reclaim_intervention {
        lifecycle.insert(
            "reported_status".to_string(),
            Value::String(input.reported_status.to_string()),
        );
        lifecycle.insert(
            "host_cancellation_supported".to_string(),
            Value::Bool(false),
        );
        lifecycle.insert(
            "host_worker_may_still_be_running".to_string(),
            Value::Bool(true),
        );
        lifecycle.insert(
            "active_reclaim_intervention_at".to_string(),
            Value::String(input.timestamp.to_string()),
        );
    }

    Value::Object(lifecycle)
}

pub(crate) fn apply_subagent_run_state_update(
    run_state_record: &mut Value,
    input: SubagentRunStateUpdateInput<'_>,
) -> io::Result<()> {
    let run_state_object = run_state_record.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(input.timestamp.to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({
            "command": input.next_action
        }),
    );
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(input.current_phase_name),
    );

    Ok(())
}

pub(crate) fn apply_subagent_orchestrator_state_update(
    orchestrator_state_record: &mut Value,
    input: SubagentOrchestratorStateUpdateInput<'_>,
) -> bool {
    let Some(orchestrator_object) = orchestrator_state_record.as_object_mut() else {
        return false;
    };
    orchestrator_object.insert(
        "decision".to_string(),
        json!({
            "next_step": input.next_action,
            "can_advance": input.can_advance,
            "summary": input.summary,
        }),
    );
    orchestrator_object.insert(
        "execution_request".to_string(),
        json!({
            "entrypoint": "ccc_subagent_update",
            "child_agent_id": input.child_agent_id,
            "lane_id": input.lane_id,
            "thread_id": input.thread_id,
            "status": input.status,
            "review_outcome": input.review_outcome,
            "captain_intervention": input.captain_intervention,
            "fallback_reason": input.fallback_reason,
            "active_handle_cleanup": input.active_handle_cleanup,
            "recorded_at": input.timestamp,
        }),
    );

    true
}

pub(crate) fn update_subagent_run_child_agent_entry(
    run_object: &mut Map<String, Value>,
    input: SubagentRunRecordChildInput<'_>,
) {
    let mut child_agents = run_object
        .get("child_agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let child_index = child_agents.iter().position(|entry| {
        entry.get("task_card_id").and_then(Value::as_str) == Some(input.active_task_card_id)
            && entry.get("agent_id").and_then(Value::as_str) == Some(input.child_agent_id)
            && entry_matches_lane(entry, input.lane_id)
    });
    let mut child_entry = child_index
        .and_then(|index| child_agents.get(index).cloned())
        .unwrap_or_else(|| {
            json!({
                "agent_id": input.child_agent_id,
                "parent_agent_id": "captain",
                "role": input.assigned_role,
                "status": input.status,
                "task_card_id": input.active_task_card_id,
                "lane_id": input.lane_id,
                "execution_mode": "codex_subagent",
                "created_at": input.timestamp,
            })
        });
    if let Some(object) = child_entry.as_object_mut() {
        object.insert(
            "status".to_string(),
            Value::String(input.status.to_string()),
        );
        object.insert(
            "task_card_id".to_string(),
            Value::String(input.active_task_card_id.to_string()),
        );
        if let Some(lane_id) = input.lane_id {
            object.insert("lane_id".to_string(), Value::String(lane_id.to_string()));
        }
        object.insert(
            "updated_at".to_string(),
            Value::String(input.timestamp.to_string()),
        );
        object.insert(
            "execution_mode".to_string(),
            Value::String("codex_subagent".to_string()),
        );
        if let Some(thread_id) = input.primary_thread_id {
            object.insert(
                "thread_id".to_string(),
                Value::String(thread_id.to_string()),
            );
        }
        if !input.stale_output_after_reclaim {
            if let Some(summary) = input.summary {
                object.insert("summary".to_string(), Value::String(summary.to_string()));
            }
        }
        if let Some(outcome) = input.review_outcome {
            object.insert(
                "review_outcome".to_string(),
                Value::String(outcome.to_string()),
            );
        }
        if let Some(model) = input.observed_model {
            object.insert(
                "observed_model".to_string(),
                Value::String(model.to_string()),
            );
        }
        if let Some(usage) = input.total_token_usage {
            object.insert("total_token_usage".to_string(), usage.clone());
        }
        if let Some(context_tokens) = input.context_tokens {
            object.insert("context_tokens".to_string(), json!(context_tokens));
        }
    }
    if let Some(index) = child_index {
        child_agents[index] = child_entry;
    } else {
        child_agents.push(child_entry);
    }
    run_object.insert("child_agents".to_string(), Value::Array(child_agents));
}

pub(crate) fn update_subagent_run_specialist_executor_entry(
    run_object: &mut Map<String, Value>,
    input: SubagentRunRecordExecutorInput<'_>,
) {
    let mut specialist_executors = run_object
        .get("specialist_executors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let executor_id = input
        .lane_id
        .map(|lane| format!("specialist-executor:{}:{lane}", input.child_agent_id))
        .unwrap_or_else(|| format!("specialist-executor:{}", input.child_agent_id));
    let executor_index = specialist_executors.iter().position(|entry| {
        entry.get("executor_id").and_then(Value::as_str) == Some(executor_id.as_str())
            && entry.get("task_card_id").and_then(Value::as_str) == Some(input.active_task_card_id)
            && entry_matches_lane(entry, input.lane_id)
    });
    let mut executor_entry = executor_index
        .and_then(|index| specialist_executors.get(index).cloned())
        .unwrap_or_else(|| {
            json!({
                "executor_id": executor_id,
                "status": input.status,
                "task_card_id": input.active_task_card_id,
                "lane_id": input.lane_id,
                "child_agent_id": input.child_agent_id,
                "execution_mode": "codex_subagent",
                "created_at": input.timestamp,
            })
        });
    if let Some(object) = executor_entry.as_object_mut() {
        object.insert(
            "status".to_string(),
            Value::String(input.status.to_string()),
        );
        object.insert(
            "updated_at".to_string(),
            Value::String(input.timestamp.to_string()),
        );
        if let Some(lane_id) = input.lane_id {
            object.insert("lane_id".to_string(), Value::String(lane_id.to_string()));
        }
        object.insert(
            "execution_mode".to_string(),
            Value::String("codex_subagent".to_string()),
        );
        if let Some(thread_id) = input.primary_thread_id {
            object.insert(
                "thread_id".to_string(),
                Value::String(thread_id.to_string()),
            );
        }
        if let Some(reason) = input.fallback_reason {
            object.insert(
                "fallback_reason".to_string(),
                Value::String(reason.to_string()),
            );
        }
        if let Some(outcome) = input.review_outcome {
            object.insert(
                "review_outcome".to_string(),
                Value::String(outcome.to_string()),
            );
        }
        if let Some(model) = input.observed_model {
            object.insert(
                "observed_model".to_string(),
                Value::String(model.to_string()),
            );
        }
        if let Some(usage) = input.total_token_usage {
            object.insert("total_token_usage".to_string(), usage.clone());
        }
        if let Some(context_tokens) = input.context_tokens {
            object.insert("context_tokens".to_string(), json!(context_tokens));
        }
    }
    if let Some(index) = executor_index {
        specialist_executors[index] = executor_entry;
    } else {
        specialist_executors.push(executor_entry);
    }
    run_object.insert(
        "specialist_executors".to_string(),
        Value::Array(specialist_executors),
    );
}

fn is_blank_fan_in_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

fn entry_matches_lane(entry: &Value, lane_id: Option<&str>) -> bool {
    match lane_id {
        Some(required_lane_id) => {
            entry.get("lane_id").and_then(Value::as_str) == Some(required_lane_id)
        }
        None => entry.get("lane_id").is_none() || entry.get("lane_id") == Some(&Value::Null),
    }
}

fn merge_fan_in_field(
    prior_fan_in: &Map<String, Value>,
    preserve_prior_completed_fan_in: bool,
    field: &str,
    incoming: Value,
    default: Value,
) -> Value {
    if preserve_prior_completed_fan_in && is_blank_fan_in_value(&incoming) {
        if let Some(prior_value) = prior_fan_in.get(field).cloned() {
            if !is_blank_fan_in_value(&prior_value) {
                return prior_value;
            }
        }
    }
    if is_blank_fan_in_value(&incoming) {
        default
    } else {
        incoming
    }
}
