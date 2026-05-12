use serde_json::{json, Value};

#[derive(Clone, Copy)]
struct HookTier {
    id: &'static str,
    lifecycle_point: &'static str,
    boundary: &'static str,
    affects: &'static [&'static str],
}

const HOOK_TIERS: &[HookTier] = &[
    HookTier {
        id: "planning",
        lifecycle_point: "before_planning_route",
        boundary: "longway_planning",
        affects: &["routing"],
    },
    HookTier {
        id: "recovery",
        lifecycle_point: "before_recovery_route",
        boundary: "host_subagent_recovery",
        affects: &["routing", "verification"],
    },
    HookTier {
        id: "compaction",
        lifecycle_point: "before_context_rollover",
        boundary: "codex_cli_session_boundary",
        affects: &["routing", "verification"],
    },
    HookTier {
        id: "tool_guard",
        lifecycle_point: "before_tool_mutation",
        boundary: "captain_direct_mutation_guard",
        affects: &["mutation", "verification"],
    },
    HookTier {
        id: "continuation",
        lifecycle_point: "before_resume_or_advance",
        boundary: "active_checkpoint",
        affects: &["routing"],
    },
    HookTier {
        id: "fan_in",
        lifecycle_point: "before_fan_in_merge",
        boundary: "fan_in_barrier",
        affects: &["routing", "verification"],
    },
    HookTier {
        id: "review",
        lifecycle_point: "before_acceptance_gate",
        boundary: "arbiter_review",
        affects: &["verification"],
    },
    HookTier {
        id: "reporting",
        lifecycle_point: "before_status_projection",
        boundary: "operator_status_reporting",
        affects: &["verification"],
    },
    HookTier {
        id: "notification",
        lifecycle_point: "after_lifecycle_decision",
        boundary: "status_notice",
        affects: &["routing", "mutation", "verification"],
    },
];

pub(crate) fn create_lifecycle_hook_tiers_payload(
    runtime_config: &Value,
    current_task_card: &Value,
    longway: &Value,
    run_truth_surface: &Value,
    active_checkpoint: &Value,
    recovery_lane: &Value,
    long_session_mitigation: &Value,
    captain_direct_mutation_guard: &Value,
    latest_delegate_result: &Value,
) -> Value {
    let hooks_config = runtime_config
        .get("lifecycle_hooks")
        .filter(|value| value.is_object())
        .unwrap_or(&Value::Null);
    let globally_enabled = hooks_config
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let decisions = HOOK_TIERS
        .iter()
        .map(|tier| {
            create_hook_decision(
                tier,
                hooks_config,
                globally_enabled,
                current_task_card,
                longway,
                run_truth_surface,
                active_checkpoint,
                recovery_lane,
                long_session_mitigation,
                captain_direct_mutation_guard,
                latest_delegate_result,
            )
        })
        .collect::<Vec<_>>();
    let active_tiers = decision_tiers_with_status(&decisions, "decision");
    let skipped_tiers = decision_tiers_with_status(&decisions, "skipped");
    let failed_tiers = decision_tiers_with_status(&decisions, "failed");
    let impacting_decision_count = active_tiers.len();
    let failure_count = failed_tiers.len();

    json!({
        "schema": "ccc.lifecycle_hook_tiers.v1",
        "owner": "rust_policy",
        "public_commands": false,
        "policy_source": if hooks_config.is_object() { "runtime_config.lifecycle_hooks" } else { "ccc_default_policy" },
        "tiers": hook_tier_definitions(),
        "decisions": decisions,
        "active_tiers": active_tiers,
        "skipped_tiers": skipped_tiers,
        "failed_tiers": failed_tiers,
        "impacting_decision_count": impacting_decision_count,
        "failure_count": failure_count,
        "status": if failure_count > 0 {
            "failed"
        } else if impacting_decision_count > 0 {
            "active"
        } else {
            "clear"
        },
    })
}

fn create_hook_decision(
    tier: &HookTier,
    hooks_config: &Value,
    globally_enabled: bool,
    current_task_card: &Value,
    longway: &Value,
    run_truth_surface: &Value,
    active_checkpoint: &Value,
    recovery_lane: &Value,
    long_session_mitigation: &Value,
    captain_direct_mutation_guard: &Value,
    latest_delegate_result: &Value,
) -> Value {
    if !globally_enabled {
        return hook_decision(
            tier,
            "skipped",
            "disabled_by_policy",
            "lifecycle hooks disabled",
        );
    }
    if let Some(failure) = config_failure_decision(tier, hooks_config) {
        return failure;
    }

    match tier.id {
        "planning" => planning_decision(tier, longway),
        "recovery" => recovery_decision(tier, recovery_lane),
        "compaction" => compaction_decision(tier, long_session_mitigation),
        "tool_guard" => tool_guard_decision(tier, captain_direct_mutation_guard),
        "continuation" => continuation_decision(tier, active_checkpoint),
        "fan_in" => fan_in_decision(tier, run_truth_surface),
        "review" => review_decision(tier, current_task_card),
        "reporting" => reporting_decision(tier, current_task_card, latest_delegate_result),
        "notification" => notification_decision(
            tier,
            recovery_lane,
            long_session_mitigation,
            captain_direct_mutation_guard,
        ),
        _ => hook_decision(tier, "skipped", "unknown_tier", "tier is not recognized"),
    }
}

fn config_failure_decision(tier: &HookTier, hooks_config: &Value) -> Option<Value> {
    let tier_config = hooks_config.get(tier.id)?;
    if let Some(enabled) = tier_config.get("enabled") {
        if enabled.as_bool() == Some(false) {
            return Some(hook_decision(
                tier,
                "skipped",
                "disabled_by_policy",
                "tier disabled by lifecycle hook policy",
            ));
        }
        if !enabled.is_boolean() {
            return Some(hook_decision(
                tier,
                "failed",
                "invalid_policy",
                "`enabled` must be a boolean",
            ));
        }
    }

    // 0.0.15-pre defines lifecycle tiers only; command execution remains outside
    // the hook policy so hooks cannot become a parallel operator command surface.
    if tier_config.get("command").is_some() || tier_config.get("commands").is_some() {
        return Some(hook_decision(
            tier,
            "failed",
            "user_facing_hook_commands_unsupported",
            "hook commands are not supported in 0.0.15-pre",
        ));
    }

    None
}

fn planning_decision(tier: &HookTier, longway: &Value) -> Value {
    let lifecycle_state = text_value(longway, "/lifecycle_state").unwrap_or("unknown");
    let active_status = text_value(longway, "/active_phase_status").unwrap_or("unknown");
    if matches!(lifecycle_state, "planned" | "planning" | "active")
        || matches!(active_status, "pending_longway_approval" | "in_progress")
    {
        return hook_decision(
            tier,
            "decision",
            "route_planning_boundary",
            "LongWay planning boundary affects routing",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "no_planning_boundary",
        "no planning route change",
    )
}

fn recovery_decision(tier: &HookTier, recovery_lane: &Value) -> Value {
    let status = text_value(recovery_lane, "/status").unwrap_or("clear");
    let action = text_value(recovery_lane, "/recommended_action").unwrap_or("none");
    if matches!(status, "recovery_pending" | "reclaim_pending") || action != "none" {
        return hook_decision(
            tier,
            "decision",
            action,
            "recovery lane changed the next routing action",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "no_recovery_action",
        "recovery lane is clear",
    )
}

fn compaction_decision(tier: &HookTier, long_session_mitigation: &Value) -> Value {
    if long_session_mitigation
        .get("recommended")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let action =
            text_value(long_session_mitigation, "/recommended_action").unwrap_or("/compact");
        return hook_decision(
            tier,
            "decision",
            action,
            "context pressure requires checkpoint before Codex CLI rollover",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "no_context_pressure",
        "compaction boundary is clear",
    )
}

fn tool_guard_decision(tier: &HookTier, guard: &Value) -> Value {
    let state = text_value(guard, "/state").unwrap_or("unknown");
    if matches!(
        state,
        "blocked_unrecorded_direct_mutation" | "exception_recorded"
    ) {
        return hook_decision(
            tier,
            "decision",
            state,
            "mutation guard affects file mutation or verification routing",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "mutation_guard_clear",
        "no mutation guard route change",
    )
}

fn continuation_decision(tier: &HookTier, active_checkpoint: &Value) -> Value {
    if !active_checkpoint.is_object() {
        return hook_decision(
            tier,
            "skipped",
            "no_active_checkpoint",
            "no active run checkpoint",
        );
    }
    let resume_action = text_value(active_checkpoint, "/resume_action").unwrap_or("advance");
    hook_decision(
        tier,
        "decision",
        resume_action,
        "active checkpoint owns continuation routing",
    )
}

fn fan_in_decision(tier: &HookTier, run_truth_surface: &Value) -> Value {
    if run_truth_surface
        .get("fan_in_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return hook_decision(
            tier,
            "decision",
            "fan_in_ready",
            "fan-in barrier is ready for captain merge/review",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "fan_in_not_ready",
        "fan-in barrier is not ready",
    )
}

fn review_decision(tier: &HookTier, current_task_card: &Value) -> Value {
    if current_task_card.get("review_policy").is_some()
        || current_task_card.get("review_fan_in").is_some()
        || current_task_card.get("orchestrator_review_gate").is_some()
    {
        return hook_decision(
            tier,
            "decision",
            "review_boundary_present",
            "review policy or fan-in affects verification",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "no_review_boundary",
        "no review boundary is active",
    )
}

fn reporting_decision(
    tier: &HookTier,
    current_task_card: &Value,
    latest_delegate_result: &Value,
) -> Value {
    if latest_delegate_result.is_object()
        || current_task_card.get("subagent_fan_in").is_some()
        || current_task_card.get("review_fan_in").is_some()
    {
        return hook_decision(
            tier,
            "decision",
            "report_status_update",
            "fan-in or delegate result affects verification visibility",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "nothing_to_report",
        "no reporting route change",
    )
}

fn notification_decision(
    tier: &HookTier,
    recovery_lane: &Value,
    long_session_mitigation: &Value,
    captain_direct_mutation_guard: &Value,
) -> Value {
    let recovery_attention = recovery_lane
        .get("needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rollover_attention = long_session_mitigation
        .get("operator_choice_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let guard_blocked = text_value(captain_direct_mutation_guard, "/state")
        == Some("blocked_unrecorded_direct_mutation");
    if recovery_attention || rollover_attention || guard_blocked {
        return hook_decision(
            tier,
            "decision",
            "emit_status_notice",
            "operator-visible status notice is required",
        );
    }
    hook_decision(
        tier,
        "skipped",
        "no_notice_required",
        "no notification-affecting event",
    )
}

fn hook_decision(tier: &HookTier, status: &str, action: &str, reason: &str) -> Value {
    json!({
        "tier": tier.id,
        "status": status,
        "action": action,
        "reason": reason,
        "lifecycle_point": tier.lifecycle_point,
        "boundary": tier.boundary,
        "affects": tier.affects,
    })
}

fn hook_tier_definitions() -> Value {
    Value::Array(
        HOOK_TIERS
            .iter()
            .map(|tier| {
                json!({
                    "tier": tier.id,
                    "lifecycle_point": tier.lifecycle_point,
                    "boundary": tier.boundary,
                    "affects": tier.affects,
                    "owner": "rust_policy",
                    "public_command": false,
                })
            })
            .collect(),
    )
}

fn decision_tiers_with_status(decisions: &[Value], status: &str) -> Vec<String> {
    decisions
        .iter()
        .filter(|decision| decision.get("status").and_then(Value::as_str) == Some(status))
        .filter_map(|decision| decision.get("tier").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn text_value<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
