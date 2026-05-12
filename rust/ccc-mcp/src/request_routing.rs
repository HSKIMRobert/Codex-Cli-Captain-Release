use crate::skill_registry::load_skill_registry_for_agent;
use crate::specialist_roles::{
    agent_id_for_role, generated_custom_agent_name, load_role_config_snapshot_from_config,
    load_shared_ccc_config, normalize_dispatch_role_hint, role_for_agent_id,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

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

fn has_ascii_word_or_phrase(request: &str, phrase: &str) -> bool {
    let normalized_request = normalized_ascii_search_text(request);
    let normalized_phrase = normalized_ascii_search_text(phrase);
    !normalized_phrase.trim().is_empty()
        && normalized_request.contains(&format!(" {} ", normalized_phrase.trim()))
}

pub(crate) fn combine_request_text_for_routing(parsed: &Value) -> String {
    [
        parsed.get("request").and_then(Value::as_str),
        parsed.get("goal").and_then(Value::as_str),
        parsed.get("title").and_then(Value::as_str),
        parsed.get("intent").and_then(Value::as_str),
        parsed.get("scope").and_then(Value::as_str),
        parsed.get("prompt").and_then(Value::as_str),
        parsed.get("acceptance").and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn infer_task_shape(request: &str, request_shape: &str) -> &'static str {
    let file_path_mentions = request.matches('.').count();
    if request_shape == "way" || request_shape == "mutation" || file_path_mentions >= 3 {
        "multi_step_or_unclear"
    } else {
        "single_scoped_task"
    }
}

fn has_visibility_diagnostic_signal(request: &str, normalized: &str) -> bool {
    let has_explicit_visibility_signal = [
        "smoke",
        "smoke test",
        "diagnostic",
        "diagnostics",
        "visibility",
        "app visibility",
        "app_panel",
        "app-panel",
        "status output",
        "check-install",
        "server identity",
        "install surface",
        "health check",
        "스모크",
        "진단",
        "가시성",
        "상태 확인",
        "설치 상태",
        "보이는지",
        "정상적으로 보이는지",
    ]
    .iter()
    .any(|keyword| request.contains(keyword) || normalized.contains(keyword));
    if has_explicit_visibility_signal {
        return true;
    }

    let has_lifecycle_context = [
        "cli",
        "installed",
        "install",
        "binary",
        "lifecycle",
        "quiet",
    ]
    .iter()
    .any(|keyword| has_ascii_word_or_phrase(request, keyword));
    let has_lifecycle_command_signal = [
        "start",
        "status",
        "checklist",
        "orchestrate",
        "subagent update",
    ]
    .iter()
    .any(|keyword| has_ascii_word_or_phrase(request, keyword));
    let has_ccc_lifecycle_command = [
        "ccc start",
        "ccc status",
        "ccc checklist",
        "ccc orchestrate",
        "ccc subagent update",
    ]
    .iter()
    .any(|keyword| has_ascii_word_or_phrase(request, keyword));
    has_ccc_lifecycle_command || (has_lifecycle_context && has_lifecycle_command_signal)
}

fn infer_requested_tool_names(request: &str) -> Vec<&'static str> {
    let normalized = request.to_ascii_lowercase();
    let mut names = Vec::new();

    let mentions_openai_docs = normalized.contains("openai docs")
        || normalized.contains("platform.openai.com")
        || normalized.contains("developers.openai.com")
        || normalized.contains("responses api")
        || normalized.contains("chatgpt apps sdk");
    let mentions_context7 = normalized.contains("context7")
        || normalized.contains("library docs")
        || normalized.contains("framework docs")
        || normalized.contains("package docs");
    let mentions_fetch = normalized.contains("http://")
        || normalized.contains("https://")
        || normalized.contains("fetch ")
        || normalized.contains("fetch:")
        || normalized.contains("url ")
        || normalized.contains("web page")
        || normalized.contains("webpage")
        || normalized.contains("website");
    let mentions_gh = normalized == "gh"
        || normalized.contains("gh ")
        || normalized.contains("gh:")
        || normalized.contains("github")
        || normalized.contains("pull request")
        || normalized.contains("pr ")
        || request.contains("깃허브")
        || request.contains("プルリク")
        || request.contains("拉取请求");
    let mentions_git = normalized.contains("git ")
        || normalized.contains("git:")
        || request.contains("깃")
        || [
            "commit",
            "branch",
            "diff",
            "log",
            "blame",
            "checkout",
            "rebase",
            "tag",
            "staged",
            "unstaged",
            "git tag",
            "release tag",
            "tag release",
        ]
        .iter()
        .any(|phrase| has_ascii_word_or_phrase(request, phrase));
    let mentions_filesystem = normalized.contains("current directory")
        || normalized.contains("current dir")
        || normalized.contains("workspace")
        || normalized.contains("repo tree")
        || request.contains("현재 디렉토리")
        || request.contains("작업공간")
        || request.contains("파일")
        || request.contains("ディレクトリ")
        || request.contains("ファイル")
        || request.contains("目录")
        || request.contains("文件")
        || normalized.contains("read file")
        || normalized.contains("read the file")
        || normalized.contains("files ")
        || normalized.contains("file ")
        || normalized.contains("directory")
        || normalized.contains("directories")
        || normalized.contains("folder")
        || normalized.contains("tree")
        || normalized.contains("search files")
        || normalized.contains("list directory")
        || normalized.contains("find file")
        || normalized.contains("path ")
        || normalized.contains("install.sh")
        || normalized.contains("install.ps1")
        || normalized.contains("build-release-asset")
        || normalized.contains("verify-release-asset")
        || normalized.contains("scripts/release");

    if mentions_filesystem {
        names.push("filesystem");
    }
    if mentions_gh {
        names.push("gh");
    }
    if mentions_git {
        names.push("git");
    }
    if mentions_context7 {
        names.push("context7");
    }
    if mentions_fetch {
        names.push("fetch");
    }
    if mentions_openai_docs {
        names.push("openaiDeveloperDocs");
    }

    names
}

fn has_explicit_gh_release_mutation_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    ["upload", "edit", "create", "delete"].iter().any(|verb| {
        normalized.contains(&format!("gh release {verb}"))
            || normalized.contains(&format!("github release {verb}"))
            || normalized.contains(&format!("{verb} github release"))
    })
}

fn has_release_install_script_repair_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "install.sh",
        "install.ps1",
        "install script",
        "installer script",
        "build-release-asset",
        "verify-release-asset",
        "release asset packaging",
        "release asset build",
        "release packaging script",
        "asset packaging script",
        "scripts/release",
    ]
    .iter()
    .any(|signal| normalized.contains(signal))
}

fn infer_tool_operation(request: &str, tool_names: &[&str], mutation_intent: &str) -> &'static str {
    let normalized = request.to_ascii_lowercase();
    let release_install_repair_guard = has_release_install_script_repair_signal(request)
        && !has_explicit_gh_release_mutation_signal(request);
    let git_mutation_signals = [
        "commit",
        "git stage",
        "stage files",
        "stage changes",
        "unstage",
        "checkout",
        "rebase",
        "merge",
        "reset",
        "git tag",
        "release tag",
        "tag release",
        "push",
    ]
    .iter()
    .any(|phrase| has_ascii_word_or_phrase(request, phrase))
        || (normalized.contains("release") && !release_install_repair_guard)
        || has_explicit_gh_release_mutation_signal(request)
        || request.contains("커밋")
        || request.contains("푸시")
        || request.contains("릴리즈")
        || request.contains("コミット")
        || request.contains("プッシュ")
        || request.contains("リリース")
        || request.contains("提交")
        || request.contains("推送")
        || request.contains("发布");
    if (tool_names.contains(&"git") || tool_names.contains(&"gh"))
        && (git_mutation_signals
            || (mutation_intent == "explicit_or_strong" && !release_install_repair_guard))
    {
        "mutation"
    } else {
        "read"
    }
}

fn route_class_for_tools(tool_names: &[&str], operation: &str) -> &'static str {
    if tool_names.is_empty() {
        "none"
    } else if tool_names.len() > 1 {
        let docs_only = tool_names
            .iter()
            .all(|name| matches!(*name, "context7" | "fetch" | "openaiDeveloperDocs"));
        if docs_only {
            "docs_lookup"
        } else {
            "multi_source_evidence"
        }
    } else {
        match (tool_names[0], operation) {
            ("filesystem", _) => "workspace_inspection",
            ("gh", "mutation") => "git_mutation",
            ("gh", _) => "git_inspection",
            ("git", "mutation") => "git_mutation",
            ("git", _) => "git_inspection",
            ("context7", _) | ("fetch", _) | ("openaiDeveloperDocs", _) => "docs_lookup",
            _ => "multi_source_evidence",
        }
    }
}

pub(crate) fn default_tool_routing_config() -> Value {
    json!({
        "default_model": "gpt-5.4-mini",
        "default_variant": "high",
        "fallback_mode": "visible_degraded_host_fallback",
        "tools": {
            "filesystem": {
                "allowed_operations": ["read"],
                "owner_companion_agent": "companion_reader"
            },
            "fetch": {
                "allowed_operations": ["read"],
                "owner_companion_agent": "companion_reader"
            },
            "context7": {
                "allowed_operations": ["read"],
                "owner_companion_agent": "companion_reader"
            },
            "openaiDeveloperDocs": {
                "allowed_operations": ["read"],
                "owner_companion_agent": "companion_reader"
            },
            "git": {
                "allowed_operations": ["read", "mutation"],
                "owner_companion_agent": "companion_reader",
                "mutation_owner_companion_agent": "companion_operator"
            },
            "gh": {
                "allowed_operations": ["read", "mutation"],
                "owner_companion_agent": "companion_reader",
                "mutation_owner_companion_agent": "companion_operator",
                "model": "gpt-5.4-mini",
                "variant": "high"
            }
        }
    })
}

pub(crate) fn default_routing_config() -> Value {
    json!({
        "mode": "category_shortlist",
        "categories": {
            "read_repo": {
                "keywords": ["read", "inspect", "trace", "find", "analyze", "why", "where", "status", "summary"],
                "intent_types": ["read_only", "diagnosis"],
                "tool_signals": ["filesystem"],
                "agents": ["scout", "companion_reader"]
            },
            "write_code": {
                "keywords": ["fix", "patch", "implement", "update", "change", "repair"],
                "intent_types": ["mutation"],
                "tool_signals": ["filesystem", "git"],
                "agents": ["raider", "companion_operator"]
            },
            "write_docs": {
                "keywords": ["docs", "readme", "release note", "document", "skill", "translate", "translation", "localize", "번역"],
                "intent_types": ["documentation"],
                "tool_signals": ["filesystem"],
                "agents": ["scribe", "scout"]
            },
            "verify": {
                "keywords": ["verify", "review", "test", "validate", "check"],
                "intent_types": ["review", "validation"],
                "tool_signals": ["filesystem"],
                "agents": ["arbiter", "scout"]
            },
            "ownership": {
                "keywords": ["ownership", "boundary", "owner", "route", "classify"],
                "intent_types": ["ownership"],
                "tool_signals": ["filesystem", "git"],
                "agents": ["sentinel", "scout"]
            }
        }
    })
}

pub(crate) fn load_tool_routing_policy() -> Value {
    load_shared_ccc_config()
        .ok()
        .and_then(|config| config.get("tool_routing").cloned())
        .unwrap_or_else(default_tool_routing_config)
}

fn effective_tool_owner_role(policy_entry: &Value, operation: &str) -> Option<String> {
    if operation == "mutation" {
        policy_entry
            .get("mutation_owner_companion_agent")
            .and_then(Value::as_str)
            .or_else(|| {
                policy_entry
                    .get("mutation_owner_role")
                    .and_then(Value::as_str)
            })
            .map(str::to_string)
    } else {
        policy_entry
            .get("owner_companion_agent")
            .and_then(Value::as_str)
            .or_else(|| policy_entry.get("owner_role").and_then(Value::as_str))
            .map(str::to_string)
    }
}

pub(crate) fn create_companion_tool_route_payload(request: &str, mutation_intent: &str) -> Value {
    let tool_routing = load_tool_routing_policy();
    create_companion_tool_route_payload_for_policy(&tool_routing, request, mutation_intent)
}

pub(crate) fn create_companion_tool_route_payload_for_policy(
    tool_routing: &Value,
    request: &str,
    mutation_intent: &str,
) -> Value {
    let tool_names = infer_requested_tool_names(request);
    if tool_names.is_empty() {
        return json!({
            "route_class": "none",
            "tool_names": [],
            "operation": "none",
            "owner_role": Value::Null,
            "model": Value::Null,
            "variant": Value::Null,
            "fallback_mode": "visible_degraded_host_fallback",
            "execution_state": "not_applicable",
        });
    }

    let operation = infer_tool_operation(request, &tool_names, mutation_intent);
    let default_model = tool_routing
        .get("default_model")
        .cloned()
        .unwrap_or(Value::Null);
    let default_variant = tool_routing
        .get("default_variant")
        .cloned()
        .unwrap_or(Value::Null);
    let global_fallback_mode = tool_routing
        .get("fallback_mode")
        .and_then(Value::as_str)
        .unwrap_or("visible_degraded_host_fallback")
        .to_string();

    let selected_entries = tool_routing
        .get("tools")
        .and_then(Value::as_object)
        .map(|tools| {
            tool_names
                .iter()
                .filter_map(|tool_name| {
                    let entry = tools.get(*tool_name)?;
                    let allowed = entry
                        .get("allowed_operations")
                        .and_then(Value::as_array)
                        .map(|operations| {
                            operations
                                .iter()
                                .filter_map(Value::as_str)
                                .any(|candidate| candidate == operation)
                        })
                        .unwrap_or(false);
                    allowed.then(|| (*tool_name, entry.clone()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if selected_entries.is_empty() {
        return json!({
            "route_class": "none",
            "tool_names": [],
            "operation": "none",
            "owner_role": Value::Null,
            "model": Value::Null,
            "variant": Value::Null,
            "fallback_mode": global_fallback_mode,
            "execution_state": "degraded_host_fallback",
        });
    }

    let routed_tool_names = selected_entries
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    let route_class = route_class_for_tools(&routed_tool_names, operation);
    let effective_owner_roles = selected_entries
        .iter()
        .filter_map(|(_, entry)| effective_tool_owner_role(entry, operation))
        .collect::<Vec<_>>();
    let owner_role = if effective_owner_roles.is_empty() {
        None
    } else {
        let first = effective_owner_roles[0].clone();
        if effective_owner_roles
            .iter()
            .all(|candidate| *candidate == first)
        {
            Some(first)
        } else {
            Some("companion_reader".to_string())
        }
    };
    let owner_role = owner_role.filter(|value| !value.trim().is_empty());

    let first_entry = &selected_entries[0].1;
    let model = first_entry
        .get("model")
        .cloned()
        .unwrap_or_else(|| default_model.clone());
    let variant = first_entry
        .get("variant")
        .cloned()
        .unwrap_or_else(|| default_variant.clone());
    let fallback_mode = first_entry
        .get("fallback_mode")
        .and_then(Value::as_str)
        .unwrap_or(&global_fallback_mode)
        .to_string();

    json!({
        "route_class": route_class,
        "tool_names": selected_entries.iter().map(|(name, _)| Value::String((*name).to_string())).collect::<Vec<_>>(),
        "operation": operation,
        "owner_role": owner_role,
        "model": model,
        "variant": variant,
        "fallback_mode": fallback_mode,
        "execution_state": "route_backed_specialist_owned",
    })
}

fn infer_intent_types(request: &str, request_shape: &str) -> Vec<String> {
    let normalized = request.to_ascii_lowercase();
    let mut intents = BTreeSet::new();
    match request_shape {
        "mutation" => {
            intents.insert("mutation".to_string());
        }
        "lookup" | "diagnostic" => {
            intents.insert("read_only".to_string());
        }
        _ => {}
    }
    if request_shape == "diagnostic" {
        intents.insert("diagnosis".to_string());
    }
    if request_shape == "review" {
        intents.insert("review".to_string());
        intents.insert("validation".to_string());
        intents.insert("verification".to_string());
    }
    if request_shape == "way" {
        intents.insert("planning".to_string());
    }
    if [
        "docs",
        "readme",
        "release note",
        "document",
        "documentation",
        "translate",
        "translation",
        "localize",
        "문서",
        "번역",
        "읽어보기",
        "ドキュメント",
        "翻訳",
        "文書",
        "文档",
        "翻译",
    ]
    .iter()
    .any(|keyword| request.contains(keyword) || normalized.contains(keyword))
    {
        intents.insert("documentation".to_string());
    }
    if [
        "ownership",
        "owner",
        "responsible",
        "ownership drift",
        "sentinel",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
    {
        intents.insert("ownership".to_string());
    }
    if [
        "inspect", "analyze", "analysis", "trace", "find", "why", "where", "status", "summary",
        "분석", "확인", "점검", "요약", "調査", "確認", "分析", "検査", "检查", "总结",
    ]
    .iter()
    .any(|keyword| request.contains(keyword) || normalized.contains(keyword))
    {
        intents.insert("diagnosis".to_string());
    }
    intents.into_iter().collect()
}

pub(crate) fn create_specialist_shortlist_payload_from_config(
    config: &Value,
    request: &str,
    fallback_role: &str,
    route_owner_role: Option<&str>,
) -> Value {
    let routing = config
        .get("routing")
        .cloned()
        .unwrap_or_else(default_routing_config);
    let mode = routing
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("disabled");
    if mode != "category_shortlist" {
        return json!({
            "mode": mode,
            "selected_category": Value::Null,
            "selected_role": Value::Null,
            "selected_agent_id": Value::Null,
            "summary": "Category shortlist routing is disabled.",
            "candidates": [],
            "categories_considered": [],
        });
    }

    let request_shape = infer_request_shape(request);
    let intent_types = infer_intent_types(request, request_shape);
    let tool_signals = infer_requested_tool_names(request)
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let normalized_request = request.to_ascii_lowercase();
    let categories = routing
        .get("categories")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut considered = categories
        .into_iter()
        .filter_map(|(category_name, entry)| {
            let keywords = entry
                .get("keywords")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|value| value.to_ascii_lowercase())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let matched_keywords = keywords
                .iter()
                .filter(|keyword| normalized_request.contains(keyword.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let configured_intents = entry
                .get("intent_types")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let matched_intents = configured_intents
                .iter()
                .filter(|intent| intent_types.iter().any(|candidate| candidate == *intent))
                .cloned()
                .collect::<Vec<_>>();
            let configured_tool_signals = entry
                .get("tool_signals")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let matched_tool_signals = configured_tool_signals
                .iter()
                .filter(|tool| tool_signals.iter().any(|candidate| candidate == *tool))
                .cloned()
                .collect::<Vec<_>>();
            let candidate_agents = entry
                .get("agents")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if candidate_agents.is_empty() {
                return None;
            }
            let score = matched_intents.len() * 4
                + matched_keywords.len() * 3
                + matched_tool_signals.len() * 2;
            let candidates = candidate_agents
                .iter()
                .map(|agent_id| {
                    let normalized_agent_id = agent_id.trim().strip_prefix("ccc_").unwrap_or(agent_id);
                    let role = role_for_agent_id(agent_id)
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            normalize_dispatch_role_hint(Some(agent_id), fallback_role)
                        });
                    let snapshot = load_role_config_snapshot_from_config(config, &role);
                    let registry =
                        create_registry_routing_evidence(normalized_agent_id, &snapshot);
                    json!({
                        "agent_id": agent_id,
                        "role": role,
                        "summary": snapshot.get("summary").cloned().unwrap_or(Value::Null),
                        "model": snapshot.get("model").cloned().unwrap_or(Value::Null),
                        "variant": snapshot.get("variant").cloned().unwrap_or(Value::Null),
                        "routing_evidence_source": registry.get("routing_evidence_source").cloned().unwrap_or(Value::String("fallback_heuristic".to_string())),
                        "skill_registry": registry,
                    })
                })
                .collect::<Vec<_>>();
            Some(json!({
                "category": category_name,
                "score": score,
                "matched_keywords": matched_keywords,
                "matched_intents": matched_intents,
                "matched_tool_signals": matched_tool_signals,
                "candidates": candidates,
            }))
        })
        .collect::<Vec<_>>();
    considered.sort_by(|left, right| {
        let left_score = left.get("score").and_then(Value::as_u64).unwrap_or(0);
        let right_score = right.get("score").and_then(Value::as_u64).unwrap_or(0);
        right_score.cmp(&left_score).then_with(|| {
            left.get("category")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .cmp(
                    right
                        .get("category")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
        })
    });

    let best = considered
        .iter()
        .find(|entry| entry.get("score").and_then(Value::as_u64).unwrap_or(0) > 0);
    let shortlist = best
        .and_then(|entry| entry.get("candidates"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let shortlist_roles = shortlist
        .iter()
        .filter_map(|candidate| candidate.get("role").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let selected_role = route_owner_role
        .filter(|role| shortlist_roles.iter().any(|candidate| candidate == role))
        .map(str::to_string)
        .or_else(|| {
            shortlist_roles
                .iter()
                .find(|candidate| **candidate == fallback_role)
                .map(|role| (*role).to_string())
        })
        .or_else(|| shortlist_roles.first().map(|role| (*role).to_string()));
    let selected_agent_id = selected_role
        .as_ref()
        .and_then(|role| agent_id_for_role(role));
    let selected_candidate = selected_role.as_ref().and_then(|role| {
        shortlist
            .iter()
            .find(|candidate| candidate.get("role").and_then(Value::as_str) == Some(role))
    });
    let selected_routing_evidence_source = selected_candidate
        .and_then(|candidate| {
            candidate
                .get("routing_evidence_source")
                .and_then(Value::as_str)
        })
        .unwrap_or("fallback_heuristic");
    let selected_display_metadata_sources = selected_candidate
        .map(create_selected_display_metadata_sources)
        .unwrap_or_else(|| {
            json!({
                "agent": "fallback_heuristic",
                "model": "fallback_heuristic",
                "variant": "fallback_heuristic",
                "reasoning": "fallback_heuristic",
            })
        });
    let selected_category = best
        .and_then(|entry| entry.get("category"))
        .cloned()
        .unwrap_or(Value::Null);
    let summary = if let (Some(category), Some(role)) =
        (selected_category.as_str(), selected_role.as_deref())
    {
        let candidate_list = shortlist
            .iter()
            .filter_map(|candidate| candidate.get("agent_id").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Category shortlist matched {category} and selected {role} from [{candidate_list}]."
        )
    } else {
        format!("Category shortlist found no confident match; fell back to {fallback_role}.")
    };

    json!({
        "mode": mode,
        "request_shape": request_shape,
        "intent_types": intent_types,
        "tool_signals": tool_signals,
        "selected_category": selected_category,
        "selected_role": selected_role,
        "selected_agent_id": selected_agent_id,
        "selected_routing_evidence_source": selected_routing_evidence_source,
        "selected_display_metadata_sources": selected_display_metadata_sources,
        "summary": summary,
        "candidates": shortlist,
        "categories_considered": considered.into_iter().take(3).collect::<Vec<_>>(),
    })
}

fn create_registry_routing_evidence(agent_id: &str, role_config_snapshot: &Value) -> Value {
    // Registry evidence is advisory: Router can cite manifest Scheduling data,
    // but missing or stale sidecars must keep the existing heuristic path alive.
    let custom_agent_name = generated_custom_agent_name(agent_id);
    let registry = load_skill_registry_for_agent(&custom_agent_name, role_config_snapshot);
    let registry_status = registry
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let scheduling = registry
        .get("skill_ssl_manifest")
        .and_then(|manifest| manifest.get("scheduling"))
        .cloned()
        .unwrap_or(Value::Null);
    let source = if registry_status == "available" && scheduling.is_object() {
        "skill_registry"
    } else {
        "fallback_heuristic"
    };

    json!({
        "source": "skill_registry",
        "routing_evidence_source": source,
        "status": registry_status,
        "agent_name": custom_agent_name,
        "scheduling": scheduling,
    })
}

fn create_selected_display_metadata_sources(candidate: &Value) -> Value {
    let registry_available = candidate
        .get("routing_evidence_source")
        .and_then(Value::as_str)
        == Some("skill_registry");
    let agent_source = if registry_available {
        "skill_registry"
    } else {
        "fallback_heuristic"
    };
    json!({
        "agent": agent_source,
        "model": "role_config",
        "variant": "role_config",
        "reasoning": "role_config",
    })
}

fn selected_skill_payload_for_role(role: &str) -> Value {
    let agent_id = agent_id_for_role(role).unwrap_or(role);
    let custom_agent_name = generated_custom_agent_name(agent_id);
    let role_config = load_role_config_snapshot_from_config(
        &load_shared_ccc_config().unwrap_or(Value::Null),
        role,
    );
    let registry = load_skill_registry_for_agent(&custom_agent_name, &role_config);
    let manifest = registry
        .get("skill_ssl_manifest")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or(Value::Null);
    let skill_id = manifest
        .get("skill_id")
        .and_then(Value::as_str)
        .unwrap_or(custom_agent_name.as_str());

    json!({
        "id": skill_id,
        "name": custom_agent_name.clone(),
        "display_agent_id": manifest.pointer("/scheduling/display_agent_id").cloned().unwrap_or_else(|| Value::String(custom_agent_name.clone())),
        "role_family": manifest.pointer("/scheduling/role_family").cloned().unwrap_or(Value::Null),
        "registry_status": registry.get("status").cloned().unwrap_or(Value::String("missing".to_string())),
        "mutation_allowed": manifest.pointer("/scheduling/mutation_allowed").cloned().unwrap_or(Value::Null),
        "expected_outputs": manifest.pointer("/scheduling/expected_outputs").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "logical_actions": manifest.pointer("/logical/actions").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    })
}

fn risk_rank(risk: &str) -> u8 {
    match risk {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "none" => 1,
        _ => 0,
    }
}

fn infer_selected_risk(selected_skill: &Value, mutation_intent: &str) -> &'static str {
    let default_risk = if mutation_intent == "explicit_or_strong" {
        "medium"
    } else {
        "low"
    };
    let mut risk = "none";
    let mut saw_action_risk = false;
    if let Some(actions) = selected_skill
        .get("logical_actions")
        .and_then(Value::as_array)
    {
        for action in actions {
            if let Some(action_risk) = action.get("risk").and_then(Value::as_str) {
                saw_action_risk = true;
                if risk_rank(action_risk) > risk_rank(risk) {
                    risk = match action_risk {
                        "critical" => "critical",
                        "high" => "high",
                        "medium" => "medium",
                        "low" => "low",
                        "none" => "none",
                        _ => risk,
                    };
                }
            }
        }
    }
    if saw_action_risk {
        risk
    } else {
        default_risk
    }
}

fn infer_evidence_need(selected_skill: &Value, request_shape: &str) -> &'static str {
    let expected_outputs = selected_skill
        .get("expected_outputs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if expected_outputs
        .iter()
        .any(|value| matches!(*value, "findings" | "review_outcome" | "residual_risk"))
        || request_shape == "review"
    {
        "findings_and_acceptance_evidence"
    } else if expected_outputs
        .iter()
        .any(|value| matches!(*value, "validation_results" | "changed_files"))
    {
        "changed_files_and_validation_results"
    } else {
        "bounded_summary_and_evidence_paths"
    }
}

fn infer_verification_need(role: &str, risk: &str, mutation_intent: &str) -> &'static str {
    if role == "verifier" {
        "review_judgment_required"
    } else if mutation_intent == "explicit_or_strong"
        || matches!(risk, "medium" | "high" | "critical")
    {
        "focused_validation_required"
    } else {
        "evidence_only"
    }
}

fn selected_specialist_reason(
    selected_category: Option<&str>,
    selected_role: &str,
    selected_agent_id: &str,
    companion_route_enforced: bool,
    release_install_repair_guard: bool,
    specialist_route: &Value,
) -> String {
    if companion_route_enforced {
        return format!(
            "Companion tool ownership selected {selected_role}/{selected_agent_id} for a mutation route."
        );
    }
    if release_install_repair_guard {
        return format!(
            "Release/install script repair stayed with {selected_role}/{selected_agent_id} instead of GitHub mutation ownership."
        );
    }
    if let Some(category) = selected_category {
        return format!(
            "Category {category} selected {selected_role}/{selected_agent_id} from the shortlist."
        );
    }
    specialist_route
        .get("summary")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("Fallback selected {selected_role}/{selected_agent_id}."))
}

fn create_internal_routing_protocol_payload(
    selected_category: Value,
    selected_skill: Value,
    risk: &str,
    evidence_need: &str,
    verification_need: &str,
    mutation_intent: &str,
    selected_role: &str,
    selected_agent_id: &str,
    reason: &str,
) -> Value {
    // This is an internal status/debug contract, not a public command surface.
    json!({
        "schema": "ccc.internal_routing.v1",
        "selected_category": selected_category,
        "selected_skill": selected_skill,
        "risk": risk,
        "evidence_need": evidence_need,
        "verification_need": verification_need,
        "mutation_intent": mutation_intent,
        "selected_role": selected_role,
        "selected_agent_id": selected_agent_id,
        "reason": reason,
    })
}

fn create_specialist_shortlist_payload(
    request: &str,
    fallback_role: &str,
    route_owner_role: Option<&str>,
) -> Value {
    let config = load_shared_ccc_config().unwrap_or(Value::Null);
    create_specialist_shortlist_payload_from_config(
        &config,
        request,
        fallback_role,
        route_owner_role,
    )
}

pub(crate) fn create_routing_trace_payload(request: &str, fallback_role: &str) -> Value {
    let request_shape = infer_request_shape(request);
    let mutation_intent = infer_mutation_intent(request_shape);
    let tool_route = create_companion_tool_route_payload(request, mutation_intent);
    let release_install_repair_guard = mutation_intent == "explicit_or_strong"
        && has_release_install_script_repair_signal(request)
        && !has_explicit_gh_release_mutation_signal(request);
    let route_owner_role = if release_install_repair_guard {
        None
    } else {
        tool_route.get("owner_role").and_then(Value::as_str)
    };
    let route_execution_state = tool_route
        .get("execution_state")
        .and_then(Value::as_str)
        .unwrap_or("not_applicable");
    let route_operation = tool_route
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let specialist_route =
        create_specialist_shortlist_payload(request, fallback_role, route_owner_role);
    let selected_role = (route_execution_state == "route_backed_specialist_owned"
        && route_operation == "mutation")
        .then_some(route_owner_role)
        .flatten()
        .map(str::to_string)
        .or_else(|| {
            specialist_route
                .get("selected_role")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            if route_execution_state == "route_backed_specialist_owned" {
                route_owner_role.map(str::to_string)
            } else {
                None
            }
        })
        .or_else(|| match (fallback_role, route_owner_role) {
            ("explorer", Some("companion_reader")) => Some("companion_reader".to_string()),
            ("code specialist", Some("companion_operator")) => {
                Some("companion_operator".to_string())
            }
            _ => None,
        })
        .unwrap_or_else(|| fallback_role.to_string());
    let companion_route_enforced = route_execution_state == "route_backed_specialist_owned"
        && route_owner_role == Some(selected_role.as_str())
        && route_operation == "mutation"
        && !release_install_repair_guard;
    let selected_agent_id = agent_id_for_role(&selected_role)
        .unwrap_or(selected_role.as_str())
        .to_string();
    let selected_category = specialist_route
        .get("selected_category")
        .cloned()
        .unwrap_or(Value::Null);
    let selected_skill = selected_skill_payload_for_role(&selected_role);
    let risk = infer_selected_risk(&selected_skill, mutation_intent);
    let evidence_need = infer_evidence_need(&selected_skill, request_shape);
    let verification_need = infer_verification_need(&selected_role, risk, mutation_intent);
    let reason = selected_specialist_reason(
        selected_category.as_str(),
        &selected_role,
        &selected_agent_id,
        companion_route_enforced,
        release_install_repair_guard,
        &specialist_route,
    );
    let routing_summary = if companion_route_enforced {
        let route_class = tool_route
            .get("route_class")
            .and_then(Value::as_str)
            .unwrap_or("tool_route");
        let operation = tool_route
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("operation");
        format!(
            "Companion tool routing selected {selected_role} for {route_class} {operation}; captain should request that companion subagent unless explicit fallback is recorded."
        )
    } else if release_install_repair_guard && selected_role == "code specialist" {
        "Release/install script repair routing selected code specialist; captain should dispatch the specialist unless explicit fallback or degradation is recorded.".to_string()
    } else {
        specialist_route
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Routing fell back to {fallback_role}."))
    };
    let internal_routing = create_internal_routing_protocol_payload(
        selected_category.clone(),
        selected_skill.clone(),
        risk,
        evidence_need,
        verification_need,
        mutation_intent,
        &selected_role,
        &selected_agent_id,
        &reason,
    );
    json!({
        "request_shape": request_shape,
        "mutation_intent": mutation_intent,
        "fallback_role": fallback_role,
        "selected_category": selected_category,
        "selected_skill": selected_skill,
        "selected_skill_id": internal_routing.pointer("/selected_skill/id").cloned().unwrap_or(Value::Null),
        "selected_skill_name": internal_routing.pointer("/selected_skill/name").cloned().unwrap_or(Value::Null),
        "risk": risk,
        "evidence_need": evidence_need,
        "verification_need": verification_need,
        "selected_role": selected_role,
        "selected_agent_id": selected_agent_id,
        "reason": reason,
        "summary": routing_summary,
        "routing_protocol": internal_routing,
        "routing_evidence_source": specialist_route.get("selected_routing_evidence_source").cloned().unwrap_or(Value::String("fallback_heuristic".to_string())),
        "display_metadata_sources": specialist_route.get("selected_display_metadata_sources").cloned().unwrap_or_else(|| json!({
            "agent": "fallback_heuristic",
            "model": "fallback_heuristic",
            "variant": "fallback_heuristic",
            "reasoning": "fallback_heuristic",
        })),
        "companion_route_enforced": companion_route_enforced,
        "release_install_script_repair_guard": release_install_repair_guard,
        "tool_route": tool_route,
        "companion_tool_route": tool_route,
        "specialist_route": specialist_route,
    })
}

fn normalize_assigned_agent_id(value: &str) -> String {
    value
        .trim()
        .strip_prefix("ccc_")
        .unwrap_or_else(|| value.trim())
        .to_string()
}

fn assignment_expected_family_for_task_card(
    task_card: &Value,
    request_text: &str,
) -> Option<Value> {
    if let Some(planned_expected) = explicit_planned_row_expected_family(task_card) {
        return Some(planned_expected);
    }

    let task_kind = task_card
        .get("task_kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let request_shape = infer_request_shape(request_text);
    let mutation_intent = infer_mutation_intent(request_shape);
    let intent_types = infer_intent_types(request_text, request_shape);
    let tool_route = create_companion_tool_route_payload(request_text, mutation_intent);
    let route_class = tool_route
        .get("route_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let operation = tool_route
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let route_owner_role = tool_route.get("owner_role").and_then(Value::as_str);

    if request_shape == "diagnostic" {
        return Some(json!({
            "family": "read_only_diagnostic",
            "expected_roles": ["explorer", "companion_reader"],
            "expected_agent_ids": ["scout", "ccc_scout", "companion_reader", "ccc_companion_reader"],
            "reason": "Smoke, install, status, and visibility diagnostics should stay read-only and should not require review routing.",
        }));
    }

    if task_kind == "review"
        || request_shape == "review"
        || intent_types
            .iter()
            .any(|intent| matches!(intent.as_str(), "review" | "validation" | "verification"))
    {
        return Some(json!({
            "family": "review_acceptance",
            "expected_roles": ["verifier"],
            "expected_agent_ids": ["arbiter", "ccc_arbiter"],
            "reason": "Review, verification, and acceptance work should route to ccc_arbiter.",
        }));
    }

    if operation == "mutation"
        && route_owner_role == Some("companion_operator")
        && matches!(route_class, "git_mutation")
    {
        return Some(json!({
            "family": "operator_side_mutation",
            "expected_roles": ["companion_operator"],
            "expected_agent_ids": ["companion_operator", "ccc_companion_operator"],
            "reason": "Narrow git or GitHub mutation should route to companion_operator.",
        }));
    }

    if intent_types.iter().any(|intent| intent == "documentation") {
        return Some(json!({
            "family": "docs_planning",
            "expected_roles": ["documenter"],
            "expected_agent_ids": ["scribe", "ccc_scribe"],
            "reason": "Documentation, release-plan, and planning-doc updates should route to ccc_scribe.",
        }));
    }

    if request_shape == "mutation" {
        return Some(json!({
            "family": "bounded_mutation",
            "expected_roles": ["code specialist"],
            "expected_agent_ids": ["raider", "ccc_raider"],
            "reason": "Bounded code or config mutation should route to ccc_raider.",
        }));
    }

    if task_kind == "way" || matches!(request_shape, "way" | "lookup" | "diagnostic") {
        return Some(json!({
            "family": "read_only_investigation_or_design",
            "expected_roles": ["explorer", "companion_reader", "way"],
            "expected_agent_ids": ["scout", "ccc_scout", "companion_reader", "ccc_companion_reader", "tactician", "ccc_tactician"],
            "reason": "Read-only investigation or design assessment should route to ccc_scout, ccc_companion_reader, or ccc_tactician.",
        }));
    }

    None
}

fn explicit_planned_row_expected_family(task_card: &Value) -> Option<Value> {
    let planned_row = task_card.get("planned_longway_row")?;
    let planned_role = planned_row
        .get("planned_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "unassigned")?;
    let planned_agent_id = planned_row
        .get("planned_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "unassigned")?;

    let normalized_agent_id = normalize_assigned_agent_id(planned_agent_id);
    let family = match (planned_role, normalized_agent_id.as_str()) {
        ("way", "tactician") => "planning_way",
        ("explorer", "scout") => "read_only_investigation_or_design",
        ("code specialist", "raider") => "bounded_mutation",
        ("verifier", "arbiter") => "review_acceptance",
        ("documenter", "scribe") => "docs_planning",
        ("companion_reader", "companion_reader") => "read_only_diagnostic",
        ("companion_operator", "companion_operator") => "operator_side_mutation",
        _ => return None,
    };

    Some(json!({
        "family": family,
        "expected_roles": [planned_role],
        "expected_agent_ids": [planned_agent_id, format!("ccc_{normalized_agent_id}")],
        "reason": "Approved LongWay planned-row owner metadata is explicit and takes precedence over text-only diagnostic heuristics.",
        "source": "planned_longway_row",
    }))
}

pub(crate) fn create_assignment_quality_payload(task_card: &Value) -> Value {
    let request_text = combine_request_text_for_routing(task_card);
    if request_text.trim().is_empty() {
        return json!({
            "state": "unknown",
            "summary": "Assignment quality could not be checked because the task card has no request text.",
        });
    }
    let Some(expected) = assignment_expected_family_for_task_card(task_card, &request_text) else {
        return json!({
            "state": "unknown",
            "summary": "Assignment quality could not infer an expected specialist family.",
        });
    };

    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let assigned_agent_id = task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let sequence = task_card
        .get("sequence")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if assigned_role.is_none() && assigned_agent_id.is_none() {
        return json!({
            "state": "pending",
            "phase": if sequence == "PLAN_SEQUENCE" { "planning" } else { "execution" },
            "drift_severity": "unknown",
            "expected_family": expected.get("family").cloned().unwrap_or(Value::Null),
            "expected_roles": expected.get("expected_roles").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "expected_agent_ids": expected.get("expected_agent_ids").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "reason": expected.get("reason").cloned().unwrap_or(Value::Null),
            "summary": "Assignment quality is pending because no specialist is assigned yet.",
        });
    }

    let assigned_role_label = assigned_role.unwrap_or("unassigned");
    let assigned_agent_label = assigned_agent_id.unwrap_or("unassigned");
    let normalized_assigned_agent_id = normalize_assigned_agent_id(assigned_agent_label);
    if sequence == "PLAN_SEQUENCE"
        && assigned_role_label == "way"
        && normalized_assigned_agent_id == "tactician"
    {
        let execution_expected_family = expected
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return json!({
            "state": "matched",
            "phase": "planning",
            "route_relation": "planning_route_valid_execution_route_deferred",
            "drift_severity": "info",
            "expected_family": "planning_way",
            "expected_roles": ["way"],
            "expected_agent_ids": ["tactician", "ccc_tactician"],
            "execution_expected_family": expected.get("family").cloned().unwrap_or(Value::Null),
            "execution_expected_roles": expected.get("expected_roles").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "execution_expected_agent_ids": expected.get("expected_agent_ids").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "assigned_role": assigned_role_label,
            "assigned_agent_id": assigned_agent_label,
            "reason": "PLAN_SEQUENCE uses Way/tactician for bounded planning; execution routing is deferred to planned rows and Scheduler.",
            "summary": format!("Assignment matches PLAN_SEQUENCE planning route; execution expectation is {execution_expected_family}."),
        });
    }

    let expected_roles = expected
        .get("expected_roles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_agent_ids = expected
        .get("expected_agent_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let role_matches = assigned_role.map_or(true, |role| {
        expected_roles
            .iter()
            .filter_map(Value::as_str)
            .any(|expected_role| expected_role == role)
    });
    let agent_matches = assigned_agent_id.map_or(true, |agent_id| {
        let normalized_agent_id = normalize_assigned_agent_id(agent_id);
        expected_agent_ids
            .iter()
            .filter_map(Value::as_str)
            .any(|expected_agent| {
                normalize_assigned_agent_id(expected_agent) == normalized_agent_id
            })
    });
    let state = if role_matches && agent_matches {
        "matched"
    } else {
        "mismatch"
    };
    let expected_family = expected
        .get("family")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let summary = if state == "matched" {
        format!("Assignment matches expected {expected_family} specialist family.")
    } else {
        format!(
            "Routing drift: assigned {assigned_role_label}/{assigned_agent_label}, expected {expected_family} specialist family."
        )
    };

    json!({
        "state": state,
        "phase": if sequence == "PLAN_SEQUENCE" { "planning" } else { "execution" },
        "route_relation": if state == "matched" { "matched" } else { "routing_drift" },
        "drift_severity": if state == "matched" { "none" } else { "blocking" },
        "expected_family": expected.get("family").cloned().unwrap_or(Value::Null),
        "expected_roles": expected_roles,
        "expected_agent_ids": expected_agent_ids,
        "assigned_role": assigned_role.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
        "assigned_agent_id": assigned_agent_id.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
        "reason": expected.get("reason").cloned().unwrap_or(Value::Null),
        "summary": summary,
    })
}

pub(crate) fn infer_request_shape(request: &str) -> &'static str {
    let normalized = request.to_ascii_lowercase();
    let normalized_tokens = normalized
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mentions_strong_mutation_token = normalized_tokens.iter().any(|token| {
        matches!(
            *token,
            "fix"
                | "implement"
                | "change"
                | "update"
                | "rename"
                | "wire"
                | "add"
                | "remove"
                | "refactor"
                | "convert"
                | "patch"
                | "repair"
                | "mutate"
                | "mutation"
                | "mutating"
        )
    });
    let mentions_non_update_strong_mutation_token = normalized_tokens.iter().any(|token| {
        matches!(
            *token,
            "fix"
                | "implement"
                | "change"
                | "rename"
                | "wire"
                | "add"
                | "remove"
                | "refactor"
                | "convert"
                | "patch"
                | "repair"
                | "mutate"
                | "mutation"
                | "mutating"
        )
    });
    let has_visibility_diagnostic_signal = has_visibility_diagnostic_signal(request, &normalized);
    let mentions_review_token = normalized_tokens.iter().any(|token| {
        matches!(
            *token,
            "review"
                | "reviews"
                | "reviewed"
                | "reviewing"
                | "verify"
                | "verifies"
                | "verified"
                | "verifying"
                | "verification"
                | "validation"
                | "regression"
                | "check"
                | "checks"
                | "checked"
                | "checking"
                | "test"
                | "tests"
                | "tested"
                | "testing"
        )
    });
    if has_visibility_diagnostic_signal && !mentions_non_update_strong_mutation_token {
        "diagnostic"
    } else if mentions_strong_mutation_token {
        "mutation"
    } else if has_visibility_diagnostic_signal {
        "diagnostic"
    } else if mentions_review_token
        || [
            "검토",
            "확인",
            "점검",
            "レビュー",
            "確認",
            "検証",
            "检查",
            "验证",
        ]
        .iter()
        .any(|keyword| request.contains(keyword))
    {
        "review"
    } else if [
        "way",
        "plan",
        "longway",
        "phase",
        "step",
        "roadmap",
        "migration",
        "strategy",
        "investigate",
        "analyze",
        "compare",
        "summary",
        "status",
        "what remains",
        "남은 작업",
        "막힌",
        "계획",
        "전략",
        "计划",
        "策略",
        "計画",
        "方針",
    ]
    .iter()
    .any(|keyword| request.contains(keyword) || normalized.contains(keyword))
    {
        "way"
    } else if [
        "fix",
        "implement",
        "change",
        "update",
        "rename",
        "wire",
        "add",
        "remove",
        "refactor",
        "convert",
        "retry",
        "patch",
        "수정",
        "구현",
        "추가",
        "삭제",
        "변경",
        "更新",
        "修正",
        "追加",
        "削除",
        "実装",
        "修改",
        "实现",
        "添加",
        "删除",
    ]
    .iter()
    .any(|keyword| request.contains(keyword) || normalized.contains(keyword))
    {
        "mutation"
    } else {
        "lookup"
    }
}

pub(crate) fn infer_mutation_intent(request_shape: &str) -> &'static str {
    if request_shape == "mutation" {
        "explicit_or_strong"
    } else {
        "read_only_or_unspecified"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multilingual_requests_route_to_expected_shapes_and_tools() {
        assert_eq!(infer_request_shape("README 문서를 수정해줘"), "mutation");
        assert_eq!(infer_request_shape("この実装をレビューして"), "review");
        assert_eq!(infer_request_shape("制定下一步计划"), "way");
        assert_eq!(infer_request_shape("0.0.11 Smoke CLI Status"), "diagnostic");
        assert_eq!(
            infer_request_shape("CCC 상태와 LongWay app visibility가 정상적으로 보이는지 확인"),
            "diagnostic"
        );

        let korean_tools = infer_requested_tool_names("깃허브 릴리즈를 만들고 푸시해줘");
        assert!(korean_tools.contains(&"gh"));
        assert!(korean_tools.contains(&"git"));
        assert_eq!(
            infer_tool_operation(
                "깃허브 릴리즈를 만들고 푸시해줘",
                &korean_tools,
                "explicit_or_strong"
            ),
            "mutation"
        );

        let japanese_tools = infer_requested_tool_names("現在のディレクトリのファイルを確認して");
        assert!(japanese_tools.contains(&"filesystem"));

        let intents = infer_intent_types("文档を更新して", "mutation");
        assert!(intents.contains(&"documentation".to_string()));
        let diagnostic_intents = infer_intent_types("Smoke app visibility check", "diagnostic");
        assert!(diagnostic_intents.contains(&"read_only".to_string()));
        assert!(diagnostic_intents.contains(&"diagnosis".to_string()));
    }

    #[test]
    fn honest_acceptance_text_does_not_trigger_test_review_routing() {
        assert_eq!(
            infer_request_shape(
                "Retry Phase 3 Step 2 with a bounded planning pass\nPersist an honest bounded run with one Way checkpoint."
            ),
            "way"
        );
        assert_eq!(infer_request_shape("Run the targeted tests."), "review");
    }

    #[test]
    fn installed_cli_lifecycle_diagnostics_stay_read_only_despite_subagent_update_token() {
        let lifecycle_request =
            "Confirm installed binary start/status/checklist/orchestrate/subagent-update surfaces work";
        assert_eq!(infer_request_shape(lifecycle_request), "diagnostic");
        let intents = infer_intent_types(lifecycle_request, infer_request_shape(lifecycle_request));
        assert!(intents.contains(&"read_only".to_string()));
        assert!(intents.contains(&"diagnosis".to_string()));

        assert_eq!(
            infer_request_shape("Record ccc subagent-update --quiet lifecycle output"),
            "diagnostic"
        );
        assert_eq!(
            infer_request_shape("Use ccc subagent-update to repair task card lifecycle state"),
            "mutation"
        );
        assert_eq!(
            infer_request_shape("Use ccc subagent-update to mutate task card lifecycle state"),
            "mutation"
        );
        assert_eq!(
            infer_request_shape("Use ccc subagent-update for task card lifecycle mutation"),
            "mutation"
        );
    }

    #[test]
    fn release_install_script_repair_does_not_promote_github_wording_to_gh_mutation() {
        let tools = infer_requested_tool_names(
            "Fix install.sh and scripts/release/build-release-asset.sh for the GitHub release asset packaging repair.",
        );
        assert!(tools.contains(&"filesystem"));
        assert!(tools.contains(&"gh"));
        assert_eq!(
            infer_tool_operation(
                "Fix install.sh and scripts/release/build-release-asset.sh for the GitHub release asset packaging repair.",
                &tools,
                "explicit_or_strong",
            ),
            "read"
        );
    }

    #[test]
    fn explicit_gh_release_mutation_verbs_stay_operator_owned() {
        for verb in ["upload", "edit", "create", "delete"] {
            let request = format!("Use gh release {verb} for v0.0.8-pre.");
            let tools = infer_requested_tool_names(&request);
            assert!(tools.contains(&"gh"));
            assert_eq!(
                infer_tool_operation(&request, &tools, "read_only_or_unspecified"),
                "mutation"
            );
        }
    }
}
