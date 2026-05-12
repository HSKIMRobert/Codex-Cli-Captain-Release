use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct OrchestrationAttemptPayload {
    pub(crate) payload: Value,
    pub(crate) summary: String,
}

pub(crate) struct OrchestrationAttemptPayloadInput<'a> {
    pub(crate) attempt_id: &'a str,
    pub(crate) run_id: &'a str,
    pub(crate) timestamp: &'a str,
    pub(crate) requested_progression_mode: &'a str,
    pub(crate) starting_next_step: &'a str,
    pub(crate) post_fan_in_next_step: &'a str,
    pub(crate) codex_bin: &'a str,
    pub(crate) effective_task_card: &'a Value,
    pub(crate) current_task_card: &'a Value,
    pub(crate) effective_delegation_plan: &'a Value,
    pub(crate) resolved_summary: Option<&'a str>,
    pub(crate) follow_up_task_card: Option<&'a Value>,
    pub(crate) retry_current_specialist: bool,
    pub(crate) launch_result: Option<&'a Value>,
    pub(crate) reclaimed_targets: &'a [Value],
    pub(crate) collapsed_fan_in: Option<&'a Value>,
    pub(crate) consumed_pending_follow_up_for_attempt: Option<&'a Value>,
    pub(crate) consumed_queued_captain_follow_up: bool,
    pub(crate) preferred_execution_mode: &'a str,
    pub(crate) subagent_available: bool,
    pub(crate) codex_exec_dispatch_allowed: bool,
    pub(crate) dispatched_worker_terminal: Option<&'a str>,
    pub(crate) dispatched_worker_state: &'a str,
    pub(crate) scheduler_runtime_decision: &'a Value,
    pub(crate) post_fan_in_captain_decision: &'a Value,
    pub(crate) next_step_after_attempt: &'a str,
    pub(crate) can_advance_after_attempt: bool,
}

pub(crate) fn resolve_requested_progression_mode(parsed: &Value) -> String {
    if parsed.get("fast_mode").and_then(Value::as_bool) == Some(true) {
        "two_step".to_string()
    } else if parsed.get("progression_step_count").and_then(Value::as_u64) == Some(2) {
        "two_step".to_string()
    } else {
        parsed
            .get("progression_mode")
            .and_then(Value::as_str)
            .unwrap_or("single_step")
            .to_string()
    }
}

pub(crate) fn next_orchestration_attempt_file(
    run_directory: &Path,
) -> io::Result<(String, PathBuf)> {
    let attempts_directory = run_directory.join("orchestration").join("attempts");
    fs::create_dir_all(&attempts_directory)?;
    let mut max_index = 0_u32;

    for entry in fs::read_dir(&attempts_directory)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("attempt-") || !file_name.ends_with(".json") {
            continue;
        }
        let number_text = &file_name["attempt-".len()..file_name.len() - ".json".len()];
        if let Ok(index) = number_text.parse::<u32>() {
            max_index = max_index.max(index);
        }
    }

    let next_index = max_index + 1;
    let attempt_id = format!("attempt-{next_index:04}");
    let attempt_file = attempts_directory.join(format!("{attempt_id}.json"));
    Ok((attempt_id, attempt_file))
}

pub(crate) fn create_orchestration_attempt_payload(
    input: OrchestrationAttemptPayloadInput<'_>,
) -> OrchestrationAttemptPayload {
    let summary = create_orchestration_attempt_summary(&input);
    let stop_reason = resolve_orchestration_attempt_stop_reason(&input);
    let step_summary = create_orchestration_attempt_step_summary(&input);
    let consumed_worker_result_envelope =
        create_consumed_worker_result_envelope_citation(&input, stop_reason);
    let scheduler_decision =
        create_attempt_scheduler_decision(&input, &consumed_worker_result_envelope);

    let payload = json!({
        "attempt_id": input.attempt_id,
        "entrypoint": "ccc_orchestrate",
        "run_id": input.run_id,
        "task_card_id": input.effective_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "started_at": input.timestamp,
        "completed_at": input.timestamp,
        "requested_progression_mode": input.requested_progression_mode,
        "starting_next_step": input.starting_next_step,
        "codex_bin": input.codex_bin,
        "dispatch_policy": {
            "preferred_execution_mode": input.preferred_execution_mode,
            "subagent_available": input.subagent_available,
            "codex_exec_dispatch_allowed": input.codex_exec_dispatch_allowed,
        },
        "scheduler_decision": scheduler_decision,
        "post_fan_in_captain_decision": input.post_fan_in_captain_decision,
        "consumed_worker_result_envelope": consumed_worker_result_envelope,
        "stop": {
            "reason": stop_reason,
            "summary": summary.clone(),
        },
        "launch_result": input.launch_result,
        "reclaimed_targets": input.reclaimed_targets,
        "collapsed_fan_in": input.collapsed_fan_in,
        "consumed_pending_follow_up": input.consumed_pending_follow_up_for_attempt,
        "follow_up_task_card": input.follow_up_task_card,
        "steps": [
            {
                "step_number": 1,
                "command": input.starting_next_step,
                "before": {
                    "next_step": input.starting_next_step,
                    "can_advance": true
                },
                "after": {
                    "next_step": input.next_step_after_attempt,
                    "can_advance": input.can_advance_after_attempt
                },
                "summary": step_summary
            }
        ]
    });

    OrchestrationAttemptPayload { payload, summary }
}

fn create_attempt_scheduler_decision(
    input: &OrchestrationAttemptPayloadInput<'_>,
    consumed_worker_result_envelope: &Value,
) -> Value {
    let selected_task_card = input
        .follow_up_task_card
        .unwrap_or(input.effective_task_card);
    let selected_planned_row = selected_task_card
        .get("planned_longway_row")
        .cloned()
        .unwrap_or(Value::Null);
    let parallel_required_lane_ids = selected_task_card
        .pointer("/parallel_fanout/required_lane_ids")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let decision_source = if !selected_planned_row.is_null() {
        "planned_row_materialization"
    } else if input.follow_up_task_card.is_some() {
        "captain_follow_up_task_card"
    } else if selected_task_card
        .get("parallel_fanout")
        .is_some_and(|value| value.is_object())
    {
        "bounded_parallel_fanout"
    } else if input.collapsed_fan_in.is_some() {
        "compact_fan_in"
    } else if input.reclaimed_targets.is_empty() {
        "approved_longway_task_cards"
    } else {
        "blocked_work_reclaim"
    };

    json!({
        "schema": "ccc.scheduler_decision.v1",
        "decision_source": decision_source,
        "starting_next_step": input.starting_next_step,
        "post_fan_in_next_step": input.post_fan_in_next_step,
        "next_step_after_attempt": input.next_step_after_attempt,
        "can_advance_after_attempt": input.can_advance_after_attempt,
        "selected_task_card_id": selected_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "selected_role": selected_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "selected_agent_id": selected_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "selected_planned_row": selected_planned_row,
        "parallel": {
            "candidate_count": parallel_required_lane_ids.as_array().map(Vec::len).unwrap_or(0),
            "required_lane_ids": parallel_required_lane_ids,
            "mode": selected_task_card.pointer("/parallel_fanout/mode").cloned().unwrap_or(Value::Null),
        },
        "blocked": {
            "reclaimed_target_count": input.reclaimed_targets.len(),
            "retry_current_specialist": input.retry_current_specialist,
        },
        "action": input.scheduler_runtime_decision.get("action").cloned().unwrap_or(Value::Null),
        "post_fan_in_captain_decision": input.post_fan_in_captain_decision,
        "consumed_worker_result_envelope": consumed_worker_result_envelope.clone(),
        "owns": {
            "next_task_selection": true,
            "planned_row_materialization": true,
            "bounded_parallel_fanout": true,
            "blocked_work": true,
            "pending_card_updates": true,
        }
    })
}

fn create_consumed_worker_result_envelope_citation(
    input: &OrchestrationAttemptPayloadInput<'_>,
    stop_reason: &str,
) -> Value {
    let Some(collapsed_fan_in) = input.collapsed_fan_in else {
        return Value::Null;
    };
    let envelope = input
        .current_task_card
        .get("worker_result_envelope")
        .or_else(|| input.current_task_card.get("subagent_fan_in"));
    let envelope_present = envelope.is_some_and(|value| value.is_object());

    json!({
        "schema": "ccc.consumed_worker_result_envelope.v1",
        "source": if input.current_task_card.get("worker_result_envelope").is_some() {
            "current_task_card.worker_result_envelope"
        } else if input.current_task_card.get("subagent_fan_in").is_some() {
            "current_task_card.subagent_fan_in"
        } else {
            "collapsed_fan_in"
        },
        "task_card_id": input.current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "decision": {
            "stop_reason": stop_reason,
            "next_step_after_attempt": input.next_step_after_attempt,
            "can_advance_after_attempt": input.can_advance_after_attempt,
        },
        "collapsed_fan_in": {
            "collapsed_at": collapsed_fan_in.get("collapsed_at").cloned().unwrap_or(Value::Null),
            "completed": collapsed_fan_in.get("completed").cloned().unwrap_or(Value::Null),
            "failed": collapsed_fan_in.get("failed").cloned().unwrap_or(Value::Null),
            "cancelled": collapsed_fan_in.get("cancelled").cloned().unwrap_or(Value::Null),
            "summary": collapsed_fan_in.get("summary").cloned().unwrap_or(Value::Null),
            "thread_ids": collapsed_fan_in.get("thread_ids").cloned().unwrap_or(Value::Array(Vec::new())),
        },
        "envelope_present": envelope_present,
        "worker_result_envelope": envelope.cloned().unwrap_or(Value::Null),
        "captain_consumed_for_decision": true,
    })
}

fn create_orchestration_attempt_summary(input: &OrchestrationAttemptPayloadInput<'_>) -> String {
    let reclaimed_worker = !input.reclaimed_targets.is_empty();
    let collapsed_worker_fan_in = input.collapsed_fan_in.is_some();

    if let Some(summary) = input.resolved_summary {
        if collapsed_worker_fan_in {
            format!("Captain collapsed explicit fan-in and resolved the run. {summary}")
        } else {
            summary.to_string()
        }
    } else if let Some(task_card) = input.follow_up_task_card {
        let next_agent = task_card
            .get("assigned_agent_id")
            .and_then(Value::as_str)
            .unwrap_or("worker");
        if input.consumed_queued_captain_follow_up && collapsed_worker_fan_in {
            format!(
                "Captain collapsed explicit fan-in, consumed the queued captain follow-up, and selected {next_agent} as the next bounded specialist."
            )
        } else if input.consumed_queued_captain_follow_up {
            format!(
                "Captain consumed the queued captain follow-up and selected {next_agent} as the next bounded specialist."
            )
        } else if collapsed_worker_fan_in {
            format!(
                "Captain collapsed explicit fan-in, updated the LongWay, and selected {next_agent} as the next bounded specialist."
            )
        } else {
            format!(
                "Captain updated the LongWay and selected {next_agent} as the next bounded specialist."
            )
        }
    } else if input.retry_current_specialist {
        let specialist = input
            .current_task_card
            .get("assigned_agent_id")
            .and_then(Value::as_str)
            .unwrap_or("worker");
        if collapsed_worker_fan_in {
            format!(
                "Captain collapsed explicit fan-in, selected a bounded retry for {specialist}, and reopened the run for explicit execute_task dispatch."
            )
        } else {
            format!(
                "Captain selected a bounded retry for {specialist} and reopened the run for explicit execute_task dispatch."
            )
        }
    } else if let Some(launch) = input.launch_result {
        if let Some(worker_status) = input.dispatched_worker_terminal {
            format!(
                "Rust ccc_orchestrate launched {} for {}, observed an immediate bounded {} result, and moved the run to await_fan_in.",
                launch
                    .get("child_agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or("worker"),
                input.starting_next_step,
                worker_status
            )
        } else {
            format!(
                "Rust ccc_orchestrate launched {} for {}, handed supervision to a detached worker monitor, and moved the run to await_fan_in ({}).",
                launch
                    .get("child_agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or("worker"),
                input.starting_next_step,
                input.dispatched_worker_state,
            )
        }
    } else if reclaimed_worker {
        format!(
            "Rust ccc_orchestrate reclaimed {} stuck worker(s) at await_fan_in and reopened the run for captain advance.",
            input.reclaimed_targets.len()
        )
    } else if let Some(collapsed) = input.collapsed_fan_in {
        collapsed
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Rust ccc_orchestrate collapsed explicit fan-in and reopened the run for captain advance.")
            .to_string()
    } else if input.starting_next_step == "advance" {
        "Captain checkpoint is ready for the next bounded specialist selection.".to_string()
    } else if input.post_fan_in_next_step == "execute_task"
        && input.preferred_execution_mode == "codex_subagent"
        && input.subagent_available
        && !input.codex_exec_dispatch_allowed
    {
        let custom_agent = input
            .effective_delegation_plan
            .pointer("/subagent_spawn_contract/custom_agent_name")
            .and_then(Value::as_str)
            .or_else(|| {
                input
                    .effective_delegation_plan
                    .get("preferred_custom_agent_name")
                    .and_then(Value::as_str)
            })
            .unwrap_or("configured custom subagent");
        format!(
            "Captain must dispatch {custom_agent} as a host custom subagent; direct codex exec fallback is blocked until an explicit fallback reason is recorded."
        )
    } else {
        format!(
            "Rust ccc_orchestrate recorded a {} checkpoint for {}. Codex dispatch is still pending the Rust execution port.",
            input.requested_progression_mode, input.starting_next_step
        )
    }
}

fn resolve_orchestration_attempt_stop_reason(
    input: &OrchestrationAttemptPayloadInput<'_>,
) -> &'static str {
    if input.launch_result.is_some() {
        "await_fan_in"
    } else if input.post_fan_in_next_step == "execute_task"
        && input.preferred_execution_mode == "codex_subagent"
        && input.subagent_available
        && !input.codex_exec_dispatch_allowed
    {
        "await_host_custom_subagent"
    } else if input.resolved_summary.is_some() {
        "resolved"
    } else if input.consumed_queued_captain_follow_up {
        "captain_pending_follow_up_consumed"
    } else if input.follow_up_task_card.is_some() {
        "captain_replanned"
    } else if input.retry_current_specialist {
        "captain_retry_selected"
    } else if !input.reclaimed_targets.is_empty() {
        "reclaimed_worker"
    } else if input.collapsed_fan_in.is_some() {
        "collapsed_fan_in"
    } else if input.starting_next_step == "advance" {
        "captain_checkpoint"
    } else {
        "await_rust_execution_port"
    }
}

fn create_orchestration_attempt_step_summary(
    input: &OrchestrationAttemptPayloadInput<'_>,
) -> &'static str {
    if input.launch_result.is_some() {
        if input.dispatched_worker_terminal.is_some() {
            "Launched a bounded worker, observed an immediate terminal result, and persisted await_fan_in truth."
        } else {
            "Launched a bounded worker under detached supervision and persisted await_fan_in truth."
        }
    } else if input.resolved_summary.is_some() {
        "Captain resolved the run without launching another specialist."
    } else if input.consumed_queued_captain_follow_up {
        "Consumed the queued captain follow-up and queued the next specialist for explicit dispatch."
    } else if input.follow_up_task_card.is_some() {
        "Captain updated the LongWay and queued the next specialist for explicit dispatch."
    } else if input.retry_current_specialist {
        "Captain selected a bounded retry of the current specialist."
    } else if !input.reclaimed_targets.is_empty() {
        "Reclaimed stuck worker state and reopened the run for explicit captain follow-up."
    } else if input.collapsed_fan_in.is_some() {
        "Collapsed terminal child worker state into an explicit captain fan-in checkpoint."
    } else if input.starting_next_step == "advance" {
        "Persisted an explicit captain checkpoint without launching another specialist."
    } else {
        "Persisted an explicit orchestration checkpoint without dispatching Codex because the Rust execution bridge is not ported yet."
    }
}
