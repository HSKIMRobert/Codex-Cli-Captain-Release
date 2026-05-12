use crate::host_subagent_lifecycle::{
    is_active_host_subagent_status, is_terminal_host_subagent_status,
};
use crate::{
    read_json_document, resolve_ccc_config_directory, resolve_codex_home, write_json_document,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) const CCC_RUN_REF_PREFIX: &str = "ccc-run:";

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRunLocator {
    pub(crate) cwd: PathBuf,
    pub(crate) run_id: String,
    pub(crate) run_directory: PathBuf,
}

pub(crate) fn resolve_workspace_path(cwd: Option<&str>) -> io::Result<PathBuf> {
    let candidate = match cwd {
        Some(value) if !value.trim().is_empty() => {
            let input = PathBuf::from(value.trim());
            if input.is_absolute() {
                input
            } else {
                env::current_dir()?.join(input)
            }
        }
        _ => env::current_dir()?,
    };

    Ok(normalize_path(&candidate))
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        canonical
    } else if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(current_dir) = env::current_dir() {
        current_dir.join(path)
    } else {
        path.to_path_buf()
    }
}

pub(crate) fn compute_workspace_storage_key(base_directory: &Path) -> String {
    let normalized_workspace = normalize_path(base_directory);
    let base_name = normalized_workspace
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("workspace");
    let digest = Sha256::digest(normalized_workspace.to_string_lossy().as_bytes());
    let path_hash = digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{base_name}--{path_hash}")
}

pub(crate) fn resolve_legacy_workspace_ccc_directory(base_directory: &Path) -> PathBuf {
    normalize_path(base_directory).join(".ccc")
}

pub(crate) fn resolve_workspace_ccc_directory(base_directory: &Path) -> PathBuf {
    normalize_path(base_directory).join(".ccc")
}

pub(crate) fn create_workspace_run_directory_from_workspace(
    base_directory: &Path,
    run_id: &str,
) -> PathBuf {
    resolve_workspace_ccc_directory(base_directory)
        .join("runs")
        .join(run_id)
}

pub(crate) fn resolve_legacy_codex_workspace_ccc_directory(
    base_directory: &Path,
) -> io::Result<PathBuf> {
    Ok(resolve_codex_home()?
        .join(".ccc")
        .join("workspaces")
        .join(compute_workspace_storage_key(base_directory)))
}

pub(crate) fn resolve_central_workspace_ccc_directory(
    base_directory: &Path,
) -> io::Result<PathBuf> {
    Ok(resolve_ccc_config_directory()?
        .join("workspaces")
        .join(compute_workspace_storage_key(base_directory)))
}

pub(crate) fn should_use_legacy_workspace_ccc_directory(
    base_directory: &Path,
    run_id: &str,
) -> bool {
    let legacy_ccc_dir = resolve_legacy_workspace_ccc_directory(base_directory);
    legacy_ccc_dir.join("runs").join(run_id).exists()
        || legacy_ccc_dir.join("role-defaults.json").exists()
}

pub(crate) fn resolve_existing_workspace_ccc_directory(
    base_directory: &Path,
    run_id: &str,
) -> io::Result<Option<PathBuf>> {
    let resolved_workspace = normalize_path(base_directory);
    let workspace_ccc_dir = resolve_workspace_ccc_directory(&resolved_workspace);
    let legacy_ccc_dir = resolve_legacy_workspace_ccc_directory(&resolved_workspace);
    let legacy_codex_ccc_dir = resolve_legacy_codex_workspace_ccc_directory(&resolved_workspace)?;
    let central_ccc_dir = resolve_central_workspace_ccc_directory(&resolved_workspace)?;

    for candidate in [
        workspace_ccc_dir.clone(),
        legacy_ccc_dir.clone(),
        legacy_codex_ccc_dir,
        central_ccc_dir,
    ] {
        if candidate.join("runs").join(run_id).exists() {
            return Ok(Some(candidate));
        }
    }

    if workspace_ccc_dir.join("role-defaults.json").exists() {
        return Ok(Some(workspace_ccc_dir));
    }

    if legacy_ccc_dir.join("role-defaults.json").exists() {
        return Ok(Some(legacy_ccc_dir));
    }

    Ok(None)
}

pub(crate) fn create_run_directory_from_workspace(
    base_directory: &Path,
    run_id: &str,
) -> io::Result<PathBuf> {
    let resolved_workspace = normalize_path(base_directory);
    let ccc_dir = if let Some(existing) =
        resolve_existing_workspace_ccc_directory(&resolved_workspace, run_id)?
    {
        existing
    } else if should_use_legacy_workspace_ccc_directory(&resolved_workspace, run_id) {
        resolve_legacy_workspace_ccc_directory(&resolved_workspace)
    } else {
        resolve_central_workspace_ccc_directory(&resolved_workspace)?
    };

    Ok(normalize_path(&ccc_dir.join("runs").join(run_id)))
}

fn run_directory_has_run_record(run_directory: &Path) -> bool {
    run_directory.join("run.json").is_file()
}

pub(crate) fn candidate_global_workspace_roots() -> io::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    roots.push(resolve_ccc_config_directory()?.join("workspaces"));
    roots.push(resolve_codex_home()?.join(".ccc").join("workspaces"));
    roots.sort();
    roots.dedup();
    Ok(roots)
}

pub(crate) fn find_global_run_directory_by_run_id_in_roots(
    run_id: &str,
    workspace_roots: &[PathBuf],
) -> io::Result<Option<PathBuf>> {
    let mut matches = Vec::new();
    for workspace_root in workspace_roots {
        let Ok(entries) = fs::read_dir(workspace_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("runs").join(run_id);
            if run_directory_has_run_record(&candidate) {
                matches.push(normalize_path(&candidate));
            }
        }
    }
    matches.sort();
    Ok(matches.into_iter().next())
}

pub(crate) fn resolve_run_id_locator_with_global_fallback_in_roots(
    workspace_dir: &Path,
    run_id: &str,
    workspace_roots: &[PathBuf],
) -> io::Result<ResolvedRunLocator> {
    let workspace_dir = normalize_path(workspace_dir);
    let workspace_run_directory = create_run_directory_from_workspace(&workspace_dir, run_id)?;
    if run_directory_has_run_record(&workspace_run_directory) {
        return Ok(ResolvedRunLocator {
            run_directory: workspace_run_directory,
            cwd: workspace_dir,
            run_id: run_id.to_string(),
        });
    }

    if let Some(global_run_directory) =
        find_global_run_directory_by_run_id_in_roots(run_id, workspace_roots)?
    {
        return resolve_run_directory_locator(&global_run_directory.to_string_lossy());
    }

    Ok(ResolvedRunLocator {
        run_directory: workspace_run_directory,
        cwd: workspace_dir,
        run_id: run_id.to_string(),
    })
}

fn resolve_run_id_locator_with_global_fallback(
    workspace_dir: &Path,
    run_id: &str,
) -> io::Result<ResolvedRunLocator> {
    resolve_run_id_locator_with_global_fallback_in_roots(
        workspace_dir,
        run_id,
        &candidate_global_workspace_roots()?,
    )
}

pub(crate) fn candidate_workspace_run_roots(base_directory: &Path) -> io::Result<Vec<PathBuf>> {
    let resolved_workspace = normalize_path(base_directory);
    Ok(vec![
        resolve_workspace_ccc_directory(&resolved_workspace).join("runs"),
        resolve_legacy_workspace_ccc_directory(&resolved_workspace).join("runs"),
        resolve_legacy_codex_workspace_ccc_directory(&resolved_workspace)?.join("runs"),
        resolve_central_workspace_ccc_directory(&resolved_workspace)?.join("runs"),
    ])
}

pub(crate) fn inspect_active_runs_for_workspace(
    workspace_dir: &Path,
    current_run_id: Option<&str>,
) -> io::Result<Value> {
    let mut inspected_count = 0_usize;
    let mut active_runs = Vec::new();
    let mut stale_runs = Vec::new();
    let mut seen_run_ids = BTreeSet::<String>::new();

    for runs_root in candidate_workspace_run_roots(workspace_dir)? {
        let Ok(entries) = fs::read_dir(&runs_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let run_directory = entry.path();
            if !run_directory.is_dir() {
                continue;
            }
            let Some(run_id) = run_directory
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if current_run_id == Some(run_id.as_str()) || !seen_run_ids.insert(run_id.clone()) {
                continue;
            }
            let Ok(run_record) = read_json_document(&run_directory.join("run.json")) else {
                continue;
            };
            inspected_count += 1;
            let longway =
                read_json_document(&run_directory.join("longway.json")).unwrap_or(Value::Null);
            let lifecycle_state = longway
                .get("lifecycle_state")
                .and_then(Value::as_str)
                .or_else(|| run_record.get("status").and_then(Value::as_str))
                .unwrap_or("active");
            let child_agents = run_record
                .get("child_agents")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let active_subagent_count = child_agents
                .iter()
                .filter(|entry| {
                    entry
                        .get("status")
                        .and_then(Value::as_str)
                        .map(is_active_host_subagent_status)
                        .unwrap_or(false)
                })
                .count();
            let pending_merge_count = child_agents
                .iter()
                .filter(|entry| {
                    entry
                        .get("status")
                        .and_then(Value::as_str)
                        .map(is_terminal_host_subagent_status)
                        .unwrap_or(false)
                })
                .count();
            let run_summary = json!({
                "run_id": run_id,
                "run_directory": run_directory.to_string_lossy(),
                "run_ref": create_ccc_run_ref(&run_directory),
                "active_task_card_id": run_record.get("active_task_card_id").cloned().unwrap_or(Value::Null),
                "active_agent_id": run_record.get("active_agent_id").cloned().unwrap_or(Value::Null),
                "active_role": run_record.get("active_role").cloned().unwrap_or(Value::Null),
                "active_subagent_count": active_subagent_count,
                "pending_merge_count": pending_merge_count,
                "lifecycle_state": lifecycle_state,
                "updated_at": run_record.get("updated_at").cloned().unwrap_or(Value::Null),
            });
            if matches!(
                lifecycle_state,
                "completed" | "halt_completed" | "resolved" | "reclaimed" | "cancelled"
            ) {
                stale_runs.push(run_summary);
            } else {
                active_runs.push(run_summary);
            }
        }
    }

    let active_count = active_runs.len();
    let stale_count = stale_runs.len();
    let state = if active_count > 0 {
        "active_run_detected"
    } else {
        "no_active_runs"
    };
    let summary = if active_count > 0 {
        format!(
            "{active_count} active run(s) already exist for this workspace; captain should merge, replan, or mark stale subagent work reclaimed before continuing from the latest request."
        )
    } else {
        "No active prior CCC runs were detected for this workspace.".to_string()
    };

    Ok(json!({
        "active_run_scan_state": state,
        "active_run_scan_summary": summary,
        "inspected_active_run_count": inspected_count,
        "fresh_active_run_count": active_count,
        "stale_active_run_count": stale_count,
        "active_runs": active_runs,
        "stale_runs": stale_runs,
        "prior_run_cleanup_performed": false,
        "prior_run_cleanup_summary": if active_count > 0 {
            "No prior run was automatically closed; captain must explicitly merge, replan, reclaim, or close visible stale work."
        } else {
            "No prior active run cleanup was needed."
        },
        "continuity_strategy": if active_count > 0 { "merge_replan_or_reclaim_recommended" } else { "fresh_run_ok" },
        "host_subagent_cancel_supported": false,
        "host_subagent_cancel_summary": "CCC cannot guarantee forced cancellation of host Codex custom subagents; record reclaimed/merged lifecycle truth and continue from the combined latest request.",
    }))
}

pub(crate) fn reclaim_prior_active_runs_for_workspace(
    workspace_dir: &Path,
    current_run_id: Option<&str>,
    timestamp: &str,
) -> io::Result<Value> {
    let mut reclaimed_runs = Vec::new();
    let mut inspected_count = 0_usize;
    let mut seen_run_ids = BTreeSet::<String>::new();

    for runs_root in candidate_workspace_run_roots(workspace_dir)? {
        let Ok(entries) = fs::read_dir(&runs_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let run_directory = entry.path();
            if !run_directory.is_dir() {
                continue;
            }
            let Some(run_id) = run_directory
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if current_run_id == Some(run_id.as_str()) || !seen_run_ids.insert(run_id.clone()) {
                continue;
            }

            let run_file = run_directory.join("run.json");
            let Ok(mut run_record) = read_json_document(&run_file) else {
                continue;
            };
            inspected_count += 1;
            let longway_file = run_directory.join("longway.json");
            let longway = read_json_document(&longway_file).unwrap_or(Value::Null);
            let lifecycle_state = longway
                .get("lifecycle_state")
                .and_then(Value::as_str)
                .or_else(|| run_record.get("status").and_then(Value::as_str))
                .unwrap_or("active");
            if matches!(
                lifecycle_state,
                "completed" | "halt_completed" | "resolved" | "reclaimed" | "cancelled"
            ) {
                continue;
            }

            let active_task_card_id = run_record
                .get("active_task_card_id")
                .and_then(Value::as_str)
                .map(str::to_string);
            let active_agent_id_before = run_record
                .get("active_agent_id")
                .cloned()
                .unwrap_or(Value::Null);
            let active_thread_id_before = run_record
                .get("active_thread_id")
                .cloned()
                .unwrap_or(Value::Null);
            let mut reclaimed_child_count = 0_u64;
            if let Some(children) = run_record
                .get_mut("child_agents")
                .and_then(Value::as_array_mut)
            {
                for child in children {
                    let status = child.get("status").and_then(Value::as_str).unwrap_or("");
                    if is_active_host_subagent_status(status)
                        || is_terminal_host_subagent_status(status)
                    {
                        if let Some(object) = child.as_object_mut() {
                            object.insert(
                                "status".to_string(),
                                Value::String("reclaimed".to_string()),
                            );
                            object.insert(
                                "reclaim_reason".to_string(),
                                Value::String("prior_run_auto_cleanup".to_string()),
                            );
                            object.insert(
                                "updated_at".to_string(),
                                Value::String(timestamp.to_string()),
                            );
                        }
                        reclaimed_child_count += 1;
                    }
                }
            }
            if let Some(executors) = run_record
                .get_mut("specialist_executors")
                .and_then(Value::as_array_mut)
            {
                for executor in executors {
                    let status = executor.get("status").and_then(Value::as_str).unwrap_or("");
                    if is_active_host_subagent_status(status)
                        || is_terminal_host_subagent_status(status)
                    {
                        if let Some(object) = executor.as_object_mut() {
                            object.insert(
                                "status".to_string(),
                                Value::String("reclaimed".to_string()),
                            );
                            object.insert(
                                "reclaim_reason".to_string(),
                                Value::String("prior_run_auto_cleanup".to_string()),
                            );
                            object.insert(
                                "updated_at".to_string(),
                                Value::String(timestamp.to_string()),
                            );
                        }
                    }
                }
            }

            if let Some(object) = run_record.as_object_mut() {
                object.insert("status".to_string(), Value::String("reclaimed".to_string()));
                object.insert(
                    "active_agent_id".to_string(),
                    Value::String("captain".to_string()),
                );
                object.insert(
                    "active_role".to_string(),
                    Value::String("orchestrator".to_string()),
                );
                object.insert("active_thread_id".to_string(), Value::Null);
                object.insert(
                    "updated_at".to_string(),
                    Value::String(timestamp.to_string()),
                );
                object.insert(
                    "completed_at".to_string(),
                    Value::String(timestamp.to_string()),
                );
                object.insert(
                    "latest_reclaim".to_string(),
                    json!({
                        "reason": "prior_run_auto_cleanup",
                        "summary": "New CCC start reclaimed this older active run before continuing.",
                        "reclaimed_at": timestamp,
                    }),
                );
                object.insert(
                    "host_subagent_handle_cleanup".to_string(),
                    json!({
                        "state": "released_by_start_cleanup",
                        "task_card_id": active_task_card_id,
                        "active_agent_id_before": active_agent_id_before,
                        "active_thread_id_before": active_thread_id_before,
                        "released_at": timestamp,
                        "summary": "Prior active host subagent handle was marked reclaimed by new CCC start.",
                    }),
                );
            }
            write_json_document(&run_file, &run_record)?;

            if let Ok(mut longway_record) = read_json_document(&longway_file) {
                if let Some(object) = longway_record.as_object_mut() {
                    object.insert(
                        "lifecycle_state".to_string(),
                        Value::String("reclaimed".to_string()),
                    );
                    object.insert(
                        "active_phase_status".to_string(),
                        Value::String("reclaimed".to_string()),
                    );
                    object.insert(
                        "updated_at".to_string(),
                        Value::String(timestamp.to_string()),
                    );
                    object.insert(
                        "completed_at".to_string(),
                        Value::String(timestamp.to_string()),
                    );
                }
                write_json_document(&longway_file, &longway_record)?;
            }
            let run_state_file = run_directory.join("run-state.json");
            if let Ok(mut run_state_record) = read_json_document(&run_state_file) {
                if let Some(object) = run_state_record.as_object_mut() {
                    object.insert(
                        "updated_at".to_string(),
                        Value::String(timestamp.to_string()),
                    );
                    object.insert(
                        "next_action".to_string(),
                        json!({"command": "halt_reclaimed"}),
                    );
                }
                write_json_document(&run_state_file, &run_state_record)?;
            }
            if let Some(active_task_card_id) = active_task_card_id.as_deref() {
                let task_card_file = run_directory
                    .join("task-cards")
                    .join(format!("{active_task_card_id}.json"));
                if let Ok(mut task_card_record) = read_json_document(&task_card_file) {
                    if let Some(object) = task_card_record.as_object_mut() {
                        object.insert(
                            "updated_at".to_string(),
                            Value::String(timestamp.to_string()),
                        );
                        object.insert(
                            "subagent_lifecycle".to_string(),
                            json!({
                                "status": "reclaimed",
                                "reclaim_reason": "prior_run_auto_cleanup",
                                "summary": "New CCC start reclaimed this older active task-card before continuing.",
                                "recorded_at": timestamp,
                            }),
                        );
                    }
                    write_json_document(&task_card_file, &task_card_record)?;
                }
            }
            reclaimed_runs.push(json!({
                "run_id": run_id,
                "run_directory": run_directory.to_string_lossy(),
                "run_ref": create_ccc_run_ref(&run_directory),
                "reclaimed_child_count": reclaimed_child_count,
                "reclaimed_at": timestamp,
                "reason": "prior_run_auto_cleanup",
            }));
        }
    }

    let reclaimed_count = reclaimed_runs.len();
    Ok(json!({
        "inspected_prior_run_count": inspected_count,
        "reclaimed_prior_run_count": reclaimed_count,
        "reclaimed_runs": reclaimed_runs,
        "prior_run_cleanup_performed": reclaimed_count > 0,
        "prior_run_cleanup_summary": if reclaimed_count > 0 {
            format!("{reclaimed_count} prior active run(s) were marked reclaimed before starting the new run.")
        } else {
            "No prior active run cleanup was needed.".to_string()
        },
    }))
}

pub(crate) fn read_workspace_storage_context_if_present(ccc_dir: &Path) -> Option<PathBuf> {
    let file_path = ccc_dir.join("workspace-context.json");
    let contents = fs::read_to_string(file_path).ok()?;
    let candidate = serde_json::from_str::<Value>(&contents).ok()?;
    candidate
        .get("workspace_dir")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .map(|path| normalize_path(&path))
}

pub(crate) fn resolve_run_directory_locator(run_directory: &str) -> io::Result<ResolvedRunLocator> {
    let candidate = PathBuf::from(run_directory);

    if !candidate.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "run_dir must be an absolute path to a CCC runs/<run-id> directory.",
        ));
    }

    let normalized_run_directory = normalize_path(&candidate);
    let run_id = normalized_run_directory
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "run_dir must point to a CCC runs/<run-id> directory.",
            )
        })?
        .to_string();
    let runs_directory = normalized_run_directory.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "run_dir must point to a CCC runs/<run-id> directory.",
        )
    })?;
    let ccc_directory = runs_directory.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "run_dir must point to a CCC runs/<run-id> directory.",
        )
    })?;

    if run_id == "."
        || run_id == ".."
        || runs_directory.file_name().and_then(|value| value.to_str()) != Some("runs")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "run_dir must point to an absolute CCC runs/<run-id> directory.",
        ));
    }

    let cwd = if matches!(
        ccc_directory.file_name().and_then(|value| value.to_str()),
        Some(".ccc")
    ) {
        ccc_directory.parent().map(normalize_path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "run_dir must point to an absolute CCC runs/<run-id> directory.",
            )
        })?
    } else if let Some(workspace_dir) = read_workspace_storage_context_if_present(ccc_directory) {
        workspace_dir
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "run_dir must point to a legacy workspace CCC run directory or a central workspace CCC run directory.",
        ));
    };

    Ok(ResolvedRunLocator {
        cwd,
        run_id,
        run_directory: normalized_run_directory,
    })
}

pub(crate) fn resolve_run_ref_locator(run_ref: &str) -> io::Result<ResolvedRunLocator> {
    let run_directory = if let Some(run_directory) = run_ref.strip_prefix(CCC_RUN_REF_PREFIX) {
        run_directory
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "run_ref must start with {CCC_RUN_REF_PREFIX} and point to an absolute CCC runs/<run-id> directory."
            ),
        ));
    };

    resolve_run_directory_locator(run_directory)
}

pub(crate) fn resolve_run_locator_arguments(
    arguments: &Value,
    tool_name: &str,
) -> io::Result<ResolvedRunLocator> {
    let value = arguments.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{tool_name} arguments must be an object."),
        )
    })?;

    for key in value.keys() {
        if !matches!(
            key.as_str(),
            "run_id" | "run_ref" | "run_dir" | "cwd" | "compact"
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unexpected {tool_name} argument: {key}."),
            ));
        }
    }

    let run_id = value
        .get("run_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let run_ref = value
        .get("run_ref")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let run_dir = value
        .get("run_dir")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let cwd = value
        .get("cwd")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut resolved: Option<ResolvedRunLocator> = None;

    if let Some(run_id_value) = run_id {
        let workspace_dir = resolve_workspace_path(cwd)?;
        resolved = Some(resolve_run_id_locator_with_global_fallback(
            &workspace_dir,
            run_id_value,
        )?);
    }

    if let Some(run_dir_value) = run_dir {
        let candidate = resolve_run_directory_locator(run_dir_value)?;
        if let Some(previous) = resolved.as_ref() {
            if previous.cwd != candidate.cwd
                || previous.run_id != candidate.run_id
                || previous.run_directory != candidate.run_directory
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Run locator mismatch: run_dir resolves to {} but the earlier locator resolves to {}.",
                        candidate.run_directory.display(),
                        previous.run_directory.display()
                    ),
                ));
            }
        } else {
            resolved = Some(candidate);
        }
    }

    if let Some(run_ref_value) = run_ref {
        let candidate = resolve_run_ref_locator(run_ref_value)?;
        if let Some(previous) = resolved.as_ref() {
            if previous.cwd != candidate.cwd
                || previous.run_id != candidate.run_id
                || previous.run_directory != candidate.run_directory
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Run locator mismatch: run_ref resolves to {} but the earlier locator resolves to {}.",
                        candidate.run_directory.display(),
                        previous.run_directory.display()
                    ),
                ));
            }
        } else {
            resolved = Some(candidate);
        }
    }

    let resolved = resolved.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{tool_name} requires one of run_id, run_ref, or run_dir."),
        )
    })?;

    if run_id.is_none() {
        if let Some(cwd_value) = cwd {
            let hinted_cwd = resolve_workspace_path(Some(cwd_value))?;
            if hinted_cwd != resolved.cwd {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Run locator mismatch: cwd {} does not match the resolved run workspace {}.",
                        hinted_cwd.display(),
                        resolved.cwd.display()
                    ),
                ));
            }
        }
    }

    Ok(resolved)
}

pub(crate) fn create_ccc_run_ref(run_directory: &Path) -> String {
    format!("{CCC_RUN_REF_PREFIX}{}", run_directory.display())
}

pub(crate) fn run_directory_to_ccc_directory(run_directory: &Path) -> io::Result<PathBuf> {
    let runs_directory = run_directory.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "run directory must be nested under runs/",
        )
    })?;
    runs_directory.parent().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "run directory must be nested under runs/",
        )
    })
}

pub(crate) fn ensure_run_paths_for_start(
    workspace_dir: &Path,
    run_directory: &Path,
) -> io::Result<()> {
    let ccc_dir = run_directory_to_ccc_directory(run_directory)?;
    let planner_dir = run_directory.join("way");
    fs::create_dir_all(&ccc_dir)?;
    fs::write(
        ccc_dir.join("workspace-context.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "workspace_dir": normalize_path(workspace_dir).to_string_lossy(),
            "storage_key": compute_workspace_storage_key(workspace_dir),
        }))
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("encode workspace context: {error}"),
            )
        })?,
    )?;
    for path in [
        planner_dir.clone(),
        planner_dir.join("attempts"),
        run_directory.join("orchestration"),
        run_directory.join("orchestration").join("attempts"),
        run_directory.join("explore"),
        run_directory.join("task-cards"),
        run_directory.join("handoffs"),
        run_directory.join("raw-events"),
    ] {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
