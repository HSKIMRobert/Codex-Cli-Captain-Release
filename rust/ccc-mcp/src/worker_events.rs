use crate::summarize_text_for_visibility;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

pub(crate) fn resolve_delegation_raw_events_file(
    run_directory: &Path,
    delegation: &Value,
) -> Option<PathBuf> {
    if let Some(path) = delegation
        .get("worker_launch_evidence")
        .and_then(|value| value.get("raw_events_file"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
    {
        return Some(path);
    }

    delegation
        .get("delegation_id")
        .and_then(Value::as_str)
        .map(|delegation_id| {
            run_directory
                .join("raw-events")
                .join(format!("{delegation_id}.jsonl"))
        })
}

fn extract_total_token_usage_from_raw_events(path: &Path) -> Option<Value> {
    extract_worker_artifacts_from_raw_events(path).and_then(|(_, _, usage)| usage)
}

fn extract_latest_agent_message_text_from_raw_events(path: &Path) -> Option<String> {
    extract_worker_artifacts_from_raw_events(path).and_then(|(_, latest_message, _)| latest_message)
}

fn extract_worker_artifacts_from_raw_events(
    path: &Path,
) -> Option<(Option<String>, Option<String>, Option<Value>)> {
    let content = fs::read_to_string(path).ok()?;
    let mut latest_thread_id = None;
    let mut latest_message = None;
    let mut latest = None;
    for line in content.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(thread_id) = value.get("thread_id").and_then(Value::as_str).or_else(|| {
            value
                .get("thread")
                .and_then(|thread| thread.get("id"))
                .and_then(Value::as_str)
        }) {
            let thread_id = thread_id.trim();
            if !thread_id.is_empty() {
                latest_thread_id = Some(thread_id.to_string());
            }
        }
        let text = value
            .get("item")
            .and_then(|item| {
                (item.get("type").and_then(Value::as_str) == Some("agent_message"))
                    .then(|| item.get("text").and_then(Value::as_str))
                    .flatten()
            })
            .or_else(|| {
                value.get("message").and_then(|message| {
                    (message.get("role").and_then(Value::as_str) == Some("assistant"))
                        .then(|| message.get("content").and_then(Value::as_str))
                        .flatten()
                })
            });
        if let Some(text) = text.map(str::trim).filter(|text| !text.is_empty()) {
            latest_message = Some(text.to_string());
        }
        let total_usage = value
            .get("payload")
            .and_then(|payload| payload.get("info"))
            .and_then(|info| info.get("total_token_usage"))
            .cloned()
            .or_else(|| value.get("usage").cloned())
            .or_else(|| {
                value
                    .get("response")
                    .and_then(|response| response.get("usage"))
                    .cloned()
            })
            .or_else(|| {
                value
                    .get("payload")
                    .and_then(|payload| payload.get("usage"))
                    .cloned()
            });
        if let Some(total_usage) = total_usage.filter(|value| value.is_object()) {
            latest = Some(total_usage);
        }
    }
    Some((latest_thread_id, latest_message, latest))
}

pub(crate) fn extract_worker_terminal_event_from_raw_events(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let mut latest_terminal_event = None;
    for line in content.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(event_type) = value.get("type").and_then(Value::as_str) else {
            continue;
        };
        if matches!(event_type, "turn.completed" | "turn.failed") {
            latest_terminal_event = Some(event_type.to_string());
        }
    }
    latest_terminal_event
}

fn raw_events_indicate_execution_failure(path: &Path) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    for line in content.lines() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if value.get("type").and_then(Value::as_str) == Some("error") {
                return true;
            }
        }
    }
    let normalized = content.to_ascii_lowercase();
    [
        " failed to connect ",
        " dns error",
        " stream disconnected",
        " transport channel closed",
        " error sending request",
        " websocket",
        " reconnecting...",
        "fatal:",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn extract_non_json_raw_events_preview(path: &Path, max_chars: usize) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| serde_json::from_str::<Value>(line).is_err())
        .take(4)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return None;
    }
    Some(summarize_text_for_visibility(&lines.join(" "), max_chars))
}

fn raw_events_size_bytes(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn create_process_exit_payload(exit_status: &ExitStatus) -> Value {
    json!({
        "success": exit_status.success(),
        "exit_code": exit_status.code(),
    })
}

pub(crate) fn resolve_delegation_token_usage(
    run_directory: &Path,
    delegation: &Value,
) -> Option<Value> {
    delegation
        .get("worker_result")
        .and_then(|value| value.get("total_token_usage"))
        .filter(|value| value.is_object())
        .cloned()
        .or_else(|| {
            resolve_delegation_raw_events_file(run_directory, delegation)
                .and_then(|path| extract_total_token_usage_from_raw_events(&path))
        })
}

pub(crate) fn resolve_delegation_message_preview(
    run_directory: &Path,
    delegation: &Value,
) -> Option<String> {
    delegation
        .get("worker_result")
        .and_then(|value| value.get("assistant_message_preview"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            resolve_delegation_raw_events_file(run_directory, delegation)
                .and_then(|path| extract_latest_agent_message_text_from_raw_events(&path))
                .map(|text| summarize_text_for_visibility(&text, 180))
        })
}

pub(crate) fn build_worker_completion_snapshot(
    raw_events_file: &Path,
    completed_at: &str,
    process_exit: Option<&ExitStatus>,
) -> (String, String, Value, Value) {
    let (thread_id, latest_message, total_token_usage) =
        extract_worker_artifacts_from_raw_events(raw_events_file).unwrap_or((None, None, None));
    let terminal_event = extract_worker_terminal_event_from_raw_events(raw_events_file);
    let assistant_message_preview =
        latest_message.map(|text| summarize_text_for_visibility(&text, 180));
    let raw_output_preview = extract_non_json_raw_events_preview(raw_events_file, 220);
    let raw_output_bytes = raw_events_size_bytes(raw_events_file);
    let execution_failure_observed = raw_events_indicate_execution_failure(raw_events_file);
    let process_exit_payload = process_exit
        .map(create_process_exit_payload)
        .unwrap_or(Value::Null);
    let status = match terminal_event.as_deref() {
        Some("turn.completed") => "completed",
        Some("turn.failed") => "failed",
        _ if assistant_message_preview.is_some() || total_token_usage.is_some() => "completed",
        _ => "failed",
    };
    let summary = match status {
        "completed" if assistant_message_preview.is_some() => {
            "Worker returned a bounded result to captain fan-in.".to_string()
        }
        "completed" => "Worker finished and returned control to captain fan-in.".to_string(),
        _ => {
            let mut details = Vec::new();
            if let Some(event_type) = terminal_event.as_deref() {
                details.push(format!("terminal_event={event_type}"));
            }
            if let Some(exit_status) = process_exit {
                if let Some(exit_code) = exit_status.code() {
                    details.push(format!("exit_code={exit_code}"));
                } else if exit_status.success() {
                    details.push("exit_success=true".to_string());
                } else {
                    details.push("exit_status=nonzero_without_code".to_string());
                }
            }
            if let Some(raw_output_bytes) = raw_output_bytes {
                details.push(format!("raw_events_bytes={raw_output_bytes}"));
            }
            if let Some(raw_output_preview) = raw_output_preview.as_ref() {
                details.push(format!("raw_output_preview=\"{raw_output_preview}\""));
            }
            if details.is_empty() {
                "Worker exited without parseable Codex result or usage artifacts.".to_string()
            } else {
                format!(
                    "Worker exited without parseable Codex result or usage artifacts. {}",
                    details.join(" ")
                )
            }
        }
    };
    let latest_failure = if status == "failed" {
        let reason = if terminal_event.as_deref() == Some("turn.failed")
            || process_exit
                .and_then(|value| value.code())
                .map(|value| value != 0)
                == Some(true)
            || execution_failure_observed
        {
            "execution_failed"
        } else if raw_output_bytes.unwrap_or(0) > 0 {
            "invalid_output"
        } else {
            "unknown"
        };
        json!({
            "stage": "execution",
            "reason": reason,
            "summary": summary,
            "recorded_at": completed_at,
        })
    } else {
        Value::Null
    };

    (
        status.to_string(),
        summary.clone(),
        json!({
            "status": status,
            "recorded_at": completed_at,
            "thread_id": thread_id,
            "assistant_message_preview": assistant_message_preview,
            "total_token_usage": total_token_usage,
            "process_exit": process_exit_payload,
            "raw_output_preview": raw_output_preview,
            "raw_output_bytes": raw_output_bytes,
        }),
        latest_failure,
    )
}
