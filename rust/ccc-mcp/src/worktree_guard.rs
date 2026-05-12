use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const BASELINE_SCHEMA: &str = "ccc.worktree_mutation_baseline.v1";
const GUARD_SCHEMA: &str = "ccc.captain_direct_mutation_guard.v1";
const IGNORED_PREFIXES: &[&str] = &[".ccc/"];
const IGNORED_PATHS: &[&str] = &["CCC_LONGWAY_PROJECTION.md"];

pub(crate) fn create_worktree_mutation_baseline(workspace_dir: &Path, captured_at: &str) -> Value {
    let snapshot = create_worktree_snapshot(workspace_dir);
    json!({
        "schema": BASELINE_SCHEMA,
        "captured_at": captured_at,
        "source": "git_status_porcelain_v1",
        "ignored_prefixes": IGNORED_PREFIXES,
        "ignored_paths": IGNORED_PATHS,
        "status": snapshot.get("status").cloned().unwrap_or(Value::String("unavailable".to_string())),
        "dirty_path_count": snapshot.get("dirty_path_count").cloned().unwrap_or(Value::from(0)),
        "dirty_paths": snapshot.get("dirty_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "entries": snapshot.get("entries").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "reason": snapshot.get("reason").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn create_captain_direct_mutation_guard(
    workspace_dir: &Path,
    run_record: &Value,
    current_task_card: &Value,
    captain_action_contract: &Value,
    explicit_exception_recorded: bool,
) -> Value {
    let policy_allowed = captain_action_contract
        .pointer("/direct_file_mutation_policy/allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let baseline = run_record
        .get("worktree_mutation_baseline")
        .cloned()
        .unwrap_or(Value::Null);
    let operator_override_recorded = explicit_operator_override_recorded(run_record)
        || explicit_operator_override_recorded(current_task_card);
    let exception_recorded = explicit_exception_recorded || operator_override_recorded;

    if policy_allowed {
        return json!({
            "schema": GUARD_SCHEMA,
            "state": "not_applicable",
            "direct_file_mutation_allowed": true,
            "exception_recorded": exception_recorded,
            "changed_path_count": 0,
            "changed_paths": [],
            "summary": "Direct file mutation policy currently allows captain mutation."
        });
    }

    if exception_recorded {
        return json!({
            "schema": GUARD_SCHEMA,
            "state": "exception_recorded",
            "direct_file_mutation_allowed": false,
            "exception_recorded": true,
            "changed_path_count": 0,
            "changed_paths": [],
            "summary": "Direct mutation exception has been recorded through terminal fallback or operator override."
        });
    }

    if baseline.get("schema").and_then(Value::as_str) != Some(BASELINE_SCHEMA) {
        return json!({
            "schema": GUARD_SCHEMA,
            "state": "baseline_missing",
            "direct_file_mutation_allowed": false,
            "exception_recorded": false,
            "changed_path_count": 0,
            "changed_paths": [],
            "summary": "No run-start worktree mutation baseline is available."
        });
    }

    let current = create_worktree_snapshot(workspace_dir);
    if current.get("status").and_then(Value::as_str) != Some("available") {
        return json!({
            "schema": GUARD_SCHEMA,
            "state": "unavailable",
            "direct_file_mutation_allowed": false,
            "exception_recorded": false,
            "changed_path_count": 0,
            "changed_paths": [],
            "baseline": compact_baseline_summary(&baseline),
            "current": current,
            "summary": "Current git worktree status is unavailable; direct mutation drift cannot be evaluated."
        });
    }

    let changed_paths = changed_dirty_paths_since_baseline(&baseline, &current);
    let state = if changed_paths.is_empty() {
        "clear"
    } else {
        "blocked_unrecorded_direct_mutation"
    };
    let summary = if changed_paths.is_empty() {
        "No new dirty workspace paths since run-start baseline."
    } else {
        "Dirty workspace paths changed since run start while direct captain file mutation is blocked and no terminal fallback/operator override is recorded."
    };
    json!({
        "schema": GUARD_SCHEMA,
        "state": state,
        "direct_file_mutation_allowed": false,
        "exception_recorded": false,
        "changed_path_count": changed_paths.len(),
        "changed_paths": changed_paths,
        "baseline": compact_baseline_summary(&baseline),
        "current": compact_baseline_summary(&current),
        "required_action": if state == "clear" { Value::Null } else { Value::String("record_terminal_fallback_or_operator_override_before_captain_merge".to_string()) },
        "summary": summary,
    })
}

fn create_worktree_snapshot(workspace_dir: &Path) -> Value {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_dir)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--untracked-files=all")
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return json!({
                "status": "unavailable",
                "reason": format!("git status unavailable: {error}"),
                "dirty_path_count": 0,
                "dirty_paths": [],
                "entries": [],
            });
        }
    };

    if !output.status.success() {
        return json!({
            "status": "unavailable",
            "reason": format!("git status exited with status {}", output.status),
            "dirty_path_count": 0,
            "dirty_paths": [],
            "entries": [],
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let entries = text
        .lines()
        .filter_map(|line| parse_porcelain_line(line))
        .filter(|entry| !ignored_worktree_path(&entry.path))
        .map(|entry| {
            let content_sha256 = content_sha256_for_path(workspace_dir, &entry.path);
            json!({
                "path": entry.path,
                "status": entry.status,
                "content_sha256": content_sha256,
            })
        })
        .collect::<Vec<_>>();
    let dirty_paths = entries
        .iter()
        .filter_map(|entry| entry.get("path").and_then(Value::as_str))
        .map(|path| Value::String(path.to_string()))
        .collect::<Vec<_>>();

    json!({
        "status": "available",
        "dirty_path_count": dirty_paths.len(),
        "dirty_paths": dirty_paths,
        "entries": entries,
    })
}

struct PorcelainEntry {
    path: String,
    status: String,
}

fn parse_porcelain_line(line: &str) -> Option<PorcelainEntry> {
    if line.len() < 4 {
        return None;
    }
    let status = line.get(0..2)?.to_string();
    let raw_path = line.get(3..)?.trim();
    let path = raw_path
        .rsplit_once(" -> ")
        .map(|(_, target)| target)
        .unwrap_or(raw_path)
        .trim_matches('"')
        .to_string();
    if path.is_empty() {
        return None;
    }
    Some(PorcelainEntry { path, status })
}

fn ignored_worktree_path(path: &str) -> bool {
    let normalized = path.trim_start_matches("./");
    IGNORED_PATHS.iter().any(|ignored| normalized == *ignored)
        || IGNORED_PREFIXES
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
}

fn content_sha256_for_path(workspace_dir: &Path, relative_path: &str) -> Value {
    let path = workspace_dir.join(relative_path);
    let Ok(metadata) = fs::metadata(&path) else {
        return Value::Null;
    };
    if !metadata.is_file() {
        return Value::Null;
    }
    match fs::read(&path) {
        Ok(bytes) => {
            let digest = Sha256::digest(&bytes);
            Value::String(
                digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
            )
        }
        Err(_) => Value::Null,
    }
}

fn entries_by_path(snapshot: &Value) -> BTreeMap<String, Value> {
    snapshot
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.get("path").and_then(Value::as_str)?;
            Some((path.to_string(), entry.clone()))
        })
        .collect()
}

fn changed_dirty_paths_since_baseline(baseline: &Value, current: &Value) -> Vec<Value> {
    let baseline_entries = entries_by_path(baseline);
    entries_by_path(current)
        .into_iter()
        .filter_map(|(path, entry)| {
            let changed = baseline_entries
                .get(&path)
                .map(|baseline_entry| baseline_entry != &entry)
                .unwrap_or(true);
            changed.then_some(Value::String(path))
        })
        .collect()
}

fn compact_baseline_summary(snapshot: &Value) -> Value {
    json!({
        "status": snapshot.get("status").cloned().unwrap_or(Value::String("unavailable".to_string())),
        "dirty_path_count": snapshot.get("dirty_path_count").cloned().unwrap_or(Value::from(0)),
        "dirty_paths": snapshot.get("dirty_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "captured_at": snapshot.get("captured_at").cloned().unwrap_or(Value::Null),
        "reason": snapshot.get("reason").cloned().unwrap_or(Value::Null),
    })
}

fn explicit_operator_override_recorded(value: &Value) -> bool {
    [
        "/operator_override",
        "/explicit_operator_override",
        "/captain_action_contract/operator_override_recorded",
        "/direct_file_mutation_policy/operator_override_recorded",
    ]
    .iter()
    .any(|pointer| {
        value
            .pointer(pointer)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}
