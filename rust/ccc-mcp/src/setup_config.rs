use crate::entry_policy::normalize_entry_policy_mode_value;
use crate::request_routing::{default_routing_config, default_tool_routing_config};
use crate::{
    create_timestamped_backup, read_optional_json_document, read_optional_toml_document,
    resolve_legacy_shared_json_config_path, resolve_legacy_shared_toml_config_path,
    resolve_previous_shared_config_path_for, resolve_shared_config_path,
    timestamped_backup_path_for, write_toml_document,
};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) const CURRENT_GENERATED_DEFAULTS_VERSION: u64 = 16;

fn managed_agent_config(
    name: &str,
    summary: &str,
    model: &str,
    variant: &str,
    fast_mode: bool,
    role_metadata: Value,
) -> Value {
    let mut config = json!({
        "name": name,
        "summary": summary,
        "model": model,
        "variant": variant,
        "fast_mode": fast_mode,
        "config_entries": []
    });
    merge_missing_config_fields(&mut config, &role_metadata);
    config
}

fn role_metadata_for_config_key(key: &str) -> Option<Value> {
    let (display_name, callsign, workflows, lsp_capabilities) = match key {
        "way" | "planner" => (
            "Executor",
            "Executor",
            vec!["hyperplan"],
            vec!["lsp_diagnostics", "lsp_definition"],
        ),
        "explorer" => (
            "Observer",
            "Observer",
            vec!["github-triage", "get-unpublished-changes"],
            vec!["lsp_diagnostics", "lsp_references", "lsp_definition"],
        ),
        "code specialist" => (
            "Marauder",
            "Marauder",
            vec![
                "remove-deadcode",
                "ai-slop-remover",
                "lsp-safe-refactor",
                "rust-analyzer-lsp",
            ],
            vec![
                "lsp_diagnostics",
                "lsp_references",
                "lsp_definition",
                "lsp_prepare_rename",
                "lsp_rename",
                "rust-analyzer-lsp",
            ],
        ),
        "documenter" => (
            "Adjutant",
            "Adjutant",
            vec!["release-note", "readme-maintenance", "changelog"],
            vec!["lsp_diagnostics"],
        ),
        "verifier" => (
            "Arbiter",
            "Arbiter",
            vec!["review-work", "pre-publish-review"],
            vec!["lsp_diagnostics", "lsp_references", "lsp_definition"],
        ),
        "sentinel" => (
            "Overseer",
            "Overseer",
            vec!["role-ownership", "lane-conflict", "fallback-classification"],
            vec!["lsp_diagnostics"],
        ),
        "companion_reader" => (
            "Probe",
            "Probe",
            vec![
                "github-triage",
                "filesystem-evidence",
                "get-unpublished-changes",
            ],
            vec!["lsp_diagnostics", "lsp_references", "lsp_definition"],
        ),
        "companion_operator" => (
            "SCV",
            "SCV",
            vec!["git-master", "publish", "release-command-discipline"],
            Vec::new(),
        ),
        _ => return None,
    };

    Some(json!({
        "display_name": display_name,
        "callsign": callsign,
        "theme": "starcraft_display_callsign",
        "inspired_by": ["oh-my-openagent"],
        "recommended_workflows": workflows,
        "lsp_capabilities": lsp_capabilities,
    }))
}

fn default_sentinel_agent_config() -> Value {
    managed_agent_config(
        "sentinel",
        "Ownership and execution-path classification for bounded routing decisions.",
        "gpt-5.4-mini",
        "high",
        true,
        role_metadata_for_config_key("sentinel").expect("sentinel metadata"),
    )
}

fn default_companion_agents_config() -> Value {
    json!({
        "companion_reader": managed_agent_config(
            "companion_reader",
            "Low-cost read-only tool work for filesystem, docs, web, git/gh inspection, and evidence gathering.",
            "gpt-5.4-mini",
            "medium",
            true,
            role_metadata_for_config_key("companion_reader").expect("companion reader metadata"),
        ),
        "companion_operator": managed_agent_config(
            "companion_operator",
            "Low-cost bounded operator work for git/gh mutation, release commands, and other narrow tool execution.",
            "gpt-5.4-mini",
            "medium",
            true,
            role_metadata_for_config_key("companion_operator").expect("companion operator metadata"),
        )
    })
}

fn default_specialist_agents_config() -> Value {
    json!({
        "way": managed_agent_config(
            "tactician",
            "Way creation and bounded planning when the next move is still unclear.",
            "gpt-5.5",
            "high",
            true,
            role_metadata_for_config_key("way").expect("way metadata"),
        ),
        "explorer": managed_agent_config(
            "scout",
            "Read-only repo investigation and evidence gathering.",
            "gpt-5.4-mini",
            "high",
            true,
            role_metadata_for_config_key("explorer").expect("explorer metadata"),
        ),
        "code specialist": managed_agent_config(
            "raider",
            "Bounded code and config mutation for implementation and repair.",
            "gpt-5.5",
            "high",
            true,
            role_metadata_for_config_key("code specialist").expect("raider metadata"),
        ),
        "documenter": managed_agent_config(
            "scribe",
            "Docs and operator-facing text updates.",
            "gpt-5.4-mini",
            "medium",
            true,
            role_metadata_for_config_key("documenter").expect("documenter metadata"),
        ),
        "verifier": managed_agent_config(
            "arbiter",
            "Review, regression detection, and acceptance judgment when needed.",
            "gpt-5.5",
            "high",
            true,
            role_metadata_for_config_key("verifier").expect("verifier metadata"),
        ),
        "sentinel": default_sentinel_agent_config()
    })
}

fn default_lsp_config() -> Value {
    json!({
        "enabled": false,
        "runtime_execution": "deferred",
        "deferred_reason": "0.0.15-pre records TypeScript/JavaScript and Rust rust-analyzer LSP command contracts but does not run language servers from CCC yet.",
        "capabilities": [
            "lsp_diagnostics",
            "lsp_references",
            "lsp_definition",
            "lsp_prepare_rename",
            "lsp_rename",
            "rust-analyzer-lsp"
        ],
        "language_servers": {
            "typescript_javascript": {
                "command": "typescript-language-server",
                "args": ["--stdio"],
                "package_hint": "npm install -g typescript typescript-language-server",
                "file_extensions": ["ts", "tsx", "js", "jsx", "mjs", "cjs"]
            },
            "rust": {
                "command": "rust-analyzer",
                "args": [],
                "package_hint": "rustup component add rust-analyzer",
                "file_extensions": ["rs"]
            }
        }
    })
}

fn default_features_config() -> Value {
    json!({
        "graph_context": false,
        "goals": false,
        "prompt_refinement": false
    })
}

fn default_goal_bridge_config() -> Value {
    json!({
        "enabled": false,
        "mode": "captain_owned",
        "brief_language": "en",
        "brief_max_lines": 12,
        "require_verifiable_stop": true,
        "host_goal_state_is_truth": false,
        "specialists": {
            "allow_specialist_goal_context": true,
            "allow_specialist_set_goal": false,
            "allow_specialist_clear_goal": false,
            "allow_specialist_override_goal": false,
            "max_subgoal_lines": 8,
            "require_captain_acceptance": true
        }
    })
}

fn default_graph_context_config() -> Value {
    json!({
        "enabled": false,
        "provider": "graphify",
        "mode": "read_only",
        "canonical_backend": "graphify",
        "replace_legacy_ccc_graph_backend": true,
        "allow_legacy_graph_backend_fallback": false,
        "fallback_when_unavailable": "scout_source_evidence",
        "report_path": "graphify-out/GRAPH_REPORT.md",
        "graph_path": "graphify-out/graph.json",
        "max_report_bytes": 20000,
        "max_query_bytes": 8000,
        "prefer_report_before_grep": true,
        "allow_cli_query": true,
        "allow_mcp_query": false,
        "allow_rebuild": false,
        "auto_install_external_dependency": false,
        "source_of_truth": false,
        "install": {
            "managed_by_ccc_setup": true,
            "check_install_reports_readiness": true,
            "require_graphify_cli_for_queries": true,
            "allow_missing_provider_fallback": true
        },
        "edges": {
            "allow_extracted": true,
            "allow_inferred": true,
            "allow_ambiguous": false,
            "require_source_check_for_mutation": true
        }
    })
}

fn default_generated_defaults_policy() -> Value {
    json!({
        "version": CURRENT_GENERATED_DEFAULTS_VERSION,
        "policy": "ccc-managed-defaults",
    })
}

fn default_runtime_config() -> Value {
    json!({
        "preferred_specialist_execution_mode": "codex_subagent",
        "fallback_specialist_execution_mode": "codex_exec",
        "worker_poll_interval_ms": 90000,
        "worker_stuck_after_ms": 45000,
        "worker_kill_grace_ms": 2000,
        "worker_auto_reclaim_enabled": true,
        "worker_max_retries_per_phase": 1,
        "worker_retry_backoff_ms": 1000,
        "worker_prompt_scope_max_chars": 320,
        "worker_prompt_acceptance_max_chars": 220,
        "worker_prompt_task_max_chars": 720,
        "run_lock_stale_after_ms": 300000,
    })
}

fn merge_missing_config_fields(target: &mut Value, defaults: &Value) -> bool {
    let (Value::Object(target_entries), Value::Object(default_entries)) = (target, defaults) else {
        return false;
    };

    let mut changed = false;
    for (key, default_value) in default_entries {
        match target_entries.get_mut(key) {
            Some(existing_value) if existing_value.is_null() => {
                *existing_value = default_value.clone();
                changed = true;
            }
            Some(existing_value) => {
                if merge_missing_config_fields(existing_value, default_value) {
                    changed = true;
                }
            }
            None => {
                target_entries.insert(key.clone(), default_value.clone());
                changed = true;
            }
        }
    }
    changed
}

fn generated_defaults_version(config: &Value) -> u64 {
    config
        .get("generated_defaults")
        .and_then(|value| value.get("version"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn entry_policy_mode_backfill_reason(config: &Value) -> Option<String> {
    let entry_policy = config.get("entry_policy")?;
    let Some(entry_policy_object) = entry_policy.as_object() else {
        return Some(
            "Entry policy config must be an object with a supported mode; run setup to backfill `guided_explicit`."
                .to_string(),
        );
    };
    let Some(mode_value) = entry_policy_object.get("mode") else {
        return Some(
            "Entry policy config is missing `mode`; run setup to backfill `guided_explicit`."
                .to_string(),
        );
    };
    let Some(raw_mode) = mode_value.as_str() else {
        return Some(
            "Entry policy mode must be a string; run setup to backfill `guided_explicit`."
                .to_string(),
        );
    };
    match normalize_entry_policy_mode_value(raw_mode) {
        Some(canonical_mode) if canonical_mode == raw_mode => None,
        Some(canonical_mode) => Some(format!(
            "Entry policy mode `{raw_mode}` is a legacy alias for `{canonical_mode}`; run setup to backfill the canonical value."
        )),
        None => Some(format!(
            "Entry policy mode `{raw_mode}` is not supported; run setup to backfill `guided_explicit`."
        )),
    }
}

fn entry_policy_mode_visibility(
    config: &Value,
) -> (&'static str, Option<String>, Option<String>, String) {
    let Some(entry_policy) = config.get("entry_policy") else {
        return (
            "missing/backfill-needed",
            None,
            Some("guided_explicit".to_string()),
            "Entry policy config is missing; run setup to backfill `guided_explicit`.".to_string(),
        );
    };
    let Some(entry_policy_object) = entry_policy.as_object() else {
        return (
            "invalid/backfill-needed",
            None,
            Some("guided_explicit".to_string()),
            "Entry policy config must be an object with a supported mode; run setup to backfill `guided_explicit`."
                .to_string(),
        );
    };
    let Some(mode_value) = entry_policy_object.get("mode") else {
        return (
            "missing/backfill-needed",
            None,
            Some("guided_explicit".to_string()),
            "Entry policy mode is missing; run setup to backfill `guided_explicit`.".to_string(),
        );
    };
    let Some(raw_mode) = mode_value.as_str() else {
        return (
            "invalid/backfill-needed",
            None,
            Some("guided_explicit".to_string()),
            "Entry policy mode must be a string; run setup to backfill `guided_explicit`."
                .to_string(),
        );
    };
    match normalize_entry_policy_mode_value(raw_mode) {
        Some(canonical_mode) if canonical_mode == raw_mode => (
            "canonical",
            Some(raw_mode.to_string()),
            Some(canonical_mode.to_string()),
            format!("Entry policy mode `{raw_mode}` is canonical and supported."),
        ),
        Some(canonical_mode) => (
            "legacy/backfill-needed",
            Some(raw_mode.to_string()),
            Some(canonical_mode.to_string()),
            format!(
                "Entry policy mode `{raw_mode}` is runtime-compatible as `{canonical_mode}`, but setup should backfill the canonical value."
            ),
        ),
        None => (
            "invalid/unsupported",
            Some(raw_mode.to_string()),
            Some("guided_explicit".to_string()),
            format!(
                "Entry policy mode `{raw_mode}` is unsupported; runtime falls back to `guided_explicit`, and setup should backfill it."
            ),
        ),
    }
}

fn unavailable_entry_policy_mode_visibility(
) -> (&'static str, Option<String>, Option<String>, String) {
    (
        "unavailable",
        None,
        None,
        "Entry policy mode health is unavailable because no readable canonical CCC config was loaded."
            .to_string(),
    )
}

fn config_install_state(
    status: &'static str,
    action_status: &'static str,
    backup_status: &'static str,
    summary: String,
    source_path: Option<PathBuf>,
    backup_source_path: Option<PathBuf>,
    backup_path: Option<PathBuf>,
    value: Value,
    canonical_ready: bool,
    config_exists: bool,
    restart_status: &'static str,
) -> CccConfigInstallState {
    let (
        entry_policy_mode_status,
        entry_policy_mode_raw,
        entry_policy_mode_canonical,
        entry_policy_mode_summary,
    ) = if canonical_ready && config_exists {
        entry_policy_mode_visibility(&value)
    } else {
        unavailable_entry_policy_mode_visibility()
    };

    CccConfigInstallState {
        status,
        action_status,
        backup_status,
        summary,
        source_path,
        backup_source_path,
        backup_path,
        value,
        canonical_ready,
        config_exists,
        restart_status,
        entry_policy_mode_status,
        entry_policy_mode_raw,
        entry_policy_mode_canonical,
        entry_policy_mode_summary,
    }
}

fn backfill_entry_policy_mode(config_entries: &mut serde_json::Map<String, Value>) -> bool {
    let Some(entry_policy) = config_entries.get_mut("entry_policy") else {
        return false;
    };
    let Some(entry_policy_object) = entry_policy.as_object_mut() else {
        *entry_policy = json!({ "mode": "guided_explicit" });
        return true;
    };
    let current_mode = entry_policy_object.get("mode").and_then(Value::as_str);
    let next_mode = current_mode
        .and_then(normalize_entry_policy_mode_value)
        .unwrap_or("guided_explicit");
    if current_mode == Some(next_mode) {
        return false;
    }
    entry_policy_object.insert("mode".to_string(), Value::String(next_mode.to_string()));
    true
}

fn uses_medium_mini_generated_default(path: &[&str]) -> bool {
    path == ["agents", "documenter"]
        || path == ["companion_agents", "companion_reader"]
        || path == ["companion_agents", "companion_operator"]
}

fn upgrade_role_generated_defaults(config: &mut Value, path: &[&str]) -> bool {
    let mut current = config;
    for key in path {
        let Some(next) = current.get_mut(*key) else {
            return false;
        };
        current = next;
    }

    let Some(object) = current.as_object_mut() else {
        return false;
    };

    let mut changed = false;
    // `planner` is the legacy config key for the Way role. Upgrade both keys so
    // older generated installs keep using the documented high-reasoning Way.
    if path == ["agents", "way"]
        || path == ["agents", "planner"]
        || path == ["agents", "verifier"]
        || path == ["agents", "code specialist"]
    {
        if object.get("model").and_then(Value::as_str) == Some("gpt-5.5")
            && object.get("variant").and_then(Value::as_str) == Some("medium")
        {
            object.insert("variant".to_string(), Value::String("high".to_string()));
            changed = true;
        }
    } else if object
        .get("model")
        .and_then(Value::as_str)
        .map(|value| value == "gpt-5.4-mini")
        .unwrap_or(false)
    {
        let target_variant = if uses_medium_mini_generated_default(path) {
            "medium"
        } else {
            "high"
        };
        let current_variant = object.get("variant").and_then(Value::as_str);
        let stale_generated_variant = (target_variant == "medium"
            && current_variant == Some("high"))
            || (target_variant == "high" && current_variant == Some("medium"));
        if stale_generated_variant {
            object.insert(
                "variant".to_string(),
                Value::String(target_variant.to_string()),
            );
            changed = true;
        }
        if object.get("fast_mode").and_then(Value::as_bool) == Some(false) {
            object.insert("fast_mode".to_string(), Value::Bool(true));
            changed = true;
        }
    }
    changed
}

fn apply_generated_default_drift_upgrades(config: &mut Value) -> bool {
    let existing_generated_defaults_version = generated_defaults_version(config);
    let mut changed = false;

    if existing_generated_defaults_version < CURRENT_GENERATED_DEFAULTS_VERSION {
        if let Some(orchestrator) = config
            .get_mut("agents")
            .and_then(|agents| agents.get_mut("orchestrator"))
            .and_then(Value::as_object_mut)
        {
            if orchestrator.get("model").and_then(Value::as_str) == Some("gpt-5.5")
                && orchestrator.get("variant").and_then(Value::as_str) == Some("high")
            {
                orchestrator.insert("variant".to_string(), Value::String("medium".to_string()));
                changed = true;
            }
        }

        for path in [
            &["agents", "explorer"][..],
            &["agents", "sentinel"][..],
            &["agents", "documenter"][..],
            &["agents", "code specialist"][..],
            &["agents", "way"][..],
            &["agents", "planner"][..],
            &["agents", "verifier"][..],
            &["companion_agents", "companion_reader"][..],
            &["companion_agents", "companion_operator"][..],
        ] {
            if upgrade_role_generated_defaults(config, path) {
                changed = true;
            }
        }

        if let Some(runtime) = config.get_mut("runtime").and_then(Value::as_object_mut) {
            if runtime
                .get("fallback_specialist_execution_mode")
                .and_then(Value::as_str)
                == Some("visible_degraded_host_fallback")
            {
                runtime.insert(
                    "fallback_specialist_execution_mode".to_string(),
                    Value::String("codex_exec".to_string()),
                );
                changed = true;
            }
        }

        for (key, default_value) in [
            ("features", default_features_config()),
            ("goal_bridge", default_goal_bridge_config()),
            ("graph_context", default_graph_context_config()),
        ] {
            match config.get_mut(key) {
                Some(existing_value) if existing_value.is_null() => {
                    *existing_value = default_value;
                    changed = true;
                }
                Some(existing_value) => {
                    if merge_missing_config_fields(existing_value, &default_value) {
                        changed = true;
                    }
                }
                None => {
                    if let Some(config_entries) = config.as_object_mut() {
                        config_entries.insert(key.to_string(), default_value);
                        changed = true;
                    }
                }
            }
        }
    }

    let Some(config_entries) = config.as_object_mut() else {
        return changed;
    };

    let default_policy = default_generated_defaults_policy();
    match config_entries.get_mut("generated_defaults") {
        Some(existing) if existing.is_null() => {
            *existing = default_policy;
            changed = true;
        }
        Some(existing) => {
            if merge_missing_config_fields(existing, &default_policy) {
                changed = true;
            }
            if existing
                .get("version")
                .and_then(Value::as_u64)
                .map(|version| version < CURRENT_GENERATED_DEFAULTS_VERSION)
                .unwrap_or(true)
            {
                if let Some(object) = existing.as_object_mut() {
                    object.insert(
                        "version".to_string(),
                        Value::from(CURRENT_GENERATED_DEFAULTS_VERSION),
                    );
                    changed = true;
                }
            }
        }
        None => {
            config_entries.insert("generated_defaults".to_string(), default_policy);
            changed = true;
        }
    }

    changed
}

fn backfill_generated_defaults(config: &mut Value) -> bool {
    let Some(config_entries) = config.as_object_mut() else {
        return false;
    };
    let mut changed = false;

    if backfill_entry_policy_mode(config_entries) {
        changed = true;
    }

    if let Some(existing_value) = config_entries.get_mut("companion_agents") {
        if existing_value.is_null() {
            *existing_value = default_companion_agents_config();
            changed = true;
        } else if merge_missing_config_fields(existing_value, &default_companion_agents_config()) {
            changed = true;
        }
    }
    if let Some(existing_value) = config_entries.get_mut("agents") {
        let specialist_defaults = default_specialist_agents_config();
        if existing_value.is_null() {
            *existing_value = specialist_defaults;
            changed = true;
        } else if merge_missing_config_fields(existing_value, &specialist_defaults) {
            changed = true;
        }
        for role_key in [
            "way",
            "planner",
            "explorer",
            "documenter",
            "code specialist",
            "verifier",
            "sentinel",
        ] {
            if let Some(role_config) = existing_value.get_mut(role_key) {
                if let Some(metadata) = role_metadata_for_config_key(role_key) {
                    if merge_missing_config_fields(role_config, &metadata) {
                        changed = true;
                    }
                }
            }
        }
    }
    if let Some(existing_value) = config_entries.get_mut("companion_agents") {
        for role_key in ["companion_reader", "companion_operator"] {
            if let Some(role_config) = existing_value.get_mut(role_key) {
                if let Some(metadata) = role_metadata_for_config_key(role_key) {
                    if merge_missing_config_fields(role_config, &metadata) {
                        changed = true;
                    }
                }
            }
        }
    }

    // Keep wholly omitted operational sections omitted for minimal configs. Existing
    // sections still get missing fields backfilled so upgrades remain stable.
    for (key, default_value) in [
        ("features", default_features_config()),
        ("goal_bridge", default_goal_bridge_config()),
        ("graph_context", default_graph_context_config()),
        ("routing", default_routing_config()),
        ("tool_routing", default_tool_routing_config()),
        ("runtime", default_runtime_config()),
        ("lsp", default_lsp_config()),
    ] {
        match config_entries.get_mut(key) {
            Some(existing_value) if existing_value.is_null() => {
                *existing_value = default_value;
                changed = true;
            }
            Some(existing_value) => {
                if merge_missing_config_fields(existing_value, &default_value) {
                    changed = true;
                }
            }
            None => {}
        }
    }
    if apply_generated_default_drift_upgrades(config) {
        changed = true;
    }
    changed
}

#[derive(Clone, Debug)]
pub(crate) struct CccConfigInstallState {
    pub(crate) status: &'static str,
    pub(crate) action_status: &'static str,
    pub(crate) backup_status: &'static str,
    pub(crate) summary: String,
    pub(crate) source_path: Option<PathBuf>,
    pub(crate) backup_source_path: Option<PathBuf>,
    pub(crate) backup_path: Option<PathBuf>,
    pub(crate) value: Value,
    pub(crate) canonical_ready: bool,
    pub(crate) config_exists: bool,
    pub(crate) restart_status: &'static str,
    pub(crate) entry_policy_mode_status: &'static str,
    pub(crate) entry_policy_mode_raw: Option<String>,
    pub(crate) entry_policy_mode_canonical: Option<String>,
    pub(crate) entry_policy_mode_summary: String,
}

impl CccConfigInstallState {
    pub(crate) fn source_path_value(&self) -> Value {
        self.source_path
            .as_ref()
            .map(|path| Value::String(path.to_string_lossy().into_owned()))
            .unwrap_or(Value::Null)
    }

    pub(crate) fn backup_source_path_value(&self) -> Value {
        self.backup_source_path
            .as_ref()
            .map(|path| Value::String(path.to_string_lossy().into_owned()))
            .unwrap_or(Value::Null)
    }

    pub(crate) fn backup_path_value(&self) -> Value {
        self.backup_path
            .as_ref()
            .map(|path| Value::String(path.to_string_lossy().into_owned()))
            .unwrap_or(Value::Null)
    }

    pub(crate) fn entry_policy_mode_raw_value(&self) -> Value {
        self.entry_policy_mode_raw
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null)
    }

    pub(crate) fn entry_policy_mode_canonical_value(&self) -> Value {
        self.entry_policy_mode_canonical
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null)
    }
}

fn read_optional_config_source(path: &Path, format: &str) -> io::Result<Option<(PathBuf, Value)>> {
    let value = match format {
        "json" => read_optional_json_document(path)?,
        _ => read_optional_toml_document(path)?,
    };
    Ok(value.map(|value| (path.to_path_buf(), value)))
}

pub(crate) fn collect_ccc_config_install_state_at(
    config_path: &Path,
    legacy_toml_path: &Path,
    legacy_json_path: &Path,
) -> io::Result<CccConfigInstallState> {
    let canonical = read_optional_config_source(config_path, "toml")?;
    let previous = resolve_previous_shared_config_path_for(config_path)
        .filter(|previous_path| previous_path != config_path)
        .map(|previous_path| read_optional_config_source(&previous_path, "toml"))
        .transpose()?
        .flatten();
    let legacy_toml = read_optional_config_source(legacy_toml_path, "toml")?;
    let legacy_json = read_optional_config_source(legacy_json_path, "json")?;

    if let Some((canonical_path, canonical_value)) = canonical {
        let conflicting_source = previous
            .as_ref()
            .filter(|(_, value)| value != &canonical_value)
            .map(|(path, _)| path.clone());
        if let Some(conflicting_source) = conflicting_source {
            return Ok(config_install_state(
                "conflict",
                "preserved",
                "not-required",
                format!(
                    "Canonical CCC config is present at {}, but a legacy migration source at {} differs; setup preserves the canonical file.",
                    canonical_path.display(),
                    conflicting_source.display()
                ),
                Some(canonical_path),
                None,
                None,
                canonical_value,
                true,
                true,
                "not-required",
            ));
        }

        let mut planned_value = canonical_value.clone();
        let entry_policy_backfill_reason = entry_policy_mode_backfill_reason(&canonical_value);
        if backfill_generated_defaults(&mut planned_value) {
            let backfill_detail = entry_policy_backfill_reason
                .map(|reason| format!(" {reason}"))
                .unwrap_or_default();
            return Ok(config_install_state(
                "canonical-needs-backfill",
                "setup-backfill-available",
                "setup-backup-available",
                format!(
                    "Canonical CCC config at {} is missing or has stale generated defaults; run setup to create a timestamped backup, backfill or upgrade generated defaults while preserving customized values, then restart Codex CLI.{}",
                    canonical_path.display(),
                    backfill_detail
                ),
                Some(canonical_path.clone()),
                Some(canonical_path.clone()),
                Some(timestamped_backup_path_for(&canonical_path)),
                canonical_value,
                true,
                true,
                "restart-required-after-setup",
            ));
        }

        return Ok(config_install_state(
            "canonical-current",
            "preserved",
            "not-required",
            format!(
                "Canonical CCC config is current at {}.",
                canonical_path.display()
            ),
            Some(canonical_path),
            None,
            None,
            canonical_value,
            true,
            true,
            "not-required",
        ));
    }

    if let Some((source_path, value)) = previous.or(legacy_toml).or(legacy_json) {
        return Ok(config_install_state(
            "legacy-only",
            "skipped",
            "setup-backup-available",
            format!(
                "CCC config is only available from legacy source {}; run setup to create a timestamped backup, migrate it to the canonical path, then restart Codex CLI.",
                source_path.display()
            ),
            Some(source_path.clone()),
            Some(source_path.clone()),
            Some(timestamped_backup_path_for(&source_path)),
            value,
            false,
            true,
            "restart-required-after-setup",
        ));
    }

    Ok(config_install_state(
        "missing",
        "skipped",
        "not-required",
        format!(
            "No CCC config was found at the canonical path {} or known legacy sources; run setup to create a generated default config, then restart Codex CLI.",
            config_path.display()
        ),
        None,
        None,
        None,
        Value::Null,
        false,
        false,
        "restart-required-after-setup",
    ))
}

pub(crate) fn plan_ccc_config_setup_at(
    config_path: &Path,
    legacy_toml_path: &Path,
    legacy_json_path: &Path,
) -> io::Result<CccConfigInstallState> {
    let mut plan =
        collect_ccc_config_install_state_at(config_path, legacy_toml_path, legacy_json_path)?;
    match plan.status {
        "legacy-only" => {
            plan.action_status = "would-migrate";
            plan.summary = format!(
                "Setup would create a timestamped backup of {}, migrate it to {}, preserve customized values, and require a Codex CLI restart.",
                plan.source_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "the legacy CCC config".to_string()),
                config_path.display()
            );
        }
        "canonical-needs-backfill" => {
            plan.action_status = "would-backfill";
            plan.summary = format!(
                "Setup would create a timestamped backup of {}, backfill or upgrade generated defaults while preserving customized values, and require a Codex CLI restart.",
                config_path.display()
            );
        }
        "missing" => {
            plan.action_status = "would-create";
            plan.summary = format!(
                "Setup would create generated defaults at {} and require a Codex CLI restart.",
                config_path.display()
            );
        }
        "conflict" => {
            plan.summary.push_str(
                " Dry-run will not resolve this conflict; setup preserves the canonical file.",
            );
        }
        _ => {
            plan.summary
                .push_str(" Setup would not change the CCC config.");
        }
    }
    Ok(plan)
}

#[derive(Clone, Debug)]
struct CccConfigApplyReport {
    backup_status: &'static str,
    backup_source_path: Option<PathBuf>,
    backup_path: Option<PathBuf>,
}

impl CccConfigApplyReport {
    fn not_required() -> Self {
        Self {
            backup_status: "not-required",
            backup_source_path: None,
            backup_path: None,
        }
    }
}

fn backup_config_apply_source(
    source_path: &Path,
    report: &mut CccConfigApplyReport,
) -> io::Result<()> {
    let backup_path = create_timestamped_backup(source_path)?;
    report.backup_status = "created";
    report.backup_source_path = Some(source_path.to_path_buf());
    report.backup_path = Some(backup_path);
    Ok(())
}

pub(crate) fn rollback_ccc_config_from_backup_at(
    config_path: &Path,
    backup_path: &Path,
) -> io::Result<CccConfigInstallState> {
    let metadata = fs::metadata(backup_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "CCC config rollback backup {} is not readable: {error}",
                backup_path.display()
            ),
        )
    })?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "CCC config rollback backup {} is not a file.",
                backup_path.display()
            ),
        ));
    }
    let value = read_optional_toml_document(backup_path)?.unwrap_or(Value::Null);

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(backup_path, config_path)?;

    Ok(config_install_state(
        "rollback-restored",
        "rolled-back",
        "restored",
        format!(
            "CCC config rollback restored {} to the canonical path {}; restart Codex CLI to use the restored config.",
            backup_path.display(),
            config_path.display()
        ),
        Some(config_path.to_path_buf()),
        Some(backup_path.to_path_buf()),
        Some(backup_path.to_path_buf()),
        value,
        true,
        true,
        "restart-required",
    ))
}

pub(crate) fn ensure_ccc_config_file_at(
    config_path: &Path,
    legacy_toml_path: &Path,
    legacy_json_path: &Path,
) -> io::Result<(PathBuf, bool)> {
    let (config_path, created, _) =
        ensure_ccc_config_file_at_with_report(config_path, legacy_toml_path, legacy_json_path)?;
    Ok((config_path, created))
}

fn ensure_ccc_config_file_at_with_report(
    config_path: &Path,
    legacy_toml_path: &Path,
    legacy_json_path: &Path,
) -> io::Result<(PathBuf, bool, CccConfigApplyReport)> {
    let mut report = CccConfigApplyReport::not_required();
    if config_path.exists() {
        if let Ok(Some(mut existing_config)) = read_optional_toml_document(config_path) {
            if backfill_generated_defaults(&mut existing_config) {
                backup_config_apply_source(config_path, &mut report)?;
                write_toml_document(config_path, &existing_config)?;
            }
        }
        return Ok((config_path.to_path_buf(), false, report));
    }

    if let Some(previous_path) = resolve_previous_shared_config_path_for(config_path) {
        if previous_path != config_path {
            if let Some(mut previous_config) = read_optional_toml_document(&previous_path)? {
                backfill_generated_defaults(&mut previous_config);
                backup_config_apply_source(&previous_path, &mut report)?;
                write_toml_document(config_path, &previous_config)?;
                return Ok((config_path.to_path_buf(), true, report));
            }
        }
    }

    if let Some(mut legacy_config) = read_optional_toml_document(legacy_toml_path)? {
        backfill_generated_defaults(&mut legacy_config);
        backup_config_apply_source(legacy_toml_path, &mut report)?;
        write_toml_document(config_path, &legacy_config)?;
        return Ok((config_path.to_path_buf(), true, report));
    }

    if let Some(mut legacy_config) = read_optional_json_document(legacy_json_path)? {
        backfill_generated_defaults(&mut legacy_config);
        backup_config_apply_source(legacy_json_path, &mut report)?;
        write_toml_document(config_path, &legacy_config)?;
        return Ok((config_path.to_path_buf(), true, report));
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let default_config = json!({
        "version": 1,
        "generated_defaults": default_generated_defaults_policy(),
        "entry_policy": {
            "mode": "guided_explicit"
        },
        "output": {
            "verbosity": "default",
            "changed_max_chars": 160,
            "include_agent_loop_when_idle": false
        },
        "features": default_features_config(),
        "goal_bridge": default_goal_bridge_config(),
        "lsp": default_lsp_config(),
        "graph_context": default_graph_context_config(),
        "agents": {
            "orchestrator": {
                "name": "captain",
                "summary": "Captain keeps the LongWay, chooses specialists, and closes the loop.",
                "model": "gpt-5.5",
                "variant": "medium",
                "fast_mode": false,
                "config_entries": []
            },
            "way": managed_agent_config(
                "tactician",
                "Way creation and bounded planning when the next move is still unclear.",
                "gpt-5.5",
                "high",
                true,
                role_metadata_for_config_key("way").expect("way metadata"),
            ),
            "explorer": managed_agent_config(
                "scout",
                "Read-only repo investigation and evidence gathering.",
                "gpt-5.4-mini",
                "high",
                true,
                role_metadata_for_config_key("explorer").expect("explorer metadata"),
            ),
            "code specialist": managed_agent_config(
                "raider",
                "Bounded code and config mutation for implementation and repair.",
                "gpt-5.5",
                "high",
                true,
                role_metadata_for_config_key("code specialist").expect("raider metadata"),
            ),
            "documenter": managed_agent_config(
                "scribe",
                "Docs and operator-facing text updates.",
                "gpt-5.4-mini",
                "medium",
                true,
                role_metadata_for_config_key("documenter").expect("documenter metadata"),
            ),
            "verifier": managed_agent_config(
                "arbiter",
                "Review, regression detection, and acceptance judgment when needed.",
                "gpt-5.5",
                "high",
                true,
                role_metadata_for_config_key("verifier").expect("verifier metadata"),
            ),
            "sentinel": default_sentinel_agent_config()
        }
    });
    write_toml_document(config_path, &default_config)?;
    Ok((config_path.to_path_buf(), true, report))
}

pub(crate) fn ensure_ccc_config_file_at_with_state(
    config_path: &Path,
    legacy_toml_path: &Path,
    legacy_json_path: &Path,
) -> io::Result<(PathBuf, bool, CccConfigInstallState)> {
    let before =
        collect_ccc_config_install_state_at(config_path, legacy_toml_path, legacy_json_path).ok();
    let (created_path, created, apply_report) =
        ensure_ccc_config_file_at_with_report(config_path, legacy_toml_path, legacy_json_path)?;
    let mut state =
        collect_ccc_config_install_state_at(config_path, legacy_toml_path, legacy_json_path)?;
    state.restart_status = "restart-required";
    state.backup_status = apply_report.backup_status;
    state.backup_source_path = apply_report.backup_source_path;
    state.backup_path = apply_report.backup_path;

    if let Some(before) = before {
        if created && before.status == "legacy-only" {
            state.status = "migrated-from-previous";
            state.action_status = "migrated-from-previous";
            state.summary = format!(
                "CCC config was migrated from {} to the canonical path {}; backup_status={}; restart Codex CLI to use the refreshed install surface.",
                before
                    .source_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "a legacy source".to_string()),
                config_path.display(),
                state.backup_status
            );
        } else if created && before.status == "missing" {
            state.status = "created";
            state.action_status = "created";
            state.backup_status = "not-required";
            state.summary = format!(
                "CCC config was created at {}; restart Codex CLI to use the refreshed install surface.",
                config_path.display()
            );
        } else if !created && before.value != state.value {
            state.status = "canonical-current";
            state.action_status = "backfilled";
            state.summary = format!(
                "Canonical CCC config at {} was backfilled with missing defaults after creating a timestamped backup; restart Codex CLI to use the refreshed install surface.",
                config_path.display()
            );
        } else if !created && before.status == "conflict" {
            state.status = "conflict";
            state.action_status = "preserved";
            state.backup_status = "not-required";
            state.backup_source_path = None;
            state.backup_path = None;
            state.summary = format!(
                "Canonical CCC config at {} was preserved because a legacy migration source conflicts with it; restart Codex CLI if setup changed registration or skill files.",
                config_path.display()
            );
        } else if !created {
            state.status = "canonical-current";
            state.action_status = "preserved";
            state.backup_status = "not-required";
            state.backup_source_path = None;
            state.backup_path = None;
            state.summary = format!(
                "Canonical CCC config at {} was already current; restart Codex CLI if setup changed registration or skill files.",
                config_path.display()
            );
        }
    }

    Ok((created_path, created, state))
}

pub(crate) fn ensure_ccc_config_file() -> io::Result<(PathBuf, bool)> {
    let config_path = resolve_shared_config_path();
    let legacy_toml_path = resolve_legacy_shared_toml_config_path();
    let legacy_json_path = resolve_legacy_shared_json_config_path();
    ensure_ccc_config_file_at(&config_path, &legacy_toml_path, &legacy_json_path)
}

pub(crate) fn ensure_ccc_config_file_with_state(
) -> io::Result<(PathBuf, bool, CccConfigInstallState)> {
    let config_path = resolve_shared_config_path();
    let legacy_toml_path = resolve_legacy_shared_toml_config_path();
    let legacy_json_path = resolve_legacy_shared_json_config_path();
    ensure_ccc_config_file_at_with_state(&config_path, &legacy_toml_path, &legacy_json_path)
}
