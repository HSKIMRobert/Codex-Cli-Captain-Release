use crate::create_ccc_status_operator_text;
use crate::specialist_roles::{
    agent_id_for_role, load_role_config_snapshot, normalize_dispatch_role_hint, role_for_agent_id,
};
use crate::status_app_panel::create_codex_app_panel_text;
use crate::status_render::{build_longway_checklist_block, build_operator_projection_status_block};
use crate::text_utils::summarize_text_for_visibility;
use serde_json::Value;

pub(crate) fn create_status_quiet_line(payload: &Value) -> String {
    quiet_lifecycle_line(
        payload.get("run_id").and_then(Value::as_str),
        payload.get("status").and_then(Value::as_str),
        payload.get("next_step").and_then(Value::as_str),
    )
}

fn quiet_lifecycle_line(run_id: Option<&str>, status: Option<&str>, next: Option<&str>) -> String {
    format!(
        "run_id={} status={} next={}",
        run_id.unwrap_or("unknown-run"),
        status.unwrap_or("unknown"),
        next.unwrap_or("unknown"),
    )
}

pub(crate) fn create_checklist_text(status_payload: &Value) -> String {
    if let Some(block) = build_longway_checklist_block(status_payload) {
        if block.lines().count() > 1 {
            return block;
        }
    }
    let planned_lines = concise_planned_rows(status_payload);
    if planned_lines.is_empty() {
        "LongWay".to_string()
    } else {
        let mut lines = vec!["LongWay".to_string()];
        lines.extend(planned_lines);
        lines.join("\n")
    }
}

pub(crate) fn create_checklist_quiet_text(status_payload: &Value) -> String {
    let planned_lines = concise_planned_rows(status_payload);
    if !planned_lines.is_empty() {
        return planned_lines.join("\n");
    }
    let lines = concise_phase_rows(status_payload);
    if lines.is_empty() {
        "LongWay".to_string()
    } else {
        lines.join("\n")
    }
}

fn concise_phase_rows(status_payload: &Value) -> Vec<String> {
    status_payload
        .pointer("/longway/phase_rows")
        .or_else(|| status_payload.pointer("/longway/phases"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let title = row_text(row, "title")
                        .or_else(|| row_text(row, "label"))
                        .or_else(|| row_text(row, "phase_name"))?;
                    let status = row
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending");
                    let agent = row_text(row, "owner_agent")
                        .or_else(|| row_text(row, "assigned_agent_id"))
                        .or_else(|| current_task_agent(status_payload))
                        .unwrap_or_else(|| "unassigned".to_string());
                    let model = row_text(row, "model")
                        .or_else(|| current_task_model(status_payload))
                        .unwrap_or_else(|| "unknown".to_string());
                    let reasoning = row_text(row, "reasoning")
                        .or_else(|| row_text(row, "variant"))
                        .or_else(|| current_task_variant(status_payload))
                        .unwrap_or_else(|| "unknown".to_string());
                    Some(format!(
                        "[{}] {} - {} - {} / {}",
                        checklist_symbol(status),
                        summarize_text_for_visibility(&title, 96),
                        display_agent(&agent),
                        model,
                        reasoning
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn concise_planned_rows(status_payload: &Value) -> Vec<String> {
    status_payload
        .pointer("/app_panel/longway_progress/planned_rows")
        .or_else(|| status_payload.pointer("/longway/planned_rows"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let title = row_text(row, "title")?;
                    let status = row
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("planned");
                    let agent = row_text(row, "display_agent_id")
                        .or_else(|| row_text(row, "planned_agent_id"))
                        .or_else(|| row_text(row, "owner_agent"))
                        .or_else(|| {
                            row_text(row, "display_role")
                                .or_else(|| row_text(row, "planned_role"))
                                .and_then(|role| agent_id_for_role(&role).map(str::to_string))
                        })
                        .unwrap_or_else(|| "unassigned".to_string());
                    let role_config = planned_row_role_config_snapshot(row, &agent);
                    let model = row_text(row, "model")
                        .or_else(|| {
                            role_config
                                .as_ref()
                                .and_then(|snapshot| row_text(snapshot, "model"))
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    let reasoning = row_text(row, "reasoning")
                        .or_else(|| row_text(row, "variant"))
                        .or_else(|| {
                            role_config
                                .as_ref()
                                .and_then(|snapshot| row_text(snapshot, "variant"))
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    Some(format!(
                        "[{}] {} - {} - {} / {}",
                        checklist_symbol(status),
                        summarize_text_for_visibility(&title, 96),
                        display_agent(&agent),
                        model,
                        reasoning
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn planned_row_role_config_snapshot(row: &Value, agent: &str) -> Option<Value> {
    let mut role_candidates = Vec::new();
    for key in ["display_role", "planned_role", "role"] {
        if let Some(role) = row_text(row, key) {
            role_candidates.push(role);
        }
    }
    if let Some(role) = role_for_display_agent(agent) {
        role_candidates.push(role.to_string());
    }

    role_candidates.into_iter().find_map(|role| {
        let normalized = normalize_dispatch_role_hint(Some(&role), &role);
        let snapshot = load_role_config_snapshot(&normalized);
        if snapshot.get("model").and_then(Value::as_str).is_some()
            || snapshot.get("variant").and_then(Value::as_str).is_some()
        {
            Some(snapshot)
        } else {
            None
        }
    })
}

fn role_for_display_agent(agent: &str) -> Option<&'static str> {
    let normalized = agent.trim().trim_start_matches("ccc_");
    role_for_agent_id(normalized).or_else(|| {
        normalized
            .split_once('-')
            .and_then(|(agent_prefix, _)| role_for_agent_id(agent_prefix))
    })
}

fn row_text(row: &Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "unassigned")
        .map(str::to_string)
}

fn current_task_agent(payload: &Value) -> Option<String> {
    payload
        .pointer("/current_task_card/assigned_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn current_task_model(payload: &Value) -> Option<String> {
    payload
        .get("current_task_card")
        .and_then(current_task_model_value)
        .map(str::to_string)
}

fn current_task_variant(payload: &Value) -> Option<String> {
    payload
        .get("current_task_card")
        .and_then(current_task_variant_value)
        .map(str::to_string)
}

fn display_agent(agent: &str) -> String {
    match agent {
        "scout" | "scribe" | "raider" | "arbiter" | "tactician" | "companion_reader"
        | "companion_operator" => format!("ccc_{agent}"),
        _ => agent.to_string(),
    }
}

fn checklist_symbol(status: &str) -> &'static str {
    match status {
        "completed" | "passed" | "merged" | "materialized" => "x",
        "in_progress" | "running" | "active" | "ready" => ">",
        "failed" | "blocked" | "stalled" => "!",
        "cancelled" | "skipped" | "reclaimed" => "-",
        _ => " ",
    }
}

pub(crate) fn create_start_text_line(start_payload: &Value, status_payload: &Value) -> String {
    let mut lines = vec![format!(
        "Run {} created.",
        start_payload
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown-run")
    )];
    if let Some(projection) = build_operator_projection_status_block(status_payload) {
        lines.push(projection);
    } else if let Some(panel) = compact_progress_panel(status_payload) {
        lines.push(panel);
    } else {
        lines.push(
            build_lifecycle_current_work_line(status_payload)
                .unwrap_or_else(|| "Current Work: unavailable".to_string()),
        );
    }
    lines.push(build_lifecycle_next_line(status_payload));
    lines.join("\n")
}

pub(crate) fn create_start_quiet_line(start_payload: &Value, status_payload: &Value) -> String {
    quiet_lifecycle_line(
        start_payload.get("run_id").and_then(Value::as_str),
        start_payload
            .get("status")
            .and_then(Value::as_str)
            .or_else(|| status_payload.get("status").and_then(Value::as_str)),
        start_payload
            .get("next_step")
            .and_then(Value::as_str)
            .or_else(|| status_payload.get("next_step").and_then(Value::as_str)),
    )
}

pub(crate) fn create_orchestrate_text_line(
    orchestrate_payload: &Value,
    status_payload: &Value,
) -> String {
    let summary = orchestrate_payload
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("Rust ccc_orchestrate persisted an explicit checkpoint.");
    let mut lines = vec![summary.to_string()];
    if let Some(projection) = build_operator_projection_status_block(status_payload) {
        lines.push(projection);
    } else if let Some(panel) = compact_progress_panel(status_payload) {
        lines.push(panel);
    } else {
        lines.push(create_ccc_status_operator_text(status_payload));
    }
    lines.push(build_lifecycle_next_line(status_payload));
    lines.join("\n")
}

fn compact_progress_panel(status_payload: &Value) -> Option<String> {
    let app_panel = status_payload.get("app_panel")?;
    if app_panel.is_null() {
        None
    } else {
        Some(create_codex_app_panel_text(app_panel))
    }
}

pub(crate) fn create_orchestrate_quiet_line(
    orchestrate_payload: &Value,
    status_payload: &Value,
) -> String {
    quiet_lifecycle_line(
        orchestrate_payload.get("run_id").and_then(Value::as_str),
        status_payload.get("status").and_then(Value::as_str),
        orchestrate_payload
            .get("next_step")
            .and_then(Value::as_str)
            .or_else(|| status_payload.get("next_step").and_then(Value::as_str)),
    )
}

pub(crate) fn create_subagent_update_text_line(
    update_payload: &Value,
    status_payload: &Value,
) -> String {
    let mut lines = vec![
        "CCC Subagent".to_string(),
        build_lifecycle_event_line(update_payload, status_payload),
    ];
    if let Some(summary) = update_payload
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!(
            "Summary: {}",
            summarize_text_for_visibility(summary, 120)
        ));
    }
    if let Some(sentinel_line) = build_update_sentinel_intervention_line(update_payload) {
        lines.push(sentinel_line);
    }
    lines.push(build_lifecycle_next_line(status_payload));
    box_transcript_lines(&lines)
}

fn build_lifecycle_event_line(update_payload: &Value, status_payload: &Value) -> String {
    let agent = update_payload
        .get("child_agent_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown-agent");
    let status = update_payload
        .get("subagent_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let event = subagent_lifecycle_event_label(status);
    let role = status_payload
        .get("current_task_card")
        .and_then(|value| value.get("assigned_role"))
        .and_then(Value::as_str)
        .unwrap_or("unassigned");
    let model = status_payload
        .get("current_task_card")
        .and_then(current_task_model_value)
        .unwrap_or("unknown");
    let reasoning = status_payload
        .get("current_task_card")
        .and_then(current_task_variant_value)
        .unwrap_or("unknown");
    let task = status_payload
        .get("current_task_card")
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .map(|value| summarize_text_for_visibility(value, 80))
        .unwrap_or_else(|| "task".to_string());
    let mut parts = vec![format!(
        "{event}: {agent} role={role} model={model} reasoning={reasoning} task=\"{task}\""
    )];
    if let Some(lane_id) = update_payload
        .get("lane_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("lane={lane_id}"));
    }
    parts.join(" ")
}

fn build_update_sentinel_intervention_line(update_payload: &Value) -> Option<String> {
    let intervention = update_payload
        .get("sentinel_intervention")
        .filter(|value| value.is_object())?;
    let mut parts = Vec::new();
    if let Some(classification) = compact_intervention_field(intervention, "classification") {
        parts.push(format!("class={classification}"));
    }
    if let Some(next_action) = compact_intervention_field(intervention, "next_action") {
        parts.push(format!("next={next_action}"));
    }
    if let Some(source) = compact_intervention_field(intervention, "source") {
        parts.push(format!("source={source}"));
    }
    if let Some(summary) = intervention
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!(
            "summary=\"{}\"",
            summarize_text_for_visibility(summary, 96)
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("Sentinel: {}", parts.join(" ")))
    }
}

fn compact_intervention_field(intervention: &Value, key: &str) -> Option<String> {
    intervention
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('_', "-"))
}

fn box_transcript_lines(lines: &[String]) -> String {
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .min(160);
    let border = format!("+{}+", "-".repeat(width + 2));
    let mut boxed = Vec::with_capacity(lines.len() + 2);
    boxed.push(border.clone());
    boxed.extend(lines.iter().map(|line| {
        let line = summarize_text_for_visibility(line, width);
        let padding = width.saturating_sub(line.chars().count());
        format!("| {line}{} |", " ".repeat(padding))
    }));
    boxed.push(border);
    boxed.join("\n")
}

fn subagent_lifecycle_event_label(status: &str) -> &str {
    match status {
        "spawned" | "acknowledged" => "opened",
        "running" => "running",
        "completed" => "completed",
        "merged" | "reclaimed" => "closed",
        "failed" | "stalled" => "stopped",
        _ => status,
    }
}

fn current_task_model_value(task: &Value) -> Option<&str> {
    task.pointer("/runtime_dispatch/model")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/model"))
        .or_else(|| task.pointer("/delegation_plan/model"))
        .or_else(|| task.pointer("/role_config_snapshot/model"))
        .and_then(Value::as_str)
}

fn current_task_variant_value(task: &Value) -> Option<&str> {
    task.pointer("/runtime_dispatch/variant")
        .or_else(|| task.pointer("/delegation_plan/runtime_dispatch/variant"))
        .or_else(|| task.pointer("/delegation_plan/variant"))
        .or_else(|| task.pointer("/role_config_snapshot/variant"))
        .and_then(Value::as_str)
}

fn build_lifecycle_current_work_line(payload: &Value) -> Option<String> {
    let task = payload.get("current_task_card")?;
    let title = task
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            task.get("execution_prompt")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })?;
    let title = summarize_text_for_visibility(title, 96);
    let role = task
        .get("assigned_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let agent = task
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let owner = match (role, agent) {
        (Some(role), Some(agent)) => format!(" role={role} agent={agent}"),
        (Some(role), None) => format!(" role={role}"),
        (None, Some(agent)) => format!(" agent={agent}"),
        (None, None) => String::new(),
    };
    Some(format!("Current Work: {title}{owner}"))
}

fn build_lifecycle_next_line(payload: &Value) -> String {
    let next_step = payload
        .get("next_step")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let fan_in_ready = payload
        .get("run_truth_surface")
        .and_then(|value| value.get("fan_in_ready"))
        .and_then(Value::as_bool)
        .or_else(|| {
            payload
                .get("host_subagent_state")
                .and_then(|value| value.get("fan_in_ready"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    let actor = if fan_in_ready || next_step == "advance" {
        "captain".to_string()
    } else if matches!(
        next_step,
        "halt_completed" | "halt_failed" | "halt_cancelled"
    ) {
        "completed".to_string()
    } else if next_step == "await_operator" {
        "operator".to_string()
    } else if next_step == "execute_task" {
        payload
            .get("current_task_card")
            .and_then(|value| value.get("assigned_agent_id"))
            .and_then(Value::as_str)
            .unwrap_or("worker")
            .to_string()
    } else {
        next_step.to_string()
    };
    format!("Next: {actor}")
}

pub(crate) fn create_subagent_update_quiet_line(
    update_payload: &Value,
    status_payload: &Value,
) -> String {
    quiet_lifecycle_line(
        update_payload.get("run_id").and_then(Value::as_str),
        update_payload
            .get("subagent_status")
            .and_then(Value::as_str),
        status_payload.get("next_step").and_then(Value::as_str),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn quiet_status_line_is_minimal_lifecycle_summary() {
        let line = create_status_quiet_line(&json!({
            "run_id": "run-quiet",
            "status": "active",
            "stage": "execution",
            "next_step": "advance",
            "can_advance": true,
            "token_usage": {
                "total_tokens": 3100
            }
        }));
        assert_eq!(line, "run_id=run-quiet status=active next=advance");
        assert!(!line.contains("tokens="));
        assert!(!line.contains("gauge="));
    }

    #[test]
    fn quiet_start_line_omits_token_unavailable_reason() {
        let line = create_start_quiet_line(
            &json!({
                "run_id": "run-start",
                "status": "active",
                "task_card_id": "task-1",
                "next_step": "execute_task",
                "can_advance": true
            }),
            &json!({
                "host_subagent_state": {
                    "total_subagent_count": 1
                },
                "token_usage": {
                    "total_tokens": 0,
                    "by_subagent": []
                }
            }),
        );
        assert_eq!(line, "run_id=run-start status=active next=execute_task");
        assert!(!line.contains("token_reason="));
        assert!(!line.contains("gauge="));
    }

    #[test]
    fn quiet_start_line_omits_role_and_agent_from_status() {
        let line = create_start_quiet_line(
            &json!({
                "run_id": "run-start",
                "status": "active",
                "task_card_id": "task-1",
                "next_step": "execute_task",
                "can_advance": true
            }),
            &json!({
                "current_task_card": {
                    "assigned_role": "documenter",
                    "assigned_agent_id": "scribe"
                }
            }),
        );
        assert_eq!(line, "run_id=run-start status=active next=execute_task");
        assert!(!line.contains("role="));
        assert!(!line.contains("agent="));
    }

    #[test]
    fn checklist_text_line_renders_standalone_longway_block() {
        let line = create_checklist_text(&json!({
            "longway": {
                "completed_phase_count": 0,
                "phase_count": 1,
                "current_item": "item-1",
                "phases": [
                    {
                        "phase_name": "mutate",
                        "status": "pending",
                        "assigned_agent_id": "raider"
                    }
                ]
            }
        }));
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "LongWay");
        assert_eq!(lines[1], "[>] mutate");
        assert!(!line.contains("Current Work:"));
        assert!(!line.contains("Next:"));
        assert!(!line.contains("LongWay Progress"));
        assert!(!line.contains("Gauge:"));
    }

    #[test]
    fn checklist_text_line_renders_current_item_when_rows_are_absent() {
        let line = create_checklist_text(&json!({
            "current_task_card": {
                "title": "Verify standalone checklist"
            },
            "longway": {
                "active_phase_name": "way",
                "active_phase_status": "pending",
                "completed_phase_count": 0,
                "phase_count": 1,
                "current_item": "item-1",
                "phase_rows": []
            }
        }));
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(lines, vec!["LongWay", "[>] Verify standalone checklist"]);
        assert!(!line.contains("LongWay:"));
        assert!(!line.contains("Current Item:"));
    }

    #[test]
    fn start_text_line_is_lifecycle_output_with_checklist_command() {
        let line = create_start_text_line(
            &json!({
                "run_id": "run-start"
            }),
            &json!({
                "current_task_card": {
                    "title": "Implement bounded status progress visibility",
                    "assigned_role": "code specialist",
                    "assigned_agent_id": "raider"
                },
                "next_step": "execute_task",
                "longway": {
                    "completed_phase_count": 0,
                    "phase_count": 1,
                    "current_item": "item-1",
                    "phases": [
                        {
                            "phase_name": "mutate",
                            "status": "pending",
                            "assigned_agent_id": "raider"
                        }
                    ]
                },
                "output": {
                    "verbosity": "default",
                    "changed_max_chars": 120,
                    "include_agent_loop_when_idle": false
                }
            }),
        );
        assert!(line.contains("Run run-start created."));
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[1],
            "Current Work: Implement bounded status progress visibility role=code specialist agent=raider"
        );
        assert_eq!(lines[2], "Next: raider");
        assert!(!line.contains("[>] mutate"));
        assert!(!line.contains("LongWay Progress"));
        assert!(!line.contains("Gauge:"));
    }

    #[test]
    fn start_text_line_points_to_projection_when_diff_flow_is_active() {
        let line = create_start_text_line(
            &json!({
                "run_id": "run-start"
            }),
            &json!({
                "run_id": "run-start",
                "operator_longway_projection": {
                    "kind": "ccc_longway_projection",
                    "path": "/tmp/work/CCC_LONGWAY_PROJECTION.md",
                    "diff_visibility": {
                        "diff_command": "git diff -- CCC_LONGWAY_PROJECTION.md"
                    }
                },
                "next_step": "await_operator",
                "longway": {
                    "phase_rows": [{
                        "title": "Draft bounded plan",
                        "status": "in_progress",
                        "owner_agent": "ccc_tactician"
                    }]
                },
                "app_panel": {
                    "longway_progress": {
                        "phase_rows": [{
                            "title": "Draft bounded plan",
                            "status": "in_progress"
                        }]
                    }
                }
            }),
        );

        assert!(line.contains("Run run-start created."));
        assert!(line.contains("LongWay Projection: CCC_LONGWAY_PROJECTION.md"));
        assert!(line.contains("ccc status --projection --json '{\"run_id\":\"run-start\"}'"));
        assert!(line.contains("Diff: git diff -- CCC_LONGWAY_PROJECTION.md"));
        assert!(!line.contains("[>] Draft bounded plan"));
        assert!(!line.contains("CCC LongWay"));
    }

    #[test]
    fn checklist_text_line_renders_multiphase_longway_block() {
        let line = create_checklist_text(&json!({
            "longway": {
                "completed_phase_count": 1,
                "phase_count": 3,
                "current_item": "item-2",
                "phases": [
                    {
                        "phase_name": "inspect",
                        "status": "completed",
                        "assigned_agent_id": "scout"
                    },
                    {
                        "phase_name": "mutate",
                        "status": "active",
                        "assigned_agent_id": "raider"
                    },
                    {
                        "phase_name": "verify",
                        "status": "pending",
                        "assigned_agent_id": "captain"
                    }
                ]
            }
        }));
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(
            lines,
            vec!["LongWay", "[x] inspect", "[>] mutate", "[ ] verify"]
        );
        assert!(!line.contains("LongWay Progress"));
    }

    #[test]
    fn checklist_quiet_line_renders_ordered_work_rows_without_source_metadata() {
        let line = create_checklist_quiet_text(&json!({
            "longway": {
                "phase_rows": [{
                    "title": "Whole operator request summary",
                    "status": "in_progress",
                    "owner_agent": "ccc_tactician"
                }]
            },
            "app_panel": {
                "longway_progress": {
                    "planned_rows": [
                        {
                            "title": "Inspect output surfaces",
                            "status": "planned",
                            "display_agent_id": "ccc_scout",
                            "model": "gpt-5.4-mini",
                            "reasoning": "high",
                            "agent_source": "role_config",
                            "model_source": "role_config",
                            "reasoning_source": "role_config"
                        },
                        {
                            "title": "Implement concise LongWay",
                            "status": "planned",
                            "planned_role": "code specialist",
                            "model": "gpt-5.3-codex",
                            "variant": "high",
                            "agent_source": "role_mapping",
                            "model_source": "role_config",
                            "reasoning_source": "role_config"
                        }
                    ]
                }
            }
        }));
        assert_eq!(
            line.lines().collect::<Vec<_>>(),
            vec![
                "[ ] Inspect output surfaces - ccc_scout - gpt-5.4-mini / high",
                "[ ] Implement concise LongWay - ccc_raider - gpt-5.3-codex / high"
            ]
        );
        assert!(!line.contains("sources="));
        assert!(!line.contains("CCC LongWay"));
        assert!(!line.contains("Whole operator request summary"));
        assert!(!line.contains("+---"));
    }

    #[test]
    fn checklist_text_line_renders_planned_rows_without_app_panel_box() {
        let line = create_checklist_text(&json!({
            "longway": {
                "phase_count": 0,
                "planned_rows": [
                    {
                        "title": "Inspect output surfaces",
                        "status": "planned",
                        "planned_agent_id": "ccc_scout",
                        "planned_role": "explorer",
                        "agent_source": "role_config",
                        "model_source": "role_config",
                        "reasoning_source": "role_config",
                        "scope": "Inspect status text.",
                        "acceptance": "Rows stay visible."
                    },
                    {
                        "title": "Implement concise LongWay",
                        "status": "planned",
                        "planned_agent_id": "ccc_raider",
                        "planned_role": "code specialist"
                    }
                ]
            },
            "app_panel": {
                "longway_progress": {
                    "planned_rows": [{
                        "title": "Wide panel row should not replace LongWay",
                        "status": "planned",
                        "display_agent_id": "ccc_scribe"
                    }]
                }
            }
        }));
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "LongWay");
        assert!(line.contains(
            "Planned: Inspect output surfaces [Observer(ccc_scout)] role=Observer(ccc_scout)/explorer"
        ));
        assert!(
            line.contains(
                "Planned: Implement concise LongWay [Marauder(ccc_raider)] role=Marauder(ccc_raider)/code specialist"
            )
        );
        assert!(!line.contains("Wide panel row should not replace LongWay"));
        assert!(!line.contains("+---"));
        assert!(!line.contains("sources="));
        assert!(!line.contains("scope="));
        assert!(!line.contains("accept="));
        assert!(!line.contains("CCC LongWay"));
        assert!(!line.contains("Gauge:"));
    }

    #[test]
    fn checklist_quiet_line_resolves_companion_operator_model_for_operator_planned_row() {
        let line = create_checklist_quiet_text(&json!({
            "longway": {
                "planned_rows": [{
                    "title": "Build and package release assets",
                    "status": "planned",
                    "planned_role": "operator",
                    "planned_agent_id": "companion_operator"
                }]
            }
        }));

        assert_eq!(
            line,
            "[ ] Build and package release assets - ccc_companion_operator - gpt-5.4-mini / medium"
        );
    }

    #[test]
    fn subagent_update_text_line_is_lifecycle_output_with_checklist_command() {
        let line = create_subagent_update_text_line(
            &json!({
                "summary": "Raider finished the bounded implementation.",
                "child_agent_id": "raider",
                "subagent_status": "completed",
                "fan_in": {
                    "status": "completed"
                },
                "active_handle_cleanup": {
                    "state": "released"
                }
            }),
            &json!({
                "next_step": "await_fan_in",
                "host_subagent_state": {
                    "fan_in_ready": true
                },
                "current_task_card": {
                    "title": "Implement bounded status progress visibility",
                    "assigned_role": "code specialist",
                    "runtime_dispatch": {
                        "model": "gpt-5.3-codex",
                        "variant": "high"
                    }
                },
                "run_id": "run-subagent",
                "longway": {
                    "completed_phase_count": 1,
                    "phase_count": 1,
                    "current_item": "none",
                    "phases": [
                        {
                            "phase_name": "mutate",
                            "status": "completed",
                            "assigned_agent_id": "raider"
                        }
                    ]
                }
            }),
        );
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 6);
        assert!(lines[0].starts_with('+'));
        assert!(lines[1].contains("CCC Subagent"));
        assert!(lines[2]
            .contains("completed: raider role=code specialist model=gpt-5.3-codex reasoning=high"));
        assert!(lines[2].contains("task=\"Implement bounded status progress visibility\""));
        assert!(lines[3].contains("Summary: Raider finished the bounded implementation."));
        assert!(lines[4].contains("Next: captain"));
        assert!(lines[5].starts_with('+'));
        assert!(!line.contains("[x] mutate"));
        assert!(!line.contains("LongWay Progress"));
        assert!(!line.contains("Captain Guard:"));
        assert!(!line.contains("ccc checklist"));
    }

    #[test]
    fn subagent_update_text_line_renders_sentinel_intervention_line() {
        let line = create_subagent_update_text_line(
            &json!({
                "summary": "Captain directly produced output for a specialist-owned task.",
                "child_agent_id": "captain",
                "subagent_status": "completed",
                "sentinel_intervention": {
                    "classification": "enforce",
                    "next_action": "require_acceptance_gate",
                    "source": "policy_drift_guardrail",
                    "summary": "Direct captain output requires an acceptance gate."
                }
            }),
            &json!({
                "next_step": "await_operator",
                "current_task_card": {
                    "title": "Repair Sentinel schema and text output",
                    "assigned_role": "code specialist"
                }
            }),
        );

        assert!(line.contains("Sentinel: class=enforce next=require-acceptance-gate source=policy-drift-guardrail summary=\"Direct captain output requires an acceptance gate.\""));
        assert!(line.contains("Next: operator"));
    }

    #[test]
    fn subagent_update_text_line_maps_open_running_completed_and_closed_events() {
        let status_payload = json!({
            "next_step": "execute_task",
            "current_task_card": {
                "title": "Inspect lifecycle text",
                "assigned_role": "explorer",
                "delegation_plan": {
                    "runtime_dispatch": {
                        "model": "gpt-5.4-mini",
                        "variant": "medium"
                    }
                }
            }
        });
        for (status, label) in [
            ("spawned", "opened"),
            ("running", "running"),
            ("completed", "completed"),
            ("merged", "closed"),
        ] {
            let line = create_subagent_update_text_line(
                &json!({
                    "child_agent_id": "ccc_scout",
                    "subagent_status": status,
                    "lane_id": "scout-a"
                }),
                &status_payload,
            );
            assert!(line.contains(&format!("{label}: ccc_scout")));
            assert!(line.contains("role=explorer model=gpt-5.4-mini reasoning=medium"));
            assert!(line.contains("lane=scout-a"));
            assert!(line.contains("Next: worker"));
            assert!(line.starts_with('+'));
        }
    }

    #[test]
    fn quiet_orchestrate_and_subagent_lines_are_minimal_lifecycle_summaries() {
        let status_payload = json!({
            "status": "active",
            "next_step": "await_fan_in"
        });
        let orchestrate_line = create_orchestrate_quiet_line(
            &json!({
                "run_id": "run-orch",
                "attempt_id": "attempt-1",
                "next_step": "execute_task",
                "can_advance": false
            }),
            &status_payload,
        );
        assert_eq!(
            orchestrate_line,
            "run_id=run-orch status=active next=execute_task"
        );

        let subagent_line = create_subagent_update_quiet_line(
            &json!({
                "run_id": "run-orch",
                "child_agent_id": "ccc_raider",
                "lane_id": "raider-a",
                "subagent_status": "completed"
            }),
            &json!({
                "next_step": "execute_task",
                "token_usage": {
                    "total_tokens": 1200
                }
            }),
        );
        assert_eq!(
            subagent_line,
            "run_id=run-orch status=completed next=execute_task"
        );
        assert!(!subagent_line.contains("tokens="));
        assert!(!subagent_line.contains("gauge="));
    }
}
