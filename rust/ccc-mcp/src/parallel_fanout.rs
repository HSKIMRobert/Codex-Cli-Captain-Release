use crate::host_subagent_lifecycle::{
    is_active_host_subagent_status, is_terminal_or_merged_host_subagent_status,
};
use crate::request_routing::infer_request_shape;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const RAIDER_LANE_IDS: &[&str] = &["raider-a", "raider-b", "raider-c", "raider-d"];
const SCOUT_LANE_IDS: &[&str] = &["scout-a", "scout-b", "scout-c", "scout-d"];
const DEFAULT_PARALLEL_RAIDER_LANE_COUNT: usize = 2;
const MAX_PARALLEL_RAIDER_LANE_COUNT: usize = 4;
const DEFAULT_PARALLEL_SCOUT_LANE_COUNT: usize = 2;
const MAX_PARALLEL_SCOUT_LANE_COUNT: usize = 4;

fn normalize_raider_lane_id(raw: &str) -> Option<&'static str> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "raider-a" | "lane-a" | "lane_a" | "lane a" | "a" => Some("raider-a"),
        "raider-b" | "lane-b" | "lane_b" | "lane b" | "b" => Some("raider-b"),
        "raider-c" | "lane-c" | "lane_c" | "lane c" | "c" => Some("raider-c"),
        "raider-d" | "lane-d" | "lane_d" | "lane d" | "d" => Some("raider-d"),
        _ => None,
    }
}

fn normalize_scout_lane_id(raw: &str) -> Option<&'static str> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "scout-a" | "scout_a" | "scout a" => Some("scout-a"),
        "scout-b" | "scout_b" | "scout b" => Some("scout-b"),
        "scout-c" | "scout_c" | "scout c" => Some("scout-c"),
        "scout-d" | "scout_d" | "scout d" => Some("scout-d"),
        _ => None,
    }
}

pub(crate) fn normalize_host_lane_id(raw: &str) -> Option<String> {
    normalize_raider_lane_id(raw)
        .map(str::to_string)
        .or_else(|| normalize_scout_lane_id(raw).map(str::to_string))
}

pub(crate) fn supported_host_lane_ids() -> Vec<&'static str> {
    RAIDER_LANE_IDS
        .iter()
        .chain(SCOUT_LANE_IDS.iter())
        .copied()
        .collect()
}

pub(crate) fn normalize_parallel_lane_id_for_state(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    normalize_host_lane_id(trimmed).or_else(|| Some(trimmed.to_ascii_lowercase()))
}

fn parse_explicit_lane_scopes(
    request: &str,
    catalog: &[&str],
    normalize_lane_id: fn(&str) -> Option<&'static str>,
) -> Vec<(String, String)> {
    let mut by_lane = BTreeMap::<String, String>::new();
    for segment in request
        .split('\n')
        .flat_map(|line| line.split(';'))
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some((left, right)) = segment.split_once(':') else {
            continue;
        };
        let Some(lane_id) = normalize_lane_id(left) else {
            continue;
        };
        let scope = right.trim();
        if scope.is_empty() {
            continue;
        }
        by_lane.insert(lane_id.to_string(), scope.to_string());
    }

    catalog
        .iter()
        .filter_map(|lane| {
            by_lane
                .get(*lane)
                .cloned()
                .map(|scope| (lane.to_string(), scope))
        })
        .collect()
}

fn parse_explicit_raider_lane_scopes(request: &str) -> Vec<(String, String)> {
    parse_explicit_lane_scopes(request, RAIDER_LANE_IDS, normalize_raider_lane_id)
}

fn parse_explicit_scout_lane_scopes(request: &str) -> Vec<(String, String)> {
    parse_explicit_lane_scopes(request, SCOUT_LANE_IDS, normalize_scout_lane_id)
}

fn has_explicit_parallel_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "parallel",
        "fan-out",
        "fanout",
        "in parallel",
        "simultaneous",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

pub(crate) fn has_large_mutation_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "large mutation",
        "large refactor",
        "many files",
        "multiple files",
        "multi-file",
        "multi file",
        "several files",
        "broad change",
        "cross-cutting",
        "cross cutting",
        "cross-file",
        "cross file",
        "across the codebase",
        "across files",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

fn has_broad_exploration_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "broad investigation",
        "broad explore",
        "explore broadly",
        "repo-wide",
        "across the repo",
        "across repository",
        "sweep",
        "scan all",
        "deep dive",
        "wide search",
        "parallel scout",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

fn has_single_scope_mutation_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "single file",
        "one file",
        "single-file",
        "single scoped",
        "single scope",
        "shared scope",
        "same file",
        "same scope",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

fn create_parallel_fanout_payload_from_catalog(
    lane_catalog: &[&str],
    explicit_lane_scopes: Vec<(String, String)>,
    lane_count: usize,
    selection_basis: &str,
    default_lane_count: usize,
    max_lane_count: usize,
    disjoint_scope_required: bool,
    disjoint_scope_verified: bool,
    summary: String,
    timestamp: &str,
) -> Value {
    let bounded_lane_count = lane_count
        .max(1)
        .min(lane_catalog.len())
        .min(max_lane_count);
    let required_lane_ids = lane_catalog
        .iter()
        .take(bounded_lane_count)
        .map(|value| Value::String((*value).to_string()))
        .collect::<Vec<_>>();
    let explicit_lane_map = explicit_lane_scopes
        .into_iter()
        .collect::<BTreeMap<String, String>>();
    let lanes = lane_catalog
        .iter()
        .take(bounded_lane_count)
        .map(|lane_id| {
            json!({
                "lane_id": lane_id,
                "required": true,
                "scope": explicit_lane_map.get(*lane_id).map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
                "lifecycle": Value::Null,
                "fan_in": Value::Null,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "mode": if bounded_lane_count > 1 { "parallel" } else { "sequential" },
        "requested_parallel": true,
        "selection_basis": selection_basis,
        "default_lane_count": default_lane_count,
        "max_lane_count": max_lane_count,
        "required_lane_ids": required_lane_ids,
        "all_lane_ids": lane_catalog,
        "disjoint_scope_required": disjoint_scope_required,
        "disjoint_scope_verified": disjoint_scope_verified,
        "summary": summary,
        "lanes": lanes,
        "aggregate": {
            "required_lane_count": bounded_lane_count,
            "active_lane_count": 0,
            "terminal_lane_count": 0,
            "fan_in_ready": false,
            "status": "awaiting_lane_updates",
            "updated_at": timestamp,
        },
        "recorded_at": timestamp,
        "updated_at": timestamp,
    })
}

pub(crate) fn maybe_create_parallel_fanout_payload(
    task_kind: &str,
    assigned_role: &str,
    title: &str,
    intent: &str,
    scope: &str,
    execution_prompt: &str,
    workflow_variant_selection: Option<&Value>,
    timestamp: &str,
) -> Option<Value> {
    let combined_request = [title, intent, scope, execution_prompt].join("\n");
    let workflow_variant = workflow_variant_selection
        .and_then(|value| value.get("workflow_variant"))
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let explicit_parallel = has_explicit_parallel_signal(&combined_request)
        || matches!(
            workflow_variant.as_deref(),
            Some("parallel" | "parallel_fanout")
        );
    if task_kind == "execution" && assigned_role == "code specialist" {
        let broad_mutation = has_large_mutation_signal(&combined_request)
            && infer_request_shape(&combined_request) == "mutation";
        let explicit_lane_scopes = parse_explicit_raider_lane_scopes(&combined_request);
        if !explicit_parallel && !broad_mutation && explicit_lane_scopes.is_empty() {
            return None;
        }

        let disjoint_scope_verified = explicit_lane_scopes.len() >= 2;
        let clearly_single_scope = has_single_scope_mutation_signal(&combined_request);
        let lane_count = if disjoint_scope_verified {
            explicit_lane_scopes
                .len()
                .max(DEFAULT_PARALLEL_RAIDER_LANE_COUNT)
                .min(MAX_PARALLEL_RAIDER_LANE_COUNT)
        } else if clearly_single_scope {
            1
        } else if explicit_parallel || broad_mutation {
            DEFAULT_PARALLEL_RAIDER_LANE_COUNT
        } else {
            1
        };
        let summary = if lane_count > 1 && disjoint_scope_verified {
            format!(
                "Parallel fan-out enabled for bounded raider work across {lane_count} disjoint lane(s)."
            )
        } else if lane_count > 1 {
            format!(
                "Parallel fan-out enabled for broad raider work across {lane_count} lane(s); keep each lane scoped and merge through explicit fan-in."
            )
        } else {
            "Sequential fallback selected because the mutation scope is shared or single-file."
                .to_string()
        };

        return Some(create_parallel_fanout_payload_from_catalog(
            RAIDER_LANE_IDS,
            explicit_lane_scopes,
            lane_count,
            if explicit_parallel {
                "explicit_parallel_signal"
            } else {
                "broad_mutation_signal"
            },
            DEFAULT_PARALLEL_RAIDER_LANE_COUNT,
            MAX_PARALLEL_RAIDER_LANE_COUNT,
            true,
            disjoint_scope_verified,
            summary,
            timestamp,
        ));
    }

    if assigned_role == "explorer" || task_kind == "explore" {
        let broad_explore = has_broad_exploration_signal(&combined_request);
        if !explicit_parallel && !broad_explore {
            return None;
        }
        let explicit_lane_scopes = parse_explicit_scout_lane_scopes(&combined_request);
        let disjoint_scope_verified = explicit_lane_scopes.len() >= 2;
        let lane_count = if disjoint_scope_verified {
            explicit_lane_scopes
                .len()
                .max(DEFAULT_PARALLEL_SCOUT_LANE_COUNT)
                .min(MAX_PARALLEL_SCOUT_LANE_COUNT)
        } else {
            DEFAULT_PARALLEL_SCOUT_LANE_COUNT
        };
        let summary = if disjoint_scope_verified {
            format!(
                "Parallel scout fan-out enabled for read-only exploration across {lane_count} lanes."
            )
        } else {
            format!(
                "Parallel scout fan-out enabled for broad read-only exploration across {lane_count} default lanes."
            )
        };
        return Some(create_parallel_fanout_payload_from_catalog(
            SCOUT_LANE_IDS,
            explicit_lane_scopes,
            lane_count,
            if explicit_parallel {
                "explicit_parallel_signal"
            } else {
                "broad_exploration_signal"
            },
            DEFAULT_PARALLEL_SCOUT_LANE_COUNT,
            MAX_PARALLEL_SCOUT_LANE_COUNT,
            false,
            disjoint_scope_verified,
            summary,
            timestamp,
        ));
    }

    None
}

pub(crate) fn compact_fan_in_fields(value: &Value) -> Value {
    json!({
        "schema": value.get("schema").cloned().unwrap_or(Value::String("ccc.worker_result_envelope.v1".to_string())),
        "summary": value.get("summary").cloned().unwrap_or(Value::Null),
        "status": value.get("status").cloned().unwrap_or(Value::Null),
        "evidence_paths": value.get("evidence_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "next_action": value.get("next_action").cloned().unwrap_or(Value::Null),
        "open_questions": value.get("open_questions").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "confidence": value.get("confidence").cloned().unwrap_or(Value::Null),
        "risk": value.get("risk").cloned().unwrap_or(Value::Null),
        "checks": value.get("checks").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "contract": value.get("contract").cloned().unwrap_or_else(|| json!({
            "captain_consumes_compact_fan_in": true,
        })),
        "artifact_ref": value.get("artifact_ref").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn parallel_required_lane_ids(task_card: &Value) -> Vec<String> {
    task_card
        .pointer("/parallel_fanout/required_lane_ids")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter_map(normalize_parallel_lane_id_for_state)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn parallel_lane_statuses(task_card: &Value) -> BTreeMap<String, String> {
    task_card
        .pointer("/parallel_fanout/lanes")
        .and_then(Value::as_array)
        .map(|lanes| {
            lanes
                .iter()
                .filter_map(|lane| {
                    let lane_id = lane
                        .get("lane_id")
                        .and_then(Value::as_str)
                        .and_then(normalize_parallel_lane_id_for_state)?;
                    let status = lane
                        .pointer("/lifecycle/status")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())?;
                    Some((lane_id.to_string(), status.to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn default_lane_catalog_for_lane_id(lane_id: &str) -> Vec<String> {
    if lane_id.starts_with("scout-") {
        SCOUT_LANE_IDS.iter().map(|lane| lane.to_string()).collect()
    } else {
        RAIDER_LANE_IDS
            .iter()
            .map(|lane| lane.to_string())
            .collect()
    }
}

fn lane_catalog_from_parallel_fanout(
    prior_parallel_fanout: Option<&Value>,
    lane_id: &str,
) -> Vec<String> {
    let mut catalog = prior_parallel_fanout
        .and_then(|value| value.get("all_lane_ids"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter_map(normalize_parallel_lane_id_for_state)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if catalog.is_empty() {
        catalog = default_lane_catalog_for_lane_id(lane_id);
    }
    if !catalog.iter().any(|candidate| candidate == lane_id) {
        catalog.push(lane_id.to_string());
    }
    catalog
}

pub(crate) fn update_parallel_fanout_for_lane(
    prior_parallel_fanout: Option<&Value>,
    lane_id: &str,
    status: &str,
    child_agent_id: &str,
    thread_id: Option<&str>,
    summary: Option<&str>,
    fan_in_compact: &Value,
    timestamp: &str,
) -> Value {
    let lane_catalog = lane_catalog_from_parallel_fanout(prior_parallel_fanout, lane_id);
    let lane_family = if lane_catalog.iter().all(|lane| lane.starts_with("scout-")) {
        "scout"
    } else if lane_catalog.iter().all(|lane| lane.starts_with("raider-")) {
        "raider"
    } else {
        "parallel"
    };
    let default_lane_count = prior_parallel_fanout
        .and_then(|value| value.get("default_lane_count"))
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| {
            if lane_family == "scout" {
                DEFAULT_PARALLEL_SCOUT_LANE_COUNT
            } else {
                DEFAULT_PARALLEL_RAIDER_LANE_COUNT
            }
        });
    let configured_max_lane_count = prior_parallel_fanout
        .and_then(|value| value.get("max_lane_count"))
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| {
            if lane_family == "scout" {
                MAX_PARALLEL_SCOUT_LANE_COUNT
            } else {
                MAX_PARALLEL_RAIDER_LANE_COUNT
            }
        });
    let max_parallel_lane_count = configured_max_lane_count
        .max(1)
        .min(lane_catalog.len().max(1));

    let mut parallel_fanout = prior_parallel_fanout.cloned().unwrap_or_else(|| {
        json!({
            "mode": "sequential",
            "requested_parallel": false,
            "selection_basis": "lane_update_only",
            "default_lane_count": default_lane_count,
            "max_lane_count": max_parallel_lane_count,
            "required_lane_ids": [lane_id],
            "all_lane_ids": lane_catalog.clone(),
            "disjoint_scope_required": lane_family == "raider",
            "disjoint_scope_verified": false,
            "summary": "Lane-aware fan-in state was initialized from subagent updates.",
            "lanes": [],
            "aggregate": {
                "required_lane_count": 1,
                "active_lane_count": 0,
                "terminal_lane_count": 0,
                "fan_in_ready": false,
                "status": "awaiting_lane_updates",
                "updated_at": timestamp,
            },
            "recorded_at": timestamp,
            "updated_at": timestamp,
        })
    });

    let mut required_lane_ids = parallel_fanout
        .get("required_lane_ids")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter_map(normalize_parallel_lane_id_for_state)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if required_lane_ids.is_empty() {
        required_lane_ids.push(lane_id.to_string());
    } else if !required_lane_ids.iter().any(|value| value == lane_id) {
        required_lane_ids.push(lane_id.to_string());
    }
    if required_lane_ids.len() > max_parallel_lane_count {
        required_lane_ids.truncate(max_parallel_lane_count);
    }
    required_lane_ids.sort_by_key(|lane| {
        lane_catalog
            .iter()
            .position(|candidate| candidate == lane)
            .unwrap_or(max_parallel_lane_count)
    });
    required_lane_ids.dedup();

    let mut lanes = parallel_fanout
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for required_lane_id in &required_lane_ids {
        let exists = lanes.iter().any(|entry| {
            entry.get("lane_id").and_then(Value::as_str) == Some(required_lane_id.as_str())
        });
        if !exists {
            lanes.push(json!({
                "lane_id": required_lane_id,
                "required": true,
                "scope": Value::Null,
                "lifecycle": Value::Null,
                "fan_in": Value::Null,
            }));
        }
    }

    let lane_index = lanes
        .iter()
        .position(|entry| entry.get("lane_id").and_then(Value::as_str) == Some(lane_id))
        .unwrap_or_else(|| {
            lanes.push(json!({
                "lane_id": lane_id,
                "required": true,
                "scope": Value::Null,
                "lifecycle": Value::Null,
                "fan_in": Value::Null,
            }));
            lanes.len() - 1
        });

    let mut lane_entry = lanes
        .get(lane_index)
        .cloned()
        .unwrap_or_else(|| json!({ "lane_id": lane_id }));
    if let Some(lane_object) = lane_entry.as_object_mut() {
        lane_object.insert("lane_id".to_string(), Value::String(lane_id.to_string()));
        lane_object.insert("required".to_string(), Value::Bool(true));
        let prior_lifecycle = lane_object
            .get("lifecycle")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut lifecycle = serde_json::Map::new();
        for (key, value) in prior_lifecycle {
            lifecycle.insert(key, value);
        }
        lifecycle.insert("status".to_string(), Value::String(status.to_string()));
        lifecycle.insert(
            "child_agent_id".to_string(),
            Value::String(child_agent_id.to_string()),
        );
        lifecycle.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
        if let Some(thread_id) = thread_id {
            lifecycle.insert(
                "thread_id".to_string(),
                Value::String(thread_id.to_string()),
            );
        }
        if let Some(summary) = summary {
            lifecycle.insert("summary".to_string(), Value::String(summary.to_string()));
        }
        let lifecycle_timestamp_key = match status {
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
                .or_insert_with(|| Value::String(timestamp.to_string()));
        }
        lane_object.insert("lifecycle".to_string(), Value::Object(lifecycle));
        if is_terminal_or_merged_host_subagent_status(status) {
            let mut lane_fan_in = compact_fan_in_fields(fan_in_compact);
            if let Some(fan_in_object) = lane_fan_in.as_object_mut() {
                fan_in_object.insert(
                    "recorded_at".to_string(),
                    Value::String(timestamp.to_string()),
                );
            }
            lane_object.insert("fan_in".to_string(), lane_fan_in);
        }
    }
    lanes[lane_index] = lane_entry;

    let mut active_lane_count = 0_usize;
    let mut terminal_lane_count = 0_usize;
    let mut terminal_lane_ids = Vec::new();
    let mut missing_lane_ids = Vec::new();
    for required_lane_id in &required_lane_ids {
        let lane_status = lanes
            .iter()
            .find(|entry| entry.get("lane_id").and_then(Value::as_str) == Some(required_lane_id))
            .and_then(|entry| entry.pointer("/lifecycle/status"))
            .and_then(Value::as_str);
        match lane_status {
            Some(value) if is_terminal_or_merged_host_subagent_status(value) => {
                terminal_lane_count += 1;
                terminal_lane_ids.push(Value::String(required_lane_id.to_string()));
            }
            Some(value) if is_active_host_subagent_status(value) => {
                active_lane_count += 1;
                missing_lane_ids.push(Value::String(required_lane_id.to_string()));
            }
            _ => {
                missing_lane_ids.push(Value::String(required_lane_id.to_string()));
            }
        }
    }
    let fan_in_ready = !required_lane_ids.is_empty() && missing_lane_ids.is_empty();
    let aggregate_status = if fan_in_ready {
        "ready_for_fan_in"
    } else if active_lane_count > 0 {
        "awaiting_active_lanes"
    } else {
        "awaiting_lane_updates"
    };
    let summary = if fan_in_ready {
        format!(
            "All required {lane_family} lanes reached terminal state ({}/{}) and are ready for fan-in.",
            terminal_lane_count,
            required_lane_ids.len()
        )
    } else {
        format!(
            "Waiting on {} required {lane_family} lane(s) before fan-in readiness.",
            missing_lane_ids.len()
        )
    };

    if let Some(object) = parallel_fanout.as_object_mut() {
        object.insert(
            "mode".to_string(),
            Value::String(if required_lane_ids.len() > 1 {
                "parallel".to_string()
            } else {
                "sequential".to_string()
            }),
        );
        object.insert(
            "default_lane_count".to_string(),
            Value::Number(default_lane_count.into()),
        );
        object.insert(
            "max_lane_count".to_string(),
            Value::Number(max_parallel_lane_count.into()),
        );
        object.insert(
            "required_lane_ids".to_string(),
            Value::Array(
                required_lane_ids
                    .iter()
                    .map(|lane| Value::String(lane.to_string()))
                    .collect(),
            ),
        );
        object.insert(
            "all_lane_ids".to_string(),
            Value::Array(
                lane_catalog
                    .iter()
                    .map(|lane| Value::String(lane.to_string()))
                    .collect(),
            ),
        );
        object.insert("lanes".to_string(), Value::Array(lanes));
        object.insert(
            "aggregate".to_string(),
            json!({
                "required_lane_count": required_lane_ids.len(),
                "active_lane_count": active_lane_count,
                "terminal_lane_count": terminal_lane_count,
                "fan_in_ready": fan_in_ready,
                "missing_lane_ids": missing_lane_ids,
                "terminal_lane_ids": terminal_lane_ids,
                "status": aggregate_status,
                "updated_at": timestamp,
            }),
        );
        object.insert("summary".to_string(), Value::String(summary));
        object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }

    parallel_fanout
}
