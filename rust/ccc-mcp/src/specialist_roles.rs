use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::execution_contract::create_execution_contract_for_role;
use crate::skill_registry::load_skill_registry_for_agent;
use crate::{
    load_runtime_config, read_optional_shared_config_document,
    resolve_custom_agent_install_directory, sanitize_value_for_toml, write_string_atomic,
};

const GENERATED_CUSTOM_AGENT_FILE_PREFIX: &str = "ccc-";
pub(crate) const GENERATED_CUSTOM_AGENT_NAME_PREFIX: &str = "ccc_";
pub(crate) const DEFAULT_COMMIT_MESSAGE_GUIDANCE: &str =
    "Commit message guidance: for commit-related delegated work, if the operator did not provide commit message, style, or language instructions, use the default Conventional Commit-style fallback `fix(hub, worker): 비전 기본 가중치를 metric 0.4 text 0.6으로 조정`. If the operator supplies a commit message/style/language instruction, that instruction wins.";
pub(crate) const SUBAGENT_FALLBACK_REASON_CODES: &[&str] = &[
    "subagent_spawn_unavailable",
    "approval_blocked",
    "sandbox_incompatible",
    "config_sync_stale",
    "host_subagent_thread_limit",
    "child_timeout",
    "parent_override_conflict",
];

#[derive(Clone, Debug)]
struct ManagedCustomAgentSpec {
    role: String,
    agent_id: String,
    generated_name: String,
    file_name: String,
    content: String,
}

pub(crate) fn assigned_role_for_task_kind(task_kind: &str) -> &'static str {
    match task_kind {
        "way" => "way",
        "explore" => "explorer",
        "review" => "verifier",
        _ => "code specialist",
    }
}

fn is_captain_or_orchestrator_role(role: &str) -> bool {
    matches!(role.trim(), "captain" | "orchestrator")
}

fn is_captain_or_orchestrator_agent_id(agent_id: &str) -> bool {
    matches!(agent_id.trim(), "captain")
        || role_for_agent_id(agent_id.trim()) == Some("orchestrator")
}

fn task_kind_follow_up_fallback_role(task_card: &Value) -> &'static str {
    let task_kind = task_card
        .get("task_kind")
        .and_then(Value::as_str)
        .unwrap_or("execution");
    let role = assigned_role_for_task_kind(task_kind);
    if is_captain_or_orchestrator_role(role) {
        "code specialist"
    } else {
        role
    }
}

pub(crate) fn resolve_follow_up_specialist_role(
    task_card: &Value,
    role_hint: Option<&str>,
) -> String {
    let original_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|role| !is_captain_or_orchestrator_role(role));
    let fallback_role =
        original_role.unwrap_or_else(|| task_kind_follow_up_fallback_role(task_card));
    if role_hint
        .map(str::trim)
        .is_some_and(is_captain_or_orchestrator_role)
    {
        return task_kind_follow_up_fallback_role(task_card).to_string();
    }
    let selected_role = role_hint
        .map(|hint| normalize_dispatch_role_hint(Some(hint), fallback_role))
        .unwrap_or_else(|| fallback_role.to_string());
    if is_captain_or_orchestrator_role(&selected_role) {
        task_kind_follow_up_fallback_role(task_card).to_string()
    } else {
        selected_role
    }
}

pub(crate) fn resolve_follow_up_specialist_assignment(
    task_card: &Value,
    role_hint: Option<&str>,
    agent_id_hint: Option<&str>,
) -> (String, String) {
    let assigned_role = resolve_follow_up_specialist_role(task_card, role_hint);
    let assigned_agent_id = agent_id_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|agent_id| !is_captain_or_orchestrator_agent_id(agent_id))
        .filter(|agent_id| role_for_agent_id(agent_id) == Some(assigned_role.as_str()))
        .map(str::to_string)
        .or_else(|| agent_id_for_role(&assigned_role).map(str::to_string))
        .unwrap_or_else(|| "raider".to_string());
    (assigned_role, assigned_agent_id)
}

fn task_kind_for_role(role: &str) -> &'static str {
    match role {
        "way" => "way",
        "explorer" | "companion_reader" => "explore",
        "verifier" => "review",
        _ => "execution",
    }
}

pub(crate) fn agent_id_for_role(role: &str) -> Option<&'static str> {
    match role {
        "orchestrator" => Some("captain"),
        "way" => Some("tactician"),
        "explorer" => Some("scout"),
        "code specialist" => Some("raider"),
        "documenter" => Some("scribe"),
        "verifier" => Some("arbiter"),
        "sentinel" => Some("sentinel"),
        "companion_reader" => Some("companion_reader"),
        "companion_operator" => Some("companion_operator"),
        _ => None,
    }
}

fn spawnable_custom_agent_id_for_role(role: &str) -> Option<&'static str> {
    if is_captain_or_orchestrator_role(role) {
        None
    } else {
        agent_id_for_role(role)
    }
}

pub(crate) fn phase_name_for_role(role: &str) -> &'static str {
    match role {
        "way" => "way",
        "explorer" | "companion_reader" => "inspect",
        "documenter" => "document",
        "verifier" => "verify",
        "sentinel" => "ownership_check",
        "orchestrator" => "fan_in",
        _ => "mutate",
    }
}

pub(crate) fn expertise_phrase_for_role_and_task_shape(
    role: &str,
    task_shape: &str,
) -> &'static str {
    match role {
        "way" => {
            if task_shape == "multi_step_or_unclear" {
                "You are an expert in bounded planning, tradeoff framing, and step decomposition."
            } else {
                "You are an expert in bounded planning and next-step clarity."
            }
        }
        "explorer" => {
            "You are an expert in repository investigation, evidence gathering, and concise source-backed reporting."
        }
        "documenter" => {
            "You are an expert in release-note documentation and operator-facing clarity."
        }
        "verifier" => {
            "You are an expert in code review, regression detection, and acceptance risk."
        }
        "sentinel" => {
            "You are an expert in ownership classification, route fit, and execution-boundary risk."
        }
        "companion_reader" => {
            "You are an expert in bounded operator evidence gathering and source-focused reading."
        }
        "companion_operator" => {
            "You are an expert in bounded operator tool-operation and command-result clarity."
        }
        _ => {
            "You are an expert in bounded implementation, repair, module ownership, and focused validation."
        }
    }
}

pub(crate) fn task_stance_for_role_and_task_shape(role: &str, task_shape: &str) -> &'static str {
    match role {
        "way" => {
            if task_shape == "multi_step_or_unclear" {
                "bounded_planning"
            } else {
                "next_step_planning"
            }
        }
        "explorer" => "read_only_investigation",
        "documenter" => "documentation_update",
        "verifier" => "findings_first_review",
        "sentinel" => "ownership_classification",
        "companion_reader" => "bounded_operator_reader",
        "companion_operator" => "bounded_operator_execution",
        _ => {
            if task_shape == "multi_step_or_unclear" {
                "scoped_implementation_with_boundary_control"
            } else {
                "bounded_implementation"
            }
        }
    }
}

pub(crate) fn expected_thinking_mode_for_role(role: &str) -> &'static str {
    match role {
        "way" => "compare-options-then-select-next-bound",
        "explorer" => "evidence-first-read-only",
        "documenter" => "style-and-audience-first",
        "verifier" => "findings-first-risk-review",
        "sentinel" => "classification-first-boundary-review",
        "companion_reader" => "bounded-reader-evidence-first",
        "companion_operator" => "command-boundary-first",
        _ => "smallest-defensible-change",
    }
}

pub(crate) fn task_expertise_framing_for_role(role: &str, task_shape: &str) -> Value {
    json!({
        "expertise_phrase": expertise_phrase_for_role_and_task_shape(role, task_shape),
        "task_stance": task_stance_for_role_and_task_shape(role, task_shape),
        "expected_thinking_mode": expected_thinking_mode_for_role(role),
        "task_shape": task_shape,
    })
}

pub(crate) fn apply_task_expertise_framing(task_card: &mut Value, role: &str, task_shape: &str) {
    let expertise_framing = task_expertise_framing_for_role(role, task_shape);
    let Some(task_card_object) = task_card.as_object_mut() else {
        return;
    };
    task_card_object.insert("expertise_framing".to_string(), expertise_framing.clone());
    if let Some(delegation_plan) = task_card_object
        .get_mut("delegation_plan")
        .and_then(Value::as_object_mut)
    {
        delegation_plan.insert("expertise_framing".to_string(), expertise_framing.clone());
        if let Some(runtime_dispatch) = delegation_plan
            .get_mut("runtime_dispatch")
            .and_then(Value::as_object_mut)
        {
            runtime_dispatch.insert("expertise_framing".to_string(), expertise_framing.clone());
        }
        if let Some(spawn_contract) = delegation_plan
            .get_mut("subagent_spawn_contract")
            .and_then(Value::as_object_mut)
        {
            spawn_contract.insert(
                "expertise_phrase".to_string(),
                expertise_framing
                    .get("expertise_phrase")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            spawn_contract.insert(
                "task_stance".to_string(),
                expertise_framing
                    .get("task_stance")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            spawn_contract.insert(
                "expected_thinking_mode".to_string(),
                expertise_framing
                    .get("expected_thinking_mode")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
    }
}

pub(crate) fn normalize_dispatch_role_hint(hint: Option<&str>, fallback_role: &str) -> String {
    let normalized = hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    match normalized.as_deref() {
        Some("way") | Some("plan") | Some("tactician") => "way".to_string(),
        Some("explore") | Some("explorer") | Some("inspect") | Some("scout") => {
            "explorer".to_string()
        }
        Some("companion_reader") | Some("companion-reader") | Some("reader") => {
            "companion_reader".to_string()
        }
        Some("companion_operator") | Some("companion-operator") | Some("operator") => {
            "companion_operator".to_string()
        }
        Some("review") | Some("verify") | Some("verifier") | Some("arbiter") => {
            "verifier".to_string()
        }
        Some("document") | Some("docs") | Some("documenter") | Some("scribe") => {
            "documenter".to_string()
        }
        Some("ownership") | Some("sentinel") => "sentinel".to_string(),
        Some("execute")
        | Some("execution")
        | Some("mutate")
        | Some("implement")
        | Some("repair")
        | Some("retry")
        | Some("raider")
        | Some("code")
        | Some("code specialist") => "code specialist".to_string(),
        _ => fallback_role.to_string(),
    }
}

pub(crate) fn build_task_card_payload_with_role(
    run_id: &str,
    task_card_id: &str,
    title: &str,
    intent: &str,
    scope: &str,
    execution_prompt: &str,
    acceptance: &str,
    assigned_role: &str,
    timestamp: &str,
) -> Value {
    let role_config_snapshot = load_role_config_snapshot(assigned_role);
    let model_tier_intent = role_model_tier_intent(&role_config_snapshot);
    let sandbox_mode = sandbox_mode_for_role(assigned_role);
    let sandbox_rationale = sandbox_rationale_for_role(assigned_role);
    let expertise_framing = task_expertise_framing_for_role(assigned_role, "single_scoped_task");
    let delegation_plan = create_specialist_delegation_plan(
        assigned_role,
        &role_config_snapshot,
        sandbox_mode,
        sandbox_rationale,
    );
    json!({
        "task_card_id": task_card_id,
        "run_id": run_id,
        "title": title,
        "intent": intent,
        "scope": scope,
        "execution_prompt": execution_prompt,
        "way_attempt_id": Value::Null,
        "workflow_skill_id": Value::Null,
        "workflow_step_index": Value::Null,
        "workflow_step_skill_id": Value::Null,
        "workflow_next_step_skill_id": Value::Null,
        "task_kind": task_kind_for_role(assigned_role),
        "acceptance_checks": [],
        "review_of_task_card_ids": [],
        "depends_on_task_card_ids": [],
        "fan_in_from_task_card_ids": [],
        "node_kind": "execution",
        "status": "active",
        "owner_role": "orchestrator",
        "assigned_role": assigned_role,
        "assigned_agent_id": agent_id_for_role(assigned_role),
        "sandbox_mode": sandbox_mode,
        "sandbox_rationale": sandbox_rationale,
        "expertise_framing": expertise_framing,
        "role_config_snapshot": role_config_snapshot,
        "delegation_plan": delegation_plan,
        "parallel_fanout": Value::Null,
        "review_policy": Value::Null,
        "model_tier_intent": model_tier_intent,
        "child_aggregation_contract": "explicit_fan_in_summary",
        "fan_in_barrier_semantics": "explicit_wait_for_all_sources",
        "orchestrator_review_gate": "after_child_completion",
        "acceptance": acceptance,
        "input_handoff_id": Value::Null,
        "output_handoff_id": Value::Null,
        "verification_state": "pending",
        "review_pass_count": 0,
        "latest_failure": Value::Null,
        "latest_model_launch": Value::Null,
        "ownership_chain": Value::Null,
        "thread_ids": [],
        "completed_by_agent_id": Value::Null,
        "created_at": timestamp,
        "updated_at": timestamp,
        "completed_at": Value::Null,
    })
}

pub(crate) fn load_shared_ccc_config() -> io::Result<Value> {
    Ok(read_optional_shared_config_document()?
        .map(|(_, value)| value)
        .unwrap_or(Value::Null))
}

pub(crate) fn sandbox_mode_for_role(role: &str) -> &'static str {
    match role {
        "way" | "explorer" | "verifier" | "sentinel" | "companion_reader" => "read-only",
        _ => "workspace-write",
    }
}

pub(crate) fn sandbox_rationale_for_role(role: &str) -> &'static str {
    match role {
        "way" => "Way creates or updates the LongWay through bounded read-only analysis before captain selects the next specialist.",
        "explorer" => "Scout work is evidence gathering and should not mutate the workspace.",
        "verifier" => "Verifier work should inspect and judge without mutating the workspace.",
        "sentinel" => "Sentinel work is classification-only and should stay read-only.",
        "companion_reader" => "Companion reader work is lightweight tool-routed evidence gathering and should stay read-only.",
        "companion_operator" => "Companion operator work is lightweight tool-routed mutation or operator-side execution and needs workspace-write.",
        "documenter" => "Scribe work may update docs or release text and needs workspace-write.",
        "code specialist" => "Raider work may change code or config and needs workspace-write.",
        _ => "Captain-directed mutation work needs workspace-write.",
    }
}

pub(crate) fn load_output_verbosity() -> String {
    load_shared_ccc_config()
        .ok()
        .and_then(|config| config.get("output").cloned())
        .and_then(|output| output.get("verbosity").cloned())
        .and_then(|value| value.as_str().map(str::to_string))
        .filter(|value| matches!(value.as_str(), "quiet" | "default" | "debug"))
        .unwrap_or_else(|| "default".to_string())
}

pub(crate) fn load_output_config() -> Value {
    let config = load_shared_ccc_config().unwrap_or(Value::Null);
    let output = config.get("output").cloned().unwrap_or(Value::Null);
    json!({
        "verbosity": output
            .get("verbosity")
            .and_then(Value::as_str)
            .filter(|value| matches!(*value, "quiet" | "default" | "debug"))
            .unwrap_or("default"),
        "changed_max_chars": output
            .get("changed_max_chars")
            .and_then(Value::as_i64)
            .filter(|value| *value > 0)
            .unwrap_or(160),
        "include_agent_loop_when_idle": output
            .get("include_agent_loop_when_idle")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn create_empty_role_config_snapshot(role: &str) -> Value {
    json!({
        "source": "shared_role_config",
        "role": role,
        "summary": Value::Null,
        "profile": Value::Null,
        "model": Value::Null,
        "variant": Value::Null,
        "fast_mode": false,
        "config_entries": [],
    })
}

fn create_default_companion_role_config_snapshot(role: &str) -> Value {
    json!({
        "source": "shared_role_config_default",
        "role": role,
        "summary": fallback_role_summary(role),
        "display_name": callsign_for_role(role).map(Value::from).unwrap_or(Value::Null),
        "callsign": callsign_for_role(role).map(Value::from).unwrap_or(Value::Null),
        "theme": "starcraft_display_callsign",
        "inspired_by": ["oh-my-openagent"],
        "profile": Value::Null,
        "model": "gpt-5.4-mini",
        "variant": "medium",
        "fast_mode": true,
        "config_entries": [],
        "recommended_workflows": [],
        "lsp_capabilities": [],
    })
}

fn create_default_sentinel_role_config_snapshot() -> Value {
    json!({
        "source": "shared_role_config_default",
        "role": "sentinel",
        "summary": fallback_role_summary("sentinel"),
        "display_name": callsign_for_role("sentinel").map(Value::from).unwrap_or(Value::Null),
        "callsign": callsign_for_role("sentinel").map(Value::from).unwrap_or(Value::Null),
        "theme": "starcraft_display_callsign",
        "inspired_by": ["oh-my-openagent"],
        "profile": Value::Null,
        "model": "gpt-5.4-mini",
        "variant": "high",
        "fast_mode": true,
        "config_entries": [],
        "recommended_workflows": [],
        "lsp_capabilities": [],
    })
}

fn config_role_lookup_keys(role: &str) -> Vec<&str> {
    match role {
        "way" => vec!["way", "planner", "plan"],
        "companion_reader" => vec!["companion_reader"],
        "companion_operator" => vec!["companion_operator"],
        _ => vec![role],
    }
}

pub(crate) fn load_role_config_snapshot_from_config(config: &Value, role: &str) -> Value {
    let agent_config = if matches!(role, "companion_reader" | "companion_operator") {
        config
            .get("companion_agents")
            .and_then(Value::as_object)
            .and_then(|agents| {
                config_role_lookup_keys(role)
                    .iter()
                    .find_map(|key| agents.get(*key))
            })
    } else {
        config
            .get("agents")
            .and_then(Value::as_object)
            .and_then(|agents| {
                config_role_lookup_keys(role)
                    .iter()
                    .find_map(|key| agents.get(*key))
            })
    };
    let Some(agent_config) = agent_config else {
        if matches!(role, "companion_reader" | "companion_operator") {
            return create_default_companion_role_config_snapshot(role);
        }
        if role == "sentinel" {
            return create_default_sentinel_role_config_snapshot();
        }
        return create_empty_role_config_snapshot(role);
    };

    json!({
        "source": "shared_role_config",
        "role": role,
        "summary": agent_config.get("summary").cloned().unwrap_or(Value::Null),
        "display_name": agent_config.get("display_name").cloned().unwrap_or_else(|| callsign_for_role(role).map(Value::from).unwrap_or(Value::Null)),
        "callsign": agent_config.get("callsign").cloned().unwrap_or_else(|| callsign_for_role(role).map(Value::from).unwrap_or(Value::Null)),
        "theme": agent_config.get("theme").cloned().unwrap_or(Value::Null),
        "inspired_by": agent_config.get("inspired_by").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "profile": agent_config.get("profile").cloned().unwrap_or(Value::Null),
        "model": agent_config.get("model").cloned().unwrap_or(Value::Null),
        "variant": agent_config.get("variant").cloned().unwrap_or(Value::Null),
        "fast_mode": agent_config.get("fast_mode").and_then(Value::as_bool).unwrap_or(false),
        "config_entries": agent_config.get("config_entries").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "recommended_workflows": agent_config.get("recommended_workflows").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "lsp_capabilities": agent_config.get("lsp_capabilities").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    })
}

pub(crate) fn load_role_config_snapshot(role: &str) -> Value {
    let config = match load_shared_ccc_config() {
        Ok(config) => config,
        Err(_) => return create_empty_role_config_snapshot(role),
    };
    load_role_config_snapshot_from_config(&config, role)
}

pub(crate) fn role_for_agent_id(agent_id: &str) -> Option<&'static str> {
    match agent_id.trim() {
        "captain" => Some("orchestrator"),
        "tactician" => Some("way"),
        "scout" => Some("explorer"),
        "raider" => Some("code specialist"),
        "scribe" => Some("documenter"),
        "arbiter" => Some("verifier"),
        "sentinel" => Some("sentinel"),
        "companion_reader" => Some("companion_reader"),
        "companion_operator" => Some("companion_operator"),
        _ => None,
    }
}

fn role_model_tier_intent(role_config_snapshot: &Value) -> &'static str {
    match role_config_snapshot.get("variant").and_then(Value::as_str) {
        Some("low") => "low_cost",
        Some("high") | Some("xhigh") => "high_tier",
        _ => "standard",
    }
}

fn request_kind_for_task_kind(task_kind: &str) -> &'static str {
    match task_kind {
        "way" => "way",
        "review" => "verification",
        "explore" => "advisory",
        _ => "execution",
    }
}

pub(crate) fn collect_codex_config_overrides(role_config_snapshot: &Value) -> Vec<String> {
    let mut config_entries = role_config_snapshot
        .get("config_entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let has_reasoning_override = config_entries
        .iter()
        .any(|entry| entry.trim_start().starts_with("model_reasoning_effort="));
    if !has_reasoning_override {
        if let Some(variant) = role_config_snapshot.get("variant").and_then(Value::as_str) {
            config_entries.push(format!("model_reasoning_effort=\"{variant}\""));
        }
    }

    config_entries
}

pub(crate) fn create_role_model_launch_evidence(
    role: &str,
    task_kind: &str,
    codex_path: &str,
    role_config_snapshot: &Value,
    dispatched_config_entries: &[String],
    recorded_at: &str,
) -> Value {
    json!({
        "role": role,
        "request_kind": request_kind_for_task_kind(task_kind),
        "launch_source": "ccc_spawn",
        "codex_path": codex_path,
        "configured_profile": role_config_snapshot.get("profile").cloned().unwrap_or(Value::Null),
        "configured_model": role_config_snapshot.get("model").cloned().unwrap_or(Value::Null),
        "configured_variant": role_config_snapshot.get("variant").cloned().unwrap_or(Value::Null),
        "configured_fast_mode": role_config_snapshot.get("fast_mode").and_then(Value::as_bool).unwrap_or(false),
        "dispatched_profile": role_config_snapshot.get("profile").cloned().unwrap_or(Value::Null),
        "dispatched_model": role_config_snapshot.get("model").cloned().unwrap_or(Value::Null),
        "dispatched_variant": role_config_snapshot.get("variant").cloned().unwrap_or(Value::Null),
        "dispatched_fast_mode": role_config_snapshot.get("fast_mode").and_then(Value::as_bool).unwrap_or(false),
        "dispatched_config_entries": dispatched_config_entries,
        "actual_profile": role_config_snapshot.get("profile").cloned().unwrap_or(Value::Null),
        "actual_model": role_config_snapshot.get("model").cloned().unwrap_or(Value::Null),
        "actual_variant": role_config_snapshot.get("variant").cloned().unwrap_or(Value::Null),
        "actual_fast_mode": role_config_snapshot.get("fast_mode").and_then(Value::as_bool).unwrap_or(false),
        "actual_config_entries": dispatched_config_entries,
        "observed_profile": Value::Null,
        "observed_model": Value::Null,
        "observed_variant": Value::Null,
        "observed_source": Value::Null,
        "observed_confidence": Value::Null,
        "observed_capability": "launch_request_only",
        "observation_status": "not_started",
        "observation_match_state": "not_started",
        "observation_unavailable_reason": Value::Null,
        "observation_mismatch_summary": Value::Null,
        "match_state": "verified_match",
        "mismatch_summary": Value::Null,
        "recorded_at": recorded_at,
    })
}

fn configured_role_list() -> [&'static str; 9] {
    [
        "orchestrator",
        "way",
        "explorer",
        "code specialist",
        "documenter",
        "verifier",
        "sentinel",
        "companion_reader",
        "companion_operator",
    ]
}

pub(crate) fn create_configured_role_models_payload_from_config(config: &Value) -> Vec<Value> {
    configured_role_list()
    .into_iter()
    .map(|role| {
        let snapshot = load_role_config_snapshot_from_config(config, role);
        json!({
            "role": role,
            "agent_id": agent_id_for_role(role),
            "summary": snapshot.get("summary").cloned().unwrap_or(Value::Null),
            "profile": snapshot.get("profile").cloned().unwrap_or(Value::Null),
            "model": snapshot.get("model").cloned().unwrap_or(Value::Null),
            "variant": snapshot.get("variant").cloned().unwrap_or(Value::Null),
            "fast_mode": snapshot.get("fast_mode").and_then(Value::as_bool).unwrap_or(false),
            "config_entries": snapshot.get("config_entries").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        })
    })
    .collect()
}

fn managed_custom_agent_role_list() -> [&'static str; 8] {
    [
        "way",
        "explorer",
        "code specialist",
        "documenter",
        "verifier",
        "sentinel",
        "companion_reader",
        "companion_operator",
    ]
}

pub(crate) fn is_managed_custom_agent_name(agent_name: &str) -> bool {
    let Some(agent_id) = agent_name.strip_prefix(GENERATED_CUSTOM_AGENT_NAME_PREFIX) else {
        return false;
    };

    managed_custom_agent_role_list()
        .into_iter()
        .filter_map(agent_id_for_role)
        .any(|managed_agent_id| managed_agent_id == agent_id)
}

fn fallback_role_summary(role: &str) -> &'static str {
    match role {
        "orchestrator" => "Captain keeps the LongWay, selects specialists, and closes the loop.",
        "way" => "Way creation and bounded planning when the next move is still unclear.",
        "explorer" => "Read-only repo investigation and evidence gathering.",
        "code specialist" => "Bounded code and config mutation for implementation and repair.",
        "documenter" => "Docs and operator-facing text updates.",
        "verifier" => "Review, regression detection, and acceptance judgment when needed.",
        "sentinel" => "Ownership and execution-path classification for bounded routing decisions.",
        "companion_reader" => {
            "Lightweight tool-routed evidence gathering for files, docs, and read-only inspection."
        }
        "companion_operator" => {
            "Lightweight tool-routed mutation and operator-side execution for git-backed actions."
        }
        _ => "CCC-managed specialist.",
    }
}

pub(crate) fn generated_custom_agent_name(agent_id: &str) -> String {
    format!("{GENERATED_CUSTOM_AGENT_NAME_PREFIX}{agent_id}")
}

pub(crate) fn callsign_for_role(role: &str) -> Option<&'static str> {
    match role {
        "way" => Some("Executor"),
        "explorer" => Some("Observer"),
        "code specialist" => Some("Marauder"),
        "documenter" => Some("Adjutant"),
        "verifier" => Some("Arbiter"),
        "sentinel" => Some("Overseer"),
        "companion_reader" => Some("Probe"),
        "companion_operator" => Some("SCV"),
        _ => None,
    }
}

pub(crate) fn status_display_role(role: &str) -> String {
    let Some(callsign) = callsign_for_role(role) else {
        return role.to_string();
    };
    let Some(agent_id) = agent_id_for_role(role) else {
        return role.to_string();
    };
    let stable_id = generated_custom_agent_name(agent_id);
    format!("{callsign}({stable_id})/{role}")
}

pub(crate) fn status_display_agent(agent_id: &str) -> String {
    let trimmed = agent_id.trim();
    if trimmed.is_empty() || trimmed == "unassigned" {
        return trimmed.to_string();
    }
    if !trimmed.starts_with(GENERATED_CUSTOM_AGENT_NAME_PREFIX) && trimmed.contains('-') {
        return trimmed.to_string();
    }
    let normalized = trimmed
        .strip_prefix(GENERATED_CUSTOM_AGENT_NAME_PREFIX)
        .unwrap_or(trimmed);
    let agent_base = if role_for_agent_id(normalized).is_some() {
        normalized
    } else {
        normalized
            .split_once('_')
            .map(|(base, _)| base)
            .or_else(|| normalized.split_once('-').map(|(base, _)| base))
            .unwrap_or(normalized)
    };
    let Some(role) = role_for_agent_id(agent_base) else {
        return trimmed.to_string();
    };
    let Some(callsign) = callsign_for_role(role) else {
        return trimmed.to_string();
    };
    let stable_id = if trimmed.starts_with(GENERATED_CUSTOM_AGENT_NAME_PREFIX) {
        trimmed.to_string()
    } else {
        generated_custom_agent_name(agent_base)
    };
    format!("{callsign}({stable_id})")
}

fn generated_custom_agent_file_name(agent_id: &str) -> String {
    format!("{GENERATED_CUSTOM_AGENT_FILE_PREFIX}{agent_id}.toml")
}

pub(crate) fn normalize_specialist_execution_mode(value: Option<&str>, default: &str) -> String {
    match value
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
    {
        Some("codex_subagent") => "codex_subagent".to_string(),
        Some("codex_exec") => "codex_exec".to_string(),
        Some("visible_degraded_host_fallback") => "visible_degraded_host_fallback".to_string(),
        _ => default.to_string(),
    }
}

pub(crate) fn preferred_specialist_execution_mode(runtime_config: &Value) -> String {
    normalize_specialist_execution_mode(
        runtime_config
            .get("preferred_specialist_execution_mode")
            .and_then(Value::as_str),
        "codex_subagent",
    )
}

pub(crate) fn fallback_specialist_execution_mode(runtime_config: &Value) -> String {
    normalize_specialist_execution_mode(
        runtime_config
            .get("fallback_specialist_execution_mode")
            .and_then(Value::as_str),
        "codex_exec",
    )
}

pub(crate) fn create_specialist_delegation_plan_with_runtime(
    role: &str,
    role_config_snapshot: &Value,
    runtime_config: &Value,
    sandbox_mode: &str,
    sandbox_rationale: &str,
) -> Value {
    let preferred_execution_mode = preferred_specialist_execution_mode(runtime_config);
    let fallback_execution_mode = fallback_specialist_execution_mode(runtime_config);
    let assigned_agent_id = agent_id_for_role(role).map(str::to_string);
    let spawnable_agent_id = spawnable_custom_agent_id_for_role(role);
    let preferred_custom_agent_name = spawnable_agent_id.map(generated_custom_agent_name);
    let preferred_custom_agent_file = spawnable_agent_id.map(generated_custom_agent_file_name);
    let skill_registry = preferred_custom_agent_name
        .as_deref()
        .map(|agent_name| load_skill_registry_for_agent(agent_name, role_config_snapshot))
        .unwrap_or_else(|| {
            json!({
                "schema": "ccc.skill_registry.v1",
                "status": "not_applicable",
                "blocking": false,
                "runtime_truth": false,
                "advisory_only": true,
                "fallback": "SKILL.md + ccc-config.toml",
            })
        });
    let skill_ssl_manifest = skill_registry
        .get("skill_ssl_manifest")
        .cloned()
        .unwrap_or_else(|| skill_registry.clone());
    let summary = role_config_snapshot
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| fallback_role_summary(role).to_string());
    let model = role_config_snapshot
        .get("model")
        .cloned()
        .unwrap_or(Value::Null);
    let variant = role_config_snapshot
        .get("variant")
        .cloned()
        .unwrap_or(Value::Null);
    let fast_mode = role_config_snapshot
        .get("fast_mode")
        .cloned()
        .unwrap_or(Value::Bool(false));
    let expertise_framing = task_expertise_framing_for_role(role, "single_scoped_task");
    let execution_contract = create_execution_contract_for_role(
        role,
        role_config_snapshot,
        runtime_config,
        sandbox_mode,
        sandbox_rationale,
    );
    let spec_surfaces = json!({
        "role_owned": {
            "owned_by": "role_config_snapshot",
            "fields": [
                "summary",
                "model",
                "variant",
                "fast_mode",
            ],
        },
        "sandbox_owned": {
            "owned_by": "sandbox_policy_helpers",
            "fields": [
                "sandbox_mode",
                "sandbox_rationale",
            ],
        },
        "workflow_owned": {
            "owned_by": "delegation_plan",
            "fields": [
                "routing_source",
                "preferred_execution_mode",
                "fallback_execution_mode",
                "supported_execution_modes",
                "execution_contract",
                "runtime_dispatch",
                "expertise_framing",
                "fan_in_contract",
                "lane_artifact_contract",
                "verify_retry_recap_report_contract",
                "subagent_spawn_contract",
                "subagent_update_contract",
                "fallback_gate",
            ],
        },
        "plan_invariants": {
            "owned_by": "delegation_plan_invariants",
            "fields": [
                "policy_drift_check_required",
                "captain_checkpoint_required",
                "fallback_reason_codes",
            ],
        },
        "skill_ssl_manifest": {
            "owned_by": "skill_registry",
            "fields": [
                "scheduling",
                "structural",
                "logical",
            ],
        },
        "skill_registry": {
            "owned_by": "skill_registry",
            "fields": [
                "status",
                "evidence_sources",
                "source_priority",
            ],
        },
    });
    let supported_execution_modes = if preferred_execution_mode == fallback_execution_mode {
        vec![Value::String(preferred_execution_mode.clone())]
    } else {
        vec![
            Value::String(preferred_execution_mode.clone()),
            Value::String(fallback_execution_mode.clone()),
        ]
    };
    let runtime_dispatch = json!({
        "source": "config_backed",
        "execution_mode_source": "runtime_config",
        "role_profile_source": "role_config_snapshot",
        "custom_agent_source": "role_mapping",
        "plan_invariants_source": "delegation_plan_invariants",
        "preferred_execution_mode": preferred_execution_mode.clone(),
        "fallback_execution_mode": fallback_execution_mode.clone(),
        "supported_execution_modes": supported_execution_modes.clone(),
        "preferred_custom_agent_name": preferred_custom_agent_name.clone(),
        "preferred_custom_agent_file": preferred_custom_agent_file.clone(),
        "skill_ssl_manifest": skill_ssl_manifest.clone(),
        "skill_registry": skill_registry.clone(),
        "assigned_role": role,
        "assigned_agent_id": assigned_agent_id.clone(),
        "summary": summary.clone(),
        "execution_contract": execution_contract.clone(),
        "expertise_framing": expertise_framing.clone(),
        "model": model.clone(),
        "variant": variant.clone(),
        "fast_mode": fast_mode.clone(),
    });

    json!({
        "routing_source": "ccc_config_custom_agent_sync",
        "preferred_execution_mode": preferred_execution_mode.clone(),
        "fallback_execution_mode": fallback_execution_mode.clone(),
        "supported_execution_modes": supported_execution_modes.clone(),
        "preferred_custom_agent_name": preferred_custom_agent_name.clone(),
        "preferred_custom_agent_file": preferred_custom_agent_file.clone(),
        "skill_ssl_manifest": skill_ssl_manifest.clone(),
        "skill_registry": skill_registry.clone(),
        "assigned_role": role,
        "assigned_agent_id": assigned_agent_id.clone(),
        "summary": summary.clone(),
        "execution_contract": execution_contract,
        "expertise_framing": expertise_framing.clone(),
        "model": model.clone(),
        "variant": variant.clone(),
        "fast_mode": fast_mode.clone(),
        "runtime_dispatch": runtime_dispatch,
        "sandbox_mode": sandbox_mode,
        "sandbox_rationale": sandbox_rationale,
        "spec_surfaces": spec_surfaces,
        "fan_in_contract": {
            "mode": "structured_summary",
            "required_fields": [
                "summary",
                "status",
                "evidence_paths",
                "next_action",
                "open_questions",
                "confidence",
            ],
        },
        "lane_artifact_contract": {
            "result": {
                "field": "fan_in",
                "source": "parallel_fanout.lanes[].fan_in",
            },
            "log": {
                "field": "lifecycle",
                "source": "parallel_fanout.lanes[].lifecycle",
            },
            "recap": {
                "field": "fan_in.summary",
                "source": "parallel_fanout.lanes[].fan_in.summary",
            },
        },
        "verify_retry_recap_report_contract": {
            "verify": {
                "field": "verification_state",
                "states": [
                    "pending",
                    "passed",
                    "needs_work",
                    "blocked",
                ],
            },
            "retry": {
                "field": "captain_follow_up",
                "budget_key": "retry",
                "states": [
                    "queued",
                    "consumed",
                ],
            },
            "recap": {
                "field": "lane_artifact_contract.recap",
                "source": "parallel_fanout.lanes[].fan_in.summary",
            },
            "report": {
                "field": "latest_delegate_result.result_summary",
                "fallback_field": "latest_delegate_result.assistant_message_preview",
            },
        },
        "subagent_spawn_contract": {
            "mode": "custom_agent",
            "custom_agent_name": preferred_custom_agent_name,
            "custom_agent_file": preferred_custom_agent_file,
            "expertise_phrase": expertise_framing.get("expertise_phrase").cloned().unwrap_or(Value::Null),
            "task_stance": expertise_framing.get("task_stance").cloned().unwrap_or(Value::Null),
            "expected_thinking_mode": expertise_framing.get("expected_thinking_mode").cloned().unwrap_or(Value::Null),
            "forbid_full_history_fork": true,
            "prefer_fresh_child_context": true,
            "omit_agent_type_override": true,
            "omit_model_override": true,
            "omit_reasoning_effort_override": true,
            "retry_once_without_full_history_fork_on_conflict": true,
        },
        "subagent_update_contract": {
            "transport": "ccc_cli_subcommand",
            "command": "ccc subagent-update --quiet --json '{...}'",
            "inline_command": "ccc subagent-update --quiet --json '{...}'",
            "default_payload_transport": "inline_json",
            "avoid_mcp_tool_call": true,
        },
        "fallback_gate": {
            "must_attempt_preferred_subagent_first": true,
            "host_subagent_spawn_required": preferred_custom_agent_name.is_some()
                && preferred_execution_mode == "codex_subagent",
            "block_direct_execution_until_spawn_recorded": preferred_custom_agent_name.is_some()
                && preferred_execution_mode == "codex_subagent",
            "block_exec_fallback_while_subagent_active": true,
            "require_explicit_subagent_fallback_reason": true,
        },
        "policy_drift_check_required": true,
        "captain_checkpoint_required": true,
        "subagent_available": preferred_custom_agent_name.is_some(),
        "fallback_reason_codes": SUBAGENT_FALLBACK_REASON_CODES,
    })
}

pub(crate) fn create_specialist_delegation_plan(
    role: &str,
    role_config_snapshot: &Value,
    sandbox_mode: &str,
    sandbox_rationale: &str,
) -> Value {
    let runtime_config = load_runtime_config().unwrap_or_else(|_| {
        json!({
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec",
        })
    });
    create_specialist_delegation_plan_with_runtime(
        role,
        role_config_snapshot,
        &runtime_config,
        sandbox_mode,
        sandbox_rationale,
    )
}

fn custom_agent_description_for_role(role: &str, snapshot: &Value) -> String {
    let base = snapshot
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback_role_summary(role));
    format!("CCC-synced specialist. {base}")
}

fn external_path_guidance_instruction() -> &'static str {
    "External paths: use exact operator paths only when readable and sandbox-allowed; if blocked, report path+approval needed."
}

fn fan_in_output_instruction() -> &'static str {
    "Fan-in only: summary, status, evidence_paths, next_action, open_questions, confidence. Stop when acceptance evidence is enough."
}

fn token_discipline_instruction() -> &'static str {
    "Token discipline: task-specific context only; no full-history dumps, repeated task-card text, or narrative transcript."
}

pub(crate) fn custom_agent_developer_instructions_for_role(role: &str, snapshot: &Value) -> String {
    let summary = snapshot
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback_role_summary(role));
    let sandbox = sandbox_mode_for_role(role);
    let sandbox_rationale = sandbox_rationale_for_role(role);
    let external_path_guidance = external_path_guidance_instruction();
    let fan_in_output = fan_in_output_instruction();
    let token_discipline = token_discipline_instruction();

    match role {
        "orchestrator" => format!(
            "CCC captain: own LongWay, routing, fan-in, validation, closeout. Use operator language and compact prompts/results.\nClarify broad/risky/ambiguous/irreversible work with 1-3 questions; narrow work may proceed with assumptions.\nAfter $cap, no apply_patch or direct shell mutation for specialist-owned work unless terminal fallback/operator override is recorded.\nDo not repeat delegated search or mutation ownership unless reclaim, stale output, or an explicit reason is recorded in status.\nIf subagent capacity is exhausted: wait for fan-in, close completed host threads, or record reclaim/reassign/fallback before retrying.\nIf trivial/fallback work grows non-trivial, stop and delegate the follow-up.\nShow role/lane/scope/fan-in/review gate only when useful.\n{token_discipline}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        "way" => format!(
            "CCC Way: return only the distilled plan/next move. Compare options, assumptions, gates, parallel fit, and operator decisions. No file mutation; concise operator-language LongWay.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        "explorer" | "companion_reader" => format!(
            "CCC read-only: inspect/trace and return concise file-referenced evidence. Prefer primary sources, note search bounds, avoid long excerpts. No mutation; smallest useful context.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        "verifier" => format!(
            "CCC verifier: findings first, evidence-backed, scoped to acceptance. Check regressions, security, performance, accessibility, tests, behavior; include file-line fixes when useful. No mutation.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        "sentinel" => format!(
            "CCC sentinel: classify owner, execution path, lane fit, shared-scope conflict, and checkpoint need. No mutation; concise risk/ownership decision.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        "documenter" => format!(
            "CCC docs: edit only assigned docs/operator text. Keep style/translation fidelity, avoid unrelated edits, report changed files and docs-only validation.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        "companion_operator" => format!(
            "CCC companion operator: only assigned lightweight mutation/operator command work. For git/gh/release/commit actions, keep scope explicit, avoid destructive commands without approval, and report command outcomes. {DEFAULT_COMMIT_MESSAGE_GUIDANCE} Do not broaden without captain fallback/reassignment.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
        _ => format!(
            "CCC implementer: make the smallest defensible scoped change. Respect module/ownership boundaries, choose focused tests, report validation or blocked repair handoff. Split helpers only for real duplication/testability; avoid broad rewrites and unrelated reverts.\n{token_discipline}\n{fan_in_output}\n{external_path_guidance}\nSummary: {summary}\nSandbox: {sandbox}. {sandbox_rationale}"
        ),
    }
}

fn render_custom_agent_toml(snapshot: &Value, role: &str, agent_id: &str) -> io::Result<String> {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "name".to_string(),
        Value::String(generated_custom_agent_name(agent_id)),
    );
    payload.insert(
        "description".to_string(),
        Value::String(custom_agent_description_for_role(role, snapshot)),
    );
    payload.insert(
        "developer_instructions".to_string(),
        Value::String(custom_agent_developer_instructions_for_role(role, snapshot)),
    );
    payload.insert(
        "sandbox_mode".to_string(),
        Value::String(sandbox_mode_for_role(role).to_string()),
    );
    if let Some(model) = snapshot
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        payload.insert("model".to_string(), Value::String(model.to_string()));
    }
    if let Some(variant) = snapshot
        .get("variant")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        payload.insert(
            "model_reasoning_effort".to_string(),
            Value::String(variant.to_string()),
        );
    }
    if snapshot
        .get("fast_mode")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        payload.insert(
            "service_tier".to_string(),
            Value::String("fast".to_string()),
        );
    }

    toml::to_string_pretty(
        &sanitize_value_for_toml(&Value::Object(payload))
            .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
    )
    .map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode custom agent TOML for {role}: {error}"),
        )
    })
}

fn create_managed_custom_agent_specs_from_config(
    config: &Value,
) -> io::Result<Vec<ManagedCustomAgentSpec>> {
    managed_custom_agent_role_list()
        .into_iter()
        .filter_map(|role| {
            let agent_id = agent_id_for_role(role)?;
            let snapshot = load_role_config_snapshot_from_config(config, role);
            Some((role, agent_id, snapshot))
        })
        .map(|(role, agent_id, snapshot)| {
            Ok(ManagedCustomAgentSpec {
                role: role.to_string(),
                agent_id: agent_id.to_string(),
                generated_name: generated_custom_agent_name(agent_id),
                file_name: generated_custom_agent_file_name(agent_id),
                content: render_custom_agent_toml(&snapshot, role, agent_id)?,
            })
        })
        .collect()
}

pub(crate) fn sync_generated_custom_agents_in_directory(
    config: &Value,
    install_directory: &Path,
) -> io::Result<Value> {
    fs::create_dir_all(install_directory)?;
    let specs = create_managed_custom_agent_specs_from_config(config)?;
    let expected_files = specs
        .iter()
        .map(|spec| spec.file_name.clone())
        .collect::<BTreeSet<_>>();

    for spec in &specs {
        write_string_atomic(&install_directory.join(&spec.file_name), &spec.content)?;
    }

    for entry in fs::read_dir(install_directory)?.filter_map(Result::ok) {
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if file_name.starts_with(GENERATED_CUSTOM_AGENT_FILE_PREFIX)
            && file_name.ends_with(".toml")
            && !expected_files.contains(&file_name)
        {
            fs::remove_file(path)?;
        }
    }

    Ok(json!({
        "status": "matching_sync",
        "summary": format!(
            "Synced {} CCC-managed custom agents into {}.",
            specs.len(),
            install_directory.display()
        ),
        "directory_path": install_directory.to_string_lossy(),
        "generated_names": specs.iter().map(|spec| Value::String(spec.generated_name.clone())).collect::<Vec<_>>(),
        "generated_files": specs.iter().map(|spec| Value::String(spec.file_name.clone())).collect::<Vec<_>>(),
        "generated_roles": specs
            .iter()
            .map(|spec| {
                json!({
                    "role": spec.role,
                    "agent_id": spec.agent_id,
                    "generated_name": spec.generated_name,
                    "file_name": spec.file_name,
                })
            })
            .collect::<Vec<_>>(),
        "file_count": specs.len(),
    }))
}

pub(crate) fn sync_generated_custom_agents_from_config(config: &Value) -> io::Result<Value> {
    let install_directory = resolve_custom_agent_install_directory()?;
    sync_generated_custom_agents_in_directory(config, &install_directory)
}

pub(crate) fn inspect_generated_custom_agents_in_directory(
    config: &Value,
    install_directory: &Path,
) -> io::Result<Value> {
    let specs = create_managed_custom_agent_specs_from_config(config)?;
    let mut missing = Vec::new();
    let mut mismatched = Vec::new();
    let mut present_names = Vec::new();

    for spec in &specs {
        let path = install_directory.join(&spec.file_name);
        match fs::read_to_string(&path) {
            Ok(content) => {
                present_names.push(Value::String(spec.generated_name.clone()));
                if content != spec.content {
                    mismatched.push(Value::String(spec.file_name.clone()));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                missing.push(Value::String(spec.file_name.clone()));
            }
            Err(error) => return Err(error),
        }
    }

    let stale_managed_files = match fs::read_dir(install_directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                (file_name.starts_with(GENERATED_CUSTOM_AGENT_FILE_PREFIX)
                    && file_name.ends_with(".toml")
                    && !specs.iter().any(|spec| spec.file_name == file_name))
                .then(|| Value::String(file_name))
            })
            .collect::<Vec<_>>(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => specs
            .iter()
            .map(|spec| Value::String(spec.file_name.clone()))
            .collect::<Vec<_>>(),
        Err(error) => return Err(error),
    };

    let status = if missing.is_empty() && mismatched.is_empty() && stale_managed_files.is_empty() {
        "matching_sync"
    } else if present_names.is_empty() {
        "missing_sync"
    } else {
        "mismatched_sync"
    };
    let summary = match status {
        "matching_sync" => format!(
            "CCC-managed custom agents are synced under {}.",
            install_directory.display()
        ),
        "missing_sync" => format!(
            "CCC-managed custom agents are missing under {}.",
            install_directory.display()
        ),
        _ => format!(
            "CCC-managed custom agents under {} need resync.",
            install_directory.display()
        ),
    };

    Ok(json!({
        "status": status,
        "summary": summary,
        "directory_path": install_directory.to_string_lossy(),
        "generated_names": specs.iter().map(|spec| Value::String(spec.generated_name.clone())).collect::<Vec<_>>(),
        "generated_files": specs.iter().map(|spec| Value::String(spec.file_name.clone())).collect::<Vec<_>>(),
        "file_count": specs.len(),
        "missing_files": missing,
        "mismatched_files": mismatched,
        "stale_managed_files": stale_managed_files,
    }))
}

pub(crate) fn inspect_generated_custom_agents_from_config(config: &Value) -> io::Result<Value> {
    let install_directory = resolve_custom_agent_install_directory()?;
    inspect_generated_custom_agents_in_directory(config, &install_directory)
}
