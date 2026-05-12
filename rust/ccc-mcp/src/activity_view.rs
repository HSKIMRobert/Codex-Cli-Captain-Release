use crate::{
    build_captain_intervention_line, create_ccc_status_payload, create_worker_lifecycle_view,
    load_runtime_config, read_json_document, refresh_running_delegation_heartbeat,
    ResolvedRunLocator, SessionContext,
};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;

fn create_latest_orchestration_attempt_payload(run_directory: &Path) -> io::Result<Value> {
    let attempts_directory = run_directory.join("orchestration").join("attempts");
    let entries = match fs::read_dir(&attempts_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Value::Null),
        Err(error) => return Err(error),
    };

    let mut files = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    files.sort();

    let Some(latest_file) = files.last() else {
        return Ok(Value::Null);
    };

    let attempt = read_json_document(latest_file)?;
    let step_count = attempt
        .get("steps")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let stop_reason = attempt
        .get("stop")
        .and_then(|value| value.get("reason"))
        .cloned()
        .unwrap_or(Value::Null);
    let step_summaries = attempt
        .get("steps")
        .and_then(Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .map(|step| {
                    json!({
                        "step_number": step.get("step_number").cloned().unwrap_or(Value::Null),
                        "command": step.get("command").cloned().unwrap_or(Value::Null),
                        "before_status": step.get("before").and_then(|value| value.get("status")).cloned().unwrap_or(Value::Null),
                        "before_stage": step.get("before").and_then(|value| value.get("stage")).cloned().unwrap_or(Value::Null),
                        "after_status": step.get("after").and_then(|value| value.get("status")).cloned().unwrap_or(Value::Null),
                        "after_stage": step.get("after").and_then(|value| value.get("stage")).cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(json!({
        "file": latest_file.to_string_lossy(),
        "attempt_id": attempt.get("attempt_id").cloned().unwrap_or(Value::Null),
        "entrypoint": attempt.get("entrypoint").cloned().unwrap_or(Value::Null),
        "started_at": attempt.get("started_at").cloned().unwrap_or(Value::Null),
        "completed_at": attempt.get("completed_at").cloned().unwrap_or(Value::Null),
        "step_count": step_count,
        "stop_reason": stop_reason,
        "summary": format!(
            "latest attempt {} entrypoint={} steps={} stop={}",
            attempt.get("attempt_id").and_then(Value::as_str).unwrap_or("unknown"),
            attempt.get("entrypoint").and_then(Value::as_str).unwrap_or("unknown"),
            step_count,
            attempt.get("stop").and_then(|value| value.get("reason")).and_then(Value::as_str).unwrap_or("in_progress")
        ),
        "steps": step_summaries,
    }))
}

fn create_active_task_delegations_payload(
    run_directory: &Path,
    active_task_card_id: Option<&str>,
) -> io::Result<Value> {
    let Some(task_card_id) = active_task_card_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(Value::Null);
    };
    let delegations_directory = run_directory.join("delegations");
    let runtime_config = load_runtime_config()?;
    let entries = match fs::read_dir(&delegations_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Value::Null),
        Err(error) => return Err(error),
    };

    let mut total = 0_usize;
    let mut queued = 0_usize;
    let mut running = 0_usize;
    let mut completed = 0_usize;
    let mut failed = 0_usize;
    let mut cancelled = 0_usize;
    let mut active = Vec::new();

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

        total += 1;
        let child_status = delegation
            .get("child_agent")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        match child_status {
            "queued" => queued += 1,
            "running" => running += 1,
            "completed" => completed += 1,
            "failed" => failed += 1,
            "cancelled" => cancelled += 1,
            _ => {}
        }

        if child_status == "queued" || child_status == "running" {
            active.push(json!({
                "delegation_id": delegation.get("delegation_id").cloned().unwrap_or(Value::Null),
                "delegated_by_role": delegation.get("delegated_by_role").cloned().unwrap_or(Value::Null),
                "summary": delegation.get("summary").cloned().unwrap_or(Value::String("bounded delegation is active".to_string())),
                "child_agent": delegation.get("child_agent").cloned().unwrap_or(Value::Null),
                "executor": delegation.get("executor").cloned().unwrap_or(Value::Null),
                "worker_lifecycle": create_worker_lifecycle_view(&delegation, &runtime_config),
                "updated_at": delegation.get("updated_at").cloned().unwrap_or(Value::Null),
            }));
        }
    }

    Ok(json!({
        "task_card_id": task_card_id,
        "total": total,
        "queued": queued,
        "running": running,
        "completed": completed,
        "failed": failed,
        "cancelled": cancelled,
        "active": active,
    }))
}

fn create_activity_checkpoint_summary(
    status_payload: &Value,
    latest_attempt: &Value,
    active_task_delegations: &Value,
) -> String {
    let next_action = status_payload
        .get("run_state")
        .and_then(|value| value.get("next_action"))
        .and_then(|value| {
            value
                .get("command")
                .or_else(|| value.get("action"))
                .or_else(|| value.get("type"))
        })
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let active_task_title = status_payload
        .get("current_task_card")
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("no active task");
    let attempt_summary = latest_attempt
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("no orchestration attempt recorded");
    let active_count = active_task_delegations
        .get("active")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let reclaim_needed = status_payload
        .get("reclaim_plan")
        .and_then(|value| value.get("reclaim_needed_worker_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let intervention_action = status_payload
        .get("latest_captain_intervention")
        .and_then(|value| value.get("chosen_next_action"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let pending_follow_up = status_payload
        .get("pending_captain_follow_up")
        .and_then(|value| value.get("action"))
        .and_then(Value::as_str)
        .map(|action| {
            let status = status_payload
                .get("pending_captain_follow_up")
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("queued");
            if status == "queued" {
                action.to_string()
            } else {
                format!("{action}:{status}")
            }
        })
        .unwrap_or_else(|| "none".to_string());

    format!(
        "Captain checkpoint: task=\"{active_task_title}\" next={next_action}; {attempt_summary}; active_delegations={active_count}; reclaim_needed={reclaim_needed}; intervention_next={intervention_action}; pending_follow_up={pending_follow_up}"
    )
}

pub(crate) fn create_ccc_activity_payload(
    session_context: &SessionContext,
    locator: &ResolvedRunLocator,
) -> io::Result<Value> {
    let mut payload = create_ccc_status_payload(session_context, locator)?;
    let latest_orchestration_attempt =
        create_latest_orchestration_attempt_payload(&locator.run_directory)?;
    let active_task_delegations = create_active_task_delegations_payload(
        &locator.run_directory,
        payload.get("active_task_card_id").and_then(Value::as_str),
    )?;
    let checkpoint_summary = create_activity_checkpoint_summary(
        &payload,
        &latest_orchestration_attempt,
        &active_task_delegations,
    );

    let map = payload.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "status payload must be an object",
        )
    })?;
    map.insert(
        "latest_orchestration_attempt".to_string(),
        latest_orchestration_attempt,
    );
    map.insert(
        "active_task_delegations".to_string(),
        active_task_delegations,
    );
    map.insert(
        "checkpoint_summary".to_string(),
        Value::String(checkpoint_summary),
    );

    Ok(payload)
}

pub(crate) fn create_ccc_activity_text(payload: &Value) -> String {
    let run_id = payload
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown-run");
    let summary = payload
        .get("checkpoint_summary")
        .and_then(Value::as_str)
        .unwrap_or("Captain checkpoint summary unavailable.");
    let total_tokens = payload
        .get("token_usage")
        .and_then(|value| value.get("total_tokens"))
        .and_then(Value::as_u64);
    let active_agents = payload
        .get("active_task_delegations")
        .and_then(|value| value.get("active"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("child_agent")
                        .and_then(|child| child.get("agent_id"))
                        .and_then(Value::as_str)
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let mut lines = vec![format!("Activity {run_id}"), summary.to_string()];
    if let Some(total_tokens) = total_tokens {
        lines.push(format!("Tokens: {total_tokens} used"));
    }
    if !active_agents.is_empty() {
        lines.push(format!("Active delegates: {active_agents}"));
    }
    if let Some(intervention_line) = build_captain_intervention_line(payload) {
        lines.push(intervention_line);
    }
    lines.join("\n")
}
