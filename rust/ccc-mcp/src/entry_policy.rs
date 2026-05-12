use crate::install_check::create_server_identity_payload;
use crate::request_routing::{
    create_companion_tool_route_payload, infer_mutation_intent, infer_request_shape,
    infer_task_shape,
};
use crate::review_policy::{
    collect_os_review_pressure_snapshot, create_review_policy_payload, read_u64_field,
    runtime_review_pressure_snapshot_from_value, RuntimeReviewPressureSnapshot,
};
use crate::run_bootstrap::create_ccc_start_payload;
use crate::run_locator::{resolve_run_locator_arguments, resolve_workspace_path};
use crate::specialist_roles::load_role_config_snapshot;
use crate::token_usage::create_token_usage_payload;
use crate::{
    create_ccc_status_payload, create_worker_visibility_payload, load_runtime_config,
    read_json_document, read_optional_shared_config_document, SessionContext,
};
use serde_json::{json, Value};
use std::io;
use std::path::Path;

pub(crate) fn normalize_entry_policy_mode_value(value: &str) -> Option<&'static str> {
    match value {
        "explicit_only" => Some("explicit_only"),
        "guided_explicit" => Some("guided_explicit"),
        "ccc_first_bounded" => Some("ccc_first_bounded"),
        "codex_cli_ccc_first" | "codex_cli_foreman_first" => Some("codex_cli_ccc_first"),
        _ => None,
    }
}

fn normalize_entry_policy_mode(value: Option<&str>) -> String {
    value
        .and_then(normalize_entry_policy_mode_value)
        .unwrap_or("guided_explicit")
        .to_string()
}

fn load_entry_policy_mode() -> String {
    read_optional_shared_config_document()
        .ok()
        .flatten()
        .map(|(_, config)| config)
        .and_then(|config| config.get("entry_policy").cloned())
        .and_then(|entry_policy| entry_policy.get("mode").cloned())
        .and_then(|mode| {
            mode.as_str()
                .map(|value| normalize_entry_policy_mode(Some(value)))
        })
        .unwrap_or_else(|| "guided_explicit".to_string())
}

fn create_entry_policy_summary(policy_mode: &str) -> &'static str {
    match policy_mode {
        "codex_cli_ccc_first" => {
            "Entry policy prefers CCC-first for fresh MCP requests through the bounded auto-entry surface."
        }
        "ccc_first_bounded" => {
            "Entry policy is opt-in CCC-first on the explicit auto-entry surface."
        }
        _ => "Entry policy remains explicit and expects recommend-entry plus explicit start/run surfaces.",
    }
}

fn create_entry_boundary_payload(policy_mode: &str) -> Value {
    match policy_mode {
        "codex_cli_ccc_first" => json!({
            "entry_boundary": "session_instruction_plus_wrapper",
            "entry_boundary_summary": "The supported CCC-first boundary is bounded MCP session guidance plus the explicit wrapper surface."
        }),
        "ccc_first_bounded" => json!({
            "entry_boundary": "explicit_auto_entry",
            "entry_boundary_summary": "The supported CCC-first boundary is the explicit auto-entry surface only."
        }),
        _ => json!({
            "entry_boundary": "explicit_cli_or_mcp",
            "entry_boundary_summary": "The supported CCC entry boundary stays fully explicit."
        }),
    }
}

fn create_entry_guard_reason(
    request_shape: &str,
    task_shape: &str,
    documented_completion_requested: bool,
    direct_allowed: bool,
) -> String {
    if direct_allowed {
        "The request is a bounded read-only lookup and can proceed directly without mutating CCC state."
            .to_string()
    } else if documented_completion_requested {
        "The request asks for documented completion, so captain should stay inside CCC state until the criteria are met or explicitly blocked."
            .to_string()
    } else if request_shape == "mutation" {
        "The request contains mutation signals and should not bypass CCC control-plane state."
            .to_string()
    } else if request_shape == "review" {
        "The request is a review pass and should stay inside CCC control-plane truth before any follow-up."
            .to_string()
    } else if task_shape == "multi_step_or_unclear" {
        "The request is multi-step or unclear and should stay inside CCC control-plane state before execution."
            .to_string()
    } else {
        "The request should stay inside CCC control-plane state before execution.".to_string()
    }
}

fn create_entry_guard_active_run_summary() -> &'static str {
    "ccc_recommend_entry does not scan active runs; use ccc_status for run-scoped truth."
}

fn infer_recommended_entrypoint(request_shape: &str) -> &'static str {
    match request_shape {
        "lookup" | "diagnostic" | "review" => "start",
        _ => "way",
    }
}

fn infer_recommended_task_kind(request_shape: &str) -> &'static str {
    match request_shape {
        "review" => "review",
        "lookup" | "diagnostic" => "explore",
        _ => "way",
    }
}

pub(crate) fn runtime_review_pressure_snapshot_from_run_directory(
    run_directory: &Path,
) -> io::Result<Option<RuntimeReviewPressureSnapshot>> {
    let runtime_config = load_runtime_config()?;
    let run_record = read_json_document(&run_directory.join("run.json"))?;
    let active_task_card_id = run_record
        .get("active_task_card_id")
        .and_then(Value::as_str);
    let worker_visibility =
        create_worker_visibility_payload(run_directory, active_task_card_id, &runtime_config)?;
    let token_usage = create_token_usage_payload(run_directory)?;
    let snapshot = RuntimeReviewPressureSnapshot {
        source: "run_status_surfaces".to_string(),
        stale_worker_count: read_u64_field(&worker_visibility, "stale_worker_count"),
        timed_out_worker_count: read_u64_field(&worker_visibility, "timed_out_worker_count"),
        reclaim_needed_worker_count: read_u64_field(
            &worker_visibility,
            "reclaim_needed_worker_count",
        ),
        active_run_count: 0,
        token_total: read_u64_field(&token_usage, "total_tokens"),
        token_soft_limit: runtime_config
            .get("review_token_soft_limit")
            .and_then(Value::as_u64)
            .filter(|limit| *limit > 0),
        cpu_available_parallelism: None,
        memory_total_kib: None,
        memory_available_kib: None,
        memory_available_percent: None,
        pressure_reason: None,
    };
    if let Some(os_snapshot) = collect_os_review_pressure_snapshot() {
        let snapshot = RuntimeReviewPressureSnapshot {
            source: "combined_run_and_os_snapshot".to_string(),
            cpu_available_parallelism: os_snapshot.cpu_available_parallelism,
            memory_total_kib: os_snapshot.memory_total_kib,
            memory_available_kib: os_snapshot.memory_available_kib,
            memory_available_percent: os_snapshot.memory_available_percent,
            ..snapshot
        };
        return Ok(snapshot.has_observed_pressure().then_some(snapshot));
    }
    Ok(snapshot.has_observed_pressure().then_some(snapshot))
}

fn create_recommendation_title(request: &str) -> String {
    let single_line = request.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = single_line.trim();
    if trimmed.chars().count() <= 88 {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(85).collect::<String>())
    }
}

struct NormalizedEntryRequest<'a> {
    raw: &'a str,
    lowercase: String,
    compact_lowercase: String,
}

impl<'a> NormalizedEntryRequest<'a> {
    fn new(raw: &'a str) -> Self {
        let lowercase = raw.to_ascii_lowercase();
        let compact_lowercase = lowercase.split_whitespace().collect::<Vec<_>>().join(" ");
        Self {
            raw,
            lowercase,
            compact_lowercase,
        }
    }
}

fn request_mentions_documented_completion(request: &NormalizedEntryRequest<'_>) -> bool {
    request.lowercase.contains("finish")
        || request.lowercase.contains("complete")
        || request.lowercase.contains("end to end")
        || request.lowercase.contains("until done")
        || request.lowercase.contains("release note")
        || request.lowercase.contains("docs/")
        || request.lowercase.contains(".md")
        || request.raw.contains("끝까지")
        || request.raw.contains("완료")
        || request.raw.contains("마무리")
        || request.raw.contains("문서")
        || request.raw.contains("릴리즈")
}

fn request_has_ambiguous_entry_signal(request: &NormalizedEntryRequest<'_>) -> bool {
    let file_path_mentions = request.raw.matches('.').count();
    file_path_mentions >= 3
        || [
            "ambiguous",
            "unclear",
            "not sure",
            "investigate",
            "analyze",
            "diagnose",
            "multi-step",
            "runtime mutation",
            "what remains",
            "plan the next",
            "next bounded step",
            "next step",
        ]
        .iter()
        .any(|signal| request.compact_lowercase.contains(signal))
}

fn request_has_broad_way_confirmation_signal(request: &NormalizedEntryRequest<'_>) -> bool {
    [
        "across",
        "across modules",
        "cross-module",
        "cross module",
        "repository-wide",
        "repo-wide",
        "strategy",
        "strategic",
        "multiple tasks",
        "multiple workstreams",
        "multi-part",
        "multi part",
        "several tasks",
        "전체",
        "다방면",
        "여러 작업",
        "복수 작업",
    ]
    .iter()
    .any(|signal| request.compact_lowercase.contains(signal))
}

fn request_mentions_release_install_mutation(
    request: &NormalizedEntryRequest<'_>,
    request_shape: &str,
) -> bool {
    if request_shape != "mutation" {
        return false;
    }
    let release_or_install = [
        "release",
        "install",
        "installer",
        "install.sh",
        "install.ps1",
        "release asset",
        "scripts/release",
    ]
    .iter()
    .any(|signal| request.lowercase.contains(signal));
    let mutation_signal = [
        "fix",
        "repair",
        "implement",
        "change",
        "update",
        "patch",
        "upload",
        "edit",
        "create",
        "delete",
    ]
    .iter()
    .any(|signal| request.lowercase.contains(signal));
    release_or_install && mutation_signal
}

fn create_completion_discipline_payload(
    request: &NormalizedEntryRequest<'_>,
    task_shape: &str,
) -> Value {
    let documented_completion = request_mentions_documented_completion(request);
    json!({
        "state": if documented_completion { "required" } else { "bounded" },
        "completion_mode": if documented_completion { "documented_completion_criteria" } else { "single_bounded_checkpoint" },
        "documented_completion_requested": documented_completion,
        "task_shape": task_shape,
        "summary": if documented_completion {
            "Captain must continue through the referenced document or checklist until acceptance is met, explicitly out of scope, or blocked on an operator decision; stopping after a partial slice is not complete."
        } else {
            "Captain may keep the first run to one bounded checkpoint, then continue only when the persisted next action requires it."
        },
        "captain_obligations": if documented_completion {
            json!([
                "derive concrete remaining items from the referenced document or checklist",
                "persist progress and next action after each slice",
                "continue slices until every in-scope item is completed, explicitly deferred, or blocked",
                "record validation or the exact blocker before answering"
            ])
        } else {
            json!([
                "persist one bounded run and visible next captain action",
                "avoid hidden local fallback when CCC owns the entry"
            ])
        },
    })
}

fn intent_confirmation_reason_codes(
    request: &NormalizedEntryRequest<'_>,
    request_shape: &str,
    task_shape: &str,
    risk: &str,
    tool_operation: &str,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if request_mentions_release_install_mutation(request, request_shape) {
        reasons.push("release_install_mutation");
    }
    if request_shape == "mutation" && task_shape == "multi_step_or_unclear" {
        reasons.push("multi_step_runtime_mutation");
    }
    if request_shape == "way"
        && task_shape == "multi_step_or_unclear"
        && request_has_broad_way_confirmation_signal(request)
    {
        reasons.push("broad_way_request");
    }
    if request_shape == "way" && request_has_ambiguous_entry_signal(request) {
        reasons.push("ambiguous_way_request");
    }
    if risk == "high" {
        reasons.push("high_risk_request");
    }
    if tool_operation == "mutation" {
        reasons.push("companion_tool_mutation");
    }
    reasons
}

fn create_intent_confirmation_payload(
    request: &NormalizedEntryRequest<'_>,
    request_shape: &str,
    task_shape: &str,
    mutation_intent: &str,
    recommended_entrypoint: &str,
    recommended_task_kind: &str,
    risk: &str,
    direct_allowed: bool,
    tool_route: &Value,
) -> Value {
    let tool_operation = tool_route
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let reason_codes =
        intent_confirmation_reason_codes(request, request_shape, task_shape, risk, tool_operation);
    let required = !direct_allowed && !reason_codes.is_empty();
    let next_action = if required {
        "await_operator"
    } else {
        "proceed"
    };
    let interpretation = json!({
        "request_title": create_recommendation_title(request.raw),
        "request_shape": request_shape,
        "task_shape": task_shape,
        "mutation_intent": mutation_intent,
        "recommended_entrypoint": recommended_entrypoint,
        "recommended_task_kind": recommended_task_kind,
        "risk": risk,
        "companion_tool_operation": tool_operation,
        "companion_tool_route_class": tool_route.get("route_class").cloned().unwrap_or(Value::String("none".to_string())),
    });
    let prompt = if required {
        Value::String(
            "Confirm this interpretation before CCC creates a Way/run, or rephrase the request with the intended scope."
                .to_string(),
        )
    } else {
        Value::Null
    };

    json!({
        "state": if required { "required" } else { "not_required" },
        "required": required,
        "next_action": next_action,
        "confirmation_kind": if required { "intent_interpretation" } else { "none" },
        "awaiting": if required { "operator_confirmation" } else { "none" },
        "reason_codes": reason_codes,
        "interpretation": interpretation,
        "clarification_policy": {
            "question_count": if required { "1-3" } else { "0" },
            "required_for": [
                "broad work",
                "risky work",
                "ambiguous scope",
                "irreversible actions"
            ],
            "narrow_work_default": "proceed_with_explicit_assumptions"
        },
        "prompt": prompt,
        "summary": if required {
            "CCC must await operator confirmation of the interpreted intent before Way planning or normal entry."
        } else if direct_allowed {
            "No intent confirmation is required for this bounded read-only request."
        } else {
            "No blocking intent confirmation is required by the current bounded policy."
        },
    })
}

pub(crate) fn create_ccc_recommend_entry_payload_for_policy(
    parsed: &Value,
    policy_mode_override: Option<&str>,
) -> Value {
    let request = parsed
        .get("request")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let normalized_request = NormalizedEntryRequest::new(request);
    let cwd = parsed.get("cwd").cloned().unwrap_or(Value::Null);
    let policy_mode = policy_mode_override
        .map(|value| normalize_entry_policy_mode(Some(value)))
        .unwrap_or_else(load_entry_policy_mode);
    let request_shape = infer_request_shape(request);
    let mutation_intent = infer_mutation_intent(request_shape);
    let task_shape = infer_task_shape(request, request_shape);
    let completion_discipline =
        create_completion_discipline_payload(&normalized_request, task_shape);
    let payload_runtime_pressure = runtime_review_pressure_snapshot_from_value(
        parsed.get("runtime_pressure"),
        "request_payload",
    );
    let review_policy = create_review_policy_payload(
        request,
        request_shape,
        task_shape,
        None,
        payload_runtime_pressure.as_ref(),
    );
    let recommended_entrypoint = infer_recommended_entrypoint(request_shape);
    let recommended_task_kind = infer_recommended_task_kind(request_shape);
    let tool_route = create_companion_tool_route_payload(request, mutation_intent);
    let documented_completion_requested = completion_discipline
        .get("documented_completion_requested")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_allowed = matches!(request_shape, "lookup" | "diagnostic")
        && task_shape == "single_scoped_task"
        && !documented_completion_requested;
    let requires_user_confirmation = !direct_allowed;
    let recommended_action = if direct_allowed {
        "direct_read_only"
    } else {
        "enter_ccc_control_plane"
    };
    let entry_guard_reason = create_entry_guard_reason(
        request_shape,
        task_shape,
        documented_completion_requested,
        direct_allowed,
    );
    let confidence = if task_shape == "multi_step_or_unclear" {
        "high"
    } else {
        "medium"
    };
    let automatic_entry_supported = matches!(
        policy_mode.as_str(),
        "ccc_first_bounded" | "codex_cli_ccc_first"
    );
    let entry_boundary = create_entry_boundary_payload(&policy_mode);
    let orchestrator_role_config = load_role_config_snapshot("orchestrator");
    let risk = review_policy
        .get("risk")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let intent_confirmation = create_intent_confirmation_payload(
        &normalized_request,
        request_shape,
        task_shape,
        mutation_intent,
        recommended_entrypoint,
        recommended_task_kind,
        risk,
        direct_allowed,
        &tool_route,
    );
    let intent_confirmation_required = intent_confirmation
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rationale = match request_shape {
        "mutation" => vec![
            "The request contains explicit mutation signals.".to_string(),
            "Captain should still enter through a bounded LongWay contract before specialist execution.".to_string(),
        ],
        "way" => vec![
            "The request contains Way or multi-step investigation signals.".to_string(),
            "Captain should keep the first move on a bounded Way route.".to_string(),
        ],
        "review" => vec![
            "The request reads like a review or verification pass.".to_string(),
            "A bounded read-only or review-scoped run is safer than direct execution.".to_string(),
        ],
        _ => vec![
            "The request is lightweight and can stay on a bounded read-only start path.".to_string(),
        ],
    };
    let summary = if recommended_entrypoint == "way" {
        "Recommend `Way` because captain should generate a bounded LongWay before specialist execution."
    } else {
        "Recommend `start` because the request can stay on a single bounded read-only or review-scoped run."
    };

    json!({
        "cwd": cwd,
        "request": request,
        "policy_mode": policy_mode,
        "policy_summary": create_entry_policy_summary(&policy_mode),
        "automatic_entry_supported": automatic_entry_supported,
        "entry_boundary": entry_boundary.get("entry_boundary").cloned().unwrap_or(Value::Null),
        "entry_boundary_summary": entry_boundary.get("entry_boundary_summary").cloned().unwrap_or(Value::Null),
        "upstream_codex_binary_intercept_supported": false,
        "upstream_codex_binary_intercept_summary": "Hidden upstream Codex CLI binary interception is not a supported CCC entry boundary.",
        "orchestrator_scope": "bounded_synthesis_decision_and_state_supervision",
        "orchestrator_scope_summary": "Captain stays responsible for Way and LongWay supervision, bounded routing, persisted state supervision, and operator-facing visibility.",
        "orchestrator_agent": {
            "role": "orchestrator",
            "roster_name": "captain",
            "profile": orchestrator_role_config.get("profile").cloned().unwrap_or(Value::Null),
            "model": orchestrator_role_config.get("model").cloned().unwrap_or(Value::Null),
            "variant": orchestrator_role_config.get("variant").cloned().unwrap_or(Value::Null),
            "config_entries": orchestrator_role_config.get("config_entries").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        },
        "orchestrator_request_settings_preview": {
            "source": "shared_role_config",
            "profile": orchestrator_role_config.get("profile").cloned().unwrap_or(Value::Null),
            "model": orchestrator_role_config.get("model").cloned().unwrap_or(Value::Null),
            "variant": orchestrator_role_config.get("variant").cloned().unwrap_or(Value::Null),
            "config_entries": orchestrator_role_config.get("config_entries").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        },
        "recommended_entrypoint": recommended_entrypoint,
        "task_shape": task_shape,
        "request_shape": request_shape,
        "mutation_intent": mutation_intent,
        "recommended_task_kind": recommended_task_kind,
        "recommended_action": recommended_action,
        "direct_allowed": direct_allowed,
        "requires_user_confirmation": requires_user_confirmation,
        "active_run_summary": create_entry_guard_active_run_summary(),
        "risk": review_policy.get("risk").cloned().unwrap_or(Value::Null),
        "reason": entry_guard_reason,
        "next_action": intent_confirmation.get("next_action").cloned().unwrap_or(Value::String("proceed".to_string())),
        "operator_confirmation_required": intent_confirmation_required,
        "confirmation_prompt": intent_confirmation.get("prompt").cloned().unwrap_or(Value::Null),
        "intent_confirmation": intent_confirmation,
        "completion_discipline": completion_discipline,
        "review_policy": review_policy,
        "confidence": confidence,
        "summary": summary,
        "rationale": rationale,
        "suggested_cli_command": "ccc start",
        "suggested_mcp_tool": if recommended_entrypoint == "way" { "ccc_start" } else { "ccc_start" },
        "workflow_variant_selection": Value::Null,
        "companion_tool_route_class": tool_route.get("route_class").cloned().unwrap_or(Value::String("none".to_string())),
        "companion_tool_names": tool_route.get("tool_names").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "companion_tool_operation": tool_route.get("operation").cloned().unwrap_or(Value::String("none".to_string())),
        "companion_tool_owner_role": tool_route.get("owner_role").cloned().unwrap_or(Value::Null),
        "companion_tool_model": tool_route.get("model").cloned().unwrap_or(Value::Null),
        "companion_tool_variant": tool_route.get("variant").cloned().unwrap_or(Value::Null),
        "companion_tool_fallback_mode": tool_route.get("fallback_mode").cloned().unwrap_or(Value::String("visible_degraded_host_fallback".to_string())),
        "companion_tool_execution_state": tool_route.get("execution_state").cloned().unwrap_or(Value::String("not_applicable".to_string())),
    })
}

pub(crate) fn create_ccc_recommend_entry_payload(parsed: &Value) -> Value {
    create_ccc_recommend_entry_payload_for_policy(parsed, None)
}

pub(crate) fn create_ccc_recommend_entry_text(payload: &Value) -> String {
    let mut text = format!(
        "CCC entry recommendation: action={} direct_allowed={} next_action={} risk={} confidence={}",
        payload
            .get("recommended_action")
            .and_then(Value::as_str)
            .unwrap_or("enter_ccc_control_plane"),
        payload
            .get("direct_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        payload
            .get("next_action")
            .and_then(Value::as_str)
            .unwrap_or("proceed"),
        payload
            .get("risk")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        payload
            .get("confidence")
            .and_then(Value::as_str)
            .unwrap_or("medium"),
    );
    if payload
        .get("operator_confirmation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if let Some(prompt) = payload.get("confirmation_prompt").and_then(Value::as_str) {
            text.push_str(" prompt=\"");
            text.push_str(prompt);
            text.push('"');
        }
    }
    text
}

fn create_auto_entry_start_request(parsed: &Value, recommendation: &Value) -> Value {
    let request = parsed
        .get("request")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let request_shape = recommendation
        .get("request_shape")
        .and_then(Value::as_str)
        .unwrap_or("way");
    let recommended_task_kind = recommendation
        .get("recommended_task_kind")
        .and_then(Value::as_str)
        .unwrap_or("way");
    let recommended_entrypoint = recommendation
        .get("recommended_entrypoint")
        .and_then(Value::as_str)
        .unwrap_or("way");
    let completion_discipline = recommendation
        .get("completion_discipline")
        .cloned()
        .unwrap_or(Value::Null);
    let documented_completion = completion_discipline
        .get("documented_completion_requested")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let title = create_recommendation_title(request);
    let intent = match request_shape {
        "mutation" => "Create a bounded captain-first Way before mutation execution.",
        "review" => "Create a bounded review-scoped run before verification.",
        "lookup" => "Create a bounded read-only run for the operator request.",
        _ => "Create a bounded captain-first Way run for the operator request.",
    };
    let scope = if documented_completion {
        "Continue through the referenced document or checklist in bounded slices until every in-scope completion criterion is met, explicitly deferred, or blocked on an operator decision."
    } else {
        match request_shape {
            "mutation" => "Keep the first run limited to one Way checkpoint and the smallest next implementation slice.",
            "review" => "Keep the run limited to one review or verification checkpoint.",
            "lookup" => "Keep the run read-only and bounded to one visible checkpoint.",
            _ => "Keep the run bounded to one Way checkpoint and the smallest next specialist handoff.",
        }
    };
    let acceptance = if documented_completion {
        "Done only when all referenced document/checklist items are completed with validation recorded, explicitly out of scope, or blocked with the exact operator decision needed; do not report success after a partial slice."
    } else {
        "Persist an honest bounded run with one active task-card, a visible next captain action, and no hidden reuse or hydration wait."
    };
    let assigned_role = recommendation
        .get("companion_tool_owner_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|_| recommended_entrypoint == "start");

    json!({
        "cwd": parsed.get("cwd").cloned().unwrap_or(Value::Null),
        "goal": request,
        "title": title,
        "intent": intent,
        "scope": scope,
        "acceptance": acceptance,
        "prompt": request,
        "task_kind": recommended_task_kind,
        "assigned_role": assigned_role,
        "codex_bin": parsed.get("codex_bin").cloned().unwrap_or(Value::Null),
        "review_policy": recommendation.get("review_policy").cloned().unwrap_or(Value::Null),
        "completion_discipline": completion_discipline,
    })
}

pub(crate) fn create_ccc_auto_entry_payload_for_policy(
    session_context: &SessionContext,
    parsed: &Value,
    policy_mode_override: Option<&str>,
) -> io::Result<Value> {
    let recommendation =
        create_ccc_recommend_entry_payload_for_policy(parsed, policy_mode_override);
    let automatic_entry_supported = recommendation
        .get("automatic_entry_supported")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cwd = resolve_workspace_path(parsed.get("cwd").and_then(Value::as_str))?;

    if !automatic_entry_supported {
        return Ok(json!({
            "cwd": cwd.to_string_lossy(),
            "request": parsed.get("request").cloned().unwrap_or(Value::Null),
            "policy_mode": recommendation.get("policy_mode").cloned().unwrap_or(Value::Null),
            "automatic_entry_supported": false,
            "entry_boundary": recommendation.get("entry_boundary").cloned().unwrap_or(Value::Null),
            "entry_boundary_summary": recommendation.get("entry_boundary_summary").cloned().unwrap_or(Value::Null),
            "upstream_codex_binary_intercept_supported": false,
            "created": false,
            "run_selection": "explicit_entry_required",
            "active_run_scan_state": "skipped_for_rust_deterministic_path",
            "active_run_scan_summary": "Rust auto-entry intentionally skipped active-run reuse and requires explicit entry under the current policy.",
            "inspected_active_run_count": 0,
            "fresh_active_run_count": 0,
            "stale_active_run_count": 0,
            "entrypoint_used": Value::Null,
            "scoping_source": "recommendation_only",
            "run_decision_reason": "policy_requires_explicit_entry",
            "summary": "Rust auto-entry returned a deterministic recommendation only because the current policy does not allow automatic run creation.",
            "review_policy": recommendation.get("review_policy").cloned().unwrap_or(Value::Null),
            "completion_discipline": recommendation.get("completion_discipline").cloned().unwrap_or(Value::Null),
            "answer_trace": {
                "request_shape": recommendation.get("request_shape").cloned().unwrap_or(Value::Null),
                "mutation_intent": recommendation.get("mutation_intent").cloned().unwrap_or(Value::Null),
                "selected_role": "captain",
                "execution_path": "recommendation_only",
                "budget_class": "way_budget",
                "review_requirement": recommendation.pointer("/review_policy/decision").cloned().unwrap_or(Value::String("not_applicable".to_string())),
                "completion_mode": recommendation.pointer("/completion_discipline/completion_mode").cloned().unwrap_or(Value::Null),
            },
            "recommendation": recommendation,
            "server_identity": create_server_identity_payload(session_context),
        }));
    }

    if recommendation
        .get("operator_confirmation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(json!({
            "cwd": cwd.to_string_lossy(),
            "request": parsed.get("request").cloned().unwrap_or(Value::Null),
            "policy_mode": recommendation.get("policy_mode").cloned().unwrap_or(Value::Null),
            "automatic_entry_supported": true,
            "entry_boundary": recommendation.get("entry_boundary").cloned().unwrap_or(Value::Null),
            "entry_boundary_summary": recommendation.get("entry_boundary_summary").cloned().unwrap_or(Value::Null),
            "upstream_codex_binary_intercept_supported": false,
            "created": false,
            "run_selection": "operator_confirmation_required",
            "active_run_scan_state": "skipped_pending_operator_confirmation",
            "active_run_scan_summary": "Rust auto-entry did not scan or create a run because intent confirmation is required before Way planning or normal entry.",
            "inspected_active_run_count": 0,
            "fresh_active_run_count": 0,
            "stale_active_run_count": 0,
            "entrypoint_used": Value::Null,
            "scoping_source": "recommendation_only",
            "run_decision_reason": "operator_confirmation_required",
            "next_action": recommendation.get("next_action").cloned().unwrap_or(Value::String("await_operator".to_string())),
            "operator_confirmation_required": true,
            "intent_confirmation": recommendation.get("intent_confirmation").cloned().unwrap_or(Value::Null),
            "summary": "Rust auto-entry returned a confirmation requirement without creating a run.",
            "review_policy": recommendation.get("review_policy").cloned().unwrap_or(Value::Null),
            "completion_discipline": recommendation.get("completion_discipline").cloned().unwrap_or(Value::Null),
            "answer_trace": {
                "request_shape": recommendation.get("request_shape").cloned().unwrap_or(Value::Null),
                "mutation_intent": recommendation.get("mutation_intent").cloned().unwrap_or(Value::Null),
                "selected_role": "captain",
                "execution_path": "await_operator_confirmation",
                "budget_class": "confirmation_gate",
                "review_requirement": recommendation.pointer("/review_policy/decision").cloned().unwrap_or(Value::String("not_applicable".to_string())),
                "completion_mode": recommendation.pointer("/completion_discipline/completion_mode").cloned().unwrap_or(Value::Null),
            },
            "recommendation": recommendation,
            "server_identity": create_server_identity_payload(session_context),
        }));
    }

    let start_request = create_auto_entry_start_request(parsed, &recommendation);
    let start_payload = create_ccc_start_payload(&start_request)?;
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": start_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_status",
    )?;
    let status_payload = create_ccc_status_payload(session_context, &locator)?;
    let request_shape = recommendation
        .get("request_shape")
        .and_then(Value::as_str)
        .unwrap_or("way");

    Ok(json!({
        "cwd": cwd.to_string_lossy(),
        "request": parsed.get("request").cloned().unwrap_or(Value::Null),
        "policy_mode": recommendation.get("policy_mode").cloned().unwrap_or(Value::Null),
        "automatic_entry_supported": true,
        "entry_boundary": recommendation.get("entry_boundary").cloned().unwrap_or(Value::Null),
        "entry_boundary_summary": recommendation.get("entry_boundary_summary").cloned().unwrap_or(Value::Null),
        "upstream_codex_binary_intercept_supported": false,
        "created": true,
        "run_selection": start_payload.get("run_selection").cloned().unwrap_or(Value::String("new_run_created".to_string())),
        "active_run_scan_state": start_payload.pointer("/active_run_scan/active_run_scan_state").cloned().unwrap_or(Value::Null),
        "active_run_scan_summary": start_payload.pointer("/active_run_scan/active_run_scan_summary").cloned().unwrap_or(Value::Null),
        "inspected_active_run_count": start_payload.pointer("/active_run_scan/inspected_active_run_count").cloned().unwrap_or(Value::Number(0.into())),
        "fresh_active_run_count": start_payload.pointer("/active_run_scan/fresh_active_run_count").cloned().unwrap_or(Value::Number(0.into())),
        "stale_active_run_count": start_payload.pointer("/active_run_scan/stale_active_run_count").cloned().unwrap_or(Value::Number(0.into())),
        "active_run_scan": start_payload.get("active_run_scan").cloned().unwrap_or(Value::Null),
        "entrypoint_used": recommendation.get("recommended_entrypoint").cloned().unwrap_or(Value::String("way".to_string())),
        "scoping_source": "rust_deterministic_auto_entry",
        "run_id": start_payload.get("run_id").cloned().unwrap_or(Value::Null),
        "task_card_id": start_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
        "run_directory": start_payload.get("run_directory").cloned().unwrap_or(Value::Null),
        "run_ref": start_payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "status": status_payload.get("status").cloned().unwrap_or(Value::Null),
        "stage": status_payload.get("stage").cloned().unwrap_or(Value::Null),
        "next_step": start_payload.get("next_step").cloned().unwrap_or(Value::Null),
        "can_advance": start_payload.get("can_advance").cloned().unwrap_or(Value::Null),
        "allowed_next_commands": start_payload.get("allowed_next_commands").cloned().unwrap_or(Value::Null),
        "run_decision_reason": if start_payload.pointer("/active_run_scan/fresh_active_run_count").and_then(Value::as_u64).unwrap_or(0) > 0 { "fresh_run_created_with_active_prior_run_visibility" } else { "fresh_deterministic_rust_entry" },
        "summary": if start_payload.pointer("/active_run_scan/fresh_active_run_count").and_then(Value::as_u64).unwrap_or(0) > 0 { "Rust auto-entry created a fresh bounded run while surfacing active prior-run continuity guidance for captain merge/replan/reclaim handling." } else { "Rust auto-entry created a fresh bounded run on a deterministic captain-first path." },
        "review_policy": status_payload.get("review_policy").cloned().unwrap_or_else(|| recommendation.get("review_policy").cloned().unwrap_or(Value::Null)),
        "completion_discipline": status_payload
            .get("current_task_card")
            .and_then(|value| value.get("completion_discipline"))
            .cloned()
            .unwrap_or_else(|| recommendation.get("completion_discipline").cloned().unwrap_or(Value::Null)),
        "answer_trace": {
            "request_shape": request_shape,
            "mutation_intent": recommendation.get("mutation_intent").cloned().unwrap_or(Value::Null),
            "selected_role": status_payload
                .get("current_task_card")
                .and_then(|value| value.get("assigned_agent_id"))
                .cloned()
                .or_else(|| status_payload.get("active_agent_id").cloned())
                .unwrap_or(Value::Null),
            "execution_path": "new_run",
            "budget_class": if request_shape == "mutation" { Value::String("implementation_budget".to_string()) } else { Value::String("way_budget".to_string()) },
            "review_requirement": recommendation.pointer("/review_policy/decision").cloned().unwrap_or_else(|| if request_shape == "review" { Value::String("required".to_string()) } else { Value::String("optional".to_string()) }),
            "completion_mode": recommendation.pointer("/completion_discipline/completion_mode").cloned().unwrap_or(Value::Null),
            "why_selected": "Rust auto-entry keeps the first move bounded and persisted instead of answering through captain-local fallback.",
            "why_not_local": "This request was routed into a persisted run instead of a no-run local answer.",
            "why_not_heavier_role": "Heavier reviewed paths wait until a bounded specialist result exists.",
        },
        "recommendation": recommendation,
        "current_task_card": status_payload.get("current_task_card").cloned().unwrap_or(Value::Null),
        "run_state": status_payload.get("run_state").cloned().unwrap_or(Value::Null),
        "server_identity": create_server_identity_payload(session_context),
    }))
}

pub(crate) fn create_ccc_auto_entry_payload(
    session_context: &SessionContext,
    parsed: &Value,
) -> io::Result<Value> {
    create_ccc_auto_entry_payload_for_policy(session_context, parsed, None)
}

pub(crate) fn create_ccc_auto_entry_text(payload: &Value) -> String {
    if payload.get("created").and_then(Value::as_bool) == Some(true) {
        format!(
            "Rust CCC auto-entry created run {} through {}",
            payload
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown-run"),
            payload
                .get("entrypoint_used")
                .and_then(Value::as_str)
                .unwrap_or("way"),
        )
    } else {
        "Rust CCC auto-entry returned a recommendation without creating a run.".to_string()
    }
}
