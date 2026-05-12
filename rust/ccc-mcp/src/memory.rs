use crate::{read_json_document, read_optional_shared_config_document, write_string_atomic};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const MEMORY_SCHEMA_VERSION: u32 = 1;
const MEMORY_MAX_ENTRIES: usize = 50;
const MEMORY_MAX_FILE_BYTES: usize = 16 * 1024;
const MEMORY_MAX_TEXT_CHARS: usize = 400;
const MEMORY_STALE_AFTER_MS: u128 = 30 * 24 * 60 * 60 * 1000;
const MEMORY_PREVIEW_TOKEN_PREFIX: &str = "ccc-memory-preview-v1";

const ALLOWED_ENTRY_KINDS: [&str; 4] = [
    "user_preference",
    "repeated_rule",
    "verified_project_fact",
    "captain_instruction",
];
const ALLOWED_SOURCE_KINDS: [&str; 3] = ["operator_confirmation", "project_file", "test_result"];
const FORBIDDEN_TRUTH_SOURCES: [&str; 4] =
    ["longway", "run_state", "latest_work_result", "inference"];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct CccMemoryStore {
    pub(crate) schema_version: u32,
    pub(crate) enabled: bool,
    pub(crate) workspace: String,
    pub(crate) updated_at_unix_ms: u128,
    pub(crate) entries: Vec<CccMemoryEntry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct CccMemoryEntry {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) text: String,
    pub(crate) source_kind: String,
    pub(crate) source: String,
    pub(crate) certainty: String,
    pub(crate) evidence_paths: Vec<String>,
    pub(crate) created_at_unix_ms: u128,
    pub(crate) updated_at_unix_ms: u128,
}

#[derive(Clone, Debug)]
struct MemoryProposal {
    accepted: Vec<CccMemoryEntry>,
    rejected: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TolariaMemoryMirror {
    vault_path: PathBuf,
    repo_folder: PathBuf,
    note_path: PathBuf,
    relative_note_path: String,
}

pub(crate) fn default_memory_store_path(workspace: &Path) -> PathBuf {
    workspace.join(".ccc").join("memory.json")
}

pub(crate) fn create_memory_payload(arguments: &Value) -> io::Result<Value> {
    if arguments.get("store_path").is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "store_path is not accepted for public memory JSON; CCC memory always uses workspace .ccc/memory.json",
        ));
    }

    let workspace = resolve_memory_workspace(arguments)?;
    let store_path = default_memory_store_path(&workspace);
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("status");

    match action {
        "status" => Ok(json!({
            "action": "status",
            "memory": create_memory_status_payload_at_with_arguments(&workspace, &store_path, arguments)
        })),
        "preview" => create_memory_preview_payload(&workspace, &store_path, arguments),
        "write" => create_memory_write_payload(&workspace, &store_path, arguments),
        "off" => create_memory_off_payload(&workspace, &store_path, arguments),
        unknown => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unknown memory action: {unknown}"),
        )),
    }
}

pub(crate) fn create_memory_status_payload(cwd: &Path) -> Value {
    let workspace = normalize_workspace_path(cwd);
    let store_path = default_memory_store_path(&workspace);
    create_memory_status_payload_at(&workspace, &store_path)
}

fn create_memory_preview_payload(
    workspace: &Path,
    store_path: &Path,
    arguments: &Value,
) -> io::Result<Value> {
    let current_store = load_memory_store_with_tolaria(workspace, store_path, arguments)?;
    let proposal = create_memory_entry_proposal(arguments, now_unix_ms());
    let proposed_store = apply_entries_to_store(workspace, current_store.clone(), &proposal)?;
    let diff = create_memory_diff(current_store.as_ref(), &proposed_store)?;
    let expected_updated_at = current_store.as_ref().map(|store| store.updated_at_unix_ms);
    let preview_token =
        create_entries_preview_token("write", expected_updated_at, &proposal.accepted)?;
    Ok(json!({
        "action": "preview",
        "written": false,
        "memory": create_memory_status_payload_at_with_arguments(workspace, store_path, arguments),
        "accepted_entries": proposal.accepted,
        "rejected_entries": proposal.rejected,
        "diff": diff,
        "next_write": {
            "action": "write",
            "preview_ack": true,
            "preview_token": preview_token,
            "expected_updated_at_unix_ms": expected_updated_at
        }
    }))
}

fn create_memory_write_payload(
    workspace: &Path,
    store_path: &Path,
    arguments: &Value,
) -> io::Result<Value> {
    if !arguments
        .get("preview_ack")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "`ccc memory` writes require preview_ack=true after reviewing the preview diff.",
        ));
    }

    let current_store = load_memory_store_with_tolaria(workspace, store_path, arguments)?;
    ensure_expected_memory_version(arguments, current_store.as_ref())?;
    let proposal = create_memory_entry_proposal(arguments, now_unix_ms());
    if !proposal.rejected.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "memory write rejected one or more entries; rerun preview and fix rejected_entries.",
        ));
    }
    if proposal.accepted.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "memory write requires at least one accepted entry.",
        ));
    }
    let expected_token = create_entries_preview_token(
        "write",
        current_store.as_ref().map(|store| store.updated_at_unix_ms),
        &proposal.accepted,
    )?;
    ensure_preview_token(arguments, &expected_token)?;

    let proposed_store = apply_entries_to_store(workspace, current_store.clone(), &proposal)?;
    write_memory_store(store_path, &proposed_store)?;
    let tolaria = sync_memory_store_to_tolaria(
        &resolve_tolaria_memory_mirror(arguments, workspace),
        &proposed_store,
        store_path,
    );
    let diff = create_memory_diff(current_store.as_ref(), &proposed_store)?;
    Ok(json!({
        "action": "write",
        "written": true,
        "memory": create_memory_status_payload_at_with_arguments(workspace, store_path, arguments),
        "tolaria": tolaria.unwrap_or_else(|| json!({
            "available": false,
            "enabled": false,
            "reason": "Tolaria memory mirror is not configured or not detected"
        })),
        "accepted_entries": proposal.accepted,
        "rejected_entries": [],
        "diff": diff
    }))
}

fn create_memory_off_payload(
    workspace: &Path,
    store_path: &Path,
    arguments: &Value,
) -> io::Result<Value> {
    let current_store = load_memory_store_with_tolaria(workspace, store_path, arguments)?;
    let mut proposed_store = current_store
        .clone()
        .unwrap_or_else(|| empty_memory_store(workspace, now_unix_ms()));
    proposed_store.enabled = false;
    proposed_store.updated_at_unix_ms = now_unix_ms();
    let diff = create_memory_diff(current_store.as_ref(), &proposed_store)?;
    let expected_updated_at = current_store.as_ref().map(|store| store.updated_at_unix_ms);
    let preview_token = create_off_preview_token(current_store.as_ref(), expected_updated_at)?;
    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if apply {
        ensure_expected_memory_version(arguments, current_store.as_ref())?;
        ensure_preview_token(arguments, &preview_token)?;
        write_memory_store(store_path, &proposed_store)?;
        let _ = sync_memory_store_to_tolaria(
            &resolve_tolaria_memory_mirror(arguments, workspace),
            &proposed_store,
            store_path,
        );
    }

    Ok(json!({
        "action": "off",
        "written": apply,
        "memory": create_memory_status_payload_at_with_arguments(workspace, store_path, arguments),
        "diff": diff,
        "next_write": if apply {
            Value::Null
        } else {
            json!({
                "action": "off",
                "apply": true,
                "preview_token": preview_token,
                "expected_updated_at_unix_ms": expected_updated_at
            })
        }
    }))
}

fn create_memory_status_payload_at(workspace: &Path, store_path: &Path) -> Value {
    create_memory_status_payload_at_with_arguments(workspace, store_path, &Value::Null)
}

fn create_memory_status_payload_at_with_arguments(
    workspace: &Path,
    store_path: &Path,
    arguments: &Value,
) -> Value {
    let tolaria_mirror = resolve_tolaria_memory_mirror(arguments, workspace);
    match load_memory_store_with_tolaria(workspace, store_path, arguments) {
        Ok(Some(store)) => {
            let entry_counts = entry_counts_by_kind(&store);
            let source_counts = entry_counts_by_source_kind(&store);
            let captain_instruction_source_counts =
                captain_instruction_counts_by_source_kind(&store);
            let captain_instruction_source_summary =
                compact_count_summary(&captain_instruction_source_counts);
            let captain_instruction_count = entry_counts
                .get("captain_instruction")
                .copied()
                .unwrap_or(0);
            let captain_instruction_status = if store.enabled && captain_instruction_count > 0 {
                "active"
            } else if store.enabled {
                "none"
            } else {
                "off"
            };
            let stale =
                now_unix_ms().saturating_sub(store.updated_at_unix_ms) > MEMORY_STALE_AFTER_MS;
            json!({
                "available": true,
                "configured": true,
                "enabled": store.enabled,
                "schema_version": store.schema_version,
                "workspace": path_key(workspace),
                "path": store_path.to_string_lossy(),
                "entry_count": store.entries.len(),
                "entry_counts": entry_counts,
                "entry_source_counts": source_counts,
                "captain_instruction_count": captain_instruction_count,
                "captain_instruction_status": captain_instruction_status,
                "captain_instruction_source": "ccc_memory",
                "captain_instruction_source_counts": captain_instruction_source_counts,
                "captain_instruction_source_summary": captain_instruction_source_summary,
                "updated_at_unix_ms": store.updated_at_unix_ms,
                "stale": stale,
                "stale_reason": if stale { Value::String("memory has not been refreshed in 30 days; verify before relying on it".to_string()) } else { Value::Null },
                "tolaria": tolaria_memory_status(&tolaria_mirror, "memory_loaded").unwrap_or_else(|| json!({
                    "available": false,
                    "enabled": false,
                    "reason": "Tolaria memory mirror is not configured or not detected"
                })),
                "allowed_entry_kinds": ALLOWED_ENTRY_KINDS,
                "allowed_source_kinds": ALLOWED_SOURCE_KINDS,
                "forbidden_truth_sources": FORBIDDEN_TRUTH_SOURCES,
                "inference_entries_allowed": false,
                "max_entries": MEMORY_MAX_ENTRIES,
                "max_file_bytes": MEMORY_MAX_FILE_BYTES
            })
        }
        Ok(None) => json!({
            "available": false,
            "configured": false,
            "enabled": false,
            "workspace": path_key(workspace),
            "path": store_path.to_string_lossy(),
            "entry_count": 0,
            "entry_counts": {},
            "entry_source_counts": {},
            "captain_instruction_count": 0,
            "captain_instruction_status": "unconfigured",
            "captain_instruction_source": "none",
            "captain_instruction_source_counts": {},
            "captain_instruction_source_summary": "none",
            "stale": false,
            "tolaria": tolaria_memory_status(&tolaria_mirror, "missing").unwrap_or_else(|| json!({
                "available": false,
                "enabled": false,
                "reason": "Tolaria memory mirror is not configured or not detected"
            })),
            "allowed_entry_kinds": ALLOWED_ENTRY_KINDS,
            "allowed_source_kinds": ALLOWED_SOURCE_KINDS,
            "forbidden_truth_sources": FORBIDDEN_TRUTH_SOURCES,
            "inference_entries_allowed": false,
            "max_entries": MEMORY_MAX_ENTRIES,
            "max_file_bytes": MEMORY_MAX_FILE_BYTES,
            "reason": "workspace memory is not configured; use action=preview before action=write"
        }),
        Err(error) => json!({
            "available": false,
            "configured": true,
            "enabled": false,
            "workspace": path_key(workspace),
            "path": store_path.to_string_lossy(),
            "entry_count": 0,
            "entry_counts": {},
            "entry_source_counts": {},
            "captain_instruction_count": 0,
            "captain_instruction_status": "unavailable",
            "captain_instruction_source": "unavailable",
            "captain_instruction_source_counts": {},
            "captain_instruction_source_summary": "unavailable",
            "stale": true,
            "reason": error.to_string(),
            "tolaria": tolaria_memory_status(&tolaria_mirror, "error").unwrap_or_else(|| json!({
                "available": false,
                "enabled": false,
                "reason": "Tolaria memory mirror is not configured or not detected"
            })),
            "allowed_entry_kinds": ALLOWED_ENTRY_KINDS,
            "allowed_source_kinds": ALLOWED_SOURCE_KINDS,
            "forbidden_truth_sources": FORBIDDEN_TRUTH_SOURCES,
            "inference_entries_allowed": false
        }),
    }
}

fn create_memory_entry_proposal(arguments: &Value, timestamp: u128) -> MemoryProposal {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let entries = arguments
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for entry in entries {
        match parse_memory_entry(&entry, timestamp) {
            Ok(parsed) => accepted.push(parsed),
            Err(reason) => rejected.push(json!({
                "entry": entry,
                "reason": reason
            })),
        }
    }

    MemoryProposal { accepted, rejected }
}

fn parse_memory_entry(value: &Value, timestamp: u128) -> Result<CccMemoryEntry, String> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "entry.kind is required".to_string())?
        .trim();
    if !ALLOWED_ENTRY_KINDS.contains(&kind) {
        return Err(format!(
            "entry.kind must be one of {}",
            ALLOWED_ENTRY_KINDS.join(", ")
        ));
    }

    let text = value
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "entry.text is required".to_string())?
        .trim();
    if text.is_empty() {
        return Err("entry.text must not be empty".to_string());
    }
    if text.chars().count() > MEMORY_MAX_TEXT_CHARS {
        return Err(format!(
            "entry.text must be at most {MEMORY_MAX_TEXT_CHARS} characters"
        ));
    }

    let source_kind = value
        .get("source_kind")
        .and_then(Value::as_str)
        .unwrap_or(default_source_kind_for_entry_kind(kind))
        .trim();
    let source_kind = normalize_memory_source_kind(source_kind);
    if source_kind_is_forbidden_truth_source(&source_kind) {
        return Err(format!(
            "{source_kind} is not an allowed memory truth source; verify from operator input, project files, or tests"
        ));
    }
    if !ALLOWED_SOURCE_KINDS.contains(&source_kind.as_str()) {
        return Err(format!(
            "entry.source_kind must be one of {}",
            ALLOWED_SOURCE_KINDS.join(", ")
        ));
    }

    let evidence_paths = value
        .get("evidence_paths")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .take(10)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if kind == "verified_project_fact" && evidence_paths.is_empty() {
        return Err("verified_project_fact entries require evidence_paths".to_string());
    }
    if kind == "repeated_rule" {
        let generally_useful = value
            .get("generally_useful")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !generally_useful
            || !matches!(source_kind.as_str(), "project_file" | "test_result")
            || evidence_paths.len() < 2
        {
            return Err(
                "repeated_rule entries require generally_useful=true, source_kind project_file or test_result, and at least two evidence_paths"
                    .to_string(),
            );
        }
    }

    // Inferences are intentionally rejected instead of stored at lower confidence. CCC memory is
    // only for durable operator preferences, repeated rules, and facts verified outside run state.
    let certainty = if kind == "verified_project_fact" {
        "verified"
    } else if kind == "repeated_rule" {
        "verified_repeated"
    } else {
        "operator_stated"
    };

    Ok(CccMemoryEntry {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("mem-{timestamp}-{}", stable_text_token(text))),
        kind: kind.to_string(),
        text: text.to_string(),
        source_kind,
        source: value
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("operator")
            .trim()
            .to_string(),
        certainty: certainty.to_string(),
        evidence_paths,
        created_at_unix_ms: timestamp,
        updated_at_unix_ms: timestamp,
    })
}

fn apply_entries_to_store(
    workspace: &Path,
    current_store: Option<CccMemoryStore>,
    proposal: &MemoryProposal,
) -> io::Result<CccMemoryStore> {
    let timestamp = now_unix_ms();
    let mut store = current_store.unwrap_or_else(|| empty_memory_store(workspace, timestamp));
    store.schema_version = MEMORY_SCHEMA_VERSION;
    store.enabled = true;
    store.workspace = path_key(workspace);

    for entry in &proposal.accepted {
        upsert_memory_entry(&mut store.entries, entry.clone());
    }
    if store.entries.len() > MEMORY_MAX_ENTRIES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("memory store is limited to {MEMORY_MAX_ENTRIES} entries"),
        ));
    }
    store.updated_at_unix_ms = timestamp;
    ensure_memory_file_size(&store)?;
    Ok(store)
}

fn upsert_memory_entry(entries: &mut Vec<CccMemoryEntry>, new_entry: CccMemoryEntry) {
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.kind == new_entry.kind && entry.text == new_entry.text)
    {
        let created_at = existing.created_at_unix_ms;
        *existing = new_entry;
        existing.created_at_unix_ms = created_at;
    } else {
        entries.push(new_entry);
    }
}

fn ensure_expected_memory_version(
    arguments: &Value,
    current_store: Option<&CccMemoryStore>,
) -> io::Result<()> {
    let expected = arguments.get("expected_updated_at_unix_ms");
    let current = current_store.map(|store| store.updated_at_unix_ms);
    let matches = match (expected, current) {
        (None, _) => false,
        (Some(Value::Null), None) => true,
        (Some(value), Some(current)) => value.as_u64().map(u128::from) == Some(current),
        _ => false,
    };
    if matches {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "stale memory write: rerun action=preview and use its expected_updated_at_unix_ms",
    ))
}

fn ensure_preview_token(arguments: &Value, expected_token: &str) -> io::Result<()> {
    let provided = arguments
        .get("preview_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty());
    if provided == Some(expected_token) {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "memory write requires matching preview_token from action=preview",
    ))
}

fn create_entries_preview_token(
    action: &str,
    expected_updated_at_unix_ms: Option<u128>,
    entries: &[CccMemoryEntry],
) -> io::Result<String> {
    let entries = entries
        .iter()
        .map(canonical_memory_entry_for_preview)
        .collect::<Vec<_>>();
    create_preview_token(&json!({
        "action": action,
        "expected_updated_at_unix_ms": expected_updated_at_unix_ms,
        "entries": entries
    }))
}

fn create_off_preview_token(
    current_store: Option<&CccMemoryStore>,
    expected_updated_at_unix_ms: Option<u128>,
) -> io::Result<String> {
    let current_entries = current_store
        .map(|store| {
            store
                .entries
                .iter()
                .map(canonical_memory_entry_for_preview)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    create_preview_token(&json!({
        "action": "off",
        "expected_updated_at_unix_ms": expected_updated_at_unix_ms,
        "current_enabled": current_store.map(|store| store.enabled),
        "current_entries": current_entries,
        "proposed_enabled": false
    }))
}

fn create_preview_token(value: &Value) -> io::Result<String> {
    let canonical = serde_json::to_vec(value).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode CCC memory preview token: {error}"),
        )
    })?;
    let digest = Sha256::digest(&canonical);
    Ok(format!("{MEMORY_PREVIEW_TOKEN_PREFIX}:{digest:x}"))
}

fn canonical_memory_entry_for_preview(entry: &CccMemoryEntry) -> Value {
    json!({
        "kind": entry.kind,
        "text": entry.text,
        "source_kind": entry.source_kind,
        "source": entry.source,
        "certainty": entry.certainty,
        "evidence_paths": entry.evidence_paths
    })
}

fn empty_memory_store(workspace: &Path, timestamp: u128) -> CccMemoryStore {
    CccMemoryStore {
        schema_version: MEMORY_SCHEMA_VERSION,
        enabled: true,
        workspace: path_key(workspace),
        updated_at_unix_ms: timestamp,
        entries: Vec::new(),
    }
}

fn load_memory_store(store_path: &Path) -> io::Result<Option<CccMemoryStore>> {
    if !store_path.exists() {
        return Ok(None);
    }
    let value = read_json_document(store_path)?;
    serde_json::from_value(value).map(Some).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid CCC memory store {}: {error}", store_path.display()),
        )
    })
}

fn load_memory_store_with_tolaria(
    workspace: &Path,
    store_path: &Path,
    arguments: &Value,
) -> io::Result<Option<CccMemoryStore>> {
    if let Some(store) = load_memory_store(store_path)? {
        return Ok(Some(store));
    }
    load_memory_store_from_tolaria(&resolve_tolaria_memory_mirror(arguments, workspace))
}

fn write_memory_store(store_path: &Path, store: &CccMemoryStore) -> io::Result<()> {
    ensure_memory_file_size(store)?;
    let content = serde_json::to_string_pretty(store).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode CCC memory store {}: {error}", store_path.display()),
        )
    })?;
    write_string_atomic(store_path, &content)
}

fn ensure_memory_file_size(store: &CccMemoryStore) -> io::Result<()> {
    let content = serde_json::to_vec_pretty(store).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode CCC memory store: {error}"),
        )
    })?;
    if content.len() > MEMORY_MAX_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("memory store is limited to {MEMORY_MAX_FILE_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn resolve_tolaria_memory_mirror(
    arguments: &Value,
    workspace: &Path,
) -> Option<TolariaMemoryMirror> {
    let explicit_enabled = arguments
        .get("tolaria_enabled")
        .and_then(Value::as_bool)
        .or_else(|| arguments.get("tolaria_sync").and_then(Value::as_bool));
    if explicit_enabled == Some(false) {
        return None;
    }

    let configured_vault = arguments
        .get("tolaria_vault_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CCC_TOLARIA_VAULT_PATH").map(PathBuf::from))
        .or_else(|| std::env::var_os("TOLARIA_VAULT_PATH").map(PathBuf::from))
        .or_else(|| std::env::var_os("VAULT_PATH").map(PathBuf::from))
        .or_else(shared_config_tolaria_vault_path)
        .or_else(|| {
            (explicit_enabled == Some(true))
                .then(default_tolaria_vault_path)
                .flatten()
        });

    let vault_path = configured_vault?;
    if !vault_path.is_dir() {
        return None;
    }
    if explicit_enabled != Some(true)
        && !tolaria_app_installed()
        && !vault_has_tolaria_marker(&vault_path)
    {
        return None;
    }

    let repo_folder_name = repo_memory_folder_name(workspace);
    let relative_note_path = format!("ccc/repos/{repo_folder_name}/memory.md");
    let repo_folder = vault_path.join("ccc").join("repos").join(repo_folder_name);
    Some(TolariaMemoryMirror {
        note_path: vault_path.join(&relative_note_path),
        repo_folder,
        vault_path,
        relative_note_path,
    })
}

fn shared_config_tolaria_vault_path() -> Option<PathBuf> {
    let (_, config) = read_optional_shared_config_document().ok()??;
    config
        .pointer("/integrations/tolaria/vault_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            config
                .pointer("/memory/tolaria/vault_path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
}

fn default_tolaria_vault_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let candidate = home.join("tolaria");
    candidate.is_dir().then_some(candidate)
}

fn tolaria_app_installed() -> bool {
    Path::new("/Applications/Tolaria.app/Contents/Resources/mcp-server/index.js").exists()
}

fn vault_has_tolaria_marker(vault_path: &Path) -> bool {
    vault_path.join("AGENTS.md").exists()
        || vault_path.join("type.md").exists()
        || vault_path.join("views").is_dir()
}

fn repo_memory_folder_name(workspace: &Path) -> String {
    let workspace_key = path_key(workspace);
    let repo_name = workspace
        .file_name()
        .and_then(|value| value.to_str())
        .map(slugify_tolaria_segment)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "repo".to_string());
    format!("{repo_name}-{}", stable_hex_hash(workspace_key.as_bytes()))
}

fn tolaria_memory_status(mirror: &Option<TolariaMemoryMirror>, state: &str) -> Option<Value> {
    let mirror = mirror.as_ref()?;
    Some(json!({
        "available": mirror.note_path.exists(),
        "enabled": true,
        "state": state,
        "vault_path": mirror.vault_path.to_string_lossy(),
        "repo_folder": mirror.repo_folder.to_string_lossy(),
        "note_path": mirror.note_path.to_string_lossy(),
        "relative_note_path": mirror.relative_note_path,
        "reason": if mirror.note_path.exists() { "Tolaria memory mirror note exists" } else { "Tolaria memory mirror note has not been written" },
    }))
}

fn sync_memory_store_to_tolaria(
    mirror: &Option<TolariaMemoryMirror>,
    store: &CccMemoryStore,
    local_store_path: &Path,
) -> Option<Value> {
    let mirror = mirror.as_ref()?;
    let result = write_tolaria_memory_note(mirror, store, local_store_path);
    Some(match result {
        Ok(()) => json!({
            "available": true,
            "enabled": true,
            "state": "synced",
            "vault_path": mirror.vault_path.to_string_lossy(),
            "repo_folder": mirror.repo_folder.to_string_lossy(),
            "note_path": mirror.note_path.to_string_lossy(),
            "relative_note_path": mirror.relative_note_path,
            "reason": "CCC memory store was mirrored into Tolaria",
        }),
        Err(error) => json!({
            "available": false,
            "enabled": true,
            "state": "sync_failed",
            "vault_path": mirror.vault_path.to_string_lossy(),
            "repo_folder": mirror.repo_folder.to_string_lossy(),
            "note_path": mirror.note_path.to_string_lossy(),
            "relative_note_path": mirror.relative_note_path,
            "reason": error.to_string(),
        }),
    })
}

fn write_tolaria_memory_note(
    mirror: &TolariaMemoryMirror,
    store: &CccMemoryStore,
    local_store_path: &Path,
) -> io::Result<()> {
    fs::create_dir_all(&mirror.repo_folder)?;
    let content = render_tolaria_memory_note(store, local_store_path)?;
    write_string_atomic(&mirror.note_path, &content)
}

fn render_tolaria_memory_note(
    store: &CccMemoryStore,
    local_store_path: &Path,
) -> io::Result<String> {
    let memory_json = serde_json::to_string_pretty(store).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode Tolaria memory mirror: {error}"),
        )
    })?;
    let repo_title = Path::new(&store.workspace)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&store.workspace);
    let entries = store
        .entries
        .iter()
        .map(|entry| {
            format!(
                "- **{}** `{}`: {}",
                entry.kind,
                entry.source_kind,
                entry.text.replace('\n', " ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
        r#"---
type: Note
related_to: "[[ccc-memory-index]]"
ccc_repo_root: "{}"
ccc_memory_schema_version: {}
ccc_memory_enabled: {}
ccc_memory_entry_count: {}
ccc_memory_updated_at_unix_ms: {}
ccc_memory_store_path: "{}"
---

# CCC Memory - {}

This note is managed by CCC. It mirrors repo-specific CCC memory so Tolaria can
search and surface durable operator preferences, repeated rules, and verified
project facts.

## Entries

{}

## Memory Store JSON

```json
{}
```
"#,
        escape_yaml_scalar(&store.workspace),
        store.schema_version,
        store.enabled,
        store.entries.len(),
        store.updated_at_unix_ms,
        escape_yaml_scalar(&local_store_path.to_string_lossy()),
        repo_title,
        if entries.is_empty() {
            "- No memory entries.".to_string()
        } else {
            entries
        },
        memory_json
    ))
}

fn load_memory_store_from_tolaria(
    mirror: &Option<TolariaMemoryMirror>,
) -> io::Result<Option<CccMemoryStore>> {
    let Some(mirror) = mirror else {
        return Ok(None);
    };
    if !mirror.note_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&mirror.note_path)?;
    let Some(json_text) = extract_tolaria_memory_json(&content) else {
        return Ok(None);
    };
    serde_json::from_str(json_text).map(Some).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid Tolaria memory mirror {}: {error}",
                mirror.note_path.display()
            ),
        )
    })
}

fn extract_tolaria_memory_json(content: &str) -> Option<&str> {
    let marker = "```json";
    let start = content.find(marker)?;
    let after_marker = &content[start + marker.len()..];
    let json_start = after_marker.find('\n')? + 1;
    let json_and_rest = &after_marker[json_start..];
    let end = json_and_rest.find("\n```")?;
    Some(&json_and_rest[..end])
}

fn slugify_tolaria_segment(value: &str) -> String {
    let mut slug = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.') {
            slug.push(ch);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn stable_hex_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn escape_yaml_scalar(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn create_memory_diff(
    current_store: Option<&CccMemoryStore>,
    proposed_store: &CccMemoryStore,
) -> io::Result<Value> {
    let current_value = match current_store {
        Some(store) => serde_json::to_value(store).map_err(json_encode_error)?,
        None => Value::Null,
    };
    let proposed_value = serde_json::to_value(proposed_store).map_err(json_encode_error)?;
    Ok(json!({
        "format": "json_line_summary",
        "before": summarize_memory_store_value(&current_value),
        "after": summarize_memory_store_value(&proposed_value),
        "changed": current_value != proposed_value
    }))
}

fn summarize_memory_store_value(value: &Value) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    json!({
        "enabled": value.get("enabled").cloned().unwrap_or(Value::Null),
        "entry_count": value.get("entries").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        "updated_at_unix_ms": value.get("updated_at_unix_ms").cloned().unwrap_or(Value::Null)
    })
}

fn entry_counts_by_kind(store: &CccMemoryStore) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in &store.entries {
        *counts.entry(entry.kind.clone()).or_insert(0) += 1;
    }
    counts
}

fn entry_counts_by_source_kind(store: &CccMemoryStore) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in &store.entries {
        *counts.entry(entry.source_kind.clone()).or_insert(0) += 1;
    }
    counts
}

fn captain_instruction_counts_by_source_kind(store: &CccMemoryStore) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in &store.entries {
        if entry.kind == "captain_instruction" {
            *counts.entry(entry.source_kind.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn compact_count_summary(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .iter()
        .map(|(key, count)| format!("{key}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn create_memory_text(payload: &Value) -> String {
    if let Some(action) = payload.get("action").and_then(Value::as_str) {
        match action {
            "preview" => return create_memory_preview_text(payload),
            "write" => return create_memory_write_text(payload),
            "off" => return create_memory_off_text(payload),
            _ => {}
        }
    }

    let memory = payload.get("memory").unwrap_or(payload);
    create_memory_status_text(memory)
}

fn create_memory_status_text(memory: &Value) -> String {
    let enabled = memory
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let configured = memory
        .get("configured")
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
    format!(
        "Memory: {mode} entries={entry_count} captain_instructions={captain_instruction_count} captain_instruction_status={captain_instruction_status} captain_instruction_source={captain_instruction_source_summary} stale={stale} path={path}"
    )
}

fn create_memory_preview_text(payload: &Value) -> String {
    let accepted_entries = payload
        .get("accepted_entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rejected_entries = payload
        .get("rejected_entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let preview_token = payload
        .pointer("/next_write/preview_token")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let expected_updated_at = payload
        .pointer("/next_write/expected_updated_at_unix_ms")
        .map(compact_memory_value)
        .unwrap_or_else(|| "null".to_string());
    format!(
        "Memory: preview written=false accepted={} rejected={} diff={} preview_token={} expected_updated_at_unix_ms={} accepted_entries=[{}] rejected_entries=[{}] {}",
        accepted_entries.len(),
        rejected_entries.len(),
        compact_memory_diff(payload.get("diff")),
        preview_token,
        expected_updated_at,
        compact_memory_entries(&accepted_entries),
        compact_memory_rejections(&rejected_entries),
        create_memory_status_text(payload.get("memory").unwrap_or(payload))
    )
}

fn create_memory_write_text(payload: &Value) -> String {
    let accepted_entries = payload
        .get("accepted_entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rejected_entries = payload
        .get("rejected_entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    format!(
        "Memory: write written=true accepted={} rejected={} diff={} accepted_entries=[{}] rejected_entries=[{}] {}",
        accepted_entries.len(),
        rejected_entries.len(),
        compact_memory_diff(payload.get("diff")),
        compact_memory_entries(&accepted_entries),
        compact_memory_rejections(&rejected_entries),
        create_memory_status_text(payload.get("memory").unwrap_or(payload))
    )
}

fn create_memory_off_text(payload: &Value) -> String {
    let written = payload
        .get("written")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let preview_token = payload
        .pointer("/next_write/preview_token")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let expected_updated_at = payload
        .pointer("/next_write/expected_updated_at_unix_ms")
        .map(compact_memory_value)
        .unwrap_or_else(|| "null".to_string());
    format!(
        "Memory: off written={} diff={} preview_token={} expected_updated_at_unix_ms={} {}",
        written,
        compact_memory_diff(payload.get("diff")),
        preview_token,
        expected_updated_at,
        create_memory_status_text(payload.get("memory").unwrap_or(payload))
    )
}

fn compact_memory_entries(entries: &[Value]) -> String {
    compact_memory_items(entries, |entry| {
        let kind = entry
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let text = entry
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("missing text");
        format!("{kind}: {}", compact_memory_text(text))
    })
}

fn compact_memory_rejections(entries: &[Value]) -> String {
    compact_memory_items(entries, |entry| {
        let rejected_entry = entry.get("entry").unwrap_or(entry);
        let kind = rejected_entry
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let text = rejected_entry
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("missing text");
        let reason = entry
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("missing reason");
        format!(
            "{kind}: {} reason={}",
            compact_memory_text(text),
            compact_memory_text(reason)
        )
    })
}

fn compact_memory_items<F>(entries: &[Value], render: F) -> String
where
    F: Fn(&Value) -> String,
{
    let mut items = entries.iter().take(3).map(render).collect::<Vec<_>>();
    if entries.len() > items.len() {
        items.push(format!("+{} more", entries.len() - items.len()));
    }
    items.join("; ")
}

fn compact_memory_diff(diff: Option<&Value>) -> String {
    let Some(diff) = diff else {
        return "unavailable".to_string();
    };
    let before = diff.get("before").unwrap_or(&Value::Null);
    let after = diff.get("after").unwrap_or(&Value::Null);
    format!(
        "before={} after={}",
        compact_memory_value(before),
        compact_memory_value(after)
    )
}

fn compact_memory_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => compact_memory_text(value),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "unavailable".to_string()),
    }
}

fn compact_memory_text(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut compact = normalized.chars().take(80).collect::<String>();
    if normalized.chars().count() > compact.chars().count() {
        compact.push_str("...");
    }
    compact
}

fn resolve_memory_workspace(arguments: &Value) -> io::Result<PathBuf> {
    let cwd = arguments
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    Ok(normalize_workspace_path(&cwd))
}

fn normalize_workspace_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn default_source_kind_for_entry_kind(kind: &str) -> &'static str {
    if kind == "verified_project_fact" {
        "project_file"
    } else {
        "operator_confirmation"
    }
}

fn normalize_memory_source_kind(source_kind: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;
    for character in source_kind.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if character == '_' || character == '-' || character.is_whitespace() {
            if !normalized.is_empty() && !previous_was_separator {
                normalized.push('_');
                previous_was_separator = true;
            }
        }
    }
    normalized.trim_matches('_').to_string()
}

fn source_kind_is_forbidden_truth_source(source_kind: &str) -> bool {
    let compact_source_kind = source_kind.replace('_', "");
    FORBIDDEN_TRUTH_SOURCES.contains(&source_kind)
        || FORBIDDEN_TRUTH_SOURCES
            .iter()
            .any(|forbidden| compact_source_kind == forbidden.replace('_', ""))
        || source_kind.starts_with("inference_")
        || compact_source_kind.starts_with("inference")
}

fn stable_text_token(text: &str) -> String {
    let mut acc: u64 = 1469598103934665603;
    for byte in text.as_bytes() {
        acc ^= u64::from(*byte);
        acc = acc.wrapping_mul(1099511628211);
    }
    format!("{acc:016x}")
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn json_encode_error(error: serde_json::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("encode CCC memory diff: {error}"),
    )
}
