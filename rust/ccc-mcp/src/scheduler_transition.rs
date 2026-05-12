use crate::{read_optional_json_document, write_json_document};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct SchedulerTransitionRecordInput<'a> {
    pub(crate) run_id: &'a str,
    pub(crate) timestamp: &'a str,
    pub(crate) action_kind: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) selected_task_card: &'a Value,
    pub(crate) selected_planned_row: &'a Value,
    pub(crate) next_step_after_attempt: &'a str,
    pub(crate) can_advance_after_attempt: bool,
}

pub(crate) fn append_scheduler_transition_record(
    run_directory: &Path,
    input: SchedulerTransitionRecordInput<'_>,
) -> io::Result<Value> {
    let transitions_directory = scheduler_transitions_directory(run_directory);
    fs::create_dir_all(&transitions_directory)?;
    let (transition_id, transition_path) = next_transition_file(&transitions_directory)?;
    let record = create_scheduler_transition_record(&transition_id, input);
    write_json_document(&transition_path, &record)?;
    Ok(record)
}

pub(crate) fn read_latest_scheduler_transition(run_directory: &Path) -> io::Result<Value> {
    let transitions_directory = scheduler_transitions_directory(run_directory);
    let Some(path) = latest_transition_file(&transitions_directory)? else {
        return Ok(Value::Null);
    };
    read_optional_json_document(&path).map(|value| value.unwrap_or(Value::Null))
}

fn scheduler_transitions_directory(run_directory: &Path) -> PathBuf {
    run_directory.join("scheduler").join("transitions")
}

fn next_transition_file(transitions_directory: &Path) -> io::Result<(String, PathBuf)> {
    let next_index = max_transition_index(transitions_directory)? + 1;
    let transition_id = format!("transition-{next_index:04}");
    let path = transitions_directory.join(format!("{transition_id}.json"));
    Ok((transition_id, path))
}

fn latest_transition_file(transitions_directory: &Path) -> io::Result<Option<PathBuf>> {
    if !transitions_directory.exists() {
        return Ok(None);
    }
    let max_index = max_transition_index(transitions_directory)?;
    if max_index == 0 {
        return Ok(None);
    }
    Ok(Some(
        transitions_directory.join(format!("transition-{max_index:04}.json")),
    ))
}

fn max_transition_index(transitions_directory: &Path) -> io::Result<u32> {
    if !transitions_directory.exists() {
        return Ok(0);
    }
    let mut max_index = 0_u32;
    for entry in fs::read_dir(transitions_directory)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(index_text) = file_name
            .strip_prefix("transition-")
            .and_then(|value| value.strip_suffix(".json"))
        else {
            continue;
        };
        if let Ok(index) = index_text.parse::<u32>() {
            max_index = max_index.max(index);
        }
    }
    Ok(max_index)
}

fn create_scheduler_transition_record(
    transition_id: &str,
    input: SchedulerTransitionRecordInput<'_>,
) -> Value {
    json!({
        "schema": "ccc.scheduler_transition.v1",
        "transition_id": transition_id,
        "recorded_at": input.timestamp,
        "run_id": input.run_id,
        "decision_source": "planned_row_materialization",
        "action": {
            "kind": input.action_kind,
            "reason": input.reason,
            "next_step_after_attempt": input.next_step_after_attempt,
            "can_advance_after_attempt": input.can_advance_after_attempt,
        },
        "selected_task_card_id": input.selected_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "selected_role": input.selected_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "selected_agent_id": input.selected_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "selected_planned_row": input.selected_planned_row,
        "route": compact_route(input.selected_task_card),
        "blocked": {
            "blocked": false,
            "reason": Value::Null,
        },
        "fan_out": compact_fan_out(input.selected_task_card),
        "next_expected_lifecycle_event": {
            "event": "subagent_update",
            "status": "running",
            "next_step": input.next_step_after_attempt,
        },
    })
}

fn compact_route(task_card: &Value) -> Value {
    json!({
        "assigned_role": task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
        "model": task_card.pointer("/delegation_plan/runtime_dispatch/model")
            .or_else(|| task_card.pointer("/role_config_snapshot/model"))
            .cloned()
            .unwrap_or(Value::Null),
        "reasoning": task_card.pointer("/delegation_plan/runtime_dispatch/variant")
            .or_else(|| task_card.pointer("/role_config_snapshot/variant"))
            .cloned()
            .unwrap_or(Value::Null),
        "routing_summary": task_card.get("routing_summary").cloned().unwrap_or(Value::Null),
        "routing_trace": task_card.get("routing_trace").cloned().unwrap_or(Value::Null),
    })
}

fn compact_fan_out(task_card: &Value) -> Value {
    let required_lane_ids = task_card
        .pointer("/parallel_fanout/required_lane_ids")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    json!({
        "mode": task_card.pointer("/parallel_fanout/mode").cloned().unwrap_or(Value::Null),
        "required_lane_ids": required_lane_ids,
        "candidate_count": task_card
            .pointer("/parallel_fanout/lanes")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
    })
}
