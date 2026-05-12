use crate::request_routing::combine_request_text_for_routing;
use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Keeps graph/memory state anchored to the actual work target instead of the
// host cwd, which may be a parent folder in Codex App or CLI sessions.
#[derive(Clone, Debug)]
pub(crate) struct TargetWorkspaceResolution {
    pub(crate) root: PathBuf,
    pub(crate) root_kind: String,
    pub(crate) confidence: String,
    pub(crate) confirmation_required: bool,
    pub(crate) reason: String,
    pub(crate) candidates: Vec<PathBuf>,
}

fn is_git_root(path: &Path) -> bool {
    path.join(".git").exists()
}

fn find_git_root_for_target(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    let mut cursor = fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    loop {
        if is_git_root(&cursor) {
            return Some(cursor);
        }
        if !cursor.pop() {
            return None;
        }
    }
}

fn expand_target_path_token(token: &str, workspace_dir: &Path) -> Option<PathBuf> {
    let trimmed = token.trim_matches(|character: char| {
        matches!(
            character,
            '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | ':'
        )
    });
    if trimmed.is_empty() {
        return None;
    }
    let expanded = if let Some(rest) = trimmed.strip_prefix("~/") {
        env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(rest))?
    } else {
        PathBuf::from(trimmed)
    };
    let candidate = if expanded.is_absolute() {
        expanded
    } else if trimmed.contains('/') || trimmed.contains('.') {
        workspace_dir.join(expanded)
    } else {
        return None;
    };
    candidate
        .exists()
        .then(|| fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.to_path_buf()))
}

fn trim_target_path_text(text: &str) -> &str {
    text.trim_matches(|character: char| {
        matches!(
            character,
            '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | ':'
        )
    })
}

fn strip_line_suffix(text: &str) -> &str {
    let trimmed = trim_target_path_text(text.trim());
    if let Some((path, suffix)) = trimmed.rsplit_once(':') {
        if !path.is_empty() && suffix.chars().all(|character| character.is_ascii_digit()) {
            return trim_target_path_text(path);
        }
    }
    trimmed
}

fn candidate_path_from_text(text: &str, workspace_dir: &Path) -> Option<PathBuf> {
    let trimmed = strip_line_suffix(text);
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(rest));
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        Some(candidate)
    } else if trimmed.contains('/') || trimmed.contains('.') {
        Some(workspace_dir.join(candidate))
    } else {
        None
    }
}

fn existing_target_path_from_span(span: &str, workspace_dir: &Path) -> Option<PathBuf> {
    let span = strip_line_suffix(span);
    let direct = candidate_path_from_text(span, workspace_dir)?;
    if direct.exists() {
        return Some(fs::canonicalize(&direct).unwrap_or(direct));
    }

    // Natural-language prompts often append words after a path. Walk backward
    // to the longest existing prefix so paths with spaces still resolve.
    let mut best_directory = None;
    let mut end_indices = span
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    end_indices.push(span.len());
    end_indices.sort_unstable();
    end_indices.dedup();
    for end in end_indices.into_iter().rev() {
        if end == 0 {
            continue;
        }
        let prefix = strip_line_suffix(&span[..end]);
        let Some(candidate) = candidate_path_from_text(prefix, workspace_dir) else {
            continue;
        };
        if !candidate.exists() {
            continue;
        }
        let canonical = fs::canonicalize(&candidate).unwrap_or(candidate);
        if canonical.is_file() {
            return Some(canonical);
        }
        if best_directory.is_none() && canonical.components().count() > 2 {
            best_directory = Some(canonical);
        }
    }
    best_directory
}

fn push_markdown_link_targets(text: &str, spans: &mut BTreeSet<String>) {
    let mut remaining = text;
    while let Some(start) = remaining.find("](") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find(')') else {
            break;
        };
        let target = after_start[..end].trim();
        if !target.is_empty() {
            spans.insert(target.to_string());
        }
        remaining = &after_start[end + 1..];
    }
}

fn is_path_start_boundary(previous: Option<char>) -> bool {
    previous
        .map(|character| {
            character.is_whitespace()
                || matches!(
                    character,
                    '"' | '\'' | '`' | '(' | '[' | '{' | '<' | ':' | '='
                )
        })
        .unwrap_or(true)
}

fn push_path_like_line_spans(text: &str, spans: &mut BTreeSet<String>) {
    for line in text.lines() {
        let chars = line.char_indices().collect::<Vec<_>>();
        for (position, (byte_index, character)) in chars.iter().enumerate() {
            let previous = position
                .checked_sub(1)
                .and_then(|index| chars.get(index).map(|(_, value)| *value));
            let starts_absolute_path = *character == '/' && is_path_start_boundary(previous);
            let starts_home_path = *character == '~'
                && line[*byte_index..].starts_with("~/")
                && is_path_start_boundary(previous);
            if starts_absolute_path || starts_home_path {
                let span = line[*byte_index..].trim();
                if !span.is_empty() {
                    spans.insert(span.to_string());
                }
            }
        }
    }
}

fn request_target_path_spans(text: &str) -> Vec<String> {
    let mut spans = BTreeSet::new();
    push_markdown_link_targets(text, &mut spans);
    push_path_like_line_spans(text, &mut spans);
    for token in text.split_whitespace() {
        spans.insert(token.to_string());
    }
    spans.into_iter().collect()
}

fn push_structured_target_mentions(value: &Value, spans: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            if let Some(path) = path_text_from_structured_value(text) {
                spans.insert(path);
            }
        }
        Value::Array(items) => {
            for item in items {
                push_structured_target_mentions(item, spans);
            }
        }
        Value::Object(object) => {
            // Codex App and CLI hosts can surface attachments under slightly
            // different keys. Keep this alias list path-only so arbitrary text
            // fields do not become target-root evidence.
            for key in [
                "path",
                "file_path",
                "artifact_path",
                "target_path",
                "absolute_path",
                "uri",
            ] {
                if let Some(item) = object.get(key) {
                    push_structured_target_mentions(item, spans);
                }
            }
        }
        _ => {}
    }
}

fn path_text_from_structured_value(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    if path.starts_with('/')
        || path.starts_with("~/")
        || path.contains('/')
        || path.contains('\\')
        || path.contains('.')
    {
        Some(path.to_string())
    } else {
        None
    }
}

fn structured_target_path_spans(parsed: &Value) -> Vec<String> {
    let mut spans = BTreeSet::new();
    for field in [
        "target_paths",
        "file_paths",
        "artifact_paths",
        "mentioned_files",
        "input_items",
        "items",
    ] {
        if let Some(value) = parsed.get(field) {
            push_structured_target_mentions(value, &mut spans);
        }
        if let Some(value) = parsed.pointer(&format!("/structured_target_mentions/{field}")) {
            push_structured_target_mentions(value, &mut spans);
        }
    }
    spans.into_iter().collect()
}

fn collect_explicit_target_paths(workspace_dir: &Path, parsed: &Value) -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();
    for span in structured_target_path_spans(parsed) {
        if let Some(path) = existing_target_path_from_span(&span, workspace_dir)
            .or_else(|| expand_target_path_token(&span, workspace_dir))
        {
            paths.insert(path);
        }
    }

    let request_text = combine_request_text_for_routing(parsed);
    for span in request_target_path_spans(&request_text) {
        if let Some(path) = existing_target_path_from_span(&span, workspace_dir)
            .or_else(|| expand_target_path_token(&span, workspace_dir))
        {
            paths.insert(path);
        }
    }
    paths.into_iter().collect()
}

fn immediate_child_git_roots(workspace_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(workspace_dir) else {
        return Vec::new();
    };
    let mut roots = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && is_git_root(path))
        .map(|path| fs::canonicalize(&path).unwrap_or(path))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
}

fn target_document_root(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

fn common_path_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut common = paths.first()?.clone();
    for path in paths.iter().skip(1) {
        while !path.starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    Some(common)
}

fn document_bundle_common_root(
    document_roots: &[PathBuf],
    workspace_dir: &Path,
) -> Option<PathBuf> {
    if document_roots.len() < 2 {
        return None;
    }

    // Non-git document work often names several files from one bundle, such as
    // a release folder with notes and assets. Collapse sibling document roots
    // to that bundle parent, but avoid treating the host cwd itself as a
    // meaningful bundle when unrelated folders are mentioned.
    let common = common_path_ancestor(document_roots)?;
    if common == workspace_dir || common.components().count() < 2 {
        return None;
    }
    Some(common)
}

fn remove_ancestor_target_candidates(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other != *candidate
                    && other.starts_with(candidate)
                    && other.components().count() > candidate.components().count()
            })
        })
        .cloned()
        .collect()
}

pub(crate) fn resolve_target_workspace_root(
    workspace_dir: &Path,
    parsed: &Value,
) -> TargetWorkspaceResolution {
    let workspace_dir =
        fs::canonicalize(workspace_dir).unwrap_or_else(|_| workspace_dir.to_path_buf());
    let target_paths = collect_explicit_target_paths(&workspace_dir, parsed);
    if !target_paths.is_empty() {
        let mut candidates = BTreeSet::new();
        let mut saw_document_root = false;
        let mut saw_git_root = false;
        let mut document_roots = Vec::new();
        for path in &target_paths {
            if let Some(root) = find_git_root_for_target(path) {
                saw_git_root = true;
                candidates.insert(root);
            } else {
                saw_document_root = true;
                let root = target_document_root(path);
                candidates.insert(root.clone());
                document_roots.push(root);
            }
        }
        let candidates = remove_ancestor_target_candidates(candidates.into_iter().collect());
        if !saw_git_root {
            if let Some(root) = document_bundle_common_root(&document_roots, &workspace_dir) {
                return TargetWorkspaceResolution {
                    root: root.clone(),
                    root_kind: "document_root".to_string(),
                    confidence: "medium".to_string(),
                    confirmation_required: false,
                    reason: "Resolved from the common parent of explicit document targets."
                        .to_string(),
                    candidates: vec![root],
                };
            }
        }
        if candidates.len() == 1 {
            let root = candidates[0].clone();
            let root_kind = if is_git_root(&root) {
                "git_repo"
            } else {
                "document_root"
            };
            return TargetWorkspaceResolution {
                root,
                root_kind: root_kind.to_string(),
                confidence: if saw_document_root { "medium" } else { "high" }.to_string(),
                confirmation_required: false,
                reason: "Resolved from explicit target path in the request.".to_string(),
                candidates,
            };
        }
        return TargetWorkspaceResolution {
            root: workspace_dir,
            root_kind: "ambiguous_target".to_string(),
            confidence: "low".to_string(),
            confirmation_required: true,
            reason: "Request mentions multiple target roots; ask the operator to confirm the intended repo or document root.".to_string(),
            candidates,
        };
    }

    if let Some(root) = find_git_root_for_target(&workspace_dir) {
        return TargetWorkspaceResolution {
            root,
            root_kind: "git_repo".to_string(),
            confidence: "high".to_string(),
            confirmation_required: false,
            reason: "Resolved from current workspace git root.".to_string(),
            candidates: Vec::new(),
        };
    }

    let child_repos = immediate_child_git_roots(&workspace_dir);
    match child_repos.as_slice() {
        [root] => TargetWorkspaceResolution {
            root: root.clone(),
            root_kind: "single_child_git_repo".to_string(),
            confidence: "medium".to_string(),
            confirmation_required: false,
            reason: "Current workspace is not a git repo, but it contains one child git repo.".to_string(),
            candidates: child_repos,
        },
        [] => TargetWorkspaceResolution {
            root: workspace_dir,
            root_kind: "cwd_fallback".to_string(),
            confidence: "low".to_string(),
            confirmation_required: false,
            reason: "No explicit target path or git repo root was detected; using cwd fallback.".to_string(),
            candidates: Vec::new(),
        },
        _ => TargetWorkspaceResolution {
            root: workspace_dir,
            root_kind: "ambiguous_child_git_repos".to_string(),
            confidence: "low".to_string(),
            confirmation_required: true,
            reason: "Current workspace is not a git repo and contains multiple child git repos; ask the operator to choose the target path.".to_string(),
            candidates: child_repos,
        },
    }
}
