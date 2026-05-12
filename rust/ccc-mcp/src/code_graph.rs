use crate::{read_json_document, read_optional_shared_config_document, write_json_document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const CODE_GRAPH_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct CodeGraphStore {
    pub(crate) schema_version: u32,
    pub(crate) repo_root: String,
    pub(crate) indexed_at_unix_ms: u128,
    pub(crate) files: BTreeMap<String, CodeGraphFile>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct CodeGraphFile {
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) size_bytes: u64,
    pub(crate) modified_unix_ms: u128,
    pub(crate) imports: Vec<CodeGraphImport>,
    pub(crate) symbols: Vec<CodeGraphSymbol>,
    pub(crate) identifier_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct CodeGraphImport {
    pub(crate) target: String,
    pub(crate) raw: String,
    pub(crate) line: usize,
    pub(crate) kind: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct CodeGraphSymbol {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphSummary {
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) imports: Vec<String>,
    pub(crate) symbols: Vec<String>,
    pub(crate) symbol_count: usize,
    pub(crate) reference_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphRelation {
    pub(crate) path: String,
    pub(crate) reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphBlastRadius {
    pub(crate) changed_files: Vec<String>,
    pub(crate) impacted_files: Vec<CodeGraphRelation>,
    pub(crate) related_tests: Vec<CodeGraphRelation>,
    pub(crate) risk_score: u8,
    pub(crate) risk_level: String,
    pub(crate) risk_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphReviewContext {
    pub(crate) summaries: Vec<CodeGraphSummary>,
    pub(crate) callers: Vec<CodeGraphRelation>,
    pub(crate) callees: Vec<CodeGraphRelation>,
    pub(crate) blast_radius: CodeGraphBlastRadius,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphFlowEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) depth: usize,
    pub(crate) reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphFlowTrace {
    pub(crate) roots: Vec<String>,
    pub(crate) direction: String,
    pub(crate) max_depth: usize,
    pub(crate) edges: Vec<CodeGraphFlowEdge>,
    pub(crate) truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphCriticality {
    pub(crate) path: String,
    pub(crate) score: u8,
    pub(crate) level: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) caller_count: usize,
    pub(crate) callee_count: usize,
    pub(crate) related_test_count: usize,
    pub(crate) symbol_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeGraphSearchMatch {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) score: u8,
    pub(crate) match_type: String,
    pub(crate) snippet: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CodeGraphTarget {
    repo_root: PathBuf,
    store_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TolariaGraphMirror {
    vault_path: PathBuf,
    repo_folder: PathBuf,
    note_path: PathBuf,
    relative_note_path: String,
}

pub(crate) fn detect_repo_root(start: &Path) -> io::Result<PathBuf> {
    let start_path = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    let mut cursor = fs::canonicalize(start_path).unwrap_or_else(|_| start_path.to_path_buf());

    loop {
        if is_repo_marker_directory(&cursor) {
            return Ok(cursor);
        }
        if !cursor.pop() {
            return Ok(fs::canonicalize(start_path).unwrap_or_else(|_| start_path.to_path_buf()));
        }
    }
}

pub(crate) fn default_graph_store_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".ccc").join("graph").join("store.json")
}

pub(crate) fn load_code_graph_store(store_path: &Path) -> io::Result<Option<CodeGraphStore>> {
    if !store_path.exists() {
        return Ok(None);
    }
    let value = read_json_document(store_path)?;
    serde_json::from_value(value).map(Some).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid code graph store {}: {error}", store_path.display()),
        )
    })
}

pub(crate) fn write_code_graph_store(store_path: &Path, store: &CodeGraphStore) -> io::Result<()> {
    let value = serde_json::to_value(store).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode code graph store {}: {error}", store_path.display()),
        )
    })?;
    write_json_document(store_path, &value)
}

#[cfg(test)]
pub(crate) fn update_code_graph_store_for_repo(start: &Path) -> io::Result<CodeGraphStore> {
    let repo_root = detect_repo_root(start)?;
    let store_path = default_graph_store_path(&repo_root);
    update_code_graph_store_at(&repo_root, &store_path)
}

pub(crate) fn create_code_graph_payload(arguments: &Value) -> io::Result<Value> {
    let cwd = arguments
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let query_name = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("review_context");
    let update = arguments
        .get("update")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let paths = arguments
        .get("paths")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let direction = arguments
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("both");
    let max_depth = arguments
        .get("max_depth")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 6) as usize)
        .unwrap_or(2);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 200) as usize)
        .unwrap_or(50);

    let explicit_store_path = arguments
        .get("store_path")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let allow_document_root = update || tolaria_explicitly_enabled(arguments);
    let target = resolve_code_graph_target(&cwd, &paths, explicit_store_path, allow_document_root)?;
    let repo_root = target.repo_root;
    let store_path = target.store_path;
    let paths = normalize_query_paths_for_target(&cwd, &repo_root, &paths);
    let tolaria_mirror = resolve_tolaria_graph_mirror(arguments, &repo_root);
    let (store, tolaria_status) = if update {
        let store = update_code_graph_store_at(&repo_root, &store_path)?;
        let status = sync_code_graph_store_to_tolaria(&tolaria_mirror, &store, &store_path);
        (store, status)
    } else {
        match load_code_graph_store(&store_path)? {
            Some(store) => {
                let status = tolaria_mirror_status(&tolaria_mirror, "local_store_loaded");
                (store, status)
            }
            None => match load_code_graph_store_from_tolaria(&tolaria_mirror) {
                Ok(Some((store, status))) => (store, Some(status)),
                Ok(None) => {
                    let mut message =
                        format!("code graph store not found: {}", store_path.display());
                    if let Some(status) = tolaria_mirror_status(&tolaria_mirror, "missing") {
                        if let Some(reason) = status.get("reason").and_then(Value::as_str) {
                            message
                                .push_str(&format!("; Tolaria graph mirror unavailable: {reason}"));
                        }
                    }
                    return Err(io::Error::new(io::ErrorKind::NotFound, message));
                }
                Err(error) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "code graph store not found: {}; Tolaria graph mirror could not be loaded: {error}",
                            store_path.display()
                        ),
                    ));
                }
            },
        }
    };

    let query = CodeGraphQuery::new(&store);
    let query_payload = match query_name {
        "file_summary" => json!({
            "file_summaries": paths
                .iter()
                .filter_map(|path| query.file_summary(path))
                .map(summary_to_value)
                .collect::<Vec<_>>()
        }),
        "imports" => json!({
            "file_summaries": paths
                .iter()
                .filter_map(|path| query.file_summary(path))
                .map(|summary| json!({"path": summary.path, "imports": summary.imports}))
                .collect::<Vec<_>>()
        }),
        "callers" => json!({
            "callers": paths
                .iter()
                .flat_map(|path| query.callers_for_file(path))
                .map(relation_to_value)
                .collect::<Vec<_>>()
        }),
        "callees" => json!({
            "callees": paths
                .iter()
                .flat_map(|path| query.callees_for_file(path))
                .map(relation_to_value)
                .collect::<Vec<_>>()
        }),
        "tests" => json!({
            "related_tests": paths
                .iter()
                .flat_map(|path| query.related_tests_for_file(path))
                .map(relation_to_value)
                .collect::<Vec<_>>()
        }),
        "impact" | "blast_radius" => {
            json!({"blast_radius": blast_radius_to_value(query.blast_radius_for_changed_paths(&paths))})
        }
        "review_context" => {
            json!({"review_context": review_context_to_value(query.minimal_review_context(&paths))})
        }
        "flow_trace" | "flows" => json!({
            "flow_trace": flow_trace_to_value(query.flow_trace_for_paths(&paths, direction, max_depth))
        }),
        "criticality" | "criticality_scores" => json!({
            "criticality_scores": query
                .criticality_scores(&paths)
                .into_iter()
                .take(limit)
                .map(criticality_to_value)
                .collect::<Vec<_>>()
        }),
        "communities" | "architecture_overview" => {
            json!({"architecture_overview": architecture_overview_to_value(&query, limit)})
        }
        "full_text_search" | "search" => {
            let text = query_text(arguments)?;
            json!({"search": full_text_search_to_value(full_text_search(&store, &repo_root, &text, limit))})
        }
        "multi_repo_search" => {
            let text = query_text(arguments)?;
            json!({"multi_repo_search": multi_repo_search(arguments, &text, limit)?})
        }
        unknown => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported code graph query: {unknown}"),
            ));
        }
    };

    let query_path_values = paths
        .iter()
        .map(|path| Value::String(path_key(path)))
        .collect::<Vec<_>>();
    let evidence_note =
        create_code_graph_query_evidence_note(query_name, &query_payload, &query_path_values);

    Ok(json!({
        "schema_version": store.schema_version,
        "repo_root": store.repo_root,
        "store_path": store_path.to_string_lossy(),
        "indexed_at_unix_ms": store.indexed_at_unix_ms,
        "file_count": store.files.len(),
        "query": query_name,
        "query_paths": query_path_values,
        "updated": update,
        "tolaria": tolaria_status.unwrap_or_else(|| json!({
            "available": false,
            "enabled": false,
            "reason": "Tolaria graph mirror is not configured or not detected"
        })),
        "evidence_note": evidence_note,
        "query_result": query_payload
    }))
}

pub(crate) fn create_code_graph_status_payload(cwd: &Path) -> Value {
    match resolve_code_graph_status_target(cwd) {
        Ok((target, resolution)) => code_graph_status_for_target(&target, resolution.as_deref()),
        Err(error) => json!({
            "available": false,
            "reason": error.to_string(),
            "diagnostic_severity": "warning",
            "blocking": false,
            "recommended_action": "Pass cwd as the target repo path, run ccc graph with update=true, or ignore this warning for smoke runs that do not require graph context."
        }),
    }
}

fn code_graph_status_for_target(target: &CodeGraphTarget, resolution: Option<&str>) -> Value {
    let tolaria_mirror = resolve_tolaria_graph_mirror(&Value::Null, &target.repo_root);
    match load_code_graph_store(&target.store_path) {
        Ok(Some(store)) => {
            let mut payload = json!({
                "available": true,
                "repo_root": store.repo_root,
                "store_path": target.store_path.to_string_lossy(),
                "schema_version": store.schema_version,
                "indexed_at_unix_ms": store.indexed_at_unix_ms,
                "file_count": store.files.len(),
                "tolaria": tolaria_mirror_status(&tolaria_mirror, "local_store_loaded").unwrap_or_else(|| json!({
                    "available": false,
                    "enabled": false,
                    "reason": "Tolaria graph mirror is not configured or not detected"
                })),
                "evidence_note": create_code_graph_status_evidence_note(&store)
            });
            if let Some(value) = resolution {
                payload["resolution"] = json!(value);
            }
            payload
        }
        Ok(None) => {
            let tolaria_status = match load_code_graph_store_from_tolaria(&tolaria_mirror) {
                Ok(Some((store, status))) => json!({
                    "available": true,
                    "repo_root": store.repo_root,
                    "store_path": target.store_path.to_string_lossy(),
                    "schema_version": store.schema_version,
                    "indexed_at_unix_ms": store.indexed_at_unix_ms,
                    "file_count": store.files.len(),
                    "tolaria": status,
                    "evidence_note": create_code_graph_status_evidence_note(&store)
                }),
                Ok(None) => json!({
                    "available": false,
                    "repo_root": path_key(&target.repo_root),
                    "store_path": target.store_path.to_string_lossy(),
                    "reason": "code graph store has not been built",
                    "diagnostic_severity": "warning",
                    "blocking": false,
                    "recommended_action": "Run ccc graph with update=true when graph-informed planning is required.",
                    "tolaria": tolaria_mirror_status(&tolaria_mirror, "missing").unwrap_or_else(|| json!({
                        "available": false,
                        "enabled": false,
                        "reason": "Tolaria graph mirror is not configured or not detected"
                    }))
                }),
                Err(error) => json!({
                    "available": false,
                    "repo_root": path_key(&target.repo_root),
                    "store_path": target.store_path.to_string_lossy(),
                    "reason": format!("code graph store has not been built; Tolaria graph mirror could not be loaded: {error}"),
                    "diagnostic_severity": "warning",
                    "blocking": false,
                    "recommended_action": "Run ccc graph with update=true when graph-informed planning is required.",
                    "tolaria": tolaria_mirror_status(&tolaria_mirror, "error").unwrap_or_else(|| json!({
                        "available": false,
                        "enabled": false,
                        "reason": "Tolaria graph mirror is not configured or not detected"
                    }))
                }),
            };
            let mut payload = tolaria_status;
            if let Some(value) = resolution {
                payload["resolution"] = json!(value);
            }
            payload
        }
        Err(error) => {
            let mut payload = json!({
                "available": false,
                "repo_root": path_key(&target.repo_root),
                "store_path": target.store_path.to_string_lossy(),
                "reason": error.to_string(),
                "diagnostic_severity": "warning",
                "blocking": false,
                "recommended_action": "Rebuild the graph with ccc graph update=true when graph-informed planning is required."
            });
            if let Some(value) = resolution {
                payload["resolution"] = json!(value);
            }
            payload
        }
    }
}

fn create_code_graph_status_evidence_note(store: &CodeGraphStore) -> Value {
    let mut directory_counts = BTreeMap::<String, usize>::new();
    for path in store.files.keys() {
        let directory = path
            .split('/')
            .next()
            .filter(|value| !value.is_empty())
            .unwrap_or(".");
        *directory_counts.entry(directory.to_string()).or_insert(0) += 1;
    }
    let mut top_directories = directory_counts.into_iter().collect::<Vec<_>>();
    top_directories.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let top_directories = top_directories
        .into_iter()
        .take(3)
        .map(|(directory, count)| Value::String(format!("{directory}:{count}")))
        .collect::<Vec<_>>();
    let text = if top_directories.is_empty() {
        "availability=store_loaded indexed_files=0".to_string()
    } else {
        format!(
            "availability=store_loaded indexed_files={} top_dirs={}",
            store.files.len(),
            top_directories
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        )
    };

    json!({
        "source": "code_graph_store",
        "kind": "availability",
        "text": text,
        "top_directories": top_directories,
    })
}

fn resolve_tolaria_graph_mirror(arguments: &Value, repo_root: &Path) -> Option<TolariaGraphMirror> {
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

    let repo_folder_name = tolaria_workspace_folder_name(repo_root);
    let relative_note_path = format!("ccc/repos/{repo_folder_name}/graph.md");
    let repo_folder = vault_path.join("ccc").join("repos").join(repo_folder_name);
    Some(TolariaGraphMirror {
        note_path: vault_path.join(&relative_note_path),
        repo_folder,
        vault_path,
        relative_note_path,
    })
}

fn tolaria_explicitly_enabled(arguments: &Value) -> bool {
    arguments
        .get("tolaria_enabled")
        .and_then(Value::as_bool)
        .or_else(|| arguments.get("tolaria_sync").and_then(Value::as_bool))
        == Some(true)
}

fn shared_config_tolaria_vault_path() -> Option<PathBuf> {
    let (_, config) = read_optional_shared_config_document().ok()??;
    config
        .pointer("/integrations/tolaria/vault_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            config
                .pointer("/code_graph/tolaria/vault_path")
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

fn tolaria_workspace_folder_name(workspace_root: &Path) -> String {
    let workspace_key = path_key(workspace_root);
    let workspace_name = workspace_root
        .file_name()
        .and_then(|value| value.to_str())
        .map(slugify_tolaria_segment)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "repo".to_string());
    format!(
        "{workspace_name}-{}",
        stable_hex_hash(workspace_key.as_bytes())
    )
}

fn tolaria_mirror_status(mirror: &Option<TolariaGraphMirror>, state: &str) -> Option<Value> {
    let mirror = mirror.as_ref()?;
    Some(json!({
        "available": mirror.note_path.exists(),
        "enabled": true,
        "state": state,
        "vault_path": mirror.vault_path.to_string_lossy(),
        "repo_folder": mirror.repo_folder.to_string_lossy(),
        "note_path": mirror.note_path.to_string_lossy(),
        "relative_note_path": mirror.relative_note_path,
        "reason": if mirror.note_path.exists() { "Tolaria graph mirror note exists" } else { "Tolaria graph mirror note has not been written" },
    }))
}

fn sync_code_graph_store_to_tolaria(
    mirror: &Option<TolariaGraphMirror>,
    store: &CodeGraphStore,
    local_store_path: &Path,
) -> Option<Value> {
    let mirror = mirror.as_ref()?;
    let result = write_tolaria_graph_note(mirror, store, local_store_path);
    Some(match result {
        Ok(()) => json!({
            "available": true,
            "enabled": true,
            "state": "synced",
            "vault_path": mirror.vault_path.to_string_lossy(),
            "repo_folder": mirror.repo_folder.to_string_lossy(),
            "note_path": mirror.note_path.to_string_lossy(),
            "relative_note_path": mirror.relative_note_path,
            "reason": "CCC graph store was mirrored into Tolaria",
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

fn write_tolaria_graph_note(
    mirror: &TolariaGraphMirror,
    store: &CodeGraphStore,
    local_store_path: &Path,
) -> io::Result<()> {
    // Store graph beside memory under the same Tolaria workspace namespace so
    // repo and document-root context can be discovered together.
    fs::create_dir_all(&mirror.repo_folder)?;
    let content = render_tolaria_graph_note(store, local_store_path)?;
    fs::write(&mirror.note_path, content)
}

fn render_tolaria_graph_note(
    store: &CodeGraphStore,
    local_store_path: &Path,
) -> io::Result<String> {
    let repo_root = &store.repo_root;
    let title = format!(
        "CCC Code Graph - {}",
        Path::new(repo_root)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(repo_root)
    );
    let store_json = serde_json::to_string_pretty(store).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode Tolaria graph mirror: {error}"),
        )
    })?;
    let file_rows = store
        .files
        .values()
        .take(200)
        .map(|file| {
            let symbols = file
                .symbols
                .iter()
                .take(8)
                .map(symbol_label)
                .collect::<Vec<_>>()
                .join(", ");
            let imports = file
                .imports
                .iter()
                .take(8)
                .map(|import| import.target.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "- `{}` language={} symbols=[{}] imports=[{}]",
                file.path, file.language, symbols, imports
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
        r#"---
type: Note
related_to: "[[ccc-code-graph-index]]"
ccc_repo_root: "{}"
ccc_graph_schema_version: {}
ccc_graph_indexed_at_unix_ms: {}
ccc_graph_file_count: {}
ccc_graph_store_path: "{}"
---

# {}

This note is managed by CCC. It mirrors the local CCC code graph so Tolaria can
search, relate, and surface repository context for captain planning.

## Indexed Files

{}

## Graph Store JSON

```json
{}
```
"#,
        escape_yaml_scalar(repo_root),
        store.schema_version,
        store.indexed_at_unix_ms,
        store.files.len(),
        escape_yaml_scalar(&local_store_path.to_string_lossy()),
        title,
        if file_rows.is_empty() {
            "- No indexed files.".to_string()
        } else {
            file_rows
        },
        store_json
    ))
}

fn load_code_graph_store_from_tolaria(
    mirror: &Option<TolariaGraphMirror>,
) -> io::Result<Option<(CodeGraphStore, Value)>> {
    let Some(mirror) = mirror else {
        return Ok(None);
    };
    if !mirror.note_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&mirror.note_path)?;
    let Some(json_text) = extract_tolaria_graph_json(&content) else {
        return Ok(None);
    };
    let store: CodeGraphStore = serde_json::from_str(json_text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid Tolaria code graph mirror {}: {error}",
                mirror.note_path.display()
            ),
        )
    })?;
    Ok(Some((
        store,
        json!({
            "available": true,
            "enabled": true,
            "state": "loaded_from_tolaria",
            "vault_path": mirror.vault_path.to_string_lossy(),
            "repo_folder": mirror.repo_folder.to_string_lossy(),
            "note_path": mirror.note_path.to_string_lossy(),
            "relative_note_path": mirror.relative_note_path,
            "reason": "Local CCC graph store was missing, so Tolaria graph mirror was used",
        }),
    )))
}

fn extract_tolaria_graph_json(content: &str) -> Option<&str> {
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

fn create_code_graph_query_evidence_note(
    query: &str,
    query_payload: &Value,
    query_paths: &[Value],
) -> Value {
    let path_text = compact_value_strings(query_paths, 3).join(",");
    let mut parts = vec![format!("query={query}")];
    if !path_text.is_empty() {
        parts.push(format!("paths={path_text}"));
    }

    match query {
        "file_summary" | "imports" => {
            let summaries = value_array(query_payload, "/file_summaries");
            parts.push(format!("summaries={}", summaries.len()));
            let top = compact_paths_from_values(summaries, 3);
            if !top.is_empty() {
                parts.push(format!("files={}", top.join(",")));
            }
        }
        "callers" => append_relation_evidence(&mut parts, "callers", query_payload, "/callers"),
        "callees" => append_relation_evidence(&mut parts, "callees", query_payload, "/callees"),
        "tests" => {
            append_relation_evidence(&mut parts, "related_tests", query_payload, "/related_tests")
        }
        "impact" | "blast_radius" => {
            if let Some(risk) = query_payload
                .pointer("/blast_radius/risk_level")
                .and_then(Value::as_str)
            {
                parts.push(format!("risk={risk}"));
            }
            let impacted = value_array(query_payload, "/blast_radius/impacted_files");
            let tests = value_array(query_payload, "/blast_radius/related_tests");
            parts.push(format!("impacted={}", impacted.len()));
            parts.push(format!("tests={}", tests.len()));
        }
        "review_context" => {
            if let Some(risk) = query_payload
                .pointer("/review_context/blast_radius/risk_level")
                .and_then(Value::as_str)
            {
                parts.push(format!("risk={risk}"));
            }
            parts.push(format!(
                "summaries={}",
                value_array(query_payload, "/review_context/summaries").len()
            ));
            parts.push(format!(
                "callers={}",
                value_array(query_payload, "/review_context/callers").len()
            ));
            parts.push(format!(
                "tests={}",
                value_array(query_payload, "/review_context/blast_radius/related_tests").len()
            ));
        }
        "flow_trace" | "flows" => {
            parts.push(format!(
                "edges={}",
                value_array(query_payload, "/flow_trace/edges").len()
            ));
        }
        "criticality" | "criticality_scores" => {
            let scores = value_array(query_payload, "/criticality_scores");
            parts.push(format!("scores={}", scores.len()));
            let top = compact_paths_from_values(scores, 3);
            if !top.is_empty() {
                parts.push(format!("top={}", top.join(",")));
            }
        }
        "communities" | "architecture_overview" => {
            parts.push(format!(
                "communities={}",
                value_array(query_payload, "/architecture_overview/communities").len()
            ));
        }
        "full_text_search" | "search" => {
            let matches = value_array(query_payload, "/search/matches");
            parts.push(format!("matches={}", matches.len()));
            let top = compact_paths_from_values(matches, 3);
            if !top.is_empty() {
                parts.push(format!("top={}", top.join(",")));
            }
        }
        "multi_repo_search" => {
            parts.push(format!(
                "repos={}",
                value_array(query_payload, "/multi_repo_search/repos").len()
            ));
        }
        _ => {}
    }

    json!({
        "source": "code_graph_query",
        "kind": query,
        "text": parts.join(" "),
    })
}

fn append_relation_evidence(parts: &mut Vec<String>, label: &str, payload: &Value, pointer: &str) {
    let values = value_array(payload, pointer);
    parts.push(format!("{label}={}", values.len()));
    let top = compact_paths_from_values(values, 3);
    if !top.is_empty() {
        parts.push(format!("top={}", top.join(",")));
    }
}

fn value_array<'a>(payload: &'a Value, pointer: &str) -> &'a [Value] {
    payload
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn compact_paths_from_values(values: &[Value], limit: usize) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| value.get("path").and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .take(limit)
        .map(str::to_string)
        .collect()
}

fn compact_value_strings(values: &[Value], limit: usize) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .take(limit)
        .map(str::to_string)
        .collect()
}

fn resolve_code_graph_status_target(cwd: &Path) -> io::Result<(CodeGraphTarget, Option<String>)> {
    let detected = detect_repo_root(cwd)?;
    if is_repo_marker_directory(&detected) {
        let target = CodeGraphTarget {
            store_path: default_graph_store_path(&detected),
            repo_root: detected,
        };
        if target.store_path.exists() {
            return Ok((target, None));
        }
        let child_stores = immediate_child_graph_store_roots(cwd)?;
        return match child_stores.as_slice() {
            [repo_root] => Ok((
                CodeGraphTarget {
                    store_path: default_graph_store_path(repo_root),
                    repo_root: repo_root.clone(),
                },
                Some("single_child_graph_store".to_string()),
            )),
            [] => Ok((target, None)),
            _ => Err(ambiguous_graph_target_error(cwd, &child_stores)),
        };
    }

    let child_stores = immediate_child_graph_store_roots(cwd)?;
    let document_store_path = default_graph_store_path(&detected);
    if document_store_path.exists() {
        return Ok((
            CodeGraphTarget {
                store_path: document_store_path,
                repo_root: detected,
            },
            Some("document_graph_store".to_string()),
        ));
    }
    match child_stores.as_slice() {
        [repo_root] => Ok((
            CodeGraphTarget {
                store_path: default_graph_store_path(repo_root),
                repo_root: repo_root.clone(),
            },
            Some("single_child_graph_store".to_string()),
        )),
        [] => Err(no_graph_target_error(cwd)),
        _ => Err(ambiguous_graph_target_error(cwd, &child_stores)),
    }
}

fn resolve_code_graph_target(
    cwd: &Path,
    paths: &[PathBuf],
    explicit_store_path: Option<PathBuf>,
    update: bool,
) -> io::Result<CodeGraphTarget> {
    let detected = detect_repo_root(cwd)?;
    if is_repo_marker_directory(&detected) {
        if let Some(store_path) = explicit_store_path {
            return Ok(CodeGraphTarget {
                repo_root: detected,
                store_path,
            });
        }
        let store_path = default_graph_store_path(&detected);
        if store_path.exists() {
            return Ok(CodeGraphTarget {
                repo_root: detected,
                store_path,
            });
        }
        if let Some(repo_root) = repo_root_from_query_paths(cwd, paths)? {
            return Ok(CodeGraphTarget {
                store_path: default_graph_store_path(&repo_root),
                repo_root,
            });
        }
        let child_stores = immediate_child_graph_store_roots(cwd)?;
        return match child_stores.as_slice() {
            [repo_root] => Ok(CodeGraphTarget {
                store_path: default_graph_store_path(repo_root),
                repo_root: repo_root.clone(),
            }),
            [] => Ok(CodeGraphTarget {
                repo_root: detected,
                store_path,
            }),
            _ => Err(ambiguous_graph_target_error(cwd, &child_stores)),
        };
    }

    if let Some(store_path) = explicit_store_path {
        if let Some(repo_root) = repo_root_from_store_path(&store_path) {
            return Ok(CodeGraphTarget {
                repo_root,
                store_path,
            });
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "cwd is not a repo and store_path does not identify a repo graph store: {}. Pass cwd as the target repo path or use a store_path under <repo>/.ccc/graph/store.json.",
                store_path.display()
            ),
        ));
    }

    if let Some(repo_root) = repo_root_from_query_paths(cwd, paths)? {
        return Ok(CodeGraphTarget {
            store_path: default_graph_store_path(&repo_root),
            repo_root,
        });
    }

    let child_stores = immediate_child_graph_store_roots(cwd)?;
    let document_store_path = default_graph_store_path(&detected);
    if document_store_path.exists() || update {
        return Ok(CodeGraphTarget {
            repo_root: detected,
            store_path: document_store_path,
        });
    }
    match child_stores.as_slice() {
        [repo_root] => Ok(CodeGraphTarget {
            store_path: default_graph_store_path(repo_root),
            repo_root: repo_root.clone(),
        }),
        [] => Err(no_graph_target_error(cwd)),
        _ => Err(ambiguous_graph_target_error(cwd, &child_stores)),
    }
}

fn repo_root_from_store_path(store_path: &Path) -> Option<PathBuf> {
    let graph_dir = store_path.parent()?;
    if graph_dir.file_name().and_then(|value| value.to_str()) != Some("graph") {
        return None;
    }
    let ccc_dir = graph_dir.parent()?;
    if ccc_dir.file_name().and_then(|value| value.to_str()) != Some(".ccc") {
        return None;
    }
    let repo_root = ccc_dir.parent()?;
    is_repo_marker_directory(repo_root)
        .then(|| fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf()))
}

fn repo_root_from_query_paths(cwd: &Path, paths: &[PathBuf]) -> io::Result<Option<PathBuf>> {
    if paths.is_empty() {
        return Ok(None);
    }

    let cwd = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let child_repos = immediate_child_repo_roots(&cwd)?;
    let mut candidates = BTreeSet::new();
    for path in paths {
        let candidate_path = if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        };
        let candidate_path =
            fs::canonicalize(&candidate_path).unwrap_or_else(|_| candidate_path.clone());
        let matching_child_repos = child_repos
            .iter()
            .filter(|repo_root| candidate_path.starts_with(repo_root))
            .cloned()
            .collect::<Vec<_>>();
        if matching_child_repos.len() == 1 {
            candidates.insert(matching_child_repos[0].clone());
            continue;
        }
        let repo_root = detect_repo_root(&candidate_path)?;
        if repo_root != cwd && repo_root.starts_with(&cwd) && is_repo_marker_directory(&repo_root) {
            candidates.insert(repo_root);
        }
    }

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.into_iter().next()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "cwd is not a repo and paths identify multiple child repos under {}. Pass cwd as the target repo path.",
                cwd.display()
            ),
        )),
    }
}

fn immediate_child_graph_store_roots(cwd: &Path) -> io::Result<Vec<PathBuf>> {
    let roots = immediate_child_repo_roots(cwd)?
        .into_iter()
        .filter(|repo_root| default_graph_store_path(repo_root).exists())
        .collect::<Vec<_>>();
    Ok(roots)
}

fn immediate_child_repo_roots(cwd: &Path) -> io::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    let entries = match fs::read_dir(cwd) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(roots),
        Err(error) => return Err(error),
    };

    for entry in entries.filter_map(Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let repo_root = entry.path();
        if !is_repo_marker_directory(&repo_root) {
            continue;
        }
        roots.push(fs::canonicalize(&repo_root).unwrap_or(repo_root));
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn normalize_query_paths_for_target(
    cwd: &Path,
    repo_root: &Path,
    paths: &[PathBuf],
) -> Vec<PathBuf> {
    let cwd = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let repo_root = fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    let repo_relative_to_cwd = repo_root.strip_prefix(&cwd).ok().map(Path::to_path_buf);

    paths
        .iter()
        .map(|path| {
            if path.is_absolute() {
                return path
                    .strip_prefix(&repo_root)
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|_| path.clone());
            }
            if let Some(prefix) = &repo_relative_to_cwd {
                return path
                    .strip_prefix(prefix)
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|_| path.clone());
            }
            path.clone()
        })
        .collect()
}

fn no_graph_target_error(cwd: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "cwd must be a git repo when no code graph store exists; no code graph store was found at {} or in an immediate child repo. Pass cwd as the target repo path, pass store_path, or run graph update inside the target repo.",
            default_graph_store_path(cwd).display()
        ),
    )
}

fn ambiguous_graph_target_error(cwd: &Path, child_stores: &[PathBuf]) -> io::Error {
    let repos = child_stores
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "cwd is not a repo and multiple child code graph stores were found under {}: {repos}. Pass cwd as the target repo path or include paths that identify one child repo.",
            cwd.display()
        ),
    )
}

pub(crate) fn create_code_graph_text(payload: &Value) -> String {
    let file_count = payload
        .get("file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let repo_root = payload
        .get("repo_root")
        .and_then(Value::as_str)
        .unwrap_or("unknown-repo");
    let query_paths = payload
        .get("query_paths")
        .and_then(Value::as_array)
        .map(|values| compact_value_strings(values, 3).join(","))
        .filter(|value| !value.is_empty());
    let reference = query_paths
        .map(|paths| format!("query={query} paths={paths}"))
        .unwrap_or_else(|| format!("query={query} repo={repo_root}"));
    let risk = payload
        .pointer("/query_result/review_context/blast_radius/risk_level")
        .or_else(|| payload.pointer("/query_result/blast_radius/risk_level"))
        .and_then(Value::as_str);
    let found = match risk {
        Some(risk) => format!("{file_count} indexed files, risk={risk}"),
        None => format!("{file_count} indexed files"),
    };
    format!("Graph: Way referenced {reference}; found {found}; graph-informed planning next step.")
}

pub(crate) fn update_code_graph_store_at(
    repo_root: &Path,
    store_path: &Path,
) -> io::Result<CodeGraphStore> {
    let repo_root = fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    let previous = load_code_graph_store(store_path)?.unwrap_or_else(|| CodeGraphStore {
        schema_version: CODE_GRAPH_SCHEMA_VERSION,
        repo_root: path_key(&repo_root),
        indexed_at_unix_ms: 0,
        files: BTreeMap::new(),
    });

    let mut source_paths = Vec::new();
    collect_source_paths(&repo_root, &repo_root, &mut source_paths)?;
    source_paths.sort();

    let mut next_files = BTreeMap::new();
    for source_path in source_paths {
        let metadata = fs::metadata(&source_path)?;
        let relative_path = relative_path_key(&repo_root, &source_path);
        let modified_unix_ms = modified_unix_ms(&metadata);
        if let Some(existing) = previous.files.get(&relative_path) {
            if existing.size_bytes == metadata.len()
                && existing.modified_unix_ms == modified_unix_ms
                && existing.language == language_for_path(&source_path)
            {
                next_files.insert(relative_path, existing.clone());
                continue;
            }
        }

        let file = parse_code_graph_file(&repo_root, &source_path, &metadata)?;
        next_files.insert(file.path.clone(), file);
    }

    let store = CodeGraphStore {
        schema_version: CODE_GRAPH_SCHEMA_VERSION,
        repo_root: path_key(&repo_root),
        indexed_at_unix_ms: now_unix_ms(),
        files: next_files,
    };
    write_code_graph_store(store_path, &store)?;
    Ok(store)
}

pub(crate) struct CodeGraphQuery<'a> {
    store: &'a CodeGraphStore,
}

impl<'a> CodeGraphQuery<'a> {
    pub(crate) fn new(store: &'a CodeGraphStore) -> Self {
        Self { store }
    }

    pub(crate) fn file_summary(&self, path: &Path) -> Option<CodeGraphSummary> {
        let key = self.resolve_path(path)?;
        let file = self.store.files.get(&key)?;
        Some(CodeGraphSummary {
            path: file.path.clone(),
            language: file.language.clone(),
            imports: file
                .imports
                .iter()
                .map(|import| import.target.clone())
                .collect(),
            symbols: file.symbols.iter().map(symbol_label).collect(),
            symbol_count: file.symbols.len(),
            reference_count: file.identifier_refs.len(),
        })
    }

    pub(crate) fn callers_for_file(&self, path: &Path) -> Vec<CodeGraphRelation> {
        let Some(target_key) = self.resolve_path(path) else {
            return Vec::new();
        };
        let Some(target_file) = self.store.files.get(&target_key) else {
            return Vec::new();
        };
        let target_symbols = target_file
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();

        let mut relations = Vec::new();
        for (candidate_key, candidate) in &self.store.files {
            if candidate_key == &target_key {
                continue;
            }
            let mut reasons = BTreeSet::new();
            if candidate
                .imports
                .iter()
                .any(|import| import_matches_file(candidate_key, import, &target_key))
            {
                reasons.insert(format!("imports {target_key}"));
            }
            for symbol in &target_symbols {
                if candidate.identifier_refs.iter().any(|name| name == symbol) {
                    reasons.insert(format!("references symbol {symbol}"));
                }
            }
            push_relation(&mut relations, candidate_key, reasons);
        }
        relations
    }

    pub(crate) fn callees_for_file(&self, path: &Path) -> Vec<CodeGraphRelation> {
        let Some(source_key) = self.resolve_path(path) else {
            return Vec::new();
        };
        let Some(source_file) = self.store.files.get(&source_key) else {
            return Vec::new();
        };

        let mut relations = Vec::new();
        for (candidate_key, candidate) in &self.store.files {
            if candidate_key == &source_key {
                continue;
            }
            let mut reasons = BTreeSet::new();
            if source_file
                .imports
                .iter()
                .any(|import| import_matches_file(&source_key, import, candidate_key))
            {
                reasons.insert(format!("imports {candidate_key}"));
            }
            for symbol in &candidate.symbols {
                if source_file
                    .identifier_refs
                    .iter()
                    .any(|name| name == &symbol.name)
                {
                    reasons.insert(format!("references symbol {}", symbol.name));
                }
            }
            push_relation(&mut relations, candidate_key, reasons);
        }
        relations
    }

    pub(crate) fn related_tests_for_file(&self, path: &Path) -> Vec<CodeGraphRelation> {
        let Some(source_key) = self.resolve_path(path) else {
            return Vec::new();
        };
        let Some(source_file) = self.store.files.get(&source_key) else {
            return Vec::new();
        };
        let source_stem = file_stem_key(&source_key);
        let source_symbols = source_file
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();

        let mut relations = Vec::new();
        for (candidate_key, candidate) in &self.store.files {
            if !is_test_path(candidate_key) {
                continue;
            }
            let mut reasons = BTreeSet::new();
            if candidate_key == &source_key {
                reasons.insert("changed file is a test".to_string());
            }
            if !source_stem.is_empty() && candidate_key.contains(&source_stem) {
                reasons.insert(format!("test name matches {source_stem}"));
            }
            if candidate
                .imports
                .iter()
                .any(|import| import_matches_file(candidate_key, import, &source_key))
            {
                reasons.insert(format!("imports {source_key}"));
            }
            for symbol in &source_symbols {
                if candidate.identifier_refs.iter().any(|name| name == symbol) {
                    reasons.insert(format!("references symbol {symbol}"));
                }
            }
            push_relation(&mut relations, candidate_key, reasons);
        }
        relations
    }

    pub(crate) fn blast_radius_for_changed_paths(
        &self,
        changed_paths: &[PathBuf],
    ) -> CodeGraphBlastRadius {
        let changed_files = changed_paths
            .iter()
            .filter_map(|path| self.resolve_path(path))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let mut impacted = BTreeMap::<String, BTreeSet<String>>::new();
        let mut related_tests = BTreeMap::<String, BTreeSet<String>>::new();
        for changed_file in &changed_files {
            impacted
                .entry(changed_file.clone())
                .or_default()
                .insert("changed file".to_string());

            for relation in self.callers_for_file(Path::new(changed_file)) {
                let entry = impacted.entry(relation.path).or_default();
                entry.extend(relation.reasons);
            }
            for relation in self.related_tests_for_file(Path::new(changed_file)) {
                let entry = related_tests.entry(relation.path.clone()).or_default();
                entry.extend(relation.reasons.clone());
                let impacted_entry = impacted.entry(relation.path).or_default();
                impacted_entry.insert("related test".to_string());
            }
        }

        let related_tests = map_to_relations(related_tests);
        let impacted_files = map_to_relations(impacted);
        let (risk_score, risk_level, risk_reasons) =
            score_risk(&changed_files, &impacted_files, &related_tests);

        CodeGraphBlastRadius {
            changed_files,
            impacted_files,
            related_tests,
            risk_score,
            risk_level,
            risk_reasons,
        }
    }

    pub(crate) fn minimal_review_context(
        &self,
        changed_paths: &[PathBuf],
    ) -> CodeGraphReviewContext {
        let blast_radius = self.blast_radius_for_changed_paths(changed_paths);
        let summaries = blast_radius
            .impacted_files
            .iter()
            .filter_map(|relation| self.file_summary(Path::new(&relation.path)))
            .collect::<Vec<_>>();
        let callers = blast_radius
            .changed_files
            .iter()
            .flat_map(|path| self.callers_for_file(Path::new(path)))
            .collect::<Vec<_>>();
        let callees = blast_radius
            .changed_files
            .iter()
            .flat_map(|path| self.callees_for_file(Path::new(path)))
            .collect::<Vec<_>>();

        CodeGraphReviewContext {
            summaries,
            callers: dedupe_relations(callers),
            callees: dedupe_relations(callees),
            blast_radius,
        }
    }

    pub(crate) fn flow_trace_for_paths(
        &self,
        paths: &[PathBuf],
        direction: &str,
        max_depth: usize,
    ) -> CodeGraphFlowTrace {
        let roots = paths
            .iter()
            .filter_map(|path| self.resolve_path(path))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let direction = match direction {
            "callers" | "upstream" => "callers",
            "callees" | "downstream" => "callees",
            _ => "both",
        }
        .to_string();
        let max_depth = max_depth.clamp(1, 6);
        let mut queue = roots
            .iter()
            .map(|root| (root.clone(), 0_usize))
            .collect::<VecDeque<_>>();
        let mut visited_nodes = roots.iter().cloned().collect::<BTreeSet<_>>();
        let mut visited_edges = BTreeSet::<(String, String)>::new();
        let mut edges = Vec::new();
        let mut truncated = false;

        // Keep the trace bounded and deterministic: revisit neither graph nodes nor rendered
        // edges, and cap expansion before a broad repo can flood the MCP/status response.
        while let Some((path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            let mut neighbors = Vec::new();
            if direction == "both" || direction == "callees" {
                neighbors.extend(self.callees_for_file(Path::new(&path)).into_iter().map(
                    |relation| CodeGraphFlowEdge {
                        from: path.clone(),
                        to: relation.path,
                        depth: depth + 1,
                        reasons: relation.reasons,
                    },
                ));
            }
            if direction == "both" || direction == "callers" {
                neighbors.extend(self.callers_for_file(Path::new(&path)).into_iter().map(
                    |relation| CodeGraphFlowEdge {
                        from: relation.path,
                        to: path.clone(),
                        depth: depth + 1,
                        reasons: relation.reasons,
                    },
                ));
            }

            for edge in neighbors {
                if edges.len() >= 200 {
                    truncated = true;
                    break;
                }
                let edge_key = (edge.from.clone(), edge.to.clone());
                if !visited_edges.insert(edge_key) {
                    continue;
                }
                let next_path = if edge.from == path {
                    edge.to.clone()
                } else {
                    edge.from.clone()
                };
                if visited_nodes.insert(next_path.clone()) {
                    queue.push_back((next_path, depth + 1));
                }
                edges.push(edge);
            }
            if truncated {
                break;
            }
        }

        CodeGraphFlowTrace {
            roots,
            direction,
            max_depth,
            edges,
            truncated,
        }
    }

    pub(crate) fn criticality_scores(&self, paths: &[PathBuf]) -> Vec<CodeGraphCriticality> {
        let selected = if paths.is_empty() {
            self.store.files.keys().cloned().collect::<Vec<_>>()
        } else {
            paths
                .iter()
                .filter_map(|path| self.resolve_path(path))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        };
        let mut scores = selected
            .into_iter()
            .filter_map(|path| self.criticality_for_file(&path))
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.path.cmp(&right.path))
        });
        scores
    }

    fn criticality_for_file(&self, path: &str) -> Option<CodeGraphCriticality> {
        let file = self.store.files.get(path)?;
        let caller_count = self.callers_for_file(Path::new(path)).len();
        let callee_count = self.callees_for_file(Path::new(path)).len();
        let related_test_count = self.related_tests_for_file(Path::new(path)).len();
        let symbol_count = file.symbols.len();
        let mut score = 5_u16;
        let mut reasons = Vec::new();
        if caller_count > 0 {
            score += (caller_count as u16 * 18).min(45);
            reasons.push(format!("{caller_count} caller(s) depend on this file"));
        }
        if callee_count > 0 {
            score += (callee_count as u16 * 8).min(24);
            reasons.push(format!("{callee_count} callee dependency(ies)"));
        }
        if related_test_count == 0 && !is_test_path(path) {
            score += 12;
            reasons.push("no directly related test found".to_string());
        }
        if symbol_count >= 4 {
            score += 10;
            reasons.push(format!("{symbol_count} exported/local symbol(s)"));
        }
        if is_test_path(path) {
            score = score.saturating_sub(10);
            reasons.push("test file criticality is capped lower".to_string());
        }

        let score = score.min(100) as u8;
        Some(CodeGraphCriticality {
            path: path.to_string(),
            score,
            level: criticality_level(score).to_string(),
            reasons,
            caller_count,
            callee_count,
            related_test_count,
            symbol_count,
        })
    }

    fn resolve_path(&self, path: &Path) -> Option<String> {
        let direct = normalize_query_path(&self.store.repo_root, path);
        if self.store.files.contains_key(&direct) {
            return Some(direct);
        }
        let suffix = direct.trim_start_matches("./");
        self.store
            .files
            .keys()
            .find(|candidate| candidate.ends_with(suffix))
            .cloned()
    }
}

fn parse_code_graph_file(
    repo_root: &Path,
    path: &Path,
    metadata: &fs::Metadata,
) -> io::Result<CodeGraphFile> {
    let content = fs::read_to_string(path)?;
    let language = language_for_path(path);
    let mut imports = Vec::new();
    let mut symbols = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        parse_imports_for_line(&language, line, line_number, &mut imports);
        parse_symbols_for_line(&language, line, line_number, &mut symbols);
    }

    Ok(CodeGraphFile {
        path: relative_path_key(repo_root, path),
        language,
        size_bytes: metadata.len(),
        modified_unix_ms: modified_unix_ms(metadata),
        imports,
        symbols,
        identifier_refs: identifier_refs(&content),
    })
}

fn summary_to_value(summary: CodeGraphSummary) -> Value {
    json!({
        "path": summary.path,
        "language": summary.language,
        "imports": summary.imports,
        "symbols": summary.symbols,
        "symbol_count": summary.symbol_count,
        "reference_count": summary.reference_count
    })
}

fn relation_to_value(relation: CodeGraphRelation) -> Value {
    json!({
        "path": relation.path,
        "reasons": relation.reasons
    })
}

fn blast_radius_to_value(blast_radius: CodeGraphBlastRadius) -> Value {
    json!({
        "changed_files": blast_radius.changed_files,
        "impacted_files": blast_radius.impacted_files.into_iter().map(relation_to_value).collect::<Vec<_>>(),
        "related_tests": blast_radius.related_tests.into_iter().map(relation_to_value).collect::<Vec<_>>(),
        "risk_score": blast_radius.risk_score,
        "risk_level": blast_radius.risk_level,
        "risk_reasons": blast_radius.risk_reasons
    })
}

fn review_context_to_value(context: CodeGraphReviewContext) -> Value {
    json!({
        "summaries": context.summaries.into_iter().map(summary_to_value).collect::<Vec<_>>(),
        "callers": context.callers.into_iter().map(relation_to_value).collect::<Vec<_>>(),
        "callees": context.callees.into_iter().map(relation_to_value).collect::<Vec<_>>(),
        "blast_radius": blast_radius_to_value(context.blast_radius)
    })
}

fn query_text(arguments: &Value) -> io::Result<String> {
    arguments
        .get("text")
        .or_else(|| arguments.get("term"))
        .or_else(|| arguments.get("search"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "text, term, or search is required for search queries",
            )
        })
}

fn flow_trace_to_value(trace: CodeGraphFlowTrace) -> Value {
    json!({
        "roots": trace.roots,
        "direction": trace.direction,
        "max_depth": trace.max_depth,
        "truncated": trace.truncated,
        "edges": trace.edges.into_iter().map(|edge| json!({
            "from": edge.from,
            "to": edge.to,
            "depth": edge.depth,
            "reasons": edge.reasons
        })).collect::<Vec<_>>()
    })
}

fn criticality_to_value(score: CodeGraphCriticality) -> Value {
    json!({
        "path": score.path,
        "score": score.score,
        "level": score.level,
        "reasons": score.reasons,
        "caller_count": score.caller_count,
        "callee_count": score.callee_count,
        "related_test_count": score.related_test_count,
        "symbol_count": score.symbol_count
    })
}

fn architecture_overview_to_value(query: &CodeGraphQuery<'_>, limit: usize) -> Value {
    let mut language_counts = BTreeMap::<String, usize>::new();
    let mut communities = BTreeMap::<String, Vec<String>>::new();
    let mut cross_edges = BTreeMap::<String, BTreeSet<String>>::new();

    for (path, file) in &query.store.files {
        *language_counts.entry(file.language.clone()).or_default() += 1;
        communities
            .entry(community_id_for_path(path))
            .or_default()
            .push(path.clone());
    }

    for path in query.store.files.keys() {
        let source_community = community_id_for_path(path);
        for relation in query.callees_for_file(Path::new(path)) {
            let target_community = community_id_for_path(&relation.path);
            if source_community != target_community {
                cross_edges
                    .entry(source_community.clone())
                    .or_default()
                    .insert(target_community);
            }
        }
    }

    let criticality_by_path = query
        .criticality_scores(&[])
        .into_iter()
        .map(|score| (score.path.clone(), score))
        .collect::<BTreeMap<_, _>>();
    let mut community_values = communities
        .into_iter()
        .map(|(id, mut files)| {
            files.sort_by(|left, right| {
                criticality_by_path
                    .get(right)
                    .map(|score| score.score)
                    .unwrap_or(0)
                    .cmp(
                        &criticality_by_path
                            .get(left)
                            .map(|score| score.score)
                            .unwrap_or(0),
                    )
                    .then_with(|| left.cmp(right))
            });
            let mut community_languages = BTreeMap::<String, usize>::new();
            for path in &files {
                if let Some(file) = query.store.files.get(path) {
                    *community_languages
                        .entry(file.language.clone())
                        .or_default() += 1;
                }
            }
            json!({
                "id": id,
                "file_count": files.len(),
                "languages": community_languages,
                "representative_files": files.into_iter().take(5).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    community_values.sort_by(|left, right| {
        right
            .get("file_count")
            .and_then(Value::as_u64)
            .cmp(&left.get("file_count").and_then(Value::as_u64))
            .then_with(|| {
                left.get("id")
                    .and_then(Value::as_str)
                    .cmp(&right.get("id").and_then(Value::as_str))
            })
    });

    json!({
        "file_count": query.store.files.len(),
        "language_counts": language_counts,
        "community_count": community_values.len(),
        "communities": community_values.into_iter().take(limit).collect::<Vec<_>>(),
        "cross_community_dependencies": cross_edges
            .into_iter()
            .map(|(from, targets)| json!({"from": from, "to": targets.into_iter().collect::<Vec<_>>()}))
            .collect::<Vec<_>>()
    })
}

fn community_id_for_path(path: &str) -> String {
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let first = parts.next().unwrap_or("root");
    if matches!(first, "src" | "rust" | "crates" | "packages" | "apps") {
        parts
            .next()
            .map(|second| format!("{first}/{second}"))
            .unwrap_or_else(|| first.to_string())
    } else {
        first.to_string()
    }
}

fn criticality_level(score: u8) -> &'static str {
    match score {
        0..=34 => "low",
        35..=69 => "medium",
        _ => "high",
    }
}

fn search_score(line: &str, needle: &str) -> u8 {
    let trimmed = line.trim();
    if trimmed == needle {
        100
    } else if trimmed
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|word| word == needle)
    {
        85
    } else {
        60
    }
}

fn summarize_snippet(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() <= 160 {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(160).collect::<String>())
    }
}

fn full_text_search(
    store: &CodeGraphStore,
    repo_root: &Path,
    text: &str,
    limit: usize,
) -> Vec<CodeGraphSearchMatch> {
    let needle = text.trim();
    if needle.is_empty() {
        return Vec::new();
    }
    let needle_lowercase = needle.to_ascii_lowercase();
    let mut matches = Vec::new();

    for path in store.files.keys() {
        let file_path = repo_root.join(path);
        let Ok(content) = fs::read_to_string(&file_path) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            if !line.to_ascii_lowercase().contains(&needle_lowercase) {
                continue;
            }
            matches.push(CodeGraphSearchMatch {
                path: path.clone(),
                line: index + 1,
                score: search_score(line, needle),
                match_type: "text".to_string(),
                snippet: summarize_snippet(line),
            });
            if matches.len() >= limit {
                return matches;
            }
        }
    }
    matches
}

fn full_text_search_to_value(matches: Vec<CodeGraphSearchMatch>) -> Value {
    json!({
        "matches": matches.into_iter().map(|entry| json!({
            "path": entry.path,
            "line": entry.line,
            "score": entry.score,
            "match_type": entry.match_type,
            "snippet": entry.snippet,
        })).collect::<Vec<_>>()
    })
}

fn multi_repo_search(arguments: &Value, text: &str, limit: usize) -> io::Result<Value> {
    let repos = arguments
        .get("repos")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if repos.is_empty() {
        return Ok(json!({
            "repos": [],
            "summary": "no repos were provided"
        }));
    }

    let mut repo_results = Vec::new();
    for repo in repos {
        let repo_cwd = repo
            .as_str()
            .map(PathBuf::from)
            .or_else(|| repo.get("cwd").and_then(Value::as_str).map(PathBuf::from))
            .unwrap_or(std::env::current_dir()?);
        let repo_root = detect_repo_root(&repo_cwd)?;
        let store_path = repo
            .get("store_path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_graph_store_path(&repo_root));
        let update = repo
            .get("update")
            .and_then(Value::as_bool)
            .or_else(|| arguments.get("update").and_then(Value::as_bool))
            .unwrap_or(false);
        let store = if update {
            update_code_graph_store_at(&repo_root, &store_path)?
        } else {
            load_code_graph_store(&store_path)?.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("code graph store not found: {}", store_path.display()),
                )
            })?
        };
        repo_results.push(json!({
            "repo_root": store.repo_root,
            "store_path": store_path.to_string_lossy(),
            "file_count": store.files.len(),
            "search": full_text_search_to_value(full_text_search(&store, &repo_root, text, limit)),
        }));
    }

    Ok(json!({
        "repos": repo_results,
        "query_text": text,
    }))
}

fn parse_imports_for_line(
    language: &str,
    line: &str,
    line_number: usize,
    imports: &mut Vec<CodeGraphImport>,
) {
    let trimmed = line.trim();
    match language {
        "rust" => {
            let without_pub = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
            if let Some(rest) = without_pub.strip_prefix("use ") {
                let raw = rest.trim_end_matches(';').trim().to_string();
                imports.push(CodeGraphImport {
                    target: raw.clone(),
                    raw,
                    line: line_number,
                    kind: "use".to_string(),
                });
            } else if let Some(rest) = without_pub.strip_prefix("mod ") {
                let raw = rest.trim_end_matches(';').trim().to_string();
                imports.push(CodeGraphImport {
                    target: raw.clone(),
                    raw,
                    line: line_number,
                    kind: "mod".to_string(),
                });
            }
        }
        "typescript" | "javascript" => {
            if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                if let Some(target) =
                    quoted_segment_after_from(trimmed).or_else(|| quoted_segment(trimmed))
                {
                    imports.push(CodeGraphImport {
                        target: target.clone(),
                        raw: trimmed.to_string(),
                        line: line_number,
                        kind: "import".to_string(),
                    });
                }
            }
            if let Some(require_index) = trimmed.find("require(") {
                if let Some(target) = quoted_segment(&trimmed[require_index..]) {
                    imports.push(CodeGraphImport {
                        target,
                        raw: trimmed.to_string(),
                        line: line_number,
                        kind: "require".to_string(),
                    });
                }
            }
        }
        "markdown" => {
            parse_markdown_links_for_line(trimmed, line_number, imports);
        }
        "python" => {
            if let Some(rest) = trimmed.strip_prefix("from ") {
                if let Some((target, _)) = rest.split_once(" import ") {
                    imports.push(CodeGraphImport {
                        target: target.trim().to_string(),
                        raw: trimmed.to_string(),
                        line: line_number,
                        kind: "from".to_string(),
                    });
                }
            } else if let Some(rest) = trimmed.strip_prefix("import ") {
                for target in rest.split(',') {
                    let target = target.trim().split_whitespace().next().unwrap_or("").trim();
                    if !target.is_empty() {
                        imports.push(CodeGraphImport {
                            target: target.to_string(),
                            raw: trimmed.to_string(),
                            line: line_number,
                            kind: "import".to_string(),
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

fn parse_symbols_for_line(
    language: &str,
    line: &str,
    line_number: usize,
    symbols: &mut Vec<CodeGraphSymbol>,
) {
    let trimmed = line.trim();
    match language {
        "rust" => {
            if let Some(name) = word_after_keyword(trimmed, "fn") {
                symbols.push(symbol(name, "function", line_number));
            }
            for keyword in ["struct", "enum", "trait"] {
                if let Some(name) = word_after_keyword(trimmed, keyword) {
                    symbols.push(symbol(name, "class", line_number));
                }
            }
        }
        "python" => {
            if let Some(name) = trimmed
                .strip_prefix("async def ")
                .or_else(|| trimmed.strip_prefix("def "))
                .and_then(|rest| identifier_at_start(rest))
            {
                symbols.push(symbol(name, "function", line_number));
            }
            if let Some(name) = trimmed
                .strip_prefix("class ")
                .and_then(|rest| identifier_at_start(rest))
            {
                symbols.push(symbol(name, "class", line_number));
            }
        }
        "typescript" | "javascript" => {
            if let Some(name) = word_after_keyword(trimmed, "function") {
                symbols.push(symbol(name, "function", line_number));
            }
            if let Some(name) = word_after_keyword(trimmed, "class") {
                symbols.push(symbol(name, "class", line_number));
            }
            if let Some(name) = const_function_symbol(trimmed) {
                symbols.push(symbol(name, "function", line_number));
            }
            for keyword in ["interface", "type"] {
                if let Some(name) = word_after_keyword(trimmed, keyword) {
                    symbols.push(symbol(name, "class", line_number));
                }
            }
        }
        "markdown" => {
            if let Some(name) = markdown_heading_symbol(trimmed) {
                symbols.push(symbol(name, "heading", line_number));
            }
        }
        _ => {}
    }
}

fn parse_markdown_links_for_line(
    line: &str,
    line_number: usize,
    imports: &mut Vec<CodeGraphImport>,
) {
    let mut rest = line;
    while let Some(label_start) = rest.find('[') {
        let after_label_start = &rest[label_start + 1..];
        let Some(label_end) = after_label_start.find("](") else {
            break;
        };
        let after_link_start = &after_label_start[label_end + 2..];
        let Some(link_end) = after_link_start.find(')') else {
            break;
        };
        let target = after_link_start[..link_end].trim();
        if !target.is_empty()
            && !target.starts_with('#')
            && !target.starts_with("http://")
            && !target.starts_with("https://")
        {
            imports.push(CodeGraphImport {
                target: target.to_string(),
                raw: line.to_string(),
                line: line_number,
                kind: "markdown_link".to_string(),
            });
        }
        rest = &after_link_start[link_end + 1..];
    }
}

fn markdown_heading_symbol(line: &str) -> Option<String> {
    let level = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if level == 0 || level > 6 || !line.chars().nth(level).is_some_and(char::is_whitespace) {
        return None;
    }
    let heading = line[level..].trim().trim_matches('#').trim();
    if heading.is_empty() {
        return None;
    }
    Some(
        heading
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .chars()
            .take(80)
            .collect(),
    )
}

fn collect_source_paths(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    if should_ignore_path(root, directory) {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        let file_type = fs::symlink_metadata(&path)?.file_type();
        if file_type.is_symlink() || should_ignore_path(root, &path) {
            continue;
        }
        if file_type.is_dir() {
            collect_source_paths(root, &path, paths)?;
        } else if file_type.is_file() && is_supported_source_path(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

fn should_ignore_path(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.components().any(|component| {
        let Component::Normal(name) = component else {
            return false;
        };
        let name = name.to_string_lossy();
        matches!(
            name.as_ref(),
            ".git"
                | "target"
                | "node_modules"
                | ".ccc"
                | ".config"
                | ".cache"
                | ".next"
                | ".turbo"
                | "dist"
                | "build"
                | "coverage"
        ) || name.starts_with(".config.")
    })
}

fn is_repo_marker_directory(path: &Path) -> bool {
    path.join(".git").exists()
        || path.join("Cargo.toml").exists()
        || path.join("package.json").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
}

fn is_supported_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "md" | "markdown" | "txt")
    )
}

fn language_for_path(path: &Path) -> String {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => "rust",
        Some("ts" | "tsx") => "typescript",
        Some("js" | "jsx") => "javascript",
        Some("py") => "python",
        Some("md" | "markdown") => "markdown",
        Some("txt") => "text",
        _ => "unknown",
    }
    .to_string()
}

fn import_matches_file(importer_key: &str, import: &CodeGraphImport, target_key: &str) -> bool {
    let import_target = import.target.trim();
    if import_target.is_empty() {
        return false;
    }
    let target_without_ext = strip_known_extension(target_key);
    let normalized_import = normalize_import_target(importer_key, import_target);
    normalized_import == target_without_ext
        || normalized_import == target_key
        || target_without_ext.ends_with(&format!("/{normalized_import}"))
        || module_aliases_for_path(target_key).iter().any(|alias| {
            alias == &normalized_import
                || normalized_import.starts_with(&format!("{alias}/"))
                || import_target.ends_with(alias)
        })
}

fn normalize_import_target(importer_key: &str, target: &str) -> String {
    let target = target
        .trim_matches('"')
        .trim_matches('\'')
        .replace("::", "/")
        .replace('.', "/");
    let target = strip_known_extension(&target);
    if target.starts_with('/') {
        return path_key(Path::new(&target));
    }
    if target.starts_with("./") || target.starts_with("../") {
        let importer_parent = Path::new(importer_key)
            .parent()
            .map(path_key)
            .unwrap_or_default();
        return normalize_relative_key(&importer_parent, &target);
    }
    target
        .trim_start_matches("crate/")
        .trim_start_matches("self/")
        .to_string()
}

fn module_aliases_for_path(path: &str) -> Vec<String> {
    let without_ext = strip_known_extension(path);
    let stem = file_stem_key(path);
    let mut aliases = vec![without_ext];
    if !stem.is_empty() {
        aliases.push(stem);
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn normalize_relative_key(parent: &str, target: &str) -> String {
    let combined = if parent.is_empty() {
        PathBuf::from(target)
    } else {
        Path::new(parent).join(target)
    };
    path_key(&combined)
}

fn path_key(path: &Path) -> String {
    let mut parts = Vec::<String>::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::ParentDir => {
                parts.pop();
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

fn relative_path_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(path_key)
        .unwrap_or_else(|_| path_key(path))
}

fn normalize_query_path(repo_root_key: &str, path: &Path) -> String {
    let key = path_key(path);
    key.strip_prefix(repo_root_key)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(&key)
        .to_string()
}

fn strip_known_extension(path: &str) -> String {
    for extension in [".rs", ".ts", ".tsx", ".js", ".jsx", ".py"] {
        if let Some(stripped) = path.strip_suffix(extension) {
            return stripped.to_string();
        }
    }
    path.to_string()
}

fn file_stem_key(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_string()
}

fn is_test_path(path: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    path.split('/')
        .any(|part| part == "test" || part == "tests")
        || file_name.contains("_test")
        || file_name.contains(".test.")
        || file_name.contains(".spec.")
        || file_name.ends_with("test.rs")
        || file_name.ends_with("tests.rs")
}

fn identifier_refs(content: &str) -> Vec<String> {
    let mut refs = BTreeSet::new();
    let mut current = String::new();
    for character in content.chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == '$' {
            current.push(character);
        } else {
            if current.len() > 1 && !is_noise_identifier(&current) {
                refs.insert(current.clone());
            }
            current.clear();
        }
    }
    if current.len() > 1 && !is_noise_identifier(&current) {
        refs.insert(current);
    }
    refs.into_iter().collect()
}

fn is_noise_identifier(value: &str) -> bool {
    matches!(
        value,
        "use"
            | "mod"
            | "pub"
            | "fn"
            | "let"
            | "mut"
            | "impl"
            | "struct"
            | "enum"
            | "trait"
            | "const"
            | "class"
            | "function"
            | "return"
            | "import"
            | "from"
            | "export"
            | "def"
            | "async"
            | "await"
            | "self"
            | "crate"
            | "super"
            | "true"
            | "false"
            | "None"
            | "null"
            | "undefined"
    )
}

fn word_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let words = line
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && character != '_' && character != '$'
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.windows(2).find_map(|pair| {
        if pair[0] == keyword {
            Some(clean_identifier(pair[1]))
        } else {
            None
        }
    })
}

fn const_function_symbol(line: &str) -> Option<String> {
    let without_export = line.strip_prefix("export ").unwrap_or(line);
    let rest = without_export
        .strip_prefix("const ")
        .or_else(|| without_export.strip_prefix("let "))
        .or_else(|| without_export.strip_prefix("var "))?;
    if !(rest.contains("=>") || rest.contains("function")) {
        return None;
    }
    identifier_at_start(rest)
}

fn identifier_at_start(value: &str) -> Option<String> {
    let name = value
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '_' || *character == '$'
        })
        .collect::<String>();
    if name.is_empty() {
        None
    } else {
        Some(clean_identifier(&name))
    }
}

fn clean_identifier(value: &str) -> String {
    value
        .trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && character != '_' && character != '$'
        })
        .to_string()
}

fn symbol(name: String, kind: &str, line: usize) -> CodeGraphSymbol {
    CodeGraphSymbol {
        name,
        kind: kind.to_string(),
        line,
    }
}

fn symbol_label(symbol: &CodeGraphSymbol) -> String {
    format!("{}:{}", symbol.kind, symbol.name)
}

fn quoted_segment_after_from(value: &str) -> Option<String> {
    let (_, after_from) = value.rsplit_once(" from ")?;
    quoted_segment(after_from)
}

fn quoted_segment(value: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let Some(start) = value.find(quote) else {
            continue;
        };
        let rest = &value[start + 1..];
        if let Some(end) = rest.find(quote) {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn push_relation(relations: &mut Vec<CodeGraphRelation>, path: &str, reasons: BTreeSet<String>) {
    if reasons.is_empty() {
        return;
    }
    relations.push(CodeGraphRelation {
        path: path.to_string(),
        reasons: reasons.into_iter().collect(),
    });
}

fn map_to_relations(values: BTreeMap<String, BTreeSet<String>>) -> Vec<CodeGraphRelation> {
    values
        .into_iter()
        .map(|(path, reasons)| CodeGraphRelation {
            path,
            reasons: reasons.into_iter().collect(),
        })
        .collect()
}

fn dedupe_relations(relations: Vec<CodeGraphRelation>) -> Vec<CodeGraphRelation> {
    let mut merged = BTreeMap::<String, BTreeSet<String>>::new();
    for relation in relations {
        merged
            .entry(relation.path)
            .or_default()
            .extend(relation.reasons);
    }
    map_to_relations(merged)
}

fn score_risk(
    changed_files: &[String],
    impacted_files: &[CodeGraphRelation],
    related_tests: &[CodeGraphRelation],
) -> (u8, String, Vec<String>) {
    let mut score = 10_u8;
    let mut reasons = Vec::new();

    let changed_score = (changed_files.len() as u8).saturating_mul(15).min(30);
    score = score.saturating_add(changed_score);
    reasons.push(format!("{} changed file(s)", changed_files.len()));

    let fanout = impacted_files
        .iter()
        .filter(|relation| !changed_files.contains(&relation.path))
        .count();
    if fanout > 0 {
        score = score.saturating_add((fanout as u8).saturating_mul(5).min(25));
        reasons.push(format!("{fanout} impacted dependent file(s)"));
    }

    if related_tests.is_empty() {
        score = score.saturating_add(15);
        reasons.push("no related tests found".to_string());
    } else {
        reasons.push(format!("{} related test file(s)", related_tests.len()));
    }

    let high_risk_files = changed_files
        .iter()
        .filter(|path| {
            path.ends_with("Cargo.toml")
                || path.ends_with("package.json")
                || path.ends_with("pyproject.toml")
                || path.contains("/main.")
                || path.contains("/lib.")
                || path.contains("/mcp_dispatch.")
        })
        .count();
    if high_risk_files > 0 {
        score = score.saturating_add((high_risk_files as u8).saturating_mul(10).min(20));
        reasons.push(format!("{high_risk_files} high-risk entry/config file(s)"));
    }

    let score = score.min(100);
    let level = if score >= 60 {
        "high"
    } else if score >= 30 {
        "medium"
    } else {
        "low"
    };
    (score, level.to_string(), reasons)
}

fn modified_unix_ms(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub(crate) fn code_graph_store_to_value(store: &CodeGraphStore) -> io::Result<Value> {
    serde_json::to_value(store).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode code graph store value: {error}"),
        )
    })
}
