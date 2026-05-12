#![recursion_limit = "256"]

mod activity_view;
mod captain_intervention;
mod cli_io;
mod cli_output;
mod code_graph;
mod config_io;
mod entry_arguments;
mod entry_policy;
mod execution_contract;
#[allow(dead_code)]
mod graph_context;
mod host_subagent_lifecycle;
mod install_check;
mod lifecycle_hooks;
mod long_session;
mod mcp_dispatch;
mod mcp_tools;
mod mcp_transport;
mod memory;
mod orchestration_attempt;
mod orchestration_state;
mod parallel_fanout;
mod request_routing;
mod review_policy;
mod run_bootstrap;
mod run_locator;
mod scheduler_transition;
mod setup_config;
mod skill_manifest;
mod skill_registry;
mod specialist_roles;
mod status_app_panel;
mod status_compact;
mod status_cost_routing;
mod status_payload;
mod status_render;
mod subagent_update;
mod subagent_update_validation;
mod target_workspace;
mod text_utils;
mod token_display;
mod token_usage;
mod worker_events;
mod worker_lifecycle;
mod worker_supervisor;
mod worktree_guard;

use activity_view::create_ccc_activity_payload;
#[cfg(test)]
use activity_view::create_ccc_activity_text;
#[cfg(test)]
use captain_intervention::task_card_captain_follow_up_dedupe_key;
use captain_intervention::{
    consumed_pending_follow_up_payload, create_captain_intervention_payload,
    create_follow_up_task_card_from_pending_follow_up, create_pending_captain_follow_up_payload,
    existing_pending_follow_up_for_key, pending_follow_up_dedupe_key,
    queued_pending_captain_follow_up,
};
use chrono::{SecondsFormat, Utc};
use cli_io::{
    parse_cli_command_input, parse_cli_json_argument, print_json_payload, print_text_line,
    CliOutputMode,
};
use cli_output::{
    create_checklist_quiet_text, create_checklist_text, create_orchestrate_quiet_line,
    create_orchestrate_text_line, create_start_quiet_line, create_start_text_line,
    create_status_quiet_line, create_subagent_update_quiet_line, create_subagent_update_text_line,
};
pub(crate) use config_io::{
    create_timestamped_backup, generate_uuid_like_id, is_permission_error, read_json_document,
    read_optional_json_document, read_optional_shared_config_document, read_optional_toml_document,
    resolve_ccc_config_directory, resolve_codex_home, resolve_custom_agent_install_directory,
    resolve_legacy_shared_json_config_path, resolve_legacy_shared_json_config_path_for,
    resolve_legacy_shared_toml_config_path, resolve_legacy_shared_toml_config_path_for,
    resolve_previous_shared_config_path_for, resolve_shared_config_path, sanitize_value_for_toml,
    timestamped_backup_path_for, write_json_document, write_string_atomic, write_toml_document,
};
use entry_arguments::{
    parse_ccc_auto_entry_arguments, parse_ccc_orchestrate_arguments,
    parse_ccc_recommend_entry_arguments, parse_ccc_start_arguments,
    parse_ccc_subagent_update_arguments,
};
use entry_policy::{
    create_ccc_auto_entry_payload, create_ccc_auto_entry_text, create_ccc_recommend_entry_payload,
    create_ccc_recommend_entry_text, runtime_review_pressure_snapshot_from_run_directory,
};
#[cfg(test)]
use entry_policy::{
    create_ccc_auto_entry_payload_for_policy, create_ccc_recommend_entry_payload_for_policy,
};
use host_subagent_lifecycle::{
    is_active_host_subagent_status, is_terminal_host_subagent_status,
    is_terminal_or_merged_host_subagent_status, next_action_for_host_subagent_status,
    phase_name_for_host_subagent_status, task_card_subagent_fallback_ready,
    update_run_host_subagent_handle_state,
};
use install_check::{
    collect_install_check_payload, create_check_install_text, create_server_identity_payload,
    ensure_matching_mcp_registration, install_packaged_cap_skill,
    refresh_install_surface_config_visibility,
};
#[cfg(test)]
use install_check::{
    collect_install_check_payload_for_config_path, create_config_visibility_payload,
    create_custom_agent_visibility_payload, create_install_surface_visibility_payload,
    create_registration_visibility_payload, create_skill_visibility_payload,
    inspect_packaged_cap_skill_install_at,
};
pub(crate) use mcp_dispatch::handle_message;
use mcp_tools::tool_error;
use mcp_transport::{read_mcp_message, write_mcp_message, TransportMode};
use orchestration_attempt::{
    create_orchestration_attempt_payload, next_orchestration_attempt_file,
    resolve_requested_progression_mode, OrchestrationAttemptPayloadInput,
};
use orchestration_state::{
    apply_orchestrator_state_after_attempt, apply_run_record_after_attempt,
    apply_run_state_after_attempt, OrchestratorStateUpdateInput, RunRecordUpdateInput,
};
use parallel_fanout::{
    maybe_create_parallel_fanout_payload, normalize_host_lane_id, parallel_required_lane_ids,
    update_parallel_fanout_for_lane,
};
#[cfg(test)]
use request_routing::default_tool_routing_config;
#[cfg(test)]
use request_routing::{
    create_assignment_quality_payload, create_companion_tool_route_payload_for_policy,
    create_specialist_shortlist_payload_from_config, load_tool_routing_policy,
};
use request_routing::{create_routing_trace_payload, infer_request_shape, infer_task_shape};
#[cfg(test)]
use review_policy::task_card_reviews_source;
use review_policy::{
    create_review_fan_in_payload, create_review_policy_payload, infer_review_outcome,
    maybe_create_captain_owned_review_task_card, push_review_cap_finding,
    review_pass_cap_for_task_card, review_task_card_for_source, review_task_card_has_passed_fan_in,
    task_card_is_review, verification_state_for_review_outcome,
};
#[cfg(test)]
use review_policy::{runtime_review_pressure_snapshot_from_value, RuntimeReviewPressureSnapshot};
use run_bootstrap::create_ccc_start_payload;
#[cfg(test)]
use run_locator::normalize_path;
#[cfg(test)]
use run_locator::CCC_RUN_REF_PREFIX;
use run_locator::{create_ccc_run_ref, resolve_run_locator_arguments, ResolvedRunLocator};
use scheduler_transition::{append_scheduler_transition_record, SchedulerTransitionRecordInput};
use serde_json::{json, Map, Value};
use setup_config::{
    ensure_ccc_config_file, ensure_ccc_config_file_with_state, plan_ccc_config_setup_at,
    rollback_ccc_config_from_backup_at,
};
#[cfg(test)]
use setup_config::{
    ensure_ccc_config_file_at, ensure_ccc_config_file_at_with_state, CccConfigInstallState,
};
use specialist_roles::{
    agent_id_for_role, apply_task_expertise_framing, build_task_card_payload_with_role,
    create_specialist_delegation_plan, fallback_specialist_execution_mode,
    load_role_config_snapshot, normalize_dispatch_role_hint, normalize_specialist_execution_mode,
    phase_name_for_role, preferred_specialist_execution_mode, resolve_follow_up_specialist_role,
    role_for_agent_id, sandbox_mode_for_role, sandbox_rationale_for_role,
    sync_generated_custom_agents_from_config,
};
#[cfg(test)]
use specialist_roles::{
    create_specialist_delegation_plan_with_runtime, custom_agent_developer_instructions_for_role,
    inspect_generated_custom_agents_in_directory, sync_generated_custom_agents_in_directory,
    task_expertise_framing_for_role,
};
use status_app_panel::{create_codex_app_panel_text, write_codex_app_panel_artifact};
use status_compact::create_ccc_status_compact_payload;
pub(crate) use status_payload::{
    create_ccc_status_payload, create_current_task_card_payload,
    create_post_fan_in_captain_decision_payload, create_run_state_payload,
};
#[cfg(test)]
pub(crate) use status_render::create_ccc_status_text;
#[cfg(test)]
use status_render::create_visibility_signature;
pub(crate) use status_render::{
    build_captain_intervention_line, create_ccc_status_operator_text,
    create_operator_longway_projection_text, create_subagents_text,
};
use std::env;
use std::fs;
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
#[cfg(test)]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
pub(crate) use subagent_update::preferred_subagent_child_agent_id;
use subagent_update::{
    apply_subagent_orchestrator_state_update, apply_subagent_run_state_update,
    create_sentinel_intervention_payload, create_subagent_fan_in_compact,
    create_subagent_lifecycle_payload, create_subagent_policy_drift_payload,
    normalize_subagent_update_agent_identity, update_subagent_run_child_agent_entry,
    update_subagent_run_specialist_executor_entry, SubagentFanInCompactInput,
    SubagentLifecyclePayloadInput, SubagentOrchestratorStateUpdateInput,
    SubagentRunRecordChildInput, SubagentRunRecordExecutorInput, SubagentRunStateUpdateInput,
};
pub(crate) use text_utils::summarize_text_for_visibility;
#[cfg(test)]
use token_usage::{create_token_usage_payload, create_token_usage_visibility_payload};
#[cfg(test)]
use worker_events::build_worker_completion_snapshot;
use worker_lifecycle::{
    collapse_terminal_fan_in, reclaim_stuck_worker_delegations, task_has_active_worker_delegation,
};
pub(crate) use worker_lifecycle::{
    create_worker_lifecycle_view, create_worker_visibility_payload,
    refresh_running_delegation_heartbeat,
};
#[cfg(test)]
use worker_supervisor::build_task_execution_prompt;
use worker_supervisor::{run_worker_supervisor, spawn_codex_exec_for_task};

pub(crate) const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
pub(crate) const SERVER_NAME: &str = "ccc";
const PUBLIC_ENTRY_SKILL_NAME: &str = "cap";
const PUBLIC_ENTRY_LABEL: &str = "$cap";
const SUBAGENT_FAN_IN_ARTIFACT_LIMIT_BYTES: usize = 4096;
const SUBAGENT_FAN_IN_SUMMARY_LIMIT_CHARS: usize = 900;
const SUBAGENT_FAN_IN_INLINE_SUMMARY_CHARS: usize = 700;
const SUBAGENT_FAN_IN_INLINE_ITEMS: usize = 12;
#[derive(Clone, Debug)]
pub(crate) struct SessionContext {
    session_id: String,
    process_id: u32,
    started_at: String,
    build_identity: String,
    entrypoint_path: Option<String>,
    shared_config_path: String,
}

#[derive(Debug)]
struct RunMutationLock {
    path: PathBuf,
}

impl Drop for RunMutationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn main() {
    if let Err(error) = run() {
        let _ = writeln!(io::stderr(), "{error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        Some("mcp") => run_mcp_server(),
        Some("setup") => run_setup_command(&args[1..]),
        Some("sync-custom-agents") => run_sync_custom_agents_command(),
        Some("check-install") => run_check_install_command(),
        Some("server-identity") => run_server_identity_command(),
        Some("status") => run_status_command(&args[1..]),
        Some("checklist") => run_checklist_command(&args[1..]),
        Some("activity") => run_activity_command(&args[1..]),
        Some("graph") => run_graph_command(&args[1..]),
        Some("memory") => run_memory_command(&args[1..]),
        Some("recommend-entry") => run_recommend_entry_command(&args[1..]),
        Some("auto-entry") => run_auto_entry_command(&args[1..]),
        Some("start") => run_start_command(&args[1..]),
        Some("orchestrate") => run_orchestrate_command(&args[1..]),
        Some("subagent-update") => run_subagent_update_command(&args[1..]),
        Some("worker-supervise") => run_worker_supervise_command(&args[1..]),
        Some("--version") | Some("-V") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("--help") | Some("-h") | None => {
            print!("{}", cli_usage());
            Ok(())
        }
        Some(command) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unknown command: {command}\n\n{}", cli_usage()),
        )),
    }
}

fn cli_usage() -> String {
    [
        "Usage:",
        "  ccc mcp",
        "    Run the CCC MCP stdio server.",
        "  ccc setup",
        "    Register the local Rust MCP command with Codex CLI and install the packaged $cap skill.",
        "  ccc setup --rollback-config <backup_path>",
        "    Restore a setup-created config backup to the canonical ccc-config.toml path.",
        "  ccc check-install",
        "    Inspect the local Rust installation contract for registration, config, and $cap skill state.",
        "  ccc sync-custom-agents",
        "    Render CCC-managed Codex custom agents under CODEX_HOME/agents from ccc-config.toml.",
        "  ccc server-identity",
        "    Print the current server identity and install-check payload as JSON.",
        "  ccc status [--text|--quiet] [--subagents|--projection] --json '{...}'",
        "    Print persisted run status as JSON (default), human text (--text), or one-line summary (--quiet).",
        "  ccc status --app-panel [--text|--quiet] --json '{...}'",
        "    Print only the Codex app LongWay/status panel payload or transcript-readable panel text.",
        "  ccc checklist --text [--subagents|--projection] --json '{...}'",
        "    Print only the standalone LongWay checklist block for a run.",
        "  ccc activity --json '{...}'",
        "    Print the persisted run activity payload as JSON.",
        "  ccc graph [--text|--quiet] --json '{...}'",
        "    Query or update the repo code graph with JSON output (default) or human text.",
        "  ccc memory [--text|--quiet] --json '{...}'",
        "    Inspect or explicitly preview/write/off workspace memory. Default action is status.",
        "  ccc recommend-entry [--text|--quiet] --json '{...}'",
        "    Recommend whether a fresh request should enter CCC, with JSON output by default.",
        "  ccc auto-entry [--text|--quiet] --json '{...}'",
        "    Deterministically enter CCC for a fresh request when entry policy allows it, with JSON output by default.",
        "  ccc start [--text|--quiet] --json '{...}'",
        "    Create a bounded run with JSON output (default), human status (--text), or one-line success (--quiet).",
        "  ccc orchestrate [--text|--quiet] --json '{...}'",
        "    Advance an existing run with JSON output (default), human status (--text), or one-line success (--quiet).",
        "  ccc subagent-update [--text|--quiet] --json '{...}'",
        "    Record host subagent lifecycle/fan-in with JSON output (default), human status (--text), or one-line success (--quiet).",
        "  ccc --version",
        "",
    ]
    .join("\n")
}

fn run_setup_command(args: &[String]) -> io::Result<()> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let rollback_config_path = parse_setup_rollback_config_path(args)?;
    if dry_run && rollback_config_path.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "`ccc setup --dry-run` cannot be combined with `--rollback-config`.",
        ));
    }
    if let Some(backup_path) = rollback_config_path {
        return run_setup_rollback_config_command(&backup_path);
    }
    if dry_run {
        return run_setup_dry_run_command();
    }

    let session_context = create_session_context();
    let (config_path, _config_created, config_install_state) = ensure_ccc_config_file_with_state()?;
    let config_value = read_optional_toml_document(&config_path)?.unwrap_or(Value::Null);
    let (skill_path, skill_installed) = install_packaged_cap_skill()?;
    let custom_agent_sync = sync_generated_custom_agents_from_config(&config_value)?;
    let registration_status = ensure_matching_mcp_registration()?;
    let mut install_check = collect_install_check_payload(&session_context);
    if let Some(object) = install_check.as_object_mut() {
        object.insert(
            "configStatus".to_string(),
            Value::String(config_install_state.status.to_string()),
        );
        object.insert(
            "configActionStatus".to_string(),
            Value::String(config_install_state.action_status.to_string()),
        );
        object.insert(
            "configSummary".to_string(),
            Value::String(config_install_state.summary.clone()),
        );
        object.insert(
            "configRestartStatus".to_string(),
            Value::String(config_install_state.restart_status.to_string()),
        );
        object.insert(
            "configBackupStatus".to_string(),
            Value::String(config_install_state.backup_status.to_string()),
        );
        object.insert(
            "configBackupSourcePath".to_string(),
            config_install_state.backup_source_path_value(),
        );
        object.insert(
            "configBackupPath".to_string(),
            config_install_state.backup_path_value(),
        );
    }
    refresh_install_surface_config_visibility(&mut install_check, &config_install_state);

    println!(
        "CCC setup: registration={} config={} config_action={} skill={}",
        registration_status,
        config_install_state.status,
        config_install_state.action_status,
        if skill_installed {
            "installed"
        } else {
            "refreshed"
        }
    );
    println!("Config path: {}", config_path.display());
    println!("Skill path: {}", skill_path.display());
    println!(
        "Custom agents: {}",
        custom_agent_sync
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("custom-agent sync unavailable")
    );
    println!("{}", create_check_install_text(&install_check));
    println!("Please restart Codex CLI.");
    Ok(())
}

fn parse_setup_rollback_config_path(args: &[String]) -> io::Result<Option<PathBuf>> {
    let mut rollback_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" => {
                index += 1;
            }
            "--rollback-config" => {
                if rollback_path.is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "`--rollback-config` may only be provided once.",
                    ));
                }
                let Some(path) = args.get(index + 1) else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "`ccc setup --rollback-config` requires a backup path.",
                    ));
                };
                rollback_path = Some(PathBuf::from(path));
                index += 2;
            }
            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown setup option: {unknown}\n\n{}", cli_usage()),
                ));
            }
        }
    }
    Ok(rollback_path)
}

fn run_setup_rollback_config_command(backup_path: &Path) -> io::Result<()> {
    let config_path = resolve_shared_config_path();
    let rollback_state = rollback_ccc_config_from_backup_at(&config_path, backup_path)?;
    let session_context = create_session_context();
    let mut install_check = collect_install_check_payload(&session_context);
    if let Some(object) = install_check.as_object_mut() {
        object.insert(
            "configStatus".to_string(),
            Value::String(rollback_state.status.to_string()),
        );
        object.insert(
            "configActionStatus".to_string(),
            Value::String(rollback_state.action_status.to_string()),
        );
        object.insert(
            "configSummary".to_string(),
            Value::String(rollback_state.summary.clone()),
        );
        object.insert(
            "configRestartStatus".to_string(),
            Value::String(rollback_state.restart_status.to_string()),
        );
        object.insert(
            "configBackupStatus".to_string(),
            Value::String(rollback_state.backup_status.to_string()),
        );
        object.insert(
            "configBackupSourcePath".to_string(),
            rollback_state.backup_source_path_value(),
        );
        object.insert(
            "configBackupPath".to_string(),
            rollback_state.backup_path_value(),
        );
    }
    refresh_install_surface_config_visibility(&mut install_check, &rollback_state);

    println!(
        "CCC setup rollback: config={} config_action={} backup={} restart={}",
        rollback_state.status,
        rollback_state.action_status,
        rollback_state.backup_status,
        rollback_state.restart_status
    );
    println!("Config path: {}", config_path.display());
    println!("Rollback source: {}", backup_path.display());
    println!("{}", create_check_install_text(&install_check));
    println!("Please restart Codex CLI.");
    Ok(())
}

fn run_setup_dry_run_command() -> io::Result<()> {
    let config_path = resolve_shared_config_path();
    let legacy_toml_path = resolve_legacy_shared_toml_config_path();
    let legacy_json_path = resolve_legacy_shared_json_config_path();
    let plan = plan_ccc_config_setup_at(&config_path, &legacy_toml_path, &legacy_json_path)?;

    println!(
        "CCC setup dry-run: config={} config_action={} backup={} restart={}",
        plan.status, plan.action_status, plan.backup_status, plan.restart_status
    );
    println!("Config path: {}", config_path.display());
    println!("{}", plan.summary);
    if let Some(source_path) = &plan.backup_source_path {
        println!("Planned backup source: {}", source_path.display());
    }
    if let Some(backup_path) = &plan.backup_path {
        println!("Planned backup path: {}", backup_path.display());
    }
    println!("No files were written.");
    Ok(())
}

fn run_sync_custom_agents_command() -> io::Result<()> {
    let (config_path, _) = ensure_ccc_config_file()?;
    let config_value = read_optional_toml_document(&config_path)?.unwrap_or(Value::Null);
    let payload = sync_generated_custom_agents_from_config(&config_value)?;
    println!(
        "{}",
        payload
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("CCC custom-agent sync completed.")
    );
    if let Some(path) = payload.get("directory_path").and_then(Value::as_str) {
        println!("Custom agent directory: {path}");
    }
    if let Some(names) = payload.get("generated_names").and_then(Value::as_array) {
        let rendered = names
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !rendered.is_empty() {
            println!("Generated agents: {rendered}");
        }
    }
    Ok(())
}

fn run_check_install_command() -> io::Result<()> {
    let session_context = create_session_context();
    let payload = collect_install_check_payload(&session_context);
    println!("{}", create_check_install_text(&payload));
    Ok(())
}

fn run_server_identity_command() -> io::Result<()> {
    let session_context = create_session_context();
    print_json_payload(&json!({
        "server_identity": create_server_identity_payload(&session_context),
        "install_check": collect_install_check_payload(&session_context),
    }))
}

fn run_status_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("status", args, false)?;
    let parsed = &parsed_input.payload;
    let locator_payload = status_locator_payload_from_cli_payload(parsed);
    let session_context = create_session_context();
    let locator = resolve_run_locator_arguments(&locator_payload, "ccc_status")?;
    let payload = create_ccc_status_payload(&session_context, &locator)?;
    let result = if parsed_input.projection {
        let projection =
            write_operator_longway_projection(&locator.cwd, &locator.run_directory, &payload)?;
        match parsed_input.output_mode {
            CliOutputMode::Json => print_json_payload(&projection),
            CliOutputMode::Text | CliOutputMode::Quiet => print_text_line(&format!(
                "Projection: {}",
                projection
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("CCC_LONGWAY_PROJECTION.md")
            )),
        }
    } else if parsed_input.subagents {
        match parsed_input.output_mode {
            CliOutputMode::Text | CliOutputMode::Quiet => {
                print_text_line(&create_subagents_text(&payload))
            }
            CliOutputMode::Json => {
                if parsed
                    .get("compact")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    print_json_payload(&create_ccc_status_compact_payload(&payload))
                } else {
                    print_json_payload(&payload)
                }
            }
        }
    } else if parsed_input.app_panel {
        let app_panel = payload.get("app_panel").cloned().unwrap_or(Value::Null);
        if parsed_input.artifact {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "`--artifact` is internal-only for app-panel status; use `ccc status --app-panel` or `ccc status --projection`.",
            ))
        } else {
            match parsed_input.output_mode {
                CliOutputMode::Text | CliOutputMode::Quiet => {
                    print_text_line(&create_codex_app_panel_text(&app_panel))
                }
                CliOutputMode::Json => print_json_payload(&app_panel),
            }
        }
    } else {
        match parsed_input.output_mode {
            CliOutputMode::Text => {
                let projection_payload = sync_operator_longway_projection(
                    &locator.cwd,
                    &locator.run_directory,
                    &payload,
                )
                .unwrap_or_else(|error| {
                    json!({
                        "kind": "ccc_longway_projection",
                        "status": "sync_failed",
                        "reason": error.to_string()
                    })
                });
                let payload_for_text =
                    status_payload_with_operator_projection(&payload, &projection_payload);
                print_text_line(&create_ccc_status_operator_text(&payload_for_text))
            }
            CliOutputMode::Quiet => print_text_line(&create_status_quiet_line(&payload)),
            CliOutputMode::Json => {
                if parsed
                    .get("compact")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    print_json_payload(&create_ccc_status_compact_payload(&payload))
                } else {
                    print_json_payload(&payload)
                }
            }
        }
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn write_operator_longway_projection(
    workspace_directory: &Path,
    run_directory: &Path,
    payload: &Value,
) -> io::Result<Value> {
    let path = operator_longway_projection_path(workspace_directory);
    let text = create_operator_longway_projection_text(payload);
    write_string_atomic(&path, &text)?;
    let diff_visibility = ensure_operator_projection_diff_visible(workspace_directory, &path);
    Ok(json!({
        "kind": "ccc_longway_projection",
        "path": normalize_written_path(&path),
        "run_directory": normalize_written_path(run_directory),
        "stable": true,
        "format": "markdown",
        "cleanup": "single stable workspace file; overwritten by the next projection update",
        "diff_visibility": diff_visibility
    }))
}

fn sync_operator_longway_projection(
    workspace_directory: &Path,
    run_directory: &Path,
    payload: &Value,
) -> io::Result<Value> {
    let terminal = matches!(
        payload.get("status").and_then(Value::as_str),
        Some("completed" | "failed" | "cancelled")
    ) || matches!(
        payload
            .pointer("/run_state/next_action/command")
            .and_then(Value::as_str),
        Some("halt_completed" | "halt_failed" | "halt_cancelled")
    );
    let path = operator_longway_projection_path(workspace_directory);
    if terminal {
        match fs::remove_file(&path) {
            Ok(()) => Ok(json!({
                "kind": "ccc_longway_projection",
                "path": path.to_string_lossy(),
                "stable": true,
                "status": "removed",
                "reason": "terminal_longway"
            })),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(json!({
                "kind": "ccc_longway_projection",
                "path": path.to_string_lossy(),
                "stable": true,
                "status": "absent",
                "reason": "terminal_longway"
            })),
            Err(error) => Err(error),
        }
    } else {
        write_operator_longway_projection(workspace_directory, run_directory, payload)
    }
}

fn status_payload_with_operator_projection(
    status_payload: &Value,
    projection_payload: &Value,
) -> Value {
    let mut payload = status_payload.clone();
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "operator_longway_projection".to_string(),
            projection_payload.clone(),
        );
    }
    payload
}

fn operator_longway_projection_path(workspace_directory: &Path) -> PathBuf {
    workspace_directory.join("CCC_LONGWAY_PROJECTION.md")
}

fn normalize_written_path(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn ensure_operator_projection_diff_visible(workspace_directory: &Path, path: &Path) -> Value {
    let relative_path = path
        .strip_prefix(workspace_directory)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let status = Command::new("git")
        .arg("-C")
        .arg(workspace_directory)
        .arg("add")
        .arg("-N")
        .arg("--")
        .arg(&relative_path)
        .output();

    match status {
        Ok(output) if output.status.success() => json!({
            "status": "git_intent_to_add",
            "path": relative_path,
            "diff_command": "git diff -- CCC_LONGWAY_PROJECTION.md"
        }),
        Ok(output) => json!({
            "status": "file_only",
            "path": relative_path,
            "reason": format!("git add -N exited with status {}", output.status)
        }),
        Err(error) => json!({
            "status": "file_only",
            "path": relative_path,
            "reason": format!("git add -N unavailable: {error}")
        }),
    }
}

fn status_locator_payload_from_cli_payload(parsed: &Value) -> Value {
    let Some(object) = parsed.as_object() else {
        return parsed.clone();
    };
    let looks_like_run_record =
        object.contains_key("active_agent_id") || object.contains_key("active_task_card_id");
    if !looks_like_run_record {
        return parsed.clone();
    }

    let mut locator = Map::new();
    for key in ["run_id", "run_ref", "run_dir", "cwd"] {
        if let Some(value) = object.get(key).filter(|value| !value.is_null()) {
            locator.insert(key.to_string(), value.clone());
        }
    }
    if !locator.contains_key("run_dir") {
        if let Some(value) = object.get("run_directory").filter(|value| !value.is_null()) {
            locator.insert("run_dir".to_string(), value.clone());
        }
    }
    if locator.is_empty() {
        parsed.clone()
    } else {
        Value::Object(locator)
    }
}

fn run_checklist_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("checklist", args, false)?;
    let session_context = create_session_context();
    let locator = resolve_run_locator_arguments(&parsed_input.payload, "ccc_status")?;
    let status_payload = create_ccc_status_payload(&session_context, &locator)?;
    let payload = create_ccc_checklist_payload_from_status(&status_payload);
    let result = if parsed_input.projection {
        let projection = write_operator_longway_projection(
            &locator.cwd,
            &locator.run_directory,
            &status_payload,
        )?;
        match parsed_input.output_mode {
            CliOutputMode::Json => print_json_payload(&projection),
            CliOutputMode::Text | CliOutputMode::Quiet => print_text_line(&format!(
                "Projection: {}",
                projection
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("CCC_LONGWAY_PROJECTION.md")
            )),
        }
    } else {
        match parsed_input.output_mode {
            CliOutputMode::Json => print_json_payload(&payload),
            CliOutputMode::Text => print_text_line(&if parsed_input.subagents {
                create_subagents_text(&status_payload)
            } else {
                payload
                    .get("checklist")
                    .and_then(Value::as_str)
                    .unwrap_or("LongWay")
                    .to_string()
            }),
            CliOutputMode::Quiet => {
                if parsed_input.subagents {
                    print_text_line(&create_subagents_text(&status_payload))
                } else {
                    print_text_line(&create_checklist_quiet_text(&status_payload))
                }
            }
        }
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

#[cfg(test)]
fn create_ccc_checklist_payload(
    session_context: &SessionContext,
    locator: &ResolvedRunLocator,
) -> io::Result<Value> {
    let status_payload = create_ccc_status_payload(session_context, locator)?;
    Ok(create_ccc_checklist_payload_from_status(&status_payload))
}

fn create_ccc_checklist_payload_from_status(status_payload: &Value) -> Value {
    json!({
        "run_id": status_payload.get("run_id").cloned().unwrap_or(Value::Null),
        "run_ref": status_payload.get("run_ref").cloned().unwrap_or(Value::Null),
        "checklist": create_checklist_text(&status_payload),
    })
}

fn run_activity_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_json_argument("activity", args, false)?;
    let session_context = create_session_context();
    let locator = resolve_run_locator_arguments(&parsed_input.payload, "ccc_activity")?;
    let payload = create_ccc_activity_payload(&session_context, &locator)?;
    let result = print_json_payload(&payload);
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_graph_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("graph", args, false)?;
    let session_context = create_session_context();
    let payload = graph_context::create_graph_context_code_graph_payload_for_config_path(
        &parsed_input.payload,
        Path::new(&session_context.shared_config_path),
    )?
    .map(Ok)
    .unwrap_or_else(|| code_graph::create_code_graph_payload(&parsed_input.payload))?;
    let result = match parsed_input.output_mode {
        CliOutputMode::Text | CliOutputMode::Quiet => print_text_line(&create_graph_text(&payload)),
        CliOutputMode::Json => print_json_payload(&payload),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn create_graph_text(payload: &Value) -> String {
    if payload.get("schema").and_then(Value::as_str) == Some("ccc.graph_context_code_graph.v1") {
        graph_context::create_graph_context_code_graph_text(payload)
    } else {
        code_graph::create_code_graph_text(payload)
    }
}

fn run_memory_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("memory", args, false)?;
    let payload = memory::create_memory_payload(&parsed_input.payload)?;
    let result = match parsed_input.output_mode {
        CliOutputMode::Text | CliOutputMode::Quiet => {
            print_text_line(&memory::create_memory_text(&payload))
        }
        CliOutputMode::Json => print_json_payload(&payload),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_recommend_entry_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("recommend-entry", args, false)?;
    let parsed = parse_ccc_recommend_entry_arguments(&parsed_input.payload)?;
    let payload = create_ccc_recommend_entry_payload(&parsed);
    let result = match parsed_input.output_mode {
        CliOutputMode::Text | CliOutputMode::Quiet => {
            print_text_line(&create_ccc_recommend_entry_text(&payload))
        }
        CliOutputMode::Json => print_json_payload(&payload),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_auto_entry_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("auto-entry", args, false)?;
    let parsed = parse_ccc_auto_entry_arguments(&parsed_input.payload)?;
    let session_context = create_session_context();
    let mut payload = create_ccc_auto_entry_payload(&session_context, &parsed)?;
    sync_auto_entry_projection_after_creation(&session_context, &mut payload)?;
    let result = match parsed_input.output_mode {
        CliOutputMode::Text | CliOutputMode::Quiet => {
            print_text_line(&create_ccc_auto_entry_text(&payload))
        }
        CliOutputMode::Json => print_json_payload(&payload),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn sync_auto_entry_projection_after_creation(
    session_context: &SessionContext,
    payload: &mut Value,
) -> io::Result<()> {
    if payload.get("created").and_then(Value::as_bool) != Some(true) {
        return Ok(());
    }
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
            "cwd": payload.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_status",
    )?;
    let status_payload = create_ccc_status_payload(session_context, &locator)?;
    let projection_payload =
        sync_operator_longway_projection(&locator.cwd, &locator.run_directory, &status_payload)
            .unwrap_or_else(|error| {
                json!({
                    "kind": "ccc_longway_projection",
                    "status": "sync_failed",
                    "reason": error.to_string()
                })
            });
    if let Some(object) = payload.as_object_mut() {
        object.insert("longway_projection".to_string(), projection_payload);
        object.insert(
            "longway".to_string(),
            status_payload
                .get("longway")
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    Ok(())
}

fn run_start_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("start", args, false)?;
    let parsed = parse_ccc_start_arguments(&parsed_input.payload)?;
    let session_context = create_session_context();
    let start_payload = create_ccc_start_payload(&parsed)?;
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": start_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_status",
    )?;
    let status_payload = create_ccc_status_payload(&session_context, &locator)?;
    if let Some(app_panel) = status_payload.get("app_panel") {
        let _ = write_codex_app_panel_artifact(&locator.run_directory, app_panel);
    }
    let projection_payload =
        sync_operator_longway_projection(&locator.cwd, &locator.run_directory, &status_payload)
            .unwrap_or_else(|error| {
                json!({
                    "kind": "ccc_longway_projection",
                    "status": "sync_failed",
                    "reason": error.to_string()
                })
            });
    let status_payload_for_text =
        status_payload_with_operator_projection(&status_payload, &projection_payload);
    let result = match parsed_input.output_mode {
        CliOutputMode::Text => print_text_line(&create_start_text_line(
            &start_payload,
            &status_payload_for_text,
        )),
        CliOutputMode::Quiet => {
            print_text_line(&create_start_quiet_line(&start_payload, &status_payload))
        }
        CliOutputMode::Json
            if parsed
                .get("compact")
                .and_then(Value::as_bool)
                .unwrap_or(false) =>
        {
            print_json_payload(&create_ccc_status_compact_payload(&status_payload))
        }
        CliOutputMode::Json => print_json_payload(&json!({
            "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
            "run_id": start_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "task_card_id": start_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
            "run_directory": start_payload.get("run_directory").cloned().unwrap_or(Value::Null),
            "run_ref": start_payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "status": start_payload.get("status").cloned().unwrap_or(Value::Null),
            "stage": start_payload.get("stage").cloned().unwrap_or(Value::Null),
            "sequence": start_payload.get("sequence").cloned().unwrap_or(Value::Null),
            "approval_state": start_payload.get("approval_state").cloned().unwrap_or(Value::Null),
            "current_task_card": status_payload.get("current_task_card").cloned().unwrap_or(Value::Null),
            "next_step": start_payload.get("next_step").cloned().unwrap_or(Value::Null),
            "recommended_next_poll_ms": start_payload.get("recommended_next_poll_ms").cloned().unwrap_or(Value::Null),
            "can_advance": start_payload.get("can_advance").cloned().unwrap_or(Value::Null),
            "allowed_next_commands": start_payload.get("allowed_next_commands").cloned().unwrap_or(Value::Null),
            "longway_projection": projection_payload,
            "run_state": status_payload.get("run_state").cloned().unwrap_or(Value::Null),
            "server_identity": status_payload.get("server_identity").cloned().unwrap_or(Value::Null),
            "longway": status_payload.get("longway").cloned().unwrap_or(Value::Null),
        })),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_orchestrate_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("orchestrate", args, false)?;
    let parsed = parse_ccc_orchestrate_arguments(&parsed_input.payload)?;
    let session_context = create_session_context();
    let orchestrate_payload = create_ccc_orchestrate_payload(&parsed)?;
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": orchestrate_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "cwd": orchestrate_payload.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_status",
    )?;
    let status_payload = create_ccc_status_payload(&session_context, &locator)?;
    if let Some(app_panel) = status_payload.get("app_panel") {
        let _ = write_codex_app_panel_artifact(&locator.run_directory, app_panel);
    }
    let projection_payload =
        sync_operator_longway_projection(&locator.cwd, &locator.run_directory, &status_payload)
            .unwrap_or_else(|error| {
                json!({
                    "kind": "ccc_longway_projection",
                    "status": "sync_failed",
                    "reason": error.to_string()
                })
            });
    let status_payload_for_text =
        status_payload_with_operator_projection(&status_payload, &projection_payload);
    let result = match parsed_input.output_mode {
        CliOutputMode::Text => print_text_line(&create_orchestrate_text_line(
            &orchestrate_payload,
            &status_payload_for_text,
        )),
        CliOutputMode::Quiet => print_text_line(&create_orchestrate_quiet_line(
            &orchestrate_payload,
            &status_payload,
        )),
        CliOutputMode::Json
            if parsed
                .get("compact")
                .and_then(Value::as_bool)
                .unwrap_or(false) =>
        {
            let mut compact_payload = create_ccc_status_compact_payload(&status_payload);
            if let Some(object) = compact_payload.as_object_mut() {
                object.insert(
                    "orchestrate_result".to_string(),
                    json!({
                        "attempt_id": orchestrate_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
                        "starting_next_step": orchestrate_payload.get("starting_next_step").cloned().unwrap_or(Value::Null),
                        "next_step": orchestrate_payload.get("next_step").cloned().unwrap_or(Value::Null),
                        "summary": orchestrate_payload.get("summary").cloned().unwrap_or(Value::Null),
                    }),
                );
            }
            print_json_payload(&compact_payload)
        }
        CliOutputMode::Json => print_json_payload(&json!({
            "cwd": orchestrate_payload.get("cwd").cloned().unwrap_or(Value::Null),
            "run_id": orchestrate_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "run_directory": orchestrate_payload.get("run_directory").cloned().unwrap_or(Value::Null),
            "run_ref": orchestrate_payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "attempt_id": orchestrate_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
            "task_card_id": status_payload
                .get("current_task_card")
                .and_then(|value| value.get("task_card_id"))
                .cloned()
                .unwrap_or(Value::Null),
            "status": status_payload.get("status").cloned().unwrap_or(Value::Null),
            "stage": status_payload.get("stage").cloned().unwrap_or(Value::Null),
            "current_task_card": status_payload.get("current_task_card").cloned().unwrap_or(Value::Null),
            "starting_next_step": orchestrate_payload.get("starting_next_step").cloned().unwrap_or(Value::Null),
            "next_step": orchestrate_payload.get("next_step").cloned().unwrap_or(Value::Null),
            "progression_mode": orchestrate_payload.get("progression_mode").cloned().unwrap_or(Value::Null),
            "can_advance": orchestrate_payload.get("can_advance").cloned().unwrap_or(Value::Null),
            "advanced": orchestrate_payload.get("advanced").cloned().unwrap_or(Value::Null),
            "summary": orchestrate_payload.get("summary").cloned().unwrap_or(Value::Null),
            "launch_result": orchestrate_payload.get("launch_result").cloned().unwrap_or(Value::Null),
            "reclaimed_targets": orchestrate_payload.get("reclaimed_targets").cloned().unwrap_or(Value::Null),
            "collapsed_fan_in": orchestrate_payload.get("collapsed_fan_in").cloned().unwrap_or(Value::Null),
            "allowed_next_commands": orchestrate_payload.get("allowed_next_commands").cloned().unwrap_or(Value::Null),
            "longway_projection": projection_payload,
            "run_state": status_payload.get("run_state").cloned().unwrap_or(Value::Null),
            "server_identity": status_payload.get("server_identity").cloned().unwrap_or(Value::Null),
            "longway": status_payload.get("longway").cloned().unwrap_or(Value::Null),
        })),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_subagent_update_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_command_input("subagent-update", args, false)?;
    let parsed = parse_ccc_subagent_update_arguments(&parsed_input.payload)?;
    let session_context = create_session_context();
    let update_payload = create_ccc_subagent_update_payload(&parsed)?;
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": update_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "cwd": update_payload.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_status",
    )?;
    let status_payload = create_ccc_status_payload(&session_context, &locator)?;
    if let Some(app_panel) = status_payload.get("app_panel") {
        let _ = write_codex_app_panel_artifact(&locator.run_directory, app_panel);
    }
    let projection_payload =
        sync_operator_longway_projection(&locator.cwd, &locator.run_directory, &status_payload)
            .unwrap_or_else(|error| {
                json!({
                    "kind": "ccc_longway_projection",
                    "status": "sync_failed",
                    "reason": error.to_string()
                })
            });
    let result = match parsed_input.output_mode {
        CliOutputMode::Text => print_text_line(&create_subagent_update_text_line(
            &update_payload,
            &status_payload,
        )),
        CliOutputMode::Quiet => print_text_line(&create_subagent_update_quiet_line(
            &update_payload,
            &status_payload,
        )),
        CliOutputMode::Json
            if parsed
                .get("compact")
                .and_then(Value::as_bool)
                .unwrap_or(false) =>
        {
            print_json_payload(&json!({
            "compact": true,
            "cwd": update_payload.get("cwd").cloned().unwrap_or(Value::Null),
            "run_id": update_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": update_payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "task_card_id": update_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
            "child_agent_id": update_payload.get("child_agent_id").cloned().unwrap_or(Value::Null),
            "lane_id": update_payload.get("lane_id").cloned().unwrap_or(Value::Null),
            "thread_id": update_payload.get("thread_id").cloned().unwrap_or(Value::Null),
            "event_ref": update_payload.get("event_ref").cloned().unwrap_or(Value::Null),
            "subagent_status": update_payload.get("subagent_status").cloned().unwrap_or(Value::Null),
            "review_outcome": update_payload.get("review_outcome").cloned().unwrap_or(Value::Null),
            "summary": update_payload.get("summary").cloned().unwrap_or(Value::Null),
            "fan_in": update_payload.get("fan_in").cloned().unwrap_or(Value::Null),
            "fan_in_artifact": update_payload.get("fan_in_artifact").cloned().unwrap_or(Value::Null),
            "review_fan_in": update_payload.get("review_fan_in").cloned().unwrap_or(Value::Null),
            "captain_intervention": update_payload.get("captain_intervention").cloned().unwrap_or(Value::Null),
            "sentinel_intervention": update_payload.get("sentinel_intervention").cloned().unwrap_or(Value::Null),
            "fallback_reason": update_payload.get("fallback_reason").cloned().unwrap_or(Value::Null),
            "subagent_fallback": status_payload.pointer("/current_task_card/subagent_fallback").cloned().unwrap_or(Value::Null),
            "subagent_policy_drift": status_payload.pointer("/current_task_card/subagent_policy_drift").cloned().unwrap_or(Value::Null),
            "next_step": status_payload.get("next_step").cloned().unwrap_or(Value::Null),
            "can_advance": status_payload.get("can_advance").cloned().unwrap_or(Value::Null),
            "run_truth_surface": status_payload.get("run_truth_surface").cloned().unwrap_or(Value::Null),
            "host_subagent_state": status_payload.get("host_subagent_state").cloned().unwrap_or(Value::Null),
            "command_templates": create_ccc_status_compact_payload(&status_payload)
                .get("command_templates")
                .cloned()
                .unwrap_or(Value::Null),
            "longway_projection": projection_payload,
            }))
        }
        CliOutputMode::Json => print_json_payload(&json!({
            "cwd": update_payload.get("cwd").cloned().unwrap_or(Value::Null),
            "run_id": update_payload.get("run_id").cloned().unwrap_or(Value::Null),
            "run_directory": update_payload.get("run_directory").cloned().unwrap_or(Value::Null),
            "run_ref": update_payload.get("run_ref").cloned().unwrap_or(Value::Null),
            "task_card_id": update_payload.get("task_card_id").cloned().unwrap_or(Value::Null),
            "child_agent_id": update_payload.get("child_agent_id").cloned().unwrap_or(Value::Null),
            "lane_id": update_payload.get("lane_id").cloned().unwrap_or(Value::Null),
            "thread_id": update_payload.get("thread_id").cloned().unwrap_or(Value::Null),
            "event_ref": update_payload.get("event_ref").cloned().unwrap_or(Value::Null),
            "subagent_status": update_payload.get("subagent_status").cloned().unwrap_or(Value::Null),
            "review_outcome": update_payload.get("review_outcome").cloned().unwrap_or(Value::Null),
            "summary": update_payload.get("summary").cloned().unwrap_or(Value::Null),
            "fan_in": update_payload.get("fan_in").cloned().unwrap_or(Value::Null),
            "fan_in_artifact": update_payload.get("fan_in_artifact").cloned().unwrap_or(Value::Null),
            "review_fan_in": update_payload.get("review_fan_in").cloned().unwrap_or(Value::Null),
            "captain_intervention": update_payload.get("captain_intervention").cloned().unwrap_or(Value::Null),
            "sentinel_intervention": update_payload.get("sentinel_intervention").cloned().unwrap_or(Value::Null),
            "status": status_payload.get("status").cloned().unwrap_or(Value::Null),
            "stage": status_payload.get("stage").cloned().unwrap_or(Value::Null),
            "current_task_card": status_payload.get("current_task_card").cloned().unwrap_or(Value::Null),
            "next_step": status_payload.get("next_step").cloned().unwrap_or(Value::Null),
            "can_advance": status_payload.get("can_advance").cloned().unwrap_or(Value::Null),
            "host_subagent_state": status_payload.get("host_subagent_state").cloned().unwrap_or(Value::Null),
            "run_state": status_payload.get("run_state").cloned().unwrap_or(Value::Null),
            "server_identity": status_payload.get("server_identity").cloned().unwrap_or(Value::Null),
            "longway": status_payload.get("longway").cloned().unwrap_or(Value::Null),
            "longway_projection": projection_payload,
        })),
    };
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_worker_supervise_command(args: &[String]) -> io::Result<()> {
    let parsed_input = parse_cli_json_argument("worker-supervise", args, false)?;
    let payload = run_worker_supervisor(&parsed_input.payload)?;
    let result = print_json_payload(&payload);
    if result.is_ok() {
        parsed_input.cleanup_transient_json_file_after_success();
    }
    result
}

fn run_mcp_server() -> io::Result<()> {
    let session_context = create_session_context();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut transport_mode: Option<TransportMode> = None;
    let mut initialized = false;

    loop {
        let Some(message) = read_mcp_message(&mut reader, &mut transport_mode)? else {
            break;
        };

        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !initialized && method == "tools/call" {
            let response = tool_error(
                message.get("id").cloned(),
                -32002,
                "Server not ready. Send notifications/initialized before sending requests.",
            );
            write_mcp_message(
                &mut writer,
                &response,
                transport_mode.unwrap_or(TransportMode::Framed),
            )?;
            writer.flush()?;
            continue;
        }

        if method == "notifications/initialized" {
            initialized = true;
        }

        if let Some(response) = handle_message(&session_context, message) {
            write_mcp_message(
                &mut writer,
                &response,
                transport_mode.unwrap_or(TransportMode::Framed),
            )?;
            writer.flush()?;
        }
    }

    Ok(())
}

fn create_session_context() -> SessionContext {
    let started_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let process_id = process::id();
    let session_id = format!(
        "mcp-session-{:x}-{:x}",
        process_id,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let entrypoint_path = env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    let build_identity = format!("{SERVER_NAME}@{}:{started_at}", env!("CARGO_PKG_VERSION"));

    SessionContext {
        session_id,
        process_id,
        started_at,
        build_identity,
        entrypoint_path,
        shared_config_path: resolve_shared_config_path().to_string_lossy().into_owned(),
    }
}

fn resolve_effective_codex_bin(parsed: &Value, run_directory: Option<&Path>) -> String {
    if let Some(value) = parsed
        .get("codex_bin")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return value.to_string();
    }

    if let Some(run_directory) = run_directory {
        let orchestrator_state_file = run_directory.join("orchestrator-state.json");
        if let Ok(orchestrator_state) = read_json_document(&orchestrator_state_file) {
            if let Some(value) = orchestrator_state
                .get("execution_request")
                .and_then(|request| request.get("codex_bin"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return value.to_string();
            }
        }

        let run_file = run_directory.join("run.json");
        if let Ok(run_record) = read_json_document(&run_file) {
            if let Some(value) = run_record
                .get("latest_entry_trace")
                .and_then(|trace| trace.get("codex_bin"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return value.to_string();
            }
        }
    }

    "codex".to_string()
}

fn append_run_event(run_directory: &Path, mut event: Value) -> io::Result<()> {
    let run_state_file = run_directory.join("run-state.json");
    let existing_run_state = read_optional_json_document(&run_state_file)?;
    let current_count = existing_run_state
        .as_ref()
        .and_then(|value| value.get("event_count"))
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            fs::read_to_string(run_directory.join("events.jsonl"))
                .map(|content| {
                    content
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count() as u64
                })
                .unwrap_or(0)
        });
    let next_count = current_count.saturating_add(1);
    let event_id = format!("event-{next_count:04}");
    let timestamp = event
        .get("timestamp")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true));

    if let Some(object) = event.as_object_mut() {
        object
            .entry("event_id".to_string())
            .or_insert_with(|| Value::String(event_id.clone()));
        object
            .entry("timestamp".to_string())
            .or_insert_with(|| Value::String(timestamp.clone()));
    }

    let mut events = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(run_directory.join("events.jsonl"))?;
    writeln!(
        events,
        "{}",
        serde_json::to_string(&event).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("encode run event: {error}"),
            )
        })?
    )?;

    if let Some(mut run_state) = existing_run_state {
        if let Some(object) = run_state.as_object_mut() {
            object.insert("event_count".to_string(), json!(next_count));
            object.insert("last_event_id".to_string(), Value::String(event_id));
            object.insert("updated_at".to_string(), Value::String(timestamp));
            write_json_document(&run_state_file, &run_state)?;
        }
    }

    Ok(())
}

fn default_runtime_config() -> Value {
    json!({
        "preferred_specialist_execution_mode": "codex_subagent",
        "fallback_specialist_execution_mode": "visible_degraded_host_fallback",
        "worker_poll_interval_ms": 90_000,
        "worker_stuck_after_ms": 45_000,
        "worker_kill_grace_ms": 2_000,
        "worker_auto_reclaim_enabled": true,
        "worker_max_retries_per_phase": 1,
        "worker_retry_backoff_ms": 1_000,
        "worker_prompt_scope_max_chars": 320,
        "worker_prompt_acceptance_max_chars": 220,
        "worker_prompt_task_max_chars": 720,
        "run_lock_stale_after_ms": 300_000,
    })
}

fn normalized_runtime_config_from_shared_config(candidate: &Value) -> Value {
    let runtime = candidate.get("runtime").cloned().unwrap_or(Value::Null);

    let mut normalized = json!({
        "preferred_specialist_execution_mode": normalize_specialist_execution_mode(
            runtime
                .get("preferred_specialist_execution_mode")
                .and_then(Value::as_str),
            "codex_subagent",
        ),
        "fallback_specialist_execution_mode": normalize_specialist_execution_mode(
            runtime
                .get("fallback_specialist_execution_mode")
                .and_then(Value::as_str),
            "visible_degraded_host_fallback",
        ),
        "worker_poll_interval_ms": runtime.get("worker_poll_interval_ms").and_then(Value::as_i64).filter(|value| *value > 0).unwrap_or(90_000),
        "worker_stuck_after_ms": runtime.get("worker_stuck_after_ms").and_then(Value::as_i64).filter(|value| *value > 0).unwrap_or(45_000),
        "worker_kill_grace_ms": runtime.get("worker_kill_grace_ms").and_then(Value::as_i64).filter(|value| *value >= 0).unwrap_or(2_000),
        "worker_auto_reclaim_enabled": runtime.get("worker_auto_reclaim_enabled").and_then(Value::as_bool).unwrap_or(true),
        "worker_max_retries_per_phase": runtime.get("worker_max_retries_per_phase").and_then(Value::as_i64).filter(|value| *value >= 0).unwrap_or(1),
        "worker_retry_backoff_ms": runtime.get("worker_retry_backoff_ms").and_then(Value::as_i64).filter(|value| *value >= 0).unwrap_or(1_000),
        "worker_prompt_scope_max_chars": runtime.get("worker_prompt_scope_max_chars").and_then(Value::as_i64).filter(|value| *value > 0).unwrap_or(320),
        "worker_prompt_acceptance_max_chars": runtime.get("worker_prompt_acceptance_max_chars").and_then(Value::as_i64).filter(|value| *value > 0).unwrap_or(220),
        "worker_prompt_task_max_chars": runtime.get("worker_prompt_task_max_chars").and_then(Value::as_i64).filter(|value| *value > 0).unwrap_or(720),
        "run_lock_stale_after_ms": runtime.get("run_lock_stale_after_ms").and_then(Value::as_i64).filter(|value| *value > 0).unwrap_or(300_000),
    });
    if let Some(object) = normalized.as_object_mut() {
        for key in [
            "host_subagent_default_provider_concurrency_limit",
            "default_provider_concurrency_limit",
            "host_subagent_default_model_concurrency_limit",
            "default_model_concurrency_limit",
            "host_subagent_provider_concurrency_limits",
            "provider_concurrency_limits",
            "host_subagent_model_concurrency_limits",
            "model_concurrency_limits",
            "host_subagent_concurrency",
            "host_subagent_reclaim_after_ms",
            "lifecycle_hooks",
        ] {
            if let Some(value) = runtime.get(key).cloned() {
                object.insert(key.to_string(), value);
            }
        }
    }
    normalized
}

pub(crate) fn load_runtime_config() -> io::Result<Value> {
    let Some((_, candidate)) = read_optional_shared_config_document()? else {
        return Ok(default_runtime_config());
    };
    Ok(normalized_runtime_config_from_shared_config(&candidate))
}

pub(crate) fn load_runtime_config_from_path(config_path: &Path) -> io::Result<Value> {
    let Some(candidate) = read_optional_toml_document(config_path)? else {
        return load_runtime_config();
    };
    Ok(normalized_runtime_config_from_shared_config(&candidate))
}

fn is_lock_file_stale(path: &Path, stale_after_ms: u64) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified_at) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(modified_at) else {
        return false;
    };
    elapsed.as_millis() >= stale_after_ms as u128
}

fn acquire_run_mutation_lock(
    run_directory: &Path,
    command_name: &str,
) -> io::Result<RunMutationLock> {
    fs::create_dir_all(run_directory)?;
    let runtime_config = load_runtime_config().unwrap_or_else(|_| {
        json!({
            "run_lock_stale_after_ms": 300_000
        })
    });
    let stale_after_ms = runtime_config
        .get("run_lock_stale_after_ms")
        .and_then(Value::as_u64)
        .unwrap_or(300_000);
    let lock_path = run_directory.join(".writer.lock");

    for _ in 0..2 {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                let payload = json!({
                    "command": command_name,
                    "process_id": process::id(),
                    "created_at": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                });
                file.write_all(
                    serde_json::to_string_pretty(&payload)
                        .unwrap_or_default()
                        .as_bytes(),
                )?;
                file.write_all(b"\n")?;
                return Ok(RunMutationLock { path: lock_path });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if is_lock_file_stale(&lock_path, stale_after_ms) {
                    let _ = fs::remove_file(&lock_path);
                    continue;
                }
                let owner_summary = fs::read_to_string(&lock_path)
                    .ok()
                    .map(|text| summarize_text_for_visibility(&text, 120))
                    .unwrap_or_else(|| "unknown active writer".to_string());
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!(
                        "Run is already being mutated by another CCC writer. owner={owner_summary}"
                    ),
                ));
            }
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        "Run writer lock is unavailable after stale-lock cleanup attempt.",
    ))
}

fn summarize_prompt_title(prompt: &str) -> String {
    let compact = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        return "Captain follow-up task".to_string();
    }
    if trimmed.chars().count() <= 72 {
        return trimmed.to_string();
    }
    trimmed.chars().take(69).collect::<String>() + "..."
}

fn append_way_phase_for_follow_up(
    run_directory: &Path,
    task_card_id: &str,
    assigned_role: &str,
    title: &str,
    timestamp: &str,
) -> io::Result<()> {
    let longway_path = run_directory.join("longway.json");
    let Some(mut longway) = read_optional_json_document(&longway_path)? else {
        return Ok(());
    };
    let Some(longway_object) = longway.as_object_mut() else {
        return Ok(());
    };
    let phase_name = phase_name_for_role(assigned_role);
    let phases = longway_object
        .entry("phases".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(phases_array) = phases.as_array_mut() else {
        return Ok(());
    };
    if let Some(active_phase) = phases_array
        .iter_mut()
        .rev()
        .find(|phase| phase.get("status").and_then(Value::as_str) != Some("completed"))
    {
        if let Some(phase_object) = active_phase.as_object_mut() {
            phase_object.insert("status".to_string(), Value::String("completed".to_string()));
            phase_object.insert(
                "finished_at".to_string(),
                Value::String(timestamp.to_string()),
            );
            phase_object.insert(
                "updated_at".to_string(),
                Value::String(timestamp.to_string()),
            );
        }
    }
    phases_array.push(json!({
        "task_card_id": task_card_id,
        "phase_name": phase_name,
        "title": title,
        "status": "pending",
        "updated_at": timestamp,
        "started_at": Value::Null,
        "finished_at": Value::Null,
    }));
    longway_object.insert(
        "active_phase_name".to_string(),
        Value::String(phase_name.to_string()),
    );
    longway_object.insert(
        "active_phase_status".to_string(),
        Value::String("pending".to_string()),
    );
    longway_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&longway_path, &longway)
}

fn longway_has_phase_for_task_card(run_directory: &Path, task_card_id: &str) -> io::Result<bool> {
    let Some(longway) = read_optional_json_document(&run_directory.join("longway.json"))? else {
        return Ok(false);
    };
    Ok(longway
        .get("phases")
        .and_then(Value::as_array)
        .map(|phases| {
            phases.iter().any(|phase| {
                phase.get("task_card_id").and_then(Value::as_str) == Some(task_card_id)
            })
        })
        .unwrap_or(false))
}

fn activate_review_task_card_for_completion_gate(
    run_directory: &Path,
    review_task_card: &Value,
    timestamp: &str,
) -> io::Result<()> {
    let review_task_card_id = review_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "review task card is missing task_card_id",
            )
        })?;
    let assigned_role = review_task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("verifier");
    let title = review_task_card
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Review completed implementation");

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert(
        "active_task_card_id".to_string(),
        Value::String(review_task_card_id.to_string()),
    );
    run_object.insert(
        "active_role".to_string(),
        Value::String("orchestrator".to_string()),
    );
    run_object.insert(
        "active_agent_id".to_string(),
        Value::String("captain".to_string()),
    );
    run_object.insert(
        "latest_handoff_id".to_string(),
        Value::String(review_task_card_id.to_string()),
    );
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String("Captain queued arbiter verification before final completion.".to_string()),
    );
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    let run_state_path = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_path)?;
    let run_state_object = run_state.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(phase_name_for_role(assigned_role).to_string()),
    );
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({
            "command": "execute_task"
        }),
    );
    write_json_document(&run_state_path, &run_state)?;

    if !longway_has_phase_for_task_card(run_directory, review_task_card_id)? {
        append_way_phase_for_follow_up(
            run_directory,
            review_task_card_id,
            assigned_role,
            title,
            timestamp,
        )?;
    }

    Ok(())
}

fn maybe_require_arbiter_review_before_completion(
    run_directory: &Path,
    current_task_card: &Value,
    timestamp: &str,
) -> io::Result<Option<Value>> {
    if task_card_is_review(current_task_card) {
        return Ok(None);
    }
    if task_card_already_has_passed_verification(current_task_card) {
        return Ok(None);
    }

    let Some(source_task_card_id) = current_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    if let Some(review_task_card) = review_task_card_for_source(run_directory, source_task_card_id)?
    {
        if review_task_card_has_passed_fan_in(&review_task_card) {
            return Ok(None);
        }
        activate_review_task_card_for_completion_gate(run_directory, &review_task_card, timestamp)?;
        return Ok(Some(review_task_card));
    }

    let Some(review_task_card) =
        maybe_create_captain_owned_review_task_card(run_directory, current_task_card, timestamp)?
    else {
        return Ok(None);
    };
    activate_review_task_card_for_completion_gate(run_directory, &review_task_card, timestamp)?;
    Ok(Some(review_task_card))
}

fn task_card_already_has_passed_verification(task_card: &Value) -> bool {
    task_card
        .get("verification_state")
        .and_then(Value::as_str)
        .map(str::trim)
        == Some("passed")
        || task_card
            .get("review_fan_in")
            .is_some_and(review_task_card_has_passed_fan_in)
}

fn planned_row_text_field(row: &Value, key: &str, fallback: &str) -> String {
    planned_row_optional_text_field(row, key).unwrap_or_else(|| fallback.to_string())
}

fn planned_row_optional_text_field(row: &Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_planned_row_role(raw_role: &str) -> String {
    match raw_role.trim().to_ascii_lowercase().as_str() {
        "implementation_specialist"
        | "implementation specialist"
        | "code_specialist"
        | "code-specialist" => "code specialist".to_string(),
        "review_specialist" | "review specialist" => "verifier".to_string(),
        "exploration_specialist"
        | "exploration specialist"
        | "research_specialist"
        | "research specialist" => "explorer".to_string(),
        "documentation_specialist"
        | "documentation specialist"
        | "docs_specialist"
        | "docs specialist" => "documenter".to_string(),
        "unassigned" => "code specialist".to_string(),
        _ => normalize_dispatch_role_hint(Some(raw_role), "code specialist"),
    }
}

fn planned_row_role_is_unassigned(raw_role: &str) -> bool {
    matches!(
        raw_role.trim().to_ascii_lowercase().as_str(),
        "" | "unassigned" | "none" | "unknown" | "tbd"
    )
}

fn role_for_planned_agent_id(raw_agent_id: &str) -> Option<&'static str> {
    let normalized = raw_agent_id
        .trim()
        .trim_start_matches("ccc_")
        .to_ascii_lowercase();
    if planned_row_role_is_unassigned(&normalized) {
        return None;
    }
    role_for_agent_id(&normalized).or_else(|| {
        normalized
            .split_once('-')
            .and_then(|(agent_prefix, _)| role_for_agent_id(agent_prefix))
    })
}

fn planned_row_text_has_any(text: &str, terms: &[&str]) -> bool {
    let normalized_text = normalized_ascii_search_text(text);
    terms.iter().any(|term| {
        if term.is_ascii() {
            let normalized_term = normalized_ascii_search_text(term);
            !normalized_term.trim().is_empty()
                && normalized_text.contains(&format!(" {} ", normalized_term.trim()))
        } else {
            text.contains(term)
        }
    })
}

fn normalized_ascii_search_text(text: &str) -> String {
    let normalized = text
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    format!(" {normalized} ")
}

fn planned_row_text_inferred_role(
    title: &str,
    scope: &str,
    acceptance: &str,
) -> Option<&'static str> {
    let text = format!("{title}\n{scope}\n{acceptance}").to_ascii_lowercase();
    if planned_row_text_has_any(
        &text,
        &[
            "git",
            "gh",
            "commit",
            "push",
            "git tag",
            "release tag",
            "tag release",
            "release upload",
            "release-upload",
            "release command",
            "operator command",
            "bounded tool",
            "\u{cee4}\u{bc0b}",
            "\u{d478}\u{c2dc}",
        ],
    ) {
        return Some("companion_operator");
    }
    if planned_row_text_has_any(
        &text,
        &[
            "read-only",
            "read only",
            "readonly",
            "evidence",
            "inspect",
            "investigate",
            "collect",
            "scout",
            "search",
            "status",
            "\u{c77d}\u{ae30} \u{c804}\u{c6a9}",
            "\u{c99d}\u{ac70}",
            "\u{ac80}\u{d1a0}",
            "\u{c870}\u{c0ac}",
            "\u{c218}\u{c9d1}",
            "\u{ac80}\u{c0c9}",
            "\u{c0c1}\u{d0dc}",
        ],
    ) {
        return Some("explorer");
    }
    if planned_row_text_has_any(
        &text,
        &[
            "review",
            "verify",
            "validate",
            "test",
            "\u{ac80}\u{c99d}",
            "\u{d655}\u{c778}",
        ],
    ) {
        return Some("verifier");
    }
    if planned_row_text_has_any(
        &text,
        &[
            "docs",
            "document",
            "readme",
            "release note",
            "release-note",
            "changelog",
            "\u{bb38}\u{c11c}",
            "\u{b9b4}\u{b9ac}\u{c988} \u{b178}\u{d2b8}",
        ],
    ) {
        return Some("documenter");
    }
    if planned_row_text_has_any(
        &text,
        &[
            "implement",
            "fix",
            "edit",
            "mutate",
            "repair",
            "change",
            "update",
            "\u{ad6c}\u{d604}",
            "\u{c218}\u{c815}",
            "\u{d3b8}\u{c9d1}",
        ],
    ) {
        return Some("code specialist");
    }
    None
}

fn resolve_planned_row_assigned_role(
    planned_row: &Value,
    title: &str,
    scope: &str,
    acceptance: &str,
) -> String {
    if let Some(planned_role) = planned_row_optional_text_field(planned_row, "planned_role")
        .filter(|role| !planned_row_role_is_unassigned(role))
    {
        return normalize_planned_row_role(&planned_role);
    }
    if let Some(display_role) = planned_row_optional_text_field(planned_row, "display_role")
        .filter(|role| !planned_row_role_is_unassigned(role))
    {
        return normalize_planned_row_role(&display_role);
    }
    for agent_key in ["planned_agent_id", "display_agent_id"] {
        if let Some(role) = planned_row_optional_text_field(planned_row, agent_key)
            .and_then(|agent_id| role_for_planned_agent_id(&agent_id).map(str::to_string))
        {
            return role;
        }
    }
    planned_row_text_inferred_role(title, scope, acceptance)
        .unwrap_or("code specialist")
        .to_string()
}

fn next_unmaterialized_planned_row(longway: &Value) -> Option<(usize, Value)> {
    longway
        .get("planned_rows")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().enumerate().find_map(|(index, row)| {
                let is_planned = row.get("status").and_then(Value::as_str) == Some("planned");
                let has_task_card_id = row
                    .get("task_card_id")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty());
                if is_planned && !has_task_card_id {
                    Some((index, row.clone()))
                } else {
                    None
                }
            })
        })
}

fn mark_planned_row_materialized(
    run_directory: &Path,
    planned_row_index: usize,
    task_card_id: &str,
    timestamp: &str,
) -> io::Result<()> {
    let longway_path = run_directory.join("longway.json");
    let Some(mut longway) = read_optional_json_document(&longway_path)? else {
        return Ok(());
    };
    let Some(longway_object) = longway.as_object_mut() else {
        return Ok(());
    };
    if let Some(planned_row_object) = longway_object
        .get_mut("planned_rows")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.get_mut(planned_row_index))
        .and_then(Value::as_object_mut)
    {
        planned_row_object.insert(
            "status".to_string(),
            Value::String("materialized".to_string()),
        );
        planned_row_object.insert(
            "task_card_id".to_string(),
            Value::String(task_card_id.to_string()),
        );
        planned_row_object.insert(
            "materialized_at".to_string(),
            Value::String(timestamp.to_string()),
        );
        planned_row_object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }
    if let Some(planned_rows) = longway_object
        .get_mut("planned_rows")
        .and_then(Value::as_array_mut)
    {
        for planned_row_object in planned_rows.iter_mut().filter_map(Value::as_object_mut) {
            sanitize_planned_row_routing_fields(planned_row_object);
        }
    }
    longway_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&longway_path, &longway)
}

fn bounded_planned_row_evidence_strings(row: &Value, field: &str) -> Option<Value> {
    row.get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .take(8)
                .map(|value| Value::String(summarize_text_for_visibility(value, 160)))
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .map(Value::Array)
}

fn bounded_planned_row_routing_summary(row: &Value) -> Option<Value> {
    row.get("routing_summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Value::String(summarize_text_for_visibility(value, 240)))
}

fn first_bounded_routing_trace_string(
    trace: &serde_json::Map<String, Value>,
    keys: &[&str],
    max_chars: usize,
) -> Option<Value> {
    keys.iter().find_map(|key| {
        trace
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
    })
}

fn bounded_routing_trace_strings(
    trace: &serde_json::Map<String, Value>,
    keys: &[&str],
    max_items: usize,
    max_chars: usize,
) -> Option<Value> {
    keys.iter().find_map(|key| {
        let value = trace.get(*key)?;
        let strings = if let Some(items) = value.as_array() {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .take(max_items)
                .map(|value| Value::String(summarize_text_for_visibility(value, max_chars)))
                .collect::<Vec<_>>()
        } else {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    vec![Value::String(summarize_text_for_visibility(
                        value, max_chars,
                    ))]
                })
                .unwrap_or_default()
        };
        (!strings.is_empty()).then_some(Value::Array(strings))
    })
}

fn bounded_planned_row_routing_trace(row: &Value) -> Option<Value> {
    let trace = row.get("routing_trace")?.as_object()?;
    let mut bounded = serde_json::Map::new();
    bounded.insert(
        "source".to_string(),
        Value::String("planned_row".to_string()),
    );
    if let Some(value) =
        first_bounded_routing_trace_string(trace, &["query_kind", "query", "kind"], 80)
    {
        bounded.insert("query".to_string(), value);
    }
    for (output_key, input_keys, max_chars) in [
        ("selected_category", &["selected_category"][..], 80),
        ("selected_skill_id", &["selected_skill_id"][..], 120),
        ("selected_skill_name", &["selected_skill_name"][..], 120),
        ("risk", &["risk"][..], 40),
        ("mutation_intent", &["mutation_intent"][..], 80),
        ("evidence_need", &["evidence_need"][..], 120),
        ("verification_need", &["verification_need"][..], 120),
        ("selected_role", &["selected_role"][..], 80),
        ("selected_agent_id", &["selected_agent_id"][..], 80),
    ] {
        if let Some(value) = first_bounded_routing_trace_string(trace, input_keys, max_chars) {
            bounded.insert(output_key.to_string(), value);
        }
    }
    if let Some(value) = bounded_routing_trace_strings(trace, &["paths", "path"], 8, 160) {
        bounded.insert("paths".to_string(), value);
    }
    if let Some(value) =
        bounded_routing_trace_strings(trace, &["terms", "term", "search", "text"], 8, 120)
    {
        bounded.insert("terms".to_string(), value);
    }
    if let Some(value) = first_bounded_routing_trace_string(trace, &["reason", "rationale"], 240) {
        bounded.insert("reason".to_string(), value);
    }
    if let Some(value) = first_bounded_routing_trace_string(trace, &["summary"], 240) {
        bounded.insert("summary".to_string(), value);
    }

    (bounded.len() > 1).then_some(Value::Object(bounded))
}

fn sanitize_planned_row_routing_fields(planned_row: &mut serde_json::Map<String, Value>) {
    let planned_row_value = Value::Object(planned_row.clone());
    if let Some(value) = bounded_planned_row_routing_summary(&planned_row_value) {
        planned_row.insert("routing_summary".to_string(), value);
    }
    if let Some(value) = bounded_planned_row_routing_trace(&planned_row_value) {
        planned_row.insert("routing_trace".to_string(), value);
    } else {
        planned_row.remove("routing_trace");
    }
}

fn create_clarified_way_planned_rows(answer: &str, selected_role: &str) -> Vec<Value> {
    let answer_summary = summarize_text_for_visibility(answer, 220);
    json!([
        {
            "title": "Inspect clarified Way scope and release boundary",
            "planned_role": "exploration_specialist",
            "planned_agent_id": "scout-a",
            "scope": format!("Use the consumed Way clarification answer before mutation: {answer_summary}"),
            "acceptance": "Return concrete boundaries, release-readiness risks, and files that must be changed before execution.",
            "status": "planned",
            "routing_summary": "Way clarification was consumed; inspect boundaries before execution.",
            "routing_trace": {
                "query": "way_clarification_answer",
                "terms": ["pending LongWay", "clarified scope", "release readiness"],
                "reason": "PLAN_SEQUENCE regenerated planned rows after consuming the operator's Way clarification answer exactly once.",
                "structural_scene_id": "frame_goal",
                "summary": answer_summary
            }
        },
        {
            "title": "Execute clarified pending LongWay task",
            "planned_role": selected_role,
            "planned_agent_id": "unassigned",
            "scope": "Use the approved LongWay, the consumed clarification answer, and the boundary findings; keep execution inside the accepted scope.",
            "acceptance": "Implementation, validation evidence, and remaining release-readiness risks are ready for captain fan-in.",
            "status": "planned",
            "routing_summary": format!("Way selected {selected_role} after clarification answer consumption."),
            "routing_trace": {
                "query": "clarified_execution",
                "terms": ["approved LongWay", "clarification answer", selected_role],
                "reason": "PLAN_SEQUENCE selected the executable lane from the consumed Way clarification answer.",
                "structural_scene_id": "sequence_rows",
                "approval_scene_id": "surface_approval",
                "summary": "Execute only after explicit LongWay approval."
            }
        }
    ])
    .as_array()
    .cloned()
    .unwrap_or_default()
}

fn mark_clarification_consumed(value: &mut Value, answer: &str, timestamp: &str) -> io::Result<()> {
    let Some(object) = value.as_object_mut() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Way clarification request must be an object.",
        ));
    };
    if object
        .get("state")
        .and_then(Value::as_str)
        .is_some_and(|state| state == "consumed")
        || object
            .get("consumed_at")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Way clarification answer was already consumed.",
        ));
    }
    object.insert("state".to_string(), Value::String("consumed".to_string()));
    object.insert(
        "consumed_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    object.insert(
        "answer_summary".to_string(),
        Value::String(summarize_text_for_visibility(answer, 240)),
    );
    Ok(())
}

fn consume_way_clarification_answer(
    run_directory: &Path,
    current_task_card: &Value,
    answer: &str,
    timestamp: &str,
) -> io::Result<Value> {
    let answer = answer.trim();
    if answer.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Way clarification answer cannot be empty.",
        ));
    }

    let task_card_id = current_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "current task card is missing task_card_id",
            )
        })?;
    let selected_role = current_task_card
        .pointer("/routing_trace/selected_role")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("code specialist");
    let planned_rows = create_clarified_way_planned_rows(answer, selected_role);

    let run_path = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_path)?;
    let mut run_clarification = run_record
        .get("way_clarification_request")
        .cloned()
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "No pending Way clarification request exists on the run.",
            )
        })?;
    mark_clarification_consumed(&mut run_clarification, answer, timestamp)?;
    if let Some(run_object) = run_record.as_object_mut() {
        run_object.insert(
            "approval_state".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        run_object.insert(
            "way_clarification_request".to_string(),
            run_clarification.clone(),
        );
        run_object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }
    write_json_document(&run_path, &run_record)?;

    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_path)?;
    let mut task_clarification = task_card
        .get("way_clarification_request")
        .cloned()
        .unwrap_or_else(|| run_clarification.clone());
    if task_clarification
        .get("state")
        .and_then(Value::as_str)
        .is_some_and(|state| state != "consumed")
    {
        mark_clarification_consumed(&mut task_clarification, answer, timestamp)?;
    } else {
        task_clarification = run_clarification.clone();
    }
    if let Some(task_object) = task_card.as_object_mut() {
        task_object.insert(
            "status".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        task_object.insert(
            "approval_state".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        task_object.insert("way_clarification_request".to_string(), task_clarification);
        task_object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }
    write_json_document(&task_card_path, &task_card)?;

    let longway_path = run_directory.join("longway.json");
    let mut longway = read_json_document(&longway_path)?;
    let mut longway_clarification = longway
        .get("way_clarification_request")
        .cloned()
        .unwrap_or_else(|| run_clarification.clone());
    if longway_clarification
        .get("state")
        .and_then(Value::as_str)
        .is_some_and(|state| state != "consumed")
    {
        mark_clarification_consumed(&mut longway_clarification, answer, timestamp)?;
    } else {
        longway_clarification = run_clarification.clone();
    }
    if let Some(longway_object) = longway.as_object_mut() {
        longway_object.insert(
            "lifecycle_state".to_string(),
            Value::String("pending_approval".to_string()),
        );
        longway_object.insert(
            "approval_state".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        longway_object.insert(
            "active_phase_status".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        longway_object.insert(
            "planned_rows".to_string(),
            Value::Array(planned_rows.clone()),
        );
        longway_object.insert(
            "planned_row_count".to_string(),
            Value::from(planned_rows.len()),
        );
        longway_object.insert(
            "way_clarification_request".to_string(),
            longway_clarification.clone(),
        );
        longway_object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
        if let Some(phases) = longway_object
            .get_mut("phases")
            .and_then(Value::as_array_mut)
        {
            for phase in phases.iter_mut().filter_map(Value::as_object_mut) {
                phase.insert(
                    "status".to_string(),
                    Value::String("pending_longway_approval".to_string()),
                );
            }
        }
        if let Some(planning_context) = longway_object
            .get_mut("planning_context")
            .and_then(Value::as_object_mut)
        {
            planning_context.insert(
                "planned_row_count".to_string(),
                Value::from(planned_rows.len()),
            );
            planning_context.insert(
                "decomposition_source".to_string(),
                Value::String("way_clarification_answer".to_string()),
            );
        }
    }
    write_json_document(&longway_path, &longway)?;

    let run_state_path = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_path)?;
    if let Some(run_state_object) = run_state.as_object_mut() {
        run_state_object.insert(
            "approval_state".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        run_state_object.insert(
            "next_action".to_string(),
            json!({
                "command": "await_longway_approval",
                "reason": "Way clarification answer was consumed; pending LongWay awaits explicit approval."
            }),
        );
        run_state_object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }
    write_json_document(&run_state_path, &run_state)?;

    let orchestrator_state_path = run_directory.join("orchestrator-state.json");
    let mut orchestrator_state = read_json_document(&orchestrator_state_path)?;
    if let Some(orchestrator_object) = orchestrator_state.as_object_mut() {
        orchestrator_object.insert(
            "approval_state".to_string(),
            Value::String("pending_longway_approval".to_string()),
        );
        orchestrator_object.insert(
            "decision".to_string(),
            json!({
                "next_step": "await_longway_approval",
                "can_advance": false,
                "summary": "Way clarification answer was consumed exactly once; pending LongWay approval is ready."
            }),
        );
    }
    write_json_document(&orchestrator_state_path, &orchestrator_state)?;

    append_run_event(
        run_directory,
        json!({
            "event": "way_clarification_answer_consumed",
            "entrypoint": "ccc_orchestrate",
            "task_card_id": task_card_id,
            "planned_row_count": planned_rows.len(),
            "timestamp": timestamp,
        }),
    )?;

    Ok(json!({
        "schema": "ccc.way_clarification_consumption.v1",
        "state": "consumed",
        "task_card_id": task_card_id,
        "planned_row_count": planned_rows.len(),
        "answer_summary": summarize_text_for_visibility(answer, 240),
        "consumed_at": timestamp,
    }))
}

fn materialize_next_planned_row_task_card(
    run_directory: &Path,
    current_task_card: &Value,
    timestamp: &str,
) -> io::Result<Option<Value>> {
    let longway_path = run_directory.join("longway.json");
    let Some(longway) = read_optional_json_document(&longway_path)? else {
        return Ok(None);
    };
    let Some((planned_row_index, planned_row)) = next_unmaterialized_planned_row(&longway) else {
        return Ok(None);
    };
    let run_id = current_task_card
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "current task card is missing run_id",
            )
        })?;
    let title = planned_row_text_field(&planned_row, "title", "Planned LongWay task");
    let planned_role = planned_row_text_field(&planned_row, "planned_role", "code specialist");
    let planned_agent_id = planned_row_text_field(&planned_row, "planned_agent_id", "unassigned");
    let scope = planned_row_text_field(&planned_row, "scope", "No explicit planned-row scope.");
    let acceptance = planned_row_text_field(
        &planned_row,
        "acceptance",
        "No explicit planned-row acceptance.",
    );
    let assigned_role =
        resolve_planned_row_assigned_role(&planned_row, &title, &scope, &acceptance);
    let intent = format!("Captain materialized planned LongWay row {planned_row_index}.");
    let execution_prompt = format!(
        "Execute the planned LongWay row: {title}\n\nScope:\n{scope}\n\nAcceptance:\n{acceptance}"
    );
    let task_shape = infer_task_shape(&execution_prompt, "implementation");
    let task_card_id = generate_uuid_like_id();
    let mut task_card = build_task_card_payload_with_role(
        run_id,
        &task_card_id,
        &title,
        &intent,
        &scope,
        &execution_prompt,
        &acceptance,
        &assigned_role,
        timestamp,
    );
    if let Some(object) = task_card.as_object_mut() {
        object.insert(
            "sequence".to_string(),
            Value::String("EXECUTE_SEQUENCE".to_string()),
        );
        object.insert(
            "approval_state".to_string(),
            Value::String("approved_for_task_cards".to_string()),
        );
        object.insert("dispatch_allowed".to_string(), Value::Bool(true));
        object.insert(
            "routing_summary".to_string(),
            bounded_planned_row_routing_summary(&planned_row).unwrap_or_else(|| {
                Value::String("Captain materialized the next planned LongWay row.".to_string())
            }),
        );
        if let Some(value) = bounded_planned_row_routing_trace(&planned_row) {
            object.insert("routing_trace".to_string(), value);
        }
        if let Some(value) = bounded_planned_row_evidence_strings(&planned_row, "evidence_links") {
            object.insert("evidence_links".to_string(), value);
        }
        if let Some(value) = bounded_planned_row_evidence_strings(&planned_row, "result_links") {
            object.insert("result_links".to_string(), value);
        }
        object.insert(
            "planned_longway_row".to_string(),
            json!({
                "row_index": planned_row_index,
                "planned_role": planned_role,
                "planned_agent_id": planned_agent_id,
                "title": title.clone(),
                "status": "materialized",
                "task_card_id": task_card_id.clone(),
                "materialized_at": timestamp,
            }),
        );
    }
    apply_task_expertise_framing(&mut task_card, &assigned_role, task_shape);

    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    write_json_document(&task_card_path, &task_card)?;

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    let mut task_card_ids = run_object
        .get("task_card_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !task_card_ids
        .iter()
        .any(|value| value.as_str() == Some(task_card_id.as_str()))
    {
        task_card_ids.push(Value::String(task_card_id.clone()));
    }
    run_object.insert("task_card_ids".to_string(), Value::Array(task_card_ids));
    run_object.insert(
        "active_task_card_id".to_string(),
        Value::String(task_card_id.clone()),
    );
    run_object.insert(
        "active_role".to_string(),
        Value::String("orchestrator".to_string()),
    );
    run_object.insert(
        "active_agent_id".to_string(),
        Value::String("captain".to_string()),
    );
    run_object.insert(
        "latest_handoff_id".to_string(),
        Value::String(task_card_id.clone()),
    );
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    append_way_phase_for_follow_up(
        run_directory,
        &task_card_id,
        &assigned_role,
        &title,
        timestamp,
    )?;
    mark_planned_row_materialized(run_directory, planned_row_index, &task_card_id, timestamp)?;
    let selected_planned_row = task_card
        .get("planned_longway_row")
        .cloned()
        .unwrap_or(Value::Null);
    append_scheduler_transition_record(
        run_directory,
        SchedulerTransitionRecordInput {
            run_id,
            timestamp,
            action_kind: "materialize_planned_row",
            reason: "scheduler materialized the next planned LongWay row",
            selected_task_card: &task_card,
            selected_planned_row: &selected_planned_row,
            next_step_after_attempt: "execute_task",
            can_advance_after_attempt: true,
        },
    )?;

    Ok(Some(task_card))
}

fn complete_longway_active_phase(run_directory: &Path, timestamp: &str) -> io::Result<bool> {
    let longway_path = run_directory.join("longway.json");
    let Some(mut longway) = read_optional_json_document(&longway_path)? else {
        return Ok(false);
    };
    let Some(longway_object) = longway.as_object_mut() else {
        return Ok(false);
    };
    let active_phase_name = longway_object
        .get("active_phase_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let Some(phases) = longway_object
        .get_mut("phases")
        .and_then(Value::as_array_mut)
    else {
        return Ok(false);
    };
    let active_status_index = phases.iter().position(|phase| {
        let status = phase.get("status").and_then(Value::as_str);
        matches!(status, Some("active" | "running" | "in_progress"))
            || phase
                .get("active")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    });
    let named_active_index = active_phase_name.as_ref().and_then(|phase_name| {
        phases.iter().position(|phase| {
            phase.get("phase_name").and_then(Value::as_str) == Some(phase_name.as_str())
                && phase.get("status").and_then(Value::as_str) != Some("completed")
        })
    });
    let fallback_index = if active_phase_name.is_none() {
        phases
            .iter()
            .position(|phase| phase.get("status").and_then(Value::as_str) != Some("completed"))
    } else {
        None
    };
    let Some(active_index) = active_status_index
        .or(named_active_index)
        .or(fallback_index)
    else {
        return Ok(false);
    };

    if let Some(active_phase) = phases.get_mut(active_index) {
        if let Some(phase_object) = active_phase.as_object_mut() {
            phase_object.insert("status".to_string(), Value::String("completed".to_string()));
            phase_object.insert(
                "finished_at".to_string(),
                Value::String(timestamp.to_string()),
            );
            phase_object.insert(
                "updated_at".to_string(),
                Value::String(timestamp.to_string()),
            );
        }
    }

    longway_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&longway_path, &longway)?;
    Ok(true)
}

fn settle_longway_for_resolve(
    run_directory: &Path,
    checklist_state: &str,
    timestamp: &str,
) -> io::Result<()> {
    let longway_path = run_directory.join("longway.json");
    let Some(mut longway) = read_optional_json_document(&longway_path)? else {
        return Ok(());
    };
    let Some(longway_object) = longway.as_object_mut() else {
        return Ok(());
    };

    if let Some(phases) = longway_object
        .get_mut("phases")
        .and_then(Value::as_array_mut)
    {
        match checklist_state {
            "completed" => {
                for phase in phases.iter_mut() {
                    if phase.get("status").and_then(Value::as_str) == Some("completed") {
                        continue;
                    }
                    if let Some(phase_object) = phase.as_object_mut() {
                        phase_object
                            .insert("status".to_string(), Value::String("completed".to_string()));
                        phase_object.insert(
                            "finished_at".to_string(),
                            Value::String(timestamp.to_string()),
                        );
                        phase_object.insert(
                            "updated_at".to_string(),
                            Value::String(timestamp.to_string()),
                        );
                    }
                }
            }
            "failed" | "cancelled" => {
                if let Some(phase_object) = phases
                    .iter_mut()
                    .find(|phase| phase.get("status").and_then(Value::as_str) != Some("completed"))
                    .and_then(Value::as_object_mut)
                {
                    phase_object.insert(
                        "status".to_string(),
                        Value::String(checklist_state.to_string()),
                    );
                    phase_object.insert(
                        "finished_at".to_string(),
                        Value::String(timestamp.to_string()),
                    );
                    phase_object.insert(
                        "updated_at".to_string(),
                        Value::String(timestamp.to_string()),
                    );
                }
            }
            _ => {}
        }
    }

    longway_object.insert(
        "lifecycle_state".to_string(),
        Value::String(checklist_state.to_string()),
    );
    longway_object.insert(
        "active_phase_status".to_string(),
        Value::String(checklist_state.to_string()),
    );
    longway_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    longway_object.insert(
        "settled_at".to_string(),
        Value::String(timestamp.to_string()),
    );

    write_json_document(&longway_path, &longway)
}

fn settle_task_card_for_resolve(
    run_directory: &Path,
    current_task_card: &Value,
    checklist_state: &str,
    timestamp: &str,
) -> io::Result<()> {
    let Some(task_card_id) = current_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let Some(mut task_card) = read_optional_json_document(&task_card_path)? else {
        return Ok(());
    };
    let Some(task_card_object) = task_card.as_object_mut() else {
        return Ok(());
    };

    task_card_object.insert(
        "status".to_string(),
        Value::String(checklist_state.to_string()),
    );
    task_card_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    match checklist_state {
        "completed" => {
            task_card_object.insert(
                "completed_at".to_string(),
                Value::String(timestamp.to_string()),
            );
            task_card_object.insert(
                "verification_state".to_string(),
                Value::String("passed".to_string()),
            );
        }
        "failed" | "cancelled" => {
            task_card_object.insert(
                "completed_at".to_string(),
                Value::String(timestamp.to_string()),
            );
            task_card_object.insert(
                "verification_state".to_string(),
                Value::String(checklist_state.to_string()),
            );
        }
        "blocked" => {
            task_card_object.insert(
                "verification_state".to_string(),
                Value::String("blocked".to_string()),
            );
        }
        _ => {}
    }

    write_json_document(&task_card_path, &task_card)
}

pub(crate) fn create_follow_up_task_card(
    run_directory: &Path,
    current_task_card: &Value,
    role_hint: Option<&str>,
    replan_prompt: &str,
    resolve_summary: Option<&str>,
    timestamp: &str,
) -> io::Result<Value> {
    let run_id = current_task_card
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "current task card is missing run_id",
            )
        })?;
    let fallback_role = resolve_follow_up_specialist_role(current_task_card, None);
    let mut routing_trace = role_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|hint| {
            json!({
                "selected_role": resolve_follow_up_specialist_role(current_task_card, Some(hint)),
                "summary": format!("Captain explicitly selected {hint} for the follow-up task."),
                "tool_route": Value::Null,
                "specialist_route": Value::Null,
            })
        })
        .unwrap_or_else(|| create_routing_trace_payload(replan_prompt, &fallback_role));
    let assigned_role = resolve_follow_up_specialist_role(
        current_task_card,
        routing_trace.get("selected_role").and_then(Value::as_str),
    );
    if let Some(object) = routing_trace.as_object_mut() {
        object.insert(
            "selected_role".to_string(),
            Value::String(assigned_role.clone()),
        );
    }
    let assigned_role = routing_trace
        .get("selected_role")
        .and_then(Value::as_str)
        .unwrap_or(&assigned_role)
        .to_string();
    let task_card_id = generate_uuid_like_id();
    let title = resolve_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| summarize_prompt_title(replan_prompt));
    let intent = format!(
        "Captain selected a bounded {} follow-up after explicit fan-in.",
        assigned_role
    );
    let scope = current_task_card
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("Bounded follow-up task selected by captain.");
    let acceptance = current_task_card
        .get("acceptance")
        .and_then(Value::as_str)
        .unwrap_or("Return a bounded specialist result to captain.");
    let request_shape = routing_trace
        .get("request_shape")
        .and_then(Value::as_str)
        .unwrap_or_else(|| infer_request_shape(replan_prompt));
    let task_shape = infer_task_shape(replan_prompt, request_shape);
    let mut task_card = build_task_card_payload_with_role(
        run_id,
        &task_card_id,
        &title,
        &intent,
        scope,
        replan_prompt,
        acceptance,
        &assigned_role,
        timestamp,
    );
    let follow_up_task_kind = task_card
        .get("task_kind")
        .and_then(Value::as_str)
        .unwrap_or("execution")
        .to_string();
    if let Some(object) = task_card.as_object_mut() {
        object.insert(
            "routing_summary".to_string(),
            routing_trace.get("summary").cloned().unwrap_or(Value::Null),
        );
        object.insert("routing_trace".to_string(), routing_trace.clone());
        let runtime_pressure = runtime_review_pressure_snapshot_from_run_directory(run_directory)?;
        object.insert(
            "review_policy".to_string(),
            create_review_policy_payload(
                replan_prompt,
                request_shape,
                task_shape,
                Some(timestamp),
                runtime_pressure.as_ref(),
            ),
        );
        if let Some(parallel_fanout) = maybe_create_parallel_fanout_payload(
            &follow_up_task_kind,
            &assigned_role,
            &title,
            &intent,
            scope,
            replan_prompt,
            None,
            timestamp,
        ) {
            object.insert("parallel_fanout".to_string(), parallel_fanout);
        }
    }
    apply_task_expertise_framing(&mut task_card, &assigned_role, task_shape);
    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    write_json_document(&task_card_path, &task_card)?;

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    let mut task_card_ids = run_object
        .get("task_card_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    task_card_ids.push(Value::String(task_card_id.clone()));
    run_object.insert("task_card_ids".to_string(), Value::Array(task_card_ids));
    run_object.insert(
        "active_task_card_id".to_string(),
        Value::String(task_card_id.clone()),
    );
    run_object.insert(
        "active_role".to_string(),
        Value::String("orchestrator".to_string()),
    );
    run_object.insert(
        "active_agent_id".to_string(),
        Value::String("captain".to_string()),
    );
    run_object.insert(
        "latest_handoff_id".to_string(),
        Value::String(task_card_id.clone()),
    );
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    let run_state_path = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_path)?;
    let run_state_object = run_state.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(phase_name_for_role(&assigned_role).to_string()),
    );
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({
            "command": "execute_task"
        }),
    );
    write_json_document(&run_state_path, &run_state)?;

    append_way_phase_for_follow_up(
        run_directory,
        &task_card_id,
        &assigned_role,
        task_card
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Untitled task"),
        timestamp,
    )?;
    maybe_create_captain_owned_review_task_card(run_directory, &task_card, timestamp)?;

    Ok(task_card)
}

fn mark_run_resolved(
    run_directory: &Path,
    current_task_card: &Value,
    resolve_outcome: &str,
    resolve_summary: Option<&str>,
    timestamp: &str,
) -> io::Result<String> {
    let normalized_outcome = resolve_outcome.trim().to_ascii_lowercase();
    let (run_status, next_step, summary_prefix, checklist_state) = match normalized_outcome.as_str()
    {
        "completed" | "done" | "resolved" | "success" => (
            "completed",
            "halt_completed",
            "Captain closed the run as completed.",
            "completed",
        ),
        "blocked" => (
            "active",
            "await_operator",
            "Captain marked the run blocked and waiting for the operator.",
            "blocked",
        ),
        "failed" | "error" => (
            "failed",
            "halt_failed",
            "Captain closed the run as failed.",
            "failed",
        ),
        "cancelled" | "canceled" => (
            "cancelled",
            "halt_cancelled",
            "Captain cancelled the run.",
            "cancelled",
        ),
        _ => (
            "completed",
            "halt_completed",
            "Captain closed the run as completed.",
            "completed",
        ),
    };
    let summary = resolve_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| summary_prefix.to_string());

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert("status".to_string(), Value::String(run_status.to_string()));
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String(summary.clone()),
    );
    if matches!(run_status, "completed" | "failed" | "cancelled") {
        run_object.insert(
            "completed_at".to_string(),
            Value::String(timestamp.to_string()),
        );
    }
    write_json_document(&run_file, &run_record)?;

    let run_state_path = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_path)?;
    let run_state_object = run_state.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(
            phase_name_for_role(
                current_task_card
                    .get("assigned_role")
                    .and_then(Value::as_str)
                    .unwrap_or("orchestrator"),
            )
            .to_string(),
        ),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({
            "command": next_step
        }),
    );
    write_json_document(&run_state_path, &run_state)?;

    settle_task_card_for_resolve(run_directory, current_task_card, checklist_state, timestamp)?;
    settle_longway_for_resolve(run_directory, checklist_state, timestamp)?;

    Ok(summary)
}

fn task_kind_for_approved_role(role: &str) -> &'static str {
    match role {
        "way" => "way",
        "explorer" | "companion_reader" => "explore",
        "verifier" => "review",
        _ => "execution",
    }
}

fn approval_target_role_from_task_card(task_card: &Value) -> String {
    task_card
        .pointer("/routing_trace/selected_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "way")
        .map(str::to_string)
        .unwrap_or_else(|| "code specialist".to_string())
}

fn approve_current_planning_task_card(
    run_directory: &Path,
    task_card_id: &str,
    timestamp: &str,
) -> io::Result<()> {
    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_path)?;
    let task_card_object = task_card.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "planning task card must be an object for LongWay approval.",
        )
    })?;
    task_card_object.insert(
        "sequence".to_string(),
        Value::String("EXECUTE_SEQUENCE".to_string()),
    );
    task_card_object.insert(
        "approval_state".to_string(),
        Value::String("approved_for_task_cards".to_string()),
    );
    task_card_object.insert("status".to_string(), Value::String("completed".to_string()));
    task_card_object.insert("dispatch_allowed".to_string(), Value::Bool(false));
    task_card_object.insert(
        "approved_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    task_card_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&task_card_path, &task_card)
}

fn approve_run_sequence_for_task_card(
    run_directory: &Path,
    task_card: &Value,
    timestamp: &str,
    summary: &str,
) -> io::Result<()> {
    let task_card_id = task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "task card is missing id."))?;
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("code specialist");

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert("stage".to_string(), Value::String("execution".to_string()));
    run_object.insert(
        "sequence".to_string(),
        Value::String("EXECUTE_SEQUENCE".to_string()),
    );
    run_object.insert(
        "approval_state".to_string(),
        Value::String("approved_for_task_cards".to_string()),
    );
    run_object.insert(
        "active_task_card_id".to_string(),
        Value::String(task_card_id.to_string()),
    );
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String(summary.to_string()),
    );
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    let run_state_path = run_directory.join("run-state.json");
    let mut run_state_record = read_json_document(&run_state_path)?;
    let run_state_object = run_state_record.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "sequence".to_string(),
        Value::String("EXECUTE_SEQUENCE".to_string()),
    );
    run_state_object.insert(
        "approval_state".to_string(),
        Value::String("approved_for_task_cards".to_string()),
    );
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(phase_name_for_role(assigned_role).to_string()),
    );
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({ "command": "execute_task" }),
    );
    write_json_document(&run_state_path, &run_state_record)?;

    let longway_path = run_directory.join("longway.json");
    let mut longway = read_json_document(&longway_path)?;
    if let Some(object) = longway.as_object_mut() {
        object.insert(
            "lifecycle_state".to_string(),
            Value::String("active".to_string()),
        );
        object.insert(
            "sequence".to_string(),
            Value::String("EXECUTE_SEQUENCE".to_string()),
        );
        object.insert(
            "approval_state".to_string(),
            Value::String("approved_for_task_cards".to_string()),
        );
        object.insert(
            "active_phase_name".to_string(),
            Value::String(phase_name_for_role(assigned_role).to_string()),
        );
        object.insert(
            "active_phase_status".to_string(),
            Value::String("pending".to_string()),
        );
        object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
        if let Some(phases) = object.get_mut("phases").and_then(Value::as_array_mut) {
            for phase in phases {
                if phase
                    .get("task_card_id")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == task_card_id)
                {
                    if let Some(phase_object) = phase.as_object_mut() {
                        phase_object
                            .insert("status".to_string(), Value::String("pending".to_string()));
                        phase_object.insert(
                            "phase_name".to_string(),
                            Value::String(phase_name_for_role(assigned_role).to_string()),
                        );
                    }
                }
            }
        }
    }
    write_json_document(&longway_path, &longway)?;

    let orchestrator_state_path = run_directory.join("orchestrator-state.json");
    let mut orchestrator_state = read_json_document(&orchestrator_state_path)?;
    if let Some(object) = orchestrator_state.as_object_mut() {
        object.insert(
            "sequence".to_string(),
            Value::String("EXECUTE_SEQUENCE".to_string()),
        );
        object.insert(
            "approval_state".to_string(),
            Value::String("approved_for_task_cards".to_string()),
        );
        object.insert(
            "decision".to_string(),
            json!({
                "next_step": "execute_task",
                "can_advance": true,
                "summary": summary
            }),
        );
    }
    write_json_document(&orchestrator_state_path, &orchestrator_state)
}

fn approve_pending_longway(
    run_directory: &Path,
    current_task_card: &Value,
    timestamp: &str,
) -> io::Result<Value> {
    let run_state_path = run_directory.join("run-state.json");
    let run_state = read_json_document(&run_state_path)?;
    let next_action = run_state
        .get("next_action")
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if next_action != "await_longway_approval" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "approve_longway requires a run waiting at await_longway_approval.",
        ));
    }

    let task_card_id = current_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "current task card is missing task_card_id.",
            )
        })?;
    let longway_path = run_directory.join("longway.json");
    let longway = read_json_document(&longway_path)?;
    if next_unmaterialized_planned_row(&longway).is_some() {
        approve_current_planning_task_card(run_directory, task_card_id, timestamp)?;
        let approved_task_card =
            materialize_next_planned_row_task_card(run_directory, current_task_card, timestamp)?
                .ok_or_else(|| {
                    io::Error::new(
                io::ErrorKind::InvalidData,
                "pending LongWay approval found planned rows but none could be materialized.",
            )
                })?;
        let summary = "Captain approved the pending LongWay and materialized the first planned LongWay row for EXECUTE_SEQUENCE dispatch.";
        approve_run_sequence_for_task_card(run_directory, &approved_task_card, timestamp, summary)?;
        let approved_role = approved_task_card
            .get("assigned_role")
            .and_then(Value::as_str)
            .unwrap_or("code specialist");
        let approved_agent_id = approved_task_card
            .get("assigned_agent_id")
            .and_then(Value::as_str)
            .unwrap_or("raider");
        append_run_event(
            run_directory,
            json!({
                "event": "longway_approved",
                "entrypoint": "ccc_orchestrate",
                "task_card_id": approved_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                "assigned_role": approved_role,
                "assigned_agent_id": approved_agent_id,
                "materialized_planned_row": approved_task_card.get("planned_longway_row").cloned().unwrap_or(Value::Null),
                "timestamp": timestamp,
            }),
        )?;
        return Ok(approved_task_card);
    }
    let approved_role = approval_target_role_from_task_card(current_task_card);
    let approved_agent_id = agent_id_for_role(&approved_role).unwrap_or("raider");
    let role_config_snapshot = load_role_config_snapshot(&approved_role);
    let sandbox_mode = sandbox_mode_for_role(&approved_role);
    let sandbox_rationale = sandbox_rationale_for_role(&approved_role);
    let delegation_plan = create_specialist_delegation_plan(
        &approved_role,
        &role_config_snapshot,
        sandbox_mode,
        sandbox_rationale,
    );
    let request_shape = current_task_card
        .pointer("/routing_trace/request_shape")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            infer_request_shape(
                current_task_card
                    .get("execution_prompt")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
        });
    let task_shape = infer_task_shape(
        current_task_card
            .get("execution_prompt")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        request_shape,
    );

    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_path)?;
    let task_card_object = task_card.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "task card must be an object for LongWay approval.",
        )
    })?;
    task_card_object.insert(
        "sequence".to_string(),
        Value::String("EXECUTE_SEQUENCE".to_string()),
    );
    task_card_object.insert(
        "approval_state".to_string(),
        Value::String("approved_for_task_cards".to_string()),
    );
    task_card_object.insert("status".to_string(), Value::String("active".to_string()));
    task_card_object.insert(
        "node_kind".to_string(),
        Value::String("execution".to_string()),
    );
    task_card_object.insert("dispatch_allowed".to_string(), Value::Bool(true));
    task_card_object.insert(
        "task_kind".to_string(),
        Value::String(task_kind_for_approved_role(&approved_role).to_string()),
    );
    task_card_object.insert(
        "assigned_role".to_string(),
        Value::String(approved_role.clone()),
    );
    task_card_object.insert(
        "assigned_agent_id".to_string(),
        Value::String(approved_agent_id.to_string()),
    );
    task_card_object.insert(
        "sandbox_mode".to_string(),
        Value::String(sandbox_mode.to_string()),
    );
    task_card_object.insert(
        "sandbox_rationale".to_string(),
        Value::String(sandbox_rationale.to_string()),
    );
    task_card_object.insert("role_config_snapshot".to_string(), role_config_snapshot);
    task_card_object.insert("delegation_plan".to_string(), delegation_plan);
    task_card_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    apply_task_expertise_framing(&mut task_card, &approved_role, task_shape);
    write_json_document(&task_card_path, &task_card)?;

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    run_object.insert("stage".to_string(), Value::String("execution".to_string()));
    run_object.insert(
        "sequence".to_string(),
        Value::String("EXECUTE_SEQUENCE".to_string()),
    );
    run_object.insert(
        "approval_state".to_string(),
        Value::String("approved_for_task_cards".to_string()),
    );
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String(
            "Captain approved the pending LongWay and opened EXECUTE_SEQUENCE task dispatch."
                .to_string(),
        ),
    );
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    let mut run_state_record = read_json_document(&run_state_path)?;
    let run_state_object = run_state_record.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "run-state.json must be an object.",
        )
    })?;
    run_state_object.insert(
        "sequence".to_string(),
        Value::String("EXECUTE_SEQUENCE".to_string()),
    );
    run_state_object.insert(
        "approval_state".to_string(),
        Value::String("approved_for_task_cards".to_string()),
    );
    run_state_object.insert(
        "current_phase_name".to_string(),
        Value::String(phase_name_for_role(&approved_role).to_string()),
    );
    run_state_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    run_state_object.insert(
        "next_action".to_string(),
        json!({ "command": "execute_task" }),
    );
    write_json_document(&run_state_path, &run_state_record)?;

    let longway_path = run_directory.join("longway.json");
    let mut longway = read_json_document(&longway_path)?;
    if let Some(object) = longway.as_object_mut() {
        object.insert(
            "lifecycle_state".to_string(),
            Value::String("active".to_string()),
        );
        object.insert(
            "sequence".to_string(),
            Value::String("EXECUTE_SEQUENCE".to_string()),
        );
        object.insert(
            "approval_state".to_string(),
            Value::String("approved_for_task_cards".to_string()),
        );
        object.insert(
            "active_phase_name".to_string(),
            Value::String(phase_name_for_role(&approved_role).to_string()),
        );
        object.insert(
            "active_phase_status".to_string(),
            Value::String("pending".to_string()),
        );
        object.insert(
            "updated_at".to_string(),
            Value::String(timestamp.to_string()),
        );
        if let Some(phases) = object.get_mut("phases").and_then(Value::as_array_mut) {
            for phase in phases {
                if phase
                    .get("task_card_id")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == task_card_id)
                {
                    if let Some(phase_object) = phase.as_object_mut() {
                        phase_object
                            .insert("status".to_string(), Value::String("pending".to_string()));
                        phase_object.insert(
                            "phase_name".to_string(),
                            Value::String(phase_name_for_role(&approved_role).to_string()),
                        );
                    }
                }
            }
        }
    }
    write_json_document(&longway_path, &longway)?;

    let orchestrator_state_path = run_directory.join("orchestrator-state.json");
    let mut orchestrator_state = read_json_document(&orchestrator_state_path)?;
    if let Some(object) = orchestrator_state.as_object_mut() {
        object.insert(
            "sequence".to_string(),
            Value::String("EXECUTE_SEQUENCE".to_string()),
        );
        object.insert(
            "approval_state".to_string(),
            Value::String("approved_for_task_cards".to_string()),
        );
        object.insert(
            "decision".to_string(),
            json!({
                "next_step": "execute_task",
                "can_advance": true,
                "summary": "LongWay approval opened EXECUTE_SEQUENCE task dispatch."
            }),
        );
    }
    write_json_document(&orchestrator_state_path, &orchestrator_state)?;

    append_run_event(
        run_directory,
        json!({
            "event": "longway_approved",
            "entrypoint": "ccc_orchestrate",
            "task_card_id": task_card_id,
            "assigned_role": approved_role,
            "assigned_agent_id": approved_agent_id,
            "timestamp": timestamp,
        }),
    )?;

    Ok(task_card)
}

struct SchedulerRuntimeDecisionInput<'a> {
    run_directory: &'a Path,
    post_fan_in_next_step: &'a str,
    resolved_summary_present: bool,
    follow_up_task_card_present: bool,
    retry_current_specialist: bool,
    dispatched_execution: bool,
    dispatched_worker_terminal: Option<&'a str>,
    reclaimed_worker: bool,
    collapsed_worker_fan_in: bool,
}

fn create_scheduler_runtime_decision(
    input: SchedulerRuntimeDecisionInput<'_>,
) -> io::Result<Value> {
    let (kind, reason, next_step_after_attempt, can_advance_after_attempt) =
        if input.resolved_summary_present {
            let next_step = read_json_document(&input.run_directory.join("run-state.json"))?
                .get("next_action")
                .and_then(|value| value.get("command"))
                .and_then(Value::as_str)
                .unwrap_or("halt_completed")
                .to_string();
            (
                "complete",
                "resolve_outcome closed the active run",
                next_step,
                false,
            )
        } else if input.follow_up_task_card_present {
            (
                "replan",
                "captain follow-up task card is ready for explicit specialist dispatch",
                "execute_task".to_string(),
                true,
            )
        } else if input.retry_current_specialist {
            (
                "retry",
                "repair_action selected a bounded retry of the current specialist",
                "execute_task".to_string(),
                true,
            )
        } else if input.dispatched_execution {
            (
                "dispatch",
                "scheduler dispatched a bounded worker and is waiting for fan-in",
                "await_fan_in".to_string(),
                input.dispatched_worker_terminal.is_some(),
            )
        } else if input.reclaimed_worker {
            (
                "blocked_reclaim",
                "scheduler reclaimed stuck worker state and reopened captain advance",
                "advance".to_string(),
                true,
            )
        } else if input.collapsed_worker_fan_in {
            (
                "continue",
                "scheduler consumed compact fan-in and reopened captain advance",
                "advance".to_string(),
                true,
            )
        } else if input.post_fan_in_next_step == "advance" {
            (
                "checkpoint",
                "scheduler preserved an explicit captain advance checkpoint",
                "advance".to_string(),
                true,
            )
        } else {
            (
                "wait",
                "scheduler is waiting for the current runtime step to become actionable",
                input.post_fan_in_next_step.to_string(),
                true,
            )
        };

    Ok(json!({
        "schema": "ccc.scheduler_runtime_decision.v1",
        "owner": "scheduler",
        "action": {
            "kind": kind,
            "reason": reason,
            "next_step_after_attempt": next_step_after_attempt,
            "can_advance_after_attempt": can_advance_after_attempt,
        }
    }))
}

fn task_card_is_mutation_capable(task_card: &Value) -> bool {
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let assigned_agent_id = task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let sandbox_mode = task_card
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .unwrap_or_default();

    matches!(
        assigned_role,
        "code specialist" | "documenter" | "companion_operator"
    ) || matches!(
        assigned_agent_id,
        "raider" | "ccc_raider" | "scribe" | "ccc_scribe" | "companion_operator"
    ) || sandbox_mode == "workspace-write"
}

fn value_has_non_empty_string(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|text| !text.is_empty())
}

fn value_has_non_empty_string_array(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.as_str()
                    .map(str::trim)
                    .is_some_and(|text| !text.is_empty())
            })
        })
}

fn approval_state_is_approved(value: &Value) -> bool {
    value
        .get("approval_state")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|state| state == "approved_for_task_cards")
}

fn mutation_gate_approved_longway_sources(
    run_record: &Value,
    longway: &Value,
    task_card: &Value,
) -> Vec<Value> {
    let mut sources = Vec::new();
    if approval_state_is_approved(run_record) {
        sources.push(json!("run.approval_state"));
    }
    if approval_state_is_approved(longway) {
        sources.push(json!("longway.approval_state"));
    }
    if approval_state_is_approved(task_card) {
        sources.push(json!("task_card.approval_state"));
    }
    sources
}

fn mutation_gate_evidence_sources(task_card: &Value) -> Vec<Value> {
    let evidence_checks = [
        ("task_card.evidence_summary", "/evidence_summary"),
        ("task_card.evidence_links", "/evidence_links"),
        ("task_card.result_links", "/result_links"),
        (
            "task_card.planned_longway_row.evidence_summary",
            "/planned_longway_row/evidence_summary",
        ),
        (
            "task_card.planned_longway_row.evidence_links",
            "/planned_longway_row/evidence_links",
        ),
        (
            "task_card.planned_longway_row.result_links",
            "/planned_longway_row/result_links",
        ),
        (
            "task_card.subagent_fan_in.summary",
            "/subagent_fan_in/summary",
        ),
        (
            "task_card.subagent_fan_in.evidence_paths",
            "/subagent_fan_in/evidence_paths",
        ),
        (
            "task_card.worker_result_envelope.summary",
            "/worker_result_envelope/summary",
        ),
        (
            "task_card.worker_result_envelope.evidence_paths",
            "/worker_result_envelope/evidence_paths",
        ),
    ];

    evidence_checks
        .iter()
        .filter_map(|(source, pointer)| {
            if value_has_non_empty_string(task_card, pointer)
                || value_has_non_empty_string_array(task_card, pointer)
            {
                Some(json!(*source))
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn create_mutation_evidence_gate_payload(
    run_record: &Value,
    longway: &Value,
    task_card: &Value,
) -> Value {
    let applies = task_card_is_mutation_capable(task_card);
    let approved_longway_sources =
        mutation_gate_approved_longway_sources(run_record, longway, task_card);
    let evidence_sources = mutation_gate_evidence_sources(task_card);
    let approved_longway = !approved_longway_sources.is_empty();
    let persisted_evidence = !evidence_sources.is_empty();
    let blocked = applies && !approved_longway && !persisted_evidence;
    let state = if !applies {
        "not_applicable"
    } else if blocked {
        "blocked_missing_evidence"
    } else {
        "allowed"
    };
    let summary = if blocked {
        "Mutation dispatch is blocked until persisted evidence or approved LongWay scope exists."
    } else if approved_longway {
        "Mutation dispatch is allowed by persisted approved LongWay state."
    } else if persisted_evidence {
        "Mutation dispatch is allowed by persisted task-card or fan-in evidence."
    } else {
        "Mutation evidence gate does not apply to this task."
    };

    json!({
        "schema": "ccc.mutation_evidence_gate.v1",
        "source": "ccc_orchestrate",
        "applies": applies,
        "state": state,
        "blocked": blocked,
        "approved_longway": approved_longway,
        "persisted_evidence": persisted_evidence,
        "approved_longway_sources": approved_longway_sources,
        "evidence_sources": evidence_sources,
        "required_evidence": [
            "persisted approved LongWay approval_state",
            "task-card evidence_summary/evidence_links/result_links",
            "subagent or worker fan-in summary/evidence_paths"
        ],
        "required_action": if blocked {
            "record_fan_in_evidence_or_approve_longway_before_mutation_dispatch"
        } else {
            "none"
        },
        "summary": summary,
        "task_card_id": task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
        "assigned_role": task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
        "assigned_agent_id": task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
    })
}

fn mutation_evidence_gate_blocks_dispatch(gate: &Value) -> bool {
    gate.get("blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn create_ccc_orchestrate_payload(parsed: &Value) -> io::Result<Value> {
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": parsed.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": parsed.get("run_ref").cloned().unwrap_or(Value::Null),
            "run_dir": parsed.get("run_directory").cloned().unwrap_or(Value::Null),
            "cwd": parsed.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_orchestrate",
    )?;
    let orchestrator_state_file = locator.run_directory.join("orchestrator-state.json");
    let _run_lock = acquire_run_mutation_lock(&locator.run_directory, "ccc_orchestrate")?;
    let mut orchestrator_state = read_json_document(&orchestrator_state_file)?;
    let run_state = create_run_state_payload(&locator.run_directory)?;
    let current_next_step = run_state
        .get("next_action")
        .and_then(|value| {
            value
                .get("command")
                .or_else(|| value.get("action"))
                .or_else(|| value.get("type"))
        })
        .and_then(Value::as_str)
        .or_else(|| {
            orchestrator_state
                .get("decision")
                .and_then(|value| value.get("next_step"))
                .and_then(Value::as_str)
        })
        .unwrap_or("advance");
    let current_next_step = current_next_step.to_string();

    let requested_progression_mode = resolve_requested_progression_mode(parsed);
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let (attempt_id, attempt_file) = next_orchestration_attempt_file(&locator.run_directory)?;
    let runtime_config = load_runtime_config()?;
    let effective_codex_bin = resolve_effective_codex_bin(parsed, Some(&locator.run_directory));
    let pre_attempt_run_record = read_json_document(&locator.run_directory.join("run.json"))?;
    let current_task_card = create_current_task_card_payload(
        &locator.run_directory,
        pre_attempt_run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
    )?;
    let requested_repair_action = parsed.get("repair_action").and_then(Value::as_str);
    let requested_replan_prompt = parsed.get("replan_prompt").and_then(Value::as_str);
    let requested_resolve_outcome = parsed.get("resolve_outcome").and_then(Value::as_str);
    let requested_resolve_summary = parsed.get("resolve_summary").and_then(Value::as_str);
    let requested_approve_longway = parsed
        .get("approve_longway")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if current_next_step == "await_operator" {
        if let Some(clarification_answer) = requested_replan_prompt {
            let clarification_consumption = consume_way_clarification_answer(
                &locator.run_directory,
                &current_task_card,
                clarification_answer,
                &timestamp,
            )?;
            let post_decision_run_record =
                read_json_document(&locator.run_directory.join("run.json"))?;
            let post_decision_longway =
                read_json_document(&locator.run_directory.join("longway.json"))?;
            let post_decision_task_card = create_current_task_card_payload(
                &locator.run_directory,
                post_decision_run_record
                    .get("active_task_card_id")
                    .and_then(Value::as_str),
            )?;
            let no_host_subagent_state = Value::Null;
            let post_fan_in_captain_decision = create_post_fan_in_captain_decision_payload(
                &post_decision_run_record,
                &post_decision_longway,
                &post_decision_task_card,
                &no_host_subagent_state,
                "await_longway_approval",
                false,
                false,
                0,
                0,
            );
            let attempt_payload = json!({
                "attempt_id": attempt_id,
                "run_id": locator.run_id,
                "started_at": timestamp,
                "entrypoint": "ccc_orchestrate",
                "way_clarification_consumption": clarification_consumption,
                "post_fan_in_captain_decision": post_fan_in_captain_decision.clone(),
                "scheduler_decision": {
                    "schema": "ccc.scheduler_decision.v1",
                    "decision_source": "way_clarification_answer",
                    "starting_next_step": current_next_step,
                    "next_step_after_attempt": "await_longway_approval",
                    "can_advance_after_attempt": false,
                    "selected_task_card_id": current_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                    "selected_role": current_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
                    "selected_agent_id": current_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
                    "action": {
                        "kind": "consume_way_clarification_answer",
                        "reason": "scheduler consumed the operator's Way clarification answer and regenerated pending LongWay rows",
                        "next_step_after_attempt": "await_longway_approval",
                        "can_advance_after_attempt": false
                    },
                    "post_fan_in_captain_decision": post_fan_in_captain_decision
                },
                "starting_next_step": current_next_step,
                "next_step_after_attempt": "await_longway_approval",
                "can_advance_after_attempt": false,
                "summary": "Captain consumed the Way clarification answer and regenerated the pending LongWay for explicit approval.",
            });
            write_json_document(&attempt_file, &attempt_payload)?;
            return Ok(json!({
                "cwd": locator.cwd.to_string_lossy(),
                "run_id": locator.run_id,
                "run_directory": locator.run_directory.to_string_lossy(),
                "run_ref": create_ccc_run_ref(&locator.run_directory),
                "attempt_id": attempt_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
                "starting_next_step": current_next_step,
                "next_step": "await_longway_approval",
                "can_advance": false,
                "advanced": true,
                "progression_mode": requested_progression_mode,
                "summary": attempt_payload.get("summary").cloned().unwrap_or(Value::Null),
                "way_clarification_consumption": attempt_payload.get("way_clarification_consumption").cloned().unwrap_or(Value::Null),
                "scheduler_decision": attempt_payload.get("scheduler_decision").cloned().unwrap_or(Value::Null),
                "post_fan_in_captain_decision": attempt_payload.get("post_fan_in_captain_decision").cloned().unwrap_or(Value::Null),
                "launch_result": Value::Null,
                "reclaimed_targets": Value::Array(Vec::new()),
                "collapsed_fan_in": Value::Null,
                "consumed_pending_follow_up": Value::Null,
                "allowed_next_commands": ["approve_longway"],
            }));
        }
    }
    if requested_approve_longway {
        if current_next_step != "await_longway_approval" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "approve_longway requires current next_step=await_longway_approval.",
            ));
        }
        let approved_task_card =
            approve_pending_longway(&locator.run_directory, &current_task_card, &timestamp)?;
        let materialized_planned_row = approved_task_card
            .get("planned_longway_row")
            .cloned()
            .unwrap_or(Value::Null);
        let summary = if materialized_planned_row.is_null() {
            "Captain approved the pending LongWay and opened EXECUTE_SEQUENCE task dispatch."
                .to_string()
        } else {
            "Captain approved the pending LongWay and materialized the first planned LongWay row for EXECUTE_SEQUENCE dispatch.".to_string()
        };
        let post_decision_run_record = read_json_document(&locator.run_directory.join("run.json"))?;
        let post_decision_longway =
            read_json_document(&locator.run_directory.join("longway.json"))?;
        let no_host_subagent_state = Value::Null;
        let post_fan_in_captain_decision = create_post_fan_in_captain_decision_payload(
            &post_decision_run_record,
            &post_decision_longway,
            &approved_task_card,
            &no_host_subagent_state,
            "execute_task",
            true,
            false,
            0,
            0,
        );
        let attempt_payload = json!({
            "attempt_id": attempt_id,
            "run_id": locator.run_id,
            "started_at": timestamp,
            "entrypoint": "ccc_orchestrate",
            "approval_transition": {
                "approved": true,
                "from_sequence": "PLAN_SEQUENCE",
                "to_sequence": "EXECUTE_SEQUENCE",
                "task_card_id": approved_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                "assigned_role": approved_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
                "assigned_agent_id": approved_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
                "materialized_planned_row": materialized_planned_row.clone()
            },
            "post_fan_in_captain_decision": post_fan_in_captain_decision.clone(),
            "scheduler_decision": {
                "schema": "ccc.scheduler_decision.v1",
                "decision_source": if materialized_planned_row.is_null() { "approved_longway_task_cards" } else { "planned_row_materialization" },
                "starting_next_step": current_next_step,
                "next_step_after_attempt": "execute_task",
                "can_advance_after_attempt": true,
                "selected_task_card_id": approved_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                "selected_role": approved_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
                "selected_agent_id": approved_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
                "selected_planned_row": materialized_planned_row.clone(),
                "action": {
                    "kind": if materialized_planned_row.is_null() { "approve_longway" } else { "materialize_planned_row" },
                    "reason": if materialized_planned_row.is_null() {
                        "scheduler approved the pending LongWay and opened the approved task card for dispatch"
                    } else {
                        "scheduler approved the pending LongWay and materialized the selected planned row"
                    },
                    "next_step_after_attempt": "execute_task",
                    "can_advance_after_attempt": true
                },
                "owns": {
                    "next_task_selection": true,
                    "planned_row_materialization": true,
                    "bounded_parallel_fanout": true,
                    "blocked_work": true,
                    "pending_card_updates": true
                },
                "post_fan_in_captain_decision": post_fan_in_captain_decision
            },
            "starting_next_step": current_next_step,
            "next_step_after_attempt": "execute_task",
            "can_advance_after_attempt": true,
            "summary": summary,
        });
        write_json_document(&attempt_file, &attempt_payload)?;
        return Ok(json!({
            "cwd": locator.cwd.to_string_lossy(),
            "run_id": locator.run_id,
            "run_directory": locator.run_directory.to_string_lossy(),
            "run_ref": create_ccc_run_ref(&locator.run_directory),
            "attempt_id": attempt_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
            "starting_next_step": current_next_step,
            "next_step": "execute_task",
            "can_advance": true,
            "advanced": true,
            "progression_mode": requested_progression_mode,
            "summary": summary,
            "approval_transition": attempt_payload.get("approval_transition").cloned().unwrap_or(Value::Null),
            "scheduler_decision": attempt_payload.get("scheduler_decision").cloned().unwrap_or(Value::Null),
            "post_fan_in_captain_decision": attempt_payload.get("post_fan_in_captain_decision").cloned().unwrap_or(Value::Null),
            "launch_result": Value::Null,
            "reclaimed_targets": Value::Array(Vec::new()),
            "collapsed_fan_in": Value::Null,
            "consumed_pending_follow_up": Value::Null,
            "allowed_next_commands": ["advance"],
        }));
    }
    let reclaimed_targets = if current_next_step == "await_fan_in" {
        reclaim_stuck_worker_delegations(
            &locator.run_directory,
            current_task_card
                .get("task_card_id")
                .and_then(Value::as_str),
            &runtime_config,
        )?
    } else {
        Vec::new()
    };
    let collapsed_fan_in = if current_next_step == "await_fan_in" && reclaimed_targets.is_empty() {
        collapse_terminal_fan_in(
            &locator.run_directory,
            &current_task_card,
            "Run reopened for captain follow-up.",
        )?
    } else {
        None
    };
    let post_fan_in_next_step = if current_next_step == "await_fan_in"
        && reclaimed_targets.is_empty()
        && collapsed_fan_in.is_some()
    {
        "advance"
    } else {
        current_next_step.as_str()
    };
    let queued_pending_follow_up = if post_fan_in_next_step == "advance"
        && requested_replan_prompt.is_none()
        && requested_resolve_outcome.is_none()
    {
        queued_pending_captain_follow_up(&current_task_card)
    } else {
        None
    };
    let consumed_pending_follow_up = queued_pending_follow_up
        .as_ref()
        .map(|pending_follow_up| {
            create_follow_up_task_card_from_pending_follow_up(
                &locator.run_directory,
                &current_task_card,
                pending_follow_up,
                &timestamp,
            )
        })
        .transpose()?;
    let consumed_pending_follow_up_for_attempt = consumed_pending_follow_up
        .as_ref()
        .and_then(|task_card| task_card.get("captain_follow_up"))
        .cloned()
        .or_else(|| {
            let task_card_id = consumed_pending_follow_up
                .as_ref()?
                .get("task_card_id")
                .and_then(Value::as_str)?;
            queued_pending_follow_up.as_ref().map(|pending| {
                consumed_pending_follow_up_payload(pending, task_card_id, &timestamp)
            })
        });
    let requested_completion_outcome = requested_resolve_outcome
        .map(str::trim)
        .map(|value| value.to_ascii_lowercase())
        .is_some_and(|value| {
            matches!(
                value.as_str(),
                "completed" | "done" | "resolved" | "success"
            )
        });
    let required_review_task_card = if post_fan_in_next_step == "advance"
        && consumed_pending_follow_up.is_none()
        && requested_completion_outcome
    {
        maybe_require_arbiter_review_before_completion(
            &locator.run_directory,
            &current_task_card,
            &timestamp,
        )?
    } else {
        None
    };
    let resolved_summary = if post_fan_in_next_step == "advance" {
        requested_resolve_outcome
            .filter(|_| required_review_task_card.is_none())
            .map(|resolve_outcome| {
                mark_run_resolved(
                    &locator.run_directory,
                    &current_task_card,
                    resolve_outcome,
                    requested_resolve_summary,
                    &timestamp,
                )
            })
            .transpose()?
    } else {
        None
    };
    let requested_repair_action_is_empty = requested_repair_action
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none();
    let materialized_planned_task_card = if post_fan_in_next_step == "advance"
        && consumed_pending_follow_up.is_none()
        && required_review_task_card.is_none()
        && requested_replan_prompt.is_none()
        && requested_resolve_outcome.is_none()
        && requested_repair_action_is_empty
    {
        materialize_next_planned_row_task_card(
            &locator.run_directory,
            &current_task_card,
            &timestamp,
        )?
    } else {
        None
    };
    let follow_up_task_card = if post_fan_in_next_step == "advance" {
        if required_review_task_card.is_some() {
            required_review_task_card
        } else if consumed_pending_follow_up.is_some() {
            consumed_pending_follow_up
        } else if materialized_planned_task_card.is_some() {
            materialized_planned_task_card
        } else {
            requested_replan_prompt
                .map(|replan_prompt| {
                    create_follow_up_task_card(
                        &locator.run_directory,
                        &current_task_card,
                        requested_repair_action,
                        replan_prompt,
                        requested_resolve_summary,
                        &timestamp,
                    )
                })
                .transpose()?
        }
    } else {
        None
    };
    let consumed_queued_captain_follow_up = queued_pending_follow_up.is_some();
    let retry_current_specialist = post_fan_in_next_step == "advance"
        && resolved_summary.is_none()
        && follow_up_task_card.is_none()
        && matches!(
            requested_repair_action
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase())
                .as_deref(),
            Some("retry" | "retry_current_specialist" | "retry-current-specialist")
        );
    let effective_task_card = follow_up_task_card.as_ref().unwrap_or(&current_task_card);
    let effective_delegation_plan = effective_task_card
        .get("delegation_plan")
        .cloned()
        .unwrap_or(Value::Null);
    let preferred_execution_mode = preferred_specialist_execution_mode(&runtime_config);
    let fallback_execution_mode = fallback_specialist_execution_mode(&runtime_config);
    let subagent_available = effective_delegation_plan
        .get("subagent_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let subagent_fallback_ready = task_card_subagent_fallback_ready(
        &pre_attempt_run_record,
        effective_task_card,
        pre_attempt_run_record
            .get("active_task_card_id")
            .and_then(Value::as_str),
    );
    let explicit_codex_bin_override = parsed
        .get("codex_bin")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let custom_subagent_spawn_required =
        preferred_execution_mode == "codex_subagent" && subagent_available;
    let active_worker_delegation = task_has_active_worker_delegation(
        &locator.run_directory,
        effective_task_card
            .get("task_card_id")
            .and_then(Value::as_str),
    )?;
    let codex_exec_recovery_dispatch = current_next_step == "await_fan_in"
        && collapsed_fan_in.is_none()
        && reclaimed_targets.is_empty()
        && subagent_fallback_ready
        && (fallback_execution_mode == "codex_exec" || explicit_codex_bin_override)
        && !active_worker_delegation;
    let codex_exec_dispatch_allowed = (explicit_codex_bin_override
        && (!custom_subagent_spawn_required || subagent_fallback_ready))
        || (fallback_execution_mode == "codex_exec"
            && (preferred_execution_mode != "codex_subagent"
                || !subagent_available
                || subagent_fallback_ready));
    let longway_for_decision = read_json_document(&locator.run_directory.join("longway.json"))?;
    let attempt_host_subagent_state = if !reclaimed_targets.is_empty() {
        json!({
            "recovery_recommendation": {
                "recommended_action": "reclaim",
                "needs_operator_attention": true,
                "summary": "Persisted orchestration attempt reclaimed stuck worker state."
            }
        })
    } else if retry_current_specialist {
        json!({
            "recovery_recommendation": {
                "recommended_action": "retry",
                "needs_operator_attention": true,
                "summary": "Persisted orchestration attempt selected a bounded specialist retry."
            }
        })
    } else {
        Value::Null
    };
    let dispatch_step_requested =
        post_fan_in_next_step == "execute_task" || codex_exec_recovery_dispatch;
    let mutation_evidence_gate = if dispatch_step_requested && codex_exec_dispatch_allowed {
        create_mutation_evidence_gate_payload(
            &pre_attempt_run_record,
            &longway_for_decision,
            effective_task_card,
        )
    } else {
        Value::Null
    };
    let mut post_fan_in_decision_run_record = pre_attempt_run_record.clone();
    if !mutation_evidence_gate.is_null() {
        if let Some(object) = post_fan_in_decision_run_record.as_object_mut() {
            object.insert(
                "mutation_evidence_gate".to_string(),
                mutation_evidence_gate.clone(),
            );
        }
    }
    let post_fan_in_captain_decision = create_post_fan_in_captain_decision_payload(
        &post_fan_in_decision_run_record,
        &longway_for_decision,
        effective_task_card,
        &attempt_host_subagent_state,
        post_fan_in_next_step,
        post_fan_in_next_step == "advance" || collapsed_fan_in.is_some(),
        collapsed_fan_in.is_some(),
        0,
        0,
    );
    if mutation_evidence_gate_blocks_dispatch(&mutation_evidence_gate) {
        let summary = mutation_evidence_gate
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Mutation dispatch is blocked by the evidence-before-mutation gate.");
        let attempt_payload = json!({
            "attempt_id": attempt_id,
            "entrypoint": "ccc_orchestrate",
            "run_id": locator.run_id,
            "task_card_id": effective_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
            "started_at": timestamp,
            "completed_at": timestamp,
            "requested_progression_mode": requested_progression_mode,
            "starting_next_step": current_next_step,
            "mutation_evidence_gate": mutation_evidence_gate,
            "post_fan_in_captain_decision": post_fan_in_captain_decision,
            "scheduler_decision": {
                "schema": "ccc.scheduler_decision.v1",
                "decision_source": "mutation_evidence_gate",
                "starting_next_step": current_next_step,
                "post_fan_in_next_step": post_fan_in_next_step,
                "next_step_after_attempt": "execute_task",
                "can_advance_after_attempt": false,
                "selected_task_card_id": effective_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                "selected_role": effective_task_card.get("assigned_role").cloned().unwrap_or(Value::Null),
                "selected_agent_id": effective_task_card.get("assigned_agent_id").cloned().unwrap_or(Value::Null),
                "action": {
                    "kind": post_fan_in_captain_decision.pointer("/scheduler_action/kind").and_then(Value::as_str).unwrap_or("blocked"),
                    "reason": summary,
                    "next_step_after_attempt": "execute_task",
                    "can_advance_after_attempt": false
                },
                "post_fan_in_captain_decision": post_fan_in_captain_decision,
                "blocked": {
                    "mutation_evidence_gate": true,
                    "reason": summary
                },
                "owns": {
                    "next_task_selection": true,
                    "planned_row_materialization": true,
                    "bounded_parallel_fanout": true,
                    "blocked_work": true,
                    "pending_card_updates": true
                }
            },
            "stop": {
                "reason": "blocked_mutation_evidence_gate",
                "summary": summary
            },
            "launch_result": Value::Null,
        });
        write_json_document(&attempt_file, &attempt_payload)?;

        if let Some(orchestrator_object) = orchestrator_state.as_object_mut() {
            orchestrator_object.insert(
                "decision".to_string(),
                json!({
                    "next_step": "execute_task",
                    "can_advance": false,
                    "summary": summary
                }),
            );
            orchestrator_object.insert(
                "mutation_evidence_gate".to_string(),
                attempt_payload
                    .get("mutation_evidence_gate")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        write_json_document(&orchestrator_state_file, &orchestrator_state)?;

        let run_file = locator.run_directory.join("run.json");
        let mut run_record = read_json_document(&run_file)?;
        if let Some(run_object) = run_record.as_object_mut() {
            run_object.insert("updated_at".to_string(), Value::String(timestamp.clone()));
            run_object.insert(
                "latest_orchestrator_synthesis".to_string(),
                Value::String(summary.to_string()),
            );
            run_object.insert(
                "mutation_evidence_gate".to_string(),
                attempt_payload
                    .get("mutation_evidence_gate")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            run_object.insert(
                "latest_entry_trace".to_string(),
                json!({
                    "entrypoint": "ccc_orchestrate",
                    "attempt_id": attempt_id,
                    "requested_progression_mode": requested_progression_mode,
                    "current_next_step": current_next_step,
                    "codex_bin": effective_codex_bin,
                    "completed_at": timestamp,
                }),
            );
        }
        write_json_document(&run_file, &run_record)?;

        append_run_event(
            &locator.run_directory,
            json!({
                "event": "mutation_evidence_gate_blocked",
                "entrypoint": "ccc_orchestrate",
                "run_id": locator.run_id,
                "attempt_id": attempt_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
                "task_card_id": effective_task_card.get("task_card_id").cloned().unwrap_or(Value::Null),
                "timestamp": timestamp,
            }),
        )?;

        return Ok(json!({
            "cwd": locator.cwd.to_string_lossy(),
            "run_id": locator.run_id,
            "run_directory": locator.run_directory.to_string_lossy(),
            "run_ref": create_ccc_run_ref(&locator.run_directory),
            "attempt_id": attempt_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
            "starting_next_step": current_next_step,
            "next_step": "execute_task",
            "can_advance": false,
            "advanced": true,
            "progression_mode": requested_progression_mode,
            "summary": summary,
            "scheduler_decision": attempt_payload.get("scheduler_decision").cloned().unwrap_or(Value::Null),
            "post_fan_in_captain_decision": attempt_payload.get("post_fan_in_captain_decision").cloned().unwrap_or(Value::Null),
            "mutation_evidence_gate": attempt_payload.get("mutation_evidence_gate").cloned().unwrap_or(Value::Null),
            "launch_result": Value::Null,
            "reclaimed_targets": Value::Array(Vec::new()),
            "collapsed_fan_in": Value::Null,
            "consumed_worker_result_envelope": Value::Null,
            "consumed_pending_follow_up": Value::Null,
            "allowed_next_commands": ["record_fan_in", "approve_longway"],
        }));
    }
    let launch_result = if resolved_summary.is_some()
        || follow_up_task_card.is_some()
        || retry_current_specialist
    {
        None
    } else if (post_fan_in_next_step == "execute_task" || codex_exec_recovery_dispatch)
        && codex_exec_dispatch_allowed
    {
        Some(spawn_codex_exec_for_task(
            &locator.cwd,
            &locator.run_directory,
            &effective_codex_bin,
            effective_task_card,
        )?)
    } else {
        None
    };
    let dispatched_execution = launch_result.is_some();
    let dispatched_worker_terminal = launch_result
        .as_ref()
        .and_then(|value| value.get("terminal_status"))
        .and_then(Value::as_str);
    let dispatched_worker_state = launch_result
        .as_ref()
        .and_then(|value| value.get("worker_state"))
        .and_then(Value::as_str)
        .unwrap_or("launching");
    let reclaimed_worker = !reclaimed_targets.is_empty();
    let collapsed_worker_fan_in = collapsed_fan_in.is_some();
    let scheduler_runtime_decision =
        create_scheduler_runtime_decision(SchedulerRuntimeDecisionInput {
            run_directory: &locator.run_directory,
            post_fan_in_next_step,
            resolved_summary_present: resolved_summary.is_some(),
            follow_up_task_card_present: follow_up_task_card.is_some(),
            retry_current_specialist,
            dispatched_execution,
            dispatched_worker_terminal,
            reclaimed_worker,
            collapsed_worker_fan_in,
        })?;
    let next_step_after_attempt = scheduler_runtime_decision
        .pointer("/action/next_step_after_attempt")
        .and_then(Value::as_str)
        .unwrap_or(post_fan_in_next_step)
        .to_string();
    let can_advance_after_attempt = scheduler_runtime_decision
        .pointer("/action/can_advance_after_attempt")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let attempt = create_orchestration_attempt_payload(OrchestrationAttemptPayloadInput {
        attempt_id: &attempt_id,
        run_id: &locator.run_id,
        timestamp: &timestamp,
        requested_progression_mode: &requested_progression_mode,
        starting_next_step: &current_next_step,
        post_fan_in_next_step,
        codex_bin: &effective_codex_bin,
        effective_task_card,
        current_task_card: &current_task_card,
        effective_delegation_plan: &effective_delegation_plan,
        resolved_summary: resolved_summary.as_deref(),
        follow_up_task_card: follow_up_task_card.as_ref(),
        retry_current_specialist,
        launch_result: launch_result.as_ref(),
        reclaimed_targets: &reclaimed_targets,
        collapsed_fan_in: collapsed_fan_in.as_ref(),
        consumed_pending_follow_up_for_attempt: consumed_pending_follow_up_for_attempt.as_ref(),
        consumed_queued_captain_follow_up,
        preferred_execution_mode: &preferred_execution_mode,
        subagent_available,
        codex_exec_dispatch_allowed,
        dispatched_worker_terminal,
        dispatched_worker_state,
        scheduler_runtime_decision: &scheduler_runtime_decision,
        post_fan_in_captain_decision: &post_fan_in_captain_decision,
        next_step_after_attempt: &next_step_after_attempt,
        can_advance_after_attempt,
    });
    let summary = attempt.summary;
    let attempt_payload = attempt.payload;
    write_json_document(&attempt_file, &attempt_payload)?;

    apply_orchestrator_state_after_attempt(
        &mut orchestrator_state,
        OrchestratorStateUpdateInput {
            next_step_after_attempt: &next_step_after_attempt,
            can_advance_after_attempt,
            summary: &summary,
            launch_result: launch_result.as_ref(),
            codex_bin: &effective_codex_bin,
            timestamp: &timestamp,
        },
    )?;
    write_json_document(&orchestrator_state_file, &orchestrator_state)?;

    let run_file = locator.run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    apply_run_record_after_attempt(
        &mut run_record,
        RunRecordUpdateInput {
            timestamp: &timestamp,
            summary: &summary,
            attempt_id: &attempt_id,
            requested_progression_mode: &requested_progression_mode,
            current_next_step: &current_next_step,
            codex_bin: &effective_codex_bin,
            resolved_run: resolved_summary.is_some(),
            follow_up_or_retry: follow_up_task_card.is_some() || retry_current_specialist,
            reclaimed_worker,
            collapsed_worker_fan_in,
            dispatched_execution,
            effective_task_card,
            launch_result: launch_result.as_ref(),
            collapsed_fan_in: collapsed_fan_in.as_ref(),
        },
    )?;
    write_json_document(&run_file, &run_record)?;

    let run_state_file = locator.run_directory.join("run-state.json");
    let mut run_state_record = read_json_document(&run_state_file)?;
    let current_phase_name = phase_name_for_role(
        effective_task_card
            .get("assigned_role")
            .and_then(Value::as_str)
            .unwrap_or("code specialist"),
    );
    apply_run_state_after_attempt(
        &mut run_state_record,
        &timestamp,
        &next_step_after_attempt,
        current_phase_name,
    )?;
    write_json_document(&run_state_file, &run_state_record)?;

    append_run_event(
        &locator.run_directory,
        json!({
            "event": "run_orchestrated",
            "entrypoint": "ccc_orchestrate",
            "run_id": locator.run_id,
            "attempt_id": attempt_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
            "requested_progression_mode": requested_progression_mode,
            "current_next_step": current_next_step,
            "timestamp": timestamp,
        }),
    )?;

    Ok(json!({
        "cwd": locator.cwd.to_string_lossy(),
        "run_id": locator.run_id,
        "run_directory": locator.run_directory.to_string_lossy(),
        "run_ref": create_ccc_run_ref(&locator.run_directory),
        "attempt_id": attempt_payload.get("attempt_id").cloned().unwrap_or(Value::Null),
        "starting_next_step": current_next_step,
        "next_step": next_step_after_attempt,
        "can_advance": can_advance_after_attempt,
        "advanced": true,
        "progression_mode": requested_progression_mode,
        "summary": summary,
        "scheduler_decision": attempt_payload.get("scheduler_decision").cloned().unwrap_or(Value::Null),
        "post_fan_in_captain_decision": attempt_payload.get("post_fan_in_captain_decision").cloned().unwrap_or(Value::Null),
        "launch_result": attempt_payload.get("launch_result").cloned().unwrap_or(Value::Null),
        "reclaimed_targets": attempt_payload.get("reclaimed_targets").cloned().unwrap_or(Value::Null),
        "collapsed_fan_in": attempt_payload.get("collapsed_fan_in").cloned().unwrap_or(Value::Null),
        "consumed_worker_result_envelope": attempt_payload.get("consumed_worker_result_envelope").cloned().unwrap_or(Value::Null),
        "consumed_pending_follow_up": attempt_payload.get("consumed_pending_follow_up").cloned().unwrap_or(Value::Null),
        "allowed_next_commands": if can_advance_after_attempt { json!(["advance"]) } else { json!([]) },
    }))
}

fn is_review_lifecycle_update(task_card: &Value, child_agent_id: &str, parsed: &Value) -> bool {
    let normalized_child_agent_id = child_agent_id.trim();
    let reviewer_child_agent = matches!(normalized_child_agent_id, "arbiter" | "ccc_arbiter")
        || role_for_agent_id(normalized_child_agent_id) == Some("verifier");
    matches!(normalized_child_agent_id, "arbiter" | "ccc_arbiter")
        || (task_card_is_review(task_card) && reviewer_child_agent)
        || parsed
            .get("review_outcome")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn subagent_update_response_mode(parsed: &Value) -> &'static str {
    if parsed.get("mode").and_then(Value::as_str) == Some("compact") {
        "compact"
    } else {
        "full"
    }
}

fn sanitize_subagent_event_ref(raw: &str) -> String {
    let sanitized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        generate_uuid_like_id()
    } else {
        trimmed.chars().take(96).collect()
    }
}

fn subagent_event_ref(parsed: &Value) -> String {
    parsed
        .get("event_ref")
        .and_then(Value::as_str)
        .map(sanitize_subagent_event_ref)
        .unwrap_or_else(generate_uuid_like_id)
}

fn value_serialized_len(value: &Value) -> usize {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}

fn text_len(value: &Value) -> usize {
    value.as_str().map(|text| text.chars().count()).unwrap_or(0)
}

fn truncate_visibility_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn compact_artifact_array(value: &Value) -> Value {
    let Some(items) = value.as_array() else {
        return value.clone();
    };
    if items.len() <= SUBAGENT_FAN_IN_INLINE_ITEMS {
        return value.clone();
    }

    let mut compact_items = items
        .iter()
        .take(SUBAGENT_FAN_IN_INLINE_ITEMS)
        .cloned()
        .collect::<Vec<_>>();
    compact_items.push(json!(format!(
        "... {} more item(s) persisted in fan_in_artifact",
        items.len().saturating_sub(SUBAGENT_FAN_IN_INLINE_ITEMS)
    )));
    Value::Array(compact_items)
}

fn should_persist_subagent_fan_in_artifact(parsed: &Value, fan_in: &Value) -> bool {
    parsed.get("event_ref").and_then(Value::as_str).is_some()
        || subagent_update_response_mode(parsed) == "compact"
        || value_serialized_len(fan_in) > SUBAGENT_FAN_IN_ARTIFACT_LIMIT_BYTES
        || text_len(&fan_in["summary"]) > SUBAGENT_FAN_IN_SUMMARY_LIMIT_CHARS
        || fan_in
            .get("evidence_paths")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() > SUBAGENT_FAN_IN_INLINE_ITEMS)
        || fan_in
            .get("open_questions")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() > SUBAGENT_FAN_IN_INLINE_ITEMS)
}

fn create_artifact_backed_fan_in(fan_in: &Value, artifact_ref: &Value) -> Value {
    let mut compact = fan_in.clone();
    if let Some(object) = compact.as_object_mut() {
        if let Some(summary) = object.get("summary").and_then(Value::as_str) {
            object.insert(
                "summary".to_string(),
                Value::String(truncate_visibility_text(
                    summary,
                    SUBAGENT_FAN_IN_INLINE_SUMMARY_CHARS,
                )),
            );
        }
        if let Some(evidence_paths) = object.get("evidence_paths").cloned() {
            object.insert(
                "evidence_paths".to_string(),
                compact_artifact_array(&evidence_paths),
            );
        }
        if let Some(open_questions) = object.get("open_questions").cloned() {
            object.insert(
                "open_questions".to_string(),
                compact_artifact_array(&open_questions),
            );
        }
        object.insert("artifact_ref".to_string(), artifact_ref.clone());
    }
    compact
}

struct SubagentFanInArtifactInput<'a> {
    parsed: &'a Value,
    run_directory: &'a Path,
    run_id: &'a str,
    task_card_id: &'a str,
    child_agent_id: &'a str,
    lane_id: Option<&'a str>,
    thread_id: Option<&'a str>,
    status: &'a str,
    fan_in: &'a Value,
    review_fan_in: &'a Value,
    timestamp: &'a str,
}

fn maybe_persist_subagent_fan_in_artifact(
    input: SubagentFanInArtifactInput<'_>,
) -> io::Result<(Value, Value)> {
    if !should_persist_subagent_fan_in_artifact(input.parsed, input.fan_in) {
        return Ok((input.fan_in.clone(), Value::Null));
    }

    let event_ref = subagent_event_ref(input.parsed);
    let artifact_path = input
        .run_directory
        .join("temp-artifacts")
        .join("subagent-update")
        .join(format!("{event_ref}.json"));
    let artifact_ref = json!({
        "kind": "subagent_update_fan_in",
        "event_ref": event_ref,
        "mode": subagent_update_response_mode(input.parsed),
        "path": artifact_path.to_string_lossy(),
        "persisted_at": input.timestamp,
    });
    write_json_document(
        &artifact_path,
        &json!({
            "kind": "subagent_update_fan_in",
            "event_ref": event_ref,
            "run_id": input.run_id,
            "task_card_id": input.task_card_id,
            "child_agent_id": input.child_agent_id,
            "lane_id": input.lane_id,
            "thread_id": input.thread_id,
            "status": input.status,
            "fan_in": input.fan_in,
            "review_fan_in": input.review_fan_in,
            "recorded_at": input.timestamp,
        }),
    )?;

    Ok((
        create_artifact_backed_fan_in(input.fan_in, &artifact_ref),
        artifact_ref,
    ))
}

fn read_subagent_fan_in_event_artifact(
    run_directory: &Path,
    parsed: &Value,
) -> io::Result<Option<Value>> {
    let Some(event_ref) = parsed
        .get("event_ref")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(sanitize_subagent_event_ref)
    else {
        return Ok(None);
    };
    let artifact_path = run_directory
        .join("temp-artifacts")
        .join("subagent-update")
        .join(format!("{event_ref}.json"));
    if !artifact_path.exists() {
        return Ok(None);
    }
    let artifact = read_json_document(&artifact_path)?;
    Ok(artifact.get("fan_in").cloned())
}

fn parsed_or_artifact_field(parsed: &Value, artifact_fan_in: Option<&Value>, field: &str) -> Value {
    parsed
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .or_else(|| artifact_fan_in.and_then(|fan_in| fan_in.get(field).cloned()))
        .unwrap_or(Value::Null)
}

fn parsed_or_artifact_array(parsed: &Value, artifact_fan_in: Option<&Value>, field: &str) -> Value {
    let parsed_array = parsed.get(field).filter(|value| value.is_array());
    let artifact_array =
        artifact_fan_in.and_then(|fan_in| fan_in.get(field).filter(|value| value.is_array()));
    if parsed_array
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        return parsed_array
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));
    }
    artifact_array
        .cloned()
        .or_else(|| parsed_array.cloned())
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn parsed_or_artifact_summary<'a>(
    parsed: &'a Value,
    artifact_fan_in: Option<&'a Value>,
) -> Option<&'a str> {
    parsed.get("summary").and_then(Value::as_str).or_else(|| {
        artifact_fan_in.and_then(|fan_in| fan_in.get("summary").and_then(Value::as_str))
    })
}

pub(crate) fn create_ccc_subagent_update_payload(parsed: &Value) -> io::Result<Value> {
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": parsed.get("run_id").cloned().unwrap_or(Value::Null),
            "run_ref": parsed.get("run_ref").cloned().unwrap_or(Value::Null),
            "run_dir": parsed.get("run_directory").cloned().unwrap_or(Value::Null),
            "cwd": parsed.get("cwd").cloned().unwrap_or(Value::Null),
        }),
        "ccc_subagent_update",
    )?;
    let _run_lock = acquire_run_mutation_lock(&locator.run_directory, "ccc_subagent_update")?;
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let run_file = locator.run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    let active_task_card_id = parsed
        .get("task_card_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            run_object
                .get("active_task_card_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ccc_subagent_update could not resolve an active task_card_id.",
            )
        })?;
    let task_card_file = locator
        .run_directory
        .join("task-cards")
        .join(format!("{active_task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "ccc_subagent_update could not read task_card_id={active_task_card_id} at {}: {error}",
                task_card_file.display()
            ),
        )
    })?;
    let reported_status = parsed
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "status is required"))?;
    let parsed_child_agent_id = parsed
        .get("child_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let parsed_lane_id = parsed
        .get("lane_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(normalize_host_lane_id);
    let parsed_thread_id = parsed
        .get("thread_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (mut child_agent_id, mut normalized_thread_id) = normalize_subagent_update_agent_identity(
        &task_card,
        parsed_child_agent_id,
        parsed_thread_id,
    );
    if parsed_child_agent_id.is_some_and(|value| matches!(value, "arbiter" | "ccc_arbiter"))
        && !matches!(child_agent_id.as_str(), "arbiter" | "ccc_arbiter")
    {
        child_agent_id = parsed_child_agent_id.unwrap_or("arbiter").to_string();
        normalized_thread_id = parsed_thread_id.map(str::to_string);
    }
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("code specialist")
        .to_string();
    let required_lane_ids = parallel_required_lane_ids(&task_card);
    let lane_id = parsed_lane_id.clone().or_else(|| {
        if required_lane_ids.len() == 1 {
            required_lane_ids.first().cloned()
        } else {
            None
        }
    });
    let thread_id = normalized_thread_id.as_deref();
    let artifact_fan_in = read_subagent_fan_in_event_artifact(&locator.run_directory, parsed)?;
    let artifact_fan_in_ref = artifact_fan_in.as_ref();
    let summary = parsed_or_artifact_summary(parsed, artifact_fan_in_ref);
    let explicit_fallback_reason = parsed.get("fallback_reason").and_then(Value::as_str);
    let automatic_fallback_reason = if explicit_fallback_reason.is_none()
        && matches!(reported_status, "stalled" | "reclaimed")
    {
        Some("child_timeout")
    } else {
        None
    };
    let fallback_reason = explicit_fallback_reason.or(automatic_fallback_reason);
    let total_token_usage = parsed
        .get("total_token_usage")
        .filter(|value| value.is_object());
    let context_tokens = parsed.get("context_tokens").and_then(Value::as_u64);
    let drift_payload = create_subagent_policy_drift_payload(
        &task_card,
        Some(&child_agent_id),
        parsed.get("observed_model").and_then(Value::as_str),
        parsed.get("observed_variant").and_then(Value::as_str),
        parsed.get("observed_sandbox_mode").and_then(Value::as_str),
        parsed
            .get("observed_approval_policy")
            .and_then(Value::as_str),
        &timestamp,
    );
    let review_lifecycle_update = is_review_lifecycle_update(&task_card, &child_agent_id, parsed);
    let prior_primary_lifecycle = task_card
        .get("subagent_lifecycle")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let prior_review_lifecycle = task_card
        .get("review_lifecycle")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let prior_lifecycle = if review_lifecycle_update {
        prior_review_lifecycle
    } else {
        prior_primary_lifecycle
    };
    let prior_fan_in = task_card
        .get("subagent_fan_in")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let prior_parallel_fanout = task_card.get("parallel_fanout").cloned();
    let prior_stale_output_policy = task_card
        .pointer("/captain_intervention/stale_output_policy")
        .cloned()
        .unwrap_or_else(|| Value::String("preserve_visible".to_string()));
    let active_reclaim_intervention = is_active_host_subagent_status(reported_status)
        && parsed
            .get("chosen_next_action")
            .and_then(Value::as_str)
            .map(str::trim)
            == Some("reclaim");
    let stale_output_after_reclaim = prior_lifecycle.get("status").and_then(Value::as_str)
        == Some("reclaimed")
        && reported_status != "reclaimed"
        && reported_status != "merged"
        && (task_card
            .pointer("/captain_intervention/chosen_next_action")
            .and_then(Value::as_str)
            == Some("reclaim")
            || task_card
                .pointer("/captain_intervention/stale_output_policy")
                .and_then(Value::as_str)
                .is_some());
    let status = if stale_output_after_reclaim {
        "reclaimed"
    } else if active_reclaim_intervention {
        "reclaimed"
    } else {
        reported_status
    };
    let primary_thread_id = if stale_output_after_reclaim {
        None
    } else {
        thread_id
    };
    let incoming_fan_in_status =
        parsed_or_artifact_field(parsed, artifact_fan_in_ref, "fan_in_status");
    let incoming_fan_in_status = if incoming_fan_in_status.is_null() {
        artifact_fan_in_ref
            .and_then(|fan_in| fan_in.get("status").cloned())
            .unwrap_or(Value::Null)
    } else {
        incoming_fan_in_status
    };
    let incoming_evidence_paths =
        parsed_or_artifact_array(parsed, artifact_fan_in_ref, "evidence_paths");
    let incoming_next_action = parsed_or_artifact_field(parsed, artifact_fan_in_ref, "next_action");
    let incoming_open_questions =
        parsed_or_artifact_array(parsed, artifact_fan_in_ref, "open_questions");
    let incoming_confidence = parsed_or_artifact_field(parsed, artifact_fan_in_ref, "confidence");
    let incoming_risk = parsed_or_artifact_field(parsed, artifact_fan_in_ref, "risk");
    let incoming_checks = parsed_or_artifact_array(parsed, artifact_fan_in_ref, "checks");
    let fan_in_compact = create_subagent_fan_in_compact(SubagentFanInCompactInput {
        prior_fan_in: &prior_fan_in,
        status,
        summary,
        incoming_fan_in_status,
        incoming_evidence_paths,
        incoming_next_action,
        incoming_open_questions,
        incoming_confidence,
        incoming_risk,
        incoming_checks,
    });
    let sentinel_intervention = create_sentinel_intervention_payload(
        parsed,
        &child_agent_id,
        &drift_payload,
        &fan_in_compact,
        &timestamp,
    );
    let review_outcome = infer_review_outcome(
        &task_card,
        status,
        Some(&child_agent_id),
        fan_in_compact.get("status").and_then(Value::as_str),
        parsed.get("review_outcome").and_then(Value::as_str),
    );
    let existing_review_pass_count = task_card
        .get("review_pass_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let review_pass_cap = review_pass_cap_for_task_card(&task_card);
    let review_pass_cap_reached = review_outcome.as_deref() == Some("passed")
        && existing_review_pass_count >= review_pass_cap;
    let review_pass_cap_state = "captain_decision_required";
    let mut review_fan_in_compact = fan_in_compact.clone();
    let mut review_findings = parsed
        .get("findings")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let review_outcome = review_outcome.map(|outcome| {
        if review_pass_cap_reached {
            let finding = format!(
                "Maximum review pass count reached (review_pass_count={existing_review_pass_count}, reviewer_cap={review_pass_cap}); captain must decide next action before accepting, reassigning, or closing."
            );
            if let Some(object) = review_fan_in_compact.as_object_mut() {
                object.insert(
                    "status".to_string(),
                    Value::String("captain_decision_required".to_string()),
                );
                object.insert("next_action".to_string(), Value::String("captain_decision".to_string()));
                object.insert("summary".to_string(), Value::String(finding.clone()));
            }
            push_review_cap_finding(&mut review_fan_in_compact, Some("open_questions"), &finding);
            push_review_cap_finding(&mut review_findings, None, &finding);
            "blocked".to_string()
        } else {
            outcome
        }
    });
    let review_fan_in = review_outcome.as_ref().map(|outcome| {
        create_review_fan_in_payload(
            &task_card,
            outcome,
            &review_fan_in_compact,
            review_findings.clone(),
            &timestamp,
        )
    });
    let mut captain_intervention =
        create_captain_intervention_payload(parsed, &fan_in_compact, &timestamp);
    let mut duplicate_pending_follow_up = false;
    if let Some(intervention) = captain_intervention.as_mut() {
        if let Some(object) = intervention.as_object_mut() {
            if active_reclaim_intervention {
                object.insert(
                    "reported_subagent_status".to_string(),
                    Value::String(reported_status.to_string()),
                );
                object.insert(
                    "effective_subagent_status".to_string(),
                    Value::String(status.to_string()),
                );
                object.insert(
                    "host_cancellation_supported".to_string(),
                    Value::Bool(false),
                );
                object.insert(
                    "host_worker_may_still_be_running".to_string(),
                    Value::Bool(true),
                );
            }
        }
        let pending_follow_up = create_pending_captain_follow_up_payload(
            parsed,
            &task_card,
            &fan_in_compact,
            &active_task_card_id,
            &child_agent_id,
            lane_id.as_deref(),
            status,
            &timestamp,
        );
        let pending_follow_up = pending_follow_up.map(|pending| {
            let existing = pending_follow_up_dedupe_key(&pending)
                .and_then(|dedupe_key| existing_pending_follow_up_for_key(&task_card, dedupe_key));
            if let Some(existing) = existing {
                duplicate_pending_follow_up = true;
                existing
            } else {
                pending
            }
        });
        if let Some(object) = intervention.as_object_mut() {
            object.insert(
                "pending_follow_up".to_string(),
                pending_follow_up.unwrap_or(Value::Null),
            );
        }
    }
    let mut fan_in = fan_in_compact.clone();
    if let Some(object) = fan_in.as_object_mut() {
        object.insert("recorded_at".to_string(), Value::String(timestamp.clone()));
    }
    let review_fan_in_for_artifact = review_fan_in.clone().unwrap_or(Value::Null);
    let (state_fan_in, fan_in_artifact) =
        maybe_persist_subagent_fan_in_artifact(SubagentFanInArtifactInput {
            parsed,
            run_directory: &locator.run_directory,
            run_id: &locator.run_id,
            task_card_id: &active_task_card_id,
            child_agent_id: &child_agent_id,
            lane_id: lane_id.as_deref(),
            thread_id,
            status,
            fan_in: &fan_in,
            review_fan_in: &review_fan_in_for_artifact,
            timestamp: &timestamp,
        })?;
    let event_ref = fan_in_artifact
        .get("event_ref")
        .and_then(Value::as_str)
        .map(str::to_string);
    let task_card_object = task_card.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "task card must be a JSON object.",
        )
    })?;
    let lifecycle = create_subagent_lifecycle_payload(SubagentLifecyclePayloadInput {
        prior_lifecycle,
        status,
        child_agent_id: &child_agent_id,
        primary_thread_id,
        summary,
        stale_output_after_reclaim,
        active_reclaim_intervention,
        reported_status,
        timestamp: &timestamp,
    });
    let lifecycle_field = if review_lifecycle_update {
        "review_lifecycle"
    } else {
        "subagent_lifecycle"
    };
    task_card_object.insert(lifecycle_field.to_string(), lifecycle);
    task_card_object.insert("subagent_policy_drift".to_string(), drift_payload);
    if (is_terminal_host_subagent_status(status) || status == "merged")
        && !stale_output_after_reclaim
    {
        task_card_object.insert("subagent_fan_in".to_string(), state_fan_in.clone());
        task_card_object.insert("worker_result_envelope".to_string(), state_fan_in.clone());
        if !fan_in_artifact.is_null() {
            task_card_object.insert(
                "subagent_fan_in_artifact".to_string(),
                fan_in_artifact.clone(),
            );
        }
    }
    if stale_output_after_reclaim {
        task_card_object.insert(
            "late_subagent_output".to_string(),
            json!({
                "status": reported_status,
                "effective_status": status,
                "child_agent_id": child_agent_id,
                "lane_id": lane_id,
                "thread_id": thread_id,
                "summary": summary,
                "fan_in": state_fan_in.clone(),
                "stale_output_policy": prior_stale_output_policy,
                "authority": "captain_explicit_merge_required",
                "recorded_at": timestamp,
            }),
        );
    }
    if let (Some(outcome), Some(review_fan_in)) =
        (review_outcome.as_deref(), review_fan_in.as_ref())
    {
        task_card_object.insert("review_fan_in".to_string(), review_fan_in.clone());
        task_card_object.insert(
            "verification_state".to_string(),
            Value::String(verification_state_for_review_outcome(outcome).to_string()),
        );
        if outcome == "passed" {
            let review_pass_count = existing_review_pass_count
                .saturating_add(1)
                .min(review_pass_cap);
            task_card_object.insert("review_pass_count".to_string(), json!(review_pass_count));
        } else if review_pass_cap_reached {
            task_card_object.insert("review_pass_count".to_string(), json!(review_pass_cap));
        }
        if let Some(policy) = task_card_object
            .get_mut("review_policy")
            .and_then(Value::as_object_mut)
        {
            policy.insert(
                "state".to_string(),
                Value::String(if review_pass_cap_reached {
                    review_pass_cap_state.to_string()
                } else {
                    outcome.to_string()
                }),
            );
            policy.insert("active_reviewers".to_string(), json!(0));
            policy.insert(
                "summary".to_string(),
                Value::String(if review_pass_cap_reached {
                    format!(
                        "Review pass cap reached at {review_pass_cap}; captain must decide whether to accept, repair, reassign, or close."
                    )
                } else {
                    format!(
                        "Review returned {outcome}; captain must decide whether to accept, repair, reassign, or close."
                    )
                }),
            );
            policy.insert("updated_at".to_string(), Value::String(timestamp.clone()));
        }
    }
    if let Some(intervention) = captain_intervention.as_ref() {
        task_card_object.insert("captain_intervention".to_string(), intervention.clone());
        let mut intervention_history = task_card_object
            .get("captain_intervention_history")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !duplicate_pending_follow_up {
            intervention_history.push(intervention.clone());
        }
        task_card_object.insert(
            "captain_intervention_history".to_string(),
            Value::Array(intervention_history),
        );
    }
    if let Some(intervention) = sentinel_intervention.as_ref() {
        task_card_object.insert("sentinel_intervention".to_string(), intervention.clone());
        let mut intervention_history = task_card_object
            .get("sentinel_intervention_history")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        intervention_history.push(intervention.clone());
        task_card_object.insert(
            "sentinel_intervention_history".to_string(),
            Value::Array(intervention_history),
        );
    }
    if let Some(lane_id) = lane_id.as_deref().filter(|_| !stale_output_after_reclaim) {
        let parallel_fanout = update_parallel_fanout_for_lane(
            prior_parallel_fanout.as_ref(),
            lane_id,
            status,
            &child_agent_id,
            primary_thread_id,
            summary,
            &state_fan_in,
            &timestamp,
        );
        task_card_object.insert("parallel_fanout".to_string(), parallel_fanout);
    } else if let Some(existing_parallel_fanout) = prior_parallel_fanout.as_ref() {
        task_card_object.insert(
            "parallel_fanout".to_string(),
            existing_parallel_fanout.clone(),
        );
    }
    if let Some(reason) = fallback_reason {
        task_card_object.insert(
            "subagent_fallback".to_string(),
            json!({
                "reason": reason,
                "recorded_at": timestamp,
            }),
        );
    }
    task_card_object.insert("updated_at".to_string(), Value::String(timestamp.clone()));
    write_json_document(&task_card_file, &task_card)?;

    update_subagent_run_child_agent_entry(
        run_object,
        SubagentRunRecordChildInput {
            active_task_card_id: &active_task_card_id,
            child_agent_id: &child_agent_id,
            lane_id: lane_id.as_deref(),
            assigned_role: &assigned_role,
            status,
            primary_thread_id,
            stale_output_after_reclaim,
            summary,
            review_outcome: review_outcome.as_deref(),
            observed_model: parsed.get("observed_model").and_then(Value::as_str),
            total_token_usage,
            context_tokens,
            timestamp: &timestamp,
        },
    );
    update_subagent_run_specialist_executor_entry(
        run_object,
        SubagentRunRecordExecutorInput {
            active_task_card_id: &active_task_card_id,
            child_agent_id: &child_agent_id,
            lane_id: lane_id.as_deref(),
            status,
            primary_thread_id,
            fallback_reason,
            review_outcome: review_outcome.as_deref(),
            observed_model: parsed.get("observed_model").and_then(Value::as_str),
            total_token_usage,
            context_tokens,
            timestamp: &timestamp,
        },
    );

    let active_handle_cleanup = update_run_host_subagent_handle_state(
        run_object,
        &active_task_card_id,
        &child_agent_id,
        lane_id.as_deref(),
        primary_thread_id,
        status,
        &timestamp,
    );
    run_object.insert("updated_at".to_string(), Value::String(timestamp.clone()));
    run_object.insert(
        "active_role".to_string(),
        if is_terminal_or_merged_host_subagent_status(status) {
            Value::String("orchestrator".to_string())
        } else {
            Value::String(assigned_role.to_string())
        },
    );
    run_object.insert(
        "active_agent_id".to_string(),
        if is_terminal_or_merged_host_subagent_status(status) {
            Value::String("captain".to_string())
        } else {
            Value::String(child_agent_id.to_string())
        },
    );
    run_object.insert(
        "latest_orchestrator_synthesis".to_string(),
        Value::String(summary.map(str::to_string).unwrap_or_else(|| {
            format!("Host Codex recorded subagent {child_agent_id} as {status}.")
        })),
    );
    if let Some(intervention) = captain_intervention.as_ref() {
        run_object.insert(
            "latest_captain_intervention".to_string(),
            intervention.clone(),
        );
    }
    if let Some(intervention) = sentinel_intervention.as_ref() {
        run_object.insert(
            "latest_sentinel_intervention".to_string(),
            intervention.clone(),
        );
    }
    if stale_output_after_reclaim {
        run_object.insert(
            "latest_stale_subagent_output".to_string(),
            json!({
                "task_card_id": active_task_card_id,
                "child_agent_id": child_agent_id,
                "lane_id": lane_id,
                "thread_id": thread_id,
                "status": reported_status,
                "effective_status": status,
                "summary": summary,
                "recorded_at": timestamp,
                "authority": "captain_explicit_merge_required",
            }),
        );
    }
    run_object.insert(
        "latest_entry_trace".to_string(),
        json!({
            "entrypoint": "ccc_subagent_update",
            "child_agent_id": child_agent_id,
            "lane_id": lane_id,
            "status": status,
            "reported_status": reported_status,
            "active_reclaim_intervention": active_reclaim_intervention,
            "stale_output_after_reclaim": stale_output_after_reclaim,
            "review_outcome": review_outcome,
            "event_ref": event_ref.clone(),
            "fan_in_artifact": fan_in_artifact.clone(),
            "captain_intervention": captain_intervention.clone(),
            "sentinel_intervention": sentinel_intervention.clone(),
            "thread_id": thread_id,
            "fallback_reason": fallback_reason,
            "active_handle_cleanup": active_handle_cleanup,
            "completed_at": timestamp,
        }),
    );
    if let Some(reason) = fallback_reason {
        run_object.insert(
            "latest_failure".to_string(),
            json!({
                "stage": "subagent_execution",
                "reason": reason,
                "summary": summary.unwrap_or("Host Codex reported a subagent fallback reason."),
                "recorded_at": timestamp,
            }),
        );
    } else if matches!(status, "failed" | "stalled" | "reclaimed") {
        run_object.insert(
            "latest_failure".to_string(),
            json!({
                "stage": "subagent_execution",
                "reason": status,
                "summary": summary.unwrap_or("Host Codex reported a subagent execution problem."),
                "recorded_at": timestamp,
            }),
        );
    }
    write_json_document(&run_file, &run_record)?;

    let run_state_file = locator.run_directory.join("run-state.json");
    let mut run_state_record = read_json_document(&run_state_file)?;
    let next_action = next_action_for_host_subagent_status(status);
    apply_subagent_run_state_update(
        &mut run_state_record,
        SubagentRunStateUpdateInput {
            timestamp: &timestamp,
            next_action,
            current_phase_name: phase_name_for_host_subagent_status(&assigned_role, status),
        },
    )?;
    write_json_document(&run_state_file, &run_state_record)?;

    if is_terminal_or_merged_host_subagent_status(status) {
        complete_longway_active_phase(&locator.run_directory, &timestamp)?;
    }

    let orchestrator_state_file = locator.run_directory.join("orchestrator-state.json");
    if let Ok(mut orchestrator_state_record) = read_json_document(&orchestrator_state_file) {
        if apply_subagent_orchestrator_state_update(
            &mut orchestrator_state_record,
            SubagentOrchestratorStateUpdateInput {
                next_action,
                can_advance: matches!(status, "merged") || is_terminal_host_subagent_status(status),
                summary: summary.unwrap_or("Host Codex recorded a subagent lifecycle checkpoint."),
                child_agent_id: &child_agent_id,
                lane_id: lane_id.as_deref(),
                thread_id,
                status,
                review_outcome: review_outcome.as_deref(),
                captain_intervention: captain_intervention.as_ref(),
                fallback_reason,
                active_handle_cleanup: &active_handle_cleanup,
                timestamp: &timestamp,
            },
        ) {
            write_json_document(&orchestrator_state_file, &orchestrator_state_record)?;
        }
    }

    append_run_event(
        &locator.run_directory,
        json!({
            "event": "subagent_updated",
            "entrypoint": "ccc_subagent_update",
            "run_id": locator.run_id,
            "task_card_id": active_task_card_id,
            "child_agent_id": child_agent_id,
            "lane_id": lane_id,
            "thread_id": thread_id,
            "status": status,
            "reported_status": reported_status,
            "active_reclaim_intervention": active_reclaim_intervention,
            "stale_output_after_reclaim": stale_output_after_reclaim,
            "review_outcome": review_outcome,
            "event_ref": event_ref.clone(),
            "fan_in_artifact": fan_in_artifact.clone(),
            "captain_intervention": captain_intervention.clone(),
            "sentinel_intervention": sentinel_intervention.clone(),
            "fallback_reason": fallback_reason,
            "active_handle_cleanup": active_handle_cleanup,
            "timestamp": timestamp,
        }),
    )?;

    Ok(json!({
        "cwd": locator.cwd.to_string_lossy(),
        "run_id": locator.run_id,
        "run_directory": locator.run_directory.to_string_lossy(),
        "run_ref": create_ccc_run_ref(&locator.run_directory),
        "task_card_id": active_task_card_id,
        "child_agent_id": child_agent_id,
        "lane_id": lane_id,
        "thread_id": thread_id,
        "subagent_status": status,
        "reported_subagent_status": reported_status,
        "active_reclaim_intervention": active_reclaim_intervention,
        "stale_output_after_reclaim": stale_output_after_reclaim,
        "review_outcome": review_outcome,
        "lifecycle_field": lifecycle_field,
        "summary": summary,
        "fan_in": state_fan_in,
        "fan_in_artifact": fan_in_artifact,
        "event_ref": event_ref,
        "response_mode": subagent_update_response_mode(parsed),
        "review_fan_in": review_fan_in.unwrap_or(Value::Null),
        "captain_intervention": captain_intervention.unwrap_or(Value::Null),
        "sentinel_intervention": sentinel_intervention.unwrap_or(Value::Null),
        "fallback_reason": fallback_reason,
        "active_handle_cleanup": active_handle_cleanup,
    }))
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
