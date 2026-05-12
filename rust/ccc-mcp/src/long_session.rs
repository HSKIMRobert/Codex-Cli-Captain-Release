use crate::token_display::format_compact_token_count;
use serde_json::{json, Value};

const CONTEXT_COMPACT_THRESHOLD: u64 = 120_000;
const CONTEXT_NEW_THRESHOLD: u64 = 200_000;
const TOKEN_COMPACT_THRESHOLD: u64 = 200_000;
const ACTIVE_HANDLE_THRESHOLD: u64 = 3;

pub(crate) fn create_long_session_mitigation_payload(
    run_id: &str,
    token_usage: &Value,
    host_subagent_state: &Value,
) -> Value {
    let total_context_tokens = token_usage
        .get("total_context_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_tokens = token_usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let active_subagent_count = host_subagent_state
        .get("active_subagent_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let reclaim_needs_attention = host_subagent_state
        .pointer("/reclaim_replan_recommendation/needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let active_handle_state = host_subagent_state
        .pointer("/active_handle_cleanup/state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let mut signals = Vec::new();
    if total_context_tokens >= CONTEXT_NEW_THRESHOLD {
        signals.push(json!({
            "kind": "context_pressure",
            "severity": "high",
            "value": total_context_tokens,
            "threshold": CONTEXT_NEW_THRESHOLD,
            "summary": format!(
                "estimated context {} reached the new-session threshold",
                format_compact_token_count(total_context_tokens)
            ),
        }));
    } else if total_context_tokens >= CONTEXT_COMPACT_THRESHOLD {
        signals.push(json!({
            "kind": "context_pressure",
            "severity": "medium",
            "value": total_context_tokens,
            "threshold": CONTEXT_COMPACT_THRESHOLD,
            "summary": format!(
                "estimated context {} reached the compact threshold",
                format_compact_token_count(total_context_tokens)
            ),
        }));
    }
    if total_tokens >= TOKEN_COMPACT_THRESHOLD {
        signals.push(json!({
            "kind": "token_pressure",
            "severity": "medium",
            "value": total_tokens,
            "threshold": TOKEN_COMPACT_THRESHOLD,
            "summary": format!(
                "recorded token usage {} reached the compact threshold",
                format_compact_token_count(total_tokens)
            ),
        }));
    }
    if active_subagent_count >= ACTIVE_HANDLE_THRESHOLD || reclaim_needs_attention {
        signals.push(json!({
            "kind": "resource_pressure",
            "severity": "high",
            "active_subagent_count": active_subagent_count,
            "reclaim_needs_attention": reclaim_needs_attention,
            "summary": "host subagent lifecycle pressure should be checkpointed before continuing",
        }));
    }

    let recommended_action = if signals.iter().any(|signal| {
        signal.get("severity").and_then(Value::as_str) == Some("high")
            && signal.get("kind").and_then(Value::as_str) != Some("token_pressure")
    }) {
        "/new"
    } else if signals.is_empty() {
        "continue"
    } else {
        "/compact"
    };
    let recommended = recommended_action != "continue";
    let reason_codes = signals
        .iter()
        .filter_map(|signal| signal.get("kind").and_then(Value::as_str))
        .map(|value| Value::String(value.to_string()))
        .collect::<Vec<_>>();
    let resume_command = format!("$cap continue {run_id}");
    let checkpoint_command = format!("ccc orchestrate --quiet --json '{{\"run_id\":\"{run_id}\",\"compact\":true,\"resolve_summary\":\"Checkpoint before Codex CLI session rollover.\"}}'");

    json!({
        "recommended": recommended,
        "recommended_action": recommended_action,
        "reason_codes": reason_codes,
        "signals": signals,
        "operator_choice_required": recommended,
        "checkpoint_required": recommended,
        "checkpoint_command": if recommended { Value::String(checkpoint_command) } else { Value::Null },
        "resume_command": if recommended { Value::String(resume_command.clone()) } else { Value::Null },
        "resume_prompt": if recommended {
            Value::String(format!(
                "After the chosen Codex CLI action, resume this CCC run with `{resume_command}`."
            ))
        } else {
            Value::Null
        },
        "slash_command_boundary": "captain recommends /compact, /new, or /exit but does not claim to execute Codex TUI slash commands without Codex CLI or wrapper support",
        "choices": [
            {
                "action": "/compact",
                "when": "summarize the current visible conversation and keep working in the current CLI session"
            },
            {
                "action": "/new",
                "when": "start a fresh conversation in the same CLI after checkpointing the CCC run"
            },
            {
                "action": "/exit",
                "when": "fully restart Codex CLI after checkpointing the CCC run"
            }
        ],
        "policy": {
            "context_compact_threshold": CONTEXT_COMPACT_THRESHOLD,
            "context_new_threshold": CONTEXT_NEW_THRESHOLD,
            "token_compact_threshold": TOKEN_COMPACT_THRESHOLD,
            "active_handle_threshold": ACTIVE_HANDLE_THRESHOLD,
            "active_handle_state": active_handle_state
        }
    })
}

#[cfg(test)]
pub(crate) fn long_session_mitigation_summary(payload: &Value) -> Option<String> {
    let mitigation = payload.get("long_session_mitigation")?;
    if !mitigation
        .get("recommended")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let action = mitigation
        .get("recommended_action")
        .and_then(Value::as_str)
        .unwrap_or("/compact");
    let reasons = mitigation
        .get("reason_codes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("+")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "session_pressure".to_string());
    let resume = mitigation
        .get("resume_command")
        .and_then(Value::as_str)
        .unwrap_or("$cap continue <run_id>");
    Some(format!(
        "Rollover: recommend {action} reason={reasons}; operator choice required; checkpoint before rollover; resume with `{resume}`"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_session_recommends_compact_for_context_pressure() {
        let payload = create_long_session_mitigation_payload(
            "run-123",
            &json!({ "total_context_tokens": 125_000, "total_tokens": 0 }),
            &json!({ "active_subagent_count": 0 }),
        );

        assert_eq!(payload["recommended"], true);
        assert_eq!(payload["recommended_action"], "/compact");
        assert_eq!(payload["operator_choice_required"], true);
        assert_eq!(payload["resume_command"], "$cap continue run-123");
    }

    #[test]
    fn long_session_recommends_new_for_high_resource_pressure() {
        let payload = create_long_session_mitigation_payload(
            "run-456",
            &json!({ "total_context_tokens": 0, "total_tokens": 0 }),
            &json!({ "active_subagent_count": 3 }),
        );

        assert_eq!(payload["recommended"], true);
        assert_eq!(payload["recommended_action"], "/new");
        assert!(payload["slash_command_boundary"]
            .as_str()
            .unwrap()
            .contains("does not claim to execute"));
    }

    #[test]
    fn long_session_has_no_recommendation_when_pressure_is_low() {
        let payload = create_long_session_mitigation_payload(
            "run-789",
            &json!({ "total_context_tokens": 10_000, "total_tokens": 20_000 }),
            &json!({ "active_subagent_count": 0 }),
        );

        assert_eq!(payload["recommended"], false);
        assert_eq!(payload["recommended_action"], "continue");
        assert!(payload["resume_command"].is_null());
    }
}
