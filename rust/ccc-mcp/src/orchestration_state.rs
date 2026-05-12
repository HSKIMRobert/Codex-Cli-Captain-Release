use serde_json::{json, Value};
use std::io;

pub(crate) struct OrchestratorStateUpdateInput<'a> {
    pub(crate) next_step_after_attempt: &'a str,
    pub(crate) can_advance_after_attempt: bool,
    pub(crate) summary: &'a str,
    pub(crate) launch_result: Option<&'a Value>,
    pub(crate) codex_bin: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct RunRecordUpdateInput<'a> {
    pub(crate) timestamp: &'a str,
    pub(crate) summary: &'a str,
    pub(crate) attempt_id: &'a str,
    pub(crate) requested_progression_mode: &'a str,
    pub(crate) current_next_step: &'a str,
    pub(crate) codex_bin: &'a str,
    pub(crate) resolved_run: bool,
    pub(crate) follow_up_or_retry: bool,
    pub(crate) reclaimed_worker: bool,
    pub(crate) collapsed_worker_fan_in: bool,
    pub(crate) dispatched_execution: bool,
    pub(crate) effective_task_card: &'a Value,
    pub(crate) launch_result: Option<&'a Value>,
    pub(crate) collapsed_fan_in: Option<&'a Value>,
}

pub(crate) fn apply_orchestrator_state_after_attempt(
    orchestrator_state: &mut Value,
    input: OrchestratorStateUpdateInput<'_>,
) -> io::Result<()> {
    let orchestrator_object = orchestrator_state.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "orchestrator-state.json must be an object.",
        )
    })?;
    orchestrator_object.insert(
        "decision".to_string(),
        json!({
            "next_step": input.next_step_after_attempt,
            "can_advance": input.can_advance_after_attempt,
            "summary": input.summary
        }),
    );
    if let Some(launch) = input.launch_result {
        orchestrator_object.insert(
            "execution_request".to_string(),
            json!({
                "entrypoint": "ccc_orchestrate",
                "codex_bin": input.codex_bin,
                "requested_at": input.timestamp,
                "launch_result": launch,
            }),
        );
    }

    Ok(())
}

pub(crate) fn apply_run_record_after_attempt(
    run_record: &mut Value,
    input: RunRecordUpdateInput<'_>,
) -> io::Result<()> {
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert(
        "updated_at".to_string(),
        Value::String(input.timestamp.to_string()),
    );

    let captain_owned_after_attempt = input.resolved_run
        || input.follow_up_or_retry
        || input.reclaimed_worker
        || input.collapsed_worker_fan_in;
    run_object.insert(
        "active_role".to_string(),
        if captain_owned_after_attempt {
            Value::String("orchestrator".to_string())
        } else if input.dispatched_execution {
            input
                .effective_task_card
                .get("assigned_role")
                .cloned()
                .unwrap_or(Value::String("code specialist".to_string()))
        } else {
            run_object
                .get("active_role")
                .cloned()
                .unwrap_or(Value::Null)
        },
    );
    run_object.insert(
        "active_agent_id".to_string(),
        if captain_owned_after_attempt {
            Value::String("captain".to_string())
        } else if input.dispatched_execution {
            input
                .effective_task_card
                .get("assigned_agent_id")
                .cloned()
                .unwrap_or(Value::String("raider".to_string()))
        } else {
            run_object
                .get("active_agent_id")
                .cloned()
                .unwrap_or(Value::Null)
        },
    );
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String(input.summary.to_string()),
    );
    run_object.insert(
        "latest_entry_trace".to_string(),
        json!({
            "entrypoint": "ccc_orchestrate",
            "attempt_id": input.attempt_id,
            "requested_progression_mode": input.requested_progression_mode,
            "current_next_step": input.current_next_step,
            "codex_bin": input.codex_bin,
            "completed_at": input.timestamp,
        }),
    );

    if let Some(launch) = input.launch_result {
        append_launch_to_run_record(run_object, input.effective_task_card, launch);
    }
    if input.reclaimed_worker {
        run_object.insert(
            "latest_failure".to_string(),
            json!({
                "stage": "execution",
                "reason": "timeout",
                "summary": input.summary,
                "recorded_at": input.timestamp,
            }),
        );
    }
    if let Some(collapsed) = input.collapsed_fan_in.and_then(Value::as_object) {
        if let Some(thread_ids) = collapsed.get("thread_ids").and_then(Value::as_array) {
            merge_collapsed_thread_ids(run_object, thread_ids);
        }
    }

    Ok(())
}

pub(crate) fn apply_run_state_after_attempt(
    run_state_record: &mut Value,
    timestamp: &str,
    next_step_after_attempt: &str,
    current_phase_name: &str,
) -> io::Result<()> {
    let run_state_object = run_state_record.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({
            "command": next_step_after_attempt
        }),
    );
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(current_phase_name.to_string()),
    );

    Ok(())
}

fn append_launch_to_run_record(
    run_object: &mut serde_json::Map<String, Value>,
    effective_task_card: &Value,
    launch: &Value,
) {
    let child_agents = run_object
        .get("child_agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let specialist_executors = run_object
        .get("specialist_executors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    run_object.insert(
        "child_agents".to_string(),
        Value::Array(
            child_agents
                .into_iter()
                .chain(std::iter::once(json!({
                    "agent_id": launch.get("child_agent_id").cloned().unwrap_or(Value::Null),
                    "parent_agent_id": "captain",
                    "role": launch.get("assigned_role").cloned().unwrap_or(Value::Null),
                    "status": "running",
                    "task_card_id": effective_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                })))
                .collect(),
        ),
    );
    run_object.insert(
        "specialist_executors".to_string(),
        Value::Array(
            specialist_executors
                .into_iter()
                .chain(std::iter::once(json!({
                    "executor_id": format!("specialist-executor:{}", launch.get("child_agent_id").and_then(Value::as_str).unwrap_or("worker")),
                    "status": "running",
                    "task_card_id": effective_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                    "delegation_id": launch.get("delegation_id").cloned().unwrap_or(Value::Null),
                    "child_agent_id": launch.get("child_agent_id").cloned().unwrap_or(Value::Null),
                })))
                .collect(),
        ),
    );
}

fn merge_collapsed_thread_ids(
    run_object: &mut serde_json::Map<String, Value>,
    thread_ids: &[Value],
) {
    let mut raw_thread_ids = run_object
        .get("raw_thread_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for thread_id in thread_ids {
        if !raw_thread_ids.iter().any(|existing| existing == thread_id) {
            raw_thread_ids.push(thread_id.clone());
        }
    }
    run_object.insert(
        "raw_thread_ids".to_string(),
        Value::Array(raw_thread_ids.clone()),
    );
    if run_object
        .get("active_thread_id")
        .unwrap_or(&Value::Null)
        .is_null()
    {
        if let Some(first_thread_id) = raw_thread_ids.first() {
            run_object.insert("active_thread_id".to_string(), first_thread_id.clone());
        }
    }
}
