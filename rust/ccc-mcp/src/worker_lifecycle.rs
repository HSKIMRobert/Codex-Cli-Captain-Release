use crate::worker_events::{
    build_worker_completion_snapshot, extract_worker_terminal_event_from_raw_events,
    resolve_delegation_raw_events_file,
};
use crate::{read_json_document, write_json_document};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime};

fn parse_timestamp_millis(value: Option<&Value>) -> Option<i64> {
    let text = value.and_then(Value::as_str)?;
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn system_time_to_rfc3339(value: SystemTime) -> String {
    let timestamp: chrono::DateTime<Utc> = value.into();
    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn finalize_delegation_with_completion(
    delegation: &mut Value,
    completed_at: &str,
    status: &str,
    summary: &str,
    worker_result: Value,
    latest_failure: Value,
) -> io::Result<()> {
    let object = delegation.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "delegation artifact must be an object.",
        )
    })?;
    object.insert(
        "updated_at".to_string(),
        Value::String(completed_at.to_string()),
    );
    object.insert(
        "completed_at".to_string(),
        Value::String(completed_at.to_string()),
    );
    object.insert(
        "result_summary".to_string(),
        Value::String(summary.to_string()),
    );
    object.insert("worker_result".to_string(), worker_result);
    object.insert("latest_failure".to_string(), latest_failure);
    if let Some(child_agent) = object.get_mut("child_agent").and_then(Value::as_object_mut) {
        child_agent.insert("status".to_string(), Value::String(status.to_string()));
    }
    if let Some(executor) = object.get_mut("executor").and_then(Value::as_object_mut) {
        executor.insert("status".to_string(), Value::String(status.to_string()));
    }
    if let Some(lifecycle) = object
        .get_mut("worker_lifecycle")
        .and_then(Value::as_object_mut)
    {
        lifecycle.insert(
            "state".to_string(),
            Value::String(if status == "completed" {
                "returned".to_string()
            } else {
                "failed".to_string()
            }),
        );
        lifecycle.insert(
            "reclaim_state".to_string(),
            Value::String(if status == "completed" {
                "resumable".to_string()
            } else {
                "not_needed".to_string()
            }),
        );
        lifecycle.insert(
            "process_last_seen_at".to_string(),
            Value::String(completed_at.to_string()),
        );
        lifecycle.insert(
            "returned_at".to_string(),
            Value::String(completed_at.to_string()),
        );
        lifecycle.insert(
            "last_progress_at".to_string(),
            Value::String(completed_at.to_string()),
        );
        lifecycle.insert("summary".to_string(), Value::String(summary.to_string()));
    }
    Ok(())
}

pub(crate) fn refresh_running_delegation_heartbeat(
    run_directory: &Path,
    path: &Path,
    delegation: Value,
) -> io::Result<Value> {
    let child_status = delegation
        .get("child_agent")
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("queued");
    if child_status != "running" {
        return Ok(delegation);
    }

    let Some(raw_events_file) = resolve_delegation_raw_events_file(run_directory, &delegation)
    else {
        return Ok(delegation);
    };
    let process_id = delegation
        .get("worker_lifecycle")
        .and_then(|value| value.get("process_id"))
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    let process_alive = process_id.map(is_process_alive);
    if raw_events_file.exists()
        && extract_worker_terminal_event_from_raw_events(&raw_events_file).is_some()
        && process_alive != Some(true)
    {
        let completed_at = fs::metadata(&raw_events_file)
            .and_then(|metadata| metadata.modified())
            .map(system_time_to_rfc3339)
            .unwrap_or_else(|_| Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true));
        let mut updated = delegation;
        let (status, summary, worker_result, latest_failure) =
            build_worker_completion_snapshot(&raw_events_file, &completed_at, None);
        finalize_delegation_with_completion(
            &mut updated,
            &completed_at,
            &status,
            &summary,
            worker_result,
            latest_failure,
        )?;
        write_json_document(path, &updated)?;
        return Ok(updated);
    }
    if process_id.is_some() {
        if process_alive == Some(false) && raw_events_file.exists() {
            let completed_at = fs::metadata(&raw_events_file)
                .and_then(|metadata| metadata.modified())
                .map(system_time_to_rfc3339)
                .unwrap_or_else(|_| Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true));
            let mut updated = delegation;
            let (status, summary, worker_result, latest_failure) =
                build_worker_completion_snapshot(&raw_events_file, &completed_at, None);
            finalize_delegation_with_completion(
                &mut updated,
                &completed_at,
                &status,
                &summary,
                worker_result,
                latest_failure,
            )?;
            write_json_document(path, &updated)?;
            return Ok(updated);
        }
    }
    let metadata = match fs::metadata(&raw_events_file) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(delegation),
        Err(error) => return Err(error),
    };
    let modified_at = metadata.modified()?;
    let modified_at_text = system_time_to_rfc3339(modified_at);
    let modified_at_ms = chrono::DateTime::parse_from_rfc3339(&modified_at_text)
        .map(|value| value.timestamp_millis())
        .unwrap_or_default();

    let current_progress_ms = parse_timestamp_millis(
        delegation
            .get("worker_lifecycle")
            .and_then(|value| value.get("last_progress_at")),
    )
    .unwrap_or_default();

    if modified_at_ms <= current_progress_ms {
        return Ok(delegation);
    }

    let mut updated = delegation;
    let lifecycle = updated
        .get_mut("worker_lifecycle")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "delegation worker_lifecycle must be an object.",
            )
        })?;
    lifecycle.insert(
        "last_progress_at".to_string(),
        Value::String(modified_at_text.clone()),
    );
    lifecycle.insert(
        "process_last_seen_at".to_string(),
        Value::String(modified_at_text.clone()),
    );
    lifecycle.insert(
        "state".to_string(),
        Value::String("running_active".to_string()),
    );
    if lifecycle.get("started_at").is_none() || lifecycle.get("started_at") == Some(&Value::Null) {
        lifecycle.insert(
            "started_at".to_string(),
            Value::String(modified_at_text.clone()),
        );
    }
    if lifecycle.get("process_started_at").is_none()
        || lifecycle.get("process_started_at") == Some(&Value::Null)
    {
        lifecycle.insert(
            "process_started_at".to_string(),
            Value::String(modified_at_text.clone()),
        );
    }
    if lifecycle.get("launch_requested_at").is_none()
        || lifecycle.get("launch_requested_at") == Some(&Value::Null)
    {
        lifecycle.insert(
            "launch_requested_at".to_string(),
            Value::String(modified_at_text.clone()),
        );
    }
    if let Some(updated_at) = updated.as_object_mut() {
        updated_at.insert("updated_at".to_string(), Value::String(modified_at_text));
    }
    write_json_document(path, &updated)?;
    Ok(updated)
}

pub(crate) fn collapse_terminal_fan_in(
    run_directory: &Path,
    task_card: &Value,
    orchestrator_summary: &str,
) -> io::Result<Option<Value>> {
    let task_card_id = match task_card.get("task_card_id").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.to_string(),
        _ => return Ok(None),
    };
    let delegations_directory = run_directory.join("delegations");
    let entries = match fs::read_dir(&delegations_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(collapse_task_card_terminal_fan_in(
                task_card,
                &task_card_id,
                orchestrator_summary,
            ));
        }
        Err(error) => return Err(error),
    };

    let mut matching_paths = Vec::new();
    let mut completed = 0_usize;
    let mut failed = 0_usize;
    let mut cancelled = 0_usize;
    let mut active = 0_usize;
    let mut collapsed_thread_ids = Vec::new();

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
        if delegation.get("task_card_id").and_then(Value::as_str) != Some(task_card_id.as_str()) {
            continue;
        }
        matching_paths.push((path, delegation.clone()));
        match delegation
            .get("child_agent")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("queued")
        {
            "completed" => {
                completed += 1;
                if let Some(thread_id) = delegation
                    .get("worker_result")
                    .and_then(|value| value.get("thread_id"))
                    .and_then(Value::as_str)
                {
                    collapsed_thread_ids.push(thread_id.to_string());
                }
            }
            "failed" => failed += 1,
            "cancelled" => cancelled += 1,
            _ => active += 1,
        }
    }

    if matching_paths.is_empty() {
        return Ok(collapse_task_card_terminal_fan_in(
            task_card,
            &task_card_id,
            orchestrator_summary,
        ));
    }

    if active > 0 {
        return Ok(None);
    }

    let collapsed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    for (path, mut delegation) in matching_paths {
        if let Some(object) = delegation.as_object_mut() {
            object.insert(
                "updated_at".to_string(),
                Value::String(collapsed_at.clone()),
            );
            object.insert(
                "fan_in_collapsed_at".to_string(),
                Value::String(collapsed_at.clone()),
            );
        }
        write_json_document(&path, &delegation)?;
    }

    Ok(Some(json!({
        "collapsed_at": collapsed_at,
        "task_card_id": task_card_id,
        "completed": completed,
        "failed": failed,
        "cancelled": cancelled,
        "summary": if failed > 0 || cancelled > 0 {
            format!("Captain collapsed explicit fan-in after terminal worker results ({completed} completed, {failed} failed, {cancelled} cancelled). {orchestrator_summary}")
        } else {
            format!("Captain collapsed explicit fan-in after {completed} completed worker result(s). {orchestrator_summary}")
        },
        "thread_ids": collapsed_thread_ids,
    })))
}

fn collapse_task_card_terminal_fan_in(
    task_card: &Value,
    task_card_id: &str,
    orchestrator_summary: &str,
) -> Option<Value> {
    let envelope = task_card
        .get("worker_result_envelope")
        .or_else(|| task_card.get("subagent_fan_in"))?;
    let status = envelope.get("status").and_then(Value::as_str)?;
    let normalized_status = status.trim().to_ascii_lowercase();
    let (completed, failed, cancelled) = match normalized_status.as_str() {
        "completed" | "passed" | "success" => (1, 0, 0),
        "failed" | "error" => (0, 1, 0),
        "cancelled" | "canceled" => (0, 0, 1),
        _ => return None,
    };
    let collapsed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let summary = envelope
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Task card terminal fan-in is ready.");

    Some(json!({
        "collapsed_at": collapsed_at,
        "task_card_id": task_card_id,
        "completed": completed,
        "failed": failed,
        "cancelled": cancelled,
        "summary": if failed > 0 || cancelled > 0 {
            format!("Captain collapsed task-card fan-in after terminal worker result ({completed} completed, {failed} failed, {cancelled} cancelled). {summary} {orchestrator_summary}")
        } else {
            format!("Captain collapsed task-card fan-in after {completed} completed worker result(s). {summary} {orchestrator_summary}")
        },
        "thread_ids": [],
    }))
}

fn is_process_alive(process_id: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(process_id.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn terminate_worker_process(process_id: u32, grace_ms: u64) -> &'static str {
    if !is_process_alive(process_id) {
        return "already_exited";
    }

    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(process_id.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let deadline = SystemTime::now() + Duration::from_millis(grace_ms);

    while SystemTime::now() < deadline {
        if !is_process_alive(process_id) {
            return "terminated";
        }
        thread::sleep(Duration::from_millis(50));
    }

    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(process_id.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let kill_deadline = SystemTime::now() + Duration::from_millis(500);
    while SystemTime::now() < kill_deadline {
        if !is_process_alive(process_id) {
            return "killed";
        }
        thread::sleep(Duration::from_millis(50));
    }

    "killed"
}

pub(crate) fn reclaim_stuck_worker_delegations(
    run_directory: &Path,
    active_task_card_id: Option<&str>,
    runtime_config: &Value,
) -> io::Result<Vec<Value>> {
    let Some(task_card_id) = active_task_card_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    let delegations_directory = run_directory.join("delegations");
    let entries = match fs::read_dir(&delegations_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let grace_ms = runtime_config
        .get("worker_kill_grace_ms")
        .and_then(Value::as_u64)
        .unwrap_or(2_000);
    let mut reclaimed = Vec::new();

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

        let lifecycle_view = create_worker_lifecycle_view(&delegation, runtime_config);
        let reclaim_state = lifecycle_view
            .get("reclaim_state")
            .and_then(Value::as_str)
            .unwrap_or("not_needed");
        if reclaim_state != "reclaim_needed" {
            continue;
        }

        let state = lifecycle_view
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("stale");
        let process_id = delegation
            .get("worker_lifecycle")
            .and_then(|value| value.get("process_id"))
            .and_then(Value::as_u64)
            .map(|value| value as u32);
        let termination = process_id
            .map(|pid| terminate_worker_process(pid, grace_ms))
            .unwrap_or("already_exited");
        let reclaimed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let failure_summary = if state == "timed_out" {
            format!(
                "Worker process {} exceeded the bounded timeout and was reclaimed by Rust CCC ({termination}).",
                process_id.map(|pid| pid.to_string()).unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            format!(
                "Worker process {} stopped making bounded progress and was reclaimed by Rust CCC ({termination}).",
                process_id.map(|pid| pid.to_string()).unwrap_or_else(|| "unknown".to_string())
            )
        };

        let mut updated = delegation;
        let updated_object = updated.as_object_mut().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "delegation artifact must be an object.",
            )
        })?;
        updated_object.insert(
            "updated_at".to_string(),
            Value::String(reclaimed_at.clone()),
        );
        updated_object.insert(
            "completed_at".to_string(),
            Value::String(reclaimed_at.clone()),
        );
        updated_object.insert(
            "result_summary".to_string(),
            Value::String(failure_summary.clone()),
        );
        updated_object.insert(
            "latest_failure".to_string(),
            json!({
                "stage": "execution",
                "reason": "timeout",
                "summary": failure_summary,
                "recorded_at": reclaimed_at,
            }),
        );
        if let Some(child_agent) = updated_object
            .get_mut("child_agent")
            .and_then(Value::as_object_mut)
        {
            child_agent.insert("status".to_string(), Value::String("failed".to_string()));
        }
        if let Some(executor) = updated_object
            .get_mut("executor")
            .and_then(Value::as_object_mut)
        {
            executor.insert("status".to_string(), Value::String("failed".to_string()));
        }
        if let Some(lifecycle) = updated_object
            .get_mut("worker_lifecycle")
            .and_then(Value::as_object_mut)
        {
            lifecycle.insert("state".to_string(), Value::String(state.to_string()));
            lifecycle.insert(
                "reclaim_state".to_string(),
                Value::String("reclaimed".to_string()),
            );
            lifecycle.insert(
                "process_last_seen_at".to_string(),
                Value::String(reclaimed_at.clone()),
            );
            lifecycle.insert(
                "returned_at".to_string(),
                Value::String(reclaimed_at.clone()),
            );
            lifecycle.insert(
                "summary".to_string(),
                Value::String(failure_summary.clone()),
            );
            if state == "stale"
                && (lifecycle.get("stale_at").is_none()
                    || lifecycle.get("stale_at") == Some(&Value::Null))
            {
                lifecycle.insert("stale_at".to_string(), Value::String(reclaimed_at.clone()));
            }
            if state == "timed_out"
                && (lifecycle.get("timed_out_at").is_none()
                    || lifecycle.get("timed_out_at") == Some(&Value::Null))
            {
                lifecycle.insert(
                    "timed_out_at".to_string(),
                    Value::String(reclaimed_at.clone()),
                );
            }
        }
        write_json_document(&path, &updated)?;
        reclaimed.push(json!({
            "delegation_id": updated.get("delegation_id").cloned().unwrap_or(Value::Null),
            "state": state,
            "process_id": process_id.map(|value| value as u64),
            "termination": termination,
            "summary": failure_summary,
        }));
    }

    Ok(reclaimed)
}

pub(crate) fn task_has_active_worker_delegation(
    run_directory: &Path,
    active_task_card_id: Option<&str>,
) -> io::Result<bool> {
    let Some(task_card_id) = active_task_card_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(false);
    };
    let delegations_directory = run_directory.join("delegations");
    let entries = match fs::read_dir(&delegations_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };

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
            .unwrap_or("queued");
        if !matches!(status, "completed" | "failed" | "cancelled") {
            return Ok(true);
        }
    }

    Ok(false)
}

fn summarize_visible_worker_lifecycle_state(state: &str) -> &'static str {
    match state {
        "queued" => "queued and waiting for captain launch",
        "launching" => "launch requested and waiting for worker start proof",
        "running" | "running_active" => "running with recent bounded progress",
        "running_quiet" => "running without recent event output but still alive",
        "returned" => "returned to captain and ready for explicit fan-in",
        "failed" => "failed and needs captain follow-up",
        "cancelled" => "cancelled under captain control",
        "stale" => "stale and needs bounded reclaim",
        "timed_out" => "timed out and needs bounded reclaim",
        _ => "worker lifecycle is unavailable",
    }
}

pub(crate) fn create_worker_lifecycle_view(delegation: &Value, runtime_config: &Value) -> Value {
    let lifecycle = delegation
        .get("worker_lifecycle")
        .cloned()
        .unwrap_or(Value::Null);
    if lifecycle.is_null() {
        return Value::Null;
    }

    let child_status = delegation
        .get("child_agent")
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("queued");
    let now_ms = Utc::now().timestamp_millis();
    let progress_at_ms = parse_timestamp_millis(lifecycle.get("last_progress_at"));
    let elapsed_since_progress_ms = progress_at_ms.map(|value| (now_ms - value).max(0));
    let configured_stuck_after_ms = runtime_config
        .get("worker_stuck_after_ms")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0);
    let lifecycle_stale_after_ms = lifecycle
        .get("stale_after_ms")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .unwrap_or(45_000);
    let lifecycle_timeout_after_ms = lifecycle
        .get("timeout_after_ms")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .unwrap_or(lifecycle_stale_after_ms);
    let effective_stale_threshold_ms = configured_stuck_after_ms
        .map(|configured| configured.min(lifecycle_stale_after_ms))
        .unwrap_or(lifecycle_stale_after_ms);
    let effective_timeout_threshold_ms = configured_stuck_after_ms
        .map(|configured| configured.min(lifecycle_timeout_after_ms))
        .unwrap_or(lifecycle_timeout_after_ms);

    let mut state = lifecycle
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("queued")
        .to_string();
    let mut reclaim_state = lifecycle
        .get("reclaim_state")
        .and_then(Value::as_str)
        .unwrap_or(if state == "returned" {
            "resumable"
        } else {
            "not_needed"
        })
        .to_string();
    let mut stale_at = lifecycle.get("stale_at").cloned().unwrap_or(Value::Null);
    let mut timed_out_at = lifecycle
        .get("timed_out_at")
        .cloned()
        .unwrap_or(Value::Null);

    if child_status == "running" {
        if let Some(elapsed) = elapsed_since_progress_ms {
            if elapsed >= effective_timeout_threshold_ms {
                state = "timed_out".to_string();
                reclaim_state = "reclaim_needed".to_string();
                if timed_out_at.is_null() {
                    timed_out_at = lifecycle
                        .get("last_progress_at")
                        .cloned()
                        .unwrap_or(Value::Null);
                }
            } else if elapsed >= effective_stale_threshold_ms {
                state = "stale".to_string();
                reclaim_state = "reclaim_needed".to_string();
                if stale_at.is_null() {
                    stale_at = lifecycle
                        .get("last_progress_at")
                        .cloned()
                        .unwrap_or(Value::Null);
                }
            } else if lifecycle.get("launch_requested_at").is_some()
                && lifecycle
                    .get("started_at")
                    .unwrap_or(&Value::Null)
                    .is_null()
            {
                state = "launching".to_string();
            } else if state != "running_active" {
                state = "running_quiet".to_string();
            } else {
                state = "running_active".to_string();
            }
        }
    } else if child_status == "completed" {
        state = "returned".to_string();
        if reclaim_state != "reclaimed" {
            reclaim_state = "resumable".to_string();
        }
    } else if child_status == "failed" {
        if reclaim_state != "reclaimed" {
            state = "failed".to_string();
            reclaim_state = "not_needed".to_string();
        }
    } else if child_status == "cancelled" {
        state = "cancelled".to_string();
        reclaim_state = "not_needed".to_string();
    }

    json!({
        "state": state,
        "reclaim_state": reclaim_state,
        "queued_at": lifecycle.get("queued_at").cloned().unwrap_or(Value::Null),
        "launch_requested_at": lifecycle.get("launch_requested_at").cloned().unwrap_or(Value::Null),
        "started_at": lifecycle.get("started_at").cloned().unwrap_or(Value::Null),
        "process_id": lifecycle.get("process_id").cloned().unwrap_or(Value::Null),
        "process_started_at": lifecycle.get("process_started_at").cloned().unwrap_or(Value::Null),
        "process_last_seen_at": lifecycle.get("process_last_seen_at").cloned().unwrap_or(Value::Null),
        "process_alive": Value::Null,
        "last_progress_at": lifecycle.get("last_progress_at").cloned().unwrap_or(Value::Null),
        "returned_at": lifecycle.get("returned_at").cloned().unwrap_or(Value::Null),
        "stale_at": stale_at,
        "timed_out_at": timed_out_at,
        "stale_after_ms": effective_stale_threshold_ms,
        "timeout_after_ms": effective_timeout_threshold_ms,
        "elapsed_since_progress_ms": elapsed_since_progress_ms,
        "summary": summarize_visible_worker_lifecycle_state(&state),
    })
}

pub(crate) fn create_worker_visibility_payload(
    run_directory: &Path,
    active_task_card_id: Option<&str>,
    runtime_config: &Value,
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

    let mut workers = Vec::new();
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

        let lifecycle = create_worker_lifecycle_view(&delegation, runtime_config);
        workers.push(json!({
            "delegation_id": delegation.get("delegation_id").cloned().unwrap_or(Value::Null),
            "child_agent": delegation.get("child_agent").cloned().unwrap_or(Value::Null),
            "summary": delegation.get("summary").cloned().unwrap_or(Value::Null),
            "worker_lifecycle": lifecycle,
            "status": delegation.get("child_agent").and_then(|value| value.get("status")).cloned().unwrap_or(Value::Null),
        }));
    }

    let total_worker_count = workers.len();
    let active_workers = workers
        .iter()
        .filter(|worker| {
            matches!(
                worker
                    .get("worker_lifecycle")
                    .and_then(|value| value.get("state"))
                    .and_then(Value::as_str),
                Some(
                    "queued"
                        | "launching"
                        | "running"
                        | "running_active"
                        | "running_quiet"
                        | "stale"
                        | "timed_out"
                )
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let count_lifecycle_state = |state: &str| -> usize {
        workers
            .iter()
            .filter(|worker| {
                worker
                    .get("worker_lifecycle")
                    .and_then(|value| value.get("state"))
                    .and_then(Value::as_str)
                    == Some(state)
            })
            .count()
    };

    Ok(json!({
        "task_card_id": task_card_id,
        "total_worker_count": total_worker_count,
        "active_worker_count": active_workers.len(),
        "queued_worker_count": count_lifecycle_state("queued"),
        "launching_worker_count": count_lifecycle_state("launching"),
        "running_worker_count": count_lifecycle_state("running") + count_lifecycle_state("running_active"),
        "running_quiet_worker_count": count_lifecycle_state("running_quiet"),
        "returned_worker_count": count_lifecycle_state("returned"),
        "completed_worker_count": workers.iter().filter(|worker| worker.get("status").and_then(Value::as_str) == Some("completed")).count(),
        "failed_worker_count": workers.iter().filter(|worker| worker.get("status").and_then(Value::as_str) == Some("failed")).count(),
        "cancelled_worker_count": workers.iter().filter(|worker| worker.get("status").and_then(Value::as_str) == Some("cancelled")).count(),
        "stale_worker_count": count_lifecycle_state("stale"),
        "timed_out_worker_count": count_lifecycle_state("timed_out"),
        "reclaim_needed_worker_count": workers.iter().filter(|worker| worker.get("worker_lifecycle").and_then(|value| value.get("reclaim_state")).and_then(Value::as_str) == Some("reclaim_needed")).count(),
        "workers": workers,
        "active_workers": active_workers,
    }))
}

pub(crate) fn create_reclaim_plan_payload(
    worker_visibility: &Value,
    runtime_config: &Value,
) -> Value {
    if worker_visibility.is_null() {
        return Value::Null;
    }

    let worker_auto_reclaim_enabled = runtime_config
        .get("worker_auto_reclaim_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let targets = worker_visibility
        .get("workers")
        .and_then(Value::as_array)
        .map(|workers| {
            workers
                .iter()
                .filter_map(|worker| {
                    let lifecycle = worker.get("worker_lifecycle")?;
                    let reclaim_state = lifecycle.get("reclaim_state").and_then(Value::as_str)?;
                    if reclaim_state != "reclaim_needed" {
                        return None;
                    }

                    Some(json!({
                        "delegation_id": worker.get("delegation_id").cloned().unwrap_or(Value::Null),
                        "state": lifecycle.get("state").cloned().unwrap_or(Value::Null),
                        "process_id": lifecycle.get("process_id").cloned().unwrap_or(Value::Null),
                        "last_progress_at": lifecycle.get("last_progress_at").cloned().unwrap_or(Value::Null),
                        "elapsed_since_progress_ms": lifecycle.get("elapsed_since_progress_ms").cloned().unwrap_or(Value::Null),
                        "summary": lifecycle.get("summary").cloned().unwrap_or(Value::Null),
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let reclaim_needed_worker_count = targets.len();
    let stale_worker_count = worker_visibility
        .get("stale_worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let timed_out_worker_count = worker_visibility
        .get("timed_out_worker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    json!({
        "worker_auto_reclaim_enabled": worker_auto_reclaim_enabled,
        "reclaim_needed_worker_count": reclaim_needed_worker_count,
        "targets": targets,
        "summary": if reclaim_needed_worker_count == 0 {
            String::from("No worker reclaim is currently needed.")
        } else if worker_auto_reclaim_enabled {
            format!(
                "{reclaim_needed_worker_count} worker(s) need reclaim according to persisted heartbeat truth ({stale_worker_count} stale, {timed_out_worker_count} timed out). Auto reclaim is enabled."
            )
        } else {
            format!(
                "{reclaim_needed_worker_count} worker(s) need reclaim according to persisted heartbeat truth ({stale_worker_count} stale, {timed_out_worker_count} timed out). Auto reclaim is disabled."
            )
        }
    })
}
