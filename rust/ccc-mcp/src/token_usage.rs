use crate::read_json_document;
use crate::worker_events::resolve_delegation_token_usage;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

fn token_usage_total_count(usage: &Value) -> u64 {
    usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .or_else(|| {
            Some(
                usage
                    .get("input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + usage
                        .get("cached_input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                    + usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                    + usage
                        .get("reasoning_output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
            )
        })
        .unwrap_or(0)
}

pub(crate) fn create_token_usage_payload(run_directory: &Path) -> io::Result<Value> {
    let delegations_directory = run_directory.join("delegations");
    let entries = match fs::read_dir(&delegations_directory) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    let mut by_agent: BTreeMap<String, (u64, u64, u64, u64, u64, u64)> = BTreeMap::new();
    let mut total_input = 0_u64;
    let mut total_cached_input = 0_u64;
    let mut total_output = 0_u64;
    let mut total_reasoning = 0_u64;
    let mut total_tokens = 0_u64;
    let mut total_context_tokens = 0_u64;

    if let Some(entries) = entries {
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

            let delegation = read_json_document(&path)?;
            let Some(usage) = resolve_delegation_token_usage(run_directory, &delegation) else {
                continue;
            };

            let agent_id = delegation
                .get("child_agent")
                .and_then(|value| value.get("agent_id"))
                .and_then(Value::as_str)
                .unwrap_or("worker")
                .to_string();
            let input_tokens = usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let cached_input_tokens = usage
                .get("cached_input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let output_tokens = usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let reasoning_output_tokens = usage
                .get("reasoning_output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let usage_total = token_usage_total_count(&usage);

            total_input += input_tokens;
            total_cached_input += cached_input_tokens;
            total_output += output_tokens;
            total_reasoning += reasoning_output_tokens;
            total_tokens += usage_total;

            let entry = by_agent.entry(agent_id).or_insert((0, 0, 0, 0, 0, 0));
            entry.0 += input_tokens;
            entry.1 += cached_input_tokens;
            entry.2 += output_tokens;
            entry.3 += reasoning_output_tokens;
            entry.4 += usage_total;
        }
    }

    let run_record = read_json_document(&run_directory.join("run.json")).unwrap_or(Value::Null);
    if let Some(child_agents) = run_record.get("child_agents").and_then(Value::as_array) {
        for child in child_agents {
            let Some(usage) = child
                .get("total_token_usage")
                .filter(|value| value.is_object())
            else {
                continue;
            };
            let agent_id = child
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or("host_subagent")
                .to_string();
            let input_tokens = usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let cached_input_tokens = usage
                .get("cached_input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let output_tokens = usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let reasoning_output_tokens = usage
                .get("reasoning_output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let usage_total = token_usage_total_count(usage);

            total_input += input_tokens;
            total_cached_input += cached_input_tokens;
            total_output += output_tokens;
            total_reasoning += reasoning_output_tokens;
            let context_tokens = child
                .get("context_tokens")
                .or_else(|| child.get("estimated_context_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0);

            total_tokens += usage_total;
            total_context_tokens += context_tokens;

            let entry = by_agent.entry(agent_id).or_insert((0, 0, 0, 0, 0, 0));
            entry.0 += input_tokens;
            entry.1 += cached_input_tokens;
            entry.2 += output_tokens;
            entry.3 += reasoning_output_tokens;
            entry.4 += usage_total;
            entry.5 += context_tokens;
        }
    }

    if let Some(child_agents) = run_record.get("child_agents").and_then(Value::as_array) {
        for child in child_agents {
            if child
                .get("total_token_usage")
                .filter(|value| value.is_object())
                .is_some()
            {
                continue;
            }
            let context_tokens = child
                .get("context_tokens")
                .or_else(|| child.get("estimated_context_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if context_tokens == 0 {
                continue;
            }
            let agent_id = child
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or("host_subagent")
                .to_string();
            total_context_tokens += context_tokens;
            let entry = by_agent.entry(agent_id).or_insert((0, 0, 0, 0, 0, 0));
            entry.5 += context_tokens;
        }
    }

    if by_agent.is_empty() {
        return Ok(Value::Null);
    }

    let by_agent_totals = by_agent
        .iter()
        .map(
            |(agent_id, (_, _, _, _, agent_total_tokens, context_tokens))| {
                json!({
                    "agent_id": agent_id,
                    "total_tokens": agent_total_tokens,
                    "context_tokens": context_tokens,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(json!({
        "source": "delegation_and_host_subagent_usage_best_effort",
        "captain_tokens_available": false,
        "input_tokens": total_input,
        "cached_input_tokens": total_cached_input,
        "output_tokens": total_output,
        "reasoning_output_tokens": total_reasoning,
        "total_tokens": total_tokens,
        "total_context_tokens": total_context_tokens,
        "by_subagent": by_agent_totals,
        "by_agent": by_agent.into_iter().map(
            |(agent_id, (input_tokens, cached_input_tokens, output_tokens, reasoning_output_tokens, agent_total_tokens, context_tokens))| {
                json!({
                    "agent_id": agent_id,
                    "input_tokens": input_tokens,
                    "cached_input_tokens": cached_input_tokens,
                    "output_tokens": output_tokens,
                    "reasoning_output_tokens": reasoning_output_tokens,
                    "total_tokens": agent_total_tokens,
                    "context_tokens": context_tokens,
                })
            }
        ).collect::<Vec<_>>(),
    }))
}

pub(crate) fn create_token_usage_visibility_payload(
    token_usage: &Value,
    host_subagent_state: &Value,
) -> Value {
    let host_subagent_count = host_subagent_state
        .get("total_subagent_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_tokens = token_usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_context_tokens = token_usage
        .get("total_context_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let has_non_zero_usage = token_usage
        .get("by_subagent")
        .and_then(Value::as_array)
        .or_else(|| token_usage.get("by_agent").and_then(Value::as_array))
        .map(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("total_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    > 0
            })
        })
        .unwrap_or(false);

    if total_tokens > 0 || has_non_zero_usage {
        return json!({
            "status": "available",
            "available": true,
            "source": token_usage
                .get("source")
                .cloned()
                .unwrap_or(Value::String("delegation_raw_events_best_effort".to_string())),
            "captain_tokens_available": token_usage
                .get("captain_tokens_available")
                .cloned()
                .unwrap_or(Value::Bool(false)),
            "unavailable_reason": Value::Null,
            "unavailable_reason_code": Value::Null,
        });
    }

    if total_context_tokens > 0 {
        return json!({
            "status": "context_available",
            "available": true,
            "source": token_usage
                .get("source")
                .cloned()
                .unwrap_or(Value::String("host_subagent_context_best_effort".to_string())),
            "captain_tokens_available": false,
            "token_totals_available": false,
            "context_tokens_available": true,
            "unavailable_reason": "raw token usage unavailable; host subagent context estimates were recorded",
            "unavailable_reason_code": "raw_token_usage_unavailable_context_estimate_available",
        });
    }

    let (reason, reason_code) = if host_subagent_count > 0 {
        (
            "host custom subagents did not supply raw usage events",
            "host_custom_subagents_no_raw_usage_events",
        )
    } else {
        (
            "no raw delegated-worker usage events were captured for this run yet",
            "no_raw_usage_events",
        )
    };

    json!({
        "status": "unavailable",
        "available": false,
        "source": "none",
        "captain_tokens_available": false,
        "unavailable_reason": reason,
        "unavailable_reason_code": reason_code,
    })
}
