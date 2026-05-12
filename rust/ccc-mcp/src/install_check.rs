use crate::execution_contract::create_execution_contract_registry_from_config;
use crate::graph_context::create_graph_context_readiness_payload;
use crate::setup_config::{
    collect_ccc_config_install_state_at, CccConfigInstallState, CURRENT_GENERATED_DEFAULTS_VERSION,
};
use crate::skill_registry::load_skill_registry_for_agent;
use crate::specialist_roles::{
    create_configured_role_models_payload_from_config, inspect_generated_custom_agents_from_config,
};
use crate::{
    resolve_codex_home, resolve_legacy_shared_json_config_path,
    resolve_legacy_shared_json_config_path_for, resolve_legacy_shared_toml_config_path,
    resolve_legacy_shared_toml_config_path_for, resolve_shared_config_path, SessionContext,
    PUBLIC_ENTRY_LABEL, PUBLIC_ENTRY_SKILL_NAME, SERVER_NAME,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const MANAGED_SKILL_REGISTRY_AGENTS: [&str; 8] = [
    "ccc_tactician",
    "ccc_scout",
    "ccc_raider",
    "ccc_scribe",
    "ccc_arbiter",
    "ccc_sentinel",
    "ccc_companion_reader",
    "ccc_companion_operator",
];

#[derive(Clone, Debug, Deserialize)]
struct CodexMcpRegistryRecord {
    name: String,
    enabled: bool,
    transport: Option<CodexMcpRegistryTransport>,
}

#[derive(Clone, Debug, Deserialize)]
struct CodexMcpRegistryTransport {
    #[serde(rename = "type")]
    transport_type: String,
    command: Option<String>,
    args: Option<Vec<String>>,
}

pub(crate) fn create_server_identity_payload(session_context: &SessionContext) -> Value {
    json!({
        "server_name": SERVER_NAME,
        "server_version": env!("CARGO_PKG_VERSION"),
        "session_id": session_context.session_id,
        "process_id": session_context.process_id,
        "started_at": session_context.started_at,
        "build_identity": session_context.build_identity,
        "entrypoint_path": session_context.entrypoint_path,
        "shared_config_path": session_context.shared_config_path,
    })
}

fn normalize_registered_command(command: &str) -> String {
    let candidate = PathBuf::from(command);
    if candidate.is_absolute() {
        fs::canonicalize(&candidate)
            .unwrap_or(candidate)
            .to_string_lossy()
            .into_owned()
    } else {
        command.to_string()
    }
}

fn resolve_expected_launch_command() -> io::Result<(String, Vec<String>)> {
    let current_exe = env::current_exe()?;
    let normalized = fs::canonicalize(&current_exe).unwrap_or(current_exe);
    Ok((
        normalized.to_string_lossy().into_owned(),
        vec!["mcp".to_string()],
    ))
}

fn resolve_plugin_cache_launch_command() -> io::Result<String> {
    resolve_plugin_cache_launch_command_at(&resolve_codex_home()?)
}

fn resolve_plugin_cache_launch_command_at(codex_home: &Path) -> io::Result<String> {
    let command = codex_home
        .join("plugins")
        .join("cache")
        .join("ccc-local")
        .join("ccc")
        .join(env!("CARGO_PKG_VERSION"))
        .join("bin")
        .join("ccc");
    if !command.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "CCC plugin-cache MCP binary is not installed at {}.",
                command.display()
            ),
        ));
    }
    let normalized = fs::canonicalize(&command).unwrap_or(command);
    Ok(normalized.to_string_lossy().into_owned())
}

fn resolve_accepted_launch_commands(expected_command: &str) -> Vec<String> {
    let mut commands = vec![expected_command.to_string()];
    if let Ok(plugin_cache_command) = resolve_plugin_cache_launch_command() {
        if !commands.contains(&plugin_cache_command) {
            commands.push(plugin_cache_command);
        }
    }
    commands
}

fn read_codex_mcp_registry() -> io::Result<Vec<CodexMcpRegistryRecord>> {
    let output = Command::new("codex")
        .arg("mcp")
        .arg("list")
        .arg("--json")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "codex mcp list --json failed with {}. {}",
                output.status,
                if stderr.is_empty() {
                    "No stderr output.".to_string()
                } else {
                    stderr
                }
            ),
        ));
    }

    serde_json::from_slice::<Vec<CodexMcpRegistryRecord>>(&output.stdout).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unable to parse codex mcp list --json output: {error}"),
        )
    })
}

fn find_ccc_registration(records: &[CodexMcpRegistryRecord]) -> Option<&CodexMcpRegistryRecord> {
    records.iter().find(|record| record.name == SERVER_NAME)
}

fn registration_matches_expected(
    record: &CodexMcpRegistryRecord,
    expected_command: &str,
    expected_args: &[String],
) -> bool {
    let accepted_commands = resolve_accepted_launch_commands(expected_command);
    registration_matches_any_expected(record, &accepted_commands, expected_args)
}

fn registration_matches_any_expected(
    record: &CodexMcpRegistryRecord,
    accepted_commands: &[String],
    expected_args: &[String],
) -> bool {
    if !record.enabled {
        return false;
    }

    let Some(transport) = record.transport.as_ref() else {
        return false;
    };

    if transport.transport_type != "stdio" {
        return false;
    }

    let Some(command) = transport.command.as_ref() else {
        return false;
    };

    accepted_commands.contains(&normalize_registered_command(command))
        && transport.args.clone().unwrap_or_default() == expected_args
}

fn resolve_cap_skill_install_path() -> io::Result<PathBuf> {
    Ok(resolve_codex_home()?
        .join("plugins")
        .join("cache")
        .join("ccc-local")
        .join("ccc")
        .join(env!("CARGO_PKG_VERSION"))
        .join("skills")
        .join(PUBLIC_ENTRY_SKILL_NAME)
        .join("SKILL.md"))
}

fn resolve_packaged_cap_skill_source() -> io::Result<PathBuf> {
    let current_exe = env::current_exe()?;
    let current_exe = fs::canonicalize(&current_exe).unwrap_or(current_exe);
    let mut candidates = Vec::new();

    if let Some(parent) = current_exe.parent() {
        candidates.push(
            parent
                .join("share")
                .join("skills")
                .join(PUBLIC_ENTRY_SKILL_NAME)
                .join("SKILL.md"),
        );
        if let Some(grandparent) = parent.parent() {
            candidates.push(
                grandparent
                    .join("share")
                    .join("skills")
                    .join(PUBLIC_ENTRY_SKILL_NAME)
                    .join("SKILL.md"),
            );
        }

        for ancestor in parent.ancestors().take(6) {
            candidates.push(
                ancestor
                    .join("skills")
                    .join(PUBLIC_ENTRY_SKILL_NAME)
                    .join("SKILL.md"),
            );
        }
    }

    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../skills")
            .join(PUBLIC_ENTRY_SKILL_NAME)
            .join("SKILL.md"),
    );

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Unable to locate the packaged $cap skill next to the Rust binary or source tree.",
    ))
}

pub(crate) fn install_packaged_cap_skill() -> io::Result<(PathBuf, bool)> {
    let source = resolve_packaged_cap_skill_source()?;
    let destination = resolve_cap_skill_install_path()?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let destination_exists = destination.exists();
    fs::copy(source, &destination)?;
    Ok((destination, !destination_exists))
}

pub(crate) fn inspect_packaged_cap_skill_install_at(
    source_path: &Path,
    install_path: &Path,
) -> io::Result<Value> {
    let source_contents = fs::read_to_string(source_path)?;
    let installed_contents = match fs::read_to_string(install_path) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    let (status, action_status, restart_status, summary) = match installed_contents {
        Some(contents) if contents == source_contents => (
            "matching_install",
            "preserved",
            "not-required",
            format!(
                "The packaged $cap skill is current under {}.",
                install_path.display()
            ),
        ),
        Some(_) => (
            "mismatched_install",
            "setup-refresh-available",
            "restart-required-after-setup",
            format!(
                "The packaged $cap skill under {} differs from {}; run setup and restart Codex CLI.",
                install_path.display(),
                source_path.display()
            ),
        ),
        None => (
            "missing_install",
            "setup-install-available",
            "restart-required-after-setup",
            format!(
                "The packaged $cap skill is missing under {}; run setup and restart Codex CLI.",
                install_path.display()
            ),
        ),
    };

    Ok(json!({
        "status": status,
        "action_status": action_status,
        "restart_status": restart_status,
        "summary": summary,
        "path": install_path.to_string_lossy(),
        "source_path": source_path.to_string_lossy(),
    }))
}

fn inspect_packaged_cap_skill_install() -> Value {
    let install_path = match resolve_cap_skill_install_path() {
        Ok(path) => path,
        Err(error) => {
            return json!({
                "status": "unreadable_install",
                "action_status": "inspection-blocked",
                "restart_status": "unknown",
                "summary": error.to_string(),
                "path": Value::Null,
                "source_path": Value::Null,
            });
        }
    };
    let source_path = match resolve_packaged_cap_skill_source() {
        Ok(path) => path,
        Err(error) => {
            return json!({
                "status": "packaged-source-unavailable",
                "action_status": "blocked",
                "restart_status": "unknown",
                "summary": error.to_string(),
                "path": install_path.to_string_lossy(),
                "source_path": Value::Null,
            });
        }
    };

    inspect_packaged_cap_skill_install_at(&source_path, &install_path).unwrap_or_else(|error| {
        json!({
            "status": "unreadable_install",
            "action_status": "inspection-blocked",
            "restart_status": "unknown",
            "summary": error.to_string(),
            "path": install_path.to_string_lossy(),
            "source_path": source_path.to_string_lossy(),
        })
    })
}

pub(crate) fn ensure_matching_mcp_registration() -> io::Result<&'static str> {
    let (expected_command, expected_args) = resolve_expected_launch_command()?;
    let records = read_codex_mcp_registry()?;

    if let Some(record) = find_ccc_registration(&records) {
        if registration_matches_expected(record, &expected_command, &expected_args) {
            return Ok("already_registered");
        }

        let remove_status = Command::new("codex")
            .arg("mcp")
            .arg("remove")
            .arg(SERVER_NAME)
            .status()?;
        if !remove_status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to remove the existing {SERVER_NAME} MCP registration."),
            ));
        }
    }

    let mut add_command = Command::new("codex");
    add_command
        .arg("mcp")
        .arg("add")
        .arg(SERVER_NAME)
        .arg("--")
        .arg(&expected_command);
    for arg in &expected_args {
        add_command.arg(arg);
    }
    let add_status = add_command.status()?;
    if !add_status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to add the {SERVER_NAME} MCP registration."),
        ));
    }

    Ok("registered")
}

pub(crate) fn create_registration_visibility_payload(status: &str, summary: &str) -> Value {
    let (surface_status, action_status, restart_status) = match status {
        "matching_registration" => ("current", "preserved", "not-required"),
        "missing_registration" => (
            "missing",
            "setup-register-available",
            "restart-required-after-setup",
        ),
        "mismatched_registration" => (
            "stale",
            "setup-refresh-available",
            "restart-required-after-setup",
        ),
        "unreadable_registration" => ("unreadable", "inspection-blocked", "unknown"),
        _ => ("unknown", "inspection-blocked", "unknown"),
    };

    json!({
        "status": surface_status,
        "raw_status": status,
        "action_status": action_status,
        "restart_status": restart_status,
        "summary": summary,
    })
}

pub(crate) fn create_config_visibility_payload(state: &CccConfigInstallState) -> Value {
    let surface_status = match state.status {
        "canonical-current" => "current",
        "canonical-needs-backfill" => "stale",
        "legacy-only" | "missing" => "missing",
        "migrated-from-previous" | "created" | "rollback-restored" => "migrated",
        "conflict" => "conflict",
        "unreadable" => "unreadable",
        _ => "unknown",
    };

    json!({
        "status": surface_status,
        "raw_status": state.status,
        "action_status": state.action_status,
        "backup_status": state.backup_status,
        "restart_status": state.restart_status,
        "summary": state.summary.clone(),
        "source_path": state.source_path_value(),
        "backup_source_path": state.backup_source_path_value(),
        "backup_path": state.backup_path_value(),
        "entry_policy_mode_status": state.entry_policy_mode_status,
        "entry_policy_mode_raw": state.entry_policy_mode_raw_value(),
        "entry_policy_mode_canonical": state.entry_policy_mode_canonical_value(),
        "entry_policy_mode_summary": state.entry_policy_mode_summary.clone(),
    })
}

pub(crate) fn create_custom_agent_visibility_payload(sync: &Value) -> Value {
    let raw_status = sync
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let (surface_status, action_status, restart_status) = match raw_status {
        "matching_sync" => ("current", "preserved", "not-required"),
        "missing_sync" => (
            "missing",
            "setup-sync-available",
            "restart-required-after-setup",
        ),
        "mismatched_sync" => (
            "stale",
            "setup-sync-available",
            "restart-required-after-setup",
        ),
        "unreadable_sync" => ("unreadable", "inspection-blocked", "unknown"),
        _ => ("unknown", "inspection-blocked", "unknown"),
    };

    json!({
        "status": surface_status,
        "raw_status": raw_status,
        "action_status": action_status,
        "restart_status": restart_status,
        "summary": sync.get("summary").cloned().unwrap_or(Value::Null),
        "directory_path": sync.get("directory_path").cloned().unwrap_or(Value::Null),
        "missing_files": sync.get("missing_files").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "mismatched_files": sync.get("mismatched_files").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "stale_managed_files": sync.get("stale_managed_files").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    })
}

pub(crate) fn create_skill_visibility_payload(skill: &Value) -> Value {
    let raw_status = skill
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let surface_status = match raw_status {
        "matching_install" => "current",
        "missing_install" => "missing",
        "mismatched_install" => "stale",
        "packaged-source-unavailable" | "unreadable_install" => "unreadable",
        _ => "unknown",
    };

    json!({
        "status": surface_status,
        "raw_status": raw_status,
        "action_status": skill.get("action_status").cloned().unwrap_or(Value::String("inspection-blocked".to_string())),
        "restart_status": skill.get("restart_status").cloned().unwrap_or(Value::String("unknown".to_string())),
        "summary": skill.get("summary").cloned().unwrap_or(Value::Null),
        "path": skill.get("path").cloned().unwrap_or(Value::Null),
        "source_path": skill.get("source_path").cloned().unwrap_or(Value::Null),
    })
}

fn create_skill_registry_health_payload() -> Value {
    let agents = MANAGED_SKILL_REGISTRY_AGENTS
        .iter()
        .map(|agent_name| {
            let registry = load_skill_registry_for_agent(agent_name, &json!({}));
            json!({
                "agent_name": agent_name,
                "status": registry.get("status").cloned().unwrap_or(Value::String("missing".to_string())),
                "manifest_status": registry.get("manifest_status").cloned().unwrap_or(Value::String("missing".to_string())),
                "path": registry.pointer("/skill_ssl_manifest/path").cloned().unwrap_or(Value::Null),
                "reason": registry.pointer("/skill_ssl_manifest/reason").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let available_count = agents
        .iter()
        .filter(|agent| agent.get("status").and_then(Value::as_str) == Some("available"))
        .count();
    let non_available = agents
        .iter()
        .filter(|agent| agent.get("status").and_then(Value::as_str) != Some("available"))
        .cloned()
        .collect::<Vec<_>>();

    // Registry health is a release-readiness signal. Missing or drifted
    // manifests stay non-blocking at runtime, but check-install should show them.
    json!({
        "schema": "ccc.skill_registry_health.v1",
        "status": if non_available.is_empty() { "ok" } else { "warning" },
        "agent_count": agents.len(),
        "available_count": available_count,
        "non_available_count": non_available.len(),
        "agents": agents,
        "non_available": non_available,
        "summary": if non_available.is_empty() {
            "All managed custom-agent SSL manifests are available."
        } else {
            "One or more managed custom-agent SSL manifests are missing, stale, invalid, or drifted."
        },
    })
}

fn surface_payload(id: &str, status: &str, summary: impl Into<String>, source: &str) -> Value {
    json!({
        "surface": id,
        "status": status,
        "source": source,
        "summary": summary.into(),
    })
}

fn optional_missing_surface_payload(id: &str, summary: impl Into<String>, source: &str) -> Value {
    surface_payload(id, "optional_missing", summary, source)
}

fn create_config_surface_readiness_payload(
    config_state: &CccConfigInstallState,
    execution_contract_registry: &Value,
    custom_agent_sync: &Value,
) -> Value {
    let config = &config_state.value;
    let canonical_ready = config_state.canonical_ready;
    let runtime = config.get("runtime").and_then(Value::as_object);
    let canonical_missing = |id| {
        surface_payload(
            id,
            "missing",
            "Canonical ccc-config.toml is not ready; inspect setup migration guidance.",
            "ccc_config",
        )
    };
    let valid_mode = |mode| {
        matches!(
            mode,
            "codex_subagent" | "codex_exec" | "visible_degraded_host_fallback"
        )
    };
    let positive = |value: Option<&Value>| {
        value.map(|value| {
            value.as_u64().filter(|number| *number > 0).is_some()
                || value.as_i64().filter(|number| *number > 0).is_some()
        })
    };

    let role_count = execution_contract_registry
        .get("role_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let registry = if execution_contract_registry
        .get("status")
        .and_then(Value::as_str)
        == Some("available")
        && role_count == 8
    {
        surface_payload(
            "registry",
            "current",
            "Execution contract registry covers all managed ccc_* roles.",
            "execution_contract_registry",
        )
    } else if role_count > 0 {
        surface_payload(
            "registry",
            "stale",
            format!("Execution contract registry is partial: roles={role_count}/8."),
            "execution_contract_registry",
        )
    } else {
        surface_payload(
            "registry",
            "missing",
            "Execution contract registry is unavailable.",
            "execution_contract_registry",
        )
    };

    let category_routing = match (
        canonical_ready,
        config.get("routing").and_then(Value::as_object),
    ) {
        (false, _) => canonical_missing("category_routing"),
        (true, None) => surface_payload(
            "category_routing",
            "missing",
            "Routing taxonomy is absent; runtime will use Rust defaults.",
            "ccc_config.routing",
        ),
        (true, Some(routing))
            if routing.get("mode").and_then(Value::as_str) != Some("category_shortlist") =>
        {
            surface_payload(
                "category_routing",
                "conflict",
                "Routing mode is unsupported for category shortlist routing.",
                "ccc_config.routing.mode",
            )
        }
        (true, Some(routing)) => {
            let category_count = routing
                .get("categories")
                .and_then(Value::as_object)
                .map(|categories| categories.len())
                .unwrap_or(0);
            if category_count == 0 {
                surface_payload(
                    "category_routing",
                    "conflict",
                    "Routing categories are missing or empty.",
                    "ccc_config.routing.categories",
                )
            } else {
                surface_payload(
                    "category_routing",
                    "current",
                    format!("Category routing is configured with {category_count} categories."),
                    "ccc_config.routing",
                )
            }
        }
    };

    let fallback_policy = match (canonical_ready, runtime) {
        (false, _) => canonical_missing("fallback_policy"),
        (true, None) => surface_payload(
            "fallback_policy",
            "missing",
            "Runtime fallback policy is absent; runtime will use Rust defaults.",
            "ccc_config.runtime",
        ),
        (true, Some(runtime)) => {
            let preferred = runtime
                .get("preferred_specialist_execution_mode")
                .and_then(Value::as_str);
            let fallback = runtime
                .get("fallback_specialist_execution_mode")
                .and_then(Value::as_str);
            match (preferred, fallback) {
                (Some(preferred), Some(fallback))
                    if valid_mode(preferred) && valid_mode(fallback) =>
                {
                    if fallback == "visible_degraded_host_fallback" {
                        surface_payload(
                            "fallback_policy",
                            "stale",
                            "Fallback policy uses the legacy visible degraded host fallback alias.",
                            "ccc_config.runtime.fallback_specialist_execution_mode",
                        )
                    } else {
                        surface_payload(
                            "fallback_policy",
                            "current",
                            format!(
                                "Fallback policy is configured: preferred={preferred} fallback={fallback}."
                            ),
                            "ccc_config.runtime",
                        )
                    }
                }
                (Some(_), Some(_)) => surface_payload(
                    "fallback_policy",
                    "conflict",
                    "Fallback policy contains an unsupported execution mode.",
                    "ccc_config.runtime",
                ),
                _ => surface_payload(
                    "fallback_policy",
                    "missing",
                    "Fallback policy is missing preferred or fallback execution mode.",
                    "ccc_config.runtime",
                ),
            }
        }
    };

    let concurrency = match (canonical_ready, runtime) {
        (false, _) => canonical_missing("concurrency"),
        (true, None) => optional_missing_surface_payload(
            "concurrency",
            "Optional host subagent concurrency settings are absent; runtime will use Rust defaults.",
            "ccc_config.runtime",
        ),
        (true, Some(runtime)) => match runtime.get("host_subagent_concurrency") {
            Some(value) if !value.is_object() => surface_payload(
                "concurrency",
                "conflict",
                "host_subagent_concurrency must be an object.",
                "ccc_config.runtime.host_subagent_concurrency",
            ),
            Some(value) => {
                let nested = value.as_object().expect("checked object");
                let invalid_limit = [
                    "default_provider_concurrency_limit",
                    "default_model_concurrency_limit",
                ]
                .iter()
                .any(|key| positive(nested.get(*key)) == Some(false));
                if invalid_limit {
                    surface_payload(
                        "concurrency",
                        "conflict",
                        "Concurrency default limits must be positive integers when set.",
                        "ccc_config.runtime.host_subagent_concurrency",
                    )
                } else {
                    surface_payload(
                        "concurrency",
                        "current",
                        "Host subagent concurrency is nested under runtime.host_subagent_concurrency.",
                        "ccc_config.runtime.host_subagent_concurrency",
                    )
                }
            }
            None => {
                let has_flat_keys = [
                    "host_subagent_default_provider_concurrency_limit",
                    "default_provider_concurrency_limit",
                    "host_subagent_default_model_concurrency_limit",
                    "default_model_concurrency_limit",
                    "host_subagent_provider_concurrency_limits",
                    "provider_concurrency_limits",
                    "host_subagent_model_concurrency_limits",
                    "model_concurrency_limits",
                ]
                .iter()
                .any(|key| runtime.contains_key(*key));
                surface_payload(
                    "concurrency",
                    if has_flat_keys {
                        "stale"
                    } else {
                        "optional_missing"
                    },
                    if has_flat_keys {
                        "Host subagent concurrency uses legacy flat runtime keys."
                    } else {
                        "Optional host subagent concurrency settings are absent; runtime will use Rust defaults."
                    },
                    "ccc_config.runtime.host_subagent_concurrency",
                )
            }
        },
    };

    let prompt_sections = match (
        canonical_ready,
        config.get("prompt_sections").and_then(Value::as_object),
    ) {
        (false, _) => canonical_missing("prompt_sections"),
        (true, None) => optional_missing_surface_payload(
            "prompt_sections",
            "Named prompt sections are not configured; prompt composition will use Rust defaults.",
            "ccc_config.prompt_sections",
        ),
        (true, Some(sections)) => {
            let required = [
                "identity",
                "task",
                "routing",
                "hard_blocks",
                "evidence",
                "verification",
                "anti_duplication",
                "reporting",
            ];
            let missing = required
                .iter()
                .filter(|section| !sections.contains_key(**section))
                .copied()
                .collect::<Vec<_>>();
            if missing.is_empty() {
                surface_payload(
                    "prompt_sections",
                    "current",
                    "Named prompt sections cover the 0.0.15 prompt-composition contract.",
                    "ccc_config.prompt_sections",
                )
            } else {
                surface_payload(
                    "prompt_sections",
                    "stale",
                    format!(
                        "Named prompt sections are incomplete: missing={}.",
                        missing.join(",")
                    ),
                    "ccc_config.prompt_sections",
                )
            }
        }
    };

    let directory_rule_injection = match (
        canonical_ready,
        config
            .get("directory_rule_injection")
            .or_else(|| config.get("directory_rules")),
    ) {
        (false, _) => canonical_missing("directory_rule_injection"),
        (true, None) => optional_missing_surface_payload(
            "directory_rule_injection",
            "Directory-rule injection settings are absent; runtime will use Rust defaults.",
            "ccc_config.directory_rule_injection",
        ),
        (true, Some(value)) if !value.is_object() => surface_payload(
            "directory_rule_injection",
            "conflict",
            "Directory-rule injection settings must be an object.",
            "ccc_config.directory_rule_injection",
        ),
        (true, Some(value)) => {
            if value.get("enabled").and_then(Value::as_bool).is_some() {
                surface_payload(
                    "directory_rule_injection",
                    "current",
                    "Directory-rule injection settings are configured.",
                    "ccc_config.directory_rule_injection",
                )
            } else {
                surface_payload(
                    "directory_rule_injection",
                    "stale",
                    "Directory-rule injection settings are missing an enabled flag.",
                    "ccc_config.directory_rule_injection.enabled",
                )
            }
        }
    };

    let hook_settings = match (
        canonical_ready,
        runtime.and_then(|runtime| runtime.get("lifecycle_hooks")),
    ) {
        (false, _) => canonical_missing("hook_settings"),
        (true, None) => optional_missing_surface_payload(
            "hook_settings",
            "Lifecycle hook settings are absent; runtime will use Rust defaults.",
            "ccc_config.runtime.lifecycle_hooks",
        ),
        (true, Some(value)) if !value.is_object() => surface_payload(
            "hook_settings",
            "conflict",
            "lifecycle_hooks must be an object.",
            "ccc_config.runtime.lifecycle_hooks",
        ),
        (true, Some(value)) => {
            let has_command = value
                .as_object()
                .expect("checked object")
                .values()
                .any(|tier| tier.get("command").is_some());
            surface_payload(
                "hook_settings",
                if has_command { "conflict" } else { "current" },
                if has_command {
                    "Lifecycle hooks cannot define user-facing hook commands in 0.0.15."
                } else {
                    "Lifecycle hook settings are configured without public hook commands."
                },
                "ccc_config.runtime.lifecycle_hooks",
            )
        }
    };

    let custom_agent_sync = match custom_agent_sync
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
    {
        "matching_sync" => surface_payload(
            "custom_agent_sync",
            "current",
            "CCC-managed custom agents match generated config.",
            "custom_agent_sync",
        ),
        "missing_sync" => surface_payload(
            "custom_agent_sync",
            "missing",
            "One or more CCC-managed custom-agent files are missing.",
            "custom_agent_sync",
        ),
        "mismatched_sync" => surface_payload(
            "custom_agent_sync",
            "stale",
            "One or more CCC-managed custom-agent files are stale or extra.",
            "custom_agent_sync",
        ),
        _ => surface_payload(
            "custom_agent_sync",
            "conflict",
            "Custom-agent sync state is unreadable or unavailable.",
            "custom_agent_sync",
        ),
    };

    let generated_version = config
        .pointer("/generated_defaults/version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let generated_defaults = if !canonical_ready {
        surface_payload(
            "generated_defaults_version",
            "missing",
            "Generated-defaults version is unavailable until ccc-config.toml is migrated.",
            "ccc_config.generated_defaults",
        )
    } else if generated_version < CURRENT_GENERATED_DEFAULTS_VERSION {
        surface_payload(
            "generated_defaults_version",
            "stale",
            format!(
                "Generated defaults version is {generated_version}; expected {}.",
                CURRENT_GENERATED_DEFAULTS_VERSION
            ),
            "ccc_config.generated_defaults.version",
        )
    } else {
        surface_payload(
            "generated_defaults_version",
            "current",
            format!("Generated defaults version is current: {generated_version}."),
            "ccc_config.generated_defaults.version",
        )
    };
    let surfaces = vec![
        registry,
        category_routing,
        fallback_policy,
        concurrency,
        prompt_sections,
        directory_rule_injection,
        hook_settings,
        custom_agent_sync,
        generated_defaults.clone(),
    ];
    let mut missing_count = 0;
    let mut optional_missing_count = 0;
    let mut stale_count = 0;
    let mut conflict_count = 0;
    for surface in &surfaces {
        match surface
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
        {
            "missing" => missing_count += 1,
            "optional_missing" => optional_missing_count += 1,
            "stale" => stale_count += 1,
            "conflict" | "unknown" => conflict_count += 1,
            _ => {}
        }
    }
    let status = if conflict_count > 0 {
        "conflict"
    } else if stale_count > 0 {
        "stale"
    } else if missing_count > 0 {
        "missing"
    } else {
        "current"
    };
    let setup_guidance = json!({
        "dry_run": "ccc setup --dry-run",
        "apply": "ccc setup",
        "backup": config_state.backup_status,
        "backup_path": config_state.backup_path_value(),
        "rollback": "ccc setup --rollback-config <backup_path>",
        "restart": config_state.restart_status,
        "preservation": "setup preserves user-owned config values while backfilling generated defaults",
    });

    // Aggregate readiness is separate from runtime truth: defaults may keep CCC
    // operational even when the operator-facing config surface is missing/stale.
    json!({
        "schema": "ccc.config_surface_readiness.v1",
        "status": status,
        "surface_count": surfaces.len(),
        "missing_count": missing_count,
        "optional_missing_count": optional_missing_count,
        "stale_count": stale_count,
        "conflict_count": conflict_count,
        "surfaces": surfaces,
        "generated_defaults": generated_defaults,
        "setup_guidance": setup_guidance,
        "summary": if status == "current" && optional_missing_count == 0 {
            "All 0.0.15 config/check-install surfaces are current."
        } else if status == "current" {
            "Required 0.0.15 config/check-install surfaces are current; optional deferred surfaces are absent and covered by Rust defaults."
        } else {
            "One or more 0.0.15 config/check-install surfaces are missing, stale, or conflicting."
        },
    })
}

pub(crate) fn create_check_install_status(
    registration_status: &str,
    config_install_state: &CccConfigInstallState,
    cap_skill_status: &str,
    custom_agent_status: &str,
    config_surface_readiness: &Value,
) -> &'static str {
    let config_surface_current = config_surface_readiness
        .get("status")
        .and_then(Value::as_str)
        == Some("current");
    if registration_status == "matching_registration"
        && config_install_state.canonical_ready
        && config_install_state.status == "canonical-current"
        && cap_skill_status == "matching_install"
        && custom_agent_status == "matching_sync"
        && config_surface_current
    {
        "ok"
    } else {
        "warning"
    }
}

pub(crate) fn create_graph_context_check_install_readiness_payload(
    config: &Value,
    workspace_root: &Path,
) -> Value {
    match create_graph_context_readiness_payload(config, workspace_root) {
        Ok(payload) => annotate_graph_context_check_install_readiness(payload, workspace_root),
        Err(error) => json!({
            "schema": "ccc.graph_context_readiness.check_install.v1",
            "provider": "graphify",
            "readiness": "unavailable",
            "reason": "inspection_error",
            "fallback_when_unavailable": "scout_source_evidence",
            "fallback": "scout_source_evidence",
            "check_install_status": "warning",
            "check_install_blocking": false,
            "workspace_root": workspace_root.to_string_lossy(),
            "summary": format!(
                "Graphify graph_context readiness could not be inspected: {error}."
            ),
        }),
    }
}

fn annotate_graph_context_check_install_readiness(
    mut payload: Value,
    workspace_root: &Path,
) -> Value {
    let readiness = payload
        .get("readiness")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let check_install_status = match readiness {
        "available" => "ok",
        "disabled" => "disabled",
        _ => "warning",
    };
    let fallback = payload
        .get("fallback")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("fallback_when_unavailable")
                .and_then(Value::as_str)
        })
        .unwrap_or("scout_source_evidence");
    let reason = payload
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let summary = match readiness {
        "available" => "Graphify graph_context artifacts are available.".to_string(),
        "disabled" => {
            format!("Graphify graph_context is disabled; fallback={fallback} remains active.")
        }
        "stale" => format!("Graphify graph_context artifacts are stale; fallback={fallback}."),
        _ => format!("Graphify graph_context is unavailable ({reason}); fallback={fallback}."),
    };

    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "schema".to_string(),
            Value::String("ccc.graph_context_readiness.check_install.v1".to_string()),
        );
        object.insert(
            "check_install_status".to_string(),
            Value::String(check_install_status.to_string()),
        );
        object.insert("check_install_blocking".to_string(), Value::Bool(false));
        object.insert(
            "workspace_root".to_string(),
            Value::String(workspace_root.to_string_lossy().into_owned()),
        );
        object.insert("summary".to_string(), Value::String(summary));
    }
    payload
}

fn status_rank(status: &str) -> u8 {
    match status {
        "unreadable" | "conflict" | "unknown" => 4,
        "stale" => 3,
        "missing" => 2,
        "migrated" => 1,
        _ => 0,
    }
}

pub(crate) fn create_install_surface_visibility_payload(
    registration: Value,
    config: Value,
    skill: Value,
    custom_agents: Value,
) -> Value {
    let components = json!({
        "mcp_registration": registration,
        "ccc_config": config,
        "cap_skill": skill,
        "custom_agents": custom_agents,
    });
    let mut component_statuses = components
        .as_object()
        .into_iter()
        .flat_map(|object| object.iter())
        .filter_map(|(name, component)| {
            Some((
                name.to_string(),
                component.get("status").and_then(Value::as_str)?.to_string(),
                component
                    .get("restart_status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            ))
        })
        .collect::<Vec<_>>();
    component_statuses.sort_by(|left, right| {
        status_rank(&right.1)
            .cmp(&status_rank(&left.1))
            .then_with(|| left.0.cmp(&right.0))
    });
    let overall_status = component_statuses
        .first()
        .map(|(_, status, _)| status.as_str())
        .unwrap_or("unknown");
    let restart_required = component_statuses
        .iter()
        .any(|(_, _, restart_status)| restart_status.starts_with("restart-required"));
    let needs_setup = component_statuses
        .iter()
        .any(|(_, status, _)| matches!(status.as_str(), "missing" | "stale" | "migrated"));
    let summary = if overall_status == "current" && !restart_required {
        "Install surface is current; no setup or restart action is required.".to_string()
    } else if needs_setup {
        "Install surface needs setup refresh and Codex CLI restart before the host session sees all changes.".to_string()
    } else {
        "Install surface visibility has warnings; inspect component summaries before continuing."
            .to_string()
    };

    json!({
        "status": overall_status,
        "restart_required": restart_required,
        "setup_refresh_recommended": needs_setup,
        "summary": summary,
        "components": components,
    })
}

pub(crate) fn refresh_install_surface_config_visibility(
    payload: &mut Value,
    state: &CccConfigInstallState,
) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    let existing = object
        .get("installSurfaceVisibility")
        .cloned()
        .unwrap_or(Value::Null);
    let component = |name: &str| {
        existing
            .pointer(&format!("/components/{name}"))
            .cloned()
            .unwrap_or_else(|| {
                json!({
                    "status": "unknown",
                    "raw_status": "unknown",
                    "action_status": "inspection-blocked",
                    "restart_status": "unknown",
                    "summary": Value::Null,
                })
            })
    };
    object.insert(
        "installSurfaceVisibility".to_string(),
        create_install_surface_visibility_payload(
            component("mcp_registration"),
            create_config_visibility_payload(state),
            component("cap_skill"),
            component("custom_agents"),
        ),
    );
}

pub(crate) fn collect_install_check_payload_for_config_path(
    session_context: &SessionContext,
    preferred_config_path: PathBuf,
) -> Value {
    let workspace_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    collect_install_check_payload_for_config_path_at_workspace(
        session_context,
        preferred_config_path,
        &workspace_root,
    )
}

pub(crate) fn collect_install_check_payload_for_config_path_at_workspace(
    session_context: &SessionContext,
    preferred_config_path: PathBuf,
    workspace_root: &Path,
) -> Value {
    let legacy_toml_path = resolve_legacy_shared_toml_config_path_for(&preferred_config_path)
        .unwrap_or_else(resolve_legacy_shared_toml_config_path);
    let legacy_json_path = resolve_legacy_shared_json_config_path_for(&preferred_config_path)
        .unwrap_or_else(resolve_legacy_shared_json_config_path);
    let config_install_state = collect_ccc_config_install_state_at(
        &preferred_config_path,
        &legacy_toml_path,
        &legacy_json_path,
    )
    .unwrap_or_else(|error| CccConfigInstallState {
        status: "unreadable",
        action_status: "skipped",
        backup_status: "unavailable",
        summary: error.to_string(),
        source_path: Some(preferred_config_path.clone()),
        backup_source_path: None,
        backup_path: None,
        value: Value::Null,
        canonical_ready: false,
        config_exists: preferred_config_path.exists(),
        restart_status: "not-required",
        entry_policy_mode_status: "unavailable",
        entry_policy_mode_raw: None,
        entry_policy_mode_canonical: None,
        entry_policy_mode_summary:
            "Entry policy mode health is unavailable because the CCC config could not be read."
                .to_string(),
    });
    let config_exists = config_install_state.config_exists;
    let config_value = config_install_state.value.clone();
    let configured_role_models = create_configured_role_models_payload_from_config(&config_value);
    let execution_contract_registry = create_execution_contract_registry_from_config(&config_value);
    let custom_agent_sync = inspect_generated_custom_agents_from_config(&config_value)
        .unwrap_or_else(|error| {
            json!({
                "status": "unreadable_sync",
                "summary": error.to_string(),
                "directory_path": Value::Null,
                "generated_names": [],
                "generated_files": [],
                "file_count": 0,
                "missing_files": [],
                "mismatched_files": [],
                "stale_managed_files": [],
            })
        });
    let (expected_command, expected_args) =
        resolve_expected_launch_command().unwrap_or_else(|_| {
            (
                session_context.entrypoint_path.clone().unwrap_or_default(),
                vec!["mcp".to_string()],
            )
        });
    let registry_result = read_codex_mcp_registry();
    let (
        registration_status,
        registration_summary,
        registered_launch_command,
        registered_launch_args,
    ) = match registry_result {
        Ok(records) => {
            if let Some(record) = find_ccc_registration(&records) {
                let command = record
                    .transport
                    .as_ref()
                    .and_then(|transport| transport.command.clone());
                let args = record
                    .transport
                    .as_ref()
                    .and_then(|transport| transport.args.clone())
                    .unwrap_or_default();
                if registration_matches_expected(record, &expected_command, &expected_args) {
                    (
                        "matching_registration",
                        "Codex CLI MCP registration matches the local Rust entrypoint.".to_string(),
                        command.map(Value::String).unwrap_or(Value::Null),
                        Value::Array(args.into_iter().map(Value::String).collect()),
                    )
                } else {
                    (
                        "mismatched_registration",
                        format!(
                            "Codex CLI has a CCC MCP entry, but it does not point at this Rust {} command.",
                            env!("CARGO_PKG_VERSION")
                        ),
                        command.map(Value::String).unwrap_or(Value::Null),
                        Value::Array(args.into_iter().map(Value::String).collect()),
                    )
                }
            } else {
                (
                    "missing_registration",
                    "Codex CLI does not currently have a CCC MCP registration.".to_string(),
                    Value::Null,
                    Value::Array(Vec::new()),
                )
            }
        }
        Err(error) => (
            "unreadable_registration",
            error.to_string(),
            Value::Null,
            Value::Array(Vec::new()),
        ),
    };

    let packaged_skill_source_status = if resolve_packaged_cap_skill_source().is_ok() {
        "coherent_surface"
    } else {
        "incomplete_surface"
    };
    let cap_skill_visibility = inspect_packaged_cap_skill_install();
    let cap_skill_status = cap_skill_visibility
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unreadable_install");
    let cap_skill_summary = cap_skill_visibility
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("The packaged $cap skill install state is unavailable.");
    let registration_visibility =
        create_registration_visibility_payload(registration_status, &registration_summary);
    let config_visibility = create_config_visibility_payload(&config_install_state);
    let custom_agent_visibility = create_custom_agent_visibility_payload(&custom_agent_sync);
    let skill_visibility = create_skill_visibility_payload(&cap_skill_visibility);
    let skill_registry_health = create_skill_registry_health_payload();
    let config_surface_readiness = create_config_surface_readiness_payload(
        &config_install_state,
        &execution_contract_registry,
        &custom_agent_sync,
    );
    let graph_context_readiness =
        create_graph_context_check_install_readiness_payload(&config_value, workspace_root);
    let install_surface_visibility = create_install_surface_visibility_payload(
        registration_visibility,
        config_visibility,
        skill_visibility,
        custom_agent_visibility,
    );
    let custom_agent_status = custom_agent_sync
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let status = create_check_install_status(
        registration_status,
        &config_install_state,
        cap_skill_status,
        custom_agent_status,
        &config_surface_readiness,
    );

    json!({
        "status": status,
        "packageName": "ccc",
        "packageVersion": env!("CARGO_PKG_VERSION"),
        "publicEntrySkillName": PUBLIC_ENTRY_SKILL_NAME,
        "publicEntryLabel": PUBLIC_ENTRY_LABEL,
        "serverName": SERVER_NAME,
        "expectedLaunchCommand": expected_command,
        "expectedLaunchArgs": expected_args,
        "expectedEntrypointPath": session_context.entrypoint_path,
        "registrationStatus": registration_status,
        "registrationSummary": registration_summary,
        "registeredLaunchCommand": registered_launch_command,
        "registeredLaunchArgs": registered_launch_args,
        "registeredEntrypointPath": Value::Null,
        "configPath": preferred_config_path,
        "configExists": config_exists,
        "configCanonicalReady": config_install_state.canonical_ready,
        "configStatus": config_install_state.status,
        "configActionStatus": config_install_state.action_status,
        "configBackupStatus": config_install_state.backup_status,
        "configBackupSourcePath": config_install_state.backup_source_path_value(),
        "configBackupPath": config_install_state.backup_path_value(),
        "configSummary": config_install_state.summary,
        "configSourcePath": config_install_state.source_path_value(),
        "configRestartStatus": config_install_state.restart_status,
        "entryPolicyModeStatus": config_install_state.entry_policy_mode_status,
        "entryPolicyModeRaw": config_install_state.entry_policy_mode_raw_value(),
        "entryPolicyModeCanonical": config_install_state.entry_policy_mode_canonical_value(),
        "entryPolicyModeSummary": config_install_state.entry_policy_mode_summary,
        "registryInspectionStatus": "available",
        "registryInspectionSummary": "Codex CLI MCP registry was inspected successfully.",
        "otherInstalledMcpServers": [],
        "companionMcpUsageSummary": format!(
            "Companion MCP inspection is outside the Rust {} install contract.",
            env!("CARGO_PKG_VERSION")
        ),
        "notebookLmArchiveTargetStatus": "disabled",
        "notebookLmArchiveTargetSummary": format!(
            "NotebookLM archive inspection is not part of the Rust {} baseline.",
            env!("CARGO_PKG_VERSION")
        ),
        "packagedHarnessSurfaceStatus": packaged_skill_source_status,
        "packagedHarnessSurfaceSummary": if packaged_skill_source_status == "coherent_surface" {
            "The packaged Rust install surface includes a resolvable $cap skill asset."
        } else {
            "The Rust binary could not resolve its packaged $cap skill asset."
        },
        "capSkillStatus": cap_skill_status,
        "capSkillSummary": cap_skill_summary,
        "capSkillPath": cap_skill_visibility.get("path").cloned().unwrap_or(Value::Null),
        "capSkillActionStatus": cap_skill_visibility.get("action_status").cloned().unwrap_or(Value::String("inspection-blocked".to_string())),
        "capSkillRestartStatus": cap_skill_visibility.get("restart_status").cloned().unwrap_or(Value::String("unknown".to_string())),
        "capSkillSourcePath": cap_skill_visibility.get("source_path").cloned().unwrap_or(Value::Null),
        "customAgentStatus": custom_agent_sync.get("status").cloned().unwrap_or(Value::String("unavailable".to_string())),
        "customAgentSummary": custom_agent_sync.get("summary").cloned().unwrap_or(Value::String("Custom-agent sync state is unavailable.".to_string())),
        "customAgentActionStatus": install_surface_visibility.pointer("/components/custom_agents/action_status").cloned().unwrap_or(Value::String("inspection-blocked".to_string())),
        "customAgentRestartStatus": install_surface_visibility.pointer("/components/custom_agents/restart_status").cloned().unwrap_or(Value::String("unknown".to_string())),
        "customAgentDirectoryPath": custom_agent_sync.get("directory_path").cloned().unwrap_or(Value::Null),
        "customAgentNames": custom_agent_sync.get("generated_names").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "customAgentFileCount": custom_agent_sync.get("file_count").cloned().unwrap_or(Value::from(0)),
        "customAgentMissingFiles": custom_agent_sync.get("missing_files").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "customAgentMismatchedFiles": custom_agent_sync.get("mismatched_files").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "customAgentStaleManagedFiles": custom_agent_sync.get("stale_managed_files").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "skillRegistryHealth": skill_registry_health,
        "executionContractRegistry": execution_contract_registry,
        "configSurfaceReadiness": config_surface_readiness,
        "graphContextReadiness": graph_context_readiness,
        "modelPolicyStatus": if configured_role_models.is_empty() { "missing_config" } else { "configured" },
        "modelPolicySummary": if configured_role_models.is_empty() {
            "No shared role-model configuration was found."
        } else {
            "Shared role-model configuration was loaded from the CCC config."
        },
        "configuredRoleModels": configured_role_models,
        "toolRoutingPolicyStatus": "not_packaged",
        "toolRoutingPolicySummary": format!(
            "Companion tool-routing inspection is not part of the Rust {} install contract.",
            env!("CARGO_PKG_VERSION")
        ),
        "configuredToolRoutes": [],
        "activeRunHygieneStatus": "clean",
        "activeRunHygieneSummary": format!(
            "No run-hygiene issues are reported by the Rust {} installer surface.",
            env!("CARGO_PKG_VERSION")
        ),
        "installSurfaceVisibility": install_surface_visibility,
        "recommendedRunId": Value::Null,
        "session_registration_match": if registration_status == "matching_registration" { "matching" } else { "unknown" },
    })
}

pub(crate) fn collect_install_check_payload(session_context: &SessionContext) -> Value {
    collect_install_check_payload_for_config_path(session_context, resolve_shared_config_path())
}

fn config_surface_statuses_line(payload: &Value) -> Option<String> {
    let surfaces = payload
        .pointer("/configSurfaceReadiness/surfaces")
        .and_then(Value::as_array)?;
    let status_for = |surface_id: &str| {
        surfaces
            .iter()
            .find(|surface| surface.get("surface").and_then(Value::as_str) == Some(surface_id))
            .and_then(|surface| surface.get("status").and_then(Value::as_str))
            .unwrap_or("unknown")
    };
    Some(format!(
        "0.0.15 config surfaces: registry={} category_routing={} fallback_policy={} concurrency={} prompt_sections={} directory_rule_injection={} hook_settings={} custom_agent_sync={} generated_defaults={}",
        status_for("registry"),
        status_for("category_routing"),
        status_for("fallback_policy"),
        status_for("concurrency"),
        status_for("prompt_sections"),
        status_for("directory_rule_injection"),
        status_for("hook_settings"),
        status_for("custom_agent_sync"),
        status_for("generated_defaults_version"),
    ))
}

fn config_setup_guidance_line(payload: &Value) -> Option<String> {
    let guidance = payload.pointer("/configSurfaceReadiness/setup_guidance")?;
    let dry_run = guidance
        .get("dry_run")
        .and_then(Value::as_str)
        .unwrap_or("ccc setup --dry-run");
    let rollback = guidance
        .get("rollback")
        .and_then(Value::as_str)
        .unwrap_or("ccc setup --rollback-config <backup_path>");
    let backup = guidance
        .get("backup")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let restart = guidance
        .get("restart")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Some(format!(
        "Config setup safeguards: dry_run=\"{dry_run}\" backup={backup} rollback=\"{rollback}\" restart={restart}"
    ))
}

fn graph_context_readiness_line(payload: &Value) -> Option<String> {
    let readiness = payload
        .pointer("/graphContextReadiness/readiness")?
        .as_str()?;
    let check_install_status = payload
        .pointer("/graphContextReadiness/check_install_status")
        .and_then(Value::as_str)
        .unwrap_or("warning");
    let reason = payload
        .pointer("/graphContextReadiness/reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let fallback = payload
        .pointer("/graphContextReadiness/fallback")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .pointer("/graphContextReadiness/fallback_when_unavailable")
                .and_then(Value::as_str)
        })
        .unwrap_or("none");
    Some(format!(
        "Graph context: readiness={readiness} check_install={check_install_status} reason={reason} fallback={fallback}"
    ))
}

pub(crate) fn create_check_install_text(payload: &Value) -> String {
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("warning");
    let version = payload
        .get("packageVersion")
        .and_then(Value::as_str)
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    let entry = payload
        .get("publicEntryLabel")
        .and_then(Value::as_str)
        .unwrap_or(PUBLIC_ENTRY_LABEL);
    let registration = payload
        .get("registrationStatus")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let config = payload
        .get("configStatus")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            if payload
                .get("configExists")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "canonical-current"
            } else {
                "missing"
            }
        });
    let config_action = payload
        .get("configActionStatus")
        .and_then(Value::as_str)
        .unwrap_or("skipped");
    let config_restart = payload
        .get("configRestartStatus")
        .and_then(Value::as_str)
        .unwrap_or("not-required");
    let config_backup = payload
        .get("configBackupStatus")
        .and_then(Value::as_str)
        .unwrap_or("not-required");
    let skill = payload
        .get("capSkillStatus")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let surface = payload
        .pointer("/installSurfaceVisibility/status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let surface_restart = payload
        .pointer("/installSurfaceVisibility/restart_required")
        .and_then(Value::as_bool)
        .map(|required| if required { "required" } else { "not-required" })
        .unwrap_or("unknown");
    let custom_agents = payload
        .get("customAgentStatus")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let registry_health = payload
        .pointer("/skillRegistryHealth/status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let registry_available = payload
        .pointer("/skillRegistryHealth/available_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let registry_total = payload
        .pointer("/skillRegistryHealth/agent_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let contract_status = payload
        .pointer("/executionContractRegistry/status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let contract_count = payload
        .pointer("/executionContractRegistry/role_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let mut lines = vec![
        format!(
            "CCC install check: status={status} version={version} entry={entry} registration={registration} config={config} config_action={config_action} config_restart={config_restart} skill={skill}"
        ),
        format!(
            "Install surface: status={surface} restart={surface_restart} mcp={registration} skill={skill} custom_agents={custom_agents}"
        ),
        format!(
            "Skill registry: status={registry_health} available={registry_available}/{registry_total}"
        ),
        format!("Execution contracts: status={contract_status} roles={contract_count}"),
    ];
    if let Some(line) = config_surface_statuses_line(payload) {
        lines.push(line);
    }
    if let Some(line) = graph_context_readiness_line(payload) {
        lines.push(line);
    }
    lines.extend([
        format!(
            "Expected MCP launch: {} {}",
            payload
                .get("expectedLaunchCommand")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            payload
                .get("expectedLaunchArgs")
                .and_then(Value::as_array)
                .map(|values| values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" "))
                .unwrap_or_default()
        ),
        payload
            .get("registrationSummary")
            .and_then(Value::as_str)
            .unwrap_or("No registration summary available.")
            .to_string(),
        payload
            .get("configSummary")
            .and_then(Value::as_str)
            .unwrap_or("No config summary available.")
            .to_string(),
        format!("Config backup: status={config_backup}"),
    ]);
    if let Some(line) = config_setup_guidance_line(payload) {
        lines.push(line);
    }
    lines.extend([
        payload
            .get("capSkillSummary")
            .and_then(Value::as_str)
            .unwrap_or("No $cap summary available.")
            .to_string(),
        payload
            .get("customAgentSummary")
            .and_then(Value::as_str)
            .unwrap_or("No custom-agent summary available.")
            .to_string(),
    ]);
    lines.join("\n")
}

pub(crate) fn create_install_check_payload(session_context: &SessionContext) -> Value {
    collect_install_check_payload(session_context)
}

pub(crate) fn create_server_identity_text(session_context: &SessionContext) -> String {
    format!(
        "Attached CCC MCP session {} is running through the Rust {} runtime.",
        session_context.session_id,
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_codex_home(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!("ccc-install-check-{label}-{suffix}"))
    }

    fn create_plugin_cache_binary(codex_home: &Path) -> String {
        let command = codex_home
            .join("plugins")
            .join("cache")
            .join("ccc-local")
            .join("ccc")
            .join(env!("CARGO_PKG_VERSION"))
            .join("bin")
            .join("ccc");
        fs::create_dir_all(command.parent().expect("plugin command parent"))
            .expect("create plugin-cache bin dir");
        fs::write(&command, "#!/bin/sh\n").expect("write plugin-cache binary stub");
        resolve_plugin_cache_launch_command_at(codex_home).expect("resolve plugin-cache command")
    }

    fn registry_record(command: String, args: Vec<&str>) -> CodexMcpRegistryRecord {
        CodexMcpRegistryRecord {
            name: SERVER_NAME.to_string(),
            enabled: true,
            transport: Some(CodexMcpRegistryTransport {
                transport_type: "stdio".to_string(),
                command: Some(command),
                args: Some(args.into_iter().map(str::to_string).collect()),
            }),
        }
    }

    fn current_config_state(value: Value) -> CccConfigInstallState {
        CccConfigInstallState {
            status: "canonical-current",
            action_status: "preserved",
            backup_status: "not-required",
            summary: "Canonical config is current.".to_string(),
            source_path: None,
            backup_source_path: None,
            backup_path: None,
            value,
            canonical_ready: true,
            config_exists: true,
            restart_status: "not-required",
            entry_policy_mode_status: "canonical",
            entry_policy_mode_raw: Some("guided_explicit".to_string()),
            entry_policy_mode_canonical: Some("guided_explicit".to_string()),
            entry_policy_mode_summary:
                "Entry policy mode `guided_explicit` is canonical and supported.".to_string(),
        }
    }

    #[test]
    fn optional_missing_config_surfaces_do_not_block_current_readiness() {
        let config_state = current_config_state(json!({
            "generated_defaults": {
                "version": CURRENT_GENERATED_DEFAULTS_VERSION,
            },
            "routing": {
                "mode": "category_shortlist",
                "categories": {
                    "write_code": {
                        "agents": ["raider"],
                    },
                },
            },
            "runtime": {
                "preferred_specialist_execution_mode": "codex_subagent",
                "fallback_specialist_execution_mode": "codex_exec",
            },
        }));
        let readiness = create_config_surface_readiness_payload(
            &config_state,
            &json!({
                "status": "available",
                "role_count": 8,
            }),
            &json!({
                "status": "matching_sync",
            }),
        );
        let status_for = |surface_id: &str| {
            readiness["surfaces"]
                .as_array()
                .expect("surfaces")
                .iter()
                .find(|surface| surface["surface"] == surface_id)
                .and_then(|surface| surface["status"].as_str())
                .unwrap_or("missing")
                .to_string()
        };

        assert_eq!(readiness["status"], "current");
        assert_eq!(readiness["missing_count"], 0);
        assert_eq!(readiness["optional_missing_count"], 4);
        assert_eq!(status_for("concurrency"), "optional_missing");
        assert_eq!(status_for("prompt_sections"), "optional_missing");
        assert_eq!(status_for("directory_rule_injection"), "optional_missing");
        assert_eq!(status_for("hook_settings"), "optional_missing");
        assert_eq!(
            create_check_install_status(
                "matching_registration",
                &config_state,
                "matching_install",
                "matching_sync",
                &readiness,
            ),
            "ok"
        );
    }

    #[test]
    fn registration_match_accepts_versioned_plugin_cache_mcp_command() {
        let expected_command =
            "/Users/example/.local/share/ccc/releases/0.0.15-pre-darwin-arm64/bin/ccc".to_string();
        let expected_args = vec!["mcp".to_string()];
        let codex_home = create_temp_codex_home("matching-plugin-cache");
        let plugin_cache_command = create_plugin_cache_binary(&codex_home);
        let accepted_commands = vec![expected_command.clone(), plugin_cache_command.clone()];
        let record = registry_record(plugin_cache_command, vec!["mcp"]);

        assert!(registration_matches_any_expected(
            &record,
            &accepted_commands,
            &expected_args
        ));
    }

    #[test]
    fn registration_match_rejects_wrong_plugin_cache_version_and_args() {
        let expected_command =
            "/Users/example/.local/share/ccc/releases/0.0.15-pre-darwin-arm64/bin/ccc".to_string();
        let expected_args = vec!["mcp".to_string()];
        let codex_home = create_temp_codex_home("wrong-plugin-cache");
        let plugin_cache_command = create_plugin_cache_binary(&codex_home);
        let accepted_commands = vec![expected_command.clone(), plugin_cache_command.clone()];
        let wrong_version_command = codex_home
            .join("plugins")
            .join("cache")
            .join("ccc-local")
            .join("ccc")
            .join("0.0.14-pre")
            .join("bin")
            .join("ccc")
            .to_string_lossy()
            .into_owned();

        assert!(!registration_matches_any_expected(
            &registry_record(wrong_version_command, vec!["mcp"]),
            &accepted_commands,
            &expected_args
        ));

        assert!(!registration_matches_any_expected(
            &registry_record(plugin_cache_command, vec!["status"]),
            &accepted_commands,
            &expected_args
        ));
    }
}
