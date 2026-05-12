use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const GRAPHIFY_PROVIDER: &str = "graphify";
const READ_ONLY_MODE: &str = "read_only";
const SCOUT_SOURCE_EVIDENCE_FALLBACK: &str = "scout_source_evidence";
const LEGACY_CODE_GRAPH_BACKEND: &str = "legacy_code_graph";
const GRAPHIFY_READ_ONLY_BACKEND: &str = "graphify_read_only_artifacts";
const GRAPH_CONTEXT_SCOUT_BACKEND: &str = "graph_context_scout_source_evidence";
const DEFAULT_REPORT_PATH: &str = "graphify-out/GRAPH_REPORT.md";
const DEFAULT_GRAPH_PATH: &str = "graphify-out/graph.json";
const DEFAULT_MAX_REPORT_BYTES: u64 = 20_000;

pub(crate) fn create_graph_context_code_graph_payload_for_config_path(
    arguments: &Value,
    config_path: &Path,
) -> io::Result<Option<Value>> {
    let config = crate::read_optional_toml_document(config_path)?.unwrap_or(Value::Null);
    create_graph_context_code_graph_payload(arguments, &config)
}

pub(crate) fn create_graph_context_mcp_code_graph_payload_for_config_path(
    arguments: &Value,
    config_path: &Path,
) -> io::Result<Option<Value>> {
    let config = crate::read_optional_toml_document(config_path)?.unwrap_or(Value::Null);
    if !graph_context_explicitly_enabled(&config) {
        return Ok(None);
    }
    create_graph_context_code_graph_payload_with_query_flag(
        arguments,
        &config,
        graph_context_mcp_query_allowed(&config),
    )
}

pub(crate) fn create_graph_context_code_graph_payload(
    arguments: &Value,
    config: &Value,
) -> io::Result<Option<Value>> {
    create_graph_context_code_graph_payload_with_query_flag(arguments, config, true)
}

fn create_graph_context_code_graph_payload_with_query_flag(
    arguments: &Value,
    config: &Value,
    graphify_queries_enabled: bool,
) -> io::Result<Option<Value>> {
    if !graph_context_explicitly_enabled(config) {
        return Ok(None);
    }

    let cwd = arguments
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let query_name = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("review_context");
    let update_requested = arguments
        .get("update")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let query_paths = arguments
        .get("paths")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|path| Value::String(path.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let readiness = create_graph_context_readiness_payload(config, &cwd)?;
    let readiness_state = readiness
        .get("readiness")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let reason = readiness
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let evidence_paths = readiness
        .get("evidence_paths")
        .cloned()
        .unwrap_or_else(|| json!([]));

    if readiness_state == "available" {
        let report = graph_context_bounded_report(config, &readiness)?;
        return Ok(Some(json!({
            "schema": "ccc.graph_context_code_graph.v1",
            "provider": GRAPHIFY_PROVIDER,
            "backend": GRAPHIFY_READ_ONLY_BACKEND,
            "repo_root": cwd.to_string_lossy(),
            "query": query_name,
            "query_paths": query_paths,
            "updated": false,
            "update_requested": update_requested,
            "readiness": readiness_state,
            "reason": reason,
            "fallback": Value::Null,
            "evidence_paths": evidence_paths,
            "recommended_action": "Use the bounded Graphify report and graph metadata for graph context; verify source files directly before mutation.",
            "routing": graph_context_routing_metadata(
                GRAPHIFY_READ_ONLY_BACKEND,
                update_requested,
                graphify_queries_enabled,
            ),
            "artifacts": readiness.get("artifacts").cloned().unwrap_or_else(|| json!({})),
            "report": report,
            "graph_metadata": graph_context_graph_metadata(&readiness),
            "query_result": {
                "graph_context": {
                    "readiness": readiness_state,
                    "reason": reason,
                    "source": "graphify_existing_artifacts"
                }
            }
        })));
    }

    Ok(Some(json!({
        "schema": "ccc.graph_context_code_graph.v1",
        "provider": GRAPHIFY_PROVIDER,
        "backend": GRAPH_CONTEXT_SCOUT_BACKEND,
        "repo_root": cwd.to_string_lossy(),
        "query": query_name,
        "query_paths": query_paths,
        "updated": false,
        "update_requested": update_requested,
        "readiness": readiness_state,
        "reason": reason,
        "fallback": SCOUT_SOURCE_EVIDENCE_FALLBACK,
        "evidence_paths": evidence_paths,
        "recommended_action": "Graphify graph_context is not ready; gather source evidence directly with scout_source_evidence and do not rebuild or read the legacy code graph store.",
        "routing": graph_context_routing_metadata(
            GRAPH_CONTEXT_SCOUT_BACKEND,
            update_requested,
            graphify_queries_enabled,
        ),
        "artifacts": readiness.get("artifacts").cloned().unwrap_or_else(|| json!({})),
        "missing_artifacts": readiness.get("missing_artifacts").cloned().unwrap_or_else(|| json!([])),
        "semantic_mismatches": readiness.get("semantic_mismatches").cloned().unwrap_or_else(|| json!([])),
        "stale": readiness.get("stale").cloned().unwrap_or_else(|| json!({})),
        "query_result": {
            "graph_context": {
                "readiness": readiness_state,
                "reason": reason,
                "source": SCOUT_SOURCE_EVIDENCE_FALLBACK
            }
        }
    })))
}

pub(crate) fn create_graph_context_code_graph_text(payload: &Value) -> String {
    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let readiness = payload
        .get("readiness")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let reason = payload
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let backend = payload
        .get("backend")
        .and_then(Value::as_str)
        .unwrap_or("graph_context");

    if readiness == "available" {
        let report_bytes = payload
            .pointer("/report/content_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        return format!(
            "Graph: Graphify graph_context ready query={query} backend={backend}; bounded report bytes={report_bytes}; legacy code graph fallback disabled."
        );
    }

    let fallback = payload
        .get("fallback")
        .and_then(Value::as_str)
        .unwrap_or(SCOUT_SOURCE_EVIDENCE_FALLBACK);
    format!(
        "Graph: Graphify graph_context readiness={readiness} reason={reason}; fallback={fallback}; legacy code graph fallback disabled."
    )
}

fn graph_context_explicitly_enabled(config: &Value) -> bool {
    config
        .pointer("/features/graph_context")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && config
            .pointer("/graph_context/enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn graph_context_mcp_query_allowed(config: &Value) -> bool {
    graph_context_explicitly_enabled(config)
        && config
            .pointer("/graph_context/allow_mcp_query")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn graph_context_routing_metadata(
    backend: &str,
    update_requested: bool,
    graphify_queries_enabled: bool,
) -> Value {
    json!({
        "graph_context_enabled": true,
        "graphify_queries_enabled": graphify_queries_enabled,
        "legacy_code_graph_called": false,
        "legacy_fallback_disabled": true,
        "legacy_rebuild_disabled": true,
        "update_requested": update_requested,
        "update_ignored": update_requested,
        "ccc_graph_backend": backend,
        "ccc_code_graph_backend": backend
    })
}

fn graph_context_readiness_fallback(readiness: &str, fallback_when_unavailable: &str) -> Value {
    match readiness {
        "available" => Value::Null,
        "disabled" => Value::String(LEGACY_CODE_GRAPH_BACKEND.to_string()),
        _ => Value::String(fallback_when_unavailable.to_string()),
    }
}

fn graph_context_readiness_routing(readiness: &str) -> Value {
    let (graphify_queries_enabled, backend, legacy_fallback_disabled) = match readiness {
        "available" => (true, GRAPHIFY_READ_ONLY_BACKEND, true),
        "disabled" => (false, LEGACY_CODE_GRAPH_BACKEND, false),
        _ => (true, GRAPH_CONTEXT_SCOUT_BACKEND, true),
    };
    json!({
        "graphify_queries_enabled": graphify_queries_enabled,
        "legacy_fallback_disabled": legacy_fallback_disabled,
        "ccc_graph_backend": backend,
        "ccc_code_graph_backend": backend
    })
}

fn graph_context_bounded_report(config: &Value, readiness: &Value) -> io::Result<Value> {
    let max_report_bytes = config
        .pointer("/graph_context/max_report_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_MAX_REPORT_BYTES);
    let Some(report_path) = readiness
        .pointer("/artifacts/report/resolved_path")
        .and_then(Value::as_str)
    else {
        return Ok(json!({
            "available": false,
            "content": "",
            "content_bytes": 0,
            "max_report_bytes": max_report_bytes,
            "truncated": false
        }));
    };

    let (content, content_bytes, truncated) =
        read_bounded_text_file(Path::new(report_path), max_report_bytes)?;
    Ok(json!({
        "available": true,
        "path": readiness.pointer("/artifacts/report/path").cloned().unwrap_or(Value::Null),
        "resolved_path": report_path,
        "bytes": readiness.pointer("/artifacts/report/bytes").cloned().unwrap_or(Value::Null),
        "modified_unix_ms": readiness.pointer("/artifacts/report/modified_unix_ms").cloned().unwrap_or(Value::Null),
        "content": content,
        "content_bytes": content_bytes,
        "max_report_bytes": max_report_bytes,
        "truncated": truncated
    }))
}

fn graph_context_graph_metadata(readiness: &Value) -> Value {
    let graph = readiness
        .pointer("/artifacts/graph")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "artifact": graph,
        "content_loaded": false,
        "content_policy": "metadata_only"
    })
}

fn read_bounded_text_file(path: &Path, max_bytes: u64) -> io::Result<(String, u64, bool)> {
    let mut file = File::open(path)?;
    let read_limit = max_bytes.saturating_add(1);
    let mut buffer = Vec::new();
    file.by_ref().take(read_limit).read_to_end(&mut buffer)?;
    let truncated = buffer.len() as u64 > max_bytes;
    if truncated {
        buffer.truncate(max_bytes as usize);
    }
    let content_bytes = buffer.len() as u64;
    Ok((
        String::from_utf8_lossy(&buffer).into_owned(),
        content_bytes,
        truncated,
    ))
}

pub(crate) fn create_graph_context_readiness_payload(
    config: &Value,
    workspace_root: &Path,
) -> io::Result<Value> {
    let graph_context = config.get("graph_context").unwrap_or(&Value::Null);
    let feature_enabled = config
        .pointer("/features/graph_context")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let config_enabled = bool_field(graph_context, "enabled", false);
    let provider = string_field(graph_context, "provider", GRAPHIFY_PROVIDER);
    let mode = string_field(graph_context, "mode", READ_ONLY_MODE);
    let canonical_backend = string_field(graph_context, "canonical_backend", GRAPHIFY_PROVIDER);
    let allow_legacy_graph_backend_fallback =
        bool_field(graph_context, "allow_legacy_graph_backend_fallback", false);
    let fallback_when_unavailable = string_field(
        graph_context,
        "fallback_when_unavailable",
        SCOUT_SOURCE_EVIDENCE_FALLBACK,
    );
    let source_of_truth = bool_field(graph_context, "source_of_truth", false);
    let report_path = string_field(graph_context, "report_path", DEFAULT_REPORT_PATH);
    let graph_path = string_field(graph_context, "graph_path", DEFAULT_GRAPH_PATH);

    let report = artifact_metadata(workspace_root, "report", &report_path)?;
    let graph = artifact_metadata(workspace_root, "graph", &graph_path)?;
    let artifacts = [&report, &graph];
    let missing_artifacts = artifacts
        .iter()
        .filter(|artifact| !artifact.is_available())
        .map(|artifact| artifact.kind)
        .collect::<Vec<_>>();
    let evidence_paths = artifacts
        .iter()
        .map(|artifact| artifact.resolved_path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    let mut semantic_mismatches = Vec::new();
    if provider != GRAPHIFY_PROVIDER {
        semantic_mismatches.push("provider");
    }
    if mode != READ_ONLY_MODE {
        semantic_mismatches.push("mode");
    }
    if canonical_backend != GRAPHIFY_PROVIDER {
        semantic_mismatches.push("canonical_backend");
    }
    if allow_legacy_graph_backend_fallback {
        semantic_mismatches.push("allow_legacy_graph_backend_fallback");
    }
    if fallback_when_unavailable != SCOUT_SOURCE_EVIDENCE_FALLBACK {
        semantic_mismatches.push("fallback_when_unavailable");
    }
    if source_of_truth {
        semantic_mismatches.push("source_of_truth");
    }

    let disabled_reason = if !feature_enabled && !config_enabled {
        Some("graph_context_default_off")
    } else if !feature_enabled {
        Some("feature_disabled")
    } else if !config_enabled {
        Some("provider_disabled")
    } else {
        None
    };
    let provider_enabled = disabled_reason.is_none() && semantic_mismatches.is_empty();
    let stale = if provider_enabled && missing_artifacts.is_empty() {
        stale_artifact_payload(workspace_root, &artifacts)?
    } else {
        json!({
            "is_stale": false,
            "basis": "not_evaluated"
        })
    };

    let (readiness, reason) = if let Some(reason) = disabled_reason {
        ("disabled", reason)
    } else if !semantic_mismatches.is_empty() {
        ("unavailable", "graph_context_semantics_mismatch")
    } else if !missing_artifacts.is_empty() {
        ("unavailable", "missing_artifacts")
    } else if stale["is_stale"].as_bool().unwrap_or(false) {
        ("stale", "stale_artifacts")
    } else {
        ("available", "artifacts_available")
    };

    Ok(json!({
        "provider": provider,
        "mode": mode,
        "canonical_backend": canonical_backend,
        "feature_enabled": feature_enabled,
        "enabled": config_enabled,
        "provider_enabled": provider_enabled,
        "readiness": readiness,
        "reason": reason,
        "fallback_when_unavailable": fallback_when_unavailable,
        "fallback": graph_context_readiness_fallback(readiness, &fallback_when_unavailable),
        "allow_legacy_graph_backend_fallback": allow_legacy_graph_backend_fallback,
        "source_of_truth": source_of_truth,
        "report_path": report_path,
        "graph_path": graph_path,
        "artifacts": {
            "report": report.to_value(),
            "graph": graph.to_value()
        },
        "missing_artifacts": missing_artifacts,
        "semantic_mismatches": semantic_mismatches,
        "stale": stale,
        "evidence_paths": evidence_paths,
        "routing": graph_context_readiness_routing(readiness)
    }))
}

fn bool_field(config: &Value, key: &str, default: bool) -> bool {
    config.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn string_field(config: &Value, key: &str, default: &str) -> String {
    config
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

struct GraphContextArtifact {
    kind: &'static str,
    configured_path: String,
    resolved_path: PathBuf,
    exists: bool,
    is_file: bool,
    byte_len: Option<u64>,
    modified_unix_ms: Option<u128>,
}

impl GraphContextArtifact {
    fn is_available(&self) -> bool {
        self.exists && self.is_file
    }

    fn to_value(&self) -> Value {
        json!({
            "path": self.configured_path,
            "resolved_path": self.resolved_path.to_string_lossy(),
            "exists": self.exists,
            "is_file": self.is_file,
            "bytes": self.byte_len,
            "modified_unix_ms": self.modified_unix_ms,
            "available": self.is_available()
        })
    }
}

fn artifact_metadata(
    workspace_root: &Path,
    kind: &'static str,
    configured_path: &str,
) -> io::Result<GraphContextArtifact> {
    let resolved_path = resolve_configured_path(workspace_root, configured_path);
    match fs::metadata(&resolved_path) {
        Ok(metadata) => Ok(GraphContextArtifact {
            kind,
            configured_path: configured_path.to_string(),
            resolved_path,
            exists: true,
            is_file: metadata.is_file(),
            byte_len: metadata.is_file().then_some(metadata.len()),
            modified_unix_ms: metadata.modified().ok().and_then(system_time_to_unix_ms),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(GraphContextArtifact {
            kind,
            configured_path: configured_path.to_string(),
            resolved_path,
            exists: false,
            is_file: false,
            byte_len: None,
            modified_unix_ms: None,
        }),
        Err(error) => Err(error),
    }
}

fn resolve_configured_path(workspace_root: &Path, configured_path: &str) -> PathBuf {
    let path = PathBuf::from(configured_path);
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

fn stale_artifact_payload(
    workspace_root: &Path,
    artifacts: &[&GraphContextArtifact],
) -> io::Result<Value> {
    let latest_source = latest_workspace_source(workspace_root)?;
    let Some((source_path, source_modified_unix_ms)) = latest_source else {
        return Ok(json!({
            "is_stale": false,
            "basis": "no_workspace_source_metadata"
        }));
    };
    let oldest_artifact_modified_unix_ms = artifacts
        .iter()
        .filter_map(|artifact| artifact.modified_unix_ms)
        .min();
    let is_stale = oldest_artifact_modified_unix_ms
        .map(|artifact_modified| source_modified_unix_ms > artifact_modified)
        .unwrap_or(false);

    Ok(json!({
        "is_stale": is_stale,
        "basis": "workspace_source_newer_than_artifact",
        "latest_source_path": source_path.to_string_lossy(),
        "latest_source_modified_unix_ms": source_modified_unix_ms,
        "oldest_artifact_modified_unix_ms": oldest_artifact_modified_unix_ms
    }))
}

fn latest_workspace_source(workspace_root: &Path) -> io::Result<Option<(PathBuf, u128)>> {
    let mut latest = None;
    collect_latest_workspace_source(workspace_root, &mut latest)?;
    Ok(latest)
}

fn collect_latest_workspace_source(
    directory: &Path,
    latest: &mut Option<(PathBuf, u128)>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if should_skip_source_directory(&path) {
                continue;
            }
            collect_latest_workspace_source(&path, latest)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(modified_unix_ms) = entry
            .metadata()?
            .modified()
            .ok()
            .and_then(system_time_to_unix_ms)
        else {
            continue;
        };
        if latest
            .as_ref()
            .map(|(_, current)| modified_unix_ms > *current)
            .unwrap_or(true)
        {
            *latest = Some((path, modified_unix_ms));
        }
    }
    Ok(())
}

fn should_skip_source_directory(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some(".git" | ".ccc" | "target" | "node_modules" | "graphify-out")
    )
}

fn system_time_to_unix_ms(value: SystemTime) -> Option<u128> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}
