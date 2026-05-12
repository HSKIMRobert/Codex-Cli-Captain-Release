use crate::host_subagent_lifecycle::is_terminal_or_merged_host_subagent_status;
use crate::specialist_roles::{
    normalize_dispatch_role_hint, phase_name_for_role, resolve_follow_up_specialist_assignment,
    role_for_agent_id, GENERATED_CUSTOM_AGENT_NAME_PREFIX,
};
use crate::{create_follow_up_task_card, read_json_document, write_json_document};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn is_valid_intervention_classification(classification: &str) -> bool {
    matches!(
        classification,
        "clarification_only" | "bounded_scope_amendment" | "direction_or_risk_correction"
    )
}

pub(crate) fn is_valid_intervention_next_action(action: &str) -> bool {
    matches!(
        action,
        "amend_same_worker" | "reclaim" | "reassign" | "close" | "clarify" | "no_action"
    )
}

fn budget_remaining_for_action(budget_snapshot: &Value, action: &str) -> Option<i64> {
    let budget_key = match action {
        "amend_same_worker" => "retry",
        "reassign" => "reassign",
        _ => return None,
    };

    let budget = budget_snapshot.get(budget_key)?.as_object()?;
    budget.get("limit").and_then(Value::as_i64)?;
    budget.get("used").and_then(Value::as_i64)?;
    budget.get("remaining").and_then(Value::as_i64)
}

fn budget_unavailable_block_reason(action: &str) -> Option<&'static str> {
    match action {
        "amend_same_worker" => Some("retry_budget_unavailable"),
        "reassign" => Some("reassign_budget_unavailable"),
        _ => None,
    }
}

fn budget_exhausted_block_reason(action: &str) -> &'static str {
    match action {
        "amend_same_worker" => "retry_budget_exhausted",
        "reassign" => "reassign_budget_exhausted",
        _ => "budget_exhausted",
    }
}

pub(crate) fn normalized_reassign_target_payload(target: &Value) -> io::Result<Value> {
    let object = target.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update reassign_target must be an object.",
        )
    })?;
    let raw_role = object
        .get("assigned_role")
        .or_else(|| object.get("role"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "ccc_subagent_update reassign_target requires assigned_role.",
            )
        })?;
    let assigned_role = normalize_dispatch_role_hint(Some(raw_role), raw_role);
    let assigned_agent_id = object
        .get("assigned_agent_id")
        .or_else(|| object.get("agent_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "ccc_subagent_update reassign_target requires assigned_agent_id.",
            )
        })?;
    let agent_role = role_for_agent_id(assigned_agent_id).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update reassign_target assigned_agent_id is not a known CCC agent.",
        )
    })?;
    if agent_role != assigned_role {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update reassign_target assigned_role must match assigned_agent_id.",
        ));
    }
    let prompt = object
        .get("prompt")
        .or_else(|| object.get("execution_prompt"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "ccc_subagent_update reassign_target requires prompt.",
            )
        })?;
    let scope = object
        .get("scope")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    Ok(json!({
        "assigned_role": assigned_role,
        "assigned_agent_id": assigned_agent_id,
        "scope": scope,
        "prompt": prompt,
    }))
}

fn build_same_specialist_follow_up_prompt(
    task_card: &Value,
    fan_in_compact: &Value,
    rationale: Option<&str>,
) -> String {
    let original_prompt = task_card
        .get("execution_prompt")
        .and_then(Value::as_str)
        .unwrap_or("Complete the original bounded task.");
    let summary = fan_in_compact
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("The prior specialist result needs a bounded amendment.");
    let evidence_paths = fan_in_compact
        .get("evidence_paths")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none provided".to_string());
    let open_questions = fan_in_compact
        .get("open_questions")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none".to_string());
    let rationale = rationale.unwrap_or("Captain selected a same-specialist bounded amendment.");

    format!(
        "Bounded same-specialist amendment only.\nOriginal task: {original_prompt}\nCaptain rationale: {rationale}\nPrior result summary: {summary}\nEvidence to inspect: {evidence_paths}\nOpen questions: {open_questions}\nReturn only the narrowed repair result to captain fan-in."
    )
}

fn follow_up_agent_role_hint(agent_id: &str) -> Option<(&'static str, String)> {
    let trimmed = agent_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(role) = role_for_agent_id(trimmed) {
        return Some((role, trimmed.to_string()));
    }
    let generated_agent_id = trimmed.strip_prefix(GENERATED_CUSTOM_AGENT_NAME_PREFIX)?;
    role_for_agent_id(generated_agent_id).map(|role| (role, generated_agent_id.to_string()))
}

pub(crate) fn pending_follow_up_dedupe_key(value: &Value) -> Option<&str> {
    value
        .get("dedupe_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn existing_pending_follow_up_for_key(
    task_card: &Value,
    dedupe_key: &str,
) -> Option<Value> {
    task_card
        .pointer("/captain_intervention/pending_follow_up")
        .filter(|value| pending_follow_up_dedupe_key(value) == Some(dedupe_key))
        .cloned()
        .or_else(|| {
            task_card
                .get("captain_intervention_history")
                .and_then(Value::as_array)
                .and_then(|history| {
                    history.iter().find_map(|entry| {
                        entry
                            .get("pending_follow_up")
                            .filter(|value| pending_follow_up_dedupe_key(value) == Some(dedupe_key))
                            .cloned()
                    })
                })
        })
}

pub(crate) fn queued_pending_captain_follow_up(task_card: &Value) -> Option<Value> {
    task_card
        .pointer("/captain_intervention/pending_follow_up")
        .filter(|value| value.is_object())
        .filter(|value| value.get("status").and_then(Value::as_str) == Some("queued"))
        .filter(|value| {
            matches!(
                value.get("action").and_then(Value::as_str),
                Some("amend_same_worker" | "reassign")
            )
        })
        .filter(|value| pending_follow_up_dedupe_key(value).is_some())
        .cloned()
}

pub(crate) fn create_pending_captain_follow_up_payload(
    parsed: &Value,
    task_card: &Value,
    fan_in_compact: &Value,
    active_task_card_id: &str,
    child_agent_id: &str,
    lane_id: Option<&str>,
    status: &str,
    timestamp: &str,
) -> Option<Value> {
    if !is_terminal_or_merged_host_subagent_status(status) {
        return None;
    }
    let action = parsed.get("chosen_next_action").and_then(Value::as_str)?;
    if !matches!(action, "amend_same_worker" | "reassign") {
        return None;
    }
    let budget_snapshot = parsed
        .get("budget_snapshot")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or(Value::Null);
    if budget_remaining_for_action(&budget_snapshot, action)
        .map(|remaining| remaining <= 0)
        .unwrap_or(true)
    {
        return None;
    }

    let rationale = parsed
        .get("intervention_rationale")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match action {
        "amend_same_worker" => {
            let reported_worker = follow_up_agent_role_hint(child_agent_id);
            let (assigned_role, assigned_agent_id) = resolve_follow_up_specialist_assignment(
                task_card,
                reported_worker
                    .as_ref()
                    .map(|(role, _)| *role)
                    .or_else(|| task_card.get("assigned_role").and_then(Value::as_str)),
                reported_worker
                    .as_ref()
                    .map(|(_, agent_id)| agent_id.as_str())
                    .or_else(|| task_card.get("assigned_agent_id").and_then(Value::as_str))
                    .or(Some(child_agent_id)),
            );
            let dedupe_key = format!(
                "{active_task_card_id}:amend_same_worker:{assigned_agent_id}:{}",
                lane_id.unwrap_or("single")
            );
            Some(json!({
                "status": "queued",
                "action": "amend_same_worker",
                "source_task_card_id": active_task_card_id,
                "assigned_role": assigned_role,
                "assigned_agent_id": assigned_agent_id,
                "lane_id": lane_id,
                "scope": task_card.get("scope").cloned().unwrap_or(Value::Null),
                "prompt": build_same_specialist_follow_up_prompt(task_card, fan_in_compact, rationale),
                "budget_key": "retry",
                "budget_snapshot": budget_snapshot,
                "dedupe_key": dedupe_key,
                "queued_at": timestamp,
                "authority": "captain_owned_follow_up",
            }))
        }
        "reassign" => {
            let target = parsed
                .get("reassign_target")
                .filter(|value| value.is_object())?;
            let (assigned_role, assigned_agent_id) = resolve_follow_up_specialist_assignment(
                task_card,
                target.get("assigned_role").and_then(Value::as_str),
                target.get("assigned_agent_id").and_then(Value::as_str),
            );
            let prompt = target.get("prompt").and_then(Value::as_str)?;
            let dedupe_key = format!(
                "{active_task_card_id}:reassign:{assigned_agent_id}:{}",
                lane_id.unwrap_or("single")
            );
            Some(json!({
                "status": "queued",
                "action": "reassign",
                "source_task_card_id": active_task_card_id,
                "assigned_role": assigned_role,
                "assigned_agent_id": assigned_agent_id,
                "lane_id": lane_id,
                "scope": target.get("scope").cloned().unwrap_or(Value::Null),
                "prompt": prompt,
                "budget_key": "reassign",
                "budget_snapshot": budget_snapshot,
                "dedupe_key": dedupe_key,
                "queued_at": timestamp,
                "authority": "captain_owned_follow_up",
            }))
        }
        _ => None,
    }
}

pub(crate) fn task_card_captain_follow_up_dedupe_key(task_card: &Value) -> Option<&str> {
    task_card
        .get("captain_follow_up")
        .and_then(pending_follow_up_dedupe_key)
}

fn existing_follow_up_task_card_for_dedupe_key(
    run_directory: &Path,
    dedupe_key: &str,
) -> io::Result<Option<Value>> {
    let task_cards_directory = run_directory.join("task-cards");
    let entries = match fs::read_dir(&task_cards_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let task_card = read_json_document(&path)?;
        if task_card_captain_follow_up_dedupe_key(&task_card) == Some(dedupe_key) {
            return Ok(Some(task_card));
        }
    }

    Ok(None)
}

fn activate_follow_up_task_card(
    run_directory: &Path,
    task_card: &Value,
    timestamp: &str,
) -> io::Result<()> {
    let task_card_id = task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "follow-up task card is missing task_card_id",
            )
        })?;
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("code specialist");

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert(
        "active_task_card_id".to_string(),
        Value::String(task_card_id.to_string()),
    );
    run_object.insert(
        "active_role".to_string(),
        Value::String("orchestrator".to_string()),
    );
    run_object.insert(
        "active_agent_id".to_string(),
        Value::String("captain".to_string()),
    );
    run_object.insert(
        "latest_handoff_id".to_string(),
        Value::String(task_card_id.to_string()),
    );
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    let run_state_path = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_path)?;
    let run_state_object = run_state.as_object_mut().ok_or_else(|| {
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
        "current_phase_name".to_string(),
        Value::String(phase_name_for_role(assigned_role).to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({
            "command": "execute_task"
        }),
    );
    write_json_document(&run_state_path, &run_state)
}

pub(crate) fn consumed_pending_follow_up_payload(
    pending_follow_up: &Value,
    consumed_task_card_id: &str,
    timestamp: &str,
) -> Value {
    let mut consumed = pending_follow_up.clone();
    if let Some(object) = consumed.as_object_mut() {
        object.insert("status".to_string(), Value::String("consumed".to_string()));
        object.insert(
            "consumed_at".to_string(),
            Value::String(timestamp.to_string()),
        );
        object.insert(
            "consumed_task_card_id".to_string(),
            Value::String(consumed_task_card_id.to_string()),
        );
    }
    consumed
}

fn mark_pending_follow_up_consumed_in_intervention(
    intervention: &mut Value,
    dedupe_key: &str,
    consumed_task_card_id: &str,
    timestamp: &str,
) -> bool {
    let Some(pending_follow_up) = intervention.get_mut("pending_follow_up") else {
        return false;
    };
    if pending_follow_up_dedupe_key(pending_follow_up) != Some(dedupe_key) {
        return false;
    }
    *pending_follow_up =
        consumed_pending_follow_up_payload(pending_follow_up, consumed_task_card_id, timestamp);
    true
}

fn mark_pending_follow_up_consumed(
    run_directory: &Path,
    source_task_card_id: &str,
    dedupe_key: &str,
    consumed_task_card_id: &str,
    timestamp: &str,
) -> io::Result<Value> {
    let consumed = {
        let source_task_card_file = run_directory
            .join("task-cards")
            .join(format!("{source_task_card_id}.json"));
        let mut source_task_card = read_json_document(&source_task_card_file)?;
        if let Some(intervention) = source_task_card.get_mut("captain_intervention") {
            mark_pending_follow_up_consumed_in_intervention(
                intervention,
                dedupe_key,
                consumed_task_card_id,
                timestamp,
            );
        }
        if let Some(history) = source_task_card
            .get_mut("captain_intervention_history")
            .and_then(Value::as_array_mut)
        {
            for entry in history {
                mark_pending_follow_up_consumed_in_intervention(
                    entry,
                    dedupe_key,
                    consumed_task_card_id,
                    timestamp,
                );
            }
        }
        if let Some(object) = source_task_card.as_object_mut() {
            object.insert(
                "updated_at".to_string(),
                Value::String(timestamp.to_string()),
            );
        }
        write_json_document(&source_task_card_file, &source_task_card)?;
        source_task_card
            .pointer("/captain_intervention/pending_follow_up")
            .cloned()
            .filter(|value| pending_follow_up_dedupe_key(value) == Some(dedupe_key))
            .unwrap_or_else(|| {
                json!({
                    "status": "consumed",
                    "dedupe_key": dedupe_key,
                    "consumed_task_card_id": consumed_task_card_id,
                    "consumed_at": timestamp,
                })
            })
    };

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    if let Some(intervention) = run_record.get_mut("latest_captain_intervention") {
        mark_pending_follow_up_consumed_in_intervention(
            intervention,
            dedupe_key,
            consumed_task_card_id,
            timestamp,
        );
    }
    if let Some(intervention) = run_record.pointer_mut("/latest_entry_trace/captain_intervention") {
        mark_pending_follow_up_consumed_in_intervention(
            intervention,
            dedupe_key,
            consumed_task_card_id,
            timestamp,
        );
    }
    if let Some(object) = run_record.as_object_mut() {
        object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }
    write_json_document(&run_file, &run_record)?;

    Ok(consumed)
}

pub(crate) fn create_follow_up_task_card_from_pending_follow_up(
    run_directory: &Path,
    current_task_card: &Value,
    pending_follow_up: &Value,
    timestamp: &str,
) -> io::Result<Value> {
    let dedupe_key = pending_follow_up_dedupe_key(pending_follow_up).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pending follow-up is missing dedupe_key",
        )
    })?;
    let source_task_card_id = pending_follow_up
        .get("source_task_card_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            current_task_card
                .get("task_card_id")
                .and_then(Value::as_str)
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "pending follow-up is missing source_task_card_id",
            )
        })?;
    let pending_assigned_role = pending_follow_up
        .get("assigned_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "pending follow-up is missing assigned_role",
            )
        })?;
    let pending_assigned_agent_id = pending_follow_up
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "pending follow-up is missing assigned_agent_id",
            )
        })?;
    let (assigned_role, assigned_agent_id) = resolve_follow_up_specialist_assignment(
        current_task_card,
        Some(pending_assigned_role),
        Some(pending_assigned_agent_id),
    );
    let prompt = pending_follow_up
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "pending follow-up is missing prompt",
            )
        })?;

    if let Some(existing_task_card) =
        existing_follow_up_task_card_for_dedupe_key(run_directory, dedupe_key)?
    {
        activate_follow_up_task_card(run_directory, &existing_task_card, timestamp)?;
        mark_pending_follow_up_consumed(
            run_directory,
            source_task_card_id,
            dedupe_key,
            existing_task_card
                .get("task_card_id")
                .and_then(Value::as_str)
                .unwrap_or("existing-follow-up"),
            timestamp,
        )?;
        return Ok(existing_task_card);
    }

    let mut follow_up_task_card = create_follow_up_task_card(
        run_directory,
        current_task_card,
        Some(&assigned_role),
        prompt,
        None,
        timestamp,
    )?;
    let follow_up_task_card_id = follow_up_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "created follow-up task card is missing task_card_id",
            )
        })?
        .to_string();
    let consumed =
        consumed_pending_follow_up_payload(pending_follow_up, &follow_up_task_card_id, timestamp);

    if let Some(object) = follow_up_task_card.as_object_mut() {
        object.insert(
            "owner_role".to_string(),
            Value::String("orchestrator".to_string()),
        );
        object.insert(
            "assigned_role".to_string(),
            Value::String(assigned_role.clone()),
        );
        object.insert(
            "assigned_agent_id".to_string(),
            Value::String(assigned_agent_id.clone()),
        );
        if let Some(scope) = pending_follow_up
            .get("scope")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            object.insert("scope".to_string(), Value::String(scope.to_string()));
        }
        object.insert(
            "execution_prompt".to_string(),
            Value::String(prompt.to_string()),
        );
        object.insert(
            "depends_on_task_card_ids".to_string(),
            json!([source_task_card_id]),
        );
        object.insert(
            "fan_in_from_task_card_ids".to_string(),
            json!([source_task_card_id]),
        );
        object.insert("captain_follow_up".to_string(), consumed.clone());
    }
    let follow_up_task_card_file = run_directory
        .join("task-cards")
        .join(format!("{follow_up_task_card_id}.json"));
    write_json_document(&follow_up_task_card_file, &follow_up_task_card)?;
    mark_pending_follow_up_consumed(
        run_directory,
        source_task_card_id,
        dedupe_key,
        &follow_up_task_card_id,
        timestamp,
    )?;

    Ok(follow_up_task_card)
}

pub(crate) fn create_captain_intervention_payload(
    parsed: &Value,
    fan_in_compact: &Value,
    timestamp: &str,
) -> Option<Value> {
    let classification = parsed
        .get("intervention_classification")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let rationale = parsed
        .get("intervention_rationale")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let chosen_next_action = parsed
        .get("chosen_next_action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let budget_snapshot = parsed
        .get("budget_snapshot")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or(Value::Null);
    let stale_output_policy = parsed
        .get("stale_output_policy")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let stale_output_summary = parsed
        .get("stale_output_summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let should_record = classification.is_some()
        || rationale.is_some()
        || chosen_next_action.is_some()
        || budget_snapshot.is_object()
        || stale_output_policy.is_some()
        || stale_output_summary.is_some();
    if !should_record {
        return None;
    }

    let remaining_budget =
        chosen_next_action.and_then(|action| budget_remaining_for_action(&budget_snapshot, action));
    let reassign_target_missing = chosen_next_action == Some("reassign")
        && parsed
            .get("reassign_target")
            .filter(|value| value.is_object())
            .is_none();
    let next_action_block_reason = if reassign_target_missing {
        Some("reassign_target_missing")
    } else if let Some(action) = chosen_next_action {
        if let Some(remaining) = remaining_budget {
            (remaining <= 0).then_some(budget_exhausted_block_reason(action))
        } else {
            budget_unavailable_block_reason(action)
        }
    } else {
        None
    };
    let next_action_blocked = next_action_block_reason.is_some();

    Some(json!({
        "classification": classification.unwrap_or("direction_or_risk_correction"),
        "rationale": rationale.unwrap_or("Captain recorded dissatisfaction or intervention without a detailed rationale."),
        "chosen_next_action": chosen_next_action.unwrap_or("no_action"),
        "budget_snapshot": budget_snapshot,
        "next_action_blocked": next_action_blocked,
        "next_action_block_reason": next_action_block_reason,
        "stale_output_policy": stale_output_policy,
        "stale_output_summary": stale_output_summary,
        "summary": fan_in_compact.get("summary").cloned().unwrap_or(Value::Null),
        "subagent_status": fan_in_compact.get("status").cloned().unwrap_or(Value::Null),
        "evidence_paths": fan_in_compact.get("evidence_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "open_questions": fan_in_compact.get("open_questions").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "authority": "captain_decides_intervention",
        "recorded_at": timestamp,
    }))
}
