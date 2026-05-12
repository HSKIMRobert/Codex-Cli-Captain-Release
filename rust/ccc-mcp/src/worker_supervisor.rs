use crate::run_locator::normalize_path;
use crate::specialist_roles::{
    agent_id_for_role, collect_codex_config_overrides, create_role_model_launch_evidence,
    load_role_config_snapshot, sandbox_mode_for_role, task_expertise_framing_for_role,
    DEFAULT_COMMIT_MESSAGE_GUIDANCE,
};
use crate::text_utils::{compact_prompt_text, prompt_fields_match};
use crate::worker_events::build_worker_completion_snapshot;
use crate::worker_lifecycle::{
    finalize_delegation_with_completion, refresh_running_delegation_heartbeat,
};
use crate::{
    generate_uuid_like_id, load_runtime_config, read_json_document, resolve_codex_home,
    write_json_document,
};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub(crate) fn build_task_execution_prompt(task_card: &Value) -> String {
    let runtime_config = load_runtime_config().unwrap_or_else(|_| {
        json!({
            "worker_prompt_scope_max_chars": 320,
            "worker_prompt_acceptance_max_chars": 220,
            "worker_prompt_task_max_chars": 720,
        })
    });
    let title = task_card
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled task");
    let scope = task_card
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("No explicit scope.");
    let acceptance = task_card
        .get("acceptance")
        .and_then(Value::as_str)
        .unwrap_or("No explicit acceptance criteria.");
    let assigned_agent_id = task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .unwrap_or("raider");
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("code specialist");
    let execution_prompt = task_card
        .get("execution_prompt")
        .and_then(Value::as_str)
        .unwrap_or("Implement the bounded task.");
    let sandbox_mode = task_card
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .unwrap_or("workspace-write");
    let sandbox_rationale = task_card
        .get("sandbox_rationale")
        .and_then(Value::as_str)
        .unwrap_or("Captain selected the sandbox mode for this bounded task.");
    let task_shape = task_card
        .pointer("/expertise_framing/task_shape")
        .and_then(Value::as_str)
        .unwrap_or("single_scoped_task");
    let fallback_expertise_framing = task_expertise_framing_for_role(assigned_role, task_shape);
    let expertise_framing = task_card
        .get("expertise_framing")
        .filter(|value| value.is_object())
        .unwrap_or(&fallback_expertise_framing);
    let expertise_phrase = expertise_framing
        .get("expertise_phrase")
        .and_then(Value::as_str)
        .unwrap_or("You are an expert in bounded implementation, repair, module ownership, and focused validation.");
    let task_stance = expertise_framing
        .get("task_stance")
        .and_then(Value::as_str)
        .unwrap_or("bounded_implementation");
    let expected_thinking_mode = expertise_framing
        .get("expected_thinking_mode")
        .and_then(Value::as_str)
        .unwrap_or("smallest-defensible-change");
    let title = compact_prompt_text(title, 120);
    let scope = compact_prompt_text(
        scope,
        runtime_config
            .get("worker_prompt_scope_max_chars")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(600),
    );
    let acceptance = compact_prompt_text(
        acceptance,
        runtime_config
            .get("worker_prompt_acceptance_max_chars")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(400),
    );
    let execution_prompt = compact_prompt_text(
        execution_prompt,
        runtime_config
            .get("worker_prompt_task_max_chars")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(1_200),
    );
    let mut sections = vec![
        format!("Role: {assigned_agent_id} ({assigned_role})."),
        format!("Expertise: {expertise_phrase}"),
        format!("Stance: {task_stance}; mode: {expected_thinking_mode}."),
        format!("Title: {title}"),
    ];
    if !prompt_fields_match(&scope, &execution_prompt) {
        sections.push(format!("Scope: {scope}"));
    }
    if !prompt_fields_match(&acceptance, &execution_prompt)
        && !prompt_fields_match(&acceptance, &scope)
    {
        sections.push(format!("Acceptance: {acceptance}"));
    }
    sections.push(format!(
        "Sandbox: {sandbox_mode} ({})",
        compact_prompt_text(sandbox_rationale, 160)
    ));
    sections.push(String::new());
    sections.push(
        "Delegated CCC specialist: work only this bounded task; workspace files are primary context."
            .to_string(),
    );
    sections.push(
        "External paths: use exact operator paths only when readable and sandbox-allowed; copy only needed content."
            .to_string(),
    );
    sections.push(
        "If external paths or writes are blocked by sandbox/permissions, stop and return blocked with exact path and approval needed."
            .to_string(),
    );
    sections.push(
        "CCC boundary: do not edit .ccc, run.json, run-state.json, longway.json, task-cards, delegations, or raw-events; stdout only, supervisor persists fan-in."
            .to_string(),
    );
    sections.push(
        "Anti-duplication: own only the delegated search/mutation scope; do not repeat another agent's assigned exploration unless reclaim, stale output, or an explicit reason is recorded."
            .to_string(),
    );
    sections.push(
        "Do not invoke `$cap`, `ccc`, or any nested CCC workflow. Do not hand off.".to_string(),
    );
    let commit_related = [
        title.as_str(),
        scope.as_str(),
        acceptance.as_str(),
        execution_prompt.as_str(),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains("commit"));
    if commit_related {
        sections.push(DEFAULT_COMMIT_MESSAGE_GUIDANCE.to_string());
    }
    sections.push(
        "Return compact fan-in: summary, status, evidence_paths, next_action, open_questions, confidence; include validation commands/checks and unresolved risk when work is complete."
            .to_string(),
    );
    sections.push(String::new());
    sections.push("Task:".to_string());
    sections.push(execution_prompt);
    sections.join("\n")
}

fn create_delegation_worker_lifecycle_record(
    created_at: &str,
    state: &str,
    reclaim_state: &str,
    launch_requested_at: Option<&str>,
    started_at: Option<&str>,
    process_id: Option<u32>,
    process_started_at: Option<&str>,
    process_last_seen_at: Option<&str>,
    last_progress_at: Option<&str>,
) -> Value {
    let runtime_config = load_runtime_config().unwrap_or_else(|_| {
        json!({
            "worker_stuck_after_ms": 45_000
        })
    });
    let stale_after_ms = runtime_config
        .get("worker_stuck_after_ms")
        .and_then(Value::as_u64)
        .unwrap_or(45_000);
    json!({
        "state": state,
        "reclaim_state": reclaim_state,
        "queued_at": created_at,
        "launch_requested_at": launch_requested_at,
        "started_at": started_at,
        "process_id": process_id.map(|value| value as u64),
        "process_started_at": process_started_at,
        "process_last_seen_at": process_last_seen_at,
        "last_progress_at": last_progress_at.unwrap_or(created_at),
        "returned_at": Value::Null,
        "stale_at": Value::Null,
        "timed_out_at": Value::Null,
        "stale_after_ms": stale_after_ms,
        "timeout_after_ms": stale_after_ms,
        "summary": match state {
            "running" => "Worker is actively running.",
            "running_active" => "Worker is actively running.",
            "running_quiet" => "Worker is alive without recent event output.",
            "launching" => "Worker launch was requested and is awaiting process confirmation.",
            "returned" => "Worker returned and is ready for captain fan-in.",
            "failed" => "Worker ended in failure and needs captain follow-up.",
            "cancelled" => "Worker was cancelled under captain control.",
            "stale" => "Worker appears stale and should be reclaimed explicitly.",
            "timed_out" => "Worker exceeded the bounded timeout window and should be reclaimed explicitly.",
            _ => "Worker is queued for execution.",
        }
    })
}

fn create_worker_supervisor_spec(
    workspace_dir: &Path,
    run_directory: &Path,
    codex_bin: &str,
    task_card: &Value,
    delegation_id: &str,
    delegation_file: &Path,
    raw_events_file: &Path,
    worker_codex_home: &Path,
    role_config_snapshot: &Value,
    sandbox_mode: &str,
    prompt: &str,
    launched_at: &str,
) -> Value {
    json!({
        "workspace_dir": workspace_dir.to_string_lossy(),
        "run_directory": run_directory.to_string_lossy(),
        "codex_bin": codex_bin,
        "delegation_id": delegation_id,
        "delegation_file": delegation_file.to_string_lossy(),
        "raw_events_file": raw_events_file.to_string_lossy(),
        "worker_codex_home": worker_codex_home.to_string_lossy(),
        "sandbox_mode": sandbox_mode,
        "prompt": prompt,
        "task_card": task_card,
        "role_config_snapshot": role_config_snapshot,
        "launched_at": launched_at,
    })
}

fn launch_worker_supervisor(run_directory: &Path, spec: &Value) -> io::Result<(u32, PathBuf)> {
    let supervisors_dir = run_directory.join("supervisors");
    fs::create_dir_all(&supervisors_dir)?;
    let delegation_id = spec
        .get("delegation_id")
        .and_then(Value::as_str)
        .unwrap_or("worker");
    let spec_file = supervisors_dir.join(format!("{delegation_id}.json"));
    write_json_document(&spec_file, spec)?;
    #[cfg(test)]
    {
        run_worker_supervisor(spec)?;
        Ok((std::process::id(), spec_file))
    }
    #[cfg(not(test))]
    {
        let current_exe = std::env::current_exe().map_err(|error| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Unable to resolve current CCC executable for worker supervision: {error}"),
            )
        })?;
        let mut command = Command::new(current_exe);
        #[cfg(unix)]
        command.process_group(0);
        let child = command
            .arg("worker-supervise")
            .arg("--json-file")
            .arg(&spec_file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok((child.id(), spec_file))
    }
}

fn read_launch_result_from_delegation(
    delegation_file: &Path,
    delegation_id: &str,
    raw_events_file: &Path,
    assigned_agent_id: &str,
    assigned_role: &str,
    launched_at: &str,
) -> Value {
    let delegation = read_json_document(delegation_file).unwrap_or(Value::Null);
    let status = delegation
        .get("child_agent")
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str);
    let lifecycle_state = delegation
        .get("worker_lifecycle")
        .and_then(|value| value.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("launching");
    json!({
        "delegation_id": delegation_id,
        "raw_events_file": raw_events_file.to_string_lossy(),
        "child_agent_id": assigned_agent_id,
        "assigned_role": assigned_role,
        "launched_at": launched_at,
        "terminal_status": match status {
            Some("completed" | "failed" | "cancelled") => Value::String(status.unwrap().to_string()),
            _ => Value::Null,
        },
        "worker_state": lifecycle_state,
        "completed_at": delegation.get("completed_at").cloned().unwrap_or(Value::Null),
        "result_summary": delegation.get("result_summary").cloned().unwrap_or(Value::Null),
    })
}

fn prepare_worker_codex_home(run_directory: &Path, delegation_id: &str) -> io::Result<PathBuf> {
    let source_codex_home = resolve_codex_home()?;
    let source_auth = source_codex_home.join("auth.json");
    if !source_auth.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Codex auth.json was not found at {}.",
                source_auth.display()
            ),
        ));
    }

    let worker_codex_home = run_directory.join("worker-codex-home").join(delegation_id);
    fs::create_dir_all(&worker_codex_home)?;
    fs::copy(&source_auth, worker_codex_home.join("auth.json"))?;
    Ok(normalize_path(&worker_codex_home))
}

pub(crate) fn spawn_codex_exec_for_task(
    workspace_dir: &Path,
    run_directory: &Path,
    codex_bin: &str,
    task_card: &Value,
) -> io::Result<Value> {
    let raw_events_dir = run_directory.join("raw-events");
    let delegations_dir = run_directory.join("delegations");
    fs::create_dir_all(&raw_events_dir)?;
    fs::create_dir_all(&delegations_dir)?;

    let task_card_id = task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "task card is missing task_card_id.",
            )
        })?;
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .unwrap_or("code specialist");
    let assigned_agent_id = task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .or_else(|| agent_id_for_role(assigned_role))
        .unwrap_or("raider");
    let delegation_id = generate_uuid_like_id();
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let raw_events_file = raw_events_dir.join(format!("{delegation_id}.jsonl"));
    let worker_codex_home = prepare_worker_codex_home(run_directory, &delegation_id)?;
    let prompt = build_task_execution_prompt(task_card);
    let task_kind = task_card
        .get("task_kind")
        .and_then(Value::as_str)
        .unwrap_or("execution");
    let role_config_snapshot = task_card
        .get("role_config_snapshot")
        .cloned()
        .unwrap_or_else(|| load_role_config_snapshot(assigned_role));
    let sandbox_mode = task_card
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "read-only" | "workspace-write"))
        .unwrap_or_else(|| sandbox_mode_for_role(assigned_role));
    let dispatched_config_entries = collect_codex_config_overrides(&role_config_snapshot);
    let model_launch_evidence = create_role_model_launch_evidence(
        assigned_role,
        task_kind,
        codex_bin,
        &role_config_snapshot,
        &dispatched_config_entries,
        &timestamp,
    );

    let delegation_file = delegations_dir.join(format!("{delegation_id}.json"));
    let delegation_payload = json!({
        "delegation_id": delegation_id,
        "run_id": task_card.get("run_id").cloned().unwrap_or(Value::Null),
        "task_card_id": task_card_id,
        "delegated_by_role": "orchestrator",
        "review_round": Value::Null,
        "summary": format!("Primary worker {assigned_agent_id} launched for task \"{}\".", task_card.get("title").and_then(Value::as_str).unwrap_or("untitled task")),
        "child_agent": {
            "agent_id": assigned_agent_id,
            "parent_agent_id": "captain",
            "role": assigned_role,
            "status": "running",
            "task_card_id": task_card_id
        },
        "executor": {
            "executor_id": format!("specialist-executor:{assigned_agent_id}"),
            "status": "running",
            "task_card_id": task_card_id,
            "delegation_id": delegation_id,
            "child_agent_id": assigned_agent_id
        },
        "worker_request": {
            "prompt": build_task_execution_prompt(task_card),
            "acceptance": task_card.get("acceptance").cloned().unwrap_or(Value::Null),
            "scope": task_card.get("scope").cloned().unwrap_or(Value::Null),
            "sandbox_mode": task_card.get("sandbox_mode").cloned().unwrap_or(Value::String(sandbox_mode.to_string())),
            "sandbox_rationale": task_card.get("sandbox_rationale").cloned().unwrap_or(Value::Null)
        },
        "worker_launch_evidence": {
            "launch_source": "ccc_spawn",
            "codex_path": codex_bin,
            "worker_codex_home": worker_codex_home.to_string_lossy(),
            "sandbox_mode": sandbox_mode,
            "match_state": "verified_match",
            "mismatch_summary": Value::Null,
            "recorded_at": timestamp,
            "raw_events_file": raw_events_file.to_string_lossy(),
            "role_config_snapshot": role_config_snapshot,
        },
        "role_config_snapshot": task_card.get("role_config_snapshot").cloned().unwrap_or(Value::Null),
        "latest_model_launch": model_launch_evidence.clone(),
        "worker_lifecycle": create_delegation_worker_lifecycle_record(
            &timestamp,
            "launching",
            "not_needed",
            Some(&timestamp),
            None,
            None,
            None,
            None,
            None,
        ),
        "worker_result": Value::Null,
        "result_summary": Value::Null,
        "reviewer_outcome": Value::Null,
        "latest_failure": Value::Null,
        "created_at": timestamp,
        "updated_at": timestamp,
        "completed_at": Value::Null,
    });
    write_json_document(&delegation_file, &delegation_payload)?;

    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    if let Ok(mut task_card_record) = read_json_document(&task_card_file) {
        if let Some(task_card_object) = task_card_record.as_object_mut() {
            task_card_object.insert("latest_model_launch".to_string(), model_launch_evidence);
            task_card_object.insert("updated_at".to_string(), Value::String(timestamp.clone()));
        }
        let _ = write_json_document(&task_card_file, &task_card_record);
    }
    let supervisor_spec = create_worker_supervisor_spec(
        workspace_dir,
        run_directory,
        codex_bin,
        task_card,
        &delegation_id,
        &delegation_file,
        &raw_events_file,
        &worker_codex_home,
        &role_config_snapshot,
        sandbox_mode,
        &prompt,
        &timestamp,
    );
    let (supervisor_process_id, supervisor_spec_file) =
        launch_worker_supervisor(run_directory, &supervisor_spec)?;
    let mut delegation = read_json_document(&delegation_file)?;
    if let Some(object) = delegation.as_object_mut() {
        if let Some(launch_evidence) = object
            .get_mut("worker_launch_evidence")
            .and_then(Value::as_object_mut)
        {
            launch_evidence.insert(
                "supervisor_process_id".to_string(),
                Value::from(supervisor_process_id as u64),
            );
            launch_evidence.insert(
                "supervisor_spec_file".to_string(),
                Value::String(supervisor_spec_file.to_string_lossy().into_owned()),
            );
        }
    }
    write_json_document(&delegation_file, &delegation)?;
    #[cfg(test)]
    let launch_settle_attempts = 40;
    #[cfg(not(test))]
    let launch_settle_attempts = 16;
    for _ in 0..launch_settle_attempts {
        thread::sleep(Duration::from_millis(25));
        let refreshed = refresh_running_delegation_heartbeat(
            run_directory,
            &delegation_file,
            read_json_document(&delegation_file)?,
        )?;
        let terminal = refreshed
            .get("child_agent")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(|value| matches!(value, "completed" | "failed" | "cancelled"))
            .unwrap_or(false);
        write_json_document(&delegation_file, &refreshed)?;
        if terminal {
            break;
        }
    }
    let mut launch_result = read_launch_result_from_delegation(
        &delegation_file,
        &delegation_id,
        &raw_events_file,
        assigned_agent_id,
        assigned_role,
        &timestamp,
    );
    if let Some(object) = launch_result.as_object_mut() {
        object.insert(
            "supervisor_process_id".to_string(),
            Value::from(supervisor_process_id as u64),
        );
    }
    Ok(launch_result)
}

pub(crate) fn run_worker_supervisor(spec: &Value) -> io::Result<Value> {
    let workspace_dir = PathBuf::from(
        spec.get("workspace_dir")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "worker-supervise requires workspace_dir.",
                )
            })?,
    );
    let codex_bin = spec
        .get("codex_bin")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "worker-supervise requires codex_bin.",
            )
        })?;
    let delegation_file = PathBuf::from(
        spec.get("delegation_file")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "worker-supervise requires delegation_file.",
                )
            })?,
    );
    let raw_events_file = PathBuf::from(
        spec.get("raw_events_file")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "worker-supervise requires raw_events_file.",
                )
            })?,
    );
    let worker_codex_home = PathBuf::from(
        spec.get("worker_codex_home")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "worker-supervise requires worker_codex_home.",
                )
            })?,
    );
    let sandbox_mode = spec
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .unwrap_or("workspace-write");
    let prompt = spec
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("Implement the bounded task.");
    let role_config_snapshot = spec
        .get("role_config_snapshot")
        .cloned()
        .unwrap_or(Value::Null);
    let dispatched_config_entries = collect_codex_config_overrides(&role_config_snapshot);
    let stdout_file = fs::File::create(&raw_events_file)?;
    let stderr_file = stdout_file.try_clone()?;
    let mut command = Command::new(codex_bin);
    #[cfg(unix)]
    command.process_group(0);
    command
        .arg("exec")
        .arg("--ignore-user-config")
        .arg("--ephemeral")
        .arg("--json")
        .arg("--sandbox")
        .arg(sandbox_mode)
        .arg("--skip-git-repo-check");
    command.env("CODEX_HOME", &worker_codex_home);
    if let Some(profile) = role_config_snapshot
        .get("profile")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.arg("--profile").arg(profile);
    }
    if let Some(model) = role_config_snapshot
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.arg("--model").arg(model);
    }
    for entry in &dispatched_config_entries {
        command.arg("-c").arg(entry);
    }

    let launched_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut child = command
        .arg("-")
        .current_dir(&workspace_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;
    let process_id = child.id();
    let mut delegation = read_json_document(&delegation_file)?;
    if let Some(object) = delegation.as_object_mut() {
        object.insert("updated_at".to_string(), Value::String(launched_at.clone()));
        if let Some(lifecycle) = object
            .get_mut("worker_lifecycle")
            .and_then(Value::as_object_mut)
        {
            lifecycle.insert(
                "state".to_string(),
                Value::String("running_quiet".to_string()),
            );
            lifecycle.insert("started_at".to_string(), Value::String(launched_at.clone()));
            lifecycle.insert("process_id".to_string(), Value::from(process_id as u64));
            lifecycle.insert(
                "process_started_at".to_string(),
                Value::String(launched_at.clone()),
            );
            lifecycle.insert(
                "process_last_seen_at".to_string(),
                Value::String(launched_at.clone()),
            );
            lifecycle.insert(
                "last_progress_at".to_string(),
                Value::String(launched_at.clone()),
            );
            lifecycle.insert(
                "summary".to_string(),
                Value::String("Detached worker supervisor launched the Codex child.".to_string()),
            );
        }
    }
    write_json_document(&delegation_file, &delegation)?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }
    let child_exit_status = child.wait().ok();
    let completed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut delegation = read_json_document(&delegation_file)?;
    let (status, summary, worker_result, latest_failure) = build_worker_completion_snapshot(
        &raw_events_file,
        &completed_at,
        child_exit_status.as_ref(),
    );
    finalize_delegation_with_completion(
        &mut delegation,
        &completed_at,
        &status,
        &summary,
        worker_result,
        latest_failure,
    )?;
    write_json_document(&delegation_file, &delegation)?;

    Ok(json!({
        "delegation_id": spec.get("delegation_id").cloned().unwrap_or(Value::Null),
        "process_id": process_id,
        "status": status,
        "completed_at": completed_at,
        "result_summary": summary,
    }))
}
