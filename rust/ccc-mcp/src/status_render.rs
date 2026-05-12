#[cfg(test)]
use crate::long_session::long_session_mitigation_summary;
use crate::specialist_roles::{load_output_config, status_display_agent, status_display_role};
use crate::text_utils::summarize_text_for_visibility;
#[cfg(test)]
use crate::token_display::{
    build_context_usage_bar, build_context_usage_breakdown, build_token_usage_bar,
    build_token_usage_breakdown, format_compact_token_count, token_context_total,
    token_usage_by_agent,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(crate) fn create_visibility_signature(payload: &Value) -> String {
    let text = create_ccc_status_operator_text(payload);
    let digest = Sha256::digest(text.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub(crate) fn build_operator_projection_status_block(payload: &Value) -> Option<String> {
    let projection = payload
        .get("operator_longway_projection")
        .or_else(|| payload.get("longway_projection"))
        .filter(|value| value.is_object())?;
    if matches!(
        projection.get("status").and_then(Value::as_str),
        Some("removed" | "absent")
    ) {
        return None;
    }

    let path = projection
        .get("path")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            if value.ends_with("CCC_LONGWAY_PROJECTION.md") {
                "CCC_LONGWAY_PROJECTION.md"
            } else {
                value
            }
        })
        .unwrap_or("CCC_LONGWAY_PROJECTION.md");
    let projection_command = payload
        .get("run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|run_id| format!("ccc status --projection --json '{{\"run_id\":\"{run_id}\"}}'"))
        .unwrap_or_else(|| "ccc status --projection --json '{...}'".to_string());
    let mut lines = vec![
        format!("LongWay Projection: {path}"),
        format!("Progress: view {path} or refresh with {projection_command}"),
    ];
    if let Some(diff_command) = projection
        .pointer("/diff_visibility/diff_command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("Diff: {diff_command}"));
    }

    Some(lines.join("\n"))
}

fn operator_status_intro_lines(payload: &Value) -> Vec<String> {
    let mut lines = build_operator_projection_status_block(payload)
        .or_else(|| build_longway_checklist_block(payload))
        .map(|block| block.lines().map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["LongWay".to_string()]);
    if let Some(prompt_refinement_line) = build_prompt_refinement_line(payload) {
        lines.push(prompt_refinement_line);
    }
    lines
}

fn build_prompt_refinement_line(payload: &Value) -> Option<String> {
    let prompt_refinement = payload
        .get("prompt_refinement")
        .filter(|value| value.is_object())?;
    let state =
        compact_text_field(prompt_refinement, "state").unwrap_or_else(|| "unknown".to_string());
    let enabled = prompt_refinement
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let execution_mode = compact_text_field(prompt_refinement, "execution_mode")
        .unwrap_or_else(|| "internal".to_string());
    let owner =
        compact_text_field(prompt_refinement, "owner").unwrap_or_else(|| "captain".to_string());
    let captain_gate = compact_text_field(prompt_refinement, "captain_gate")
        .unwrap_or_else(|| "accept_adjust_reject".to_string());
    let source =
        compact_text_field(prompt_refinement, "source").unwrap_or_else(|| "internal".to_string());
    let longway_allowed = prompt_refinement
        .get("longway_materialization_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let task_card_creation_allowed = prompt_refinement
        .get("task_card_creation_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(format!(
        "Prompt Refinement: state={state} enabled={enabled} mode={execution_mode} owner={owner} gate={captain_gate} longway={longway_allowed} task_cards={task_card_creation_allowed} source={source}"
    ))
}

fn next_actor_label(payload: &Value, next_step: &str, fan_in_ready: bool) -> String {
    if fan_in_ready || next_step == "advance" {
        return "captain".to_string();
    }
    if matches!(
        next_step,
        "halt_completed" | "halt_failed" | "halt_cancelled"
    ) {
        return "completed".to_string();
    }
    if next_step == "await_operator" {
        return "operator".to_string();
    }
    if next_step == "execute_task" {
        let agent = payload
            .get("current_task_card")
            .and_then(|value| value.get("assigned_agent_id"))
            .and_then(Value::as_str)
            .unwrap_or("worker")
            .to_string();
        return status_display_agent(&agent);
    }
    next_step.to_string()
}

#[cfg(test)]
fn build_agent_loop_line(payload: &Value) -> Option<String> {
    let assigned_agent_id = payload
        .get("current_task_card")
        .and_then(|value| value.get("assigned_agent_id"))
        .and_then(Value::as_str)
        .unwrap_or("worker");
    let model = payload
        .get("current_task_card")
        .and_then(|value| value.get("latest_model_launch"))
        .and_then(|value| value.get("dispatched_model"))
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("current_task_card")
                .and_then(|value| value.get("role_config_snapshot"))
                .and_then(|value| value.get("model"))
                .and_then(Value::as_str)
        });
    let variant = payload
        .get("current_task_card")
        .and_then(|value| value.get("latest_model_launch"))
        .and_then(|value| value.get("dispatched_variant"))
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("current_task_card")
                .and_then(|value| value.get("role_config_snapshot"))
                .and_then(|value| value.get("variant"))
                .and_then(Value::as_str)
        });

    match (model, variant) {
        (Some(model), Some(variant)) => Some(format!(
            "Agent Loop: CCC launched {} [{model}/{variant}]",
            status_display_agent(assigned_agent_id)
        )),
        (Some(model), None) => Some(format!(
            "Agent Loop: CCC launched {} [{model}]",
            status_display_agent(assigned_agent_id)
        )),
        _ => Some(format!(
            "Agent Loop: CCC launched {}",
            status_display_agent(assigned_agent_id)
        )),
    }
}

#[cfg(test)]
fn build_launch_visibility_line(payload: &Value) -> Option<String> {
    let current_task = payload.get("current_task_card")?;
    if !current_task.is_object() {
        return None;
    }

    let assigned_role = current_task
        .get("assigned_role")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let parallel_fanout = current_task
        .pointer("/parallel_fanout")
        .filter(|value| value.is_object());
    let parallel_mode = parallel_fanout
        .and_then(|value| value.get("mode"))
        .and_then(Value::as_str)
        .map(|value| value == "parallel")
        .unwrap_or(false);
    let lane_ids = if parallel_mode {
        current_task
            .pointer("/parallel_fanout/required_lane_ids")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .filter(|value| !value.trim().is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let agent_id = current_task
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("worker");
    let launch_label = if parallel_mode {
        if lane_ids.is_empty() {
            "lane=unspecified".to_string()
        } else {
            format!("lane={}", lane_ids.join(","))
        }
    } else {
        format!("agent={}", status_display_agent(agent_id))
    };
    let scope = current_task
        .get("scope")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unspecified");
    let expected_fan_in_fields = current_task
        .pointer("/delegation_plan/fan_in_contract/required_fields")
        .and_then(Value::as_array)
        .or_else(|| {
            current_task
                .pointer("/subagent_contract/fan_in_fields")
                .and_then(Value::as_array)
        })
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "summary".to_string(),
                "status".to_string(),
                "evidence_paths".to_string(),
                "next_action".to_string(),
                "open_questions".to_string(),
                "confidence".to_string(),
            ]
        });
    let task_stance = current_task
        .pointer("/expertise_framing/task_stance")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task
                .pointer("/delegation_plan/expertise_framing/task_stance")
                .and_then(Value::as_str)
        });
    let thinking_mode = current_task
        .pointer("/expertise_framing/expected_thinking_mode")
        .and_then(Value::as_str)
        .or_else(|| {
            current_task
                .pointer("/delegation_plan/expertise_framing/expected_thinking_mode")
                .and_then(Value::as_str)
        });
    let expertise_suffix = match (task_stance, thinking_mode) {
        (Some(task_stance), Some(thinking_mode)) => {
            format!(" stance={task_stance} thinking={thinking_mode}")
        }
        (Some(task_stance), None) => format!(" stance={task_stance}"),
        (None, Some(thinking_mode)) => format!(" thinking={thinking_mode}"),
        (None, None) => String::new(),
    };
    Some(format!(
        "Launch: role={} {launch_label} scope=\"{}\" expected_fan_in={}{}",
        status_display_role(assigned_role),
        summarize_text_for_visibility(scope, 96),
        expected_fan_in_fields.join(","),
        expertise_suffix
    ))
}

#[cfg(test)]
fn build_runtime_dispatch_line(payload: &Value) -> Option<String> {
    let current_task = payload.get("current_task_card")?;
    if !current_task.is_object() {
        return None;
    }

    let runtime_dispatch = current_task
        .get("runtime_dispatch")
        .filter(|value| value.is_object())
        .or_else(|| {
            current_task
                .pointer("/delegation_plan/runtime_dispatch")
                .filter(|value| value.is_object())
        })?;

    let source = runtime_dispatch
        .get("source")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("config_backed");
    let execution_mode_source = runtime_dispatch
        .get("execution_mode_source")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("runtime_config");
    let role_profile_source = runtime_dispatch
        .get("role_profile_source")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("role_config_snapshot");
    let custom_agent_source = runtime_dispatch
        .get("custom_agent_source")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("role_mapping");
    let preferred_execution_mode = runtime_dispatch
        .get("preferred_execution_mode")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let fallback_execution_mode = runtime_dispatch
        .get("fallback_execution_mode")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let preferred_custom_agent_name = runtime_dispatch
        .get("preferred_custom_agent_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unassigned");
    let model = runtime_dispatch
        .get("model")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unspecified");
    let variant = runtime_dispatch
        .get("variant")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unspecified");

    Some(format!(
        "Dispatch: source={source} preferred={preferred_execution_mode}({execution_mode_source}) fallback={fallback_execution_mode}({execution_mode_source}) agent={}({custom_agent_source}) model={model}({role_profile_source}) variant={variant}({role_profile_source})",
        status_display_agent(preferred_custom_agent_name)
    ))
}

#[cfg(test)]
fn build_transport_guidance_line(payload: &Value) -> Option<String> {
    let guidance = payload
        .pointer("/execution_strategy/operator_visible_transport")
        .filter(|value| value.is_object())?;
    let preferred = guidance
        .get("preferred_transport")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let transcript_signal = guidance
        .get("transcript_signal")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ran");
    let lifecycle_mutations = guidance
        .get("lifecycle_mutations")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mcp_reserved_for = guidance
        .get("mcp_reserved_for")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if lifecycle_mutations.is_empty() || mcp_reserved_for.is_empty() {
        return None;
    }

    Some(format!(
        "Transport: prefer {preferred} ({transcript_signal}) for {} via --quiet --json-file or --quiet --json; reserve MCP for {}",
        lifecycle_mutations.join(","),
        mcp_reserved_for.join(",")
    ))
}

#[cfg(test)]
fn build_cost_routing_line(payload: &Value) -> Option<String> {
    let cost_routing = payload.get("cost_routing")?;
    let status = cost_routing
        .get("status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let subagents_enabled = cost_routing
        .pointer("/subagents/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let simple_low_cost = cost_routing
        .get("simple_routes_use_low_cost_models")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let current_role = cost_routing
        .pointer("/subagents/current_task_role")
        .and_then(Value::as_str)
        .unwrap_or("unassigned");
    let current_model = cost_routing
        .pointer("/subagents/current_task_model/model")
        .and_then(Value::as_str)
        .unwrap_or("unassigned");
    let token_status = cost_routing
        .pointer("/token_usage_observation/status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    Some(format!(
        "Cost Routing: status={status} subagents_enabled={subagents_enabled} simple_low_cost_models={simple_low_cost} current_role={} current_model={current_model} token_usage={token_status}",
        status_display_role(current_role)
    ))
}

#[cfg(test)]
fn build_assignment_quality_line(payload: &Value) -> Option<String> {
    let assignment_quality = payload.pointer("/current_task_card/assignment_quality")?;
    let state = assignment_quality
        .get("state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    if state != "mismatch" {
        return None;
    }
    let expected_family = assignment_quality
        .get("expected_family")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let assigned_role = assignment_quality
        .get("assigned_role")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unassigned");
    let assigned_agent_id = assignment_quality
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unassigned");
    let reason = assignment_quality
        .get("reason")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(" reason=\"{}\"", summarize_text_for_visibility(value, 120)))
        .unwrap_or_default();

    Some(format!(
        "Assignment Warning: routing-drift assigned={}/{} expected={expected_family}{reason}",
        status_display_role(assigned_role),
        status_display_agent(assigned_agent_id)
    ))
}

fn build_routing_trace_line(payload: &Value) -> Option<String> {
    let trace = payload
        .pointer("/current_task_card/routing_trace")
        .filter(|value| value.is_object())?;
    let category =
        compact_text_field(trace, "selected_category").unwrap_or_else(|| "unknown".to_string());
    let skill = compact_text_field(trace, "selected_skill_id")
        .or_else(|| compact_text_field(trace, "selected_skill_name"))
        .unwrap_or_else(|| "unknown".to_string());
    let role = compact_text_field(trace, "selected_role").unwrap_or_else(|| "unknown".to_string());
    let agent =
        compact_text_field(trace, "selected_agent_id").unwrap_or_else(|| "unknown".to_string());
    let risk = compact_text_field(trace, "risk");
    let reason = compact_text_field(trace, "reason")
        .or_else(|| compact_text_field(trace, "summary"))
        .map(|value| format!(" reason=\"{}\"", summarize_text_for_visibility(&value, 140)))
        .unwrap_or_default();
    let risk_suffix = risk
        .map(|value| format!(" risk={value}"))
        .unwrap_or_default();

    Some(format!(
        "Routing: category={category} skill={} role={} agent={}{}{}",
        status_display_agent(&skill),
        status_display_role(&role),
        status_display_agent(&agent),
        risk_suffix,
        reason
    ))
}

#[cfg(test)]
fn build_spec_split_line(payload: &Value) -> Option<String> {
    let current_task = payload.get("current_task_card")?;
    if !current_task.is_object() {
        return None;
    }

    let spec_surfaces = current_task
        .pointer("/spec_surfaces")
        .or_else(|| current_task.pointer("/delegation_plan/spec_surfaces"))?;
    if !spec_surfaces.is_object() {
        return None;
    }

    let surface_summary = |surface_name: &str| {
        let surface = spec_surfaces.pointer(&format!("/{surface_name}"))?;
        let source = surface
            .get("owned_by")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unspecified");
        let field_count = surface
            .pointer("/fields")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .count()
            })
            .unwrap_or(0);
        Some(format!("{surface_name}={source}[{field_count}]"))
    };

    let mut parts = Vec::new();
    for surface_name in [
        "role_owned",
        "sandbox_owned",
        "workflow_owned",
        "plan_invariants",
    ] {
        if let Some(summary) = surface_summary(surface_name) {
            parts.push(summary);
        }
    }
    if parts.is_empty() {
        return None;
    }

    Some(format!("Spec Split: {}", parts.join(" ")))
}

#[cfg(test)]
fn build_lane_artifact_line(payload: &Value) -> Option<String> {
    let current_task = payload.get("current_task_card")?;
    if !current_task.is_object() {
        return None;
    }

    let lane_artifact_contract = current_task
        .pointer("/lane_artifact_contract")
        .or_else(|| current_task.pointer("/delegation_plan/lane_artifact_contract"))?;
    if !lane_artifact_contract.is_object() {
        return None;
    }

    let result_field = lane_artifact_contract
        .pointer("/result/field")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let log_field = lane_artifact_contract
        .pointer("/log/field")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let recap_field = lane_artifact_contract
        .pointer("/recap/field")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;

    Some(format!(
        "Lane Artifacts: result={result_field} log={log_field} recap={recap_field}"
    ))
}

#[cfg(test)]
fn build_verify_retry_recap_report_line(payload: &Value) -> Option<String> {
    let current_task = payload.get("current_task_card")?;
    if !current_task.is_object() {
        return None;
    }

    let contract = current_task
        .pointer("/delegation_plan/verify_retry_recap_report_contract")
        .filter(|value| value.is_object())?;
    let verify_state = current_task
        .get("verification_state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("pending");
    let retry_budget_key = contract
        .pointer("/retry/budget_key")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("retry");
    let retry_state = current_task
        .pointer("/captain_intervention/pending_follow_up/status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            payload
                .pointer("/pending_captain_follow_up/status")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        });
    let recap_field = contract
        .pointer("/recap/field")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("lane_artifact_contract.recap");
    let report_field = contract
        .pointer("/report/field")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("latest_delegate_result.result_summary");
    let report_fallback_field = contract
        .pointer("/report/fallback_field")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());

    let retry_label = match retry_state {
        Some(state) => format!("{retry_budget_key}:{state}"),
        None => retry_budget_key.to_string(),
    };
    let report_label = report_fallback_field
        .map(|fallback| format!("{report_field}|{fallback}"))
        .unwrap_or_else(|| report_field.to_string());

    Some(format!(
        "Verify/Retry/Recap/Report: verify={verify_state} retry={retry_label} recap={recap_field} report={report_label}"
    ))
}

#[cfg(test)]
fn build_review_state_line(payload: &Value) -> Option<String> {
    let current_task = payload.get("current_task_card")?;
    if !current_task.is_object() {
        return None;
    }

    let review_policy = current_task
        .get("review_policy")
        .filter(|value| value.is_object())
        .or_else(|| {
            payload
                .get("review_policy")
                .filter(|value| value.is_object())
        });
    let review_of = current_task
        .get("review_of_task_card_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let gate = current_task
        .get("orchestrator_review_gate")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let verification = current_task
        .get("verification_state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let pass_count = current_task
        .get("review_pass_count")
        .and_then(Value::as_u64);

    if review_policy.is_none()
        && review_of.is_empty()
        && gate.is_none()
        && verification.is_none()
        && pass_count.is_none()
        && !current_task
            .get("review_fan_in")
            .map(Value::is_object)
            .unwrap_or(false)
    {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(policy) = review_policy {
        let policy_state = policy
            .get("state")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown")
            .replace('_', "-");
        parts.push(format!("state={policy_state}"));
        if let Some(decision) = policy
            .get("decision")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(format!("decision={}", decision.replace('_', "-")));
        }
        if let Some(risk) = policy
            .get("risk")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(format!("risk={risk}"));
        }
        let active_reviewers = policy.get("active_reviewers").and_then(Value::as_u64);
        let reviewer_cap = policy.get("reviewer_cap").and_then(Value::as_u64);
        if active_reviewers.is_some() || reviewer_cap.is_some() {
            parts.push(format!(
                "reviewers={}/{}",
                active_reviewers.unwrap_or(0),
                reviewer_cap.unwrap_or(0)
            ));
        }
    }
    if !review_of.is_empty() {
        parts.push(format!("of={}", review_of.join(",")));
    }
    if let Some(gate) = gate {
        parts.push(format!("gate={gate}"));
    }
    if let Some(verification) = verification {
        parts.push(format!("verification={verification}"));
    }
    if let Some(pass_count) = pass_count {
        parts.push(format!("passes={pass_count}"));
    }
    if let Some(review_fan_in) = current_task
        .get("review_fan_in")
        .filter(|value| value.is_object())
    {
        if let Some(outcome) = review_fan_in
            .get("outcome")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(format!("outcome={}", outcome.replace('_', "-")));
        }
        if let Some(unresolved_count) = review_fan_in
            .get("unresolved_finding_count")
            .and_then(Value::as_u64)
        {
            parts.push(format!("unresolved={unresolved_count}"));
        }
        if let Some(next_action) = review_fan_in
            .get("captain_next_decision")
            .or_else(|| review_fan_in.get("next_action"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(format!("next={}", next_action.replace('_', "-")));
        }
    }

    Some(format!("Review: {}", parts.join(" ")))
}

#[cfg(test)]
fn build_completion_discipline_line(payload: &Value) -> Option<String> {
    let discipline = payload
        .get("completion_discipline")
        .filter(|value| value.is_object())
        .or_else(|| {
            payload
                .pointer("/current_task_card/completion_discipline")
                .filter(|value| value.is_object())
        })?;
    let documented_completion = discipline
        .get("documented_completion_requested")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !documented_completion {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(state) = discipline
        .get("state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("state={}", state.replace('_', "-")));
    }
    if let Some(mode) = discipline
        .get("completion_mode")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("mode={}", mode.replace('_', "-")));
    }
    if let Some(summary) = discipline
        .get("summary")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "summary=\"{}\"",
            summarize_text_for_visibility(summary, 140)
        ));
    }

    (!parts.is_empty()).then(|| format!("Completion: {}", parts.join(" ")))
}

#[cfg(test)]
fn build_captain_action_contract_line(payload: &Value) -> Option<String> {
    let contract = payload
        .get("captain_action_contract")
        .filter(|value| value.is_object())?;
    let allowed_action = contract
        .get("allowed_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let required_action = contract
        .get("required_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ccc_orchestrate");
    let preflight_guard = contract
        .get("preflight_guard")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ccc_recommend_entry");
    let direct_finish_allowed = contract
        .get("direct_finish_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_mutation_allowed = contract
        .get("direct_mutation_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_file_mutation_policy = contract.get("direct_file_mutation_policy");
    let direct_file_mutation_allowed = direct_file_mutation_policy
        .and_then(|policy| policy.get("allowed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_file_mutation_route = direct_file_mutation_policy
        .and_then(|policy| policy.get("required_route"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("specialist_fan_in_then_captain_review_merge");
    let required_display = operator_command_label(required_action);
    let preflight_display = operator_command_label(preflight_guard);
    let mut parts = vec![
        format!("allowed={}", allowed_action.replace('_', "-")),
        format!("required={required_display}"),
        format!("preflight={preflight_display}"),
        format!("direct_finish={direct_finish_allowed}"),
        format!("direct_mutation={direct_mutation_allowed}"),
        format!(
            "direct_file_mutation_allowed={direct_file_mutation_allowed} route={}",
            direct_file_mutation_route.replace('_', "-")
        ),
    ];
    if let Some(reason) = contract
        .get("denied_action_reason")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "reason=\"{}\"",
            summarize_text_for_visibility(reason, 120)
        ));
    }

    Some(format!("Captain Guard: {}", parts.join(" ")))
}

fn build_captain_direct_mutation_guard_line(payload: &Value) -> Option<String> {
    let guard = payload
        .get("captain_direct_mutation_guard")
        .filter(|value| value.is_object())?;
    let state = guard
        .get("state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let changed_path_count = guard
        .get("changed_path_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut parts = vec![
        format!("state={}", state.replace('_', "-")),
        format!("changed_paths={changed_path_count}"),
    ];
    if let Some(paths) = guard.get("changed_paths").and_then(Value::as_array) {
        let visible_paths = paths
            .iter()
            .filter_map(Value::as_str)
            .take(3)
            .collect::<Vec<_>>();
        if !visible_paths.is_empty() {
            parts.push(format!("paths={}", visible_paths.join(",")));
        }
    }
    if let Some(required_action) = guard
        .get("required_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("required={}", required_action.replace('_', "-")));
    }
    Some(format!(
        "Captain Direct Mutation Guard: {}",
        parts.join(" ")
    ))
}

fn build_state_contract_line(payload: &Value) -> Option<String> {
    let contract = payload
        .get("state_contract")
        .filter(|value| value.is_object())?;
    let state = contract
        .get("state")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let active_gate = contract
        .get("active_gate")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let required_artifact = contract
        .get("required_artifact")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unspecified");
    let next_step = contract
        .get("next_step")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let mut parts = vec![
        format!("state={}", state.replace('_', "-")),
        format!("gate={}", active_gate.replace('_', "-")),
        format!("requires={}", required_artifact.replace('_', "-")),
        format!("next={}", next_step.replace('_', "-")),
    ];
    if let Some(precedence) = payload
        .pointer("/post_fan_in_captain_decision/precedence")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("decision={}", precedence.replace('_', "-")));
    }
    if let Some(allowed) = contract
        .get("allowed_next_transitions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(4)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
    {
        parts.push(format!("allowed={}", allowed.join(",")));
    }
    if let Some(captain_allowed) = contract
        .get("captain_allowed_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "captain_allowed={}",
            captain_allowed.replace('_', "-")
        ));
    }
    if let Some(captain_required) = contract
        .get("captain_required_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "captain_required={}",
            captain_required.replace('_', "-")
        ));
    }
    Some(format!("Active Gate: {}", parts.join(" ")))
}

fn build_active_checkpoint_line(payload: &Value) -> Option<String> {
    let checkpoint = payload
        .get("active_checkpoint")
        .filter(|value| value.is_object())?;
    let gate = compact_text_pointer(checkpoint, "/current_gate").unwrap_or("unknown".to_string());
    let task = compact_text_pointer(checkpoint, "/task_card_id").unwrap_or("unknown".to_string());
    let role =
        compact_text_pointer(checkpoint, "/assigned_role").unwrap_or("unassigned".to_string());
    let agent =
        compact_text_pointer(checkpoint, "/assigned_agent_id").unwrap_or("unassigned".to_string());
    let delegated = compact_text_pointer(checkpoint, "/delegated_work/summary")
        .unwrap_or_else(|| "workers=0/0 host_subagents=0/0".to_string());
    let fan_in_ready = checkpoint
        .pointer("/fan_in_state/ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let resume =
        compact_text_pointer(checkpoint, "/resume_action").unwrap_or("unknown".to_string());
    let command = compact_text_pointer(checkpoint, "/continuation_command")
        .unwrap_or_else(|| "$cap continue <run_id>".to_string());
    let next = compact_text_pointer(checkpoint, "/next_legal_action")
        .unwrap_or_else(|| "unknown".to_string());
    let late_state =
        compact_text_pointer(checkpoint, "/late_output/state").unwrap_or("none".to_string());
    let late_count = checkpoint
        .pointer("/late_output/count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let late_authority = compact_text_pointer(checkpoint, "/late_output/authority")
        .map(|value| format!(" authority={}", value.replace('_', "-")))
        .unwrap_or_default();

    // Keep the resume capsule to a single operator line; structured payloads
    // carry the larger fan-in and lane details.
    Some(format!(
        "Checkpoint: gate={} task={} role={} agent={} {} fan_in={} next={} resume={} continue=\"{}\" late={}({}){}",
        gate.replace('_', "-"),
        task,
        status_display_role(&role),
        status_display_agent(&agent),
        delegated,
        fan_in_ready,
        next.replace('_', "-"),
        resume.replace('_', "-"),
        command,
        late_state.replace('_', "-"),
        late_count,
        late_authority
    ))
}

fn build_task_session_state_line(payload: &Value) -> Option<String> {
    let state = payload
        .get("task_session_state")
        .filter(|value| value.is_object())?;
    let task =
        compact_text_pointer(state, "/active_task/task_card_id").unwrap_or("unknown".to_string());
    let gate = compact_text_pointer(state, "/current_gate/active_gate")
        .unwrap_or_else(|| "unknown".to_string());
    let agent = compact_text_pointer(state, "/delegated_agent/child_agent_id")
        .or_else(|| compact_text_pointer(state, "/active_task/assigned_agent_id"))
        .unwrap_or_else(|| "unassigned".to_string());
    let agent_status = compact_text_pointer(state, "/delegated_agent/status")
        .unwrap_or_else(|| "unknown".to_string());
    let model = compact_text_pointer(state, "/delegated_agent/model")
        .unwrap_or_else(|| "unknown".to_string());
    let variant = compact_text_pointer(state, "/delegated_agent/variant")
        .unwrap_or_else(|| "unknown".to_string());
    let fallback =
        compact_text_pointer(state, "/fallback_state/status").unwrap_or_else(|| "none".to_string());
    let evidence_count = state
        .pointer("/evidence/count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let verification =
        compact_text_pointer(state, "/verification/state").unwrap_or_else(|| "unknown".to_string());
    let unresolved = state
        .pointer("/verification/unresolved_risk_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let session = compact_text_pointer(state, "/internal_session/session_id")
        .unwrap_or_else(|| "unknown".to_string());

    Some(format!(
        "Task Session: task={} gate={} agent={} status={} model={}/{} fallback={} evidence={} verification={} unresolved_risk={} session={}",
        task,
        gate.replace('_', "-"),
        status_display_agent(&agent),
        agent_status.replace('_', "-"),
        model,
        variant,
        fallback.replace('_', "-"),
        evidence_count,
        verification.replace('_', "-"),
        unresolved,
        session
    ))
}

fn build_verification_capsule_line(payload: &Value) -> Option<String> {
    let capsule = payload
        .pointer("/current_task_card/verification_capsule")
        .or_else(|| payload.pointer("/task_session_state/verification_capsule"))
        .filter(|value| value.is_object())?;
    let evidence = capsule
        .pointer("/evidence/count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let validation = capsule
        .pointer("/validation/count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let verdict = compact_text_pointer(capsule, "/reviewer_verdict")
        .unwrap_or_else(|| "not-applicable".to_string());
    let unresolved = capsule
        .pointer("/unresolved_risk/count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let acceptance = compact_text_pointer(capsule, "/acceptance")
        .map(|value| {
            format!(
                " acceptance=\"{}\"",
                summarize_text_for_visibility(&value, 120)
            )
        })
        .unwrap_or_default();

    Some(format!(
        "Verification Capsule: evidence={} validation={} reviewer_verdict={} unresolved_risk={}{}",
        evidence,
        validation,
        verdict.replace('_', "-"),
        unresolved,
        acceptance
    ))
}

fn build_delegated_ownership_line(payload: &Value) -> Option<String> {
    let ownership = payload
        .pointer("/current_task_card/delegated_ownership")
        .or_else(|| payload.pointer("/task_session_state/delegated_ownership"))
        .or_else(|| payload.pointer("/active_checkpoint/delegated_work/ownership"))
        .filter(|value| value.is_object())?;
    let agent = compact_text_pointer(ownership, "/owner/assigned_agent_id")
        .or_else(|| compact_text_pointer(ownership, "/owner/child_agent_id"))
        .unwrap_or_else(|| "unassigned".to_string());
    let path_count = ownership
        .pointer("/search_ownership/paths")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let term_count = ownership
        .pointer("/search_ownership/terms")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let mutation_active = ownership
        .pointer("/mutation_ownership/active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reclaim_recorded = ownership
        .pointer("/repeat_guard/reclaim_recorded")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stale_recorded = ownership
        .pointer("/repeat_guard/stale_output_recorded")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Some(format!(
        "Delegated Ownership: agent={} search_paths={} search_terms={} mutation={} repeat_guard=reclaim:{} stale:{} explicit-reason-required",
        status_display_agent(&agent),
        path_count,
        term_count,
        mutation_active,
        reclaim_recorded,
        stale_recorded
    ))
}

fn build_recovery_lane_line(payload: &Value) -> Option<String> {
    let recovery_lane = payload
        .get("recovery_lane")
        .filter(|value| value.is_object())?;
    let status = recovery_lane
        .get("status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let recommended_action = recovery_lane
        .get("recommended_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("none");
    let reclaim_action = recovery_lane
        .get("reclaim_replan_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("none");
    let needs_attention = recovery_lane
        .get("needs_operator_attention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let target_count = recovery_lane
        .get("target_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut parts = vec![
        format!("status={}", status.replace('_', "-")),
        format!("action={}", recommended_action.replace('_', "-")),
        format!("reclaim={}", reclaim_action.replace('_', "-")),
        format!("attention={needs_attention}"),
        format!("targets={target_count}"),
    ];
    if let Some(summary) = recovery_lane
        .get("summary")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "summary=\"{}\"",
            summarize_text_for_visibility(summary, 140)
        ));
    }

    Some(format!("Recovery: {}", parts.join(" ")))
}

fn build_lifecycle_hooks_line(payload: &Value) -> Option<String> {
    let hooks = payload
        .get("lifecycle_hooks")
        .filter(|value| value.is_object())?;
    let status = hooks
        .get("status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let active = hooks
        .get("active_tiers")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let failures = hooks
        .get("failure_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let skipped = hooks
        .get("skipped_tiers")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    Some(format!(
        "Lifecycle Hooks: status={} active={} skipped={} failures={} internal=true",
        status.replace('_', "-"),
        if active.is_empty() {
            "none".to_string()
        } else {
            active.join(",")
        },
        skipped,
        failures
    ))
}

fn build_workflow_loop_line(payload: &Value) -> Option<String> {
    let workflow = payload
        .get("workflow_loop")
        .filter(|value| value.is_object())?;
    let current_stage = compact_text_pointer(workflow, "/current_stage")?;
    let status = compact_text_pointer(workflow, "/status").unwrap_or_else(|| "active".to_string());
    let stages = workflow
        .get("stages")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let label = compact_text_pointer(item, "/label")?;
                    let state = compact_text_pointer(item, "/status")
                        .unwrap_or_else(|| "unknown".to_string());
                    Some(format!(
                        "{}:{}",
                        label.replace(' ', "-"),
                        state.replace('_', "-")
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(format!(
        "Workflow Loop: status={} current={} stages={}",
        status.replace('_', "-"),
        current_stage.replace('_', "-"),
        if stages.is_empty() {
            "none".to_string()
        } else {
            stages.join(">")
        }
    ))
}

#[cfg(test)]
fn operator_command_label(action: &str) -> String {
    match action {
        "ccc_recommend_entry" => "internal preflight".to_string(),
        "ccc_orchestrate" => "ccc orchestrate".to_string(),
        "ccc_subagent_update" => "ccc subagent-update".to_string(),
        "ccc_status" => "ccc status".to_string(),
        "ccc_start" => "ccc start".to_string(),
        "ccc_run" => "ccc run".to_string(),
        "spawn_or_merge_review" => "spawn or merge review".to_string(),
        "spawn_or_record_specialist" => "spawn or record specialist".to_string(),
        other => other.replace('_', "-"),
    }
}

fn build_longway_planned_rows(payload: &Value) -> Vec<String> {
    payload
        .pointer("/longway/planned_rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(render_longway_planned_row)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn render_longway_planned_row(row: &Value) -> Option<String> {
    let title = row
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let title = summarize_text_for_visibility(title, 96);
    let planned_agent = planned_row_display_value(row, "planned_agent_id", "display_agent_id");
    let planned_role = planned_row_display_value(row, "planned_role", "display_role");
    let status = match row.get("status").and_then(Value::as_str) {
        Some("materialized") => "x",
        Some("blocked") => "!",
        Some("cancelled" | "skipped") => "-",
        Some("ready") => ">",
        _ => " ",
    };
    let task_card_suffix = row
        .get("task_card_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(" task_card={value}"))
        .unwrap_or_default();
    let recovery_suffix = planned_row_recovery_suffix(row);

    Some(format!(
        "[{status}] Planned: {title} [{}] role={}{}{}",
        status_display_agent(planned_agent),
        status_display_role(planned_role),
        task_card_suffix,
        recovery_suffix
    ))
}

fn render_nested_planned_row_metadata(row: &Value) -> Option<String> {
    let title = row
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let title = summarize_text_for_visibility(title, 96);
    let planned_agent = planned_row_display_value(row, "planned_agent_id", "display_agent_id");
    let planned_role = planned_row_display_value(row, "planned_role", "display_role");
    let original_status = row
        .get("status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("planned");
    let recovery_suffix = planned_row_recovery_suffix(row);

    Some(format!(
        "plan: {title} [{}] role={} original_status={original_status}{recovery_suffix}",
        status_display_agent(planned_agent),
        status_display_role(planned_role)
    ))
}

fn planned_row_display_value<'a>(row: &'a Value, primary_key: &str, display_key: &str) -> &'a str {
    let primary = row
        .get(primary_key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match primary {
        Some("unassigned") | None => row
            .get(display_key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "unassigned")
            .unwrap_or("unassigned"),
        Some(value) => value,
    }
}

fn planned_row_recovery_suffix(row: &Value) -> String {
    let Some(recovery) = row.get("recovery").filter(|value| value.is_object()) else {
        return String::new();
    };
    let mode = recovery
        .get("mode")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("fallback");
    let reason = recovery
        .get("reason")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(" reason={}", value.replace('_', "-")))
        .unwrap_or_default();
    let primary_status = recovery
        .get("primary_status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(" primary={}", value.replace('_', "-")))
        .unwrap_or_default();
    format!(" recovered={mode}{reason}{primary_status}")
}

fn row_planned_row_lines(row: &Value) -> Vec<String> {
    row.get("planned_rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(render_nested_planned_row_metadata)
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn row_lifecycle_suffix(row: &Value) -> String {
    row.pointer("/lifecycle_sync/status")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty() && *value != "not_started")
        .map(|value| format!(" lifecycle={value}"))
        .unwrap_or_default()
}

fn row_task_unit_suffix(row: &Value) -> String {
    let labels = compact_row_task_unit_labels(row, 4, 64);

    if labels.is_empty() {
        String::new()
    } else {
        format!(" units={}", labels.join(","))
    }
}

fn compact_row_task_unit_labels(row: &Value, limit: usize, max_chars: usize) -> Vec<String> {
    row.get("task_unit_labels")
        .and_then(Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .take(limit)
                .map(|value| summarize_text_for_visibility(value, max_chars))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn render_longway_phase_row(
    status: &str,
    work_item: &str,
    owner_agent: Option<&str>,
    row: &Value,
) -> String {
    if let Some(owner_agent) = owner_agent {
        format!(
            "[{status}] {work_item} [{}]{}{}",
            status_display_agent(owner_agent),
            row_task_unit_suffix(row),
            row_lifecycle_suffix(row)
        )
    } else {
        format!(
            "[{status}] {work_item}{}{}",
            row_task_unit_suffix(row),
            row_lifecycle_suffix(row)
        )
    }
}

fn render_longway_phase_row_block(
    status: &str,
    work_item: &str,
    owner_agent: Option<&str>,
    row: &Value,
) -> String {
    let mut lines = vec![render_longway_phase_row(
        status,
        work_item,
        owner_agent,
        row,
    )];
    lines.extend(row_lifecycle_detail_lines(row));
    lines.extend(row_planned_row_lines(row));
    lines.join("\n")
}

fn row_lifecycle_detail_lines(row: &Value) -> Vec<String> {
    row.pointer("/lifecycle_sync/details")
        .and_then(Value::as_array)
        .map(|details| {
            details
                .iter()
                .filter_map(render_lifecycle_detail_line)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn render_lifecycle_detail_line(detail: &Value) -> Option<String> {
    let label = detail
        .get("label")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let status = detail
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let mut line = format!("- {label} {}", status.replace('_', "-"));
    if let Some(child_agent_id) = detail
        .get("child_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != label)
    {
        line.push_str(&format!(" {}", status_display_agent(child_agent_id)));
    }
    if let Some(summary) = detail
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        line.push_str(&format!(" {}", summarize_text_for_visibility(summary, 96)));
    }
    if let Some(evidence_count) = detail
        .get("evidence_count")
        .and_then(Value::as_u64)
        .filter(|count| *count > 0)
    {
        line.push_str(&format!(" evidence={evidence_count}"));
    }
    if let Some(confidence) = detail
        .get("confidence")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        line.push_str(&format!(" confidence={confidence}"));
    }
    Some(line)
}

#[derive(Clone, Copy)]
struct SubagentRowLabels {
    heading: &'static str,
    child: &'static str,
    role: &'static str,
    task: &'static str,
    task_card: &'static str,
    no_activity: &'static str,
}

const OPERATOR_PROJECTION_MAX_ROWS: usize = 12;

const ENGLISH_SUBAGENT_LABELS: SubagentRowLabels = SubagentRowLabels {
    heading: "Subagents",
    child: "child",
    role: "role",
    task: "task",
    task_card: "task_card",
    no_activity: "[ ] no subagent lane activity",
};

const KOREAN_SUBAGENT_LABELS: SubagentRowLabels = SubagentRowLabels {
    heading: "서브에이전트",
    child: "자식",
    role: "역할",
    task: "작업",
    task_card: "작업카드",
    no_activity: "[ ] 서브에이전트 lane 활동 없음",
};

pub(crate) fn create_subagents_text(payload: &Value) -> String {
    create_subagents_text_with_labels(payload, ENGLISH_SUBAGENT_LABELS)
}

pub(crate) fn create_operator_longway_projection_text(payload: &Value) -> String {
    let korean = payload_appears_korean(payload);
    let labels = if korean {
        KOREAN_SUBAGENT_LABELS
    } else {
        ENGLISH_SUBAGENT_LABELS
    };
    let title = if korean {
        "LongWay 투영"
    } else {
        "LongWay Projection"
    };
    let run_label = if korean { "실행" } else { "Run" };
    let next_label = if korean { "다음" } else { "Next" };
    let mut lines = vec![title.to_string()];
    if let Some(run_id) = compact_text_field(payload, "run_id") {
        lines.push(format!("{run_label}: {run_id}"));
    }
    if let Some(next_step) = compact_text_field(payload, "next_step")
        .or_else(|| compact_text_pointer(payload, "/scheduler/next_step"))
        .or_else(|| compact_text_pointer(payload, "/run_state/next_action/command"))
    {
        lines.push(format!("{next_label}: {next_step}"));
    }
    if let Some(state_contract_line) = build_state_contract_line(payload) {
        lines.push(state_contract_line);
    }
    if let Some(active_checkpoint_line) = build_active_checkpoint_line(payload) {
        lines.push(active_checkpoint_line);
    }
    if let Some(task_session_state_line) = build_task_session_state_line(payload) {
        lines.push(task_session_state_line);
    }
    if let Some(verification_capsule_line) = build_verification_capsule_line(payload) {
        lines.push(verification_capsule_line);
    }
    if let Some(delegated_ownership_line) = build_delegated_ownership_line(payload) {
        lines.push(delegated_ownership_line);
    }
    if let Some(recovery_lane_line) = build_recovery_lane_line(payload) {
        lines.push(recovery_lane_line);
    }
    if let Some(workflow_loop_line) = build_workflow_loop_line(payload) {
        lines.push(workflow_loop_line);
    }
    if let Some(lifecycle_hooks_line) = build_lifecycle_hooks_line(payload) {
        lines.push(lifecycle_hooks_line);
    }
    if let Some(routing_trace_line) = build_routing_trace_line(payload) {
        lines.push(routing_trace_line);
    }
    if let Some(captain_direct_mutation_guard_line) =
        build_captain_direct_mutation_guard_line(payload)
    {
        lines.push(captain_direct_mutation_guard_line);
    }
    lines.push(String::new());
    if let Some(checklist) = build_longway_checklist_block(payload) {
        lines.push(checklist);
        lines.push(String::new());
    }
    if payload.get("approval_state").and_then(Value::as_str) == Some("pending_longway_approval")
        || payload
            .pointer("/run_state/next_action/command")
            .and_then(Value::as_str)
            == Some("await_longway_approval")
    {
        if korean {
            lines.push("승인 대기".to_string());
            lines.push("[ ] 이 LongWay 계획으로 작업을 진행할지 확인 필요".to_string());
            lines.push(
                "승인: ccc orchestrate --quiet --json '{\"approve_longway\":true,...}'".to_string(),
            );
        } else {
            lines.push("Approval".to_string());
            lines.push("[ ] Confirm whether to execute this LongWay plan".to_string());
            lines.push(
                "Approve: ccc orchestrate --quiet --json '{\"approve_longway\":true,...}'"
                    .to_string(),
            );
        }
        lines.push(String::new());
    }
    lines.push(create_subagents_text_with_labels_limit(
        payload,
        labels,
        Some(OPERATOR_PROJECTION_MAX_ROWS),
    ));
    lines.join("\n")
}

fn create_subagents_text_with_labels(payload: &Value, labels: SubagentRowLabels) -> String {
    create_subagents_text_with_labels_limit(payload, labels, None)
}

fn create_subagents_text_with_labels_limit(
    payload: &Value,
    labels: SubagentRowLabels,
    max_rows: Option<usize>,
) -> String {
    let mut rows = parallel_fanout_lane_rows(payload, labels);
    if rows.is_empty() {
        rows = host_subagent_activity_rows(payload, labels);
    }

    let mut lines = vec![labels.heading.to_string()];
    if rows.is_empty() {
        lines.push(labels.no_activity.to_string());
    } else {
        if let Some(max_rows) = max_rows.filter(|max_rows| *max_rows > 0) {
            let omitted = rows.len().saturating_sub(max_rows);
            rows.truncate(max_rows);
            if omitted > 0 {
                let omitted_label = if payload_appears_korean(payload) {
                    format!("[ ] ... {omitted}개 항목 생략")
                } else {
                    format!("[ ] ... {omitted} more rows omitted")
                };
                rows.push(omitted_label);
            }
        }
        lines.extend(rows);
    }
    lines.join("\n")
}

fn parallel_fanout_lane_rows(payload: &Value, labels: SubagentRowLabels) -> Vec<String> {
    let task = payload.get("current_task_card").unwrap_or(&Value::Null);
    let lanes = task
        .pointer("/parallel_fanout/lanes")
        .or_else(|| payload.pointer("/parallel_fanout/lanes"))
        .and_then(Value::as_array);
    let Some(lanes) = lanes else {
        return Vec::new();
    };

    lanes
        .iter()
        .filter_map(|lane| render_parallel_lane_row(payload, task, lane, labels))
        .collect()
}

fn render_parallel_lane_row(
    payload: &Value,
    task: &Value,
    lane: &Value,
    labels: SubagentRowLabels,
) -> Option<String> {
    let lane_id = compact_text_field(lane, "lane_id")?;
    let status = compact_text_pointer(lane, "/fan_in/status")
        .or_else(|| compact_text_pointer(lane, "/lifecycle/status"))
        .unwrap_or_else(|| "not_started".to_string());
    let child_agent = compact_text_pointer(lane, "/fan_in/child_agent_id")
        .or_else(|| compact_text_pointer(lane, "/lifecycle/child_agent_id"))
        .unwrap_or_else(|| "unassigned".to_string());
    let role = compact_text_field(lane, "assigned_role")
        .or_else(|| compact_text_pointer(lane, "/fan_in/assigned_role"))
        .or_else(|| compact_text_pointer(lane, "/lifecycle/assigned_role"))
        .or_else(|| compact_text_field(task, "assigned_role"))
        .unwrap_or_else(|| "unassigned".to_string());
    let display_role = status_display_role(&role);
    let task_label = compact_task_label(task, labels)
        .or_else(|| {
            compact_text_field(lane, "task_card_id")
                .map(|value| format!("{}={value}", labels.task_card))
        })
        .or_else(|| {
            compact_text_field(payload, "task_card_id")
                .map(|value| format!("{}={value}", labels.task_card))
        })
        .unwrap_or_else(|| format!("{}=task", labels.task));
    Some(format!(
        "[{}] {lane_id} {} {}={} {}={display_role} {task_label}",
        subagent_status_marker(&status),
        status.replace('_', "-"),
        labels.child,
        status_display_agent(&child_agent),
        labels.role
    ))
}

fn host_subagent_activity_rows(payload: &Value, labels: SubagentRowLabels) -> Vec<String> {
    payload
        .pointer("/host_subagent_state/subagent_activity")
        .or_else(|| payload.pointer("/host_subagent_state/active_subagents"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| render_host_subagent_activity_row(item, labels))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn render_host_subagent_activity_row(item: &Value, labels: SubagentRowLabels) -> Option<String> {
    let child_agent = compact_text_field(item, "child_agent_id")?;
    let lane_id = compact_text_field(item, "lane_id").unwrap_or_else(|| "subagent".to_string());
    let status = compact_text_field(item, "status").unwrap_or_else(|| "unknown".to_string());
    let role =
        compact_text_field(item, "assigned_role").unwrap_or_else(|| "unassigned".to_string());
    let display_role = status_display_role(&role);
    let task_label = compact_text_field(item, "task_title")
        .map(|value| {
            format!(
                "{}=\"{}\"",
                labels.task,
                summarize_text_for_visibility(&value, 72)
            )
        })
        .or_else(|| {
            compact_text_field(item, "task_card_id")
                .map(|value| format!("{}={value}", labels.task_card))
        })
        .unwrap_or_else(|| format!("{}=task", labels.task));
    Some(format!(
        "[{}] {lane_id} {} {}={} {}={display_role} {task_label}",
        subagent_status_marker(&status),
        status.replace('_', "-"),
        labels.child,
        status_display_agent(&child_agent),
        labels.role
    ))
}

fn compact_task_label(task: &Value, labels: SubagentRowLabels) -> Option<String> {
    compact_text_field(task, "title")
        .map(|value| {
            format!(
                "{}=\"{}\"",
                labels.task,
                summarize_text_for_visibility(&value, 72)
            )
        })
        .or_else(|| {
            compact_text_field(task, "task_card_id")
                .map(|value| format!("{}={value}", labels.task_card))
        })
}

fn payload_appears_korean(payload: &Value) -> bool {
    if compact_text_field(payload, "operator_language")
        .map(|value| value == "ko" || value == "korean")
        .unwrap_or(false)
    {
        return true;
    }

    [
        "/current_task_card/execution_prompt",
        "/current_task_card/prompt",
        "/current_task_card/request",
        "/current_task_card/intent",
        "/current_task_card/scope",
        "/longway/prompt",
        "/longway/request",
        "/request",
        "/prompt",
        "/goal",
    ]
    .iter()
    .filter_map(|pointer| payload.pointer(pointer).and_then(Value::as_str))
    .any(text_appears_korean)
}

fn text_appears_korean(text: &str) -> bool {
    text.chars().any(|ch| {
        ('\u{ac00}'..='\u{d7af}').contains(&ch)
            || ('\u{1100}'..='\u{11ff}').contains(&ch)
            || ('\u{3130}'..='\u{318f}').contains(&ch)
    })
}

fn compact_text_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn compact_text_pointer(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn subagent_status_marker(status: &str) -> &'static str {
    match status {
        "completed" | "passed" | "merged" | "materialized" => "x",
        "in_progress" | "running" | "active" | "ready" | "spawned" | "acknowledged" => ">",
        "failed" | "blocked" | "stalled" => "!",
        "cancelled" | "skipped" | "reclaimed" => "-",
        _ => " ",
    }
}

pub(crate) fn build_longway_checklist_block(payload: &Value) -> Option<String> {
    let longway = payload.get("longway")?;
    let rows = longway
        .get("phase_rows")
        .and_then(Value::as_array)
        .or_else(|| longway.get("phases").and_then(Value::as_array));
    let current_index = longway
        .get("current_item")
        .and_then(Value::as_str)
        .and_then(|value| value.strip_prefix("item-"))
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|value| value.checked_sub(1));
    let mut lines = vec!["LongWay".to_string()];
    let row_values;
    let rows = match rows {
        Some(rows) if !rows.is_empty() => rows,
        _ => {
            let phase_count = longway
                .get("phase_count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if phase_count == 0 {
                lines.extend(build_longway_planned_rows(payload));
                return Some(lines.join("\n"));
            }
            let active_phase_name = longway
                .get("active_phase_name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("item");
            let active_phase_status = longway
                .get("active_phase_status")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("pending");
            let active_title = payload
                .pointer("/current_task_card/title")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(active_phase_name);
            row_values = (0..phase_count)
                .map(|index| {
                    let is_current = current_index
                        .map(|current| current == index as usize)
                        .unwrap_or(index == 0);
                    json!({
                        "status": if is_current { active_phase_status } else { "pending" },
                        "title": if is_current { active_title } else { active_phase_name },
                    })
                })
                .collect::<Vec<_>>();
            &row_values
        }
    };
    for (index, row) in rows.iter().enumerate() {
        let status = match row.get("status").and_then(Value::as_str) {
            Some("completed") => "x",
            Some("failed") => "!",
            Some("cancelled" | "canceled") => "-",
            Some("in_progress" | "running" | "active") => ">",
            Some("pending") if current_index == Some(index) => ">",
            _ => " ",
        };
        let work_item = row
            .get("title")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                row.get("label")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                row.get("phase_name")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .map(|value| summarize_text_for_visibility(value, 96))
            .unwrap_or_else(|| "item".to_string());
        if let Some(owner_agent) = row
            .get("owner_agent")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            lines.push(render_longway_phase_row_block(
                status,
                &work_item,
                Some(owner_agent),
                row,
            ));
        } else {
            lines.push(render_longway_phase_row_block(
                status, &work_item, None, row,
            ));
        }
    }
    let planned_rows = build_longway_planned_rows(payload);
    if !planned_rows.is_empty()
        && !lines
            .iter()
            .any(|line| line.contains("Planned:") || line.contains("plan:"))
    {
        lines.extend(planned_rows);
    }
    Some(lines.join("\n"))
}

pub(crate) fn build_captain_intervention_line(payload: &Value) -> Option<String> {
    let intervention = payload
        .get("latest_captain_intervention")
        .filter(|value| value.is_object())
        .or_else(|| {
            payload
                .pointer("/current_task_card/captain_intervention")
                .filter(|value| value.is_object())
        })?;
    let mut parts = Vec::new();
    if let Some(classification) = intervention
        .get("classification")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("class={}", classification.replace('_', "-")));
    }
    if let Some(action) = intervention
        .get("chosen_next_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("next={}", action.replace('_', "-")));
    }
    if intervention
        .get("next_action_blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let reason = intervention
            .get("next_action_block_reason")
            .and_then(Value::as_str)
            .unwrap_or("blocked");
        parts.push(format!("blocked={}", reason.replace('_', "-")));
    }
    if let Some(follow_up_action) = intervention
        .pointer("/pending_follow_up/action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        let follow_up_status = intervention
            .pointer("/pending_follow_up/status")
            .and_then(Value::as_str)
            .unwrap_or("queued");
        if follow_up_status == "queued" {
            parts.push(format!("follow_up={}", follow_up_action.replace('_', "-")));
        } else {
            parts.push(format!(
                "follow_up={}:{}",
                follow_up_action.replace('_', "-"),
                follow_up_status.replace('_', "-")
            ));
        }
    }
    if let Some(policy) = intervention
        .get("stale_output_policy")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("stale={}", policy.replace('_', "-")));
    }
    if let Some(rationale) = intervention
        .get("rationale")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "rationale=\"{}\"",
            summarize_text_for_visibility(rationale, 120)
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("Intervention: {}", parts.join(" ")))
    }
}

pub(crate) fn build_sentinel_intervention_line(payload: &Value) -> Option<String> {
    let intervention = payload
        .get("latest_sentinel_intervention")
        .filter(|value| value.is_object())
        .or_else(|| {
            payload
                .pointer("/current_task_card/sentinel_intervention")
                .filter(|value| value.is_object())
        })?;
    let mut parts = Vec::new();
    if let Some(classification) = intervention
        .get("classification")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("class={}", classification.replace('_', "-")));
    }
    if let Some(action) = intervention
        .get("next_action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("next={}", action.replace('_', "-")));
    }
    if let Some(source) = intervention
        .get("source")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("source={}", source.replace('_', "-")));
    }
    if let Some(rationale) = intervention
        .get("rationale")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!(
            "rationale=\"{}\"",
            summarize_text_for_visibility(rationale, 120)
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("Sentinel: {}", parts.join(" ")))
    }
}

#[cfg(test)]
fn build_code_graph_line(payload: &Value) -> Option<String> {
    let code_graph = payload.get("code_graph")?;
    if !code_graph
        .get("available")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if code_graph
            .get("diagnostic_severity")
            .and_then(Value::as_str)
            == Some("warning")
        {
            let reason = code_graph
                .get("reason")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(|value| summarize_text_for_visibility(value, 180))
                .unwrap_or_else(|| "graph context unavailable".to_string());
            let blocking = code_graph
                .get("blocking")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            return Some(format!("Graph Warning: blocking={blocking} {reason}"));
        }
        return None;
    }
    let file_count = code_graph
        .get("file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let scope = code_graph
        .get("resolution")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("repo graph");
    let top_dirs = code_graph
        .pointer("/evidence_note/top_directories")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .take(3)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let found = if top_dirs.is_empty() {
        format!("{file_count} indexed files")
    } else {
        format!("{file_count} indexed files in {}", top_dirs.join(","))
    };
    Some(format!(
        "Graph: Way referenced {scope}; found {found}; graph-informed planning next step."
    ))
}

fn build_graph_context_line(payload: &Value) -> Option<String> {
    let graph_context = payload.get("graph_context")?;
    if !graph_context.is_object() {
        return None;
    }
    let provider = graph_context
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("graphify");
    let readiness = graph_context
        .get("readiness")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let reason = graph_context
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let fallback = graph_context
        .get("fallback")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let artifact_state = graph_context
        .get("artifact_state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Some(format!(
        "Graph Context: provider={provider} readiness={readiness} reason={reason} fallback={fallback} artifacts={artifact_state}"
    ))
}

fn build_registry_evidence_line(payload: &Value) -> Option<String> {
    let registry = payload.get("registry_evidence")?;
    if registry.is_null() {
        return None;
    }
    let agent = registry
        .get("agent_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("agent");
    let status = registry
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let ssl_status = registry
        .get("manifest_status")
        .and_then(Value::as_str)
        .or_else(|| {
            registry
                .pointer("/skill_ssl_manifest/status")
                .and_then(Value::as_str)
        })
        .unwrap_or("missing");
    let advisory = registry
        .get("advisory_only")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    Some(format!(
        "Registry: {} status={status} ssl={ssl_status} advisory={advisory}",
        status_display_agent(agent)
    ))
}

#[cfg(test)]
fn build_memory_line(payload: &Value) -> Option<String> {
    let memory = payload.get("memory")?;
    let configured = memory
        .get("configured")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let enabled = memory
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let entry_count = memory
        .get("entry_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stale = memory
        .get("stale")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let captain_instruction_count = memory
        .get("captain_instruction_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let captain_instruction_status = memory
        .get("captain_instruction_status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let captain_instruction_source_summary = memory
        .get("captain_instruction_source_summary")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let path = memory
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mode = if enabled {
        "enabled"
    } else if configured {
        "off"
    } else {
        "unconfigured"
    };
    Some(format!(
        "Memory: {mode} entries={entry_count} captain_instructions={captain_instruction_count} captain_instruction_status={captain_instruction_status} captain_instruction_source={captain_instruction_source_summary} stale={stale} path={path}"
    ))
}

#[cfg(test)]
pub(crate) fn create_ccc_status_text(payload: &Value) -> String {
    let output_config = payload
        .get("output")
        .cloned()
        .unwrap_or_else(load_output_config);
    let next_action = payload
        .get("next_step")
        .and_then(Value::as_str)
        .unwrap_or("see structuredContent.run_state");
    let fan_in_ready = payload
        .get("run_truth_surface")
        .and_then(|value| value.get("fan_in_ready"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let worker_active = payload
        .get("worker_visibility")
        .and_then(|value| value.get("active_worker_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next_line = next_actor_label(payload, next_action, fan_in_ready);
    let mut lines = operator_status_intro_lines(payload);
    if let Some(sequence) = payload
        .get("sequence")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        let stage = payload
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let approval_state = payload
            .get("approval_state")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        lines.push(format!(
            "Sequence: {sequence} stage={stage} approval={approval_state}"
        ));
    }

    if let Some(total_tokens) = payload
        .get("token_usage")
        .and_then(|value| value.get("total_tokens"))
        .and_then(Value::as_u64)
    {
        let by_agent = token_usage_by_agent(payload);
        let total_context_tokens = token_context_total(payload).unwrap_or(0);
        if total_tokens > 0 {
            lines.push(format!(
                "Tokens: {} used",
                format_compact_token_count(total_tokens)
            ));
        }
        if let Some(by_agent) = by_agent.filter(|_| total_tokens > 0) {
            if let Some(breakdown) = build_token_usage_breakdown(by_agent, total_tokens) {
                lines.push(format!("By Agent: {breakdown}"));
            }
        }
        if total_context_tokens > 0 {
            lines.push(format!(
                "Estimated Context: {}",
                format_compact_token_count(total_context_tokens)
            ));
            if let Some(by_agent) = by_agent {
                if let Some(breakdown) =
                    build_context_usage_breakdown(by_agent, total_context_tokens)
                {
                    lines.push(format!("By Agent Context Estimate: {breakdown}"));
                }
            }
        }
        if let Some(bar) = build_token_usage_bar(by_agent.unwrap_or(&[]), total_tokens)
            .or_else(|| build_context_usage_bar(by_agent.unwrap_or(&[]), total_context_tokens))
        {
            lines.push(format!("Gauge: {bar}"));
        }
    }

    if let Some(launch_visibility_line) = build_launch_visibility_line(payload) {
        lines.push(launch_visibility_line);
    }
    if let Some(runtime_dispatch_line) = build_runtime_dispatch_line(payload) {
        lines.push(runtime_dispatch_line);
    }
    if let Some(transport_guidance_line) = build_transport_guidance_line(payload) {
        lines.push(transport_guidance_line);
    }
    if let Some(cost_routing_line) = build_cost_routing_line(payload) {
        lines.push(cost_routing_line);
    }
    if let Some(assignment_quality_line) = build_assignment_quality_line(payload) {
        lines.push(assignment_quality_line);
    }
    if let Some(routing_trace_line) = build_routing_trace_line(payload) {
        lines.push(routing_trace_line);
    }
    if let Some(spec_split_line) = build_spec_split_line(payload) {
        lines.push(spec_split_line);
    }
    if let Some(lane_artifact_line) = build_lane_artifact_line(payload) {
        lines.push(lane_artifact_line);
    }
    if let Some(verify_retry_recap_report_line) = build_verify_retry_recap_report_line(payload) {
        lines.push(verify_retry_recap_report_line);
    }
    if let Some(code_graph_line) = build_code_graph_line(payload) {
        lines.push(code_graph_line);
    }
    if let Some(graph_context_line) = build_graph_context_line(payload) {
        lines.push(graph_context_line);
    }
    if let Some(registry_line) = build_registry_evidence_line(payload) {
        lines.push(registry_line);
    }
    if let Some(memory_line) = build_memory_line(payload) {
        lines.push(memory_line);
    }
    if let Some(completion_line) = build_completion_discipline_line(payload) {
        lines.push(completion_line);
    }
    if let Some(review_state_line) = build_review_state_line(payload) {
        lines.push(review_state_line);
    }
    if let Some(captain_action_contract_line) = build_captain_action_contract_line(payload) {
        lines.push(captain_action_contract_line);
    }
    if let Some(state_contract_line) = build_state_contract_line(payload) {
        lines.push(state_contract_line);
    }
    if let Some(active_checkpoint_line) = build_active_checkpoint_line(payload) {
        lines.push(active_checkpoint_line);
    }
    if let Some(task_session_state_line) = build_task_session_state_line(payload) {
        lines.push(task_session_state_line);
    }
    if let Some(verification_capsule_line) = build_verification_capsule_line(payload) {
        lines.push(verification_capsule_line);
    }
    if let Some(delegated_ownership_line) = build_delegated_ownership_line(payload) {
        lines.push(delegated_ownership_line);
    }
    if let Some(recovery_lane_line) = build_recovery_lane_line(payload) {
        lines.push(recovery_lane_line);
    }
    if let Some(workflow_loop_line) = build_workflow_loop_line(payload) {
        lines.push(workflow_loop_line);
    }
    if let Some(lifecycle_hooks_line) = build_lifecycle_hooks_line(payload) {
        lines.push(lifecycle_hooks_line);
    }
    if let Some(captain_direct_mutation_guard_line) =
        build_captain_direct_mutation_guard_line(payload)
    {
        lines.push(captain_direct_mutation_guard_line);
    }
    if let Some(intervention_line) = build_captain_intervention_line(payload) {
        lines.push(intervention_line);
    }
    if let Some(intervention_line) = build_sentinel_intervention_line(payload) {
        lines.push(intervention_line);
    }
    if worker_active > 0
        || output_config
            .get("include_agent_loop_when_idle")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        if let Some(agent_loop) = build_agent_loop_line(payload) {
            lines.push(agent_loop);
        }
    }
    if worker_active > 0 {
        let assigned_agent_id = payload
            .get("current_task_card")
            .and_then(|value| value.get("assigned_agent_id"))
            .and_then(Value::as_str)
            .unwrap_or("worker");
        lines.push(format!(
            "Spawned: {}",
            status_display_agent(assigned_agent_id)
        ));
    }
    if let Some(host_subagent_reclaim_summary) = payload
        .pointer("/host_subagent_state/reclaim_replan_recommendation/summary")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("Host Subagents: {host_subagent_reclaim_summary}"));
    }
    if let Some(rollover_summary) = long_session_mitigation_summary(payload) {
        lines.push(rollover_summary);
    }
    if let Some(handle_cleanup) = payload.pointer("/host_subagent_state/active_handle_cleanup") {
        let show_handle_cleanup = handle_cleanup
            .get("released_handle_count")
            .and_then(Value::as_u64)
            .map(|count| count > 0)
            .unwrap_or(false)
            || !handle_cleanup
                .get("active_thread_id")
                .unwrap_or(&Value::Null)
                .is_null()
            || matches!(
                handle_cleanup.get("state").and_then(Value::as_str),
                Some("active" | "released" | "already_clear")
            );
        if show_handle_cleanup {
            if handle_cleanup
                .get("host_close_required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let action = handle_cleanup
                    .get("host_close_action")
                    .and_then(Value::as_str)
                    .unwrap_or("close_agent");
                let target = handle_cleanup
                    .get("host_close_target")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        handle_cleanup
                            .pointer("/latest_cleanup/child_agent_id")
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("host subagent");
                lines.push(format!(
                    "Host Handles: released CCC handle; host {action} still required for {target}"
                ));
            } else if let Some(host_subagent_handle_summary) = handle_cleanup
                .get("summary")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                lines.push(format!("Host Handles: {host_subagent_handle_summary}"));
            }
        }
    }
    if let Some(changed) = payload
        .get("latest_delegate_result")
        .and_then(|value| value.get("assistant_message_preview"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            payload
                .get("latest_delegate_result")
                .and_then(|value| value.get("result_summary"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            payload
                .get("latest_orchestrator_synthesis")
                .and_then(Value::as_str)
                .filter(|value| {
                    let normalized = value.to_ascii_lowercase();
                    normalized.contains("reclaimed")
                        || normalized.contains("selected")
                        || normalized.contains("closed the run")
                        || normalized.contains("checkpoint")
                })
        })
    {
        let changed_max_chars = output_config
            .get("changed_max_chars")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(160);
        lines.push(format!(
            "Changed: {}",
            summarize_text_for_visibility(changed, changed_max_chars)
        ));
    }
    lines.push(format!("Next: {next_line}"));
    lines.join("\n")
}

pub(crate) fn create_ccc_status_operator_text(payload: &Value) -> String {
    let output_config = payload
        .get("output")
        .cloned()
        .unwrap_or_else(load_output_config);
    let next_action = payload
        .get("next_step")
        .and_then(Value::as_str)
        .unwrap_or("see structuredContent.run_state");
    let fan_in_ready = payload
        .get("run_truth_surface")
        .and_then(|value| value.get("fan_in_ready"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let next_line = next_actor_label(payload, next_action, fan_in_ready);
    let mut lines = operator_status_intro_lines(payload);
    if let Some(sequence) = payload
        .get("sequence")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        let stage = payload
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let approval_state = payload
            .get("approval_state")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        lines.push(format!(
            "Sequence: {sequence} stage={stage} approval={approval_state}"
        ));
    }
    if let Some(registry_line) = build_registry_evidence_line(payload) {
        lines.push(registry_line);
    }
    if let Some(graph_context_line) = build_graph_context_line(payload) {
        lines.push(graph_context_line);
    }
    if let Some(state_contract_line) = build_state_contract_line(payload) {
        lines.push(state_contract_line);
    }
    if let Some(active_checkpoint_line) = build_active_checkpoint_line(payload) {
        lines.push(active_checkpoint_line);
    }
    if let Some(task_session_state_line) = build_task_session_state_line(payload) {
        lines.push(task_session_state_line);
    }
    if let Some(verification_capsule_line) = build_verification_capsule_line(payload) {
        lines.push(verification_capsule_line);
    }
    if let Some(delegated_ownership_line) = build_delegated_ownership_line(payload) {
        lines.push(delegated_ownership_line);
    }
    if let Some(recovery_lane_line) = build_recovery_lane_line(payload) {
        lines.push(recovery_lane_line);
    }
    if let Some(workflow_loop_line) = build_workflow_loop_line(payload) {
        lines.push(workflow_loop_line);
    }
    if let Some(lifecycle_hooks_line) = build_lifecycle_hooks_line(payload) {
        lines.push(lifecycle_hooks_line);
    }
    if let Some(routing_trace_line) = build_routing_trace_line(payload) {
        lines.push(routing_trace_line);
    }
    if let Some(captain_direct_mutation_guard_line) =
        build_captain_direct_mutation_guard_line(payload)
    {
        lines.push(captain_direct_mutation_guard_line);
    }
    if let Some(intervention_line) = build_sentinel_intervention_line(payload) {
        lines.push(intervention_line);
    }
    if let Some(changed) = payload
        .get("latest_delegate_result")
        .and_then(|value| value.get("assistant_message_preview"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            payload
                .get("latest_delegate_result")
                .and_then(|value| value.get("result_summary"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            payload
                .get("latest_orchestrator_synthesis")
                .and_then(Value::as_str)
                .filter(|value| {
                    let normalized = value.to_ascii_lowercase();
                    normalized.contains("reclaimed")
                        || normalized.contains("selected")
                        || normalized.contains("closed the run")
                        || normalized.contains("checkpoint")
                })
        })
    {
        let changed_max_chars = output_config
            .get("changed_max_chars")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(160);
        lines.push(format!(
            "Changed: {}",
            summarize_text_for_visibility(changed, changed_max_chars)
        ));
    }
    lines.push(format!("Next: {next_line}"));
    lines.join("\n")
}
