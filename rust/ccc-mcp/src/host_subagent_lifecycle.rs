use chrono::Utc;
use serde_json::{json, Value};

use crate::parallel_fanout::{
    normalize_parallel_lane_id_for_state, parallel_lane_statuses, parallel_required_lane_ids,
};
use crate::phase_name_for_role;
use std::collections::BTreeMap;

const DEFAULT_HOST_SUBAGENT_PROVIDER_CONCURRENCY_LIMIT: u64 = 4;
const DEFAULT_HOST_SUBAGENT_MODEL_CONCURRENCY_LIMIT: u64 = 2;
const DEFAULT_HOST_SUBAGENT_RECLAIM_AFTER_MS: u64 = 45_000;

struct HostSubagentLifecycleThresholds {
    reclaim_after_ms: u64,
    reclaim_after_source: &'static str,
    stale_after_ms: u64,
    stale_after_source: &'static str,
}

pub(crate) fn is_active_host_subagent_status(status: &str) -> bool {
    matches!(status, "spawned" | "acknowledged" | "running")
}

pub(crate) fn is_terminal_host_subagent_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "stalled" | "reclaimed")
}

pub(crate) fn is_terminal_or_merged_host_subagent_status(status: &str) -> bool {
    status == "merged" || is_terminal_host_subagent_status(status)
}

pub(crate) fn task_card_has_terminal_or_merged_host_subagent_state(task_card: &Value) -> bool {
    ["subagent_lifecycle", "review_lifecycle"]
        .iter()
        .any(|field| {
            task_card
                .get(*field)
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                .map(is_terminal_or_merged_host_subagent_status)
                .unwrap_or(false)
        })
        || task_card
            .pointer("/parallel_fanout/lanes")
            .and_then(Value::as_array)
            .is_some_and(|lanes| {
                lanes.iter().any(|lane| {
                    lane.pointer("/lifecycle/status")
                        .and_then(Value::as_str)
                        .map(is_terminal_or_merged_host_subagent_status)
                        .unwrap_or(false)
                })
            })
}

pub(crate) fn task_card_required_parallel_fan_in_ready(task_card: &Value) -> Option<bool> {
    required_parallel_fan_in_ready_from_statuses(
        &parallel_required_lane_ids(task_card),
        &parallel_lane_statuses(task_card),
    )
}

pub(crate) fn task_card_has_explicit_subagent_fallback_reason(task_card: &Value) -> bool {
    task_card
        .pointer("/subagent_fallback/reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|reason| !reason.is_empty())
}

fn child_agent_entries_for_task(
    run_record: &Value,
    active_task_card_id: Option<&str>,
) -> Vec<Value> {
    run_record
        .get("child_agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            active_task_card_id.is_none()
                || entry.get("task_card_id").and_then(Value::as_str) == active_task_card_id
        })
        .collect::<Vec<_>>()
}

fn positive_u64(value: &Value) -> Option<u64> {
    value.as_u64().filter(|value| *value > 0).or_else(|| {
        value
            .as_i64()
            .filter(|value| *value > 0)
            .map(|value| value as u64)
    })
}

fn nested_host_subagent_concurrency_config(runtime_config: &Value) -> Option<&Value> {
    runtime_config
        .get("host_subagent_concurrency")
        .filter(|value| value.is_object())
}

fn runtime_positive_limit(
    runtime_config: &Value,
    keys: &[&str],
    default: u64,
) -> (u64, &'static str) {
    for key in keys {
        if let Some(limit) = runtime_config.get(*key).and_then(positive_u64) {
            return (limit, "explicit");
        }
    }
    if let Some(nested) = nested_host_subagent_concurrency_config(runtime_config) {
        for key in keys {
            if let Some(limit) = nested.get(*key).and_then(positive_u64) {
                return (limit, "explicit");
            }
        }
    }
    (default, "default")
}

fn runtime_limit_map(runtime_config: &Value, keys: &[&str]) -> BTreeMap<String, u64> {
    let mut limits = BTreeMap::new();
    for config in [
        Some(runtime_config),
        nested_host_subagent_concurrency_config(runtime_config),
    ]
    .into_iter()
    .flatten()
    {
        for key in keys {
            if let Some(object) = config.get(*key).and_then(Value::as_object) {
                for (name, value) in object {
                    if let Some(limit) = positive_u64(value) {
                        limits.insert(name.to_string(), limit);
                    }
                }
            }
        }
    }
    limits
}

fn host_subagent_lifecycle_thresholds(runtime_config: &Value) -> HostSubagentLifecycleThresholds {
    let stale_after_ms = runtime_config
        .get("worker_stuck_after_ms")
        .and_then(positive_u64)
        .unwrap_or(DEFAULT_HOST_SUBAGENT_RECLAIM_AFTER_MS);
    let stale_after_source = if runtime_config
        .get("worker_stuck_after_ms")
        .and_then(positive_u64)
        .is_some()
    {
        "worker_stuck_after_ms"
    } else {
        "default"
    };
    let (reclaim_after_ms, reclaim_after_source) = runtime_config
        .get("host_subagent_reclaim_after_ms")
        .and_then(positive_u64)
        .map(|value| (value, "host_subagent_reclaim_after_ms"))
        .unwrap_or((stale_after_ms, stale_after_source));

    HostSubagentLifecycleThresholds {
        reclaim_after_ms,
        reclaim_after_source,
        stale_after_ms,
        stale_after_source,
    }
}

fn text_field(value: &Value, pointer_or_key: &str) -> Option<String> {
    let candidate = if pointer_or_key.starts_with('/') {
        value.pointer(pointer_or_key)
    } else {
        value.get(pointer_or_key)
    };
    candidate
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn current_task_model(current_task_card: &Value) -> Option<String> {
    [
        "/runtime_dispatch/model",
        "/delegation_plan/runtime_dispatch/model",
        "/delegation_plan/model",
        "/role_config_snapshot/model",
    ]
    .iter()
    .find_map(|pointer| text_field(current_task_card, pointer))
}

fn host_subagent_entry_model(entry: &Value, fallback_task_card: Option<&Value>) -> Option<String> {
    [
        "model",
        "observed_model",
        "dispatched_model",
        "/latest_model_launch/actual_model",
        "/latest_model_launch/dispatched_model",
        "/latest_model_launch/configured_model",
    ]
    .iter()
    .find_map(|key| text_field(entry, key))
    .or_else(|| fallback_task_card.and_then(current_task_model))
}

fn host_subagent_entry_provider(
    entry: &Value,
    fallback_task_card: Option<&Value>,
) -> Option<String> {
    [
        "provider",
        "observed_provider",
        "model_provider",
        "/runtime_dispatch/provider",
    ]
    .iter()
    .find_map(|key| text_field(entry, key))
    .or_else(|| {
        let current_task_card = fallback_task_card?;
        [
            "/runtime_dispatch/provider",
            "/delegation_plan/runtime_dispatch/provider",
        ]
        .iter()
        .find_map(|key| text_field(current_task_card, key))
    })
    .or_else(|| {
        host_subagent_entry_model(entry, fallback_task_card).map(|model| {
            if model.contains('/') {
                model.split('/').next().unwrap_or("unknown").to_string()
            } else {
                "openai".to_string()
            }
        })
    })
}

fn increment_count(counts: &mut BTreeMap<String, u64>, key: String) {
    *counts.entry(key).or_insert(0) += 1;
}

fn create_limit_visibility_map(
    active_counts: BTreeMap<String, u64>,
    configured_limits: BTreeMap<String, u64>,
    default_limit: u64,
) -> serde_json::Map<String, Value> {
    let mut keys = configured_limits.keys().cloned().collect::<Vec<_>>();
    for key in active_counts.keys() {
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.clone());
        }
    }
    keys.sort();

    keys.into_iter()
        .map(|key| {
            let active_count = active_counts.get(&key).copied().unwrap_or(0);
            let (limit, limit_source) = configured_limits
                .get(&key)
                .copied()
                .map(|limit| (limit, "explicit"))
                .unwrap_or((default_limit, "default"));
            (
                key,
                json!({
                    "limit": limit,
                    "limit_source": limit_source,
                    "active_count": active_count,
                    "remaining_capacity": limit.saturating_sub(active_count),
                    "exceeded": active_count > limit,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>()
}

fn limit_map_exceeded(limit_map: &serde_json::Map<String, Value>) -> bool {
    limit_map
        .values()
        .any(|entry| entry.get("exceeded").and_then(Value::as_bool) == Some(true))
}

fn create_host_subagent_concurrency_payload(
    child_agent_entries: &[Value],
    current_task_card: &Value,
    active_task_card_id: Option<&str>,
    runtime_config: &Value,
) -> Value {
    let (default_provider_limit, provider_limit_source) = runtime_positive_limit(
        runtime_config,
        &[
            "host_subagent_default_provider_concurrency_limit",
            "default_provider_concurrency_limit",
        ],
        DEFAULT_HOST_SUBAGENT_PROVIDER_CONCURRENCY_LIMIT,
    );
    let (default_model_limit, model_limit_source) = runtime_positive_limit(
        runtime_config,
        &[
            "host_subagent_default_model_concurrency_limit",
            "default_model_concurrency_limit",
        ],
        DEFAULT_HOST_SUBAGENT_MODEL_CONCURRENCY_LIMIT,
    );
    let provider_limits = runtime_limit_map(
        runtime_config,
        &[
            "host_subagent_provider_concurrency_limits",
            "provider_concurrency_limits",
        ],
    );
    let model_limits = runtime_limit_map(
        runtime_config,
        &[
            "host_subagent_model_concurrency_limits",
            "model_concurrency_limits",
        ],
    );
    let mut provider_counts = BTreeMap::new();
    let mut model_counts = BTreeMap::new();
    let mut active_count = 0;

    for entry in child_agent_entries.iter().filter(|entry| {
        entry
            .get("status")
            .and_then(Value::as_str)
            .map(is_active_host_subagent_status)
            .unwrap_or(false)
    }) {
        active_count += 1;
        let fallback_task_card = active_task_card_id
            .filter(|task_card_id| {
                entry.get("task_card_id").and_then(Value::as_str) == Some(*task_card_id)
            })
            .map(|_| current_task_card);
        if let Some(provider) = host_subagent_entry_provider(entry, fallback_task_card) {
            increment_count(&mut provider_counts, provider);
        }
        if let Some(model) = host_subagent_entry_model(entry, fallback_task_card) {
            increment_count(&mut model_counts, model);
        }
    }

    let per_provider =
        create_limit_visibility_map(provider_counts, provider_limits, default_provider_limit);
    let per_model = create_limit_visibility_map(model_counts, model_limits, default_model_limit);
    let provider_exceeded = limit_map_exceeded(&per_provider);
    let model_exceeded = limit_map_exceeded(&per_model);

    json!({
        "schema": "ccc.host_subagent_concurrency.v1",
        "active_count": active_count,
        "default_provider_limit": default_provider_limit,
        "default_provider_limit_source": provider_limit_source,
        "default_model_limit": default_model_limit,
        "default_model_limit_source": model_limit_source,
        "per_provider": per_provider,
        "per_model": per_model,
        "provider_exceeded": provider_exceeded,
        "model_exceeded": model_exceeded,
        "exceeded": provider_exceeded || model_exceeded,
    })
}

fn lane_statuses_for_task(
    task_card: &Value,
    child_agent_entries: &[Value],
) -> BTreeMap<String, String> {
    let mut lane_status_by_id = parallel_lane_statuses(task_card);
    for entry in child_agent_entries {
        let lane_id = entry
            .get("lane_id")
            .and_then(Value::as_str)
            .and_then(normalize_parallel_lane_id_for_state);
        let status = entry
            .get("status")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if let (Some(lane_id), Some(status)) = (lane_id, status) {
            lane_status_by_id.insert(lane_id, status);
        }
    }
    lane_status_by_id
}

fn required_parallel_fan_in_ready_from_statuses(
    required_lane_ids: &[String],
    lane_status_by_id: &BTreeMap<String, String>,
) -> Option<bool> {
    if required_lane_ids.is_empty() {
        return None;
    }

    Some(required_lane_ids.iter().all(|lane_id| {
        lane_status_by_id
            .get(lane_id)
            .map(|status| is_terminal_or_merged_host_subagent_status(status))
            .unwrap_or(false)
    }))
}

pub(crate) fn run_record_has_terminal_or_merged_host_subagent_for_task(
    run_record: &Value,
    active_task_card_id: Option<&str>,
) -> bool {
    run_record
        .get("child_agents")
        .and_then(Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                if active_task_card_id.is_some()
                    && entry.get("task_card_id").and_then(Value::as_str) != active_task_card_id
                {
                    return false;
                }

                entry
                    .get("status")
                    .and_then(Value::as_str)
                    .map(is_terminal_or_merged_host_subagent_status)
                    .unwrap_or(false)
            })
        })
}

pub(crate) fn task_card_subagent_fallback_ready(
    run_record: &Value,
    task_card: &Value,
    active_task_card_id: Option<&str>,
) -> bool {
    if !task_card_has_explicit_subagent_fallback_reason(task_card) {
        return false;
    }

    let child_agent_entries = child_agent_entries_for_task(run_record, active_task_card_id);
    let required_lane_ids = parallel_required_lane_ids(task_card);
    let lane_status_by_id = lane_statuses_for_task(task_card, &child_agent_entries);
    if let Some(parallel_ready) =
        required_parallel_fan_in_ready_from_statuses(&required_lane_ids, &lane_status_by_id)
    {
        return parallel_ready;
    }

    task_card_has_terminal_or_merged_host_subagent_state(task_card)
        || run_record_has_terminal_or_merged_host_subagent_for_task(run_record, active_task_card_id)
}

pub(crate) fn next_action_for_host_subagent_status(status: &str) -> &'static str {
    if status == "merged" {
        "advance"
    } else if is_active_host_subagent_status(status) || is_terminal_host_subagent_status(status) {
        "await_fan_in"
    } else {
        "advance"
    }
}

pub(crate) fn phase_name_for_host_subagent_status(task_role: &str, status: &str) -> String {
    if status == "merged" || is_terminal_host_subagent_status(status) {
        "fan_in".to_string()
    } else {
        phase_name_for_role(task_role).to_string()
    }
}

pub(crate) fn update_run_host_subagent_handle_state(
    run_object: &mut serde_json::Map<String, Value>,
    active_task_card_id: &str,
    child_agent_id: &str,
    lane_id: Option<&str>,
    thread_id: Option<&str>,
    status: &str,
    timestamp: &str,
) -> Value {
    if let Some(thread_id) = thread_id {
        let mut raw_thread_ids = run_object
            .get("raw_thread_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        push_unique_value(&mut raw_thread_ids, Value::String(thread_id.to_string()));
        run_object.insert("raw_thread_ids".to_string(), Value::Array(raw_thread_ids));
    }

    let active_agent_id_before = run_object
        .get("active_agent_id")
        .cloned()
        .unwrap_or(Value::Null);
    let active_thread_id_before = run_object
        .get("active_thread_id")
        .cloned()
        .unwrap_or(Value::Null);

    if !is_terminal_or_merged_host_subagent_status(status) {
        if let Some(thread_id) = thread_id {
            run_object.insert(
                "active_thread_id".to_string(),
                Value::String(thread_id.to_string()),
            );
        }
        let cleanup = json!({
            "state": "active",
            "task_card_id": active_task_card_id,
            "child_agent_id": child_agent_id,
            "lane_id": lane_id,
            "thread_id": thread_id,
            "status": status,
            "updated_at": timestamp,
            "summary": format!("Host subagent {child_agent_id} remains active."),
        });
        run_object.insert("host_subagent_handle_cleanup".to_string(), cleanup.clone());
        return cleanup;
    }

    let archived_thread_id = thread_id
        .map(|value| Value::String(value.to_string()))
        .unwrap_or_else(|| active_thread_id_before.clone());
    let thread_id_already_archived = thread_id
        .map(|thread_id| {
            run_object
                .get("host_subagent_handle_archive")
                .and_then(Value::as_array)
                .map(|archive| {
                    archive.iter().any(|entry| {
                        entry.get("task_card_id").and_then(Value::as_str)
                            == Some(active_task_card_id)
                            && entry.get("child_agent_id").and_then(Value::as_str)
                                == Some(child_agent_id)
                            && entry.get("thread_id").and_then(Value::as_str) == Some(thread_id)
                    })
                })
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let had_active_handle = (thread_id.is_some() && !thread_id_already_archived)
        || !active_thread_id_before.is_null()
        || active_agent_id_before.as_str() == Some(child_agent_id);
    run_object.insert("active_thread_id".to_string(), Value::Null);

    let cleanup_state = if had_active_handle {
        "released"
    } else {
        "already_clear"
    };
    let cleanup_summary = if had_active_handle {
        format!("Released host subagent active handle for {child_agent_id} after {status}.")
    } else {
        format!("Host subagent {child_agent_id} already had no active handle after {status}.")
    };
    let host_close_reason = if had_active_handle {
        Value::String(format!(
            "CCC released only the persisted active handle marker; host close_agent is still required for {child_agent_id} when the host API is available."
        ))
    } else {
        Value::Null
    };
    let cleanup = json!({
        "state": cleanup_state,
        "task_card_id": active_task_card_id,
        "child_agent_id": child_agent_id,
        "lane_id": lane_id,
        "thread_id": archived_thread_id,
        "status": status,
        "active_agent_id_before": active_agent_id_before,
        "active_thread_id_before": active_thread_id_before,
        "released_at": timestamp,
        "host_close_required": had_active_handle,
        "host_close_status": if had_active_handle { "host_action_required" } else { "not_required" },
        "host_close_action": if had_active_handle { Value::String("close_agent".to_string()) } else { Value::Null },
        "host_close_reason": host_close_reason,
        "summary": cleanup_summary,
    });

    if had_active_handle {
        let mut archive = run_object
            .get("host_subagent_handle_archive")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        push_unique_value(&mut archive, cleanup.clone());
        run_object.insert(
            "host_subagent_handle_archive".to_string(),
            Value::Array(archive),
        );
    }
    run_object.insert("host_subagent_handle_cleanup".to_string(), cleanup.clone());

    cleanup
}

pub(crate) fn create_host_subagent_state_payload(
    run_record: &Value,
    current_task_card: &Value,
    active_task_card_id: Option<&str>,
    runtime_config: &Value,
) -> Value {
    let filtered = child_agent_entries_for_task(run_record, active_task_card_id);
    let run_wide_child_agent_entries = child_agent_entries_for_task(run_record, None);
    let total = filtered.len();
    let active = filtered
        .iter()
        .filter(|entry| {
            entry
                .get("status")
                .and_then(Value::as_str)
                .map(is_active_host_subagent_status)
                .unwrap_or(false)
        })
        .count();
    let pending_merge = filtered
        .iter()
        .filter(|entry| {
            entry
                .get("status")
                .and_then(Value::as_str)
                .map(is_terminal_host_subagent_status)
                .unwrap_or(false)
        })
        .count();
    let merged = filtered
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("merged"))
        .count();
    let reclaimed = filtered
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("reclaimed"))
        .count();
    let failed_or_stalled_subagents = filtered
        .iter()
        .filter(|entry| {
            matches!(
                entry.get("status").and_then(Value::as_str),
                Some("failed" | "stalled" | "reclaimed")
            )
        })
        .map(|entry| {
            json!({
                "child_agent_id": entry.get("agent_id").cloned().unwrap_or(Value::Null),
                "lane_id": entry.get("lane_id").cloned().unwrap_or(Value::Null),
                "status": entry.get("status").cloned().unwrap_or(Value::Null),
                "thread_id": entry.get("thread_id").cloned().unwrap_or(Value::Null),
                "summary": entry.get("summary").cloned().unwrap_or(Value::Null),
                "updated_at": entry.get("updated_at").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let thread_ids = filtered
        .iter()
        .filter_map(|entry| entry.get("thread_id").and_then(Value::as_str))
        .map(|thread_id| Value::String(thread_id.to_string()))
        .collect::<Vec<_>>();
    let latest = filtered.last().cloned().unwrap_or(Value::Null);
    let required_lane_ids = parallel_required_lane_ids(current_task_card);
    let lane_status_by_id = lane_statuses_for_task(current_task_card, &filtered);
    let terminal_lane_ids = required_lane_ids
        .iter()
        .filter(|lane_id| {
            lane_status_by_id
                .get(*lane_id)
                .map(|status| is_terminal_or_merged_host_subagent_status(status))
                .unwrap_or(false)
        })
        .map(|lane_id| Value::String(lane_id.to_string()))
        .collect::<Vec<_>>();
    let missing_lane_ids = required_lane_ids
        .iter()
        .filter(|lane_id| {
            !lane_status_by_id
                .get(*lane_id)
                .map(|status| is_terminal_or_merged_host_subagent_status(status))
                .unwrap_or(false)
        })
        .map(|lane_id| Value::String(lane_id.to_string()))
        .collect::<Vec<_>>();
    let active_lane_count = required_lane_ids
        .iter()
        .filter(|lane_id| {
            lane_status_by_id
                .get(*lane_id)
                .map(|status| is_active_host_subagent_status(status))
                .unwrap_or(false)
        })
        .count();
    let parallel_fan_in_ready = !required_lane_ids.is_empty() && missing_lane_ids.is_empty();
    let fan_in_ready = if required_lane_ids.is_empty() {
        total > 0 && active == 0 && pending_merge > 0
    } else {
        parallel_fan_in_ready
    };
    let lane_statuses = lane_status_by_id
        .into_iter()
        .map(|(lane_id, status)| (lane_id, Value::String(status)))
        .collect::<serde_json::Map<String, Value>>();
    let lifecycle_thresholds = host_subagent_lifecycle_thresholds(runtime_config);
    let host_subagent_reclaim_after_ms = lifecycle_thresholds.reclaim_after_ms;
    let concurrency = create_host_subagent_concurrency_payload(
        &run_wide_child_agent_entries,
        current_task_card,
        active_task_card_id,
        runtime_config,
    );
    let now = Utc::now();
    let task_title = current_task_card
        .get("title")
        .or_else(|| current_task_card.get("intent"))
        .cloned()
        .unwrap_or(Value::Null);
    let task_scope = current_task_card
        .get("scope")
        .cloned()
        .unwrap_or(Value::Null);
    let subagent_activity = filtered
        .iter()
        .map(|entry| {
            let status = entry
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            json!({
                "child_agent_id": entry.get("agent_id").cloned().unwrap_or(Value::Null),
                "assigned_role": entry.get("role").cloned().unwrap_or(Value::Null),
                "task_card_id": entry.get("task_card_id").cloned().unwrap_or(Value::Null),
                "task_title": task_title.clone(),
                "task_scope": task_scope.clone(),
                "lane_id": entry.get("lane_id").cloned().unwrap_or(Value::Null),
                "status": entry.get("status").cloned().unwrap_or(Value::Null),
                "next_action": next_action_for_host_subagent_status(status),
                "thread_id": entry.get("thread_id").cloned().unwrap_or(Value::Null),
                "summary": entry.get("summary").cloned().unwrap_or(Value::Null),
                "review_outcome": entry.get("review_outcome").cloned().unwrap_or(Value::Null),
                "created_at": entry.get("created_at").cloned().unwrap_or(Value::Null),
                "updated_at": entry.get("updated_at").cloned().unwrap_or(Value::Null),
                "execution_mode": entry.get("execution_mode").cloned().unwrap_or(Value::Null),
                "context_tokens": entry.get("context_tokens").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let active_subagents = filtered
        .iter()
        .filter(|entry| {
            entry
                .get("status")
                .and_then(Value::as_str)
                .map(is_active_host_subagent_status)
                .unwrap_or(false)
        })
        .map(|entry| {
            let updated_at = entry
                .get("updated_at")
                .and_then(Value::as_str)
                .or_else(|| entry.get("created_at").and_then(Value::as_str));
            let elapsed_since_update_ms = updated_at
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| {
                    now.signed_duration_since(value.with_timezone(&Utc))
                        .num_milliseconds()
                        .max(0) as u64
                });
            json!({
                "child_agent_id": entry.get("agent_id").cloned().unwrap_or(Value::Null),
                "assigned_role": entry.get("role").cloned().unwrap_or(Value::Null),
                "task_card_id": entry.get("task_card_id").cloned().unwrap_or(Value::Null),
                "task_title": task_title.clone(),
                "task_scope": task_scope.clone(),
                "lane_id": entry.get("lane_id").cloned().unwrap_or(Value::Null),
                "status": entry.get("status").cloned().unwrap_or(Value::Null),
                "next_action": entry
                    .get("status")
                    .and_then(Value::as_str)
                    .map(next_action_for_host_subagent_status)
                    .unwrap_or("advance"),
                "thread_id": entry.get("thread_id").cloned().unwrap_or(Value::Null),
                "summary": entry.get("summary").cloned().unwrap_or(Value::Null),
                "updated_at": updated_at.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
                "elapsed_since_update_ms": elapsed_since_update_ms.map(Value::from).unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let reclaim_targets = active_subagents
        .iter()
        .filter(|entry| {
            entry
                .get("elapsed_since_update_ms")
                .and_then(Value::as_u64)
                .map(|elapsed| elapsed >= host_subagent_reclaim_after_ms)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let archived_handles = run_record
        .get("host_subagent_handle_archive")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            active_task_card_id.is_none()
                || entry.get("task_card_id").and_then(Value::as_str) == active_task_card_id
        })
        .collect::<Vec<_>>();
    let latest_handle_cleanup_raw = run_record
        .get("host_subagent_handle_cleanup")
        .cloned()
        .unwrap_or(Value::Null);
    let latest_handle_cleanup = if active_task_card_id.is_none()
        || latest_handle_cleanup_raw
            .get("task_card_id")
            .and_then(Value::as_str)
            == active_task_card_id
    {
        latest_handle_cleanup_raw
    } else {
        Value::Null
    };
    let active_handle_cleanup_summary = if active > 0 {
        format!("{active} host subagent handle(s) remain active.")
    } else {
        latest_handle_cleanup
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "No active host subagent handle is retained.".to_string())
    };
    let latest_unconfirmed_released_handle = archived_handles
        .iter()
        .rev()
        .find(|entry| {
            entry
                .get("host_close_acknowledged")
                .and_then(Value::as_bool)
                != Some(true)
        })
        .cloned()
        .unwrap_or(Value::Null);
    let latest_cleanup_requires_host_close =
        latest_handle_cleanup.get("state").and_then(Value::as_str) == Some("released");
    let host_close_source = if latest_cleanup_requires_host_close {
        &latest_handle_cleanup
    } else {
        &latest_unconfirmed_released_handle
    };
    let host_close_required =
        latest_cleanup_requires_host_close || !latest_unconfirmed_released_handle.is_null();
    let host_close_target = host_close_source
        .get("child_agent_id")
        .and_then(Value::as_str)
        .or_else(|| host_close_source.get("thread_id").and_then(Value::as_str));
    let host_close_reason = if host_close_required {
        host_close_target
            .map(|target| {
                Value::String(format!(
                    "CCC released only the persisted active handle marker; host close_agent is still required for {target} when the host API is available."
                ))
            })
            .unwrap_or_else(|| {
                Value::String(
                    "CCC released only the persisted active handle marker; host close_agent is still required when the host API is available."
                        .to_string(),
                )
            })
    } else {
        Value::Null
    };
    let reclaim_replan_summary = if reclaim_targets.is_empty() {
        if active > 0 {
            format!(
                "{active} host subagent lane(s) are still active. Rust CCC cannot cancel host custom subagents directly; wait for fan-in or replan through captain if they stay quiet beyond {host_subagent_reclaim_after_ms} ms."
            )
        } else {
            "No host subagent reclaim/replan action is currently needed.".to_string()
        }
    } else {
        format!(
            "{} host subagent lane(s) appear slow or late. Rust CCC cannot cancel host custom subagents directly; reclaim/replan through captain.",
            reclaim_targets.len()
        )
    };
    let missing_required_lane_count = missing_lane_ids.len();
    let recovery_recommended_action = if !reclaim_targets.is_empty() {
        "reclaim"
    } else if !failed_or_stalled_subagents.is_empty() {
        "retry"
    } else if !required_lane_ids.is_empty()
        && missing_required_lane_count > 0
        && active == 0
        && total > 0
    {
        "reassign"
    } else {
        "none"
    };
    let recovery_requires_operator_attention = recovery_recommended_action != "none";
    let recovery_summary = match recovery_recommended_action {
        "reclaim" => format!(
            "{} active host subagent lane(s) exceeded the reclaim threshold; record reclaim or replan before degraded fallback.",
            reclaim_targets.len()
        ),
        "retry" => format!(
            "{} host subagent lane(s) ended failed, stalled, or reclaimed before clean fan-in; retry or reassign before degraded fallback.",
            failed_or_stalled_subagents.len()
        ),
        "reassign" => format!(
            "{missing_required_lane_count} required host subagent lane(s) have no terminal fan-in; reassign before degraded fallback."
        ),
        _ => "No host subagent recovery action is currently needed.".to_string(),
    };

    json!({
        "total_subagent_count": total,
        "active_subagent_count": active,
        "pending_merge_count": pending_merge,
        "merged_count": merged,
        "reclaimed_count": reclaimed,
        "fan_in_ready": fan_in_ready,
        "parallel_lane_state": {
            "required_lane_ids": required_lane_ids,
            "active_lane_count": active_lane_count,
            "terminal_lane_count": terminal_lane_ids.len(),
            "fan_in_ready": parallel_fan_in_ready,
            "missing_lane_ids": missing_lane_ids,
            "terminal_lane_ids": terminal_lane_ids,
            "lane_statuses": lane_statuses,
        },
        "subagent_activity": subagent_activity,
        "active_subagents": active_subagents,
        "concurrency": concurrency,
        "lifecycle_thresholds": {
            "reclaim_after_ms": lifecycle_thresholds.reclaim_after_ms,
            "reclaim_after_source": lifecycle_thresholds.reclaim_after_source,
            "stale_after_ms": lifecycle_thresholds.stale_after_ms,
            "stale_after_source": lifecycle_thresholds.stale_after_source,
        },
        "reclaim_replan_recommendation": {
            "cancellation_supported": false,
            "reclaim_after_ms": host_subagent_reclaim_after_ms,
            "stale_after_ms": lifecycle_thresholds.stale_after_ms,
            "recommended_action": if reclaim_targets.is_empty() {
                if active > 0 { "await_fan_in_or_replan" } else { "none" }
            } else {
                "reclaim_or_replan"
            },
            "needs_operator_attention": !reclaim_targets.is_empty(),
            "targets": reclaim_targets,
            "summary": reclaim_replan_summary,
        },
        "recovery_recommendation": {
            "recommended_action": recovery_recommended_action,
            "needs_operator_attention": recovery_requires_operator_attention,
            "prefer_before_degraded_fallback": recovery_requires_operator_attention,
            "targets": if recovery_recommended_action == "reclaim" {
                reclaim_targets.clone()
            } else {
                failed_or_stalled_subagents
            },
            "summary": recovery_summary,
        },
        "thread_ids": thread_ids,
        "latest": latest,
        "active_handle_cleanup": {
            "state": latest_handle_cleanup
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or(if active > 0 { "active" } else { "clear" }),
            "active_agent_id": run_record.get("active_agent_id").cloned().unwrap_or(Value::Null),
            "active_thread_id": run_record.get("active_thread_id").cloned().unwrap_or(Value::Null),
            "released_handle_count": archived_handles.len(),
            "latest_released_handle": archived_handles.last().cloned().unwrap_or(Value::Null),
            "latest_cleanup": latest_handle_cleanup,
            "host_close_required": host_close_required,
            "host_close_status": if host_close_required { "host_action_required" } else { "not_required" },
            "host_close_action": if host_close_required { Value::String("close_agent".to_string()) } else { Value::Null },
            "host_close_target": host_close_target.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
            "host_close_reason": host_close_reason,
            "summary": active_handle_cleanup_summary,
        },
    })
}

fn push_unique_value(target: &mut Vec<Value>, candidate: Value) {
    if !target.iter().any(|existing| *existing == candidate) {
        target.push(candidate);
    }
}
