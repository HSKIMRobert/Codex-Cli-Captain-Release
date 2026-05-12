use crate::captain_intervention::normalized_reassign_target_payload;
use crate::parallel_fanout::{normalize_host_lane_id, supported_host_lane_ids};
use crate::run_locator::{create_ccc_run_ref, resolve_run_locator_arguments};
use crate::subagent_update_validation::{
    canonical_subagent_fan_in_status, canonical_subagent_review_outcome,
    validate_sentinel_intervention_classification, validate_subagent_chosen_next_action,
    validate_subagent_fallback_reason_for_status, validate_subagent_intervention_classification,
};
use serde_json::{json, Value};
use std::io;

fn parse_required_string(
    arguments: &serde_json::Map<String, Value>,
    key: &str,
    tool_name: &str,
) -> io::Result<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{tool_name} requires string argument: {key}."),
            )
        })
}

fn structured_target_mentions_payload(arguments: &serde_json::Map<String, Value>) -> Value {
    let mut mentions = serde_json::Map::new();
    for key in [
        "target_paths",
        "file_paths",
        "artifact_paths",
        "mentioned_files",
        "input_items",
        "items",
    ] {
        if let Some(value) = arguments.get(key) {
            mentions.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(mentions)
}

pub(crate) fn parse_ccc_recommend_entry_arguments(arguments: &Value) -> io::Result<Value> {
    let object = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_recommend_entry arguments must be an object.",
        )
    })?;

    for key in object.keys() {
        if !matches!(key.as_str(), "request" | "cwd") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected ccc_recommend_entry argument: {key}."),
            ));
        }
    }

    Ok(json!({
        "request": parse_required_string(object, "request", "ccc_recommend_entry")?,
        "cwd": object.get("cwd").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
    }))
}

pub(crate) fn parse_ccc_auto_entry_arguments(arguments: &Value) -> io::Result<Value> {
    let object = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_auto_entry arguments must be an object.",
        )
    })?;

    for key in object.keys() {
        if !matches!(key.as_str(), "request" | "cwd" | "codex_bin") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected ccc_auto_entry argument: {key}."),
            ));
        }
    }

    Ok(json!({
        "request": parse_required_string(object, "request", "ccc_auto_entry")?,
        "cwd": object.get("cwd").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "codex_bin": object.get("codex_bin").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
    }))
}

fn normalize_sequence(
    arguments: &serde_json::Map<String, Value>,
    tool_name: &str,
) -> io::Result<String> {
    let longway_disabled = arguments
        .get("no_longway")
        .or_else(|| arguments.get("skip_longway"))
        .or_else(|| arguments.get("disable_longway"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let Some(raw_sequence) = arguments.get("sequence") else {
        return Ok(if longway_disabled {
            "EXECUTE_SEQUENCE".to_string()
        } else {
            "PLAN_SEQUENCE".to_string()
        });
    };
    let Some(sequence) = raw_sequence
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{tool_name} sequence must be a non-empty string when provided."),
        ));
    };
    match sequence {
        "PLAN_SEQUENCE" | "plan" => Ok("PLAN_SEQUENCE".to_string()),
        "EXECUTE_SEQUENCE" | "execute" => Ok("EXECUTE_SEQUENCE".to_string()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{tool_name} sequence must be PLAN_SEQUENCE or EXECUTE_SEQUENCE."),
        )),
    }
}

pub(crate) fn parse_ccc_start_arguments(arguments: &Value) -> io::Result<Value> {
    let object = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_start arguments must be an object.",
        )
    })?;

    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "goal"
                | "title"
                | "intent"
                | "scope"
                | "acceptance"
                | "prompt"
                | "task_kind"
                | "sequence"
                | "no_longway"
                | "skip_longway"
                | "disable_longway"
                | "planned_rows"
                | "target_paths"
                | "file_paths"
                | "artifact_paths"
                | "mentioned_files"
                | "input_items"
                | "items"
                | "cwd"
                | "compact"
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected ccc_start argument: {key}."),
            ));
        }
    }

    let task_kind = object
        .get("task_kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| matches!(*value, "execution" | "explore" | "review" | "way"))
        .unwrap_or("execution");
    let sequence = normalize_sequence(object, "ccc_start")?;

    Ok(json!({
        "goal": parse_required_string(object, "goal", "ccc_start")?,
        "title": parse_required_string(object, "title", "ccc_start")?,
        "intent": parse_required_string(object, "intent", "ccc_start")?,
        "scope": parse_required_string(object, "scope", "ccc_start")?,
        "acceptance": parse_required_string(object, "acceptance", "ccc_start")?,
        "prompt": parse_required_string(object, "prompt", "ccc_start")?,
        "task_kind": task_kind,
        "sequence": sequence,
        "planned_rows": object.get("planned_rows").cloned().unwrap_or(Value::Null),
        "structured_target_mentions": structured_target_mentions_payload(object),
        "cwd": object.get("cwd").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "compact": object.get("compact").and_then(Value::as_bool).unwrap_or(false),
    }))
}

pub(crate) fn parse_ccc_run_arguments(arguments: &Value) -> io::Result<Value> {
    let object = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_run arguments must be an object.",
        )
    })?;

    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "goal"
                | "title"
                | "intent"
                | "scope"
                | "acceptance"
                | "prompt"
                | "task_kind"
                | "sequence"
                | "no_longway"
                | "skip_longway"
                | "disable_longway"
                | "planned_rows"
                | "workflow_variant_selection"
                | "target_paths"
                | "file_paths"
                | "artifact_paths"
                | "mentioned_files"
                | "input_items"
                | "items"
                | "codex_bin"
                | "cwd"
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected ccc_run argument: {key}."),
            ));
        }
    }

    let task_kind = object
        .get("task_kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| matches!(*value, "execution" | "explore" | "review" | "way"))
        .unwrap_or("execution");
    let sequence = normalize_sequence(object, "ccc_run")?;

    let workflow_variant_selection = match object.get("workflow_variant_selection") {
        Some(value) if value.is_object() => value.clone(),
        Some(Value::Null) | None => Value::Null,
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ccc_run workflow_variant_selection must be an object when provided.",
            ))
        }
    };

    Ok(json!({
        "goal": parse_required_string(object, "goal", "ccc_run")?,
        "title": parse_required_string(object, "title", "ccc_run")?,
        "intent": parse_required_string(object, "intent", "ccc_run")?,
        "scope": parse_required_string(object, "scope", "ccc_run")?,
        "acceptance": parse_required_string(object, "acceptance", "ccc_run")?,
        "prompt": parse_required_string(object, "prompt", "ccc_run")?,
        "task_kind": task_kind,
        "sequence": sequence,
        "planned_rows": object.get("planned_rows").cloned().unwrap_or(Value::Null),
        "structured_target_mentions": structured_target_mentions_payload(object),
        "workflow_variant_selection": workflow_variant_selection,
        "codex_bin": object.get("codex_bin").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "cwd": object.get("cwd").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
    }))
}

pub(crate) fn parse_ccc_orchestrate_arguments(arguments: &Value) -> io::Result<Value> {
    let object = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_orchestrate arguments must be an object.",
        )
    })?;

    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "run_id"
                | "run_ref"
                | "run_dir"
                | "cwd"
                | "codex_bin"
                | "progression_mode"
                | "progression_step_count"
                | "fast_mode"
                | "max_steps"
                | "repair_action"
                | "replan_prompt"
                | "resolve_outcome"
                | "resolve_summary"
                | "approve_longway"
                | "compact"
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected ccc_orchestrate argument: {key}."),
            ));
        }
    }

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": object.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": object.get("run_ref").cloned().unwrap_or(Value::Null),
            "run_dir": object.get("run_dir").cloned().unwrap_or(Value::Null),
            "cwd": object.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_orchestrate",
    )?;
    let progression_mode = object
        .get("progression_mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| matches!(*value, "single_step" | "two_step" | "drain_until_boundary"))
        .map(str::to_string);
    let progression_step_count = object
        .get("progression_step_count")
        .and_then(Value::as_u64)
        .filter(|value| matches!(*value, 1 | 2));
    let fast_mode = object.get("fast_mode").and_then(Value::as_bool);
    let max_steps = object
        .get("max_steps")
        .and_then(Value::as_u64)
        .filter(|value| (1..=12).contains(value));

    Ok(json!({
        "cwd": locator.cwd.to_string_lossy(),
        "run_id": locator.run_id,
        "run_directory": locator.run_directory.to_string_lossy(),
        "run_ref": create_ccc_run_ref(&locator.run_directory),
        "codex_bin": object.get("codex_bin").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "progression_mode": progression_mode,
        "progression_step_count": progression_step_count,
        "fast_mode": fast_mode,
        "max_steps": max_steps,
        "repair_action": object.get("repair_action").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "replan_prompt": object.get("replan_prompt").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "resolve_outcome": object.get("resolve_outcome").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "resolve_summary": object.get("resolve_summary").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "approve_longway": object.get("approve_longway").and_then(Value::as_bool).unwrap_or(false),
        "compact": object.get("compact").and_then(Value::as_bool).unwrap_or(false),
    }))
}

pub(crate) fn parse_ccc_subagent_update_arguments(arguments: &Value) -> io::Result<Value> {
    let object = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update arguments must be an object.",
        )
    })?;

    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "run_id"
                | "run_ref"
                | "run_dir"
                | "cwd"
                | "task_card_id"
                | "child_agent_id"
                | "lane_id"
                | "thread_id"
                | "status"
                | "summary"
                | "fan_in_status"
                | "review_outcome"
                | "intervention_classification"
                | "intervention_rationale"
                | "chosen_next_action"
                | "budget_snapshot"
                | "reassign_target"
                | "stale_output_policy"
                | "stale_output_summary"
                | "sentinel_classification"
                | "sentinel_rationale"
                | "sentinel_next_action"
                | "sentinel_summary"
                | "findings"
                | "evidence_paths"
                | "next_action"
                | "open_questions"
                | "confidence"
                | "observed_model"
                | "observed_variant"
                | "observed_sandbox_mode"
                | "observed_approval_policy"
                | "fallback_reason"
                | "total_token_usage"
                | "context_tokens"
                | "estimated_context_tokens"
                | "event_ref"
                | "mode"
                | "compact"
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected ccc_subagent_update argument: {key}."),
            ));
        }
    }
    let reassign_target = object
        .get("reassign_target")
        .map(normalized_reassign_target_payload)
        .transpose()?
        .unwrap_or(Value::Null);

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": object.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": object.get("run_ref").cloned().unwrap_or(Value::Null),
            "run_dir": object.get("run_dir").cloned().unwrap_or(Value::Null),
            "cwd": object.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_subagent_update",
    )?;
    let status = object
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| {
            matches!(
                *value,
                "spawned"
                    | "acknowledged"
                    | "running"
                    | "stalled"
                    | "completed"
                    | "failed"
                    | "merged"
                    | "reclaimed"
            )
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "ccc_subagent_update requires status in {spawned, acknowledged, running, stalled, completed, failed, merged, reclaimed}.",
            )
        })?;
    let review_outcome = object
        .get("review_outcome")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let review_outcome = review_outcome
        .map(canonical_subagent_review_outcome)
        .transpose()?;
    let fan_in_status = object
        .get("fan_in_status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(canonical_subagent_fan_in_status);
    let intervention_classification = object
        .get("intervention_classification")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(classification) = intervention_classification {
        validate_subagent_intervention_classification(classification)?;
    }
    let chosen_next_action = object
        .get("chosen_next_action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(action) = chosen_next_action {
        validate_subagent_chosen_next_action(action)?;
    }
    let sentinel_classification = object
        .get("sentinel_classification")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(classification) = sentinel_classification {
        validate_sentinel_intervention_classification(classification)?;
    }

    let fallback_reason = object
        .get("fallback_reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(reason) = fallback_reason {
        validate_subagent_fallback_reason_for_status(reason, status)?;
    }
    let lane_id = object
        .get("lane_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            normalize_host_lane_id(value).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "ccc_subagent_update lane_id must be one of: {}.",
                        supported_host_lane_ids().join(", ")
                    ),
                )
            })
        })
        .transpose()?;
    let mode = object
        .get("mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("full");
    if !matches!(mode, "full" | "compact") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update mode must be one of: full, compact.",
        ));
    }

    Ok(json!({
        "cwd": locator.cwd.to_string_lossy(),
        "run_id": locator.run_id,
        "run_directory": locator.run_directory.to_string_lossy(),
        "run_ref": create_ccc_run_ref(&locator.run_directory),
        "task_card_id": object.get("task_card_id").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "child_agent_id": object.get("child_agent_id").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "lane_id": lane_id,
        "thread_id": object.get("thread_id").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "status": status,
        "summary": object.get("summary").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "fan_in_status": fan_in_status,
        "review_outcome": review_outcome,
        "intervention_classification": intervention_classification,
        "intervention_rationale": object.get("intervention_rationale").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "chosen_next_action": chosen_next_action,
        "budget_snapshot": object.get("budget_snapshot").filter(|value| value.is_object()).cloned().unwrap_or(Value::Null),
        "reassign_target": reassign_target,
        "stale_output_policy": object.get("stale_output_policy").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "stale_output_summary": object.get("stale_output_summary").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "sentinel_classification": sentinel_classification,
        "sentinel_rationale": object.get("sentinel_rationale").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "sentinel_next_action": object.get("sentinel_next_action").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "sentinel_summary": object.get("sentinel_summary").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "findings": object.get("findings").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "evidence_paths": object.get("evidence_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "next_action": object.get("next_action").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "open_questions": object.get("open_questions").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "confidence": object.get("confidence").cloned().unwrap_or(Value::Null),
        "observed_model": object.get("observed_model").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "observed_variant": object.get("observed_variant").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "observed_sandbox_mode": object.get("observed_sandbox_mode").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "observed_approval_policy": object.get("observed_approval_policy").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "fallback_reason": fallback_reason,
        "total_token_usage": object.get("total_token_usage").filter(|value| value.is_object()).cloned().unwrap_or(Value::Null),
        "context_tokens": object.get("context_tokens")
            .or_else(|| object.get("estimated_context_tokens"))
            .and_then(Value::as_u64),
        "event_ref": object.get("event_ref").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()),
        "mode": mode,
        "compact": object.get("compact").and_then(Value::as_bool).unwrap_or(false),
    }))
}
