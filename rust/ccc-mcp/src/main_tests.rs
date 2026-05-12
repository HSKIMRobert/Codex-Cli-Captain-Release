use super::*;
use std::fs::{create_dir_all, write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn create_temp_path(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!("ccc-rust-{label}-{suffix}"))
}

fn backup_paths_for(path: &Path) -> Vec<PathBuf> {
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let prefix = format!("{file_name}.");
    let mut paths = fs::read_dir(parent)
        .unwrap_or_else(|_| panic!("read backup parent {}", parent.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(|value| value.to_str())
                .map(|candidate_name| {
                    candidate_name.starts_with(&prefix) && candidate_name.ends_with(".bak")
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn expected_tool_routing_field(tool_name: &str, field: &str, default_field: &str) -> Value {
    let tool_routing = load_tool_routing_policy();
    tool_routing
        .get("tools")
        .and_then(Value::as_object)
        .and_then(|tools| tools.get(tool_name))
        .and_then(|entry| entry.get(field))
        .cloned()
        .or_else(|| tool_routing.get(default_field).cloned())
        .unwrap_or(Value::Null)
}

fn expected_role_config_field(role: &str, field: &str) -> Value {
    load_role_config_snapshot(role)
        .get(field)
        .cloned()
        .unwrap_or(Value::Null)
}

fn planned_row_schema_property_names(schema_source: &str) -> Vec<String> {
    let schema: Value = serde_json::from_str(schema_source).expect("parse planned-row schema");
    let mut keys = schema["$defs"]["plannedRow"]["properties"]
        .as_object()
        .expect("planned-row schema properties")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn assert_planned_row_keys_declared(row: &Value, schema_source: &str) {
    let declared_keys = planned_row_schema_property_names(schema_source);
    let mut unknown_keys = row
        .as_object()
        .expect("planned row object")
        .keys()
        .filter(|key| !declared_keys.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    unknown_keys.sort();
    assert!(
        unknown_keys.is_empty(),
        "planned row contains schema-undeclared keys: {unknown_keys:?}"
    );
}

fn task_card_schema_property_names(schema_source: &str) -> Vec<String> {
    let schema: Value = serde_json::from_str(schema_source).expect("parse task-card schema");
    let mut keys = schema["properties"]
        .as_object()
        .expect("task-card schema properties")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn assert_task_card_schema_declares(keys: &[&str], schema_source: &str) {
    let declared_keys = task_card_schema_property_names(schema_source);
    let mut missing_keys = keys
        .iter()
        .filter(|key| !declared_keys.iter().any(|declared| declared == **key))
        .copied()
        .collect::<Vec<_>>();
    missing_keys.sort();
    assert!(
        missing_keys.is_empty(),
        "task-card schema is missing materialized metadata keys: {missing_keys:?}"
    );
}

fn sorted_object_keys(value: &Value) -> Vec<&str> {
    let mut keys = value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

fn mark_task_card_codex_exec_fallback(run_directory: &Path, task_card_id: &str) {
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    task_card["subagent_fallback"] = json!({
        "reason": "subagent_spawn_unavailable",
        "recorded_at": "2026-04-25T00:00:00.000Z",
    });
    task_card["subagent_lifecycle"] = json!({
        "status": "failed",
        "child_agent_id": "ccc_raider",
        "summary": "Host subagent terminal fallback fixture.",
        "updated_at": "2026-04-25T00:00:00.000Z",
    });
    write_json_document(&task_card_file, &task_card).expect("write task card fallback");
}

fn write_test_run_fixture(workspace_dir: &Path, run_id: &str) -> PathBuf {
    let run_directory = workspace_dir.join(".ccc").join("runs").join(run_id);
    create_dir_all(run_directory.join("task-cards")).expect("create task-card directory");
    write(workspace_dir.join(".ccc").join("role-defaults.json"), "{}")
        .expect("write role-defaults");
    write(
        run_directory.join("run.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": run_id,
            "goal": "Implement Rust ccc_status parity",
            "status": "active",
            "stage": "execution",
            "active_role": "code specialist",
            "active_agent_id": "raider",
            "active_task_card_id": "task-1",
            "active_thread_id": null,
            "task_card_ids": ["task-1"],
            "latest_handoff_id": null,
            "child_agents": [],
            "specialist_executors": [],
            "created_at": "2026-04-22T08:00:00.000Z",
            "updated_at": "2026-04-22T08:01:00.000Z",
            "completed_at": null
        }))
        .expect("serialize run"),
    )
    .expect("write run");
    write(
        run_directory.join("run-state.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "run_id": run_id,
            "updated_at": "2026-04-22T08:01:00.000Z",
            "event_count": 3,
            "last_event_id": "event-3",
            "current_phase_name": "execute",
            "next_action": {
                "command": "advance"
            }
        }))
        .expect("serialize run-state"),
    )
    .expect("write run-state");
    write(
        run_directory.join("longway.json"),
        serde_json::to_vec_pretty(&json!({
            "lifecycle_state": "active",
            "active_phase_name": "execute",
            "active_phase_status": "in_progress",
            "phases": [{ "phase_name": "execute" }]
        }))
        .expect("serialize longway"),
    )
    .expect("write longway");
    write(
        run_directory.join("task-cards").join("task-1.json"),
        serde_json::to_vec_pretty(&json!({
            "task_card_id": "task-1",
            "run_id": run_id,
            "title": "Implement Rust ccc_status",
            "intent": "Read persisted run truth",
            "scope": "Read-only status surface",
            "status": "active",
            "task_kind": "execution",
            "assigned_role": "code specialist",
            "assigned_agent_id": "raider"
        }))
        .expect("serialize task-card"),
    )
    .expect("write task-card");

    run_directory
}

#[test]
fn permission_fallback_run_directory_uses_workspace_ccc_not_legacy_ccc() {
    let workspace_dir = create_temp_path("permission-fallback-local-ccc");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_id = "run-permission-fallback";
    let run_directory =
        crate::run_bootstrap::create_permission_fallback_run_directory_from_workspace(
            &workspace_dir,
            run_id,
        );

    assert_eq!(
        run_directory,
        crate::run_locator::normalize_path(&workspace_dir)
            .join(".ccc")
            .join("runs")
            .join(run_id)
    );
    assert!(run_directory.to_string_lossy().contains(&format!(
        "{}{}",
        std::path::MAIN_SEPARATOR,
        ".ccc"
    )));

    crate::run_locator::ensure_run_paths_for_start(&workspace_dir, &run_directory)
        .expect("create fallback run paths");
    let resolved =
        crate::run_locator::resolve_run_directory_locator(&run_directory.to_string_lossy())
            .expect("resolve workspace-local fallback run");
    assert_eq!(
        resolved.cwd,
        crate::run_locator::normalize_path(&workspace_dir)
    );
    assert_eq!(resolved.run_id, run_id);
    assert_eq!(
        resolved.run_directory,
        crate::run_locator::normalize_path(&run_directory)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

fn write_review_task_card_fixture(run_directory: &Path, run_id: &str) {
    write_json_document(
        &run_directory.join("task-cards").join("task-1.json"),
        &json!({
            "task_card_id": "task-1",
            "run_id": run_id,
            "title": "Review returned implementation",
            "intent": "Verify returned implementation without mutating it",
            "scope": "Review only",
            "status": "active",
            "task_kind": "review",
            "review_of_task_card_ids": ["task-source"],
            "orchestrator_review_gate": "after_child_completion",
            "verification_state": "pending",
            "review_pass_count": 0,
            "review_policy": {
                "decision": "require",
                "state": "running",
                "risk": "high",
                "required": true,
                "recommended_reviewers": 1,
                "reviewer_cap": 1,
                "active_reviewers": 1,
                "reason_code": "high_risk_review_required",
                "summary": "Review is running.",
                "recorded_at": "2026-04-22T08:00:00.000Z"
            },
            "assigned_role": "verifier",
            "assigned_agent_id": "arbiter",
            "delegation_plan": create_specialist_delegation_plan_with_runtime(
                "verifier",
                &json!({
                    "summary": "Bounded read-only verification.",
                    "model": "gpt-5.5",
                    "variant": "high",
                    "fast_mode": true,
                }),
                &json!({
                    "preferred_specialist_execution_mode": "codex_subagent",
                    "fallback_specialist_execution_mode": "codex_exec",
                }),
                "read-only",
                "Verifier work should inspect and judge without mutating the workspace.",
            )
        }),
    )
    .expect("write review task-card");
}

fn start_intervention_test_run(workspace_dir: &Path, label: &str) -> (String, PathBuf) {
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": format!("Record captain intervention state for {label}"),
        "title": format!("Intervention {label}"),
        "intent": "Exercise bounded captain dissatisfaction/intervention state capture",
        "scope": "One subagent update artifact",
        "acceptance": "Persist and surface the captain-owned intervention artifact",
        "prompt": "Return a bounded subagent result for captain review.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"]
        .as_str()
        .expect("run id")
        .to_string();
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    (run_id, run_directory)
}

fn parse_and_record_subagent_update(payload: Value) -> Value {
    let parsed = parse_ccc_subagent_update_arguments(&payload).expect("parse update");
    create_ccc_subagent_update_payload(&parsed).expect("record update")
}

fn read_active_task_card(run_directory: &Path) -> Value {
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task-card id");
    read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task-card")
}

fn force_run_to_captain_advance(run_directory: &Path) {
    let mut run_state =
        read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    run_state["next_action"] = json!({ "command": "advance" });
    run_state["current_phase_name"] = json!("fan_in");
    write_json_document(&run_directory.join("run-state.json"), &run_state)
        .expect("write run-state");

    let mut orchestrator_state = read_json_document(&run_directory.join("orchestrator-state.json"))
        .expect("orchestrator-state");
    orchestrator_state["decision"] = json!({
        "next_step": "advance",
        "can_advance": true,
        "summary": "captain follow-up ready"
    });
    write_json_document(
        &run_directory.join("orchestrator-state.json"),
        &orchestrator_state,
    )
    .expect("write orchestrator-state");
}

fn call_ccc_orchestrate_tool(
    session_context: &SessionContext,
    run_id: &str,
    workspace_dir: &Path,
    id: u64,
) -> Value {
    handle_message(
        session_context,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("orchestrate response")
}

fn task_cards_with_captain_follow_up_dedupe_key(
    run_directory: &Path,
    dedupe_key: &str,
) -> Vec<Value> {
    let mut task_cards = fs::read_dir(run_directory.join("task-cards"))
        .expect("task-card directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .filter_map(|path| read_json_document(&path).ok())
        .filter(|task_card| task_card_captain_follow_up_dedupe_key(task_card) == Some(dedupe_key))
        .collect::<Vec<_>>();
    task_cards.sort_by(|left, right| {
        left.get("task_card_id")
            .and_then(Value::as_str)
            .cmp(&right.get("task_card_id").and_then(Value::as_str))
    });
    task_cards
}

fn review_task_cards_for_source(run_directory: &Path, source_task_card_id: &str) -> Vec<Value> {
    let mut review_task_cards = fs::read_dir(run_directory.join("task-cards"))
        .expect("task-card directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .filter_map(|path| read_json_document(&path).ok())
        .filter(|task_card| task_card_reviews_source(task_card, source_task_card_id))
        .collect::<Vec<_>>();
    review_task_cards.sort_by(|left, right| {
        left.get("task_card_id")
            .and_then(Value::as_str)
            .cmp(&right.get("task_card_id").and_then(Value::as_str))
    });
    review_task_cards
}

fn create_fake_codex_executable_with_usage(
    workspace_dir: &Path,
    file_name: &str,
    thread_id: &str,
    message: &str,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_output_tokens: u64,
) -> PathBuf {
    let script_path = workspace_dir.join(file_name);
    write(
        &script_path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' '{}' '{}' '{}'\n",
            json!({
                "type": "thread.started",
                "thread_id": thread_id
            }),
            json!({
                "type": "item.completed",
                "item": {
                    "id": "item_0",
                    "type": "agent_message",
                    "text": message
                }
            }),
            json!({
                "type": "turn.completed",
                "usage": {
                    "input_tokens": input_tokens,
                    "cached_input_tokens": cached_input_tokens,
                    "output_tokens": output_tokens,
                    "reasoning_output_tokens": reasoning_output_tokens
                }
            }),
        ),
    )
    .expect("write fake codex");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script_path)
            .expect("fake codex metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("chmod fake codex");
    }
    script_path
}

fn create_fake_codex_executable(workspace_dir: &Path) -> PathBuf {
    create_fake_codex_executable_with_usage(
        workspace_dir,
        "fake-codex.sh",
        "thread-rust-test",
        "Bounded worker result.",
        1200,
        300,
        200,
        100,
    )
}

fn create_fake_codex_executable_requiring_stdin(
    workspace_dir: &Path,
    file_name: &str,
    thread_id: &str,
    message: &str,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_output_tokens: u64,
) -> PathBuf {
    let script_path = workspace_dir.join(file_name);
    write(
            &script_path,
            format!(
                "#!/bin/sh\nprompt=\"$(cat)\"\nif [ -z \"$prompt\" ]; then\n  printf '%s\\n' 'WARNING: proceeding, even though we could not update PATH: Operation not permitted (os error 1)' 'Reading additional input from stdin...'\n  exit 0\nfi\nprintf '%s\\n' '{}' '{}' '{}'\n",
                json!({
                    "type": "thread.started",
                    "thread_id": thread_id
                }),
                json!({
                    "type": "item.completed",
                    "item": {
                        "id": "item_0",
                        "type": "agent_message",
                        "text": message
                    }
                }),
                json!({
                    "type": "turn.completed",
                    "usage": {
                        "input_tokens": input_tokens,
                        "cached_input_tokens": cached_input_tokens,
                        "output_tokens": output_tokens,
                        "reasoning_output_tokens": reasoning_output_tokens
                    }
                }),
            ),
        )
        .expect("write fake codex requiring stdin");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script_path)
            .expect("fake codex requiring stdin metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("chmod fake codex requiring stdin");
    }
    script_path
}

fn create_failing_fake_codex_executable(
    workspace_dir: &Path,
    file_name: &str,
    exit_code: i32,
    stderr_line: &str,
) -> PathBuf {
    let script_path = workspace_dir.join(file_name);
    write(
        &script_path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' '{}' 1>&2\nexit {exit_code}\n",
            stderr_line
        ),
    )
    .expect("write failing fake codex");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script_path)
            .expect("failing fake codex metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("chmod failing fake codex");
    }
    script_path
}

fn create_turn_failed_fake_codex_executable(
    workspace_dir: &Path,
    file_name: &str,
    thread_id: &str,
    exit_code: i32,
    error_message: &str,
) -> PathBuf {
    let script_path = workspace_dir.join(file_name);
    write(
        &script_path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' '{}' '{}' '{}' '{}'\nexit {exit_code}\n",
            json!({
                "type": "thread.started",
                "thread_id": thread_id
            }),
            json!({
                "type": "turn.started"
            }),
            json!({
                "type": "error",
                "message": error_message
            }),
            json!({
                "type": "turn.failed",
                "error": {
                    "message": error_message
                }
            }),
        ),
    )
    .expect("write turn failed fake codex");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script_path)
            .expect("turn failed fake codex metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("chmod turn failed fake codex");
    }
    script_path
}

#[test]
fn server_identity_payload_contains_session_and_config_fields() {
    let session_context = create_session_context();
    let payload = create_server_identity_payload(&session_context);

    assert_eq!(payload["server_name"], SERVER_NAME);
    assert_eq!(payload["server_version"], env!("CARGO_PKG_VERSION"));
    assert!(payload["session_id"]
        .as_str()
        .unwrap_or_default()
        .starts_with("mcp-session-"));
    assert!(payload["started_at"].is_string());
    assert!(payload["build_identity"]
        .as_str()
        .unwrap_or_default()
        .contains(SERVER_NAME));
    assert!(payload["shared_config_path"]
        .as_str()
        .unwrap_or_default()
        .contains("ccc-config.toml"));
}

#[test]
fn ccc_server_identity_tool_returns_structured_identity_and_install_check() {
    let session_context = create_session_context();
    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "ccc_server_identity",
                "arguments": {}
            }
        }),
    )
    .expect("response");

    let content_text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(content_text.contains("Attached CCC MCP session"));
    assert_eq!(
        response["result"]["structuredContent"]["server_identity"]["server_name"],
        SERVER_NAME
    );
    assert_eq!(
        response["result"]["structuredContent"]["install_check"]["serverName"],
        "ccc"
    );
    assert_eq!(
        response["result"]["structuredContent"]["install_check"]["session_registration_match"],
        "matching"
    );
}

#[test]
fn setup_config_creates_default_toml_without_nulls() {
    let config_home = create_temp_path("config-default");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_toml_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_json_path = config_home.join("ccc").join("ccc-config.json");

    let (created_path, created) =
        ensure_ccc_config_file_at(&config_path, &legacy_toml_path, &legacy_json_path)
            .expect("create config");
    assert!(created);
    assert_eq!(created_path, config_path);

    let contents = fs::read_to_string(&config_path).expect("read config");
    assert!(!contents.contains("null"));
    assert!(contents.contains("[agents.orchestrator]"));
    assert!(contents.contains("[features]"));
    assert!(contents.contains("[lsp]"));
    assert!(contents.contains("[graph_context]"));
    assert!(!contents.contains("[routing.categories.read_repo]"));
    assert!(!contents.contains("[tool_routing]"));
    assert!(!contents.contains("[runtime]"));
    toml::from_str::<toml::Value>(&contents).expect("parse generated toml");
    let generated = read_optional_toml_document(&config_path)
        .expect("read generated config")
        .expect("generated config");
    assert_eq!(generated["generated_defaults"]["version"], 16);
    assert_eq!(generated["features"]["graph_context"], false);
    assert_eq!(generated["features"]["goals"], false);
    assert_eq!(generated["features"]["prompt_refinement"], false);
    assert_eq!(generated["goal_bridge"]["enabled"], false);
    assert_eq!(generated["goal_bridge"]["mode"], "captain_owned");
    assert_eq!(generated["goal_bridge"]["brief_language"], "en");
    assert_eq!(generated["goal_bridge"]["brief_max_lines"], 12);
    assert_eq!(generated["goal_bridge"]["require_verifiable_stop"], true);
    assert_eq!(generated["goal_bridge"]["host_goal_state_is_truth"], false);
    assert_eq!(
        generated["goal_bridge"]["specialists"]["allow_specialist_goal_context"],
        true
    );
    assert_eq!(
        generated["goal_bridge"]["specialists"]["allow_specialist_set_goal"],
        false
    );
    assert_eq!(
        generated["goal_bridge"]["specialists"]["allow_specialist_clear_goal"],
        false
    );
    assert_eq!(
        generated["goal_bridge"]["specialists"]["allow_specialist_override_goal"],
        false
    );
    assert_eq!(
        generated["goal_bridge"]["specialists"]["max_subgoal_lines"],
        8
    );
    assert_eq!(
        generated["goal_bridge"]["specialists"]["require_captain_acceptance"],
        true
    );
    assert_eq!(generated["lsp"]["runtime_execution"], "deferred");
    assert_eq!(generated["graph_context"]["enabled"], false);
    assert_eq!(generated["graph_context"]["provider"], "graphify");
    assert_eq!(generated["graph_context"]["mode"], "read_only");
    assert_eq!(generated["graph_context"]["canonical_backend"], "graphify");
    assert_eq!(
        generated["graph_context"]["replace_legacy_ccc_graph_backend"],
        true
    );
    assert_eq!(
        generated["graph_context"]["allow_legacy_graph_backend_fallback"],
        false
    );
    assert_eq!(
        generated["graph_context"]["fallback_when_unavailable"],
        "scout_source_evidence"
    );
    assert_eq!(
        generated["graph_context"]["report_path"],
        "graphify-out/GRAPH_REPORT.md"
    );
    assert_eq!(
        generated["graph_context"]["graph_path"],
        "graphify-out/graph.json"
    );
    assert_eq!(generated["graph_context"]["max_report_bytes"], 20000);
    assert_eq!(generated["graph_context"]["max_query_bytes"], 8000);
    assert_eq!(
        generated["graph_context"]["prefer_report_before_grep"],
        true
    );
    assert_eq!(generated["graph_context"]["allow_cli_query"], true);
    assert_eq!(generated["graph_context"]["allow_mcp_query"], false);
    assert_eq!(generated["graph_context"]["allow_rebuild"], false);
    assert_eq!(
        generated["graph_context"]["auto_install_external_dependency"],
        false
    );
    assert_eq!(generated["graph_context"]["source_of_truth"], false);
    assert_eq!(
        generated["graph_context"]["install"]["managed_by_ccc_setup"],
        true
    );
    assert_eq!(
        generated["graph_context"]["install"]["check_install_reports_readiness"],
        true
    );
    assert_eq!(
        generated["graph_context"]["install"]["require_graphify_cli_for_queries"],
        true
    );
    assert_eq!(
        generated["graph_context"]["install"]["allow_missing_provider_fallback"],
        true
    );
    assert_eq!(generated["graph_context"]["edges"]["allow_extracted"], true);
    assert_eq!(generated["graph_context"]["edges"]["allow_inferred"], true);
    assert_eq!(
        generated["graph_context"]["edges"]["allow_ambiguous"],
        false
    );
    assert_eq!(
        generated["graph_context"]["edges"]["require_source_check_for_mutation"],
        true
    );
    assert_eq!(
        generated["lsp"]["language_servers"]["typescript_javascript"]["command"],
        "typescript-language-server"
    );
    assert_eq!(
        generated["lsp"]["language_servers"]["rust"]["command"],
        "rust-analyzer"
    );
    assert_eq!(generated["agents"]["orchestrator"]["model"], "gpt-5.5");
    assert_eq!(generated["agents"]["orchestrator"]["variant"], "medium");
    assert_eq!(generated["agents"]["orchestrator"]["fast_mode"], false);
    assert_eq!(generated["agents"]["way"]["model"], "gpt-5.5");
    assert_eq!(generated["agents"]["way"]["variant"], "high");
    assert_eq!(generated["agents"]["code specialist"]["model"], "gpt-5.5");
    assert_eq!(generated["agents"]["code specialist"]["variant"], "high");
    assert_eq!(generated["agents"]["verifier"]["model"], "gpt-5.5");
    assert_eq!(generated["agents"]["verifier"]["variant"], "high");
    assert_eq!(generated["agents"]["sentinel"]["model"], "gpt-5.4-mini");
    assert_eq!(generated["agents"]["sentinel"]["variant"], "high");
    assert_eq!(generated["agents"]["sentinel"]["fast_mode"], true);
    assert_eq!(generated["agents"]["explorer"]["model"], "gpt-5.4-mini");
    assert_eq!(generated["agents"]["explorer"]["variant"], "high");
    assert_eq!(generated["agents"]["explorer"]["fast_mode"], true);
    assert_eq!(generated["agents"]["explorer"]["callsign"], "Observer");
    assert_eq!(
        generated["agents"]["explorer"]["recommended_workflows"],
        json!(["github-triage", "get-unpublished-changes"])
    );
    assert_eq!(
        generated["agents"]["code specialist"]["lsp_capabilities"],
        json!([
            "lsp_diagnostics",
            "lsp_references",
            "lsp_definition",
            "lsp_prepare_rename",
            "lsp_rename",
            "rust-analyzer-lsp"
        ])
    );
    assert_eq!(generated["agents"]["documenter"]["model"], "gpt-5.4-mini");
    assert_eq!(generated["agents"]["documenter"]["variant"], "medium");
    assert_eq!(generated["agents"]["documenter"]["fast_mode"], true);
    assert!(generated.get("companion_agents").is_none());
    assert!(generated.get("routing").is_none());
    assert!(generated.get("tool_routing").is_none());
    assert!(generated.get("runtime").is_none());

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn ccc_config_schema_declares_generated_sentinel_defaults() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/ccc-config.schema.json"))
            .expect("parse ccc config schema");
    assert!(schema.pointer("/properties/generated_defaults").is_some());
    assert!(schema
        .pointer("/properties/agents/properties/sentinel")
        .is_some());
    assert!(schema
        .pointer("/properties/agents/required")
        .and_then(Value::as_array)
        .is_some_and(|required| required.iter().any(|value| value == "sentinel")));
}

#[test]
fn ccc_config_schema_declares_graph_context_defaults() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/ccc-config.schema.json"))
            .expect("parse ccc config schema");

    assert!(schema.pointer("/properties/features").is_some());
    assert!(schema
        .pointer("/properties/features/properties/graph_context")
        .is_some());
    assert!(schema
        .pointer("/properties/features/properties/prompt_refinement")
        .is_some());
    assert!(schema.pointer("/properties/graph_context").is_some());
    assert_eq!(
        schema.pointer("/properties/graph_context/properties/provider/enum/0"),
        Some(&json!("graphify"))
    );
    assert_eq!(
        schema.pointer("/properties/graph_context/properties/mode/enum/0"),
        Some(&json!("read_only"))
    );
    assert_eq!(
        schema.pointer("/properties/graph_context/properties/canonical_backend/enum/0"),
        Some(&json!("graphify"))
    );
    assert_eq!(
        schema.pointer("/properties/graph_context/properties/fallback_when_unavailable/enum/0"),
        Some(&json!("scout_source_evidence"))
    );
    assert!(schema
        .pointer("/properties/graph_context/properties/install/properties/require_graphify_cli_for_queries")
        .is_some());
    assert!(schema
        .pointer("/properties/graph_context/properties/edges/properties/require_source_check_for_mutation")
        .is_some());
}

#[test]
fn ccc_config_schema_declares_goal_bridge_defaults() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/ccc-config.schema.json"))
            .expect("parse ccc config schema");

    assert!(schema
        .pointer("/properties/features/properties/goals")
        .is_some());
    assert!(schema.pointer("/properties/goal_bridge").is_some());
    assert_eq!(
        schema.pointer("/properties/goal_bridge/properties/mode/enum/0"),
        Some(&json!("captain_owned"))
    );
    assert_eq!(
        schema.pointer("/properties/goal_bridge/properties/brief_language/enum/0"),
        Some(&json!("en"))
    );
    assert_eq!(
        schema.pointer("/properties/goal_bridge/properties/host_goal_state_is_truth/const"),
        Some(&json!(false))
    );
    assert_eq!(
        schema.pointer(
            "/properties/goal_bridge/properties/specialists/properties/allow_specialist_set_goal/const"
        ),
        Some(&json!(false))
    );
    assert_eq!(
        schema.pointer(
            "/properties/goal_bridge/properties/specialists/properties/allow_specialist_clear_goal/const"
        ),
        Some(&json!(false))
    );
    assert_eq!(
        schema.pointer(
            "/properties/goal_bridge/properties/specialists/properties/allow_specialist_override_goal/const"
        ),
        Some(&json!(false))
    );
}

fn graph_context_enabled_test_config() -> Value {
    json!({
        "features": {
            "graph_context": true
        },
        "graph_context": {
            "enabled": true,
            "provider": "graphify",
            "mode": "read_only",
            "canonical_backend": "graphify",
            "allow_legacy_graph_backend_fallback": false,
            "fallback_when_unavailable": "scout_source_evidence",
            "report_path": "graphify-out/GRAPH_REPORT.md",
            "graph_path": "graphify-out/graph.json",
            "source_of_truth": false
        }
    })
}

fn write_status_graph_context_config(config_path: &Path, enabled: bool) {
    create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    let config = if enabled {
        r#"[features]
graph_context = true

[graph_context]
enabled = true
provider = "graphify"
mode = "read_only"
canonical_backend = "graphify"
allow_legacy_graph_backend_fallback = false
fallback_when_unavailable = "scout_source_evidence"
report_path = "graphify-out/GRAPH_REPORT.md"
graph_path = "graphify-out/graph.json"
source_of_truth = false

[runtime]
"#
    } else {
        "[runtime]\n"
    };
    write(config_path, config).expect("write graph context status config");
}

fn write_graph_context_routing_config(
    config_path: &Path,
    enabled: bool,
    max_report_bytes: Option<u64>,
    allow_mcp_query: Option<bool>,
) {
    create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    let max_report_bytes = max_report_bytes
        .map(|value| format!("max_report_bytes = {value}\n"))
        .unwrap_or_default();
    let allow_mcp_query = allow_mcp_query
        .map(|value| format!("allow_mcp_query = {value}\n"))
        .unwrap_or_default();
    let config = if enabled {
        format!(
            r#"[features]
graph_context = true

[graph_context]
enabled = true
provider = "graphify"
mode = "read_only"
canonical_backend = "graphify"
allow_legacy_graph_backend_fallback = false
fallback_when_unavailable = "scout_source_evidence"
report_path = "graphify-out/GRAPH_REPORT.md"
graph_path = "graphify-out/graph.json"
{max_report_bytes}{allow_mcp_query}source_of_truth = false

[runtime]
"#
        )
    } else {
        "[runtime]\n".to_string()
    };
    write(config_path, config).expect("write graph context routing config");
}

fn status_payload_with_graph_context_config(
    workspace_dir: &Path,
    run_id: &str,
    config_path: &Path,
) -> Value {
    write_test_run_fixture(workspace_dir, run_id);
    let mut session_context = create_session_context();
    session_context.shared_config_path = config_path.to_string_lossy().into_owned();
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    create_ccc_status_payload(&session_context, &locator).expect("status payload")
}

#[test]
fn graph_context_readiness_reports_default_off_config_disabled() {
    let config_home = create_temp_path("graph-context-default-off-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_toml_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_json_path = config_home.join("ccc").join("ccc-config.json");
    ensure_ccc_config_file_at(&config_path, &legacy_toml_path, &legacy_json_path)
        .expect("create default config");
    let config = read_optional_toml_document(&config_path)
        .expect("read generated config")
        .expect("generated config");
    let workspace_dir = create_temp_path("graph-context-default-off-workspace");
    create_dir_all(&workspace_dir).expect("create workspace");

    let readiness = graph_context::create_graph_context_readiness_payload(&config, &workspace_dir)
        .expect("graph context readiness");

    assert_eq!(readiness["feature_enabled"], false);
    assert_eq!(readiness["enabled"], false);
    assert_eq!(readiness["provider"], "graphify");
    assert_eq!(readiness["mode"], "read_only");
    assert_eq!(readiness["canonical_backend"], "graphify");
    assert_eq!(readiness["allow_legacy_graph_backend_fallback"], false);
    assert_eq!(
        readiness["fallback_when_unavailable"],
        "scout_source_evidence"
    );
    assert_eq!(readiness["source_of_truth"], false);
    assert_eq!(readiness["provider_enabled"], false);
    assert_eq!(readiness["readiness"], "disabled");
    assert_eq!(readiness["reason"], "graph_context_default_off");
    assert_eq!(readiness["fallback"], "legacy_code_graph");
    assert_eq!(
        readiness["routing"]["ccc_code_graph_backend"],
        "legacy_code_graph"
    );

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn status_graph_context_default_off_is_visible_without_changing_code_graph() {
    let config_home = create_temp_path("status-graph-context-default-off-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_status_graph_context_config(&config_path, false);
    let workspace_dir = create_temp_path("status-graph-context-default-off");
    create_dir_all(&workspace_dir).expect("create workspace");

    let status_payload = status_payload_with_graph_context_config(
        &workspace_dir,
        "run-graph-context-off",
        &config_path,
    );

    assert_eq!(
        status_payload["graph_context"]["schema"],
        "ccc.graph_context_readiness.status.v1"
    );
    assert_eq!(status_payload["graph_context"]["readiness"], "disabled");
    assert_eq!(
        status_payload["graph_context"]["reason"],
        "graph_context_default_off"
    );
    assert_eq!(
        status_payload["graph_context"]["fallback"],
        "legacy_code_graph"
    );
    assert!(status_payload.get("code_graph").is_some());

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(compact["graph_context"]["readiness"], "disabled");
    assert_eq!(compact["graph_context"]["fallback"], "legacy_code_graph");
    assert!(compact.get("code_graph").is_some());

    let app_panel = crate::status_app_panel::create_codex_app_panel_payload(&status_payload);
    assert_eq!(
        app_panel["workspace_state"]["graph_context"]["readiness"],
        "disabled"
    );
    let app_panel_text = create_codex_app_panel_text(&app_panel);
    assert!(app_panel_text.contains(
        "Graph Context: provider=graphify readiness=disabled fallback=legacy_code_graph artifacts=missing"
    ));

    let status_text = create_ccc_status_operator_text(&status_payload);
    assert!(status_text.contains(
        "Graph Context: provider=graphify readiness=disabled reason=graph_context_default_off fallback=legacy_code_graph artifacts=missing"
    ));
    assert!(!status_text.contains("graph-informed planning next step"));

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn check_install_graph_context_default_off_is_non_blocking() {
    let config_home = create_temp_path("check-install-graph-context-default-off");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_toml_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_json_path = config_home.join("ccc").join("ccc-config.json");
    ensure_ccc_config_file_at(&config_path, &legacy_toml_path, &legacy_json_path)
        .expect("create default config");
    let config = read_optional_toml_document(&config_path)
        .expect("read generated config")
        .expect("generated config");
    let workspace_dir = create_temp_path("check-install-graph-context-default-off-workspace");
    create_dir_all(&workspace_dir).expect("create workspace");

    let readiness = crate::install_check::create_graph_context_check_install_readiness_payload(
        &config,
        &workspace_dir,
    );

    assert_eq!(
        readiness["schema"],
        "ccc.graph_context_readiness.check_install.v1"
    );
    assert_eq!(readiness["readiness"], "disabled");
    assert_eq!(readiness["reason"], "graph_context_default_off");
    assert_eq!(readiness["fallback"], "legacy_code_graph");
    assert_eq!(readiness["check_install_status"], "disabled");
    assert_eq!(readiness["check_install_blocking"], false);

    let text = create_check_install_text(&json!({
        "graphContextReadiness": readiness,
    }));
    assert!(text.contains("Graph context: readiness=disabled"));
    assert!(text.contains("fallback=legacy_code_graph"));

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_readiness_reports_missing_artifacts_with_scout_fallback() {
    let workspace_dir = create_temp_path("graph-context-missing");
    create_dir_all(&workspace_dir).expect("create workspace");

    let readiness = graph_context::create_graph_context_readiness_payload(
        &graph_context_enabled_test_config(),
        &workspace_dir,
    )
    .expect("graph context readiness");

    assert_eq!(readiness["provider_enabled"], true);
    assert_eq!(readiness["readiness"], "unavailable");
    assert_eq!(readiness["reason"], "missing_artifacts");
    assert_eq!(readiness["fallback"], "scout_source_evidence");
    assert_eq!(readiness["artifacts"]["report"]["available"], false);
    assert_eq!(readiness["artifacts"]["graph"]["available"], false);
    assert_eq!(readiness["missing_artifacts"], json!(["report", "graph"]));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_code_graph_routing_default_off_preserves_legacy_behavior() {
    let config_home = create_temp_path("graph-context-routing-default-off-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_graph_context_routing_config(&config_path, false, None, None);
    let workspace_dir = create_temp_path("graph-context-routing-default-off");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("lib.rs"),
        "pub fn legacy_graph_entry() {}\n",
    )
    .expect("write source");

    let arguments = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "file_summary",
        "paths": ["src/lib.rs"],
        "update": true
    });
    let routed = graph_context::create_graph_context_code_graph_payload_for_config_path(
        &arguments,
        &config_path,
    )
    .expect("graph context route");
    assert!(routed.is_none());

    let legacy_payload = code_graph::create_code_graph_payload(&arguments).expect("legacy graph");
    assert_eq!(legacy_payload["updated"], true);
    assert_eq!(
        legacy_payload["query_result"]["file_summaries"][0]["path"],
        "src/lib.rs"
    );
    assert!(code_graph::default_graph_store_path(&workspace_dir).exists());

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_enabled_missing_artifacts_routes_to_scout_without_legacy_store() {
    let config_home = create_temp_path("graph-context-routing-missing-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_graph_context_routing_config(&config_path, true, None, Some(true));
    let workspace_dir = create_temp_path("graph-context-routing-missing");
    create_dir_all(&workspace_dir).expect("create workspace");

    let parsed = parse_cli_command_input(
        "graph",
        &[
            "--json".to_string(),
            json!({
                "cwd": workspace_dir.to_string_lossy(),
                "query": "review_context",
                "update": true
            })
            .to_string(),
        ],
        false,
    )
    .expect("parse graph cli");
    let routed = graph_context::create_graph_context_code_graph_payload_for_config_path(
        &parsed.payload,
        &config_path,
    )
    .expect("graph context route")
    .expect("graph context payload");

    assert_eq!(routed["readiness"], "unavailable");
    assert_eq!(routed["reason"], "missing_artifacts");
    assert_eq!(routed["fallback"], "scout_source_evidence");
    assert_eq!(routed["routing"]["legacy_code_graph_called"], false);
    assert_eq!(routed["routing"]["legacy_fallback_disabled"], true);
    assert_eq!(routed["routing"]["update_ignored"], true);
    assert!(!workspace_dir.join(".ccc").exists());

    let mut session_context = create_session_context();
    session_context.shared_config_path = config_path.to_string_lossy().into_owned();
    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 89,
            "method": "tools/call",
            "params": {
                "name": "ccc_code_graph",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "query": "review_context",
                    "update": true
                }
            }
        }),
    )
    .expect("response");

    let code_graph_payload = &response["result"]["structuredContent"]["code_graph"];
    assert_eq!(
        code_graph_payload["schema"],
        "ccc.graph_context_code_graph.v1"
    );
    assert_eq!(code_graph_payload["fallback"], "scout_source_evidence");
    assert_eq!(
        code_graph_payload["routing"]["ccc_code_graph_backend"],
        "graph_context_scout_source_evidence"
    );
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("legacy code graph fallback disabled"));
    assert!(!workspace_dir.join(".ccc").exists());

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_enabled_mcp_query_default_false_uses_scout_fallback() {
    let config_home = create_temp_path("graph-context-mcp-default-false-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_graph_context_routing_config(&config_path, true, None, None);
    let mut session_context = create_session_context();
    session_context.shared_config_path = config_path.to_string_lossy().into_owned();
    let workspace_dir = create_temp_path("graph-context-mcp-default-false");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("lib.rs"),
        "pub fn mcp_default_legacy() {}\n",
    )
    .expect("write source");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 90,
            "method": "tools/call",
            "params": {
                "name": "ccc_code_graph",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "query": "file_summary",
                    "paths": ["src/lib.rs"],
                    "update": true
                }
            }
        }),
    )
    .expect("response");

    let code_graph_payload = &response["result"]["structuredContent"]["code_graph"];
    assert_eq!(
        code_graph_payload["schema"],
        "ccc.graph_context_code_graph.v1"
    );
    assert_eq!(code_graph_payload["readiness"], "unavailable");
    assert_eq!(code_graph_payload["reason"], "missing_artifacts");
    assert_eq!(code_graph_payload["fallback"], "scout_source_evidence");
    assert_eq!(
        code_graph_payload["routing"]["ccc_code_graph_backend"],
        "graph_context_scout_source_evidence"
    );
    assert_eq!(
        code_graph_payload["routing"]["graphify_queries_enabled"],
        false
    );
    assert_eq!(
        code_graph_payload["routing"]["legacy_code_graph_called"],
        false
    );
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("fallback=scout_source_evidence"));
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("legacy code graph fallback disabled"));
    assert!(!code_graph::default_graph_store_path(&workspace_dir).exists());
    assert!(!workspace_dir.join(".ccc").exists());

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn status_graph_context_enabled_missing_artifacts_uses_scout_fallback() {
    let config_home = create_temp_path("status-graph-context-missing-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_status_graph_context_config(&config_path, true);
    let workspace_dir = create_temp_path("status-graph-context-missing");
    create_dir_all(&workspace_dir).expect("create workspace");

    let status_payload = status_payload_with_graph_context_config(
        &workspace_dir,
        "run-graph-context-missing",
        &config_path,
    );

    assert_eq!(status_payload["graph_context"]["readiness"], "unavailable");
    assert_eq!(
        status_payload["graph_context"]["reason"],
        "missing_artifacts"
    );
    assert_eq!(
        status_payload["graph_context"]["fallback"],
        "scout_source_evidence"
    );
    assert_eq!(status_payload["graph_context"]["artifact_state"], "missing");
    assert_eq!(
        status_payload["graph_context"]["routing"]["ccc_graph_backend"],
        "graph_context_scout_source_evidence"
    );
    assert_eq!(
        status_payload["graph_context"]["routing"]["ccc_code_graph_backend"],
        "graph_context_scout_source_evidence"
    );
    assert_eq!(
        status_payload["graph_context"]["routing"]["legacy_fallback_disabled"],
        true
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(compact["graph_context"]["readiness"], "unavailable");
    assert_eq!(compact["graph_context"]["artifacts"]["report"], false);
    assert_eq!(compact["graph_context"]["artifacts"]["graph"], false);
    assert_eq!(
        compact["command_templates"]["graph"]["payload"]["query"],
        "review_context"
    );
    assert_eq!(
        compact["command_templates"]["graph"]["payload"]["update"],
        false
    );

    let app_panel = crate::status_app_panel::create_codex_app_panel_payload(&status_payload);
    assert_eq!(
        app_panel["workspace_state"]["graph_context"]["fallback"],
        "scout_source_evidence"
    );
    let app_panel_text = create_codex_app_panel_text(&app_panel);
    assert!(app_panel_text.contains(
        "Graph Context: provider=graphify readiness=unavailable fallback=scout_source_evidence artifacts=missing"
    ));

    let status_text = create_ccc_status_operator_text(&status_payload);
    assert!(status_text.contains(
        "Graph Context: provider=graphify readiness=unavailable reason=missing_artifacts fallback=scout_source_evidence artifacts=missing"
    ));
    assert!(!status_text.contains("Graphify graph_context is active"));

    let tools = crate::mcp_tools::create_tools_list_response(Some(json!(1)));
    let tool_names = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"ccc_code_graph"));
    assert!(!tool_names.contains(&"ccc_graph_context"));

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn status_graph_context_enabled_inspection_error_uses_scout_fallback() {
    let config_home = create_temp_path("status-graph-context-inspection-error-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    write(
        &config_path,
        r#"[features]
graph_context = true

[graph_context]
enabled = true
provider = "graphify"
mode = "read_only"
canonical_backend = "graphify"
allow_legacy_graph_backend_fallback = false
fallback_when_unavailable = "scout_source_evidence"
report_path = "not-a-directory/GRAPH_REPORT.md"
graph_path = "graphify-out/graph.json"
source_of_truth = false

[runtime]
"#,
    )
    .expect("write graph context status config");
    let workspace_dir = create_temp_path("status-graph-context-inspection-error");
    create_dir_all(&workspace_dir).expect("create workspace");
    write(workspace_dir.join("not-a-directory"), "file").expect("write blocking file");

    let status_payload = status_payload_with_graph_context_config(
        &workspace_dir,
        "run-graph-context-inspection-error",
        &config_path,
    );

    assert_eq!(status_payload["graph_context"]["readiness"], "unavailable");
    assert_eq!(
        status_payload["graph_context"]["reason"],
        "inspection_error"
    );
    assert_eq!(
        status_payload["graph_context"]["fallback"],
        "scout_source_evidence"
    );
    assert_eq!(
        status_payload["graph_context"]["artifact_state"],
        "inspection_error"
    );
    assert_eq!(
        status_payload["graph_context"]["routing"]["ccc_graph_backend"],
        "graph_context_scout_source_evidence"
    );
    assert_eq!(
        status_payload["graph_context"]["routing"]["ccc_code_graph_backend"],
        "graph_context_scout_source_evidence"
    );
    assert_eq!(
        status_payload["graph_context"]["routing"]["legacy_code_graph_called"],
        false
    );
    assert_eq!(
        status_payload["graph_context"]["routing"]["legacy_fallback_disabled"],
        true
    );

    let status_text = create_ccc_status_operator_text(&status_payload);
    assert!(status_text.contains(
        "Graph Context: provider=graphify readiness=unavailable reason=inspection_error fallback=scout_source_evidence artifacts=inspection_error"
    ));

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn check_install_graph_context_enabled_missing_artifacts_warns_without_blocking() {
    let workspace_dir = create_temp_path("check-install-graph-context-missing");
    create_dir_all(&workspace_dir).expect("create workspace");

    let readiness = crate::install_check::create_graph_context_check_install_readiness_payload(
        &graph_context_enabled_test_config(),
        &workspace_dir,
    );

    assert_eq!(readiness["readiness"], "unavailable");
    assert_eq!(readiness["reason"], "missing_artifacts");
    assert_eq!(readiness["fallback"], "scout_source_evidence");
    assert_eq!(readiness["missing_artifacts"], json!(["report", "graph"]));
    assert_eq!(readiness["check_install_status"], "warning");
    assert_eq!(readiness["check_install_blocking"], false);

    let text = create_check_install_text(&json!({
        "graphContextReadiness": readiness,
    }));
    assert!(text.contains("Graph context: readiness=unavailable"));
    assert!(text.contains("check_install=warning"));
    assert!(text.contains("fallback=scout_source_evidence"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_readiness_reports_available_and_stale_artifacts() {
    let workspace_dir = create_temp_path("graph-context-artifacts");
    create_dir_all(workspace_dir.join("graphify-out")).expect("create graphify out");
    write(
        workspace_dir.join("graphify-out").join("GRAPH_REPORT.md"),
        "# Graphify report\n",
    )
    .expect("write report");
    write(
        workspace_dir.join("graphify-out").join("graph.json"),
        "{\"nodes\":[]}\n",
    )
    .expect("write graph");

    let available = graph_context::create_graph_context_readiness_payload(
        &graph_context_enabled_test_config(),
        &workspace_dir,
    )
    .expect("graph context readiness");
    assert_eq!(available["readiness"], "available");
    assert_eq!(available["reason"], "artifacts_available");
    assert_eq!(available["fallback"], Value::Null);
    assert_eq!(available["artifacts"]["report"]["available"], true);
    assert_eq!(available["artifacts"]["graph"]["available"], true);

    std::thread::sleep(std::time::Duration::from_millis(1100));
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("lib.rs"),
        "pub fn newer() {}\n",
    )
    .expect("write newer source");

    let stale = graph_context::create_graph_context_readiness_payload(
        &graph_context_enabled_test_config(),
        &workspace_dir,
    )
    .expect("graph context readiness");
    assert_eq!(stale["readiness"], "stale");
    assert_eq!(stale["reason"], "stale_artifacts");
    assert_eq!(stale["fallback"], "scout_source_evidence");
    assert_eq!(stale["stale"]["is_stale"], true);
    assert!(stale["stale"]["latest_source_path"]
        .as_str()
        .unwrap_or_default()
        .ends_with("src/lib.rs"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_enabled_available_artifacts_returns_bounded_read_only_payload() {
    let config_home = create_temp_path("graph-context-routing-available-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_graph_context_routing_config(&config_path, true, Some(12), None);
    let workspace_dir = create_temp_path("graph-context-routing-available");
    create_dir_all(workspace_dir.join("graphify-out")).expect("create graphify out");
    write(
        workspace_dir.join("graphify-out").join("GRAPH_REPORT.md"),
        "0123456789abcdef graph report tail\n",
    )
    .expect("write report");
    write(
        workspace_dir.join("graphify-out").join("graph.json"),
        r#"{"nodes":[{"id":"FULL_GRAPH_JSON_SHOULD_NOT_BE_DUMPED"}],"edges":[]}"#,
    )
    .expect("write graph");

    let payload = graph_context::create_graph_context_code_graph_payload_for_config_path(
        &json!({
            "cwd": workspace_dir.to_string_lossy(),
            "query": "architecture_overview",
            "update": false
        }),
        &config_path,
    )
    .expect("graph context route")
    .expect("graph context payload");

    assert_eq!(payload["readiness"], "available");
    assert_eq!(payload["backend"], "graphify_read_only_artifacts");
    assert_eq!(payload["fallback"], Value::Null);
    assert_eq!(payload["report"]["content"], "0123456789ab");
    assert_eq!(payload["report"]["truncated"], true);
    assert_eq!(payload["report"]["max_report_bytes"], 12);
    assert_eq!(payload["graph_metadata"]["content_loaded"], false);
    assert_eq!(payload["graph_metadata"]["content_policy"], "metadata_only");
    assert!(
        payload["graph_metadata"]["artifact"]["bytes"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert_eq!(payload["routing"]["legacy_code_graph_called"], false);
    assert!(!serde_json::to_string(&payload)
        .expect("serialize payload")
        .contains("FULL_GRAPH_JSON_SHOULD_NOT_BE_DUMPED"));
    assert!(!workspace_dir.join(".ccc").exists());

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn check_install_graph_context_available_artifacts_reports_ok() {
    let workspace_dir = create_temp_path("check-install-graph-context-available");
    create_dir_all(workspace_dir.join("graphify-out")).expect("create graphify out");
    write(
        workspace_dir.join("graphify-out").join("GRAPH_REPORT.md"),
        "# Graphify report\n",
    )
    .expect("write report");
    write(
        workspace_dir.join("graphify-out").join("graph.json"),
        "{\"nodes\":[]}\n",
    )
    .expect("write graph");

    let readiness = crate::install_check::create_graph_context_check_install_readiness_payload(
        &graph_context_enabled_test_config(),
        &workspace_dir,
    );

    assert_eq!(readiness["readiness"], "available");
    assert_eq!(readiness["fallback"], Value::Null);
    assert_eq!(readiness["check_install_status"], "ok");
    assert_eq!(readiness["check_install_blocking"], false);

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn graph_context_readiness_keeps_public_code_graph_surface_unchanged() {
    let tools = crate::mcp_tools::create_tools_list_response(Some(json!(1)));
    let tool_names = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();

    assert!(tool_names.contains(&"ccc_code_graph"));
    assert!(!tool_names
        .iter()
        .any(|name| name.contains("graphify") || name.contains("graph_context")));
}

#[test]
fn ensure_ccc_config_file_migrates_legacy_json_with_nulls_to_toml() {
    let config_home = create_temp_path("config-migrate");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(&legacy_dir).expect("create legacy dir");
    write(
        legacy_dir.join("ccc-config.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "output": {
                "verbosity": "quiet"
            },
            "agents": {
                "orchestrator": {
                    "name": "captain",
                    "profile": null,
                    "model": "gpt-5.4",
                    "variant": "high",
                    "fast_mode": false,
                    "config_entries": []
                },
                "way": {
                    "name": "tactician",
                    "profile": null,
                    "model": "gpt-5.4",
                    "variant": "medium",
                    "fast_mode": true,
                    "config_entries": []
                },
                "code specialist": {
                    "name": "raider",
                    "profile": null,
                    "model": "gpt-5.3-codex",
                    "variant": "high",
                    "fast_mode": true,
                    "config_entries": []
                },
                "documenter": {
                    "name": "scribe",
                    "profile": null,
                    "model": "gpt-5.4-mini",
                    "variant": "medium",
                    "fast_mode": false,
                    "config_entries": []
                },
                "verifier": {
                    "name": "arbiter",
                    "profile": null,
                    "model": "gpt-5.4",
                    "variant": "medium",
                    "fast_mode": true,
                    "config_entries": []
                }
            }
        }))
        .expect("serialize legacy config"),
    )
    .expect("write legacy config");

    let config_path = config_home.join("ccc").join("ccc-config.toml");
    let (created_path, created) = ensure_ccc_config_file_at(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("migrate config");
    assert!(created);
    assert_eq!(created_path, config_path);

    let contents = fs::read_to_string(&config_path).expect("read migrated config");
    assert!(!contents.contains("null"));
    assert!(!contents.contains("profile ="));
    toml::from_str::<toml::Value>(&contents).expect("parse migrated toml");

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_reports_legacy_json_as_legacy_only() {
    let config_home = create_temp_path("config-legacy-only");
    let canonical_dir = config_home.join("ccc");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(&legacy_dir).expect("create config dir");
    write(
        legacy_dir.join("ccc-config.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "output": { "verbosity": "quiet" },
            "agents": {}
        }))
        .expect("serialize legacy config"),
    )
    .expect("write legacy config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(
        &session_context,
        canonical_dir.join("ccc-config.toml"),
    );
    assert_eq!(payload["configExists"], Value::Bool(true));
    assert_eq!(payload["configCanonicalReady"], Value::Bool(false));
    assert_eq!(
        payload["configStatus"],
        Value::String("legacy-only".to_string())
    );
    assert!(create_check_install_text(&payload).contains("config=legacy-only"));
    assert!(create_check_install_text(&payload).contains("config_action=skipped"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_reports_legacy_toml_as_legacy_only_with_loaded_models() {
    let config_home = create_temp_path("config-legacy-toml-only");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(&legacy_dir).expect("create legacy dir");
    write(
        legacy_dir.join("ccc-config.toml"),
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write legacy toml");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);

    assert_eq!(
        payload["configStatus"],
        Value::String("canonical-needs-backfill".to_string())
    );
    assert_eq!(payload["configCanonicalReady"], Value::Bool(true));
    assert_eq!(
        payload["configuredRoleModels"][0]["model"],
        Value::String("gpt-5.4".to_string())
    );

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_flags_legacy_entry_policy_mode_without_rewriting_config() {
    let config_home = create_temp_path("config-legacy-entry-policy-mode");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    ensure_ccc_config_file_at(
        &canonical_path,
        &config_home.join("previous-ccc").join("ccc-config.toml"),
        &config_home.join("previous-ccc").join("ccc-config.json"),
    )
    .expect("create default config");
    let mut config = read_optional_toml_document(&canonical_path)
        .expect("read config")
        .expect("config value");
    config["entry_policy"]["mode"] = Value::String("codex_cli_foreman_first".to_string());
    write_toml_document(&canonical_path, &config).expect("write config");
    let before = fs::read_to_string(&canonical_path).expect("read before");

    let session_context = create_session_context();
    let payload =
        collect_install_check_payload_for_config_path(&session_context, canonical_path.clone());

    assert_eq!(
        payload["configStatus"],
        Value::String("canonical-needs-backfill".to_string())
    );
    assert_eq!(
        payload["configActionStatus"],
        Value::String("setup-backfill-available".to_string())
    );
    assert!(payload["configSummary"]
        .as_str()
        .unwrap_or_default()
        .contains("codex_cli_foreman_first"));
    assert!(payload["configSummary"]
        .as_str()
        .unwrap_or_default()
        .contains("codex_cli_ccc_first"));
    assert_eq!(
        payload["entryPolicyModeStatus"],
        Value::String("legacy/backfill-needed".to_string())
    );
    assert_eq!(
        payload["entryPolicyModeRaw"],
        Value::String("codex_cli_foreman_first".to_string())
    );
    assert_eq!(
        payload["entryPolicyModeCanonical"],
        Value::String("codex_cli_ccc_first".to_string())
    );
    assert!(payload["entryPolicyModeSummary"]
        .as_str()
        .unwrap_or_default()
        .contains("runtime-compatible"));
    assert_eq!(
        payload["installSurfaceVisibility"]["components"]["ccc_config"]["entry_policy_mode_status"],
        Value::String("legacy/backfill-needed".to_string())
    );
    let after = fs::read_to_string(&canonical_path).expect("read after");
    assert_eq!(after, before);

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_flags_invalid_entry_policy_mode_without_rewriting_config() {
    let config_home = create_temp_path("config-invalid-entry-policy-mode");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    ensure_ccc_config_file_at(
        &canonical_path,
        &config_home.join("previous-ccc").join("ccc-config.toml"),
        &config_home.join("previous-ccc").join("ccc-config.json"),
    )
    .expect("create default config");
    let mut config = read_optional_toml_document(&canonical_path)
        .expect("read config")
        .expect("config value");
    config["entry_policy"]["mode"] = Value::String("foreman_first".to_string());
    write_toml_document(&canonical_path, &config).expect("write config");
    let before = fs::read_to_string(&canonical_path).expect("read before");

    let session_context = create_session_context();
    let payload =
        collect_install_check_payload_for_config_path(&session_context, canonical_path.clone());

    assert_eq!(
        payload["configStatus"],
        Value::String("canonical-needs-backfill".to_string())
    );
    assert!(payload["configSummary"]
        .as_str()
        .unwrap_or_default()
        .contains("foreman_first"));
    assert!(payload["configSummary"]
        .as_str()
        .unwrap_or_default()
        .contains("guided_explicit"));
    assert_eq!(
        payload["entryPolicyModeStatus"],
        Value::String("invalid/unsupported".to_string())
    );
    assert_eq!(
        payload["entryPolicyModeRaw"],
        Value::String("foreman_first".to_string())
    );
    assert_eq!(
        payload["entryPolicyModeCanonical"],
        Value::String("guided_explicit".to_string())
    );
    assert!(payload["entryPolicyModeSummary"]
        .as_str()
        .unwrap_or_default()
        .contains("unsupported"));
    assert_eq!(
        payload["installSurfaceVisibility"]["components"]["ccc_config"]["entry_policy_mode_status"],
        Value::String("invalid/unsupported".to_string())
    );
    let after = fs::read_to_string(&canonical_path).expect("read after");
    assert_eq!(after, before);

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn ensure_ccc_config_file_migrates_legacy_toml_to_canonical_path() {
    let config_home = create_temp_path("config-migrate-legacy-toml");
    let config_dir = config_home.join("ccc");
    let legacy_dir = config_home.join("previous-ccc");
    create_dir_all(&legacy_dir).expect("create config dir");
    write(
        legacy_dir.join("ccc-config.toml"),
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write legacy toml");

    let config_path = config_dir.join("ccc-config.toml");
    let (created_path, created) = ensure_ccc_config_file_at(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("migrate legacy toml");
    assert!(created);
    assert_eq!(created_path, config_path);

    let contents = fs::read_to_string(&config_path).expect("read migrated config");
    assert!(contents.contains("[agents.orchestrator]"));
    assert!(contents.contains("model = \"gpt-5.4\""));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn ensure_ccc_config_file_migrates_previous_ccc_ccc_config_to_canonical_path() {
    let config_home = create_temp_path("config-migrate-previous-ccc");
    let canonical_dir = config_home.join("ccc");
    let legacy_dir = config_home.join("previous-ccc");
    create_dir_all(&legacy_dir).expect("create legacy dir");
    write(
        legacy_dir.join("ccc-config.toml"),
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write previous ccc config");

    let config_path = canonical_dir.join("ccc-config.toml");
    let (created_path, created) = ensure_ccc_config_file_at(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("migrate previous ccc config");
    assert!(created);
    assert_eq!(created_path, config_path);

    let contents = fs::read_to_string(&config_path).expect("read migrated config");
    assert!(contents.contains("[agents.orchestrator]"));
    assert!(contents.contains("model = \"gpt-5.4\""));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_reports_canonical_path_while_reading_previous_ccc_ccc_config() {
    let config_home = create_temp_path("config-report-previous-ccc");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let previous_dir = config_home.join("previous-ccc");
    create_dir_all(&previous_dir).expect("create previous config dir");
    write(
        previous_dir.join("ccc-config.toml"),
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write previous ccc config");

    let session_context = create_session_context();
    let payload =
        collect_install_check_payload_for_config_path(&session_context, canonical_path.clone());
    assert_eq!(
        payload["configPath"].as_str().unwrap_or_default(),
        canonical_path.to_string_lossy().as_ref()
    );
    assert_eq!(payload["configExists"], Value::Bool(false));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_state_reports_migrated_from_previous_and_restart_required() {
    let config_home = create_temp_path("config-setup-migrate-previous");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let previous_dir = config_home.join("previous-ccc");
    create_dir_all(&previous_dir).expect("create previous dir");
    write(
        previous_dir.join("ccc-config.toml"),
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write previous config");

    let (_, created, state) = ensure_ccc_config_file_at_with_state(
        &canonical_path,
        &previous_dir.join("ccc-config.toml"),
        &previous_dir.join("ccc-config.json"),
    )
    .expect("migrate previous config");

    assert!(created);
    assert_eq!(state.status, "migrated-from-previous");
    assert_eq!(state.action_status, "migrated-from-previous");
    assert_eq!(state.restart_status, "restart-required");
    assert_eq!(state.backup_status, "created");
    let backup_path = state.backup_path.expect("backup path");
    assert!(backup_path.exists());
    assert!(backup_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .starts_with("ccc-config.toml."));
    assert!(backup_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .ends_with(".bak"));
    assert!(canonical_path.exists());
    let backup_contents = fs::read_to_string(&backup_path).expect("read backup");
    assert!(backup_contents.contains("model = \"gpt-5.4\""));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_dry_run_reports_backfill_without_writing() {
    let config_home = create_temp_path("config-dry-run-backfill");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    let original = r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#;
    write(&canonical_path, original).expect("write canonical config");

    let plan = plan_ccc_config_setup_at(
        &canonical_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("plan dry-run");

    assert_eq!(plan.status, "canonical-needs-backfill");
    assert_eq!(plan.action_status, "would-backfill");
    assert_eq!(plan.backup_status, "setup-backup-available");
    assert_eq!(plan.restart_status, "restart-required-after-setup");
    assert_eq!(
        fs::read_to_string(&canonical_path).expect("read canonical"),
        original
    );
    assert!(backup_paths_for(&canonical_path).is_empty());

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_state_reports_backfilled_canonical_config() {
    let config_home = create_temp_path("config-setup-backfilled");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write canonical config");

    let (_, created, state) = ensure_ccc_config_file_at_with_state(
        &canonical_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("backfill canonical config");

    assert!(!created);
    assert_eq!(state.status, "canonical-current");
    assert_eq!(state.action_status, "backfilled");
    assert_eq!(state.backup_status, "created");
    assert_eq!(state.restart_status, "restart-required");
    let backup_path = state.backup_path.expect("backup path");
    assert!(backup_path.exists());
    let backup_contents = fs::read_to_string(&backup_path).expect("read backup");
    assert!(backup_contents.contains("model = \"gpt-5.4\""));
    assert!(!backup_contents.contains("[companion_agents.companion_reader]"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_backfill_backup_preserves_existing_user_values() {
    let config_home = create_temp_path("config-backfill-backup-preserve");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[tool_routing]
default_model = "custom-mini"

[tool_routing.tools.git]
allowed_operations = ["read", "mutation"]
owner_companion_agent = "custom_reader"
mutation_owner_companion_agent = "custom_operator"
"#,
    )
    .expect("write canonical config");

    let (_, created, state) = ensure_ccc_config_file_at_with_state(
        &canonical_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("backfill config");

    assert!(!created);
    assert_eq!(state.action_status, "backfilled");
    assert_eq!(state.backup_status, "created");
    let refreshed = read_optional_toml_document(&canonical_path)
        .expect("read refreshed config")
        .expect("refreshed config");
    assert_eq!(refreshed["tool_routing"]["default_model"], "custom-mini");
    assert_eq!(
        refreshed["tool_routing"]["tools"]["git"]["owner_companion_agent"],
        "custom_reader"
    );
    assert_eq!(
        refreshed["tool_routing"]["tools"]["git"]["mutation_owner_companion_agent"],
        "custom_operator"
    );
    assert_eq!(
        refreshed["tool_routing"]["tools"]["gh"]["owner_companion_agent"],
        "companion_reader"
    );
    let backup_contents =
        fs::read_to_string(state.backup_path.expect("backup path")).expect("read backup");
    assert!(backup_contents.contains("default_model = \"custom-mini\""));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_rollback_config_restores_backup_to_canonical_path() {
    let config_home = create_temp_path("config-rollback-restore");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let backup_path = config_home.join("backups").join("ccc-config.toml.backup");
    create_dir_all(backup_path.parent().unwrap()).expect("create backup dir");
    write(
        &backup_path,
        r#"
version = 1

[tool_routing]
default_model = "custom-rollback-mini"

[tool_routing.tools.git]
allowed_operations = ["read", "mutation"]
owner_companion_agent = "rollback_reader"
mutation_owner_companion_agent = "rollback_operator"
"#,
    )
    .expect("write backup config");

    let state =
        rollback_ccc_config_from_backup_at(&canonical_path, &backup_path).expect("restore backup");

    assert_eq!(state.status, "rollback-restored");
    assert_eq!(state.action_status, "rolled-back");
    assert_eq!(state.backup_status, "restored");
    assert_eq!(state.restart_status, "restart-required");
    assert_eq!(state.backup_path.as_ref(), Some(&backup_path));
    assert!(canonical_path.exists());
    let restored = read_optional_toml_document(&canonical_path)
        .expect("read restored config")
        .expect("restored config");
    assert_eq!(
        restored["tool_routing"]["default_model"],
        "custom-rollback-mini"
    );
    assert_eq!(
        restored["tool_routing"]["tools"]["git"]["owner_companion_agent"],
        "rollback_reader"
    );

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_rollback_config_rejects_missing_backup_path() {
    let config_home = create_temp_path("config-rollback-missing");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let missing_backup_path = config_home.join("missing").join("ccc-config.toml.bak");

    let error = rollback_ccc_config_from_backup_at(&canonical_path, &missing_backup_path)
        .expect_err("missing backup should fail");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!canonical_path.exists());

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_rollback_config_rejects_directory_backup_path() {
    let config_home = create_temp_path("config-rollback-directory");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let directory_backup_path = config_home.join("backup-dir");
    create_dir_all(&directory_backup_path).expect("create directory backup path");

    let error = rollback_ccc_config_from_backup_at(&canonical_path, &directory_backup_path)
        .expect_err("directory backup should fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!canonical_path.exists());

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_reports_conflicting_previous_config() {
    let config_home = create_temp_path("config-conflict");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    let previous_dir = config_home.join("ccc");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    create_dir_all(&previous_dir).expect("create previous dir");
    write(
        &canonical_path,
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.5"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write canonical config");
    write(
        previous_dir.join("ccc-config.toml"),
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write previous config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);
    let text = create_check_install_text(&payload);

    assert_eq!(payload["status"], Value::String("warning".to_string()));
    assert_eq!(
        payload["configStatus"],
        Value::String("canonical-needs-backfill".to_string())
    );
    assert_eq!(
        payload["configActionStatus"],
        Value::String("setup-backfill-available".to_string())
    );
    assert!(text.contains("config=canonical-needs-backfill"));
    assert!(text.contains("config_action=setup-backfill-available"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_surfaces_config_backfill_backup_and_restart_guidance() {
    let config_home = create_temp_path("config-check-backfill-guidance");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write canonical config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);
    let text = create_check_install_text(&payload);

    assert_eq!(
        payload["configStatus"],
        Value::String("canonical-needs-backfill".to_string())
    );
    assert_eq!(
        payload["configActionStatus"],
        Value::String("setup-backfill-available".to_string())
    );
    assert_eq!(
        payload["configBackupStatus"],
        Value::String("setup-backup-available".to_string())
    );
    assert_eq!(
        payload["configRestartStatus"],
        Value::String("restart-required-after-setup".to_string())
    );
    assert!(text.contains("config=canonical-needs-backfill"));
    assert!(text.contains("config_restart=restart-required-after-setup"));
    assert!(text.contains("Config backup: status=setup-backup-available"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn packaged_cap_skill_inspection_reports_matching_and_mismatched_install() {
    let root = create_temp_path("cap-skill-inspect");
    let source = root.join("source").join("SKILL.md");
    let installed = root.join("installed").join("SKILL.md");
    create_dir_all(source.parent().unwrap()).expect("create source dir");
    create_dir_all(installed.parent().unwrap()).expect("create installed dir");
    write(&source, "name: cap\nbody").expect("write source");
    write(&installed, "name: cap\nbody").expect("write installed");

    let matching =
        inspect_packaged_cap_skill_install_at(&source, &installed).expect("inspect matching");
    assert_eq!(matching["status"], "matching_install");
    assert_eq!(matching["action_status"], "preserved");
    assert_eq!(matching["restart_status"], "not-required");

    write(&installed, "name: cap\nold body").expect("write drifted install");
    let mismatched =
        inspect_packaged_cap_skill_install_at(&source, &installed).expect("inspect mismatch");
    assert_eq!(mismatched["status"], "mismatched_install");
    assert_eq!(mismatched["action_status"], "setup-refresh-available");
    assert_eq!(mismatched["restart_status"], "restart-required-after-setup");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn install_check_surfaces_skill_registry_health() {
    let config_home = create_temp_path("config-check-registry-health");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write canonical config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);
    let text = create_check_install_text(&payload);

    assert_eq!(
        payload["skillRegistryHealth"]["schema"],
        "ccc.skill_registry_health.v1"
    );
    assert_eq!(payload["skillRegistryHealth"]["agent_count"], 8);
    assert_eq!(payload["skillRegistryHealth"]["available_count"], 8);
    assert_eq!(payload["skillRegistryHealth"]["status"], "ok");
    assert!(payload["skillRegistryHealth"]["non_available"]
        .as_array()
        .expect("non available")
        .is_empty());
    assert_eq!(
        payload["executionContractRegistry"]["schema"],
        "ccc.execution_contract.v1"
    );
    assert_eq!(payload["executionContractRegistry"]["role_count"], 8);
    assert!(text.contains("Skill registry: status=ok available=8/8"));
    assert!(text.contains("Execution contracts: status=available roles=8"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_surface_visibility_summarizes_restart_required_components() {
    let config_state = CccConfigInstallState {
        status: "canonical-needs-backfill",
        action_status: "setup-backfill-available",
        backup_status: "setup-backup-available",
        summary: "Config needs generated-default backfill.".to_string(),
        source_path: None,
        backup_source_path: None,
        backup_path: None,
        value: Value::Null,
        canonical_ready: true,
        config_exists: true,
        restart_status: "restart-required-after-setup",
        entry_policy_mode_status: "canonical",
        entry_policy_mode_raw: Some("guided_explicit".to_string()),
        entry_policy_mode_canonical: Some("guided_explicit".to_string()),
        entry_policy_mode_summary:
            "Entry policy mode `guided_explicit` is canonical and supported.".to_string(),
    };
    let visibility = create_install_surface_visibility_payload(
        create_registration_visibility_payload(
            "matching_registration",
            "MCP registration is current.",
        ),
        create_config_visibility_payload(&config_state),
        create_skill_visibility_payload(&json!({
            "status": "mismatched_install",
            "action_status": "setup-refresh-available",
            "restart_status": "restart-required-after-setup",
            "summary": "$cap skill needs refresh."
        })),
        create_custom_agent_visibility_payload(&json!({
            "status": "mismatched_sync",
            "summary": "Custom agents need resync.",
            "missing_files": [],
            "mismatched_files": ["ccc-raider.toml"],
            "stale_managed_files": []
        })),
    );

    assert_eq!(visibility["status"], "stale");
    assert_eq!(visibility["restart_required"], true);
    assert_eq!(visibility["setup_refresh_recommended"], true);
    assert_eq!(visibility["components"]["cap_skill"]["status"], "stale");
    assert_eq!(
        visibility["components"]["custom_agents"]["action_status"],
        "setup-sync-available"
    );
}

#[test]
fn check_install_status_warns_for_non_current_config_surface_readiness() {
    let config_state = CccConfigInstallState {
        status: "canonical-current",
        action_status: "preserved",
        backup_status: "not-required",
        summary: "Canonical config is current.".to_string(),
        source_path: None,
        backup_source_path: None,
        backup_path: None,
        value: json!({}),
        canonical_ready: true,
        config_exists: true,
        restart_status: "not-required",
        entry_policy_mode_status: "canonical",
        entry_policy_mode_raw: Some("guided_explicit".to_string()),
        entry_policy_mode_canonical: Some("guided_explicit".to_string()),
        entry_policy_mode_summary:
            "Entry policy mode `guided_explicit` is canonical and supported.".to_string(),
    };

    assert_eq!(
        crate::install_check::create_check_install_status(
            "matching_registration",
            &config_state,
            "matching_install",
            "matching_sync",
            &json!({ "status": "current" }),
        ),
        "ok"
    );
    for readiness_status in ["missing", "stale", "conflict"] {
        assert_eq!(
            crate::install_check::create_check_install_status(
                "matching_registration",
                &config_state,
                "matching_install",
                "matching_sync",
                &json!({ "status": readiness_status }),
            ),
            "warning",
            "readiness status {readiness_status} should prevent a top-level ok status"
        );
    }
}

fn assert_config_readiness_counts_are_consistent(payload: &Value) {
    let readiness = &payload["configSurfaceReadiness"];
    let surfaces = readiness["surfaces"]
        .as_array()
        .expect("readiness surfaces");
    let count_with_status = |status: &str| {
        surfaces
            .iter()
            .filter(|surface| surface["status"].as_str() == Some(status))
            .count() as u64
    };

    assert_eq!(readiness["surface_count"], surfaces.len() as u64);
    assert_eq!(readiness["missing_count"], count_with_status("missing"));
    assert_eq!(
        readiness["optional_missing_count"],
        count_with_status("optional_missing")
    );
    assert_eq!(readiness["stale_count"], count_with_status("stale"));
    assert_eq!(readiness["conflict_count"], count_with_status("conflict"));
}

#[test]
fn check_install_text_includes_execution_contract_registry_visibility() {
    let text = create_check_install_text(&json!({
        "status": "warning",
        "packageVersion": "0.0.15-pre",
        "publicEntryLabel": "$cap",
        "registrationStatus": "missing_registration",
        "configStatus": "canonical-current",
        "configActionStatus": "preserved",
        "configRestartStatus": "not-required",
        "configBackupStatus": "not-required",
        "capSkillStatus": "matching_install",
        "customAgentStatus": "matching_sync",
        "installSurfaceVisibility": {
            "status": "current",
            "restart_required": false
        },
        "skillRegistryHealth": {
            "status": "ok",
            "available_count": 8,
            "agent_count": 8
        },
        "executionContractRegistry": {
            "status": "available",
            "role_count": 8
        },
        "expectedLaunchCommand": "/tmp/ccc",
        "expectedLaunchArgs": ["mcp"],
        "registrationSummary": "Registration summary.",
        "configSummary": "Config summary.",
        "capSkillSummary": "$cap summary.",
        "customAgentSummary": "Custom-agent summary."
    }));

    assert!(text.contains("Execution contracts: status=available roles=8"));
}

#[test]
fn install_check_surfaces_0_0_15_config_readiness() {
    let config_home = create_temp_path("config-check-0015-readiness");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[generated_defaults]
version = 16
policy = "ccc-managed-defaults"

[routing]
mode = "category_shortlist"

[routing.categories.write_code]
keywords = ["implement"]
intent_types = ["mutation"]
tool_signals = ["filesystem"]
agents = ["raider"]

[runtime]
preferred_specialist_execution_mode = "codex_subagent"
fallback_specialist_execution_mode = "codex_exec"

[runtime.host_subagent_concurrency]
default_provider_concurrency_limit = 4
default_model_concurrency_limit = 2

[runtime.lifecycle_hooks]
enabled = true

[prompt_sections.identity]
enabled = true

[prompt_sections.task]
enabled = true

[prompt_sections.routing]
enabled = true

[prompt_sections.hard_blocks]
enabled = true

[prompt_sections.evidence]
enabled = true

[prompt_sections.verification]
enabled = true

[prompt_sections.anti_duplication]
enabled = true

[prompt_sections.reporting]
enabled = true

[directory_rule_injection]
enabled = true
"#,
    )
    .expect("write canonical config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);
    let surfaces = payload["configSurfaceReadiness"]["surfaces"]
        .as_array()
        .expect("readiness surfaces");
    let status_for = |surface_id: &str| {
        surfaces
            .iter()
            .find(|surface| surface["surface"] == surface_id)
            .and_then(|surface| surface["status"].as_str())
            .unwrap_or("missing")
            .to_string()
    };

    assert_eq!(
        payload["configSurfaceReadiness"]["schema"],
        "ccc.config_surface_readiness.v1"
    );
    assert_config_readiness_counts_are_consistent(&payload);
    assert_eq!(payload["configSurfaceReadiness"]["surface_count"], 9);
    assert_eq!(
        payload["configSurfaceReadiness"]["surfaces"]
            .as_array()
            .expect("readiness surfaces")
            .len(),
        9
    );
    assert_eq!(status_for("registry"), "current");
    assert_eq!(status_for("category_routing"), "current");
    assert_eq!(status_for("fallback_policy"), "current");
    assert_eq!(status_for("concurrency"), "current");
    assert_eq!(status_for("prompt_sections"), "current");
    assert_eq!(status_for("directory_rule_injection"), "current");
    assert_eq!(status_for("hook_settings"), "current");
    assert_eq!(status_for("generated_defaults_version"), "current");
    assert_eq!(
        payload["configSurfaceReadiness"]["generated_defaults"]["status"],
        "current"
    );
    assert_eq!(payload["graphContextReadiness"]["readiness"], "disabled");
    assert_eq!(
        payload["graphContextReadiness"]["check_install_status"],
        "disabled"
    );
    assert_eq!(
        payload["graphContextReadiness"]["fallback"],
        "legacy_code_graph"
    );
    assert_eq!(
        payload["graphContextReadiness"]["check_install_blocking"],
        false
    );

    let text = create_check_install_text(&payload);
    assert!(text.contains("0.0.15 config surfaces: registry=current"));
    assert!(text.contains("category_routing=current"));
    assert!(text.contains("fallback_policy=current"));
    assert!(text.contains("concurrency=current"));
    assert!(text.contains("prompt_sections=current"));
    assert!(text.contains("directory_rule_injection=current"));
    assert!(text.contains("hook_settings=current"));
    assert!(text.contains("generated_defaults=current"));
    assert!(text.contains("Graph context: readiness=disabled"));
    assert!(text.contains("fallback=legacy_code_graph"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_keeps_optional_missing_config_surfaces_non_blocking() {
    let config_home = create_temp_path("config-check-0015-optional-missing");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[generated_defaults]
version = 15
policy = "ccc-managed-defaults"

[routing]
mode = "category_shortlist"

[routing.categories.write_code]
keywords = ["implement"]
intent_types = ["mutation"]
tool_signals = ["filesystem"]
agents = ["raider"]

[runtime]
preferred_specialist_execution_mode = "codex_subagent"
fallback_specialist_execution_mode = "codex_exec"
"#,
    )
    .expect("write canonical config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);
    let surfaces = payload["configSurfaceReadiness"]["surfaces"]
        .as_array()
        .expect("readiness surfaces");
    let status_for = |surface_id: &str| {
        surfaces
            .iter()
            .find(|surface| surface["surface"] == surface_id)
            .and_then(|surface| surface["status"].as_str())
            .unwrap_or("missing")
            .to_string()
    };

    assert_config_readiness_counts_are_consistent(&payload);
    assert_eq!(payload["configSurfaceReadiness"]["missing_count"], 0);
    assert_eq!(
        payload["configSurfaceReadiness"]["optional_missing_count"],
        4
    );
    assert_eq!(status_for("concurrency"), "optional_missing");
    assert_eq!(status_for("prompt_sections"), "optional_missing");
    assert_eq!(status_for("directory_rule_injection"), "optional_missing");
    assert_eq!(status_for("hook_settings"), "optional_missing");
    assert_eq!(status_for("fallback_policy"), "current");

    let text = create_check_install_text(&payload);
    assert!(text.contains("concurrency=optional_missing"));
    assert!(text.contains("prompt_sections=optional_missing"));
    assert!(text.contains("directory_rule_injection=optional_missing"));
    assert!(text.contains("hook_settings=optional_missing"));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn install_check_reports_missing_stale_and_conflict_0_0_15_surfaces() {
    let config_home = create_temp_path("config-check-0015-drift");
    let canonical_path = config_home.join("ccc").join("ccc-config.toml");
    create_dir_all(canonical_path.parent().unwrap()).expect("create canonical dir");
    write(
        &canonical_path,
        r#"
version = 1

[generated_defaults]
version = 11
policy = "ccc-managed-defaults"

[routing]
mode = "unsupported"

[runtime]
preferred_specialist_execution_mode = "codex_subagent"
fallback_specialist_execution_mode = "visible_degraded_host_fallback"
default_provider_concurrency_limit = 1

[runtime.lifecycle_hooks.tool_guard]
command = "ccc hook run"

[prompt_sections.identity]
enabled = true

[directory_rule_injection]
sources = ["AGENTS.md"]
"#,
    )
    .expect("write canonical config");

    let session_context = create_session_context();
    let payload = collect_install_check_payload_for_config_path(&session_context, canonical_path);
    let surfaces = payload["configSurfaceReadiness"]["surfaces"]
        .as_array()
        .expect("readiness surfaces");
    let status_for = |surface_id: &str| {
        surfaces
            .iter()
            .find(|surface| surface["surface"] == surface_id)
            .and_then(|surface| surface["status"].as_str())
            .unwrap_or("missing")
            .to_string()
    };

    assert_eq!(status_for("category_routing"), "conflict");
    assert_eq!(status_for("fallback_policy"), "stale");
    assert_eq!(status_for("concurrency"), "stale");
    assert_eq!(status_for("prompt_sections"), "stale");
    assert_eq!(status_for("directory_rule_injection"), "stale");
    assert_eq!(status_for("hook_settings"), "conflict");
    assert_eq!(status_for("generated_defaults_version"), "stale");
    assert_eq!(
        payload["configSurfaceReadiness"]["generated_defaults"]["status"],
        "stale"
    );
    assert_config_readiness_counts_are_consistent(&payload);
    assert_eq!(payload["configSurfaceReadiness"]["status"], "conflict");

    let text = create_check_install_text(&payload);
    assert!(text.contains("category_routing=conflict"));
    assert!(text.contains("fallback_policy=stale"));
    assert!(text.contains("generated_defaults=stale"));
    assert!(text.contains("Config setup safeguards: dry_run=\"ccc setup --dry-run\""));
    assert!(text.contains("rollback=\"ccc setup --rollback-config <backup_path>\""));

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn ensure_ccc_config_file_keeps_internal_defaults_out_of_minimal_config() {
    let config_home = create_temp_path("config-backfill-missing");
    let config_dir = config_home.join("ccc");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(&config_dir).expect("create config dir");
    let config_path = config_dir.join("ccc-config.toml");
    write(
        &config_path,
        r#"
version = 1

[agents.orchestrator]
name = "captain"
model = "gpt-5.4"
variant = "high"
fast_mode = false
config_entries = []
"#,
    )
    .expect("write config");

    let (returned_path, created) = ensure_ccc_config_file_at(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("backfill config");
    assert!(!created);
    assert_eq!(returned_path, config_path);

    let refreshed = read_optional_toml_document(&config_path)
        .expect("read refreshed config")
        .expect("refreshed config");
    assert!(refreshed.get("companion_agents").is_none());
    assert!(refreshed.get("routing").is_none());
    assert!(refreshed.get("tool_routing").is_none());
    assert!(refreshed.get("runtime").is_none());

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn setup_config_upgrades_stale_generated_defaults() {
    let config_home = create_temp_path("config-generated-drift");
    let config_dir = config_home.join("ccc");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(&config_dir).expect("create config dir");
    let config_path = config_dir.join("ccc-config.toml");
    write(
        &config_path,
        r#"
version = 1

[generated_defaults]
version = 1
policy = "ccc-managed-defaults"

[agents.explorer]
name = "scout"
summary = "Read-only repo investigation and evidence gathering."
model = "gpt-5.4-mini"
variant = "medium"
fast_mode = false
config_entries = []

[agents.documenter]
name = "scribe"
summary = "Docs and operator-facing text updates."
model = "gpt-5.4-mini"
variant = "high"
fast_mode = false
config_entries = []

[agents.way]
name = "tactician"
summary = "Way creation and bounded planning when the next move is still unclear."
model = "gpt-5.5"
variant = "medium"
fast_mode = true
config_entries = []

[agents.verifier]
name = "arbiter"
summary = "Review, regression detection, and acceptance judgment when needed."
model = "gpt-5.5"
variant = "medium"
fast_mode = true
config_entries = []

[companion_agents.companion_reader]
name = "companion_reader"
summary = "Read only"
model = "gpt-5.4-mini"
variant = "high"
fast_mode = false
config_entries = []

[companion_agents.companion_operator]
name = "companion_operator"
summary = "Operator"
model = "gpt-5.4-mini"
variant = "high"
fast_mode = false
config_entries = []

[runtime]
preferred_specialist_execution_mode = "codex_subagent"
fallback_specialist_execution_mode = "visible_degraded_host_fallback"
"#,
    )
    .expect("write stale generated config");

    let (_, created, state) = ensure_ccc_config_file_at_with_state(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("upgrade generated defaults");

    assert!(!created);
    assert_eq!(state.action_status, "backfilled");
    assert_eq!(state.backup_status, "created");
    let refreshed = read_optional_toml_document(&config_path)
        .expect("read refreshed config")
        .expect("refreshed config");
    assert_eq!(refreshed["generated_defaults"]["version"], 16);
    assert_eq!(refreshed["features"]["graph_context"], false);
    assert_eq!(refreshed["features"]["goals"], false);
    assert_eq!(refreshed["features"]["prompt_refinement"], false);
    assert_eq!(refreshed["goal_bridge"]["enabled"], false);
    assert_eq!(refreshed["goal_bridge"]["mode"], "captain_owned");
    assert_eq!(refreshed["goal_bridge"]["host_goal_state_is_truth"], false);
    assert_eq!(
        refreshed["goal_bridge"]["specialists"]["allow_specialist_set_goal"],
        false
    );
    assert_eq!(refreshed["graph_context"]["enabled"], false);
    assert_eq!(refreshed["graph_context"]["provider"], "graphify");
    assert_eq!(refreshed["graph_context"]["canonical_backend"], "graphify");
    assert_eq!(
        refreshed["graph_context"]["allow_legacy_graph_backend_fallback"],
        false
    );
    assert_eq!(
        refreshed["graph_context"]["fallback_when_unavailable"],
        "scout_source_evidence"
    );
    assert_eq!(refreshed["graph_context"]["source_of_truth"], false);
    assert_eq!(
        refreshed["graph_context"]["edges"]["require_source_check_for_mutation"],
        true
    );
    assert_eq!(refreshed["agents"]["explorer"]["variant"], "high");
    assert_eq!(refreshed["agents"]["explorer"]["fast_mode"], true);
    assert_eq!(refreshed["agents"]["documenter"]["variant"], "medium");
    assert_eq!(refreshed["agents"]["documenter"]["fast_mode"], true);
    assert_eq!(refreshed["agents"]["sentinel"]["model"], "gpt-5.4-mini");
    assert_eq!(refreshed["agents"]["sentinel"]["variant"], "high");
    assert_eq!(refreshed["agents"]["sentinel"]["fast_mode"], true);
    assert_eq!(refreshed["agents"]["way"]["variant"], "high");
    assert_eq!(refreshed["agents"]["verifier"]["variant"], "high");
    assert_eq!(
        refreshed["companion_agents"]["companion_reader"]["variant"],
        "medium"
    );
    assert_eq!(
        refreshed["companion_agents"]["companion_reader"]["fast_mode"],
        true
    );
    assert_eq!(
        refreshed["companion_agents"]["companion_operator"]["variant"],
        "medium"
    );
    assert_eq!(
        refreshed["companion_agents"]["companion_operator"]["fast_mode"],
        true
    );
    assert_eq!(
        refreshed["runtime"]["fallback_specialist_execution_mode"],
        "codex_exec"
    );
    assert_eq!(refreshed["agents"]["explorer"]["callsign"], "Observer");
    assert_eq!(
        refreshed["agents"]["code specialist"]["recommended_workflows"],
        json!([
            "remove-deadcode",
            "ai-slop-remover",
            "lsp-safe-refactor",
            "rust-analyzer-lsp"
        ])
    );
    assert!(state.backup_path.expect("backup path").exists());

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn ensure_ccc_config_file_upgrades_legacy_planner_alias_to_high() {
    let config_home = create_temp_path("config-planner-drift");
    let config_dir = config_home.join("ccc");
    create_dir_all(&config_dir).expect("create config dir");
    let config_path = config_dir.join("ccc-config.toml");
    write(
        &config_path,
        r#"
version = 1

[generated_defaults]
version = 9
policy = "ccc-managed-defaults"

[agents.planner]
name = "tactician"
summary = "Way creation and bounded planning when the next move is still unclear."
model = "gpt-5.5"
variant = "medium"
fast_mode = true
config_entries = []
"#,
    )
    .expect("write planner alias config");

    let legacy_dir = config_home.join("ccc");
    let (_, created, state) = ensure_ccc_config_file_at_with_state(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("upgrade planner alias defaults");

    assert!(!created);
    assert_eq!(state.action_status, "backfilled");
    let refreshed = read_optional_toml_document(&config_path)
        .expect("read refreshed config")
        .expect("refreshed config");
    assert_eq!(refreshed["generated_defaults"]["version"], 16);
    assert_eq!(refreshed["agents"]["planner"]["variant"], "high");

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn ensure_ccc_config_file_backfill_preserves_existing_user_values() {
    let config_home = create_temp_path("config-backfill-preserve");
    let config_dir = config_home.join("ccc");
    let legacy_dir = config_home.join("ccc");
    create_dir_all(&config_dir).expect("create config dir");
    let config_path = config_dir.join("ccc-config.toml");
    write(
        &config_path,
        r#"
version = 1

[companion_agents.companion_operator]
name = "companion_operator"
summary = "Custom operator summary"
model = "gpt-5.4"
variant = "high"
fast_mode = true
config_entries = []

[tool_routing]
default_model = "custom-mini"

[tool_routing.tools.git]
allowed_operations = ["read", "mutation"]
owner_companion_agent = "custom_reader"
mutation_owner_companion_agent = "custom_operator"
"#,
    )
    .expect("write config");

    ensure_ccc_config_file_at(
        &config_path,
        &legacy_dir.join("ccc-config.toml"),
        &legacy_dir.join("ccc-config.json"),
    )
    .expect("backfill config");

    let refreshed = read_optional_toml_document(&config_path)
        .expect("read refreshed config")
        .expect("refreshed config");
    assert_eq!(refreshed["tool_routing"]["default_model"], "custom-mini");
    assert_eq!(
        refreshed["tool_routing"]["tools"]["git"]["owner_companion_agent"],
        "custom_reader"
    );
    assert_eq!(
        refreshed["tool_routing"]["tools"]["git"]["mutation_owner_companion_agent"],
        "custom_operator"
    );
    assert_eq!(
        refreshed["companion_agents"]["companion_operator"]["summary"],
        "Custom operator summary"
    );
    assert_eq!(
        refreshed["companion_agents"]["companion_reader"]["model"],
        "gpt-5.4-mini"
    );
    assert_eq!(
        refreshed["tool_routing"]["tools"]["gh"]["owner_companion_agent"],
        "companion_reader"
    );

    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn sync_generated_custom_agents_writes_managed_files_and_preserves_unrelated() {
    let config_home = create_temp_path("custom-agent-sync-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_toml_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_json_path = config_home.join("ccc").join("ccc-config.json");
    ensure_ccc_config_file_at(&config_path, &legacy_toml_path, &legacy_json_path)
        .expect("create config");
    let config_value = read_optional_toml_document(&config_path)
        .expect("read config")
        .expect("config value");

    let codex_home = create_temp_path("custom-agent-sync-home");
    let install_dir = codex_home.join("agents");
    create_dir_all(&install_dir).expect("create agents dir");
    write(
        install_dir.join("user-agent.toml"),
        "name = \"user_agent\"\ndescription = \"user managed\"\ndeveloper_instructions = \"stay\"",
    )
    .expect("write unrelated agent");
    write(
        install_dir.join("ccc-captain.toml"),
        "name = \"ccc_captain\"\ndescription = \"stale captain agent\"",
    )
    .expect("write stale captain agent");

    let payload = sync_generated_custom_agents_in_directory(&config_value, &install_dir)
        .expect("sync custom agents");
    assert_eq!(payload["status"], "matching_sync");
    assert_eq!(payload["file_count"], 8);
    assert!(!payload["generated_names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "ccc_captain"));
    assert!(install_dir.join("user-agent.toml").exists());
    assert!(!install_dir.join("ccc-captain.toml").exists());

    let scout_contents =
        fs::read_to_string(install_dir.join("ccc-scout.toml")).expect("read scout agent");
    assert!(scout_contents.contains("name = \"ccc_scout\""));
    assert!(scout_contents.contains("model = \"gpt-5.4-mini\""));
    assert!(scout_contents.contains("model_reasoning_effort = \"high\""));
    assert!(scout_contents.contains("service_tier = \"fast\""));
    assert!(scout_contents.contains("sandbox_mode = \"read-only\""));

    let tactician_contents =
        fs::read_to_string(install_dir.join("ccc-tactician.toml")).expect("read tactician agent");
    assert!(tactician_contents.contains("name = \"ccc_tactician\""));
    assert!(tactician_contents.contains("model = \"gpt-5.5\""));
    assert!(tactician_contents.contains("model_reasoning_effort = \"high\""));

    let scribe_contents =
        fs::read_to_string(install_dir.join("ccc-scribe.toml")).expect("read scribe agent");
    assert!(scribe_contents.contains("name = \"ccc_scribe\""));
    assert!(scribe_contents.contains("model_reasoning_effort = \"medium\""));

    let sentinel_contents =
        fs::read_to_string(install_dir.join("ccc-sentinel.toml")).expect("read sentinel agent");
    assert!(sentinel_contents.contains("name = \"ccc_sentinel\""));
    assert!(sentinel_contents.contains("model = \"gpt-5.4-mini\""));
    assert!(sentinel_contents.contains("model_reasoning_effort = \"high\""));
    assert!(sentinel_contents.contains("service_tier = \"fast\""));
    assert!(sentinel_contents.contains("sandbox_mode = \"read-only\""));

    let companion_reader_contents =
        fs::read_to_string(install_dir.join("ccc-companion_reader.toml"))
            .expect("read companion reader agent");
    assert!(companion_reader_contents.contains("name = \"ccc_companion_reader\""));
    assert!(companion_reader_contents.contains("model_reasoning_effort = \"medium\""));

    let companion_operator_contents =
        fs::read_to_string(install_dir.join("ccc-companion_operator.toml"))
            .expect("read companion operator agent");
    assert!(companion_operator_contents.contains("name = \"ccc_companion_operator\""));
    assert!(companion_operator_contents.contains("model_reasoning_effort = \"medium\""));

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&codex_home);
}

#[test]
fn sync_generated_custom_agents_uses_explicit_sentinel_defaults_when_config_missing_role() {
    let codex_home = create_temp_path("custom-agent-sync-sentinel-default");
    let install_dir = codex_home.join("agents");
    create_dir_all(&install_dir).expect("create agents dir");

    sync_generated_custom_agents_in_directory(&json!({ "agents": {} }), &install_dir)
        .expect("sync custom agents");

    let sentinel_contents =
        fs::read_to_string(install_dir.join("ccc-sentinel.toml")).expect("read sentinel agent");
    assert!(sentinel_contents.contains("name = \"ccc_sentinel\""));
    assert!(sentinel_contents.contains("model = \"gpt-5.4-mini\""));
    assert!(sentinel_contents.contains("model_reasoning_effort = \"high\""));
    assert!(sentinel_contents.contains("service_tier = \"fast\""));

    let _ = fs::remove_dir_all(&codex_home);
}

#[test]
fn inspect_generated_custom_agents_reports_mismatch_when_file_drifted() {
    let config_home = create_temp_path("custom-agent-inspect-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_toml_path = config_home.join("ccc").join("ccc-config.toml");
    let legacy_json_path = config_home.join("ccc").join("ccc-config.json");
    ensure_ccc_config_file_at(&config_path, &legacy_toml_path, &legacy_json_path)
        .expect("create config");
    let config_value = read_optional_toml_document(&config_path)
        .expect("read config")
        .expect("config value");

    let codex_home = create_temp_path("custom-agent-inspect-home");
    let install_dir = codex_home.join("agents");
    sync_generated_custom_agents_in_directory(&config_value, &install_dir)
        .expect("sync custom agents");
    write(
        install_dir.join("ccc-raider.toml"),
        "name = \"ccc_raider\"\ndescription = \"drifted\"\ndeveloper_instructions = \"broken\"",
    )
    .expect("drift generated agent");

    let payload = inspect_generated_custom_agents_in_directory(&config_value, &install_dir)
        .expect("inspect custom agents");
    assert_eq!(payload["status"], "mismatched_sync");
    assert!(payload["mismatched_files"]
        .as_array()
        .expect("mismatched files")
        .iter()
        .any(|value| value == "ccc-raider.toml"));

    let _ = fs::remove_dir_all(&config_home);
    let _ = fs::remove_dir_all(&codex_home);
}

#[test]
fn tools_list_includes_ccc_status() {
    let session_context = create_session_context();
    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/list",
            "params": {}
        }),
    )
    .expect("response");

    let tools = response["result"]["tools"].as_array().expect("tools array");
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ccc_recommend_entry"));
    assert!(tools.iter().any(|tool| tool["name"] == "ccc_auto_entry"));
    assert!(tools.iter().any(|tool| tool["name"] == "ccc_status"));
    assert!(tools.iter().any(|tool| tool["name"] == "ccc_activity"));
    let render_tool = tools
        .iter()
        .find(|tool| tool["name"] == "ccc_render_app_panel")
        .expect("ccc_render_app_panel tool");
    assert_eq!(
        render_tool["_meta"]["ui.resourceUri"],
        "ui://ccc/app-panel.html"
    );
    assert_eq!(
        render_tool["_meta"]["openai/outputTemplate"],
        "ui://ccc/app-panel.html"
    );
    assert!(tools.iter().any(|tool| tool["name"] == "ccc_start"));
    assert!(tools.iter().any(|tool| tool["name"] == "ccc_run"));
    assert!(tools.iter().any(|tool| tool["name"] == "ccc_orchestrate"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ccc_subagent_update"));
}

#[test]
fn resources_list_and_read_expose_ccc_app_panel_template() {
    let session_context = create_session_context();
    let list_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "resources/list",
            "params": {}
        }),
    )
    .expect("list response");

    let resources = list_response["result"]["resources"]
        .as_array()
        .expect("resources array");
    assert!(resources.iter().any(|resource| {
        resource["uri"] == "ui://ccc/app-panel.html"
            && resource["mimeType"] == "text/html;profile=mcp-app"
            && resource["_meta"]["ui"]["prefersBorder"] == true
    }));

    let read_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "resources/read",
            "params": {
                "uri": "ui://ccc/app-panel.html"
            }
        }),
    )
    .expect("read response");
    assert_eq!(
        read_response["result"]["contents"][0]["mimeType"],
        "text/html;profile=mcp-app"
    );
    assert_eq!(
        read_response["result"]["contents"][0]["_meta"]["ui"]["prefersBorder"],
        true
    );
    let html = read_response["result"]["contents"][0]["text"]
        .as_str()
        .expect("html");
    assert!(html.contains("CCC LongWay Panel"));
    assert!(html.contains("panel.innerHTML"));
    assert!(html.contains("const html ="));
    assert!(html.contains("replace(/[&<>\"']/g"));
    assert!(
        !html.contains("${text("),
        "app-panel data must be escaped before innerHTML interpolation"
    );
}

#[test]
fn ccc_render_app_panel_tool_call_returns_structured_app_panel_payload() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("render-app-panel-tool-call");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Recover a failed configured CCC specialist",
        "title": "Render recovery app panel",
        "intent": "Smoke the MCP Apps app-panel structuredContent payload",
        "scope": "One failed ccc_raider fan-in",
        "acceptance": "ccc_render_app_panel returns app-panel structured content and MCP Apps meta.",
        "prompt": "Record a failed ccc_raider update and render the app panel.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-render-app-panel",
        "status": "failed",
        "summary": "Configured ccc_raider failed before clean fan-in.",
        "fan_in_status": "failed",
        "evidence_paths": [],
        "next_action": "retry",
        "open_questions": ["Retry ccc_raider or reassign."],
        "confidence": "medium"
    }))
    .expect("failed subagent update");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "tools/call",
            "params": {
                "name": "ccc_render_app_panel",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("render response");

    let structured = &response["result"]["structuredContent"];
    let app_panel = &structured["app_panel"];
    assert_eq!(app_panel["schema"], "ccc.codex_app_panel.v1");
    assert_eq!(app_panel["state_contract"]["active_gate"], "recovery");
    assert_eq!(app_panel["recovery_lane"]["status"], "recovery_pending");
    assert_eq!(
        app_panel["specialist_lanes"]["subagent_activity"][0]["child_agent_id"],
        "ccc_raider"
    );
    assert_eq!(
        response["result"]["_meta"]["ui.resourceUri"],
        "ui://ccc/app-panel.html"
    );
    assert_eq!(
        response["result"]["_meta"]["openai/outputTemplate"],
        "ui://ccc/app-panel.html"
    );
    assert_eq!(
        response["result"]["content"][0]["text"],
        create_codex_app_panel_text(app_panel)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn tools_list_subagent_update_review_outcome_accepts_unsatisfactory() {
    let session_context = create_session_context();
    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/list",
            "params": {}
        }),
    )
    .expect("response");

    let tools = response["result"]["tools"].as_array().expect("tools array");
    let subagent_update = tools
        .iter()
        .find(|tool| tool["name"] == "ccc_subagent_update")
        .expect("ccc_subagent_update tool");
    let review_outcome_enum = subagent_update["inputSchema"]["properties"]["review_outcome"]
        ["enum"]
        .as_array()
        .expect("review_outcome enum");
    assert!(review_outcome_enum
        .iter()
        .any(|value| value == "unsatisfactory"));
}

#[test]
fn ccc_public_entrypoints_accept_planned_rows_arguments_and_schemas() {
    let planned_rows = json!([
        {
            "title": "Prepare follow-up row",
            "planned_role": "implementation_specialist",
            "planned_agent_id": "raider-b",
            "scope": "planned row only",
            "acceptance": "row is persisted as planned",
            "status": "pending"
        },
        "Compact follow-up row"
    ]);
    let target_paths = json!(["docs/release-work/0.0.12/PRE_RELEASE_PLAN.md"]);
    let input_items = json!([
        {
            "type": "local_file",
            "path": "docs/release-work/0.0.12/PRE_RELEASE_PLAN.md"
        }
    ]);
    let start_arguments = crate::entry_arguments::parse_ccc_start_arguments(&json!({
        "goal": "Bootstrap planned rows",
        "title": "Initial row",
        "intent": "Accept planned rows through ccc_start",
        "scope": "Argument parsing only",
        "acceptance": "planned_rows survives parse",
        "prompt": "Create the initial row",
        "task_kind": "execution",
        "planned_rows": planned_rows,
        "target_paths": target_paths,
        "input_items": input_items
    }))
    .expect("parse ccc_start arguments");
    assert_eq!(start_arguments["sequence"], "PLAN_SEQUENCE");
    assert_eq!(start_arguments["planned_rows"], planned_rows);
    assert_eq!(
        start_arguments["structured_target_mentions"]["target_paths"],
        target_paths
    );
    assert_eq!(
        start_arguments["structured_target_mentions"]["input_items"],
        input_items
    );

    let run_arguments = crate::entry_arguments::parse_ccc_run_arguments(&json!({
        "goal": "Run planned rows",
        "title": "Initial run row",
        "intent": "Accept planned rows through ccc_run",
        "scope": "Argument parsing only",
        "acceptance": "planned_rows survives parse",
        "prompt": "Create and run the initial row",
        "task_kind": "execution",
        "planned_rows": planned_rows,
        "artifact_paths": target_paths,
        "workflow_variant_selection": {}
    }))
    .expect("parse ccc_run arguments");
    assert_eq!(run_arguments["sequence"], "PLAN_SEQUENCE");
    assert_eq!(run_arguments["planned_rows"], planned_rows);
    assert_eq!(
        run_arguments["structured_target_mentions"]["artifact_paths"],
        target_paths
    );

    let tools = crate::mcp_tools::create_tools_list_response(Some(json!(13)));
    let tool_list = tools["result"]["tools"].as_array().expect("tools");
    for tool_name in ["ccc_start", "ccc_run"] {
        let tool = tool_list
            .iter()
            .find(|tool| tool["name"] == tool_name)
            .unwrap_or_else(|| panic!("missing {tool_name}"));
        assert_eq!(
            tool["inputSchema"]["properties"]["planned_rows"]["type"],
            "array"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["planned_rows"]["items"]["oneOf"][0]["type"],
            "string"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["target_paths"]["items"]["oneOf"][0]["type"],
            "string"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["input_items"]["items"]["oneOf"][1]["type"],
            "object"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["no_longway"]["type"],
            "boolean"
        );
    }

    let execute_arguments = crate::entry_arguments::parse_ccc_start_arguments(&json!({
        "goal": "Skip LongWay",
        "title": "Direct execution",
        "intent": "Operator explicitly disabled LongWay planning",
        "scope": "Argument parsing only",
        "acceptance": "no_longway selects EXECUTE_SEQUENCE",
        "prompt": "Do not make a LongWay",
        "task_kind": "execution",
        "no_longway": true
    }))
    .expect("parse no_longway ccc_start arguments");
    assert_eq!(execute_arguments["sequence"], "EXECUTE_SEQUENCE");
}

#[test]
fn ccc_subagent_update_schema_lists_fallback_reason_codes() {
    let tools = crate::mcp_tools::create_tools_list_response(Some(json!(14)));
    let tool_list = tools["result"]["tools"].as_array().expect("tools");
    let tool = tool_list
        .iter()
        .find(|tool| tool["name"] == "ccc_subagent_update")
        .expect("ccc_subagent_update tool");

    assert_eq!(
        tool["inputSchema"]["properties"]["fallback_reason"]["enum"],
        json!(crate::specialist_roles::SUBAGENT_FALLBACK_REASON_CODES)
    );
}

#[test]
fn ccc_recommend_entry_returns_bounded_plan_guidance() {
    let session_context = create_session_context();
    let response = handle_message(
            &session_context,
            json!({
                "jsonrpc": "2.0",
                "id": 111,
                "method": "tools/call",
                "params": {
                    "name": "ccc_recommend_entry",
                    "arguments": {
                        "request": "Investigate the current repository state and plan the next bounded step"
                    }
                }
            }),
        )
        .expect("response");

    let recommendation = &response["result"]["structuredContent"]["recommendation"];
    assert_eq!(recommendation["recommended_entrypoint"], "way");
    assert_eq!(recommendation["request_shape"], "way");
    assert!(recommendation["automatic_entry_supported"].is_boolean());
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("CCC entry recommendation"));
}

#[test]
fn ccc_recommend_entry_repeats_interview_prompt_for_broad_requests_without_state() {
    let session_context = create_session_context();
    let request = "Plan a repository cleanup strategy across modules.";

    for id in [112, 113] {
        let response = handle_message(
            &session_context,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {
                    "name": "ccc_recommend_entry",
                    "arguments": {
                        "request": request
                    }
                }
            }),
        )
        .expect("response");
        let recommendation = &response["result"]["structuredContent"]["recommendation"];

        assert_eq!(recommendation["request_shape"], "way");
        assert_eq!(recommendation["operator_confirmation_required"], true);
        assert_eq!(recommendation["intent_confirmation"]["state"], "required");
        assert!(recommendation["confirmation_prompt"].as_str().is_some());
        assert!(recommendation["intent_confirmation"]["reason_codes"]
            .as_array()
            .expect("reason codes")
            .iter()
            .any(|value| value == "broad_way_request"));
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("Confirm this interpretation before CCC creates a Way/run"));
    }
}

#[test]
fn ccc_recommend_entry_keeps_narrow_read_only_requests_no_gate() {
    let session_context = create_session_context();
    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 114,
            "method": "tools/call",
            "params": {
                "name": "ccc_recommend_entry",
                "arguments": {
                    "request": "List the current workspace files."
                }
            }
        }),
    )
    .expect("response");
    let recommendation = &response["result"]["structuredContent"]["recommendation"];

    assert_eq!(recommendation["direct_allowed"], true);
    assert_eq!(recommendation["operator_confirmation_required"], false);
    assert!(recommendation["confirmation_prompt"].is_null());
    assert!(!response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("Confirm this interpretation before CCC creates a Way/run"));
}

#[test]
fn ccc_recommend_entry_exposes_no_mutation_guard_fields() {
    for (
        request,
        expected_action,
        direct_allowed,
        requires_user_confirmation,
        expected_risk,
        expected_next_action,
    ) in [
        (
            "List the current workspace files.",
            "direct_read_only",
            true,
            false,
            Some("low"),
            "proceed",
        ),
        (
            "Investigate the current repository state and plan the next bounded step",
            "enter_ccc_control_plane",
            false,
            true,
            None,
            "await_operator",
        ),
        (
            "Review the implementation for regressions.",
            "enter_ccc_control_plane",
            false,
            true,
            None,
            "await_operator",
        ),
    ] {
        let recommendation = create_ccc_recommend_entry_payload_for_policy(
            &json!({
                "request": request
            }),
            Some("codex_cli_ccc_first"),
        );

        assert_eq!(
            recommendation["recommended_action"], expected_action,
            "request: {request}"
        );
        assert_eq!(
            recommendation["direct_allowed"], direct_allowed,
            "request: {request}"
        );
        assert_eq!(
            recommendation["requires_user_confirmation"], requires_user_confirmation,
            "request: {request}"
        );
        assert_eq!(
            recommendation["next_action"], expected_next_action,
            "request: {request}"
        );
        assert_eq!(
            recommendation["operator_confirmation_required"],
            expected_next_action == "await_operator",
            "request: {request}"
        );
        assert!(
            recommendation["active_run_summary"].is_string(),
            "request: {request}"
        );
        assert!(recommendation["reason"].is_string(), "request: {request}");
        if let Some(expected_risk) = expected_risk {
            assert_eq!(recommendation["risk"], expected_risk, "request: {request}");
        } else {
            assert!(recommendation["risk"].is_string(), "request: {request}");
            assert!(
                !recommendation["risk"]
                    .as_str()
                    .unwrap_or_default()
                    .is_empty(),
                "request: {request}"
            );
        }
    }
}

#[test]
fn ccc_recommend_entry_requires_confirmation_for_release_install_mutation() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Fix install.sh and scripts/release/build-release-asset.sh for the release install mutation."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(recommendation["request_shape"], "mutation");
    assert_eq!(recommendation["next_action"], "await_operator");
    assert_eq!(recommendation["operator_confirmation_required"], true);
    assert_eq!(recommendation["intent_confirmation"]["state"], "required");
    assert_eq!(
        recommendation["intent_confirmation"]["clarification_policy"]["question_count"],
        "1-3"
    );
    assert_eq!(
        recommendation["intent_confirmation"]["clarification_policy"]["narrow_work_default"],
        "proceed_with_explicit_assumptions"
    );
    assert_eq!(
        recommendation["intent_confirmation"]["interpretation"]["recommended_entrypoint"],
        "way"
    );
    assert!(recommendation["intent_confirmation"]["reason_codes"]
        .as_array()
        .expect("reason codes")
        .iter()
        .any(|value| value == "release_install_mutation"));
}

#[test]
fn ccc_recommend_entry_treats_foreman_first_as_ccc_first_alias() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "List the current workspace files."
        }),
        Some("codex_cli_foreman_first"),
    );

    assert_eq!(recommendation["policy_mode"], "codex_cli_ccc_first");
    assert_eq!(recommendation["automatic_entry_supported"], true);
    assert_eq!(
        recommendation["entry_boundary"],
        "session_instruction_plus_wrapper"
    );
}

#[test]
fn ccc_auto_entry_awaits_operator_for_confirmation_gated_requests() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("auto-entry-confirmation-gate");
    create_dir_all(&workspace_dir).expect("create workspace");
    let parsed = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "request": "Fix install.sh and scripts/release/build-release-asset.sh for the release install mutation."
    });

    let payload = create_ccc_auto_entry_payload_for_policy(
        &session_context,
        &parsed,
        Some("codex_cli_ccc_first"),
    )
    .expect("auto-entry payload");

    assert_eq!(payload["created"], false);
    assert_eq!(payload["run_selection"], "operator_confirmation_required");
    assert_eq!(payload["next_action"], "await_operator");
    assert_eq!(payload["operator_confirmation_required"], true);
    assert_eq!(
        payload["answer_trace"]["execution_path"],
        "await_operator_confirmation"
    );
    assert_eq!(
        payload["recommendation"]["intent_confirmation"]["state"],
        "required"
    );
    assert!(!workspace_dir.join(".ccc").join("runs").exists());

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn review_policy_skips_low_risk_no_review() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "List the current workspace files."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(recommendation["review_policy"]["decision"], "skip");
    assert_eq!(recommendation["review_policy"]["state"], "skipped");
    assert_eq!(recommendation["review_policy"]["risk"], "low");
    assert_eq!(recommendation["review_policy"]["recommended_reviewers"], 0);
    assert_eq!(
        recommendation["review_policy"]["risk_evidence_source"],
        "skill_registry"
    );
    assert_eq!(
        recommendation["review_policy"]["logical_risk_precheck"]["classifier_agent"],
        "ccc_sentinel"
    );
    assert_eq!(
        recommendation["review_policy"]["logical_risk_precheck"]["agents"][0]["logical"]["actions"]
            [0]["action"],
        "classify_risk"
    );
}

#[test]
fn review_policy_skips_visibility_diagnostic_smoke() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Check CCC status, LongWay/status, and Codex app visibility smoke output."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(recommendation["request_shape"], "diagnostic");
    assert_eq!(recommendation["recommended_entrypoint"], "start");
    assert_eq!(recommendation["recommended_task_kind"], "explore");
    assert_eq!(recommendation["direct_allowed"], true);
    assert_eq!(recommendation["review_policy"]["decision"], "skip");
    assert_eq!(recommendation["review_policy"]["state"], "skipped");
    assert_eq!(recommendation["review_policy"]["risk"], "low");
}

#[test]
fn review_policy_recommends_single_review_for_moderate_risk_mutation() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Implement a bounded single file update for the output text."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(
        recommendation["review_policy"]["decision"],
        "recommend_single"
    );
    assert_eq!(recommendation["review_policy"]["state"], "recommended");
    assert_eq!(recommendation["review_policy"]["risk"], "moderate");
    assert_eq!(recommendation["review_policy"]["recommended_reviewers"], 1);
    assert_eq!(recommendation["review_policy"]["reviewer_cap"], 1);
}

#[test]
fn review_policy_requires_review_for_high_risk_work() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Implement an auth permission migration across multiple files with regression risk."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(recommendation["review_policy"]["decision"], "require");
    assert_eq!(recommendation["review_policy"]["state"], "required");
    assert_eq!(recommendation["review_policy"]["risk"], "high");
    assert_eq!(recommendation["review_policy"]["required"], true);
    assert_eq!(recommendation["review_policy"]["recommended_reviewers"], 1);
    assert_eq!(
        recommendation["review_policy"]["logical_risk_precheck"]["selected_review_owner"],
        "ccc_arbiter"
    );
    assert_eq!(
        recommendation["review_policy"]["logical_risk_precheck"]["agents"][1]["logical"]["actions"]
            [0]["action"],
        "review_diff"
    );
}

#[test]
fn review_policy_requires_review_for_failed_validation_signal() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Tests failed validation after the implementation; continue with bounded verification."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(recommendation["review_policy"]["decision"], "require");
    assert_eq!(recommendation["review_policy"]["state"], "required");
    assert_eq!(
        recommendation["review_policy"]["reason_code"],
        "failed_validation_or_unresolved_acceptance"
    );
    assert_eq!(recommendation["review_policy"]["recommended_reviewers"], 1);
}

#[test]
fn review_policy_suppresses_review_when_resource_limit_is_explicit() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Implement a bounded update with reviewer cap resource limit and no reviewer budget."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(
        recommendation["review_policy"]["decision"],
        "suppress_for_resource_limit"
    );
    assert_eq!(recommendation["review_policy"]["state"], "suppressed");
    assert_eq!(
        recommendation["review_policy"]["reason_code"],
        "reviewer_resource_limit"
    );
    assert_eq!(recommendation["review_policy"]["recommended_reviewers"], 0);
    assert_eq!(recommendation["review_policy"]["reviewer_cap"], 0);
    assert!(recommendation["review_policy"]["resource_pressure"].is_null());
}

#[test]
fn review_policy_suppresses_review_when_file_handle_pressure_is_explicit() {
    for request in [
        "Implement the v0.0.4 slice; operator saw Too many open files during tests.",
        "Repair the runtime path after os error 24 in the runner.",
        "Keep the task bounded because file descriptor pressure is already present.",
        "High-risk auth migration is requested, but EMFILE file-handle pressure blocks reviewers.",
    ] {
        let recommendation = create_ccc_recommend_entry_payload_for_policy(
            &json!({
                "request": request
            }),
            Some("codex_cli_ccc_first"),
        );

        assert_eq!(
            recommendation["review_policy"]["decision"], "suppress_for_resource_limit",
            "request: {request}"
        );
        assert_eq!(
            recommendation["review_policy"]["reason_code"], "reviewer_resource_limit",
            "request: {request}"
        );
        assert_eq!(
            recommendation["review_policy"]["recommended_reviewers"], 0,
            "request: {request}"
        );
        assert_eq!(
            recommendation["review_policy"]["reviewer_cap"], 0,
            "request: {request}"
        );
        assert!(
            recommendation["review_policy"]["resource_pressure"].is_null(),
            "request: {request}"
        );
    }
}

#[test]
fn review_policy_suppresses_review_when_runtime_pressure_is_high() {
    let runtime_pressure = RuntimeReviewPressureSnapshot {
        source: "unit_test_worker_visibility".to_string(),
        stale_worker_count: 1,
        timed_out_worker_count: 0,
        reclaim_needed_worker_count: 1,
        active_run_count: 0,
        token_total: 0,
        token_soft_limit: None,
        cpu_available_parallelism: None,
        memory_total_kib: None,
        memory_available_kib: None,
        memory_available_percent: None,
        pressure_reason: None,
    };
    let policy = create_review_policy_payload(
        "Implement a bounded single file update for the output text.",
        "mutation",
        "single_scoped_task",
        Some("2026-04-22T08:02:00.000Z"),
        Some(&runtime_pressure),
    );

    assert_eq!(policy["decision"], "suppress_for_runtime_pressure");
    assert_eq!(policy["state"], "suppressed");
    assert_eq!(policy["risk"], "moderate");
    assert_eq!(policy["required"], false);
    assert_eq!(policy["recommended_reviewers"], 0);
    assert_eq!(policy["reviewer_cap"], 0);
    assert_eq!(policy["reason_code"], "runtime_review_pressure");
    assert_eq!(
        policy["resource_pressure"]["source"],
        "unit_test_worker_visibility"
    );
    assert_eq!(policy["resource_pressure"]["high_pressure"], true);
    assert_eq!(policy["resource_pressure"]["stale_worker_count"], 1);
}

#[test]
fn review_policy_suppresses_review_when_os_memory_pressure_is_high() {
    let runtime_pressure = runtime_review_pressure_snapshot_from_value(
        Some(&json!({
            "memory_total_kib": 1_000_000,
            "memory_available_kib": 50_000,
            "memory_available_percent": 5,
            "cpu_available_parallelism": 4
        })),
        "unit_test_os_snapshot",
    )
    .expect("runtime pressure");
    let policy = create_review_policy_payload(
        "Implement an auth permission migration across multiple files with regression risk.",
        "mutation",
        "multi_step_or_unclear",
        Some("2026-04-22T08:02:00.000Z"),
        Some(&runtime_pressure),
    );

    assert_eq!(policy["decision"], "suppress_for_runtime_pressure");
    assert_eq!(policy["reason_code"], "runtime_review_pressure");
    assert_eq!(
        policy["resource_pressure"]["source"],
        "unit_test_os_snapshot"
    );
    assert_eq!(policy["resource_pressure"]["memory_available_percent"], 5);
    assert_eq!(policy["resource_pressure"]["high_pressure"], true);
}

#[test]
fn captain_review_task_helper_creates_one_for_required_and_recommended_policy() {
    for (label, request, request_shape, expected_decision) in [
        (
            "required",
            "Implement an auth permission migration with regression risk.",
            "mutation",
            "require",
        ),
        (
            "recommended",
            "Implement a bounded single file update for the output text.",
            "mutation",
            "recommend_single",
        ),
    ] {
        let workspace_dir = create_temp_path(&format!("review-task-{label}"));
        create_dir_all(&workspace_dir).expect("create workspace");
        let run_id = format!("run-review-task-{label}");
        let run_directory = write_test_run_fixture(&workspace_dir, &run_id);
        let task_card_file = run_directory.join("task-cards").join("task-1.json");
        let mut source_task = read_json_document(&task_card_file).expect("source task");
        source_task["review_policy"] = create_review_policy_payload(
            request,
            request_shape,
            "single_scoped_task",
            Some("2026-04-22T08:02:00.000Z"),
            None,
        );
        write_json_document(&task_card_file, &source_task).expect("write source task");

        let review_task = maybe_create_captain_owned_review_task_card(
            &run_directory,
            &source_task,
            "2026-04-22T08:03:00.000Z",
        )
        .expect("create review task")
        .expect("review task created");
        assert_eq!(source_task["review_policy"]["decision"], expected_decision);
        assert_eq!(review_task["owner_role"], "orchestrator");
        assert_eq!(review_task["assigned_role"], "verifier");
        assert_eq!(review_task["assigned_agent_id"], "arbiter");
        assert_eq!(review_task["task_kind"], "review");
        assert_eq!(review_task["review_of_task_card_ids"], json!(["task-1"]));
        assert_eq!(
            review_task["orchestrator_review_gate"],
            "after_child_completion"
        );
        assert_eq!(review_task["verification_state"], "pending");
        assert_eq!(review_task["review_pass_count"], 0);
        assert_eq!(review_task["review_fan_in"], Value::Null);

        let review_task_cards = review_task_cards_for_source(&run_directory, "task-1");
        assert_eq!(review_task_cards.len(), 1);
        let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
        assert_eq!(run_record["active_task_card_id"], "task-1");
        assert_eq!(
            run_record["task_card_ids"].as_array().map(Vec::len),
            Some(2)
        );

        let _ = fs::remove_dir_all(&workspace_dir);
    }
}

#[test]
fn captain_review_task_helper_skips_skipped_and_suppressed_policy() {
    for (label, request, request_shape) in [
        ("skipped", "List the current workspace files.", "lookup"),
        (
            "suppressed",
            "Implement a bounded update with reviewer cap resource limit and no reviewer budget.",
            "mutation",
        ),
    ] {
        let workspace_dir = create_temp_path(&format!("review-task-{label}"));
        create_dir_all(&workspace_dir).expect("create workspace");
        let run_id = format!("run-review-task-{label}");
        let run_directory = write_test_run_fixture(&workspace_dir, &run_id);
        let task_card_file = run_directory.join("task-cards").join("task-1.json");
        let mut source_task = read_json_document(&task_card_file).expect("source task");
        source_task["review_policy"] = create_review_policy_payload(
            request,
            request_shape,
            "single_scoped_task",
            Some("2026-04-22T08:02:00.000Z"),
            None,
        );
        write_json_document(&task_card_file, &source_task).expect("write source task");

        let review_task = maybe_create_captain_owned_review_task_card(
            &run_directory,
            &source_task,
            "2026-04-22T08:03:00.000Z",
        )
        .expect("skip review task");
        assert_eq!(review_task, None);
        assert!(review_task_cards_for_source(&run_directory, "task-1").is_empty());
        let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
        assert_eq!(
            run_record["task_card_ids"].as_array().map(Vec::len),
            Some(1)
        );

        let _ = fs::remove_dir_all(&workspace_dir);
    }
}

#[test]
fn captain_review_task_helper_skips_runtime_pressure_suppressed_policy() {
    let workspace_dir = create_temp_path("review-task-runtime-pressure");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-runtime-pressure");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut source_task = read_json_document(&task_card_file).expect("source task");
    let runtime_pressure = RuntimeReviewPressureSnapshot {
        source: "unit_test_active_run_scan".to_string(),
        stale_worker_count: 0,
        timed_out_worker_count: 0,
        reclaim_needed_worker_count: 0,
        active_run_count: 1,
        token_total: 0,
        token_soft_limit: None,
        cpu_available_parallelism: None,
        memory_total_kib: None,
        memory_available_kib: None,
        memory_available_percent: None,
        pressure_reason: None,
    };
    source_task["review_policy"] = create_review_policy_payload(
        "Implement a bounded single file update for the output text.",
        "mutation",
        "single_scoped_task",
        Some("2026-04-22T08:02:00.000Z"),
        Some(&runtime_pressure),
    );
    write_json_document(&task_card_file, &source_task).expect("write source task");

    let review_task = maybe_create_captain_owned_review_task_card(
        &run_directory,
        &source_task,
        "2026-04-22T08:03:00.000Z",
    )
    .expect("skip runtime-pressure review task");
    assert_eq!(source_task["review_policy"]["state"], "suppressed");
    assert_eq!(review_task, None);
    assert!(review_task_cards_for_source(&run_directory, "task-1").is_empty());
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(1)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn captain_review_task_helper_avoids_duplicate_review_task() {
    let workspace_dir = create_temp_path("review-task-duplicate");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-task-duplicate");
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut source_task = read_json_document(&task_card_file).expect("source task");
    source_task["review_policy"] = create_review_policy_payload(
        "Implement an auth permission migration with regression risk.",
        "mutation",
        "single_scoped_task",
        Some("2026-04-22T08:02:00.000Z"),
        None,
    );
    write_json_document(&task_card_file, &source_task).expect("write source task");

    let first_review = maybe_create_captain_owned_review_task_card(
        &run_directory,
        &source_task,
        "2026-04-22T08:03:00.000Z",
    )
    .expect("first review task");
    let second_review = maybe_create_captain_owned_review_task_card(
        &run_directory,
        &source_task,
        "2026-04-22T08:04:00.000Z",
    )
    .expect("second review task");
    assert!(first_review.is_some());
    assert_eq!(second_review, None);
    assert_eq!(
        review_task_cards_for_source(&run_directory, "task-1").len(),
        1
    );
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(2)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_completion_queues_arbiter_review_after_mutation_task() {
    let workspace_dir = create_temp_path("completion-requires-arbiter-review");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-completion-review");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-completion-review",
            "task_card_id": "task-1",
            "decision": {
                "next_step": "advance",
                "can_advance": true,
                "summary": "captain checkpoint"
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");

    let response = create_ccc_orchestrate_payload(&json!({
        "run_id": "run-completion-review",
        "cwd": workspace_dir.to_string_lossy(),
        "resolve_outcome": "completed",
        "resolve_summary": "Captain wants to close after mutation fan-in."
    }))
    .expect("orchestrate payload");

    assert_eq!(response["next_step"], "execute_task");
    assert_eq!(response["scheduler_decision"]["action"]["kind"], "replan");
    assert_eq!(
        response["scheduler_decision"]["action"]["reason"],
        "captain follow-up task card is ready for explicit specialist dispatch"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(run_record["status"], "active");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card");
    assert_ne!(active_task_card_id, "task-1");
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(2)
    );

    let review_task = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{active_task_card_id}.json")),
    )
    .expect("review task");
    assert_eq!(review_task["task_kind"], "review");
    assert_eq!(review_task["assigned_role"], "verifier");
    assert_eq!(review_task["assigned_agent_id"], "arbiter");
    assert_eq!(review_task["review_of_task_card_ids"], json!(["task-1"]));

    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert!(longway["phases"]
        .as_array()
        .map(|phases| phases.iter().any(|phase| {
            phase.get("task_card_id").and_then(Value::as_str) == Some(active_task_card_id)
        }))
        .unwrap_or(false));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_recommend_entry_surfaces_companion_reader_route_for_filesystem_lookup() {
    let recommendation = create_ccc_recommend_entry_payload_for_policy(
        &json!({
            "request": "Inspect the current directory and summarize unnecessary files."
        }),
        Some("codex_cli_ccc_first"),
    );

    assert_eq!(recommendation["recommended_entrypoint"], "start");
    assert_eq!(
        recommendation["companion_tool_route_class"],
        "workspace_inspection"
    );
    assert_eq!(
        recommendation["companion_tool_names"],
        json!(["filesystem"])
    );
    assert_eq!(recommendation["companion_tool_operation"], "read");
    assert_eq!(
        recommendation["companion_tool_owner_role"],
        "companion_reader"
    );
    assert_eq!(
        recommendation["companion_tool_model"],
        expected_tool_routing_field("filesystem", "model", "default_model")
    );
    assert_eq!(
        recommendation["companion_tool_variant"],
        expected_tool_routing_field("filesystem", "variant", "default_variant")
    );
    assert_eq!(
        recommendation["companion_tool_execution_state"],
        "route_backed_specialist_owned"
    );
}

#[test]
fn companion_tool_route_treats_pure_gh_pr_lookup_as_read() {
    let route = create_companion_tool_route_payload_for_policy(
        &default_tool_routing_config(),
        "Use gh pr list to inspect open pull requests.",
        "none",
    );

    assert_eq!(route["route_class"], "git_inspection");
    assert_eq!(route["tool_names"], json!(["gh"]));
    assert_eq!(route["operation"], "read");
    assert_eq!(route["owner_role"], "companion_reader");
    assert_eq!(route["model"], "gpt-5.4-mini");
    assert_eq!(route["variant"], "high");
}

#[test]
fn companion_tool_route_treats_gh_release_create_as_mutation() {
    let route = create_companion_tool_route_payload_for_policy(
        &default_tool_routing_config(),
        "Use gh release create to publish the notes.",
        "none",
    );

    assert_eq!(route["route_class"], "git_mutation");
    assert_eq!(route["tool_names"], json!(["gh"]));
    assert_eq!(route["operation"], "mutation");
    assert_eq!(route["owner_role"], "companion_operator");
    assert_eq!(route["model"], "gpt-5.4-mini");
    assert_eq!(route["variant"], "high");
}

#[test]
fn companion_tool_route_treats_explicit_gh_release_mutation_verbs_as_mutation() {
    for verb in ["upload", "edit", "create", "delete"] {
        let route = create_companion_tool_route_payload_for_policy(
            &default_tool_routing_config(),
            &format!("Use gh release {verb} for v0.0.8-pre."),
            "none",
        );

        assert_eq!(route["route_class"], "git_mutation");
        assert_eq!(route["tool_names"], json!(["gh"]));
        assert_eq!(route["operation"], "mutation");
        assert_eq!(route["owner_role"], "companion_operator");
    }
}

#[test]
fn ccc_start_and_orchestrate_launch_companion_reader_with_mini_profile() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("companion-reader-launch");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable(&workspace_dir);

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 112,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Inspect the current directory and summarize unnecessary files.",
                    "title": "Inspect workspace files",
                    "intent": "Run a bounded read-only workspace inspection",
                    "scope": "Read-only filesystem evidence gathering only",
                    "acceptance": "Return a bounded summary without mutating the workspace",
                    "prompt": "Inspect the current directory and summarize unnecessary files only.",
                    "task_kind": "explore",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    assert!(
        start_response.get("error").is_none(),
        "unexpected ccc_start response: {start_response:?}"
    );

    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");
    assert_eq!(task_card["assigned_role"], "companion_reader");
    assert_eq!(task_card["assigned_agent_id"], "companion_reader");
    assert_eq!(task_card["sandbox_mode"], "read-only");
    assert_eq!(
        task_card["role_config_snapshot"]["model"],
        expected_role_config_field("companion_reader", "model")
    );
    assert_eq!(
        task_card["role_config_snapshot"]["variant"],
        expected_role_config_field("companion_reader", "variant")
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run.json");
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_route_class"],
        "workspace_inspection"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_owner_role"],
        "companion_reader"
    );
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);

    let orchestrate_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 113,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        orchestrate_response.get("error").is_none(),
        "unexpected ccc_orchestrate response: {orchestrate_response:?}"
    );

    let delegations_path = run_directory.join("delegations");
    let wait_deadline = SystemTime::now() + Duration::from_secs(3);
    let delegation = loop {
        let maybe_completed = fs::read_dir(&delegations_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| read_json_document(&entry.path()).ok())
            .find(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    == Some("completed")
            });
        if maybe_completed.is_some() || SystemTime::now() >= wait_deadline {
            break maybe_completed;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    .expect("completed delegation");

    assert_eq!(delegation["child_agent"]["agent_id"], "companion_reader");
    assert_eq!(delegation["child_agent"]["role"], "companion_reader");
    assert_eq!(delegation["worker_request"]["sandbox_mode"], "read-only");
    assert_eq!(
        delegation["latest_model_launch"]["actual_model"],
        expected_role_config_field("companion_reader", "model")
    );
    let expected_variant = expected_role_config_field("companion_reader", "variant");
    assert_eq!(
        delegation["latest_model_launch"]["actual_variant"],
        expected_variant
    );
    if let Some(expected_variant) = expected_variant.as_str() {
        let expected_entry = format!("model_reasoning_effort=\"{expected_variant}\"");
        assert!(delegation["latest_model_launch"]["actual_config_entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|entry| entry == expected_entry));
    }

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_assigns_companion_operator_for_git_mutation_requests() {
    let workspace_dir = create_temp_path("companion-operator-start");
    create_dir_all(&workspace_dir).expect("create workspace");

    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Stage the current changes and create a commit summarizing the update.",
        "title": "Apply a bounded git mutation",
        "intent": "Run a bounded git-backed mutation task",
        "scope": "Stage and commit only",
        "acceptance": "Persist a bounded git mutation task card",
        "prompt": "Stage the current changes and create a commit summarizing the update.",
        "task_kind": "execution"
    }))
    .expect("start payload");

    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let task_card_id = start_payload["task_card_id"]
        .as_str()
        .expect("task card id");
    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");

    assert_eq!(task_card["assigned_role"], "companion_operator");
    assert_eq!(task_card["assigned_agent_id"], "companion_operator");
    assert_eq!(task_card["sandbox_mode"], "workspace-write");
    assert_eq!(
        task_card["role_config_snapshot"]["model"],
        expected_role_config_field("companion_operator", "model")
    );
    assert_eq!(
        task_card["role_config_snapshot"]["variant"],
        expected_role_config_field("companion_operator", "variant")
    );
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_route_class"],
        "git_mutation"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_operation"],
        "mutation"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_owner_role"],
        "companion_operator"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_assigns_raider_for_release_install_script_repair_requests() {
    let workspace_dir = create_temp_path("release-install-repair-start");
    create_dir_all(&workspace_dir).expect("create workspace");

    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Repair release install and asset packaging scripts.",
        "title": "Repair release installer packaging",
        "intent": "Keep release/install script repair with the code specialist",
        "scope": "Fix install.sh, install.ps1, and scripts/release/build-release-asset.sh only",
        "acceptance": "Persist a bounded raider task card for release/install script repair",
        "prompt": "Fix install.sh, install.ps1, and scripts/release/build-release-asset.sh for the GitHub release asset packaging repair.",
        "task_kind": "execution"
    }))
    .expect("start payload");

    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let task_card_id = start_payload["task_card_id"]
        .as_str()
        .expect("task card id");
    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");

    assert_eq!(task_card["assigned_role"], "code specialist");
    assert_eq!(task_card["assigned_agent_id"], "raider");
    assert_eq!(
        task_card["routing_trace"]["release_install_script_repair_guard"],
        true
    );
    assert_eq!(
        run_record["latest_entry_trace"]["specialist_selected_role"],
        "code specialist"
    );
    assert_ne!(
        run_record["latest_entry_trace"]["companion_tool_owner_role"],
        "companion_operator"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_assigns_companion_operator_for_way_scoped_git_mutation_requests() {
    let workspace_dir = create_temp_path("companion-operator-way-start");
    create_dir_all(&workspace_dir).expect("create workspace");

    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Plan the release and commit the completed slice.",
        "title": "Plan and commit release slice",
        "intent": "Captain should route the git mutation through the companion operator",
        "scope": "Plan release slice and commit only the completed slice",
        "acceptance": "Persist a bounded companion operator task card for the git mutation",
        "prompt": "Use git commit for the completed release slice.",
        "task_kind": "way"
    }))
    .expect("start payload");

    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let task_card_id = start_payload["task_card_id"]
        .as_str()
        .expect("task card id");
    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");

    assert_eq!(task_card["assigned_role"], "companion_operator");
    assert_eq!(task_card["assigned_agent_id"], "companion_operator");
    assert_eq!(task_card["routing_trace"]["companion_route_enforced"], true);
    assert_eq!(
        run_record["latest_entry_trace"]["specialist_selected_role"],
        "companion_operator"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_operation"],
        "mutation"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["companion_tool_owner_role"],
        "companion_operator"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_auto_entry_creates_deterministic_bounded_run_when_policy_allows_it() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("auto-entry");
    create_dir_all(&workspace_dir).expect("create workspace");
    let parsed = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "request": "Retry Phase 3 Step 2 with a bounded planning pass"
    });
    let payload = create_ccc_auto_entry_payload_for_policy(
        &session_context,
        &parsed,
        Some("codex_cli_ccc_first"),
    )
    .expect("auto-entry payload");
    assert_eq!(payload["created"], true);
    assert_eq!(payload["run_selection"], "new_run_created");
    assert_eq!(payload["entrypoint_used"], "way");
    assert_eq!(payload["next_step"], "execute_task");
    assert_eq!(payload["can_advance"], true);
    assert_eq!(payload["answer_trace"]["selected_role"], "tactician");
    assert_eq!(payload["review_policy"]["state"], "skipped");
    assert_eq!(
        payload["current_task_card"]["review_policy"]["state"],
        "skipped"
    );
    assert_eq!(payload["active_run_scan_state"], "no_active_runs");
    assert_eq!(payload["fresh_active_run_count"], 0);
    assert_eq!(
        payload["active_run_scan"]["continuity_strategy"],
        "fresh_run_ok"
    );
    assert!(
        PathBuf::from(payload["run_directory"].as_str().expect("run directory"))
            .join("run.json")
            .exists()
    );
    let run_record = read_json_document(
        &PathBuf::from(payload["run_directory"].as_str().expect("run directory")).join("run.json"),
    )
    .expect("run record");
    assert_eq!(run_record["review_policy"]["state"], "skipped");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_auto_entry_cli_projection_sync_materializes_diff_visible_file() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("auto-entry-cli-projection");
    create_dir_all(&workspace_dir).expect("create workspace");
    let git_init = Command::new("git")
        .arg("-C")
        .arg(&workspace_dir)
        .arg("init")
        .output()
        .expect("run git init");
    assert!(
        git_init.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&git_init.stderr)
    );
    let parsed = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "request": "Retry the bounded planning pass"
    });
    let mut payload = create_ccc_auto_entry_payload_for_policy(
        &session_context,
        &parsed,
        Some("codex_cli_ccc_first"),
    )
    .expect("auto-entry payload");

    sync_auto_entry_projection_after_creation(&session_context, &mut payload)
        .expect("sync projection");

    assert_eq!(payload["created"], true);
    assert_eq!(
        payload["longway_projection"]["kind"],
        "ccc_longway_projection"
    );
    assert_eq!(
        payload["longway_projection"]["diff_visibility"]["status"],
        "git_intent_to_add"
    );
    let projection_path = workspace_dir.join("CCC_LONGWAY_PROJECTION.md");
    assert!(projection_path.exists());
    let projection_text = fs::read_to_string(projection_path).expect("read projection");
    assert!(projection_text.contains(payload["run_id"].as_str().expect("run id")));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_auto_entry_handles_long_multibyte_requests_without_panicking() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("auto-entry-multibyte");
    create_dir_all(&workspace_dir).expect("create workspace");
    let parsed = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "request": "지금 docs/V1_7_0_RIGHT_SIDEBAR_AGENT_DASHBOARD_PLAN.md 의 작업을 하고 있는데 남은 작업이 뭐야? 그리고 아직 안 끝난 항목과 막힌 부분까지 한국어로 자세히 정리해줘."
    });
    let payload = create_ccc_auto_entry_payload_for_policy(
        &session_context,
        &parsed,
        Some("codex_cli_ccc_first"),
    )
    .expect("auto-entry payload");
    assert_eq!(payload["created"], true);
    assert_eq!(payload["run_selection"], "new_run_created");
    assert_eq!(payload["entrypoint_used"], "way");
    assert_eq!(payload["next_step"], "execute_task");
    assert_eq!(payload["can_advance"], true);
    assert!(
        PathBuf::from(payload["run_directory"].as_str().expect("run directory"))
            .join("run.json")
            .exists()
    );
    assert!(
        payload["current_task_card"]["title"]
            .as_str()
            .expect("task card title")
            .chars()
            .count()
            <= 88
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_auto_entry_persists_completion_discipline_for_documented_finish_request() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("auto-entry-completion-discipline");
    create_dir_all(&workspace_dir).expect("create workspace");
    let parsed = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "request": "Continue Codex-Cli-Captain/docs/release/notes/v0.0.4.md and finish the remaining documented work end to end."
    });

    let payload = create_ccc_auto_entry_payload_for_policy(
        &session_context,
        &parsed,
        Some("codex_cli_ccc_first"),
    )
    .expect("auto-entry payload");

    assert_eq!(payload["created"], true);
    assert_eq!(payload["completion_discipline"]["state"], "required");
    assert_eq!(
        payload["completion_discipline"]["completion_mode"],
        "documented_completion_criteria"
    );
    assert_eq!(
        payload["answer_trace"]["completion_mode"],
        "documented_completion_criteria"
    );
    assert_eq!(
        payload["current_task_card"]["completion_discipline"]["state"],
        "required"
    );
    assert!(payload["current_task_card"]["acceptance"]
        .as_str()
        .unwrap_or_default()
        .contains("do not report success after a partial slice"));

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": payload["run_id"].clone(),
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["completion_discipline"]["completion_mode"],
        "documented_completion_criteria"
    );
    assert!(create_ccc_status_text(&status_payload).contains("Completion: state=required"));

    let run_record =
        read_json_document(&locator.run_directory.join("run.json")).expect("run record");
    assert_eq!(
        run_record["latest_entry_trace"]["completion_discipline"]["state"],
        "required"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_tool_reads_persisted_run_truth_from_workspace_run_id() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-run-id");
    create_dir_all(&workspace_dir).expect("create workspace");
    write_test_run_fixture(&workspace_dir, "run-123");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "ccc_status",
                "arguments": {
                    "run_id": "run-123",
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("response");

    assert_eq!(
        response["result"]["structuredContent"]["status"]["run_id"],
        "run-123"
    );
    assert_eq!(
        response["result"]["structuredContent"]["status"]["status"],
        "active"
    );
    assert_eq!(
        response["result"]["structuredContent"]["status"]["stage"],
        "execution"
    );
    assert_eq!(
        response["result"]["structuredContent"]["status"]["current_task_card"]["assigned_agent_id"],
        "raider"
    );
    assert_eq!(
        response["result"]["structuredContent"]["status"]["run_state"]["next_action"]["command"],
        "advance"
    );
    assert_eq!(
        response["result"]["structuredContent"]["status"]["longway"]["active_phase_name"],
        "execute"
    );
    let status_text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(status_text.starts_with("LongWay"));
    assert!(status_text.contains("["));
    assert!(!status_text.contains("Current Item:"));
    assert!(status_text.contains("Next:"));
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_state_contract_active_gate_across_operator_views() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-state-contract");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-state-contract");
    write_json_document(
        &run_directory.join("run-state.json"),
        &json!({
            "version": 1,
            "run_id": "run-state-contract",
            "updated_at": "2026-04-22T08:01:00.000Z",
            "event_count": 3,
            "last_event_id": "event-3",
            "current_phase_name": "execute",
            "next_action": {
                "command": "execute_task"
            }
        }),
    )
    .expect("write run-state");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-state-contract",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["state_contract"]["schema"],
        "ccc.state_contract.v1"
    );
    assert_eq!(
        status_payload["state_contract"]["state"],
        "mutation_pending"
    );
    assert_eq!(status_payload["state_contract"]["active_gate"], "mutation");
    assert_eq!(
        status_payload["state_contract"]["required_artifact"],
        "mutation_scope_and_evidence"
    );
    assert_eq!(
        status_payload["state_contract"]["next_step"],
        "execute_task"
    );
    assert_eq!(
        status_payload["state_contract"]["allowed_next_transitions"],
        json!(["dispatch_specialist", "record_fan_in", "record_fallback"])
    );
    assert_eq!(
        status_payload["state_contract"]["captain_allowed_action"],
        "spawn_subagent"
    );
    assert_eq!(
        status_payload["state_contract"]["captain_required_action"],
        "spawn_or_record_specialist"
    );
    assert_eq!(
        status_payload["app_panel"]["state_contract"]["active_gate"],
        "mutation"
    );

    let app_panel_text =
        crate::status_app_panel::create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Active Gate:"));
    assert!(app_panel_text.contains(
        "state=mutation-pending gate=mutation requires=mutation-scope-and-evidence next=execute-task"
    ));

    let projection = create_operator_longway_projection_text(&status_payload);
    assert!(projection.contains(
        "Active Gate: state=mutation-pending gate=mutation requires=mutation-scope-and-evidence next=execute-task"
    ));
    assert!(projection.contains("captain_allowed=spawn-subagent"));
    assert!(projection.contains("captain_required=spawn-or-record-specialist"));

    let markdown =
        crate::status_app_panel::create_codex_app_panel_markdown(&status_payload["app_panel"]);
    assert!(markdown.contains("## Active Gate"));
    assert!(markdown.contains("gate=mutation"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_payload_key_contract_keeps_app_panel_and_command_templates_stable() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-key-contract");
    create_dir_all(&workspace_dir).expect("create workspace");
    write_test_run_fixture(&workspace_dir, "run-key-contract");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-key-contract",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    let compact = create_ccc_status_compact_payload(&status_payload);

    assert_eq!(
        sorted_object_keys(&status_payload),
        vec![
            "active_agent_id",
            "active_checkpoint",
            "active_role",
            "active_task_card_id",
            "active_thread_id",
            "app_panel",
            "approval_state",
            "can_advance",
            "captain_action_contract",
            "captain_direct_mutation_guard",
            "child_agent_count",
            "code_graph",
            "completed_at",
            "completion_discipline",
            "context_health",
            "cost_routing",
            "created_at",
            "current_task_card",
            "cwd",
            "execution_strategy",
            "goal",
            "graph_context",
            "host_subagent_state",
            "latest_captain_intervention",
            "latest_delegate_result",
            "latest_entry_trace",
            "latest_handoff_id",
            "latest_sentinel_intervention",
            "lifecycle_hooks",
            "long_session_mitigation",
            "longway",
            "memory",
            "mutation_evidence_gate",
            "next_step",
            "operator_language",
            "output",
            "output_verbosity",
            "pending_captain_follow_up",
            "post_fan_in_captain_decision",
            "prompt_refinement",
            "reclaim_plan",
            "recovery_lane",
            "registry_evidence",
            "restart_handoff",
            "review_policy",
            "run_directory",
            "run_file",
            "run_id",
            "run_ref",
            "run_state",
            "run_truth_surface",
            "runtime_config",
            "scheduler",
            "sequence",
            "server_identity",
            "specialist_executor_count",
            "stage",
            "state_contract",
            "status",
            "task_card_count",
            "task_session_state",
            "token_usage",
            "token_usage_visibility",
            "updated_at",
            "visibility_signature",
            "way_clarification_request",
            "worker_visibility",
            "workflow_loop",
        ]
    );
    assert_eq!(
        sorted_object_keys(&status_payload["app_panel"]),
        vec![
            "active_checkpoint",
            "blockers",
            "captain_direct_mutation_guard",
            "context_health",
            "current_task",
            "fan_in",
            "lifecycle_hooks",
            "longway_progress",
            "next_captain_action",
            "post_fan_in_captain_decision",
            "recovery_lane",
            "registry_evidence",
            "render_strategy",
            "restart_handoff",
            "run",
            "scheduler",
            "schema",
            "specialist_lanes",
            "state_contract",
            "surface",
            "target_workspace",
            "task_session_state",
            "warnings",
            "workflow_loop",
            "workspace_state",
        ]
    );
    assert_eq!(
        sorted_object_keys(&compact["command_templates"]),
        vec![
            "checklist",
            "graph",
            "memory",
            "operator_transport",
            "orchestrate",
            "session_rollover",
            "start",
            "status",
            "subagent_update",
        ]
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_blocks_mutation_launch_without_persisted_evidence_or_approved_longway() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("mutation-evidence-gate-block");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-mutation-gate-block");
    write_json_document(
        &run_directory.join("run-state.json"),
        &json!({
            "version": 1,
            "run_id": "run-mutation-gate-block",
            "updated_at": "2026-04-22T08:01:00.000Z",
            "event_count": 3,
            "last_event_id": "event-3",
            "current_phase_name": "execute",
            "next_action": {
                "command": "execute_task"
            }
        }),
    )
    .expect("write run-state");
    write_json_document(
        &run_directory.join("orchestrator-state.json"),
        &json!({
            "run_id": "run-mutation-gate-block",
            "decision": {
                "next_step": "execute_task",
                "can_advance": true,
                "summary": "Fixture requests mutation dispatch."
            }
        }),
    )
    .expect("write orchestrator-state");

    let response = create_ccc_orchestrate_payload(&json!({
        "run_id": "run-mutation-gate-block",
        "cwd": workspace_dir.to_string_lossy(),
        "codex_bin": "/bin/false",
    }))
    .expect("orchestrate response");

    assert_eq!(response["can_advance"], false);
    assert!(response["launch_result"].is_null());
    assert_eq!(response["mutation_evidence_gate"]["blocked"], true);
    assert_eq!(
        response["mutation_evidence_gate"]["required_action"],
        "record_fan_in_evidence_or_approve_longway_before_mutation_dispatch"
    );
    assert!(!run_directory.join("delegations").exists());

    let locator = ResolvedRunLocator {
        cwd: workspace_dir.clone(),
        run_id: "run-mutation-gate-block".to_string(),
        run_directory: run_directory.clone(),
    };
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["mutation_evidence_gate"]["state"],
        "blocked_missing_evidence"
    );
    assert_eq!(status_payload["state_contract"]["state"], "blocked");
    assert_eq!(
        status_payload["state_contract"]["active_gate"],
        "mutation_evidence"
    );
    assert_eq!(
        status_payload["state_contract"]["required_artifact"],
        "persisted_approved_longway_or_fan_in_evidence"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn mutation_evidence_gate_allows_approved_longway_or_persisted_evidence() {
    let task_card = json!({
        "task_card_id": "task-1",
        "assigned_role": "code specialist",
        "assigned_agent_id": "raider"
    });

    let approved = create_mutation_evidence_gate_payload(
        &json!({ "approval_state": "approved_for_task_cards" }),
        &json!({}),
        &task_card,
    );
    assert_eq!(approved["state"], "allowed");
    assert_eq!(approved["approved_longway"], true);
    assert_eq!(approved["blocked"], false);

    let evidence_backed_task = json!({
        "task_card_id": "task-1",
        "assigned_role": "code specialist",
        "assigned_agent_id": "raider",
        "subagent_fan_in": {
            "summary": "Scout found the bounded mutation target.",
            "evidence_paths": ["rust/ccc-mcp/src/main.rs:4129"]
        }
    });
    let evidence_backed =
        create_mutation_evidence_gate_payload(&json!({}), &json!({}), &evidence_backed_task);
    assert_eq!(evidence_backed["state"], "allowed");
    assert_eq!(evidence_backed["persisted_evidence"], true);
    assert_eq!(evidence_backed["blocked"], false);
    assert_eq!(
        evidence_backed["evidence_sources"],
        json!([
            "task_card.subagent_fan_in.summary",
            "task_card.subagent_fan_in.evidence_paths"
        ])
    );
}

#[test]
fn ccc_status_preserves_simple_longway_text_without_owner_identity() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-longway-simple");
    create_dir_all(&workspace_dir).expect("create workspace");
    write_test_run_fixture(&workspace_dir, "run-simple-longway");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-simple-longway",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["longway"]["phase_rows"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );

    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.starts_with("LongWay"));
    assert!(status_text.contains("[>] Implement Rust ccc_status"));
    assert!(!status_text.contains("LongWay: 0/1 completed"));
    assert!(!status_text.contains("execute ["));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_checklist_renders_titled_completed_longway_phase_without_owner_identity() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("checklist-titled-completed-no-owner");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-titled-completed-no-owner");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "way",
            "active_phase_status": "pending",
            "phases": [{
                "finished_at": "2026-04-30T14:02:57.286Z",
                "phase_name": "way",
                "status": "completed",
                "task_card_id": "task-1",
                "title": "Implement 0.0.9 pre-release plan",
                "updated_at": "2026-04-30T14:02:57.286Z"
            }]
        }),
    )
    .expect("write live-shape longway");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-titled-completed-no-owner",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["longway"]["phase_rows"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        status_payload["longway"]["phase_rows"][0]["owner_agent"],
        Value::Null
    );
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    assert_eq!(
        checklist_payload["checklist"],
        "LongWay\n[x] Implement 0.0.9 pre-release plan"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_longway_owner_identity_rows() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-longway-owner-rows");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-owner-longway");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "mutate",
            "active_phase_status": "in_progress",
            "phases": [
                {
                    "task_card_id": "task-scout",
                    "phase_name": "inspect",
                    "title": "Clarify filesystem boundary",
                    "status": "completed",
                    "owner_agent": "ccc_scout"
                },
                {
                    "task_card_id": "task-raider",
                    "phase_name": "mutate",
                    "title": "Implement LongWay owner identity rows",
                    "status": "in_progress",
                    "owner_agent": "ccc_raider"
                }
            ]
        }),
    )
    .expect("write longway owner rows");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-owner-longway",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(status_payload["longway"]["completed_phase_count"], 1);
    assert_eq!(status_payload["longway"]["phase_count"], 2);
    assert_eq!(
        status_payload["longway"]["phase_rows"][1]["owner_agent"],
        "ccc_raider"
    );
    assert_eq!(
        create_ccc_status_compact_payload(&status_payload)["longway"]["phase_rows"][0]
            ["owner_agent"],
        "ccc_scout"
    );

    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains("[x] Clarify filesystem boundary [Observer(ccc_scout)]"));
    assert!(
        status_text.contains("[>] Implement LongWay owner identity rows [Marauder(ccc_raider)]")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_renders_task_item_unit_ownership_labels() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-longway-task-item-units");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-task-item-units");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "fan_in",
            "active_phase_status": "completed",
            "phases": [{
                "phase_name": "fan_in",
                "title": "Complete multi surface request",
                "status": "completed",
                "task_items": [
                    {
                        "task_item_id": "docs-lane",
                        "title": "Update docs",
                        "status": "completed",
                        "owner_agent": "ccc_scribe"
                    },
                    {
                        "task_item_id": "code-lane",
                        "title": "Update status text",
                        "status": "completed",
                        "owner_agent": "ccc_raider"
                    }
                ]
            }]
        }),
    )
    .expect("write longway task item units");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-task-item-units",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    let row = &status_payload["longway"]["phase_rows"][0];
    assert_eq!(row["owner_agent"], "ccc_scribe");
    assert_eq!(
        row["task_unit_labels"],
        json!(["docs-lane:ccc_scribe", "code-lane:ccc_raider"])
    );

    let expected =
        "[x] Complete multi surface request [Adjutant(ccc_scribe)] units=docs-lane:ccc_scribe,code-lane:ccc_raider";
    assert!(create_ccc_status_text(&status_payload).contains(expected));
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    assert!(checklist_payload["checklist"]
        .as_str()
        .unwrap_or_default()
        .contains(expected));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_falls_back_to_parallel_lane_unit_ownership_labels() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-longway-lane-units");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-lane-units");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "fan_in",
            "active_phase_status": "completed",
            "phases": [{
                "phase_name": "fan_in",
                "title": "Complete parallel implementation",
                "status": "completed",
                "task_card_id": "task-1"
            }]
        }),
    )
    .expect("write longway lane units");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    task_card["parallel_fanout"] = json!({
        "lanes": [
            {
                "lane_id": "raider-a",
                "lifecycle": {
                    "status": "completed",
                    "child_agent_id": "ccc_raider"
                }
            },
            {
                "lane_id": "raider-b",
                "lifecycle": {
                    "status": "completed",
                    "child_agent_id": "ccc_raider"
                }
            }
        ]
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-lane-units",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    let row = &status_payload["longway"]["phase_rows"][0];
    assert_eq!(row["owner_agent"], Value::Null);
    assert_eq!(
        row["task_unit_labels"],
        json!(["raider-a:ccc_raider", "raider-b:ccc_raider"])
    );

    let expected =
        "[x] Complete parallel implementation units=raider-a:ccc_raider,raider-b:ccc_raider lifecycle=terminal";
    assert!(create_ccc_status_text(&status_payload).contains(expected));
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    assert!(checklist_payload["checklist"]
        .as_str()
        .unwrap_or_default()
        .contains(expected));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_projects_distinct_longway_planned_rows() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-longway-planned-rows");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-planned-longway");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "way",
            "active_phase_status": "in_progress",
            "current_item": "item-1",
            "phases": [{
                "task_card_id": "task-1",
                "phase_name": "way",
                "title": "Create the LongWay",
                "status": "in_progress",
                "owner_agent": "tactician"
            }],
            "planned_rows": [{
                "title": "Implement planned row schema",
                "planned_role": "implementation_specialist",
                "planned_agent_id": "raider-a",
                "scope": "schemas and status projection",
                "acceptance": "planned rows are visible without phase lifecycle mutation",
                "status": "planned",
                "evidence_links": ["schemas/longway.schema.json"],
                "routing_summary": "schema-only follow-up row"
            }]
        }),
    )
    .expect("write planned longway rows");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-planned-longway",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(status_payload["longway"]["phase_count"], 1);
    assert_eq!(status_payload["longway"]["planned_row_count"], 1);
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["planned_agent_id"],
        "raider-a"
    );
    assert_eq!(
        status_payload["longway"]["phase_rows"][0]["planned_rows"][0]["planned_agent_id"],
        "raider-a"
    );
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["planning_detail"]["scope"],
        "schemas and status projection"
    );
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["planning_detail"]["evidence_count"],
        1
    );
    assert_eq!(
        create_ccc_status_compact_payload(&status_payload)["longway"]["planned_rows"][0]
            ["planned_role"],
        "implementation_specialist"
    );

    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains(
        "\n  plan: Implement planned row schema [raider-a] role=implementation_specialist original_status=planned"
    ));
    assert!(!status_text.contains("sources=agent:"));
    assert!(!status_text.contains("scope=\"schemas and status projection\""));
    assert!(!status_text
        .contains("accept=\"planned rows are visible without phase lifecycle mutation\""));
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    let checklist = checklist_payload["checklist"].as_str().unwrap_or("");
    assert!(checklist.starts_with("LongWay"));
    assert!(checklist.contains(
        "plan: Implement planned row schema [raider-a] role=implementation_specialist original_status=planned"
    ));
    assert!(!checklist.contains("+---"));
    assert!(!checklist.contains("sources="));
    assert!(!checklist.contains("scope="));
    assert!(!checklist.contains("accept="));
    assert!(!checklist.contains("CCC LongWay"));
    assert!(!checklist.contains("Gauge:"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_projects_completed_planned_row_from_task_card_lifecycle() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-completed-planned-row-projection");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-completed-planned-row");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "execute",
            "active_phase_status": "completed",
            "phases": [{
                "task_card_id": "task-1",
                "phase_name": "execute",
                "title": "Execute planned row",
                "status": "completed",
                "owner_agent": "raider"
            }],
            "planned_rows": [{
                "title": "Minimal raider mutation row",
                "planned_role": "code specialist",
                "planned_agent_id": "raider",
                "task_card_id": "task-1",
                "status": "materialized"
            }]
        }),
    )
    .expect("write planned longway rows");
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["subagent_fan_in"] = json!({
        "status": "completed",
        "summary": "Raider fan-in completed."
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-completed-planned-row",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["app_panel"]["longway_progress"]["planned_rows"][0]["status"],
        "completed"
    );
    assert!(
        crate::status_app_panel::create_codex_app_panel_text(&status_payload["app_panel"])
            .contains("[x] Minimal raider mutation row -> Marauder(ccc_raider)")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_projects_recovered_planned_row_from_codex_exec_fallback() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-recovered-planned-row-projection");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-recovered-planned-row");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "execute",
            "active_phase_status": "completed",
            "phases": [{
                "task_card_id": "task-1",
                "phase_name": "execute",
                "title": "Execute recovered planned row",
                "status": "completed",
                "owner_agent": "raider"
            }],
            "planned_rows": [{
                "title": "Recover stalled raider row",
                "planned_role": "code specialist",
                "planned_agent_id": "raider",
                "task_card_id": "task-1",
                "status": "materialized"
            }]
        }),
    )
    .expect("write planned longway rows");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["subagent_fallback"] = json!({
        "reason": "child_timeout",
        "recorded_at": "2026-04-25T00:00:00.000Z"
    });
    task_card["subagent_lifecycle"] = json!({
        "status": "stalled",
        "child_agent_id": "ccc_raider",
        "summary": "Host subagent stalled before codex exec recovery."
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");
    create_dir_all(run_directory.join("delegations")).expect("create delegations");
    write_json_document(
        &run_directory
            .join("delegations")
            .join("delegation-recovered.json"),
        &json!({
            "delegation_id": "delegation-recovered",
            "task_card_id": "task-1",
            "updated_at": "2026-04-25T00:01:00.000Z",
            "result_summary": "codex exec recovered the stalled row.",
            "child_agent": {
                "agent_id": "raider",
                "role": "code specialist",
                "status": "completed"
            }
        }),
    )
    .expect("write delegation");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-recovered-planned-row",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["recovery"]["mode"],
        "codex_exec"
    );
    let app_panel_text =
        crate::status_app_panel::create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("[x] Recover stalled raider row -> Marauder(ccc_raider)"));
    assert!(app_panel_text.contains("recovered=codex_exec"));
    assert!(app_panel_text.contains("reason=child-timeout"));
    assert!(app_panel_text.contains("primary=stalled"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_review_state_in_full_compact_and_text_payloads() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-review-state");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review");
    write_json_document(
        &run_directory.join("task-cards").join("task-1.json"),
        &json!({
            "task_card_id": "task-1",
            "run_id": "run-review",
            "title": "Review raider result",
            "intent": "Verify implementation output",
            "scope": "Review only",
            "status": "active",
            "task_kind": "review",
            "review_of_task_card_ids": ["task-source"],
            "orchestrator_review_gate": "after_child_completion",
            "verification_state": "needs_work",
            "review_pass_count": 1,
            "assigned_role": "arbiter",
            "assigned_agent_id": "arbiter"
        }),
    )
    .expect("write review task-card");

    let payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: "run-review".to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");

    assert_eq!(
        payload["current_task_card"]["review_of_task_card_ids"],
        json!(["task-source"])
    );
    assert_eq!(
        payload["current_task_card"]["orchestrator_review_gate"],
        "after_child_completion"
    );
    assert_eq!(
        payload["current_task_card"]["verification_state"],
        "needs_work"
    );
    assert_eq!(payload["current_task_card"]["review_pass_count"], 1);

    let compact = create_ccc_status_compact_payload(&payload);
    assert_eq!(
        compact["current_task_card"]["review_of_task_card_ids"],
        json!(["task-source"])
    );
    assert_eq!(
        compact["current_task_card"]["verification_state"],
        "needs_work"
    );
    assert_eq!(compact["current_task_card"]["review_pass_count"], 1);

    let text = create_ccc_status_text(&payload);
    assert!(text.contains(
        "Review: of=task-source gate=after_child_completion verification=needs_work passes=1"
    ));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_review_policy_for_skipped_and_active_required_review() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-review-policy");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-policy");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task-card");
    task_card["review_policy"] = create_review_policy_payload(
        "List the current workspace files.",
        "lookup",
        "single_scoped_task",
        Some("2026-04-22T08:01:00.000Z"),
        None,
    );
    write_json_document(&task_card_file, &task_card).expect("write skipped policy");

    let locator = ResolvedRunLocator {
        cwd: workspace_dir.clone(),
        run_id: "run-review-policy".to_string(),
        run_directory: run_directory.clone(),
    };
    let skipped_payload =
        create_ccc_status_payload(&session_context, &locator).expect("skipped status");
    assert_eq!(
        skipped_payload["current_task_card"]["review_policy"]["state"],
        "skipped"
    );
    assert_eq!(
        create_ccc_status_compact_payload(&skipped_payload)["current_task_card"]["review_policy"]
            ["state"],
        "skipped"
    );
    let skipped_text = create_ccc_status_text(&skipped_payload);
    assert!(skipped_text.contains("Review: state=skipped decision=skip risk=low reviewers=0/1"));
    assert_eq!(
        skipped_payload["captain_action_contract"]["allowed_action"],
        "captain_advance"
    );

    task_card["review_policy"] = json!({
        "decision": "require",
        "state": "running",
        "risk": "high",
        "required": true,
        "recommended_reviewers": 1,
        "reviewer_cap": 1,
        "active_reviewers": 1,
        "reason_code": "high_risk_or_explicit_review",
        "summary": "Required review is active.",
        "recorded_at": "2026-04-22T08:02:00.000Z"
    });
    write_json_document(&task_card_file, &task_card).expect("write running policy");

    let running_payload =
        create_ccc_status_payload(&session_context, &locator).expect("running status");
    assert_eq!(
        running_payload["current_task_card"]["review_policy"]["state"],
        "running"
    );
    let running_text = create_ccc_status_text(&running_payload);
    assert!(running_text.contains("Review: state=running decision=require risk=high reviewers=1/1"));
    assert_eq!(
        running_payload["captain_action_contract"]["allowed_action"],
        "review_required"
    );
    assert_eq!(
        running_payload["captain_action_contract"]["required_action"],
        "spawn_or_merge_review"
    );

    task_card["review_policy"] = json!({
        "decision": "recommend_single",
        "state": "recommended",
        "risk": "moderate",
        "required": false,
        "recommended_reviewers": 1,
        "reviewer_cap": 1,
        "active_reviewers": 0,
        "reason_code": "moderate_risk_mutation",
        "summary": "Single review is recommended.",
        "recorded_at": "2026-04-22T08:03:00.000Z"
    });
    write_json_document(&task_card_file, &task_card).expect("write recommended policy");

    let recommended_payload =
        create_ccc_status_payload(&session_context, &locator).expect("recommended status");
    assert_eq!(
        recommended_payload["current_task_card"]["review_policy"]["state"],
        "recommended"
    );
    assert_eq!(
        recommended_payload["captain_action_contract"]["allowed_action"],
        "review_required"
    );
    assert_eq!(
        recommended_payload["captain_action_contract"]["direct_mutation_allowed"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_review_passed_fan_in_without_auto_accepting() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("review-passed-fan-in");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-passed");
    write_review_task_card_fixture(&run_directory, "run-review-passed");

    let update_payload = create_ccc_subagent_update_payload(&json!({
        "run_id": "run-review-passed",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review-passed",
        "status": "completed",
        "summary": "Reviewer found the implementation acceptable.",
        "fan_in_status": "passed",
        "review_outcome": "passed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_accept",
        "open_questions": [],
        "findings": [],
        "confidence": "high"
    }))
    .expect("review passed update");
    assert_eq!(update_payload["review_outcome"], "passed");
    assert_eq!(
        update_payload["review_fan_in"]["authority"],
        "captain_decides_after_review"
    );

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-review-passed",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(status_payload["next_step"], "await_fan_in");
    assert_eq!(status_payload["can_advance"], true);
    assert_eq!(
        status_payload["current_task_card"]["verification_state"],
        "passed"
    );
    assert_eq!(status_payload["current_task_card"]["review_pass_count"], 1);
    assert_eq!(
        status_payload["current_task_card"]["review_policy"]["state"],
        "passed"
    );
    assert_eq!(
        status_payload["current_task_card"]["review_policy"]["active_reviewers"],
        0
    );
    assert_eq!(
        status_payload["current_task_card"]["review_fan_in"]["outcome"],
        "passed"
    );
    assert_eq!(
        status_payload["current_task_card"]["review_fan_in"]["unresolved_finding_count"],
        0
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["current_task_card"]["review_fan_in"]["outcome"],
        "passed"
    );
    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("outcome=passed"));
    assert!(text.contains("next=captain-accept"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_keeps_worker_and_review_lifecycles_separate() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("lifecycle-split-worker-review");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-lifecycle-split");

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-lifecycle-split",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-worker",
        "status": "completed",
        "summary": "Worker completed the implementation.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("worker completed update");
    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-lifecycle-split",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-worker",
        "status": "merged",
        "summary": "Captain merged the worker result."
    }))
    .expect("worker merged update");

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-lifecycle-split",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review",
        "status": "spawned",
        "summary": "Review was spawned."
    }))
    .expect("review spawned update");

    let after_spawn = read_active_task_card(&run_directory);
    assert_eq!(after_spawn["subagent_lifecycle"]["status"], "merged");
    assert_eq!(
        after_spawn["subagent_lifecycle"]["child_agent_id"],
        "ccc_raider"
    );
    assert_eq!(after_spawn["review_lifecycle"]["status"], "spawned");
    assert_eq!(after_spawn["review_lifecycle"]["child_agent_id"], "arbiter");

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-lifecycle-split",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review",
        "status": "completed",
        "summary": "Review passed the worker result.",
        "fan_in_status": "passed",
        "review_outcome": "passed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_accept",
        "open_questions": [],
        "findings": [],
        "confidence": "high"
    }))
    .expect("review completed update");
    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-lifecycle-split",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review",
        "status": "merged",
        "summary": "Captain merged the review result."
    }))
    .expect("review merged update");

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(task_card["subagent_lifecycle"]["status"], "merged");
    assert_eq!(
        task_card["subagent_lifecycle"]["thread_id"],
        "thread-worker"
    );
    assert_eq!(
        task_card["subagent_lifecycle"]["summary"],
        "Captain merged the worker result."
    );
    assert_eq!(task_card["review_lifecycle"]["status"], "merged");
    assert_eq!(task_card["review_lifecycle"]["thread_id"], "thread-review");
    assert_eq!(
        task_card["review_lifecycle"]["summary"],
        "Captain merged the review result."
    );

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-lifecycle-split",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["child_agent_id"],
        "ccc_raider"
    );
    assert_eq!(
        status_payload["current_task_card"]["review_lifecycle"]["child_agent_id"],
        "arbiter"
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["current_task_card"]["subagent_lifecycle"]["status"],
        "merged"
    );
    assert_eq!(
        compact["current_task_card"]["review_lifecycle"]["status"],
        "merged"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_blocks_repeated_review_pass_after_cap() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("review-pass-cap");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-pass-cap");
    write_review_task_card_fixture(&run_directory, "run-review-pass-cap");

    let first_update = create_ccc_subagent_update_payload(&json!({
        "run_id": "run-review-pass-cap",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review-pass-cap-1",
        "status": "completed",
        "summary": "Reviewer found the implementation acceptable.",
        "fan_in_status": "passed",
        "review_outcome": "passed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_accept",
        "open_questions": [],
        "findings": [],
        "confidence": "high"
    }))
    .expect("first review passed update");
    assert_eq!(first_update["review_outcome"], "passed");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-review-pass-cap",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let first_status =
        create_ccc_status_payload(&session_context, &locator).expect("first status payload");
    assert_eq!(first_status["current_task_card"]["review_pass_count"], 1);
    assert_eq!(
        first_status["current_task_card"]["verification_state"],
        "passed"
    );

    let second_update = create_ccc_subagent_update_payload(&json!({
        "run_id": "run-review-pass-cap",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review-pass-cap-2",
        "status": "completed",
        "summary": "Second reviewer pass arrived after the configured cap.",
        "fan_in_status": "passed",
        "review_outcome": "passed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_accept",
        "open_questions": [],
        "findings": [],
        "confidence": "high"
    }))
    .expect("repeated review passed update");
    assert_eq!(second_update["review_outcome"], "blocked");
    assert_eq!(second_update["review_fan_in"]["outcome"], "blocked");
    assert_eq!(
        second_update["review_fan_in"]["captain_next_decision"],
        "captain_decision"
    );
    assert!(second_update["review_fan_in"]["unresolved_findings"][0]
        .as_str()
        .unwrap()
        .contains("Maximum review pass count reached"));

    let capped_status =
        create_ccc_status_payload(&session_context, &locator).expect("capped status payload");
    assert_eq!(capped_status["current_task_card"]["review_pass_count"], 1);
    assert_eq!(
        capped_status["current_task_card"]["verification_state"],
        "blocked"
    );
    assert_eq!(
        capped_status["current_task_card"]["review_fan_in"]["outcome"],
        "blocked"
    );
    assert_eq!(
        capped_status["current_task_card"]["review_fan_in"]["unresolved_finding_count"],
        1
    );
    assert_eq!(
        capped_status["current_task_card"]["review_policy"]["state"],
        "captain_decision_required"
    );
    assert_eq!(
        capped_status["captain_action_contract"]["allowed_action"],
        "captain_decision_required"
    );
    assert_eq!(
        capped_status["captain_action_contract"]["required_action"],
        "ccc_orchestrate"
    );
    assert!(create_ccc_status_text(&capped_status).contains("state=captain-decision-required"));
    assert!(create_ccc_status_text(&capped_status).contains("passes=1"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_infers_reviewer_failure_as_needs_work() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("review-failed-inferred");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-failed");
    write_review_task_card_fixture(&run_directory, "run-review-failed");

    let update_payload = create_ccc_subagent_update_payload(&json!({
        "run_id": "run-review-failed",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review-failed",
        "status": "failed",
        "summary": "Reviewer found a behavioral regression.",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:12000"],
        "next_action": "captain_repair",
        "open_questions": ["Repair the failing path before accept."],
        "findings": ["Runtime path regressed after the worker change."],
        "confidence": "medium"
    }))
    .expect("review failed update");
    assert_eq!(update_payload["review_outcome"], "needs_work");
    assert_eq!(update_payload["review_fan_in"]["outcome"], "needs_work");

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: "run-review-failed".to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["verification_state"],
        "needs_work"
    );
    assert_eq!(
        status_payload["current_task_card"]["review_policy"]["state"],
        "needs_work"
    );
    assert_eq!(
        status_payload["current_task_card"]["review_fan_in"]["unresolved_findings"][0],
        "Runtime path regressed after the worker change."
    );
    assert_eq!(
        status_payload["current_task_card"]["review_fan_in"]["captain_next_decision"],
        "captain_repair"
    );
    assert!(create_ccc_status_text(&status_payload).contains("outcome=needs-work"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_review_needs_work_and_blocked_findings() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("review-needs-work-blocked");
    create_dir_all(&workspace_dir).expect("create workspace");
    let needs_work_run = write_test_run_fixture(&workspace_dir, "run-review-needs-work");
    write_review_task_card_fixture(&needs_work_run, "run-review-needs-work");

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-review-needs-work",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review-needs-work",
        "status": "completed",
        "summary": "Reviewer found a missing regression test.",
        "fan_in_status": "needs_work",
        "review_outcome": "needs_work",
        "evidence_paths": ["tests/review.rs:12"],
        "next_action": "captain_repair",
        "open_questions": ["Should the captain request a same-raider repair?"],
        "findings": ["Missing regression test for the review path."],
        "confidence": "medium"
    }))
    .expect("needs_work update");

    let needs_work_status = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: "run-review-needs-work".to_string(),
            run_directory: needs_work_run.clone(),
        },
    )
    .expect("needs work status");
    assert_eq!(
        needs_work_status["current_task_card"]["verification_state"],
        "needs_work"
    );
    assert_eq!(
        needs_work_status["current_task_card"]["review_fan_in"]["outcome"],
        "needs_work"
    );
    assert_eq!(
        needs_work_status["current_task_card"]["review_fan_in"]["unresolved_findings"][0],
        "Missing regression test for the review path."
    );
    assert_eq!(
        needs_work_status["current_task_card"]["review_fan_in"]["captain_next_decision"],
        "captain_repair"
    );

    let blocked_run = write_test_run_fixture(&workspace_dir, "run-review-blocked");
    write_review_task_card_fixture(&blocked_run, "run-review-blocked");
    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-review-blocked",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "arbiter",
        "thread_id": "thread-review-blocked",
        "status": "failed",
        "summary": "Reviewer could not inspect the required external path.",
        "fan_in_status": "blocked",
        "review_outcome": "blocked",
        "evidence_paths": [],
        "next_action": "captain_request_operator_input",
        "open_questions": ["Need approval for /private/review-source."],
        "findings": ["External path /private/review-source was blocked."],
        "confidence": "low"
    }))
    .expect("blocked update");
    let blocked_status = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: "run-review-blocked".to_string(),
            run_directory: blocked_run.clone(),
        },
    )
    .expect("blocked status");
    assert_eq!(
        blocked_status["current_task_card"]["verification_state"],
        "blocked"
    );
    assert_eq!(
        blocked_status["current_task_card"]["review_fan_in"]["outcome"],
        "blocked"
    );
    assert_eq!(
        blocked_status["current_task_card"]["review_fan_in"]["unresolved_finding_count"],
        1
    );
    assert!(create_ccc_status_text(&blocked_status).contains("outcome=blocked"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_surfaces_review_stalled_and_reclaimed_outcomes() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("review-stalled-reclaimed");
    create_dir_all(&workspace_dir).expect("create workspace");

    for (run_id, lifecycle_status, review_outcome) in [
        ("run-review-stalled", "stalled", "stalled"),
        ("run-review-reclaimed", "reclaimed", "reclaimed"),
    ] {
        let run_directory = write_test_run_fixture(&workspace_dir, run_id);
        write_review_task_card_fixture(&run_directory, run_id);
        create_ccc_subagent_update_payload(&json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
            "child_agent_id": "arbiter",
            "thread_id": format!("thread-{review_outcome}"),
            "status": lifecycle_status,
            "summary": format!("Review ended as {review_outcome}."),
            "fan_in_status": review_outcome,
            "review_outcome": review_outcome,
            "evidence_paths": ["src/main.rs:1"],
            "next_action": "captain_replan",
            "open_questions": [format!("Review {review_outcome}; captain must choose next step.")],
            "confidence": "low"
        }))
        .expect("review terminal update");

        let status_payload = create_ccc_status_payload(
            &session_context,
            &ResolvedRunLocator {
                cwd: workspace_dir.clone(),
                run_id: run_id.to_string(),
                run_directory,
            },
        )
        .expect("status payload");
        assert_eq!(status_payload["next_step"], "await_fan_in");
        assert_eq!(
            status_payload["current_task_card"]["verification_state"],
            "blocked"
        );
        assert_eq!(
            status_payload["current_task_card"]["review_fan_in"]["outcome"],
            review_outcome
        );
        assert_eq!(
            status_payload["host_subagent_state"]["pending_merge_count"],
            1
        );
        assert!(
            create_ccc_status_text(&status_payload).contains(&format!("outcome={review_outcome}"))
        );
    }

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_intervention_unsatisfactory_same_specialist_repair_intent() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("intervention-same-specialist");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "same-repair");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-repair",
        "status": "completed",
        "summary": "Raider returned a partial fix missing the regression assertion.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": ["Need one bounded repair for the missing assertion."],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "The same specialist can repair the missing assertion without changing scope.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));

    assert_eq!(
        update_payload["captain_intervention"]["chosen_next_action"],
        "amend_same_worker"
    );
    assert_eq!(
        update_payload["captain_intervention"]["next_action_blocked"],
        false
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["action"],
        "amend_same_worker"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "raider"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["budget_key"],
        "retry"
    );
    assert!(
        update_payload["captain_intervention"]["pending_follow_up"]["prompt"]
            .as_str()
            .unwrap_or_default()
            .contains("Bounded same-specialist amendment only.")
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.clone(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["captain_intervention"]["classification"],
        "bounded_scope_amendment"
    );
    assert_eq!(
        status_payload["latest_captain_intervention"]["chosen_next_action"],
        "amend_same_worker"
    );
    assert_eq!(
        create_ccc_status_compact_payload(&status_payload)["current_task_card"]
            ["captain_intervention"]["chosen_next_action"],
        "amend_same_worker"
    );
    assert_eq!(
        status_payload["pending_captain_follow_up"]["action"],
        "amend_same_worker"
    );
    assert!(create_ccc_status_text(&status_payload)
            .contains("Intervention: class=bounded-scope-amendment next=amend-same-worker follow_up=amend-same-worker"));

    let activity_payload = create_ccc_activity_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id,
            run_directory: run_directory.clone(),
        },
    )
    .expect("activity payload");
    assert_eq!(
        activity_payload["latest_captain_intervention"]["chosen_next_action"],
        "amend_same_worker"
    );
    assert!(
        create_ccc_activity_text(&activity_payload).contains("intervention_next=amend_same_worker")
    );
    assert!(
        create_ccc_activity_text(&activity_payload).contains("pending_follow_up=amend_same_worker")
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(
        run_record["latest_captain_intervention"]["chosen_next_action"],
        "amend_same_worker"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["captain_intervention"]["authority"],
        "captain_decides_intervention"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_sentinel_enforce_for_direct_captain_bypass() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("sentinel-enforce-bypass");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "sentinel-enforce");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "captain",
        "thread_id": "thread-sentinel-enforce",
        "status": "completed",
        "summary": "Captain directly produced output for a specialist-owned task.",
        "fan_in_status": "completed",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:1"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "medium"
    }));

    assert_eq!(
        update_payload["sentinel_intervention"]["classification"],
        "enforce"
    );
    assert_eq!(
        update_payload["sentinel_intervention"]["next_action"],
        "require_acceptance_gate"
    );
    assert_eq!(
        update_payload["sentinel_intervention"]["policy_drift"]["direct_captain_bypass"],
        true
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.clone(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(
        status_payload["latest_sentinel_intervention"]["classification"],
        "enforce"
    );
    assert_eq!(
        create_ccc_status_compact_payload(&status_payload)["latest_sentinel_intervention"]
            ["next_action"],
        "require_acceptance_gate"
    );
    assert!(create_ccc_status_text(&status_payload).contains("Sentinel: class=enforce"));

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(
        run_record["latest_entry_trace"]["sentinel_intervention"]["authority"],
        "sentinel_guardrail"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_sentinel_observe_from_sentinel_report() {
    let workspace_dir = create_temp_path("sentinel-observe-report");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "sentinel-observe");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_sentinel",
        "thread_id": "thread-sentinel-observe",
        "status": "completed",
        "summary": "Sentinel observed that ownership remains with the bounded code specialist.",
        "fan_in_status": "completed",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:1"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "high",
        "sentinel_classification": "observe",
        "sentinel_next_action": "continue_current_route",
        "sentinel_rationale": "Current route remains within the assigned mutation owner."
    }));

    assert_eq!(
        update_payload["sentinel_intervention"]["classification"],
        "observe"
    );
    assert_eq!(
        update_payload["sentinel_intervention"]["source"],
        "sentinel_subagent"
    );
    assert_eq!(
        update_payload["sentinel_intervention"]["next_action"],
        "continue_current_route"
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(
        task_card["sentinel_intervention_history"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_canonicalizes_unsatisfactory_and_queues_specialist_follow_up() {
    let workspace_dir = create_temp_path("intervention-unsatisfactory-alias");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) =
        start_intervention_test_run(&workspace_dir, "unsatisfactory-alias");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-unsatisfactory-alias",
        "status": "completed",
        "summary": "The returned change is still incomplete.",
        "fan_in_status": "unsatisfactory",
        "review_outcome": "unsatisfactory",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": ["Queue one bounded repair with the original specialist."],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "The original specialist should make the narrowed repair.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));

    assert_eq!(update_payload["review_outcome"], "needs_work");
    assert_eq!(update_payload["fan_in"]["status"], "needs_work");
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["action"],
        "amend_same_worker"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_role"],
        "code specialist"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "raider"
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(task_card["subagent_fan_in"]["status"], "needs_work");
    assert_eq!(
        task_card["captain_intervention"]["pending_follow_up"]["assigned_role"],
        "code specialist"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_generated_raider_repair_ignores_stale_verifier_assignment() {
    let workspace_dir = create_temp_path("intervention-generated-raider-repair");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) =
        start_intervention_test_run(&workspace_dir, "generated-raider-repair");

    let active_task_card = read_active_task_card(&run_directory);
    let active_task_card_id = active_task_card["task_card_id"]
        .as_str()
        .expect("active task-card id");
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{active_task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["assigned_role"] = json!("verifier");
    task_card["assigned_agent_id"] = json!("arbiter");
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-generated-raider-repair",
        "status": "completed",
        "summary": "Managed raider returned an incomplete implementation repair.",
        "fan_in_status": "unsatisfactory",
        "review_outcome": "unsatisfactory",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": ["Keep repair with the reporting code specialist."],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "The generated raider should make the narrowed repair.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));

    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_role"],
        "code specialist"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "raider"
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(
        task_card["captain_intervention"]["pending_follow_up"]["assigned_role"],
        "code specialist"
    );
    assert_eq!(
        task_card["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "raider"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_intervention_unsatisfactory_reassignment_intent() {
    let workspace_dir = create_temp_path("intervention-reassign");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "reassign");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-reassign",
        "status": "failed",
        "summary": "The implementation approach modified the wrong ownership boundary.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["docs/project-plan.md:180"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "The selected role or approach is wrong; captain should choose a better-fit specialist.",
        "chosen_next_action": "reassign",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        },
        "reassign_target": {
            "assigned_role": "explorer",
            "assigned_agent_id": "scout",
            "scope": "Inspect ownership boundary before another mutation.",
            "prompt": "Inspect the ownership boundary and report the smallest safe reassignment plan."
        }
    }));

    assert_eq!(
        update_payload["captain_intervention"]["chosen_next_action"],
        "reassign"
    );
    assert_eq!(
        update_payload["captain_intervention"]["next_action_blocked"],
        false
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["action"],
        "reassign"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "scout"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["budget_key"],
        "reassign"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["prompt"],
        "Inspect the ownership boundary and report the smallest safe reassignment plan."
    );
    let task_card = read_active_task_card(&run_directory);
    assert_eq!(
        task_card["captain_intervention"]["classification"],
        "direction_or_risk_correction"
    );
    assert_eq!(
        task_card["captain_intervention_history"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_clarification_only_intervention() {
    let workspace_dir = create_temp_path("intervention-clarification");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "clarification");

    parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-clarify",
        "status": "running",
        "summary": "Captain clarified the acceptance wording while work remains active.",
        "evidence_paths": [],
        "open_questions": ["Clarify whether docs updates are in scope."],
        "intervention_classification": "clarification_only",
        "intervention_rationale": "The worker needs a bounded clarification, not a repair or reassignment.",
        "chosen_next_action": "clarify",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(
        task_card["captain_intervention"]["classification"],
        "clarification_only"
    );
    assert_eq!(
        task_card["captain_intervention"]["chosen_next_action"],
        "clarify"
    );
    assert_eq!(
        task_card["captain_intervention"]["next_action_blocked"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_active_reclaim_intervention_without_host_cancellation() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("intervention-active-reclaim");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "active-reclaim");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-active-reclaim",
        "status": "running",
        "summary": "Captain selected reclaim while the host subagent may still be running.",
        "evidence_paths": ["docs/project-plan.md:129"],
        "next_action": "captain_replan",
        "open_questions": ["Host cancellation is unsupported; wait for stale output or replan."],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "Scope changed while the host subagent was active and direct cancellation is unsupported.",
        "chosen_next_action": "reclaim",
        "stale_output_policy": "merge_explicit_only",
        "stale_output_summary": "Any later worker output must remain visible and require explicit captain merge."
    }));

    assert_eq!(update_payload["reported_subagent_status"], "running");
    assert_eq!(update_payload["subagent_status"], "reclaimed");
    assert_eq!(update_payload["active_reclaim_intervention"], true);
    assert_eq!(
        update_payload["captain_intervention"]["chosen_next_action"],
        "reclaim"
    );
    assert_eq!(
        update_payload["captain_intervention"]["host_worker_may_still_be_running"],
        true
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"],
        Value::Null
    );
    assert_eq!(update_payload["active_handle_cleanup"]["state"], "released");

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.clone(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["next_step"], "await_fan_in");
    assert_eq!(
        status_payload["host_subagent_state"]["active_subagent_count"],
        0
    );
    assert_eq!(status_payload["host_subagent_state"]["reclaimed_count"], 1);
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]
            ["cancellation_supported"],
        false
    );
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]
            ["recommended_action"],
        "none"
    );
    assert_eq!(
        status_payload["latest_captain_intervention"]["stale_output_policy"],
        "merge_explicit_only"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["status"],
        "reclaimed"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["reported_status"],
        "running"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]
            ["host_worker_may_still_be_running"],
        true
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(run_record["active_agent_id"], "captain");
    assert_eq!(run_record["active_thread_id"], Value::Null);
    assert_eq!(run_record["child_agents"][0]["status"], "reclaimed");
    assert_eq!(
        run_record["host_subagent_handle_archive"][0]["thread_id"],
        "thread-active-reclaim"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_preserves_late_completed_output_after_active_reclaim() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("intervention-active-reclaim-late");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) =
        start_intervention_test_run(&workspace_dir, "active-reclaim-late");

    parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-active-reclaim-late",
        "status": "running",
        "summary": "Captain reclaimed the active lane while host cancellation is unsupported.",
        "evidence_paths": ["docs/project-plan.md:129"],
        "next_action": "captain_replan",
        "open_questions": ["Late output needs explicit merge."],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "The active worker is stale relative to the current captain direction.",
        "chosen_next_action": "reclaim",
        "stale_output_policy": "merge_explicit_only",
        "stale_output_summary": "Any later worker output is stale and requires explicit captain merge."
    }));
    let archive_len_after_reclaim = read_json_document(&run_directory.join("run.json"))
        .expect("run record")["host_subagent_handle_archive"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);

    let late_update = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-active-reclaim-late",
        "status": "completed",
        "summary": "Late worker returned output after the active reclaim.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/lib.rs:77"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "medium"
    }));

    assert_eq!(late_update["reported_subagent_status"], "completed");
    assert_eq!(late_update["subagent_status"], "reclaimed");
    assert_eq!(late_update["stale_output_after_reclaim"], true);
    assert_eq!(
        late_update["active_handle_cleanup"]["state"],
        "already_clear"
    );
    assert_eq!(
        late_update["active_handle_cleanup"]["host_close_required"],
        false
    );
    assert_eq!(
        late_update["active_handle_cleanup"]["host_close_status"],
        "not_required"
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(task_card["subagent_lifecycle"]["status"], "reclaimed");
    assert_eq!(
        task_card["subagent_lifecycle"]["summary"],
        "Captain reclaimed the active lane while host cancellation is unsupported."
    );
    assert_eq!(
        task_card["subagent_lifecycle"]["host_worker_may_still_be_running"],
        true
    );
    assert_eq!(task_card["subagent_fan_in"]["status"], "reclaimed");
    assert_eq!(task_card["late_subagent_output"]["status"], "completed");
    assert_eq!(
        task_card["late_subagent_output"]["summary"],
        "Late worker returned output after the active reclaim."
    );
    assert_eq!(
        task_card["late_subagent_output"]["authority"],
        "captain_explicit_merge_required"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(run_record["child_agents"][0]["status"], "reclaimed");
    assert_eq!(
        run_record["child_agents"][0]["summary"],
        "Captain reclaimed the active lane while host cancellation is unsupported."
    );
    assert_eq!(
        run_record["host_subagent_handle_archive"]
            .as_array()
            .map(Vec::len),
        Some(archive_len_after_reclaim)
    );
    assert_eq!(
        run_record["latest_stale_subagent_output"]["status"],
        "completed"
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id,
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(
        status_payload["host_subagent_state"]["active_subagent_count"],
        0
    );
    assert_eq!(status_payload["host_subagent_state"]["reclaimed_count"], 1);
    assert_eq!(
        status_payload["current_task_card"]["late_subagent_output"]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["state"],
        "already_clear"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_required"],
        true
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_status"],
        "host_action_required"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_action"],
        "close_agent"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_target"],
        "ccc_raider"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_intervention_reclaim_with_stale_output_preservation() {
    let workspace_dir = create_temp_path("intervention-reclaim-stale");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "reclaim");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-reclaim",
        "status": "reclaimed",
        "summary": "Captain reclaimed the lane after scope materially changed.",
        "fan_in_status": "reclaimed",
        "evidence_paths": ["src/lib.rs:22"],
        "next_action": "captain_replan",
        "open_questions": ["Late output should remain visible but not merge automatically."],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "The output is stale after the captain changed direction.",
        "chosen_next_action": "reclaim",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        },
        "stale_output_policy": "preserve_visible",
        "stale_output_summary": "Late worker output is preserved for history and must not overwrite the reclaimed path."
    }));

    assert_eq!(
        update_payload["captain_intervention"]["stale_output_policy"],
        "preserve_visible"
    );
    assert_eq!(
        update_payload["captain_intervention"]["stale_output_summary"],
        "Late worker output is preserved for history and must not overwrite the reclaimed path."
    );
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(
        run_record["latest_captain_intervention"]["chosen_next_action"],
        "reclaim"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_preserves_reclaimed_path_when_late_stale_output_arrives() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("intervention-late-stale-output");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "late-stale");

    parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-late-stale",
        "status": "reclaimed",
        "summary": "Captain reclaimed the stale lane after changing direction.",
        "fan_in_status": "reclaimed",
        "evidence_paths": ["docs/project-plan.md:129"],
        "next_action": "captain_replan",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "The original lane is stale after re-scope.",
        "chosen_next_action": "reclaim",
        "stale_output_policy": "preserve_visible",
        "stale_output_summary": "Late output must remain visible but not overwrite the reclaimed path."
    }));
    let archive_len_after_reclaim = read_json_document(&run_directory.join("run.json"))
        .expect("run record")["host_subagent_handle_archive"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);

    let late_update = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-late-stale",
        "status": "completed",
        "summary": "Late worker output arrived after reclaim.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/lib.rs:42"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "medium"
    }));

    assert_eq!(late_update["reported_subagent_status"], "completed");
    assert_eq!(late_update["subagent_status"], "reclaimed");
    assert_eq!(late_update["stale_output_after_reclaim"], true);
    assert_eq!(
        late_update["active_handle_cleanup"]["state"],
        "already_clear"
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(task_card["subagent_lifecycle"]["status"], "reclaimed");
    assert_eq!(
        task_card["subagent_lifecycle"]["summary"],
        "Captain reclaimed the stale lane after changing direction."
    );
    assert_eq!(task_card["subagent_fan_in"]["status"], "reclaimed");
    assert_eq!(task_card["late_subagent_output"]["status"], "completed");
    assert_eq!(
        task_card["late_subagent_output"]["summary"],
        "Late worker output arrived after reclaim."
    );
    assert_eq!(
        task_card["late_subagent_output"]["authority"],
        "captain_explicit_merge_required"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(run_record["child_agents"][0]["status"], "reclaimed");
    assert_eq!(
        run_record["host_subagent_handle_archive"]
            .as_array()
            .map(Vec::len),
        Some(archive_len_after_reclaim)
    );
    assert_eq!(
        run_record["latest_stale_subagent_output"]["status"],
        "completed"
    );
    assert_eq!(
        run_record["latest_entry_trace"]["stale_output_after_reclaim"],
        true
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id,
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["host_subagent_state"]["reclaimed_count"], 1);
    assert_eq!(
        status_payload["current_task_card"]["late_subagent_output"]["status"],
        "completed"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_marks_intervention_budget_exhausted_next_action_blocked() {
    let workspace_dir = create_temp_path("intervention-budget-blocked");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, _run_directory) = start_intervention_test_run(&workspace_dir, "budget");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-budget",
        "status": "completed",
        "summary": "Raider still missed the same invariant after the allowed repair.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:10000"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "Same-worker repair was selected but the retry budget is exhausted.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));

    assert_eq!(
        update_payload["captain_intervention"]["next_action_blocked"],
        true
    );
    assert_eq!(
        update_payload["captain_intervention"]["next_action_block_reason"],
        "retry_budget_exhausted"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"],
        Value::Null
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_blocks_reassign_without_target() {
    let workspace_dir = create_temp_path("intervention-reassign-missing-target");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, _run_directory) =
        start_intervention_test_run(&workspace_dir, "reassign-missing-target");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-reassign-missing-target",
        "status": "failed",
        "summary": "Raider pursued the wrong ownership boundary.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["docs/project-plan.md:180"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "Captain selected reassignment but did not provide a target.",
        "chosen_next_action": "reassign",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));

    assert_eq!(
        update_payload["captain_intervention"]["next_action_blocked"],
        true
    );
    assert_eq!(
        update_payload["captain_intervention"]["next_action_block_reason"],
        "reassign_target_missing"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"],
        Value::Null
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_blocks_missing_budget_snapshot_for_bounded_follow_up() {
    let retry_workspace_dir = create_temp_path("intervention-retry-missing-budget");
    create_dir_all(&retry_workspace_dir).expect("create retry workspace");
    let (retry_run_id, _retry_run_directory) =
        start_intervention_test_run(&retry_workspace_dir, "retry-missing-budget");

    let retry_update = parse_and_record_subagent_update(json!({
        "run_id": retry_run_id,
        "cwd": retry_workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-retry-missing-budget",
        "status": "completed",
        "summary": "Raider missed one bounded assertion.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "Captain selected a retry without budget data.",
        "chosen_next_action": "amend_same_worker"
    }));
    assert_eq!(
        retry_update["captain_intervention"]["next_action_blocked"],
        true
    );
    assert_eq!(
        retry_update["captain_intervention"]["next_action_block_reason"],
        "retry_budget_unavailable"
    );
    assert_eq!(
        retry_update["captain_intervention"]["pending_follow_up"],
        Value::Null
    );

    let reassign_workspace_dir = create_temp_path("intervention-reassign-missing-budget");
    create_dir_all(&reassign_workspace_dir).expect("create reassign workspace");
    let (reassign_run_id, _reassign_run_directory) =
        start_intervention_test_run(&reassign_workspace_dir, "reassign-missing-budget");

    let reassign_update = parse_and_record_subagent_update(json!({
        "run_id": reassign_run_id,
        "cwd": reassign_workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-reassign-missing-budget",
        "status": "failed",
        "summary": "Raider pursued the wrong ownership boundary.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["docs/project-plan.md:180"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "Captain selected reassignment without budget data.",
        "chosen_next_action": "reassign",
        "reassign_target": {
            "assigned_role": "explorer",
            "assigned_agent_id": "scout",
            "scope": "Inspect ownership boundary before another mutation.",
            "prompt": "Inspect the ownership boundary and report the smallest safe reassignment plan."
        }
    }));
    assert_eq!(
        reassign_update["captain_intervention"]["next_action_blocked"],
        true
    );
    assert_eq!(
        reassign_update["captain_intervention"]["next_action_block_reason"],
        "reassign_budget_unavailable"
    );
    assert_eq!(
        reassign_update["captain_intervention"]["pending_follow_up"],
        Value::Null
    );

    let _ = fs::remove_dir_all(&retry_workspace_dir);
    let _ = fs::remove_dir_all(&reassign_workspace_dir);
}

#[test]
fn ccc_subagent_update_blocks_malformed_nested_budget_for_bounded_follow_up() {
    let retry_workspace_dir = create_temp_path("intervention-retry-malformed-budget");
    create_dir_all(&retry_workspace_dir).expect("create retry workspace");
    let (retry_run_id, _retry_run_directory) =
        start_intervention_test_run(&retry_workspace_dir, "retry-malformed-budget");

    let retry_update = parse_and_record_subagent_update(json!({
        "run_id": retry_run_id,
        "cwd": retry_workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-retry-malformed-budget",
        "status": "completed",
        "summary": "Raider missed one bounded assertion.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "Captain selected a retry with malformed budget data.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));
    assert_eq!(
        retry_update["captain_intervention"]["next_action_blocked"],
        true
    );
    assert_eq!(
        retry_update["captain_intervention"]["next_action_block_reason"],
        "retry_budget_unavailable"
    );
    assert_eq!(
        retry_update["captain_intervention"]["pending_follow_up"],
        Value::Null
    );

    let reassign_workspace_dir = create_temp_path("intervention-reassign-malformed-budget");
    create_dir_all(&reassign_workspace_dir).expect("create reassign workspace");
    let (reassign_run_id, _reassign_run_directory) =
        start_intervention_test_run(&reassign_workspace_dir, "reassign-malformed-budget");

    let reassign_update = parse_and_record_subagent_update(json!({
        "run_id": reassign_run_id,
        "cwd": reassign_workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-reassign-malformed-budget",
        "status": "failed",
        "summary": "Raider pursued the wrong ownership boundary.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["docs/project-plan.md:180"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "Captain selected reassignment with malformed budget data.",
        "chosen_next_action": "reassign",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "remaining": 1 }
        },
        "reassign_target": {
            "assigned_role": "explorer",
            "assigned_agent_id": "scout",
            "scope": "Inspect ownership boundary before another mutation.",
            "prompt": "Inspect the ownership boundary and report the smallest safe reassignment plan."
        }
    }));
    assert_eq!(
        reassign_update["captain_intervention"]["next_action_blocked"],
        true
    );
    assert_eq!(
        reassign_update["captain_intervention"]["next_action_block_reason"],
        "reassign_budget_unavailable"
    );
    assert_eq!(
        reassign_update["captain_intervention"]["pending_follow_up"],
        Value::Null
    );

    let _ = fs::remove_dir_all(&retry_workspace_dir);
    let _ = fs::remove_dir_all(&reassign_workspace_dir);
}

#[test]
fn ccc_subagent_update_repeated_terminal_intervention_does_not_duplicate_follow_up() {
    let workspace_dir = create_temp_path("intervention-duplicate-follow-up");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "duplicate");
    let payload = json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-duplicate",
        "status": "completed",
        "summary": "Raider returned the same incomplete repair payload twice.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "The same specialist should make one narrowed amendment.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    });

    let first_update = parse_and_record_subagent_update(payload.clone());
    let second_update = parse_and_record_subagent_update(payload);
    assert_eq!(
        first_update["captain_intervention"]["pending_follow_up"]["dedupe_key"],
        second_update["captain_intervention"]["pending_follow_up"]["dedupe_key"]
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(
        task_card["captain_intervention_history"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        task_card["captain_intervention"]["pending_follow_up"]["action"],
        "amend_same_worker"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_non_intervention_regression_keeps_artifact_null() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("intervention-regression-null");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "regression");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-intervention-regression",
        "status": "completed",
        "summary": "Raider completed the bounded task.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:1"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }));

    assert_eq!(update_payload["captain_intervention"], Value::Null);
    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id,
            run_directory,
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["latest_captain_intervention"], Value::Null);
    assert_eq!(
        status_payload["current_task_card"]["captain_intervention"],
        Value::Null
    );
    assert!(!create_ccc_status_text(&status_payload).contains("Intervention:"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_satisfactory_completed_path_has_no_intervention_follow_up() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("satisfactory-no-intervention");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "satisfactory");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-satisfactory",
        "status": "completed",
        "summary": "Raider completed the bounded task satisfactorily.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:1"],
        "next_action": "captain_accept",
        "open_questions": [],
        "confidence": "high"
    }));

    assert_eq!(update_payload["subagent_status"], "completed");
    assert_eq!(update_payload["captain_intervention"], Value::Null);
    assert_eq!(update_payload["active_handle_cleanup"]["state"], "released");

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id,
            run_directory,
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["next_step"], "await_fan_in");
    assert_eq!(status_payload["can_advance"], true);
    assert_eq!(status_payload["pending_captain_follow_up"], Value::Null);
    assert_eq!(status_payload["latest_captain_intervention"], Value::Null);
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["pending_merge_count"],
        1
    );
    assert!(!create_ccc_status_text(&status_payload).contains("Intervention:"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_tool_accepts_run_ref() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-run-ref");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-456");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "tools/call",
            "params": {
                "name": "ccc_status",
                "arguments": {
                    "run_ref": format!("{CCC_RUN_REF_PREFIX}{}", run_directory.display())
                }
            }
        }),
    )
    .expect("response");

    let normalized_run_directory = normalize_path(&run_directory);
    assert_eq!(
        response["result"]["structuredContent"]["status"]["run_ref"],
        format!("{CCC_RUN_REF_PREFIX}{}", normalized_run_directory.display())
    );
    assert_eq!(
        response["result"]["structuredContent"]["status"]["run_directory"],
        normalized_run_directory.to_string_lossy().to_string()
    );
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_activity_tool_reads_latest_attempt_and_active_delegation_summary() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("activity");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-789");
    create_dir_all(run_directory.join("orchestration").join("attempts")).expect("create attempts");
    create_dir_all(run_directory.join("delegations")).expect("create delegations");
    write(
        run_directory
            .join("orchestration")
            .join("attempts")
            .join("attempt-0001.json"),
        serde_json::to_vec_pretty(&json!({
            "attempt_id": "attempt-0001",
            "entrypoint": "ccc_orchestrate",
            "started_at": "2026-04-22T08:02:00.000Z",
            "completed_at": null,
            "stop": { "reason": "await_fan_in" },
            "steps": [
                {
                    "step_number": 1,
                    "command": "execute_task",
                    "before": { "status": "active", "stage": "execution" },
                    "after": { "status": "active", "stage": "execution" }
                }
            ]
        }))
        .expect("serialize attempt"),
    )
    .expect("write attempt");
    write(
        run_directory
            .join("delegations")
            .join("delegation-0001.json"),
        serde_json::to_vec_pretty(&json!({
            "delegation_id": "delegation-0001",
            "task_card_id": "task-1",
            "delegated_by_role": "orchestrator",
            "summary": "scout is gathering evidence",
            "child_agent": {
                "status": "running",
                "role": "explorer",
                "agent_id": "scout"
            },
            "executor": {
                "status": "running"
            },
            "worker_lifecycle": {
                "state": "running",
                "reclaim_state": "not_needed",
                "queued_at": "2026-04-22T08:01:00.000Z",
                "launch_requested_at": "2026-04-22T08:01:10.000Z",
                "started_at": "2026-04-22T08:01:20.000Z",
                "process_id": 12345,
                "process_started_at": "2026-04-22T08:01:20.000Z",
                "process_last_seen_at": "2026-04-22T08:01:30.000Z",
                "last_progress_at": "2026-04-22T08:01:30.000Z",
                "returned_at": null,
                "stale_at": null,
                "timed_out_at": null,
                "stale_after_ms": 45000,
                "timeout_after_ms": 45000,
                "summary": "running"
            },
            "updated_at": "2026-04-22T08:03:00.000Z"
        }))
        .expect("serialize delegation"),
    )
    .expect("write delegation");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "tools/call",
            "params": {
                "name": "ccc_activity",
                "arguments": {
                    "run_id": "run-789",
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("response");

    assert_eq!(
        response["result"]["structuredContent"]["activity"]["latest_orchestration_attempt"]
            ["attempt_id"],
        "attempt-0001"
    );
    assert_eq!(
        response["result"]["structuredContent"]["activity"]["active_task_delegations"]["running"],
        1
    );
    assert_eq!(
        response["result"]["structuredContent"]["activity"]["active_task_delegations"]["active"][0]
            ["worker_lifecycle"]["state"],
        "timed_out"
    );
    assert_eq!(
        response["result"]["structuredContent"]["activity"]["worker_visibility"]
            ["timed_out_worker_count"],
        1
    );
    assert_eq!(
        response["result"]["structuredContent"]["activity"]["reclaim_plan"]
            ["reclaim_needed_worker_count"],
        1
    );
    assert_eq!(
        response["result"]["structuredContent"]["activity"]["reclaim_plan"]["targets"][0]
            ["delegation_id"],
        "delegation-0001"
    );
    assert!(
        response["result"]["structuredContent"]["activity"]["checkpoint_summary"]
            .as_str()
            .unwrap_or_default()
            .contains("Captain checkpoint")
    );
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_refreshes_running_delegation_progress_from_raw_events_file() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("heartbeat");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-heartbeat");
    create_dir_all(run_directory.join("delegations")).expect("create delegations");
    create_dir_all(run_directory.join("raw-events")).expect("create raw-events");
    let raw_events_file = run_directory
        .join("raw-events")
        .join("delegation-heartbeat.jsonl");
    write(
        &raw_events_file,
        "{\"type\":\"message\",\"thread_id\":\"thread-heartbeat\"}\n",
    )
    .expect("write raw events");
    write(
        run_directory
            .join("delegations")
            .join("delegation-heartbeat.json"),
        serde_json::to_vec_pretty(&json!({
            "delegation_id": "delegation-heartbeat",
            "task_card_id": "task-1",
            "delegated_by_role": "orchestrator",
            "summary": "worker is still running",
            "child_agent": {
                "status": "running",
                "role": "code specialist",
                "agent_id": "raider"
            },
            "executor": {
                "status": "running"
            },
            "worker_launch_evidence": {
                "raw_events_file": raw_events_file.to_string_lossy()
            },
            "worker_lifecycle": {
                "state": "running",
                "reclaim_state": "not_needed",
                "queued_at": "2026-04-22T08:00:00.000Z",
                "launch_requested_at": "2026-04-22T08:00:01.000Z",
                "started_at": "2026-04-22T08:00:02.000Z",
                "process_id": 12345,
                "process_started_at": "2026-04-22T08:00:02.000Z",
                "process_last_seen_at": "2026-04-22T08:00:03.000Z",
                "last_progress_at": "2026-04-22T08:00:03.000Z",
                "returned_at": null,
                "stale_at": null,
                "timed_out_at": null,
                "stale_after_ms": 45000,
                "timeout_after_ms": 45000,
                "summary": "running"
            },
            "updated_at": "2026-04-22T08:00:03.000Z"
        }))
        .expect("serialize delegation"),
    )
    .expect("write delegation");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "tools/call",
            "params": {
                "name": "ccc_status",
                "arguments": {
                    "run_id": "run-heartbeat",
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("response");

    let refreshed_progress = response["result"]["structuredContent"]["status"]["worker_visibility"]
        ["workers"][0]["worker_lifecycle"]["last_progress_at"]
        .as_str()
        .expect("refreshed progress");
    assert_ne!(refreshed_progress, "2026-04-22T08:00:03.000Z");

    let persisted = read_json_document(
        &run_directory
            .join("delegations")
            .join("delegation-heartbeat.json"),
    )
    .expect("persisted delegation");
    assert_eq!(
        persisted["worker_lifecycle"]["last_progress_at"].as_str(),
        Some(refreshed_progress)
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_creates_run_and_round_trips_into_status() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start");
    create_dir_all(&workspace_dir).expect("create workspace");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Bootstrap a Rust-started run",
                    "title": "Implement the next bounded step",
                    "intent": "Create a run without invoking Codex",
                    "scope": "Single execution task only",
                    "acceptance": "Persist run bootstrap artifacts",
                    "prompt": "Implement the first bounded task",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );

    let run_id = response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let run_directory = response["result"]["structuredContent"]["run_directory"]
        .as_str()
        .expect("run directory");
    assert_eq!(response["result"]["structuredContent"]["status"], "active");
    assert_eq!(
        response["result"]["structuredContent"]["stage"],
        "execution"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    assert_eq!(response["result"]["structuredContent"]["can_advance"], true);
    assert_eq!(
        response["result"]["structuredContent"]["allowed_next_commands"],
        json!(["advance"])
    );
    assert!(PathBuf::from(run_directory).join("run.json").exists());
    assert!(PathBuf::from(run_directory).join("task-cards").exists());

    let status_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "tools/call",
            "params": {
                "name": "ccc_status",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("status response");

    assert_eq!(
        status_response["result"]["structuredContent"]["status"]["run_id"],
        run_id
    );
    assert_eq!(
        status_response["result"]["structuredContent"]["status"]["current_task_card"]["title"],
        "Implement the next bounded step"
    );
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_plan_sequence_stops_at_pending_longway_approval() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start-plan-sequence");
    create_dir_all(&workspace_dir).expect("create workspace");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 151,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Plan a broad runtime migration",
                    "title": "Plan the runtime migration",
                    "intent": "Create only a pending LongWay approval state",
                    "scope": "Planning only; no execution dispatch is allowed",
                    "acceptance": "Status stops at pending LongWay approval",
                    "prompt": "Plan the migration and wait for operator approval",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );

    let structured = &response["result"]["structuredContent"];
    let run_id = structured["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(structured["run_directory"].as_str().expect("run directory"));
    assert_eq!(structured["stage"], "planning");
    assert_eq!(structured["sequence"], "PLAN_SEQUENCE");
    assert_eq!(structured["approval_state"], "pending_longway_approval");
    assert_eq!(structured["next_step"], "await_longway_approval");
    assert_eq!(structured["can_advance"], false);
    assert_eq!(
        structured["allowed_next_commands"],
        json!(["approve_longway"])
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(run_record["stage"], "planning");
    assert_eq!(run_record["sequence"], "PLAN_SEQUENCE");
    assert_eq!(run_record["approval_state"], "pending_longway_approval");

    let run_state =
        read_json_document(&run_directory.join("run-state.json")).expect("run-state payload");
    assert_eq!(run_state["sequence"], "PLAN_SEQUENCE");
    assert_eq!(run_state["approval_state"], "pending_longway_approval");
    assert_eq!(
        run_state["next_action"]["command"],
        "await_longway_approval"
    );

    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["lifecycle_state"], "pending_approval");
    assert_eq!(longway["sequence"], "PLAN_SEQUENCE");
    assert_eq!(longway["approval_state"], "pending_longway_approval");
    assert_eq!(longway["active_phase_status"], "pending_longway_approval");

    let task_card_id = structured["task_card_id"].as_str().expect("task card id");
    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");
    assert_eq!(task_card["sequence"], "PLAN_SEQUENCE");
    assert_eq!(task_card["status"], "pending_longway_approval");
    assert_eq!(task_card["node_kind"], "planning");
    assert_eq!(task_card["assigned_role"], "way");
    assert_eq!(task_card["assigned_agent_id"], "tactician");
    assert_eq!(
        task_card["role_config_snapshot"]["model"],
        expected_role_config_field("way", "model")
    );
    assert_eq!(
        task_card["role_config_snapshot"]["variant"],
        expected_role_config_field("way", "variant")
    );
    assert_eq!(
        task_card["delegation_plan"]["runtime_dispatch"]["model"],
        expected_role_config_field("way", "model")
    );
    assert_eq!(
        task_card["delegation_plan"]["runtime_dispatch"]["variant"],
        expected_role_config_field("way", "variant")
    );
    assert_eq!(task_card["dispatch_allowed"], false);

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["stage"], "planning");
    assert_eq!(status_payload["sequence"], "PLAN_SEQUENCE");
    assert_eq!(status_payload["approval_state"], "pending_longway_approval");
    assert_eq!(status_payload["next_step"], "await_longway_approval");
    assert_eq!(status_payload["can_advance"], false);
    assert_eq!(
        status_payload["captain_action_contract"]["allowed_action"],
        "blocked"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["required_action"],
        "await_longway_approval"
    );
    assert_eq!(
        status_payload["current_task_card"]["assignment_quality"]["state"],
        "matched"
    );
    assert_eq!(
        status_payload["current_task_card"]["assignment_quality"]["phase"],
        "planning"
    );
    assert_eq!(
        status_payload["current_task_card"]["assignment_quality"]["drift_severity"],
        "info"
    );
    assert_eq!(
        status_payload["context_health"]["active_conflict_state"]["assignment_drift"],
        false
    );
    assert_eq!(
        status_payload["context_health"]["active_conflict_state"]["assignment_drift_severity"],
        "info"
    );
    assert_eq!(
        status_payload["context_health"]["active_conflict_state"]["blocked"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_plan_sequence_records_bounded_way_planning_context() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start-plan-context");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("planner.rs"),
        "pub fn plan_context() {}\n",
    )
    .expect("write planner");
    code_graph::update_code_graph_store_for_repo(&workspace_dir).expect("index graph");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 154,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Plan with graph evidence",
                    "title": "Plan graph-informed LongWay",
                    "intent": "Create a pending LongWay with bounded context",
                    "scope": "Planning only; consume graph and memory status",
                    "acceptance": "LongWay records bounded planning context",
                    "prompt": "Plan a graph-informed implementation",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["schema"], "ccc.way_planning_context.v1");
    assert_eq!(context["source"], "PLAN_SEQUENCE");
    assert_eq!(context["workspace_root"]["root_kind"], "git_repo");
    assert_eq!(context["workspace_root"]["confidence"], "high");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(context["graph"]["available"], true);
    assert_eq!(
        context["graph"]["repo_root"],
        fs::canonicalize(&workspace_dir)
            .expect("canonical workspace")
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string()
    );
    assert_eq!(context["graph"]["file_count"], 1);
    assert!(context["graph"]["evidence_note"]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("availability=store_loaded"));
    assert!(context["graph"].get("query_result").is_none());
    assert_eq!(context["memory"]["entry_count"], 0);
    assert_eq!(
        context["way_structural_context"]["schema"],
        "ccc.way_structural_context.v1"
    );
    assert_eq!(
        context["way_structural_context"]["source"],
        "skill_registry"
    );
    assert_eq!(
        context["way_structural_context"]["agent_name"],
        "ccc_tactician"
    );
    assert_eq!(
        context["way_structural_context"]["scenes"][0]["id"],
        "frame_goal"
    );
    assert_eq!(context["evidence_policy"]["bounded"], true);
    assert_eq!(context["evidence_policy"]["raw_graph_dump_stored"], false);
    assert_eq!(context["evidence_policy"]["memory_as_run_truth"], false);
    assert_eq!(context["planned_row_count"], 2);
    assert_eq!(context["decomposition_source"], "bounded_planning_context");
    assert!(context["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["role"] == "way"));
    assert!(context["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["role"] == "code specialist"));
    let planned_rows = longway["planned_rows"].as_array().expect("planned rows");
    assert_eq!(planned_rows.len(), 2);
    assert_eq!(planned_rows[0]["planned_role"], "exploration_specialist");
    assert_eq!(planned_rows[0]["planned_agent_id"], "scout-a");
    assert!(planned_rows[0]["scope"]
        .as_str()
        .unwrap_or_default()
        .contains("availability=store_loaded"));
    assert_eq!(
        planned_rows[0]["routing_trace"]["reason"],
        "PLAN_SEQUENCE generated this row from bounded graph, memory, and capability context."
    );
    assert_eq!(
        planned_rows[0]["routing_trace"]["structural_scene_id"],
        "frame_goal"
    );
    assert!(planned_rows[0]["routing_trace"]
        .get("query_result")
        .is_none());
    assert_eq!(planned_rows[1]["planned_role"], "code specialist");
    assert!(planned_rows[1]["routing_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("bounded capability context"));
    assert_eq!(
        planned_rows[1]["routing_trace"]["structural_scene_id"],
        "sequence_rows"
    );
    assert_eq!(
        planned_rows[1]["routing_trace"]["approval_scene_id"],
        "surface_approval"
    );

    let run_id = response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(
        status_payload["longway"]["planning_context"]["schema"],
        "ccc.way_planning_context.v1"
    );
    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["longway"]["planning_context"]["evidence_policy"]["raw_graph_dump_stored"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_plan_sequence_persists_way_clarification_request_for_broad_work() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start-plan-way-clarification");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 155,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Plan broad repo-wide work",
                    "title": "Plan broad 0.0.13 work",
                    "intent": "Create a pending LongWay only after clarification",
                    "scope": "Plan the next repo-wide strategy across multiple remaining tasks",
                    "acceptance": "CCC asks bounded Way clarification questions first",
                    "prompt": "Plan the next repo-wide strategy across multiple remaining tasks",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let structured = &response["result"]["structuredContent"];
    assert_eq!(structured["next_step"], "await_operator");
    assert_eq!(structured["approval_state"], "pending_way_clarification");
    assert_eq!(structured["can_advance"], false);
    assert_eq!(
        structured["allowed_next_commands"][0],
        "answer_way_clarification"
    );
    assert_eq!(
        structured["way_clarification_request"]["schema"],
        "ccc.way_clarification_request.v1"
    );
    assert_eq!(
        structured["way_clarification_request"]["questions"]
            .as_array()
            .expect("questions")
            .len(),
        3
    );

    let run_directory = PathBuf::from(structured["run_directory"].as_str().expect("run directory"));
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(run_record["approval_state"], "pending_way_clarification");
    assert_eq!(
        run_record["way_clarification_request"]["state"],
        "awaiting_operator"
    );
    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run state");
    assert_eq!(run_state["next_action"]["command"], "await_operator");
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["lifecycle_state"], "awaiting_clarification");
    assert_eq!(longway["active_phase_status"], "awaiting_way_clarification");
    assert!(longway.get("planned_rows").is_none());
    assert_eq!(
        longway["way_clarification_request"]["copyable_follow_up"],
        "$cap Answer the pending Way clarification for this run: primary_outcome=...; scope_boundary=...; risk_gate=..."
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: structured["run_id"].as_str().expect("run id").to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["next_step"], "await_operator");
    assert_eq!(
        status_payload["way_clarification_request"]["questions"][0]["id"],
        "primary_outcome"
    );
    assert_eq!(
        status_payload["longway"]["way_clarification_request"]["state"],
        "awaiting_operator"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_persists_internal_prompt_refinement_as_disabled_and_status_surfaces_it() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start-prompt-refinement-disabled");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 156,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Refine a prompt safely",
                    "title": "Prompt refinement plumbing",
                    "intent": "Persist an internal prompt refinement record without changing dispatch",
                    "scope": "Single internal plumbing slice",
                    "acceptance": "CCC records a bounded prompt refinement state",
                    "prompt": "Persist internal prompt refinement state only.",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let structured = &response["result"]["structuredContent"];
    assert!(structured.get("prompt_refinement").is_none());

    let run_directory = PathBuf::from(structured["run_directory"].as_str().expect("run directory"));
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert!(run_record
        .get("prompt_refinement_handoff_decision")
        .is_none());
    assert_eq!(
        run_record["prompt_refinement"]["schema"],
        "ccc.prompt_refinement.v1"
    );
    assert_eq!(run_record["prompt_refinement"]["state"], "disabled");
    assert_eq!(run_record["prompt_refinement"]["enabled"], false);
    assert_eq!(
        run_record["prompt_refinement"]["execution_mode"],
        "internal"
    );
    assert_eq!(run_record["prompt_refinement"]["owner"], "captain");
    assert_eq!(
        run_record["prompt_refinement"]["captain_gate"],
        "accept_adjust_reject"
    );
    assert_eq!(
        run_record["prompt_refinement"]["longway_materialization_allowed"],
        false
    );
    assert_eq!(
        run_record["prompt_refinement"]["task_card_creation_allowed"],
        false
    );
    assert_eq!(run_record["prompt_refinement"]["source"], "ccc_promptsmith");
    assert_eq!(
        run_record["prompt_refinement"],
        json!({
            "schema": "ccc.prompt_refinement.v1",
            "state": "disabled",
            "enabled": false,
            "execution_mode": "internal",
            "owner": "captain",
            "captain_gate": "accept_adjust_reject",
            "longway_materialization_allowed": false,
            "task_card_creation_allowed": false,
            "source": "ccc_promptsmith",
            "task_card_id": structured["task_card_id"].as_str().expect("task card id"),
            "created_at": run_record["created_at"].clone(),
            "consumed_at": Value::Null,
            "recorded_at": run_record["updated_at"].clone(),
        })
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: structured["run_id"].as_str().expect("run id").to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["prompt_refinement"]["state"], "disabled");
    assert_eq!(status_payload["prompt_refinement"]["enabled"], false);
    assert_eq!(
        status_payload["prompt_refinement"]["captain_gate"],
        "accept_adjust_reject"
    );
    assert!(status_payload
        .get("prompt_refinement_handoff_decision")
        .is_none());
    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Prompt Refinement: state=disabled"));
    assert!(text.contains("owner=captain"));
    assert!(text.contains("gate=accept_adjust_reject"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_ignores_public_prompt_refinement_when_config_is_off() {
    let workspace_dir = create_temp_path("start-prompt-refinement-public-off");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let start_payload = crate::run_bootstrap::create_ccc_start_payload_with_config(
        &json!({
            "cwd": workspace_dir.to_string_lossy(),
            "goal": "Refine a prompt safely",
            "title": "Prompt refinement config gate",
            "intent": "Ignore public prompt refinement input when the config feature is off",
            "scope": "Single internal plumbing slice",
            "acceptance": "Prompt refinement remains disabled",
            "prompt": "Try to enable prompt refinement through parsed input only.",
            "task_kind": "execution",
            "sequence": "EXECUTE_SEQUENCE",
            "prompt_refinement": {
                "enabled": true
            }
        }),
        Some(&json!({
            "features": {
                "graph_context": false,
                "prompt_refinement": false
            }
        })),
    )
    .expect("start payload");

    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    assert!(start_payload
        .get("prompt_refinement_handoff_decision")
        .is_none());
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(run_record["prompt_refinement"]["state"], "disabled");
    assert_eq!(run_record["prompt_refinement"]["enabled"], false);
    assert!(run_record
        .get("prompt_refinement_handoff_decision")
        .is_none());
    assert_eq!(
        run_record["prompt_refinement"]["longway_materialization_allowed"],
        false
    );
    assert_eq!(
        run_record["prompt_refinement"]["task_card_creation_allowed"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_omits_goal_bridge_when_config_feature_is_off() {
    let workspace_dir = create_temp_path("start-goal-bridge-config-off");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let start_payload = crate::run_bootstrap::create_ccc_start_payload_with_config(
        &json!({
            "cwd": workspace_dir.to_string_lossy(),
            "goal": "Bridge goals safely",
            "title": "Goal Bridge config gate",
            "intent": "Ignore public Goal Bridge input when the config feature is off",
            "scope": "Single internal plumbing slice",
            "acceptance": "Goal Bridge remains omitted",
            "prompt": "Try to enable Goal Bridge through parsed input only.",
            "task_kind": "execution",
            "sequence": "EXECUTE_SEQUENCE",
            "goal_bridge": {
                "enabled": true
            }
        }),
        Some(&json!({
            "features": {
                "goals": false
            },
            "goal_bridge": {
                "enabled": true
            }
        })),
    )
    .expect("start payload");

    assert!(start_payload.get("goal_bridge").is_none());
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert!(run_record.get("goal_bridge").is_none());

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_config_enabled_goal_bridge_persists_internal_non_executing_record() {
    let workspace_dir = create_temp_path("start-goal-bridge-config-on");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let start_payload = crate::run_bootstrap::create_ccc_start_payload_with_config(
        &json!({
            "cwd": workspace_dir.to_string_lossy(),
            "goal": "Bridge goals safely",
            "title": "Goal Bridge config gate",
            "intent": "Persist a Goal Bridge record without dispatching goal runtime behavior",
            "scope": "Single internal plumbing slice",
            "acceptance": "Goal Bridge is Captain-owned and non-executing",
            "prompt": "Enable Goal Bridge through config only.",
            "task_kind": "execution",
            "sequence": "EXECUTE_SEQUENCE"
        }),
        Some(&json!({
            "features": {
                "goals": true
            },
            "goal_bridge": {
                "enabled": true,
                "brief_max_lines": 12,
                "specialists": {
                    "max_subgoal_lines": 8
                }
            }
        })),
    )
    .expect("start payload");

    assert!(start_payload.get("goal_bridge").is_none());
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(
        run_record["goal_bridge"],
        json!({
            "schema": "ccc.goal_bridge.v1",
            "state": "planned",
            "enabled": true,
            "visibility": "internal",
            "execution_mode": "internal_non_executing",
            "owner": "captain",
            "mode": "captain_owned",
            "task_card_id": start_payload["task_card_id"].as_str().expect("task card id"),
            "created_at": run_record["created_at"].clone(),
            "recorded_at": run_record["updated_at"].clone(),
            "brief_contract": {
                "language": "en",
                "max_lines": 12,
                "require_verifiable_stop": true
            },
            "truth_contract": {
                "host_goal_state_is_truth": false,
                "authoritative_state": [
                    "longway",
                    "task_cards",
                    "fan_in_records",
                    "review_decisions",
                    "fallback_records",
                    "verification_capsules"
                ]
            },
            "specialist_policy": {
                "allow_specialist_goal_context": true,
                "allow_specialist_set_goal": false,
                "allow_specialist_clear_goal": false,
                "allow_specialist_override_goal": false,
                "max_subgoal_lines": 8,
                "require_captain_acceptance": true
            },
            "public_api": {
                "public_command": false,
                "public_skill": false,
                "public_entrypoint": false,
                "set_goal_api_guaranteed": false
            }
        })
    );
    assert_eq!(run_record["child_agents"], json!([]));
    assert_eq!(run_record["specialist_executors"], json!([]));

    let task_card_count = fs::read_dir(run_directory.join("task-cards"))
        .expect("read task cards")
        .filter_map(Result::ok)
        .count();
    assert_eq!(task_card_count, 1);

    let run_schema: Value = serde_json::from_str(include_str!("../../../schemas/run.schema.json"))
        .expect("parse run schema");
    assert_eq!(
        run_schema["properties"]["goal_bridge"]["$ref"],
        "#/$defs/goalBridgeRecord"
    );
    assert_eq!(
        run_schema["$defs"]["goalBridgeRecord"]["properties"]["execution_mode"]["const"],
        "internal_non_executing"
    );
    assert_eq!(
        run_schema["$defs"]["goalBridgeRecord"]["properties"]["truth_contract"]["properties"]
            ["host_goal_state_is_truth"]["const"],
        false
    );
    assert_eq!(
        run_schema["$defs"]["goalBridgeRecord"]["properties"]["public_api"]["properties"]
            ["set_goal_api_guaranteed"]["const"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_config_enabled_prompt_refinement_is_planned_but_non_executing() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start-prompt-refinement-config-on");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let start_payload = crate::run_bootstrap::create_ccc_start_payload_with_config(
        &json!({
            "cwd": workspace_dir.to_string_lossy(),
            "goal": "Refine a prompt safely",
            "title": "Prompt refinement config gate",
            "intent": "Persist a planned prompt refinement record without dispatching work",
            "scope": "Single internal plumbing slice",
            "acceptance": "Prompt refinement is planned but non-executing",
            "prompt": "Enable prompt refinement through config only.",
            "task_kind": "execution",
            "sequence": "EXECUTE_SEQUENCE"
        }),
        Some(&json!({
            "features": {
                "graph_context": false,
                "prompt_refinement": true
            }
        })),
    )
    .expect("start payload");

    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    assert!(start_payload
        .get("prompt_refinement_handoff_decision")
        .is_none());
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(run_record["prompt_refinement"]["state"], "planned");
    assert_eq!(run_record["prompt_refinement"]["enabled"], true);
    assert_eq!(
        run_record["prompt_refinement"]["execution_mode"],
        "internal"
    );
    assert_eq!(run_record["prompt_refinement"]["owner"], "captain");
    assert_eq!(
        run_record["prompt_refinement"]["captain_gate"],
        "accept_adjust_reject"
    );
    assert_eq!(
        run_record["prompt_refinement"]["longway_materialization_allowed"],
        false
    );
    assert_eq!(
        run_record["prompt_refinement"]["task_card_creation_allowed"],
        false
    );
    assert_eq!(
        run_record["prompt_refinement_handoff_decision"],
        json!({
            "schema": "ccc.prompt_refinement_handoff_decision.v1",
            "state": "pending_captain_decision",
            "visibility": "internal",
            "execution_mode": "internal_non_executing",
            "owner": "captain",
            "source": "ccc_promptsmith",
            "task_card_id": start_payload["task_card_id"].as_str().expect("task card id"),
            "created_at": run_record["created_at"].clone(),
            "updated_at": run_record["updated_at"].clone(),
            "handoff": {
                "from": "ghost",
                "to": "captain",
                "stage": "pre_materialization"
            },
            "captain_gate": {
                "state": "pending",
                "allowed_decisions": ["accept", "adjust", "reject"],
                "decision": Value::Null,
                "decided_at": Value::Null
            },
            "brief_contract": {
                "language": "en",
                "refined_brief_persisted": false,
                "status_surface_allowed": false
            },
            "dispatch_allowed": false,
            "longway_materialization_allowed": false,
            "task_card_creation_allowed": false
        })
    );
    assert_eq!(run_record["child_agents"], json!([]));
    assert_eq!(run_record["specialist_executors"], json!([]));

    let task_card_count = fs::read_dir(run_directory.join("task-cards"))
        .expect("read task cards")
        .filter_map(Result::ok)
        .count();
    assert_eq!(task_card_count, 1);

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: start_payload["run_id"]
                .as_str()
                .expect("run id")
                .to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["prompt_refinement"]["state"], "planned");
    assert_eq!(status_payload["prompt_refinement"]["enabled"], true);
    assert!(status_payload
        .get("prompt_refinement_handoff_decision")
        .is_none());

    let structured =
        crate::mcp_tools::create_start_tool_structured_content(&start_payload, &status_payload);
    assert!(structured.get("prompt_refinement").is_none());
    assert!(structured
        .get("prompt_refinement_handoff_decision")
        .is_none());

    let run_schema: Value = serde_json::from_str(include_str!("../../../schemas/run.schema.json"))
        .expect("parse run schema");
    assert_eq!(
        run_schema["properties"]["prompt_refinement_handoff_decision"]["$ref"],
        "#/$defs/promptRefinementHandoffDecisionRecord"
    );
    assert_eq!(
        run_schema["$defs"]["promptRefinementHandoffDecisionRecord"]["properties"]["captain_gate"]
            ["properties"]["allowed_decisions"]["items"]["enum"],
        json!(["accept", "adjust", "reject"])
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_consumes_way_clarification_answer_once() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("orchestrate-way-clarification-answer");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 156,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Plan broad repo-wide work",
                    "title": "Plan broad 0.0.13 work",
                    "intent": "Create a pending LongWay only after clarification",
                    "scope": "Plan the next repo-wide strategy across multiple remaining tasks",
                    "acceptance": "CCC consumes the answer once",
                    "prompt": "Plan the next repo-wide strategy across multiple remaining tasks",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    assert!(
        start_response.get("error").is_none(),
        "unexpected ccc_start error response: {start_response:?}"
    );
    let start_structured = &start_response["result"]["structuredContent"];
    assert_eq!(start_structured["next_step"], "await_operator");
    let run_id = start_structured["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(
        start_structured["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    let answer =
        "primary_outcome=finish registry readiness first; scope_boundary=defer packaging polish; risk_gate=full cargo test must pass";
    let orchestrate_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 157,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "run_id": run_id,
                    "replan_prompt": answer
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        orchestrate_response.get("error").is_none(),
        "unexpected ccc_orchestrate error response: {orchestrate_response:?}"
    );
    let structured = &orchestrate_response["result"]["structuredContent"];
    assert_eq!(structured["next_step"], "await_longway_approval");
    assert_eq!(
        structured["way_clarification_consumption"]["state"],
        "consumed"
    );
    assert_eq!(structured["allowed_next_commands"][0], "approve_longway");
    assert_eq!(
        structured["scheduler_decision"]["post_fan_in_captain_decision"]["schema"],
        "ccc.post_fan_in_captain_decision.v1"
    );
    assert_eq!(
        structured["post_fan_in_captain_decision"]["schema"],
        "ccc.post_fan_in_captain_decision.v1"
    );
    assert_eq!(
        structured["scheduler_decision"]["post_fan_in_captain_decision"]["precedence"],
        "longway_approval"
    );
    assert_eq!(
        structured["post_fan_in_captain_decision"]["precedence"],
        "longway_approval"
    );
    assert_eq!(
        structured["scheduler_decision"]["post_fan_in_captain_decision"],
        structured["post_fan_in_captain_decision"]
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run");
    assert_eq!(run_record["approval_state"], "pending_longway_approval");
    assert_eq!(run_record["way_clarification_request"]["state"], "consumed");
    assert!(run_record["way_clarification_request"]["answer_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("registry readiness"));

    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run state");
    assert_eq!(
        run_state["next_action"]["command"],
        "await_longway_approval"
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["lifecycle_state"], "pending_approval");
    assert_eq!(longway["active_phase_status"], "pending_longway_approval");
    assert_eq!(longway["way_clarification_request"]["state"], "consumed");
    let planned_rows = longway["planned_rows"].as_array().expect("planned rows");
    assert_eq!(planned_rows.len(), 2);
    assert_eq!(
        planned_rows[0]["routing_trace"]["reason"],
        "PLAN_SEQUENCE regenerated planned rows after consuming the operator's Way clarification answer exactly once."
    );
    assert_eq!(
        longway["planning_context"]["decomposition_source"],
        "way_clarification_answer"
    );
    let attempt_id = structured["attempt_id"].as_str().expect("attempt id");
    let attempt = read_json_document(
        &run_directory
            .join("orchestration")
            .join("attempts")
            .join(format!("{attempt_id}.json")),
    )
    .expect("attempt payload");
    assert_eq!(
        attempt["post_fan_in_captain_decision"]["schema"],
        "ccc.post_fan_in_captain_decision.v1"
    );
    assert_eq!(
        attempt["scheduler_decision"]["post_fan_in_captain_decision"],
        attempt["post_fan_in_captain_decision"]
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["next_step"], "await_longway_approval");
    assert_eq!(
        status_payload["way_clarification_request"]["state"],
        "consumed"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_plan_sequence_uses_mentioned_target_repo_for_graph_and_memory_context() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-target-parent");
    let repo_dir = parent_dir.join("target-repo");
    create_dir_all(repo_dir.join(".git")).expect("create git marker");
    create_dir_all(repo_dir.join("src")).expect("create src");
    write(
        repo_dir.join("src").join("lib.rs"),
        "pub fn target_repo() {}\n",
    )
    .expect("write lib");
    code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index target graph");
    let canonical_repo = fs::canonicalize(&repo_dir).expect("canonical repo");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 155,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against a mentioned target repository",
                    "title": "Plan mentioned target repo",
                    "intent": "Use the explicitly mentioned repository as graph and memory root",
                    "scope": format!("Inspect {}", repo_dir.join("src").join("lib.rs").display()),
                    "acceptance": "Planning context resolves graph and memory to the mentioned repo root",
                    "prompt": format!("Plan work for {}", repo_dir.display()),
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "git_repo");
    assert_eq!(context["workspace_root"]["confidence"], "high");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_repo.to_string_lossy().to_string()
    );
    assert_eq!(context["graph"]["available"], true);
    assert_eq!(context["graph"]["file_count"], 1);
    assert_eq!(
        context["graph"]["repo_root"],
        canonical_repo
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string()
    );
    assert_eq!(
        context["memory"]["workspace"],
        canonical_repo.to_string_lossy().to_string()
    );
    assert_eq!(
        context["memory"]["path"],
        canonical_repo
            .join(".ccc")
            .join("memory.json")
            .to_string_lossy()
            .to_string()
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_resolves_markdown_link_target_repo() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-markdown-target-parent");
    let repo_dir = parent_dir.join("target-repo");
    create_dir_all(repo_dir.join(".git")).expect("create git marker");
    create_dir_all(repo_dir.join("src")).expect("create src");
    write(
        repo_dir.join("src").join("lib.rs"),
        "pub fn markdown_target_repo() {}\n",
    )
    .expect("write lib");
    code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index target graph");
    let canonical_repo = fs::canonicalize(&repo_dir).expect("canonical repo");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 157,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against a markdown-linked target repository",
                    "title": "Plan markdown target repo",
                    "intent": "Use a markdown link target as graph and memory root evidence",
                    "scope": "Inspect [target lib](target-repo/src/lib.rs) before planning",
                    "acceptance": "Planning context resolves markdown link target to the repo root",
                    "prompt": "Use [target lib](target-repo/src/lib.rs) for graph context",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "git_repo");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_repo.to_string_lossy().to_string()
    );
    assert_eq!(context["graph"]["available"], true);
    assert_eq!(context["graph"]["file_count"], 1);

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_resolves_space_containing_file_path_target_repo() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-space-path-parent");
    let repo_dir = parent_dir.join("target repo");
    let target_file = repo_dir.join("src").join("release notes.rs");
    create_dir_all(repo_dir.join(".git")).expect("create git marker");
    create_dir_all(repo_dir.join("src")).expect("create src");
    write(&target_file, "pub fn space_path_target_repo() {}\n").expect("write target file");
    code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index target graph");
    let canonical_repo = fs::canonicalize(&repo_dir).expect("canonical repo");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 158,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against a target path containing spaces",
                    "title": "Plan space path target repo",
                    "intent": "Use a path-like span with spaces as graph and memory root evidence",
                    "scope": format!("Inspect {} and plan the fix", target_file.display()),
                    "acceptance": "Planning context resolves the file path with spaces to the repo root",
                    "prompt": format!("Use {} for graph context", target_file.display()),
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "git_repo");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_repo.to_string_lossy().to_string()
    );
    assert_eq!(context["graph"]["available"], true);
    assert_eq!(
        context["memory"]["workspace"],
        canonical_repo.to_string_lossy().to_string()
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_resolves_structured_file_path_target_repo() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-structured-file-path-parent");
    let repo_dir = parent_dir.join("target-repo");
    let target_file = repo_dir.join("src").join("lib.rs");
    create_dir_all(repo_dir.join(".git")).expect("create git marker");
    create_dir_all(target_file.parent().expect("target parent")).expect("create target parent");
    write(&target_file, "pub fn structured_file_target_repo() {}\n").expect("write target file");
    code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index target graph");
    let canonical_repo = fs::canonicalize(&repo_dir).expect("canonical repo");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 161,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against a structured file mention",
                    "title": "Plan structured file target repo",
                    "intent": "Use host-provided file_paths as graph and memory root evidence",
                    "scope": "Inspect the structured file mention before planning",
                    "acceptance": "Planning context resolves structured file_paths to the repo root",
                    "prompt": "Plan work for the attached source file",
                    "file_paths": [target_file.to_string_lossy()],
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "git_repo");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_repo.to_string_lossy().to_string()
    );
    assert_eq!(context["graph"]["available"], true);

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_resolves_structured_document_bundle_items() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-structured-document-bundle-parent");
    let bundle_dir = parent_dir.join("document-pack");
    let outline = bundle_dir.join("drafts").join("outline.md");
    let source = bundle_dir.join("sources").join("source.md");
    create_dir_all(outline.parent().expect("outline parent")).expect("create outline parent");
    create_dir_all(source.parent().expect("source parent")).expect("create source parent");
    write(&outline, "# Outline\n").expect("write outline");
    write(&source, "# Source\n").expect("write source");
    let canonical_bundle = fs::canonicalize(&bundle_dir).expect("canonical bundle");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 162,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against structured document bundle items",
                    "title": "Plan structured document bundle",
                    "intent": "Use host-provided input item paths as document-root evidence",
                    "scope": "Use the attached document bundle items without relying on natural-language paths",
                    "acceptance": "Planning context resolves structured input item paths to the shared document bundle root",
                    "prompt": "Plan work for the attached document files",
                    "input_items": [
                        { "type": "local_file", "path": outline.to_string_lossy() },
                        { "type": "artifact", "artifact_path": format!("file://{}", source.display()) }
                    ],
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "document_root");
    assert_eq!(context["workspace_root"]["confidence"], "medium");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_bundle.to_string_lossy().to_string()
    );
    assert_eq!(
        context["memory"]["workspace"],
        canonical_bundle.to_string_lossy().to_string()
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_marks_ambiguous_child_repos_for_confirmation() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-ambiguous-children");
    let repo_a = parent_dir.join("repo-a");
    let repo_b = parent_dir.join("repo-b");
    create_dir_all(repo_a.join(".git")).expect("create repo-a git marker");
    create_dir_all(repo_b.join(".git")).expect("create repo-b git marker");
    let canonical_parent = fs::canonicalize(&parent_dir).expect("canonical parent");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 156,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan from a non-git parent with multiple child repos",
                    "title": "Plan ambiguous target repo",
                    "intent": "Record that the target repo must be confirmed before graph and memory can be trusted",
                    "scope": "No explicit target path was provided",
                    "acceptance": "Planning context asks for target root confirmation",
                    "prompt": "Plan work from this parent directory",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(
        context["workspace_root"]["root_kind"],
        "ambiguous_child_git_repos"
    );
    assert_eq!(context["workspace_root"]["confidence"], "low");
    assert_eq!(context["workspace_root"]["confirmation_required"], true);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_parent.to_string_lossy().to_string()
    );
    assert_eq!(
        context["workspace_root"]["candidates"]
            .as_array()
            .expect("candidates")
            .len(),
        2
    );
    assert_eq!(
        context["memory"]["workspace"],
        canonical_parent.to_string_lossy().to_string()
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_resolves_common_document_bundle_root_for_non_git_files() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-document-bundle-parent");
    let bundle_dir = parent_dir.join("release-work").join("0.0.12");
    let plan_file = bundle_dir.join("notes").join("PRE_RELEASE_PLAN.md");
    let checklist_file = bundle_dir.join("assets").join("checklist.md");
    create_dir_all(plan_file.parent().expect("plan parent")).expect("create plan parent");
    create_dir_all(checklist_file.parent().expect("checklist parent"))
        .expect("create checklist parent");
    write(&plan_file, "# Plan\n").expect("write plan");
    write(&checklist_file, "- [ ] Verify\n").expect("write checklist");
    let canonical_bundle = fs::canonicalize(&bundle_dir).expect("canonical bundle");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 159,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against a non-git document bundle",
                    "title": "Plan document bundle",
                    "intent": "Use related document files as graph and memory root evidence",
                    "scope": format!("Inspect {} and {}", plan_file.display(), checklist_file.display()),
                    "acceptance": "Planning context resolves both document files to their shared bundle root",
                    "prompt": format!("Plan docs using [plan]({}) and {}", plan_file.display(), checklist_file.display()),
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "document_root");
    assert_eq!(context["workspace_root"]["confidence"], "medium");
    assert_eq!(context["workspace_root"]["confirmation_required"], false);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_bundle.to_string_lossy().to_string()
    );
    assert_eq!(
        context["memory"]["workspace"],
        canonical_bundle.to_string_lossy().to_string()
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_start_plan_sequence_keeps_unrelated_non_git_documents_ambiguous() {
    let session_context = create_session_context();
    let parent_dir = create_temp_path("start-plan-unrelated-documents-parent");
    let doc_a = parent_dir.join("bundle-a").join("overview.md");
    let doc_b = parent_dir.join("bundle-b").join("overview.md");
    create_dir_all(doc_a.parent().expect("doc-a parent")).expect("create doc-a parent");
    create_dir_all(doc_b.parent().expect("doc-b parent")).expect("create doc-b parent");
    write(&doc_a, "# A\n").expect("write doc-a");
    write(&doc_b, "# B\n").expect("write doc-b");
    let canonical_parent = fs::canonicalize(&parent_dir).expect("canonical parent");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 160,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": parent_dir.to_string_lossy(),
                    "goal": "Plan against unrelated non-git documents",
                    "title": "Plan unrelated documents",
                    "intent": "Ask the operator to confirm the document root when unrelated files are mentioned",
                    "scope": format!("Compare {} and {}", doc_a.display(), doc_b.display()),
                    "acceptance": "Planning context marks unrelated document roots as ambiguous",
                    "prompt": format!("Use {} and {} for context", doc_a.display(), doc_b.display()),
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_start error response: {response:?}"
    );
    let run_directory = PathBuf::from(
        response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    let context = &longway["planning_context"];
    assert_eq!(context["workspace_root"]["root_kind"], "ambiguous_target");
    assert_eq!(context["workspace_root"]["confidence"], "low");
    assert_eq!(context["workspace_root"]["confirmation_required"], true);
    assert_eq!(
        context["workspace_root"]["root"],
        canonical_parent.to_string_lossy().to_string()
    );
    assert_eq!(
        context["workspace_root"]["candidates"]
            .as_array()
            .expect("candidates")
            .len(),
        2
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_orchestrate_approve_longway_opens_execute_sequence_dispatch() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("approve-longway");
    create_dir_all(&workspace_dir).expect("create workspace");

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 152,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Plan then execute a parser migration",
                    "title": "Plan parser migration",
                    "intent": "Create a pending LongWay approval state",
                    "scope": "Planning first, then execute the parser migration after approval",
                    "acceptance": "Approval opens executable dispatch",
                    "prompt": "Implement the parser migration after planning approval",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    assert!(
        start_response.get("error").is_none(),
        "unexpected ccc_start error response: {start_response:?}"
    );
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    let approve_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 153,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "approve_longway": true
                }
            }
        }),
    )
    .expect("approve response");
    assert!(
        approve_response.get("error").is_none(),
        "unexpected approve response: {approve_response:?}"
    );
    let structured = &approve_response["result"]["structuredContent"];
    assert_eq!(structured["next_step"], "execute_task");
    assert_eq!(structured["can_advance"], true);
    assert_eq!(
        structured["approval_transition"]["to_sequence"],
        "EXECUTE_SEQUENCE"
    );
    assert_eq!(
        structured["approval_transition"]["assigned_role"],
        "explorer"
    );
    assert_eq!(
        structured["approval_transition"]["assigned_agent_id"],
        "scout"
    );
    assert_eq!(
        structured["approval_transition"]["materialized_planned_row"]["row_index"],
        0
    );
    assert_eq!(
        structured["scheduler_decision"]["schema"],
        "ccc.scheduler_decision.v1"
    );
    assert_eq!(
        structured["scheduler_decision"]["decision_source"],
        "planned_row_materialization"
    );
    assert_eq!(
        structured["scheduler_decision"]["selected_planned_row"]["row_index"],
        0
    );
    assert_eq!(
        structured["scheduler_decision"]["action"]["kind"],
        "materialize_planned_row"
    );
    assert_eq!(
        structured["scheduler_decision"]["post_fan_in_captain_decision"]["schema"],
        "ccc.post_fan_in_captain_decision.v1"
    );
    assert_eq!(
        structured["post_fan_in_captain_decision"]["schema"],
        "ccc.post_fan_in_captain_decision.v1"
    );
    assert_eq!(
        structured["scheduler_decision"]["post_fan_in_captain_decision"],
        structured["post_fan_in_captain_decision"]
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(run_record["stage"], "execution");
    assert_eq!(run_record["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(run_record["approval_state"], "approved_for_task_cards");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card");

    let run_state =
        read_json_document(&run_directory.join("run-state.json")).expect("run-state payload");
    assert_eq!(run_state["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(run_state["approval_state"], "approved_for_task_cards");
    assert_eq!(run_state["next_action"]["command"], "execute_task");
    assert_eq!(run_state["current_phase_name"], "inspect");

    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["lifecycle_state"], "active");
    assert_eq!(longway["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(longway["approval_state"], "approved_for_task_cards");
    assert_eq!(longway["active_phase_name"], "inspect");
    assert_eq!(longway["active_phase_status"], "pending");
    assert_eq!(longway["planned_rows"][0]["status"], "materialized");
    assert_eq!(
        longway["planned_rows"][0]["task_card_id"],
        active_task_card_id
    );
    assert_eq!(longway["planned_rows"][1]["status"], "planned");

    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    assert_ne!(active_task_card_id, task_card_id);
    let planning_task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("planning task card");
    assert_eq!(planning_task_card["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(
        planning_task_card["approval_state"],
        "approved_for_task_cards"
    );
    assert_eq!(planning_task_card["status"], "completed");
    assert_eq!(planning_task_card["dispatch_allowed"], false);

    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{active_task_card_id}.json")),
    )
    .expect("materialized task card");
    assert_eq!(task_card["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(task_card["approval_state"], "approved_for_task_cards");
    assert_eq!(task_card["status"], "active");
    assert_eq!(task_card["node_kind"], "execution");
    assert_eq!(task_card["dispatch_allowed"], true);
    assert_eq!(task_card["assigned_role"], "explorer");
    assert_eq!(task_card["assigned_agent_id"], "scout");
    assert_eq!(task_card["planned_longway_row"]["row_index"], 0);
    assert_eq!(
        task_card["routing_trace"]["reason"],
        "PLAN_SEQUENCE generated this row from bounded graph, memory, and capability context."
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["stage"], "execution");
    assert_eq!(status_payload["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(status_payload["approval_state"], "approved_for_task_cards");
    assert_eq!(status_payload["next_step"], "execute_task");
    assert_eq!(
        status_payload["current_task_card"]["task_card_id"],
        active_task_card_id
    );
    assert_eq!(status_payload["scheduler"]["schema"], "ccc.scheduler.v1");
    assert_eq!(
        status_payload["scheduler"]["decision_source"],
        "planned_row_materialization"
    );
    assert_eq!(
        status_payload["scheduler"]["selected_task_card_id"],
        active_task_card_id
    );
    assert_eq!(
        status_payload["scheduler"]["selected_planned_row"]["row_index"],
        0
    );
    assert_eq!(
        status_payload["scheduler"]["planned_rows"]["materialized"],
        1
    );
    assert_eq!(status_payload["scheduler"]["planned_rows"]["planned"], 1);
    assert_eq!(
        status_payload["app_panel"]["scheduler"]["selected_task_card_id"],
        active_task_card_id
    );
    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["scheduler"]["decision_source"],
        "planned_row_materialization"
    );
    let attempt_id = structured["attempt_id"].as_str().expect("attempt id");
    let attempt = read_json_document(
        &run_directory
            .join("orchestration")
            .join("attempts")
            .join(format!("{attempt_id}.json")),
    )
    .expect("attempt payload");
    assert_eq!(
        attempt["scheduler_decision"]["decision_source"],
        "planned_row_materialization"
    );
    assert_eq!(
        attempt["scheduler_decision"]["action"]["kind"],
        "materialize_planned_row"
    );
    assert_eq!(
        attempt["scheduler_decision"]["owns"]["bounded_parallel_fanout"],
        true
    );
    assert_eq!(
        attempt["post_fan_in_captain_decision"]["schema"],
        "ccc.post_fan_in_captain_decision.v1"
    );
    assert_eq!(
        attempt["scheduler_decision"]["post_fan_in_captain_decision"],
        attempt["post_fan_in_captain_decision"]
    );
    assert_ne!(
        status_payload["captain_action_contract"]["required_action"],
        "await_longway_approval"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_diagram_conformance_covers_sisyphus_core_edges() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("diagram-conformance-sisyphus");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("diagram.rs"),
        "pub fn diagram_edge() {}\n",
    )
    .expect("write source");
    code_graph::update_code_graph_store_for_repo(&workspace_dir).expect("index graph");

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 301,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Plan then execute a diagram conformance run",
                    "title": "Diagram conformance run",
                    "intent": "Exercise the core Sisyphus diagram edges",
                    "scope": "Intent, Way, approval, scheduler, router, fan-in, context health, restart handoff",
                    "acceptance": "Status and artifacts expose every diagram edge without contradictory text",
                    "prompt": "Plan with bounded graph evidence and wait for approval before execution.",
                    "task_kind": "execution",
                    "sequence": "PLAN_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    assert!(
        start_response.get("error").is_none(),
        "unexpected diagram start response: {start_response:?}"
    );
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let start_structured = &start_response["result"]["structuredContent"];
    assert_eq!(start_structured["sequence"], "PLAN_SEQUENCE");
    assert_eq!(
        start_structured["approval_state"],
        "pending_longway_approval"
    );
    assert_eq!(start_structured["next_step"], "await_longway_approval");

    let pending_status = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("pending status");
    assert_eq!(pending_status["scheduler"]["state"], "blocked");
    assert_eq!(
        pending_status["scheduler"]["decision_source"],
        "pending_longway_approval"
    );
    assert_eq!(
        pending_status["longway"]["planning_context"]["schema"],
        "ccc.way_planning_context.v1"
    );
    assert_eq!(pending_status["current_task_card"]["assigned_role"], "way");
    let planning_task_card_id = start_structured["task_card_id"]
        .as_str()
        .expect("planning task card id");
    let planning_task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{planning_task_card_id}.json")),
    )
    .expect("planning task card");
    assert_eq!(planning_task_card["dispatch_allowed"], false);

    let approve_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 302,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "approve_longway": true
                }
            }
        }),
    )
    .expect("approve response");
    assert!(
        approve_response.get("error").is_none(),
        "unexpected diagram approve response: {approve_response:?}"
    );
    let approved = &approve_response["result"]["structuredContent"];
    assert_eq!(
        approved["approval_transition"]["to_sequence"],
        "EXECUTE_SEQUENCE"
    );
    assert_eq!(
        approved["scheduler_decision"]["schema"],
        "ccc.scheduler_decision.v1"
    );
    assert_eq!(
        approved["scheduler_decision"]["action"]["kind"],
        "materialize_planned_row"
    );

    let approved_status = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("approved status");
    assert_eq!(approved_status["sequence"], "EXECUTE_SEQUENCE");
    assert_eq!(approved_status["scheduler"]["schema"], "ccc.scheduler.v1");
    assert_eq!(
        approved_status["scheduler"]["decision_source"],
        "planned_row_materialization"
    );
    assert_eq!(
        approved_status["scheduler"]["action"]["kind"],
        "materialized_planned_row"
    );
    assert_eq!(
        approved_status["current_task_card"]["assigned_role"],
        "explorer"
    );
    assert_eq!(
        approved_status["current_task_card"]["assigned_agent_id"],
        "scout"
    );
    assert_eq!(
        approved_status["current_task_card"]["routing_trace"]["reason"],
        "PLAN_SEQUENCE generated this row from bounded graph, memory, and capability context."
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_scout",
        "thread_id": "thread-diagram-scout",
        "status": "completed",
        "summary": "Scout returned bounded diagram evidence.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/diagram.rs:1"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high",
        "risk": "low",
        "checks": ["cargo test ccc_diagram_conformance_covers_sisyphus_core_edges"]
    }))
    .expect("diagram subagent update");

    let fan_in_status = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("fan-in status");
    assert_eq!(fan_in_status["next_step"], "await_fan_in");
    assert_eq!(fan_in_status["host_subagent_state"]["fan_in_ready"], true);
    assert_eq!(
        fan_in_status["current_task_card"]["subagent_fan_in"]["schema"],
        "ccc.worker_result_envelope.v1"
    );
    assert_eq!(
        fan_in_status["current_task_card"]["worker_result_envelope"]["schema"],
        "ccc.worker_result_envelope.v1"
    );
    assert_eq!(
        fan_in_status["context_health"]["schema"],
        "ccc.context_health.v1"
    );
    assert_eq!(
        fan_in_status["restart_handoff"]["schema"],
        "ccc.restart_handoff.v1"
    );
    assert_eq!(fan_in_status["restart_handoff"]["automatic_restart"], false);
    assert_eq!(
        fan_in_status["app_panel"]["schema"],
        "ccc.codex_app_panel.v1"
    );

    let compact = create_ccc_status_compact_payload(&fan_in_status);
    assert_eq!(
        compact["current_task_card"]["worker_result_envelope"]["schema"],
        "ccc.worker_result_envelope.v1"
    );
    assert_eq!(compact["context_health"]["schema"], "ccc.context_health.v1");
    assert_eq!(
        compact["restart_handoff"]["schema"],
        "ccc.restart_handoff.v1"
    );

    let status_text = create_ccc_status_text(&fan_in_status);
    assert!(status_text.contains("Sequence: EXECUTE_SEQUENCE"));
    assert!(status_text.contains("Next: captain"));
    assert!(!status_text.contains("Sequence: PLAN_SEQUENCE stage=execution"));
    assert!(!status_text.contains("Next: execute_task"));
    let artifact = write_codex_app_panel_artifact(&run_directory, &fan_in_status["app_panel"])
        .expect("write Sisyphus app panel artifact");
    assert!(artifact["latest_markdown_path"]
        .as_str()
        .expect("latest markdown")
        .ends_with("CCC_LATEST_PANEL.md"));
    assert!(artifact["latest_json_path"]
        .as_str()
        .expect("latest json")
        .ends_with("CCC_LATEST_PANEL.json"));
    let latest_panel = fs::read_to_string(
        artifact["latest_markdown_path"]
            .as_str()
            .expect("latest markdown path"),
    )
    .expect("read latest panel");
    assert!(latest_panel.contains("# CCC LongWay Panel"));
    assert!(latest_panel.contains("## Subagents"));
    assert!(latest_panel.contains("ccc_scout"));

    let complete_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 303,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "resolve_outcome": "completed",
                    "resolve_summary": "Captain accepted bounded Sisyphus smoke fan-in."
                }
            }
        }),
    )
    .expect("complete response");
    assert!(
        complete_response.get("error").is_none(),
        "unexpected diagram complete response: {complete_response:?}"
    );
    let completed = &complete_response["result"]["structuredContent"];
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["next_step"], "halt_completed");
    assert_eq!(
        completed["scheduler_decision"]["action"]["kind"],
        "complete"
    );

    let completed_status = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("completed status");
    assert_eq!(completed_status["status"], "completed");
    assert_eq!(completed_status["next_step"], "halt_completed");
    assert_eq!(completed_status["scheduler"]["state"], "terminal");
    assert_eq!(
        completed_status["restart_handoff"]["current_longway_state"],
        "completed"
    );
    assert_eq!(
        completed_status["restart_handoff"]["resume_command"],
        format!("$cap continue {run_id}")
    );
    assert_eq!(completed_status["longway"]["lifecycle_state"], "completed");
    let completed_compact = create_ccc_status_compact_payload(&completed_status);
    assert_eq!(completed_compact["app_panel"]["run"]["status"], "completed");
    assert_eq!(
        completed_compact["app_panel"]["run"]["next_step"],
        "halt_completed"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_persists_planned_longway_rows_without_materializing_task_cards() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("start-planned-rows");
    create_dir_all(&workspace_dir).expect("create workspace");

    let response = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Bootstrap a planned LongWay",
        "title": "Implement the executable first row",
        "intent": "Persist planned follow-up rows",
        "scope": "Initial task plus planned rows only",
        "acceptance": "Planned rows render without task card materialization",
        "prompt": "Implement the executable first row",
        "task_kind": "execution",
        "planned_rows": [
            {
                "title": "Verify planned row persistence",
                "planned_role": "implementation_specialist",
                "planned_agent_id": "raider-b",
                "scope": "LongWay persistence only",
                "acceptance": "Status shows the planned row",
                "status": "pending",
                "evidence_links": ["rust/ccc-mcp/src/run_bootstrap.rs"],
                "routing_summary": "bounded follow-up",
                "task_card_id": "must-not-materialize"
            },
            "Document planned row input",
            "Commit README.md and README.ja.md with a concise documentation message",
            "Stage display cleanup"
        ]
    }))
    .expect("start payload");
    let run_id = response["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(1)
    );
    let task_card_count = fs::read_dir(run_directory.join("task-cards"))
        .expect("task cards")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .count();
    assert_eq!(task_card_count, 1);

    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["phases"].as_array().map(Vec::len), Some(1));
    assert_eq!(longway["planned_rows"].as_array().map(Vec::len), Some(4));
    assert_eq!(
        longway["planned_rows"][0]["title"],
        "Verify planned row persistence"
    );
    assert_eq!(longway["planned_rows"][0]["planned_agent_id"], "raider-b");
    assert_eq!(longway["planned_rows"][0]["status"], "planned");
    assert!(longway["planned_rows"][0].get("task_card_id").is_none());
    assert_eq!(
        longway["planned_rows"][1]["title"],
        "Document planned row input"
    );
    assert_eq!(longway["planned_rows"][1]["planned_role"], "documenter");
    assert_eq!(longway["planned_rows"][1]["planned_agent_id"], "scribe");
    assert!(longway["planned_rows"][1].get("display_role").is_none());
    assert!(longway["planned_rows"][1].get("display_agent_id").is_none());
    assert_eq!(longway["planned_rows"][1]["model"], "gpt-5.4-mini");
    assert_eq!(longway["planned_rows"][1]["reasoning"], "medium");
    assert_eq!(
        longway["planned_rows"][1]["scope"],
        "No explicit planned-row scope."
    );
    assert_eq!(
        longway["planned_rows"][1]["acceptance"],
        "No explicit planned-row acceptance."
    );
    assert_eq!(longway["planned_rows"][1]["status"], "planned");
    assert_eq!(
        longway["planned_rows"][2]["planned_role"],
        "companion_operator"
    );
    assert_eq!(
        longway["planned_rows"][2]["planned_agent_id"],
        "companion_operator"
    );
    assert_ne!(
        longway["planned_rows"][3]["planned_role"],
        "companion_operator"
    );
    assert_ne!(
        longway["planned_rows"][3]["planned_agent_id"],
        "companion_operator"
    );
    assert_ne!(
        longway["planned_rows"][3]["display_role"],
        "companion_operator"
    );
    assert_ne!(
        longway["planned_rows"][3]["display_agent_id"],
        "ccc_companion_operator"
    );

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(status_payload["longway"]["planned_row_count"], 4);
    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains(
        "[ ] Document planned row input -> Adjutant(ccc_scribe) model=gpt-5.4-mini reasoning=medium"
    ));
    assert!(app_panel_text.contains(
        "sources=agent:planned_row_input,model:planned_row_input,reasoning:planned_row_input"
    ));
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    let checklist = checklist_payload["checklist"].as_str().expect("checklist");
    assert!(checklist.starts_with("LongWay"));
    assert!(checklist.contains("Verify planned row persistence [raider-b]"));
    assert!(checklist.contains(
        "Document planned row input [Adjutant(ccc_scribe)] role=Adjutant(ccc_scribe)/documenter"
    ));
    assert!(checklist.contains(
        "Commit README.md and README.ja.md with a concise documentation message [SCV(ccc_companion_operator)]"
    ));
    assert!(
        !checklist.contains("Stage display cleanup [companion_operator]"),
        "stage row must not render as companion operator: {checklist}"
    );
    assert!(!checklist.contains("+---"));
    assert!(!checklist.contains("sources="));
    assert!(!checklist.contains("scope="));
    assert!(!checklist.contains("accept="));
    assert!(!checklist.contains("CCC LongWay"));
    assert!(!checklist.contains("Gauge:"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_routes_installed_cli_smoke_to_scout_not_raider() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("installed-cli-smoke-routing");
    create_dir_all(&workspace_dir).expect("create workspace");

    let response = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "title": "CCC 0.0.13-pre installed CLI smoke",
        "intent": "Validate installed quiet lifecycle CLI paths",
        "goal": "Confirm installed binary start/status/checklist/orchestrate/subagent-update surfaces work",
        "scope": "Temporary smoke run under /private/tmp",
        "acceptance": "Quiet CLI commands return compact lifecycle lines and status/checklist expose planned rows",
        "prompt": "Smoke test installed CCC 0.0.13-pre quiet CLI paths.",
        "sequence": "EXECUTE_SEQUENCE",
        "task_kind": "execution",
        "planned_rows": [
            "Inspect installed identity",
            "Verify quiet status and checklist",
            "Record terminal subagent cleanup"
        ]
    }))
    .expect("start payload");
    let run_id = response["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));
    let task_card_id = response["task_card_id"].as_str().expect("task card id");
    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");

    assert_eq!(task_card["routing_trace"]["request_shape"], "diagnostic");
    assert_eq!(task_card["assigned_role"], "explorer");
    assert_eq!(task_card["assigned_agent_id"], "scout");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["assignment_quality"]["state"],
        "matched"
    );
    assert_eq!(
        status_payload["current_task_card"]["assignment_quality"]["expected_family"],
        "read_only_diagnostic"
    );
    assert_eq!(
        status_payload["context_health"]["active_conflict_state"]["assignment_drift"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_run_persists_planned_longway_rows_through_initial_checkpoint() {
    let workspace_dir = create_temp_path("run-planned-rows");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable(&workspace_dir);

    let response = crate::run_bootstrap::create_ccc_run_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Run a planned LongWay",
        "title": "Dispatch initial executable row",
        "intent": "Persist planned rows through ccc_run",
        "scope": "No planned row materialization yet",
        "acceptance": "Planned rows remain pending after initial checkpoint",
        "prompt": "Dispatch the executable first row",
        "task_kind": "execution",
        "codex_bin": fake_codex.to_string_lossy(),
        "workflow_variant_selection": {
            "workflow_variant": "way"
        },
        "planned_rows": [{
            "title": "Review persisted planned rows",
            "planned_role": "review_specialist",
            "planned_agent_id": "reviewer-a",
            "status": "materialized"
        }]
    }))
    .expect("run payload");
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(1)
    );
    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["phases"].as_array().map(Vec::len), Some(1));
    assert_eq!(longway["planned_rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        longway["planned_rows"][0]["title"],
        "Review persisted planned rows"
    );
    assert_eq!(
        longway["planned_rows"][0]["scope"],
        "No explicit planned-row scope."
    );
    assert_eq!(
        longway["planned_rows"][0]["acceptance"],
        "No explicit planned-row acceptance."
    );
    assert_eq!(longway["planned_rows"][0]["status"], "planned");
    assert!(longway["planned_rows"][0].get("task_card_id").is_none());

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_advance_materializes_next_planned_row() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-materialize-planned-row");
    create_dir_all(&workspace_dir).expect("create workspace");
    let unbounded_routing_summary =
        "Use the planned implementation lane with bounded graph evidence. ".repeat(8);
    let bounded_routing_summary = summarize_text_for_visibility(&unbounded_routing_summary, 240);

    let response = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Materialize planned LongWay rows just in time",
        "title": "Implement initial executable row",
        "intent": "Keep later rows planned until captain advance",
        "scope": "Initial task only",
        "acceptance": "Later rows are not task cards until advance",
        "prompt": "Implement the initial row",
        "task_kind": "execution",
        "planned_rows": [
            {
                "title": "Implement planned materialized row",
                "planned_role": "implementation_specialist",
                "planned_agent_id": "raider-c",
                "scope": "Create the next executable task from the planned row",
                "acceptance": "Persist the materialized task card id",
                "routing_summary": unbounded_routing_summary.clone(),
                "evidence_links": ["rust/ccc-mcp/src/main.rs"]
            },
            {
                "title": "Verify materialized row",
                "planned_role": "review_specialist",
                "planned_agent_id": "arbiter",
                "scope": "Review after implementation",
                "acceptance": "Remain planned until a later advance"
            }
        ]
    }))
    .expect("start payload");
    let run_id = response["run_id"].as_str().expect("run id").to_string();
    let initial_task_card_id = response["task_card_id"]
        .as_str()
        .expect("initial task card id")
        .to_string();
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));
    let longway_path = run_directory.join("longway.json");
    let mut longway = read_json_document(&longway_path).expect("longway");
    longway["planned_rows"][0]["routing_trace"] = json!({
        "query": "review_context",
        "paths": ["rust/ccc-mcp/src/main.rs", "rust/ccc-mcp/src/status_payload.rs"],
        "terms": ["planned_rows", "materialize"],
        "reason": "Graph review identified the materialization boundary.",
        "summary": "Keep only routing evidence, not graph output.",
        "query_result": {
            "raw_graph_dump": ["full", "graph", "output"]
        }
    });
    longway["planned_rows"][0]["result_links"] = json!(["rust/ccc-mcp/src/main_tests.rs"]);
    longway["planned_rows"][1]["routing_trace"] = json!({
        "query_result": {
            "raw_graph_dump": ["later", "planned", "row"]
        }
    });
    write_json_document(&longway_path, &longway).expect("write longway");

    force_run_to_captain_advance(&run_directory);
    let advance_response =
        call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 240);
    assert!(
        advance_response.get("error").is_none(),
        "unexpected materialize response: {advance_response:?}"
    );
    assert_eq!(
        advance_response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    assert_ne!(active_task_card_id, initial_task_card_id);
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(2)
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(task_card["task_card_id"], active_task_card_id);
    assert_eq!(task_card["title"], "Implement planned materialized row");
    assert_eq!(task_card["assigned_role"], "code specialist");
    assert_eq!(
        task_card["planned_longway_row"]["planned_agent_id"],
        "raider-c"
    );
    assert_task_card_schema_declares(
        &[
            "routing_summary",
            "routing_trace",
            "evidence_links",
            "result_links",
            "planned_longway_row",
        ],
        include_str!("../../../schemas/task-card.schema.json"),
    );
    assert_eq!(task_card["routing_summary"], bounded_routing_summary);
    assert!(task_card["routing_summary"]
        .as_str()
        .is_some_and(|value| value.chars().count() <= 240));
    assert_eq!(task_card["routing_trace"]["source"], "planned_row");
    assert_eq!(task_card["routing_trace"]["query"], "review_context");
    assert_eq!(
        task_card["routing_trace"]["paths"],
        json!([
            "rust/ccc-mcp/src/main.rs",
            "rust/ccc-mcp/src/status_payload.rs"
        ])
    );
    assert_eq!(
        task_card["routing_trace"]["terms"],
        json!(["planned_rows", "materialize"])
    );
    assert_eq!(
        task_card["routing_trace"]["reason"],
        "Graph review identified the materialization boundary."
    );
    assert_eq!(
        task_card["routing_trace"]["summary"],
        "Keep only routing evidence, not graph output."
    );
    assert!(task_card["routing_trace"].get("query_result").is_none());
    assert_eq!(
        task_card["evidence_links"],
        json!(["rust/ccc-mcp/src/main.rs"])
    );
    assert_eq!(
        task_card["result_links"],
        json!(["rust/ccc-mcp/src/main_tests.rs"])
    );
    let transition = read_json_document(
        &run_directory
            .join("scheduler")
            .join("transitions")
            .join("transition-0001.json"),
    )
    .expect("scheduler transition");
    assert_eq!(transition["schema"], "ccc.scheduler_transition.v1");
    assert_eq!(transition["decision_source"], "planned_row_materialization");
    assert_eq!(transition["action"]["kind"], "materialize_planned_row");
    assert_eq!(
        transition["selected_task_card_id"].as_str(),
        Some(active_task_card_id)
    );
    assert_eq!(transition["selected_planned_row"]["row_index"], 0);
    assert_eq!(
        transition["selected_planned_row"]["task_card_id"].as_str(),
        Some(active_task_card_id)
    );
    assert_eq!(transition["route"]["assigned_role"], "code specialist");
    assert_eq!(
        transition["route"]["routing_trace"]["query"],
        "review_context"
    );
    assert_eq!(
        transition["next_expected_lifecycle_event"]["event"],
        "subagent_update"
    );

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["routing_trace"]["query"],
        "review_context"
    );
    assert_eq!(
        status_payload["current_task_card"]["routing_summary"],
        bounded_routing_summary
    );
    assert_eq!(
        status_payload["current_task_card"]["evidence_links"],
        json!(["rust/ccc-mcp/src/main.rs"])
    );
    assert_eq!(
        status_payload["current_task_card"]["result_links"],
        json!(["rust/ccc-mcp/src/main_tests.rs"])
    );
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["routing_summary"],
        bounded_routing_summary
    );
    assert_eq!(
        status_payload["longway"]["planned_rows"][0]["routing_trace"]["query"],
        "review_context"
    );
    assert_eq!(
        status_payload["scheduler"]["latest_transition"]["transition_id"],
        "transition-0001"
    );
    assert_eq!(
        status_payload["scheduler"]["latest_transition"]["selected_planned_row"]["task_card_id"]
            .as_str(),
        Some(active_task_card_id)
    );
    assert!(
        status_payload["longway"]["planned_rows"][0]["routing_trace"]
            .get("query_result")
            .is_none()
    );
    assert!(
        !serde_json::to_string(&status_payload["longway"]["planned_rows"][0])
            .expect("serialize status planned row")
            .contains("raw_graph_dump")
    );
    assert!(
        !serde_json::to_string(&status_payload["longway"]["planned_rows"][1])
            .expect("serialize status later planned row")
            .contains("raw_graph_dump")
    );

    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["phases"].as_array().map(Vec::len), Some(2));
    assert_eq!(longway["planned_rows"][0]["status"], "materialized");
    assert_eq!(
        longway["planned_rows"][0]["task_card_id"],
        active_task_card_id
    );
    let materialized_row = &longway["planned_rows"][0];
    assert!(materialized_row["materialized_at"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(materialized_row["updated_at"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(materialized_row["routing_summary"], bounded_routing_summary);
    assert_eq!(materialized_row["routing_trace"]["query"], "review_context");
    assert!(materialized_row["routing_trace"]
        .get("query_result")
        .is_none());
    assert!(!serde_json::to_string(materialized_row)
        .expect("serialize persisted planned row")
        .contains("raw_graph_dump"));
    assert_planned_row_keys_declared(
        materialized_row,
        include_str!("../../../schemas/longway.schema.json"),
    );
    assert_planned_row_keys_declared(
        materialized_row,
        include_str!("../../../schemas/way.schema.json"),
    );
    assert_eq!(longway["planned_rows"][1]["status"], "planned");
    assert!(longway["planned_rows"][1].get("task_card_id").is_none());
    assert!(longway["planned_rows"][1].get("routing_trace").is_none());
    assert!(!serde_json::to_string(&longway["planned_rows"][1])
        .expect("serialize later persisted planned row")
        .contains("raw_graph_dump"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_sanitizes_current_task_card_routing_trace_graph_dumps() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-current-task-card-routing-trace-sanitize");
    create_dir_all(&workspace_dir).expect("create workspace");
    let raw_routing_summary =
        "Manual routing summary with enough detail that status must bound the visible text. "
            .repeat(8);
    let bounded_routing_summary = summarize_text_for_visibility(&raw_routing_summary, 240);

    let response = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Expose bounded active task-card routing visibility",
        "title": "Inspect active routing trace",
        "intent": "Regression for legacy manual task-card routing traces",
        "scope": "Only ccc_status current_task_card routing fields",
        "acceptance": "Status drops raw graph query results from current task cards",
        "prompt": "Inspect status payload routing trace",
        "task_kind": "explore"
    }))
    .expect("start payload");
    let run_id = response["run_id"].as_str().expect("run id").to_string();
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));
    let task_card_id = response["task_card_id"]
        .as_str()
        .expect("task card id")
        .to_string();
    let task_card_path = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_path).expect("read task card");
    task_card["routing_summary"] = json!(raw_routing_summary);
    task_card["routing_trace"] = json!({
        "source": "manual_legacy_task_card",
        "query": "review_context",
        "paths": [
            "rust/ccc-mcp/src/status_payload.rs",
            "rust/ccc-mcp/src/main_tests.rs"
        ],
        "terms": ["routing_trace", "query_result", "status_payload"],
        "summary": "Expose routing context without raw graph dumps.",
        "selected_role": "explorer",
        "companion_route_enforced": true,
        "query_result": {
            "raw_graph_dump": ["full", "unbounded", "graph"],
            "nodes": [{"path": "rust/ccc-mcp/src/status_payload.rs", "body": "raw body"}]
        },
        "tool_route": {
            "raw_graph_dump": "nested route object should not be projected"
        }
    });
    write_json_document(&task_card_path, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    let current_task = &status_payload["current_task_card"];
    assert_eq!(current_task["routing_summary"], bounded_routing_summary);
    assert_eq!(
        current_task["routing_trace"]["source"],
        "manual_legacy_task_card"
    );
    assert_eq!(current_task["routing_trace"]["query"], "review_context");
    assert_eq!(current_task["routing_trace"]["selected_role"], "explorer");
    assert_eq!(
        current_task["routing_trace"]["paths"],
        json!([
            "rust/ccc-mcp/src/status_payload.rs",
            "rust/ccc-mcp/src/main_tests.rs"
        ])
    );
    assert_eq!(
        current_task["routing_trace"]["terms"],
        json!(["routing_trace", "query_result", "status_payload"])
    );
    assert_eq!(
        current_task["routing_trace"]["summary"],
        "Expose routing context without raw graph dumps."
    );
    assert_eq!(
        current_task["routing_trace"]["companion_route_enforced"],
        true
    );
    assert!(current_task["routing_trace"].get("query_result").is_none());
    assert!(current_task["routing_trace"].get("tool_route").is_none());
    assert!(!serde_json::to_string(current_task)
        .expect("serialize current task status")
        .contains("raw_graph_dump"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_materializes_read_only_planned_row_as_scout() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-materialize-read-only-scout-row");
    create_dir_all(&workspace_dir).expect("create workspace");

    let response = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Materialize read-only planned LongWay rows safely",
        "title": "Implement initial executable row",
        "intent": "Keep later rows planned until captain advance",
        "scope": "Initial task only",
        "acceptance": "Later rows are not task cards until advance",
        "prompt": "Implement the initial row",
        "task_kind": "execution",
        "planned_rows": [
            {
                "title": "Read-only scout evidence for routing drift",
                "planned_role": "unassigned",
                "planned_agent_id": "unassigned",
                "scope": "Inspect run artifacts and collect evidence only.",
                "acceptance": "Return status and evidence without workspace mutation."
            }
        ]
    }))
    .expect("start payload");
    let run_id = response["run_id"].as_str().expect("run id").to_string();
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));
    let longway_path = run_directory.join("longway.json");
    let mut longway = read_json_document(&longway_path).expect("longway");
    let planned_row = longway["planned_rows"][0]
        .as_object_mut()
        .expect("planned row object");
    planned_row.remove("display_role");
    planned_row.remove("display_agent_id");
    write_json_document(&longway_path, &longway).expect("write longway");

    force_run_to_captain_advance(&run_directory);
    let advance_response =
        call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 243);
    assert!(
        advance_response.get("error").is_none(),
        "unexpected materialize response: {advance_response:?}"
    );
    assert_eq!(
        advance_response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );

    let task_card = read_active_task_card(&run_directory);
    assert_eq!(task_card["assigned_role"], "explorer");
    assert_eq!(task_card["assigned_agent_id"], "scout");
    assert_eq!(task_card["sandbox_mode"], "read-only");
    assert_eq!(task_card["planned_longway_row"]["planned_role"], "explorer");
    assert_eq!(
        task_card["planned_longway_row"]["planned_agent_id"],
        "scout"
    );

    let transition = read_json_document(
        &run_directory
            .join("scheduler")
            .join("transitions")
            .join("transition-0001.json"),
    )
    .expect("scheduler transition");
    assert_eq!(transition["decision_source"], "planned_row_materialization");
    assert_eq!(transition["route"]["assigned_role"], "explorer");
    assert_eq!(transition["route"]["assigned_agent_id"], "scout");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_planned_row_materialization_is_idempotent() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-materialize-planned-row-idempotent");
    create_dir_all(&workspace_dir).expect("create workspace");

    let response = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Materialize one planned row once",
        "title": "Implement first row",
        "intent": "Exercise idempotent planned-row materialization",
        "scope": "Initial task only",
        "acceptance": "Repeated advance does not duplicate the materialized row",
        "prompt": "Implement the first row",
        "task_kind": "execution",
        "planned_rows": [{
            "title": "Implement idempotent planned row",
            "planned_role": "code specialist",
            "planned_agent_id": "raider",
            "scope": "Materialize once",
            "acceptance": "Only one task card is created"
        }]
    }))
    .expect("start payload");
    let run_id = response["run_id"].as_str().expect("run id").to_string();
    let run_directory = PathBuf::from(response["run_directory"].as_str().expect("run directory"));

    force_run_to_captain_advance(&run_directory);
    let first_response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 241);
    assert!(
        first_response.get("error").is_none(),
        "unexpected first materialize response: {first_response:?}"
    );
    let first_active_task_card_id = read_json_document(&run_directory.join("run.json"))
        .expect("run record")["active_task_card_id"]
        .as_str()
        .expect("active task-card id")
        .to_string();

    force_run_to_captain_advance(&run_directory);
    let second_response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 242);
    assert!(
        second_response.get("error").is_none(),
        "unexpected second materialize response: {second_response:?}"
    );
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(
        run_record["active_task_card_id"].as_str(),
        Some(first_active_task_card_id.as_str())
    );
    assert_eq!(
        run_record["task_card_ids"].as_array().map(Vec::len),
        Some(2)
    );
    let task_card_count = fs::read_dir(run_directory.join("task-cards"))
        .expect("task cards")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .count();
    assert_eq!(task_card_count, 2);

    let longway = read_json_document(&run_directory.join("longway.json")).expect("longway");
    assert_eq!(longway["planned_rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(longway["planned_rows"][0]["status"], "materialized");
    assert_eq!(
        longway["planned_rows"][0]["task_card_id"].as_str(),
        Some(first_active_task_card_id.as_str())
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_start_surfaces_existing_active_run_for_continuity_decision() {
    let workspace_dir = create_temp_path("start-active-run-scan");
    create_dir_all(&workspace_dir).expect("create workspace");

    let first = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Inspect active work",
        "title": "First active run",
        "intent": "Keep one active run visible",
        "scope": "One active task",
        "acceptance": "Persist first run",
        "prompt": "Inspect the current state",
        "task_kind": "explore"
    }))
    .expect("first start payload");
    let first_run_id = first["run_id"].as_str().expect("first run id");
    let first_task_card_id = first["task_card_id"].as_str().expect("first task card id");
    let first_run_directory = PathBuf::from(first["run_directory"].as_str().expect("first dir"));
    create_ccc_subagent_update_payload(&json!({
        "run_id": first_run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_scout",
        "thread_id": "thread-prior-active",
        "status": "running",
        "summary": "Prior host subagent is still running."
    }))
    .expect("record prior running subagent");

    let second = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Continue with a new request",
        "title": "Second request",
        "intent": "Surface active prior run before continuing",
        "scope": "One continuity-aware task",
        "acceptance": "Expose active run scan truth",
        "prompt": "Continue from the latest request",
        "task_kind": "way"
    }))
    .expect("second start payload");

    assert_eq!(
        second["run_selection"],
        "new_run_created_after_prior_reclaim"
    );
    assert_eq!(
        second["active_run_scan"]["active_run_scan_state"],
        "no_active_runs"
    );
    assert_eq!(second["active_run_scan"]["fresh_active_run_count"], 0);
    assert_eq!(
        second["active_run_scan"]["continuity_strategy"],
        "fresh_run_ok"
    );
    assert_eq!(second["active_run_scan"]["reclaimed_prior_run_count"], 1);
    assert_eq!(
        second["active_run_scan"]["prior_run_cleanup_performed"],
        true
    );
    assert!(second["active_run_scan"]["prior_run_cleanup_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("1 prior active run"));
    assert_eq!(
        second["active_run_scan"]["reclaimed_runs"][0]["run_id"],
        first_run_id
    );
    assert_eq!(
        second["active_run_scan"]["reclaimed_runs"][0]["reclaimed_child_count"],
        1
    );
    assert_eq!(
        second["active_run_scan"]["host_subagent_cancel_supported"],
        false
    );

    let first_run_record =
        read_json_document(&first_run_directory.join("run.json")).expect("first run record");
    assert_eq!(first_run_record["status"], "reclaimed");
    assert_eq!(first_run_record["active_agent_id"], "captain");
    assert_eq!(first_run_record["active_thread_id"], Value::Null);
    assert_eq!(
        first_run_record["latest_reclaim"]["reason"],
        "prior_run_auto_cleanup"
    );
    let first_task_card = read_json_document(
        &first_run_directory
            .join("task-cards")
            .join(format!("{first_task_card_id}.json")),
    )
    .expect("first task card");
    assert_eq!(first_task_card["subagent_lifecycle"]["status"], "reclaimed");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_run_dispatches_initial_worker_and_persists_attempt() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("run-front-door");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable(&workspace_dir);

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 17,
            "method": "tools/call",
            "params": {
                "name": "ccc_run",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Continue the right-sidebar agent dashboard plan",
                    "title": "Persist the first Rust execution checkpoint",
                    "intent": "Create a run and checkpoint it for later Rust orchestration",
                    "scope": "Single execution task only",
                    "acceptance": "Persist initial orchestration attempt artifacts",
                    "prompt": "Implement the next bounded step",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE",
                    "codex_bin": fake_codex.to_string_lossy(),
                    "workflow_variant_selection": {
                        "workflow_variant": "way"
                    }
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_run error response: {response:?}"
    );

    let payload = &response["result"]["structuredContent"];
    let run_id = payload["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(payload["run_directory"].as_str().expect("run directory"));
    assert_eq!(payload["status"], "active");
    assert_eq!(payload["stage"], "execution");
    assert_eq!(payload["next_step"], "execute_task");
    assert_eq!(payload["can_advance"], true);
    assert_eq!(payload["advanced"], true);
    assert_eq!(payload["entrypoint"], "ccc_run");
    assert_eq!(payload["allowed_next_commands"], json!(["advance"]));

    let attempt = read_json_document(
        &run_directory
            .join("orchestration")
            .join("attempts")
            .join("attempt-0001.json"),
    )
    .expect("attempt payload");
    assert_eq!(attempt["entrypoint"], "ccc_orchestrate");
    assert_eq!(attempt["run_id"], run_id);
    assert_eq!(attempt["stop"]["reason"], "await_host_custom_subagent");
    assert_eq!(
        attempt["dispatch_policy"]["codex_exec_dispatch_allowed"],
        false
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(
        run_record["latest_entry_trace"]["entrypoint"],
        "ccc_orchestrate"
    );
    assert_eq!(run_record["active_agent_id"], "captain");

    let events = fs::read_to_string(run_directory.join("events.jsonl")).expect("events");
    assert!(events.contains("\"event\":\"run_checkpointed\""));
    assert!(events.contains("\"event\":\"run_orchestrated\""));
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_persists_explicit_attempt_for_existing_run() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("orchestrate");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable(&workspace_dir);

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 18,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Bootstrap a Rust-started run",
                    "title": "Implement the next bounded step",
                    "intent": "Create a run without invoking Codex",
                    "scope": "Single execution task only",
                    "acceptance": "Persist run bootstrap artifacts",
                    "prompt": "Implement the first bounded task",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 19,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        response.get("error").is_none(),
        "unexpected ccc_orchestrate error response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["progression_mode"],
        "single_step"
    );
    assert_eq!(
        response["result"]["structuredContent"]["starting_next_step"],
        "execute_task"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    assert_eq!(response["result"]["structuredContent"]["can_advance"], true);
    assert_eq!(
        response["result"]["structuredContent"]["allowed_next_commands"],
        json!(["advance"])
    );

    let attempt = read_json_document(
        &run_directory
            .join("orchestration")
            .join("attempts")
            .join("attempt-0001.json"),
    )
    .expect("attempt payload");
    assert_eq!(attempt["entrypoint"], "ccc_orchestrate");
    assert_eq!(attempt["requested_progression_mode"], "single_step");
    assert_eq!(attempt["starting_next_step"], "execute_task");
    assert_eq!(attempt["stop"]["reason"], "await_host_custom_subagent");
    assert_eq!(
        attempt["dispatch_policy"]["codex_exec_dispatch_allowed"],
        false
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(
        run_record["latest_entry_trace"]["entrypoint"],
        "ccc_orchestrate"
    );
    assert_eq!(run_record["active_agent_id"], "captain");

    let delegations_dir = run_directory.join("delegations");
    assert!(!delegations_dir.exists());

    let events = fs::read_to_string(run_directory.join("events.jsonl")).expect("events");
    assert!(events.contains("\"event\":\"run_orchestrated\""));
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_reclaims_stuck_worker_and_reopens_captain_advance() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("reclaim");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-reclaim");
    create_dir_all(run_directory.join("delegations")).expect("create delegations");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-reclaim",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "await_fan_in",
                "can_advance": false,
                "summary": "waiting for worker return"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator state"),
    )
    .expect("write orchestrator state");
    write(
        run_directory.join("run-state.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "run_id": "run-reclaim",
            "updated_at": "2026-04-22T08:01:00.000Z",
            "event_count": 3,
            "last_event_id": "event-3",
            "current_phase_name": "execute",
            "next_action": {
                "command": "await_fan_in"
            }
        }))
        .expect("serialize run-state"),
    )
    .expect("write run-state");

    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("sleep 5")
        .spawn()
        .expect("spawn worker");

    write(
            run_directory.join("delegations").join("delegation-reclaim.json"),
            serde_json::to_vec_pretty(&json!({
                "delegation_id": "delegation-reclaim",
                "run_id": "run-reclaim",
                "task_card_id": "task-1",
                "delegated_by_role": "orchestrator",
                "review_round": null,
                "summary": "worker is stalled",
                "child_agent": {
                    "agent_id": "raider",
                    "parent_agent_id": "captain",
                    "role": "code specialist",
                    "status": "running",
                    "task_card_id": "task-1"
                },
                "executor": {
                    "executor_id": "specialist-executor:raider",
                    "status": "running",
                    "task_card_id": "task-1",
                    "delegation_id": "delegation-reclaim",
                    "child_agent_id": "raider"
                },
                "worker_request": {
                    "prompt": "Implement the bounded task",
                    "acceptance": "Return a bounded result"
                },
                "worker_launch_evidence": {
                    "raw_events_file": run_directory.join("raw-events").join("missing.jsonl").to_string_lossy(),
                },
                "worker_lifecycle": {
                    "state": "running",
                    "reclaim_state": "not_needed",
                    "queued_at": "2026-04-22T07:00:00.000Z",
                    "launch_requested_at": "2026-04-22T07:00:01.000Z",
                    "started_at": "2026-04-22T07:00:02.000Z",
                    "process_id": child.id(),
                    "process_started_at": "2026-04-22T07:00:02.000Z",
                    "process_last_seen_at": "2026-04-22T07:00:03.000Z",
                    "last_progress_at": "2026-04-22T07:00:03.000Z",
                    "returned_at": null,
                    "stale_at": null,
                    "timed_out_at": null,
                    "stale_after_ms": 45000,
                    "timeout_after_ms": 45000,
                    "summary": "running"
                },
                "worker_result": null,
                "result_summary": null,
                "reviewer_outcome": null,
                "latest_failure": null,
                "created_at": "2026-04-22T07:00:00.000Z",
                "updated_at": "2026-04-22T07:00:03.000Z",
                "completed_at": null
            }))
            .expect("serialize delegation"),
        )
        .expect("write delegation");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": "run-reclaim",
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("response");
    assert!(
        response.get("error").is_none(),
        "unexpected reclaim response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "advance"
    );
    assert_eq!(response["result"]["structuredContent"]["can_advance"], true);
    assert_eq!(
        response["result"]["structuredContent"]["reclaimed_targets"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        response["result"]["structuredContent"]["scheduler_decision"]["action"]["kind"],
        "blocked_reclaim"
    );

    let delegation = read_json_document(
        &run_directory
            .join("delegations")
            .join("delegation-reclaim.json"),
    )
    .expect("delegation");
    assert_eq!(delegation["child_agent"]["status"], "failed");
    assert_eq!(delegation["executor"]["status"], "failed");
    assert_eq!(delegation["worker_lifecycle"]["reclaim_state"], "reclaimed");

    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    assert_eq!(run_state["next_action"]["command"], "advance");

    let _ = child.wait();
    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_collapses_completed_fan_in_and_reopens_captain_advance() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("fan-in-collapse");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable(&workspace_dir);

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Bootstrap a Rust-started run",
                    "title": "Implement the next bounded step",
                    "intent": "Create a run without invoking Codex",
                    "scope": "Single execution task only",
                    "acceptance": "Persist run bootstrap artifacts",
                    "prompt": "Implement the first bounded task",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);

    let first_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("first orchestrate");
    assert!(
        first_response.get("error").is_none(),
        "unexpected first fan-in orchestrate response: {first_response:?}"
    );
    let first_next_step = first_response["result"]["structuredContent"]["next_step"].as_str();
    assert!(
        first_next_step.is_none() || matches!(first_next_step, Some("await_fan_in" | "advance")),
        "unexpected first next_step: {first_response:?}"
    );
    let delegation_path = run_directory.join("delegations");
    let wait_deadline = SystemTime::now() + Duration::from_secs(1);
    loop {
        let completed = fs::read_dir(&delegation_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| read_json_document(&entry.path()).ok())
            .any(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    == Some("completed")
            });
        if completed || SystemTime::now() >= wait_deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    let worker_result_envelope = json!({
        "schema": "ccc.worker_result_envelope.v1",
        "summary": "Worker returned implementation evidence.",
        "status": "completed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high",
        "risk": "low",
        "checks": ["cargo test --lib"],
        "contract": {
            "captain_consumes_compact_fan_in": true
        }
    });
    task_card["subagent_fan_in"] = worker_result_envelope.clone();
    task_card["worker_result_envelope"] = worker_result_envelope;
    write_json_document(&task_card_file, &task_card).expect("write task card envelope");

    let second_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("second orchestrate");
    assert!(
        second_response.get("error").is_none(),
        "unexpected fan-in collapse response: {second_response:?}"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["next_step"],
        "advance"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["can_advance"],
        true
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["collapsed_fan_in"]["completed"],
        1
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["scheduler_decision"]["action"]["kind"],
        "continue"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["scheduler_decision"]
            ["post_fan_in_captain_decision"]["precedence"],
        "review_gate"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["consumed_worker_result_envelope"]["schema"],
        "ccc.consumed_worker_result_envelope.v1"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["consumed_worker_result_envelope"]["source"],
        "current_task_card.worker_result_envelope"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["consumed_worker_result_envelope"]
            ["worker_result_envelope"]["risk"],
        "low"
    );
    assert_eq!(
        second_response["result"]["structuredContent"]["scheduler_decision"]
            ["consumed_worker_result_envelope"]["worker_result_envelope"]["checks"][0],
        "cargo test --lib"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(run_record["active_agent_id"], "captain");
    assert_eq!(run_record["active_thread_id"], "thread-rust-test");
    assert_eq!(
        run_record["raw_thread_ids"].as_array().map(Vec::len),
        Some(1)
    );
    let attempt_payload = read_json_document(
        &run_directory
            .join("orchestration")
            .join("attempts")
            .join("attempt-0002.json"),
    )
    .expect("attempt payload");
    assert_eq!(
        attempt_payload["consumed_worker_result_envelope"]["captain_consumed_for_decision"],
        true
    );
    assert_eq!(
        attempt_payload["scheduler_decision"]["consumed_worker_result_envelope"]["decision"]
            ["stop_reason"],
        "collapsed_fan_in"
    );
    assert_eq!(
        attempt_payload["post_fan_in_captain_decision"]["precedence"],
        "review_gate"
    );
    assert_eq!(
        attempt_payload["scheduler_decision"]["post_fan_in_captain_decision"]["precedence"],
        "review_gate"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_completed_host_subagent_and_opens_fan_in() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-update");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-subagent");
    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    task_card["delegation_plan"] = create_specialist_delegation_plan_with_runtime(
        "code specialist",
        &json!({
            "summary": "Bounded code mutation.",
            "model": "gpt-5.3-codex",
            "variant": "high",
            "fast_mode": true,
        }),
        &json!({
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec",
        }),
        "workspace-write",
        "Raider performs bounded mutation work.",
    );
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let update_payload = create_ccc_subagent_update_payload(&json!({
        "run_id": "run-subagent",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-subagent-1",
        "status": "completed",
        "summary": "Raider returned bounded implementation evidence to captain.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/lib.rs:10", "README.md:3"],
        "next_action": "captain_merge",
        "open_questions": ["Confirm the auth owner."],
        "confidence": "medium",
        "risk": "low",
        "checks": ["cargo test --lib"],
        "observed_model": "gpt-5.3-codex",
        "observed_variant": "high",
        "observed_sandbox_mode": "workspace-write"
    }))
    .expect("subagent update");
    assert_eq!(update_payload["subagent_status"], "completed");
    assert_eq!(update_payload["thread_id"], "thread-subagent-1");
    assert_eq!(update_payload["active_handle_cleanup"]["state"], "released");
    assert_eq!(
        update_payload["active_handle_cleanup"]["host_close_required"],
        true
    );
    assert_eq!(
        update_payload["active_handle_cleanup"]["host_close_status"],
        "host_action_required"
    );
    assert_eq!(
        update_payload["active_handle_cleanup"]["host_close_action"],
        "close_agent"
    );
    assert!(update_payload["active_handle_cleanup"]["host_close_reason"]
        .as_str()
        .expect("host close reason")
        .contains("host close_agent is still required for ccc_raider"));

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-subagent",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(status_payload["next_step"], "await_fan_in");
    assert_eq!(status_payload["can_advance"], true);
    assert_eq!(status_payload["host_subagent_state"]["fan_in_ready"], true);
    assert_eq!(
        status_payload["execution_strategy"]["host_subagent_update_mode"],
        "ccc_cli_subcommand"
    );
    assert_eq!(
        status_payload["execution_strategy"]["operator_visible_transport"]["preferred_transport"],
        "ccc_cli_quiet_subcommand"
    );
    assert_eq!(
        status_payload["execution_strategy"]["operator_visible_transport"]["transcript_signal"],
        "ran"
    );
    assert_eq!(
        status_payload["execution_strategy"]["operator_visible_transport"]
            ["preferred_command_shapes"]["subagent_update"][0],
        "ccc subagent-update --quiet --json '{...}'"
    );
    assert_eq!(
        status_payload["execution_strategy"]["operator_visible_transport"]
            ["default_payload_transport"],
        "inline_json"
    );
    let status_longway_visibility = status_payload["execution_strategy"]
        ["operator_visible_transport"]["longway_visibility"]
        .as_str()
        .expect("LongWay visibility guidance");
    assert!(status_longway_visibility.contains("CCC_LONGWAY_PROJECTION.md"));
    assert!(status_longway_visibility.contains("ccc status --projection --json '{...}'"));
    assert!(!status_longway_visibility.contains("ccc checklist --text"));
    assert_eq!(
        status_payload["execution_strategy"]["operator_visible_transport"]
            ["preferred_command_shapes"]["orchestrate"][0],
        "ccc orchestrate --quiet --json '{...}'"
    );
    assert_eq!(
        status_payload["execution_strategy"]["operator_visible_transport"]["mcp_reserved_for"],
        json!(["app surfaces", "structured inspection", "CLI unavailable"])
    );
    assert_eq!(
        status_payload["execution_strategy"]["codex_exec_fallback_allowed"],
        false
    );
    assert_eq!(
        status_payload["captain_action_contract"]["preflight_guard"],
        "ccc_recommend_entry"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["allowed_action"],
        "captain_advance"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["direct_mutation_allowed"],
        false
    );
    assert_eq!(
        status_payload["captain_action_contract"]["direct_file_mutation_policy"]["allowed"],
        false
    );
    assert_eq!(
        status_payload["captain_action_contract"]["direct_file_mutation_policy"]["applies_to"],
        json!([
            "apply_patch",
            "direct_shell_file_mutation",
            "file_edits",
            "mutation_commands"
        ])
    );
    assert_eq!(
        status_payload["captain_action_contract"]["direct_file_mutation_policy"]
            ["requires_recorded_exception"],
        "explicit_terminal_fallback_or_operator_override"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["direct_file_mutation_policy"]["required_route"],
        "specialist_fan_in_then_captain_review_merge"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["direct_file_mutation_policy"]["merge_gate"],
        "specialist_fan_in_or_explicit_operator_override"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["pending_merge_count"],
        1
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_policy_drift"]["ok"],
        true
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["evidence_paths"][0],
        "src/lib.rs:10"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["schema"],
        "ccc.worker_result_envelope.v1"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["risk"],
        "low"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["checks"][0],
        "cargo test --lib"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["contract"]
            ["captain_consumes_compact_fan_in"],
        true
    );
    assert_eq!(
        status_payload["current_task_card"]["worker_result_envelope"]["schema"],
        "ccc.worker_result_envelope.v1"
    );
    assert_eq!(
        status_payload["current_task_card"]["review_fan_in"],
        Value::Null
    );
    assert_eq!(status_payload["active_agent_id"], "captain");
    assert_eq!(status_payload["active_thread_id"], Value::Null);
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["state"],
        "released"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_required"],
        true
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_status"],
        "host_action_required"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_action"],
        "close_agent"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_target"],
        "ccc_raider"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["released_handle_count"],
        1
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["latest_released_handle"]
            ["thread_id"],
        "thread-subagent-1"
    );
    assert_eq!(status_payload["run_state"]["event_count"], 4);
    assert_eq!(status_payload["run_state"]["last_event_id"], "event-0004");

    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Transport: prefer ccc_cli_quiet_subcommand (ran)"));
    assert!(text.contains("start,orchestrate,subagent-update,memory"));
    assert!(text.contains("reserve MCP for app surfaces,structured inspection,CLI unavailable"));
    assert!(text.contains("Captain Guard: allowed=captain-advance"));
    assert!(text.contains("preflight=internal preflight"));
    assert!(text.contains(
        "Host Handles: released CCC handle; host close_agent still required for ccc_raider"
    ));

    let events = fs::read_to_string(
        workspace_dir
            .join(".ccc")
            .join("runs")
            .join("run-subagent")
            .join("events.jsonl"),
    )
    .expect("events");
    assert!(events.contains("\"event\":\"subagent_updated\""));
    assert!(events.contains("\"event_id\":\"event-0004\""));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_persists_long_fan_in_artifact_for_compact_ref_mode() {
    let workspace_dir = create_temp_path("subagent-update-compact-ref");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_id = "run-compact-ref";
    let run_directory = write_test_run_fixture(&workspace_dir, run_id);
    let long_summary = "Long fan-in detail. ".repeat(120);
    let evidence_paths = (0..18)
        .map(|index| json!(format!("src/file_{index}.rs:{}", index + 1)))
        .collect::<Vec<_>>();

    let parsed = parse_ccc_subagent_update_arguments(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "run_id": run_id,
        "task_card_id": "task-1",
        "status": "completed",
        "summary": long_summary,
        "fan_in_status": "completed",
        "evidence_paths": evidence_paths,
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high",
        "event_ref": "fan-in-ref-1",
        "mode": "compact"
    }))
    .expect("parse compact-ref update");
    let update_payload =
        create_ccc_subagent_update_payload(&parsed).expect("record compact-ref update");
    let artifact_ref = update_payload["fan_in_artifact"].clone();
    let artifact_path = PathBuf::from(
        artifact_ref["path"]
            .as_str()
            .expect("artifact path should be recorded"),
    );
    let artifact = read_json_document(&artifact_path).expect("read fan-in artifact");
    let task_card = read_json_document(&run_directory.join("task-cards").join("task-1.json"))
        .expect("read task card");

    assert_eq!(update_payload["response_mode"], "compact");
    assert_eq!(artifact_ref["event_ref"], "fan-in-ref-1");
    let normalized_long_summary = long_summary.trim();
    assert_eq!(
        artifact["fan_in"]["summary"]
            .as_str()
            .expect("artifact should preserve full summary"),
        normalized_long_summary
    );
    assert_eq!(task_card["subagent_fan_in"]["artifact_ref"], artifact_ref);
    assert!(
        task_card["subagent_fan_in"]["summary"]
            .as_str()
            .expect("compacted summary")
            .len()
            < normalized_long_summary.len()
    );
    assert_eq!(
        task_card["subagent_fan_in"]["evidence_paths"]
            .as_array()
            .expect("compact evidence")
            .len(),
        SUBAGENT_FAN_IN_INLINE_ITEMS + 1
    );

    let compact_status = create_ccc_status_compact_payload(
        &create_ccc_status_payload(
            &create_session_context(),
            &resolve_run_locator_arguments(
                &json!({"cwd": workspace_dir.to_string_lossy(), "run_id": run_id}),
                "ccc_status",
            )
            .expect("locator"),
        )
        .expect("status payload"),
    );
    assert_eq!(
        compact_status["current_task_card"]["subagent_fan_in"]["artifact_ref"]["event_ref"],
        "fan-in-ref-1"
    );

    let tool_content = crate::mcp_tools::create_subagent_update_tool_structured_content(
        &update_payload,
        &json!({"next_step": "advance", "can_advance": true}),
    );
    assert_eq!(tool_content["mode"], "compact");
    assert_eq!(tool_content["fan_in_artifact"]["event_ref"], "fan-in-ref-1");
    assert!(tool_content.get("current_task_card").is_none());

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_tool_structured_content_stays_operator_compact_by_default() {
    let update_payload = json!({
        "cwd": "/tmp/work",
        "run_id": "run-compact-output",
        "run_directory": "/tmp/work/.ccc/runs/run-compact-output",
        "run_ref": "ccc-run:/tmp/work/.ccc/runs/run-compact-output",
        "task_card_id": "task-1",
        "child_agent_id": "ccc_raider",
        "subagent_status": "completed",
        "summary": "Raider completed the bounded change.",
        "fan_in": {
            "status": "completed",
            "summary": "Bounded change complete.",
            "evidence_paths": ["src/main.rs:1"],
            "next_action": "captain_verify",
            "open_questions": [],
            "confidence": "high",
            "verbose_internal_payload": {
                "large": true
            }
        }
    });
    let status_payload = json!({
        "next_step": "advance",
        "can_advance": true,
        "longway": {
            "completed_phase_count": 1,
            "phase_count": 3,
            "current_item": "item-2",
            "active_phase_name": "verify",
            "active_phase_status": "pending",
            "planned_row_count": 2
        },
        "current_task_card": {
            "task_card_id": "task-1",
            "title": "Implement compact output",
            "assigned_role": "code specialist",
            "assigned_agent_id": "raider",
            "status": "completed",
            "verification_state": "pending",
            "delegation_plan": {
                "large": "internal details should not be echoed"
            }
        },
        "host_subagent_state": {
            "subagent_activity": [{
                "child_agent_id": "ccc_raider",
                "assigned_role": "code specialist",
                "status": "completed",
                "task_title": "Implement compact output",
                "next_action": "await_fan_in",
                "verbose_internal_payload": true
            }]
        },
        "captain_action_contract": {
            "allowed_action": "spawn_subagent",
            "required_action": "spawn_or_record_specialist",
            "direct_finish_allowed": false,
            "direct_mutation_allowed": false,
            "denied_action_reason": "Current task requires specialist execution before direct captain finish or mutation.",
            "preflight_guard": "ccc_recommend_entry"
        },
        "run_state": {
            "large": "internal state should not be echoed"
        },
        "server_identity": {
            "large": "server state should not be echoed"
        }
    });

    let tool_content = crate::mcp_tools::create_subagent_update_tool_structured_content(
        &update_payload,
        &status_payload,
    );

    assert_eq!(
        tool_content["current_task"]["title"],
        "Implement compact output"
    );
    assert_eq!(tool_content["longway"]["completed"], 1);
    assert_eq!(tool_content["agents"][0]["child_agent_id"], "ccc_raider");
    assert_eq!(
        tool_content["fan_in"]["summary"],
        "Bounded change complete."
    );
    assert_eq!(
        tool_content["captain_guard"]["required_action"],
        "spawn_or_record_specialist"
    );
    assert_eq!(
        tool_content["captain_guard"]["direct_mutation_allowed"],
        false
    );
    assert_eq!(
        tool_content["captain_guard"]["direct_file_mutation_policy"]["allowed"],
        false
    );
    assert_eq!(
        tool_content["captain_guard"]["direct_file_mutation_policy"]["applies_to"],
        json!([
            "apply_patch",
            "direct_shell_file_mutation",
            "file_edits",
            "mutation_commands"
        ])
    );
    assert_eq!(
        tool_content["captain_guard"]["direct_file_mutation_policy"]["requires_recorded_exception"],
        "explicit_terminal_fallback_or_operator_override"
    );
    assert_eq!(
        tool_content["captain_guard"]["direct_file_mutation_policy"]["required_route"],
        "specialist_fan_in_then_captain_review_merge"
    );
    assert_eq!(
        tool_content["captain_guard"]["direct_file_mutation_policy"]["merge_gate"],
        "specialist_fan_in_or_explicit_operator_override"
    );
    assert_eq!(
        tool_content["captain_guard"]["preferred_operator_transport"],
        "ccc_cli_quiet_subcommand"
    );
    assert_eq!(
        tool_content["captain_guard"]["mcp_tool_call_policy"],
        "reserve_for_app_or_structured_inspection_or_cli_unavailable"
    );
    assert!(tool_content.get("current_task_card").is_none());
    assert!(tool_content.get("host_subagent_state").is_none());
    assert!(tool_content.get("run_state").is_none());
    assert!(tool_content.get("server_identity").is_none());
    assert!(tool_content["fan_in"]
        .get("verbose_internal_payload")
        .is_none());
}

#[test]
fn ccc_subagent_update_compact_ref_loads_pre_persisted_fan_in() {
    let workspace_dir = create_temp_path("subagent-update-compact-ref-load");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_id = "run-compact-ref-load";
    let run_directory = write_test_run_fixture(&workspace_dir, run_id);
    let artifact_path = run_directory
        .join("temp-artifacts")
        .join("subagent-update")
        .join("loaded-ref.json");
    write_json_document(
        &artifact_path,
        &json!({
            "kind": "subagent_update_fan_in",
            "event_ref": "loaded-ref",
            "fan_in": {
                "status": "completed",
                "summary": "Loaded fan-in from a pre-persisted artifact.",
                "evidence_paths": ["artifact/evidence.md:1"],
                "next_action": "captain_merge",
                "open_questions": [],
                "confidence": "high"
            }
        }),
    )
    .expect("write fan-in artifact");

    let parsed = parse_ccc_subagent_update_arguments(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "run_id": run_id,
        "task_card_id": "task-1",
        "status": "completed",
        "event_ref": "loaded-ref",
        "mode": "compact"
    }))
    .expect("parse compact-ref update");
    let update_payload =
        create_ccc_subagent_update_payload(&parsed).expect("record compact-ref update");
    let task_card = read_json_document(&run_directory.join("task-cards").join("task-1.json"))
        .expect("read task card");

    assert_eq!(update_payload["response_mode"], "compact");
    assert_eq!(
        task_card["subagent_fan_in"]["summary"],
        "Loaded fan-in from a pre-persisted artifact."
    );
    assert_eq!(
        task_card["subagent_fan_in"]["evidence_paths"][0],
        "artifact/evidence.md:1"
    );
    assert_eq!(update_payload["fan_in_artifact"]["event_ref"], "loaded-ref");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_prefers_retry_for_failed_subagent_before_degraded_fallback() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("failed-subagent-retry-first");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Recover a failed host subagent",
        "title": "Retry failed subagent",
        "intent": "Show recovery before degraded fallback",
        "scope": "One bounded subagent failure",
        "acceptance": "Status prefers retry or reassign before fallback",
        "prompt": "Return a failed subagent checkpoint.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-failed-retry",
        "status": "failed",
        "summary": "Host subagent failed before clean fan-in.",
        "fan_in_status": "failed",
        "evidence_paths": [],
        "next_action": "retry",
        "open_questions": ["Retry same specialist or reassign."],
        "confidence": "medium"
    }))
    .expect("failed subagent update");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["host_subagent_state"]["recovery_recommendation"]["recommended_action"],
        "retry"
    );
    assert_eq!(
        status_payload["recovery_lane"]["source"],
        "host_subagent_state"
    );
    assert_eq!(
        status_payload["recovery_lane"]["status"],
        "recovery_pending"
    );
    assert_eq!(
        status_payload["recovery_lane"]["recommended_action"],
        "retry"
    );
    assert_eq!(
        status_payload["recovery_lane"]["needs_operator_attention"],
        true
    );
    assert_eq!(
        status_payload["host_subagent_state"]["recovery_recommendation"]
            ["prefer_before_degraded_fallback"],
        true
    );
    assert_eq!(
        status_payload["execution_strategy"]["codex_exec_fallback_allowed"],
        false
    );
    assert_eq!(
        status_payload["captain_action_contract"]["allowed_action"],
        "recover_subagent"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["required_action"],
        "ccc_orchestrate"
    );
    assert_eq!(
        status_payload["lifecycle_hooks"]["schema"],
        "ccc.lifecycle_hook_tiers.v1"
    );
    assert_eq!(status_payload["lifecycle_hooks"]["public_commands"], false);
    assert!(status_payload["lifecycle_hooks"]["tiers"]
        .as_array()
        .expect("hook tier definitions")
        .iter()
        .any(|tier| tier["tier"] == "tool_guard" && tier["public_command"] == false));
    assert!(status_payload["lifecycle_hooks"]["active_tiers"]
        .as_array()
        .expect("active hook tiers")
        .iter()
        .any(|tier| tier == "recovery"));
    assert!(status_payload["lifecycle_hooks"]["active_tiers"]
        .as_array()
        .expect("active hook tiers")
        .iter()
        .any(|tier| tier == "notification"));
    assert!(status_payload["lifecycle_hooks"]["decisions"]
        .as_array()
        .expect("hook decisions")
        .iter()
        .any(|decision| decision["tier"] == "recovery"
            && decision["status"] == "decision"
            && decision["affects"]
                .as_array()
                .expect("affects")
                .contains(&json!("routing"))));

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["host_subagent_state"]["recovery_recommendation"]["recommended_action"],
        "retry"
    );
    assert_eq!(compact["recovery_lane"]["recommended_action"], "retry");
    assert_eq!(
        compact["app_panel"]["recovery_lane"]["status"],
        "recovery_pending"
    );
    assert_eq!(
        compact["captain_action_contract"]["allowed_action"],
        "recover_subagent"
    );
    assert_eq!(
        compact["lifecycle_hooks"]["status"],
        status_payload["lifecycle_hooks"]["status"]
    );
    assert_eq!(
        compact["app_panel"]["lifecycle_hooks"]["status"],
        status_payload["lifecycle_hooks"]["status"]
    );
    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains("Recovery: status=recovery-pending action=retry"));
    assert!(status_text.contains("Lifecycle Hooks:"));
    assert!(status_text.contains("active="));
    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Recovery:"));
    assert!(app_panel_text.contains("status=recovery-pending action=retry"));
    assert!(app_panel_text.contains("Lifecycle Hooks:"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn lifecycle_hook_tiers_reject_user_facing_hook_commands() {
    let payload = lifecycle_hooks::create_lifecycle_hook_tiers_payload(
        &json!({
            "lifecycle_hooks": {
                "tool_guard": {
                    "command": "ccc hook run"
                }
            }
        }),
        &Value::Null,
        &Value::Null,
        &json!({ "fan_in_ready": false }),
        &Value::Null,
        &Value::Null,
        &Value::Null,
        &json!({ "state": "blocked_unrecorded_direct_mutation" }),
        &Value::Null,
    );

    assert_eq!(payload["status"], "failed");
    assert_eq!(payload["failure_count"], 1);
    assert!(payload["failed_tiers"]
        .as_array()
        .expect("failed tiers")
        .contains(&json!("tool_guard")));
    assert!(payload["decisions"]
        .as_array()
        .expect("hook decisions")
        .iter()
        .any(|decision| decision["tier"] == "tool_guard"
            && decision["status"] == "failed"
            && decision["action"] == "user_facing_hook_commands_unsupported"));
}

#[test]
fn ccc_status_prefers_review_needs_work_before_recovery_retry_decision() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("review-before-recovery-retry");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Recover a failed host subagent with review findings",
        "title": "Review before recovery",
        "intent": "Show review needs-work precedence over retry recovery",
        "scope": "One bounded subagent failure with review fan-in",
        "acceptance": "Status selects review repair before retry recovery",
        "prompt": "Return a failed subagent checkpoint with review findings.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-review-before-recovery",
        "status": "failed",
        "summary": "Host subagent failed after review found a repair.",
        "fan_in_status": "failed",
        "evidence_paths": [],
        "next_action": "retry",
        "open_questions": ["Retry after review repair decision."],
        "confidence": "medium"
    }))
    .expect("failed subagent update");

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["review_policy"] = json!({
        "state": "needs_work",
        "decision": "required",
        "summary": "Review requires a repair before recovery retry."
    });
    task_card["review_fan_in"] = json!({
        "status": "needs_work",
        "outcome": "needs_work",
        "captain_next_decision": "captain_repair",
        "unresolved_finding_count": 1,
        "unresolved_findings": ["Repair review finding before retry fallback."]
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["recovery_lane"]["recommended_action"],
        "retry"
    );
    assert_eq!(
        status_payload["post_fan_in_captain_decision"]["precedence"],
        "review_needs_work"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["allowed_action"],
        "review_repair_required"
    );
    assert_eq!(
        status_payload["scheduler"]["action"]["captain_decision_precedence"],
        "review_needs_work"
    );
    assert_eq!(
        status_payload["state_contract"]["active_gate"],
        "review_gate"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_dispatches_codex_exec_after_stalled_host_subagent() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("stalled-host-subagent-codex-exec-recovery");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable(&workspace_dir);
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Repair release install and asset packaging scripts through harness execution",
        "title": "Repair release installer packaging through fallback",
        "intent": "Route release/install repair to raider, then use codex exec fallback after stalled host fan-in",
        "scope": "Fix install.sh, install.ps1, and scripts/release/build-release-asset.sh only.",
        "acceptance": "A stalled host raider records fallback and codex exec returns mutation fan-in.",
        "prompt": "Fix install.sh, install.ps1, and scripts/release/build-release-asset.sh for the GitHub release asset packaging repair.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let task_card_id = start_payload["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "task_card_id": task_card_id,
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-stalled-raider",
        "status": "stalled",
        "summary": "Host raider did not return bounded fan-in.",
        "fan_in_status": "stalled",
        "evidence_paths": [],
        "next_action": "retry",
        "open_questions": [],
        "confidence": "medium"
    }))
    .expect("stalled subagent update");

    let task_card = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{task_card_id}.json")),
    )
    .expect("task card");
    assert_eq!(task_card["assigned_agent_id"], "raider");
    assert_eq!(task_card["subagent_fallback"]["reason"], "child_timeout");
    assert_eq!(task_card["subagent_lifecycle"]["status"], "stalled");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 242,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        response.get("error").is_none(),
        "unexpected orchestrate response: {response:?}"
    );

    let payload = &response["result"]["structuredContent"];
    assert_eq!(payload["next_step"], "await_fan_in");
    assert_eq!(payload["launch_result"]["child_agent_id"], "raider");
    assert_eq!(payload["launch_result"]["terminal_status"], "completed");
    assert_eq!(payload["scheduler_decision"]["action"]["kind"], "dispatch");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_terminal_status_releases_active_handle_without_accumulating_active_refs() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-terminal-cleanup");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Verify terminal subagent cleanup",
        "title": "Release terminal host handles",
        "intent": "Record repeated terminal updates without retaining active host handles",
        "scope": "One bounded subagent cleanup regression",
        "acceptance": "Terminal updates clear active thread and leave only archived handle history",
        "prompt": "Return terminal subagent updates.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-terminal-1",
        "status": "completed",
        "summary": "First terminal result returned.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:10"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("first terminal update");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-terminal-2",
        "status": "completed",
        "summary": "Repeated terminal result returned.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:20"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "medium"
    }))
    .expect("second terminal update");

    let repeated_update = create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-terminal-2",
        "status": "completed",
        "summary": "Repeated terminal result returned again.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:30"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "medium"
    }))
    .expect("idempotent terminal update");
    assert_eq!(
        repeated_update["active_handle_cleanup"]["state"],
        "already_clear"
    );
    assert_eq!(
        repeated_update["active_handle_cleanup"]["host_close_required"],
        false
    );
    assert_eq!(
        repeated_update["active_handle_cleanup"]["host_close_status"],
        "not_required"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(run_record["active_agent_id"], "captain");
    assert_eq!(run_record["active_thread_id"], Value::Null);
    assert_eq!(
        run_record["host_subagent_handle_archive"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(run_record["child_agents"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        run_record["specialist_executors"].as_array().map(Vec::len),
        Some(1)
    );

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["host_subagent_state"]["active_subagent_count"],
        0
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_subagents"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["released_handle_count"],
        2
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_required"],
        true
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_status"],
        "host_action_required"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["active_thread_id"],
        Value::Null
    );
    assert_eq!(
        status_payload["host_subagent_state"]["active_handle_cleanup"]["latest_released_handle"]
            ["thread_id"],
        "thread-terminal-2"
    );

    let text = create_ccc_status_text(&status_payload);
    assert!(text
        .contains("Host Handles: released CCC handle; host close_agent still required for raider"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_terminal_cleanup_stress_releases_failed_stalled_merged_reclaimed_handles() {
    let session_context = create_session_context();

    for terminal_status in ["failed", "stalled", "merged", "reclaimed"] {
        let workspace_dir =
            create_temp_path(&format!("subagent-terminal-cleanup-{terminal_status}"));
        create_dir_all(&workspace_dir).expect("create workspace");
        let start_payload = create_ccc_start_payload(&json!({
                "cwd": workspace_dir.to_string_lossy(),
                "goal": format!("Verify {terminal_status} terminal cleanup"),
                "title": "Release repeated terminal host handles",
                "intent": "Record repeated terminal updates without retaining active host handles",
                "scope": "One bounded subagent cleanup stress regression",
                "acceptance": "Terminal updates clear active thread and keep archived handle history visible",
                "prompt": "Return repeated terminal subagent updates.",
                "task_kind": "execution"
            }))
            .expect("start payload");
        let run_id = start_payload["run_id"].as_str().expect("run id");
        let run_directory = PathBuf::from(
            start_payload["run_directory"]
                .as_str()
                .expect("run directory"),
        );

        for update_index in 1..=3 {
            let thread_id = format!("thread-{terminal_status}-{update_index}");
            let update_payload = create_ccc_subagent_update_payload(&json!({
                "run_id": run_id,
                "cwd": workspace_dir.to_string_lossy(),
                "child_agent_id": "raider",
                "thread_id": thread_id,
                "status": terminal_status,
                "summary": format!("{terminal_status} terminal update {update_index}."),
                "fan_in_status": terminal_status,
                "evidence_paths": [format!("src/main.rs:{update_index}")],
                "next_action": "captain_merge",
                "open_questions": [],
                "confidence": "medium"
            }))
            .expect("terminal update");

            assert_eq!(update_payload["subagent_status"], terminal_status);
            assert_eq!(update_payload["active_handle_cleanup"]["state"], "released");
            assert_eq!(
                update_payload["active_handle_cleanup"]["host_close_required"],
                true
            );
            assert_eq!(
                update_payload["active_handle_cleanup"]["thread_id"],
                format!("thread-{terminal_status}-{update_index}")
            );
        }

        let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
        assert_eq!(run_record["active_agent_id"], "captain");
        assert_eq!(run_record["active_thread_id"], Value::Null);
        assert_eq!(
            run_record["raw_thread_ids"].as_array().map(Vec::len),
            Some(3)
        );
        assert_eq!(
            run_record["host_subagent_handle_archive"]
                .as_array()
                .map(Vec::len),
            Some(3)
        );
        assert_eq!(run_record["child_agents"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            run_record["specialist_executors"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            run_record["host_subagent_handle_archive"][2]["thread_id"],
            format!("thread-{terminal_status}-3")
        );

        let locator = resolve_run_locator_arguments(
            &json!({
                "run_id": run_id,
                "cwd": workspace_dir.to_string_lossy(),
            }),
            "ccc_status",
        )
        .expect("locator");
        let status_payload =
            create_ccc_status_payload(&session_context, &locator).expect("status payload");
        assert_eq!(
            status_payload["host_subagent_state"]["active_subagent_count"],
            0
        );
        assert_eq!(
            status_payload["host_subagent_state"]["active_subagents"]
                .as_array()
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            status_payload["host_subagent_state"]["active_handle_cleanup"]["released_handle_count"],
            3
        );
        assert_eq!(
            status_payload["host_subagent_state"]["active_handle_cleanup"]["host_close_required"],
            true
        );
        assert_eq!(
            status_payload["host_subagent_state"]["active_handle_cleanup"]["active_thread_id"],
            Value::Null
        );
        assert_eq!(
            status_payload["host_subagent_state"]["active_handle_cleanup"]
                ["latest_released_handle"]["thread_id"],
            format!("thread-{terminal_status}-3")
        );

        let _ = fs::remove_dir_all(&workspace_dir);
    }
}

#[test]
fn ccc_status_exposes_host_subagent_reclaim_replan_visibility_for_late_lanes() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-host-subagent-replan");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-host-subagent-replan");
    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file).expect("read run");
    run_record["child_agents"] = json!([
        {
            "agent_id": "ccc_scout",
            "parent_agent_id": "captain",
            "role": "explorer",
            "status": "running",
            "task_card_id": "task-1",
            "lane_id": "scout-a",
            "thread_id": "thread-host-scout-a",
            "summary": "Scout lane A is still processing.",
            "updated_at": "2020-01-01T00:00:00.000Z",
            "created_at": "2020-01-01T00:00:00.000Z"
        }
    ]);
    write_json_document(&run_file, &run_record).expect("write run");

    let run_state_file = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_file).expect("read run-state");
    run_state["next_action"] = json!({ "command": "await_fan_in" });
    write_json_document(&run_state_file, &run_state).expect("write run-state");

    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("read task-card");
    task_card["task_kind"] = json!("explore");
    task_card["assigned_role"] = json!("explorer");
    task_card["assigned_agent_id"] = json!("scout");
    task_card["parallel_fanout"] = json!({
        "mode": "parallel",
        "required_lane_ids": ["scout-a", "scout-b"],
        "all_lane_ids": ["scout-a", "scout-b", "scout-c", "scout-d"],
        "lanes": [
            { "lane_id": "scout-a", "required": true, "scope": null, "lifecycle": { "status": "running" }, "fan_in": null },
            { "lane_id": "scout-b", "required": true, "scope": null, "lifecycle": null, "fan_in": null }
        ],
        "aggregate": {
            "required_lane_count": 2,
            "active_lane_count": 1,
            "terminal_lane_count": 0,
            "fan_in_ready": false
        }
    });
    write_json_document(&task_card_file, &task_card).expect("write task-card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-host-subagent-replan",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["host_subagent_state"]["active_subagent_count"],
        1
    );
    assert_eq!(
        status_payload["host_subagent_state"]["parallel_lane_state"]["active_lane_count"],
        1
    );
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]
            ["cancellation_supported"],
        false
    );
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]
            ["recommended_action"],
        "reclaim_or_replan"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["recovery_recommendation"]["recommended_action"],
        "reclaim"
    );
    assert_eq!(
        status_payload["recovery_lane"]["source"],
        "host_subagent_state"
    );
    assert_eq!(status_payload["recovery_lane"]["status"], "reclaim_pending");
    assert_eq!(
        status_payload["recovery_lane"]["recommended_action"],
        "reclaim"
    );
    assert_eq!(
        status_payload["recovery_lane"]["reclaim_replan_action"],
        "reclaim_or_replan"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["allowed_action"],
        "reclaim_subagent"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["required_action"],
        "ccc_subagent_update"
    );
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]["targets"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(status_payload["scheduler"]["state"], "await_fan_in");
    assert_eq!(
        status_payload["scheduler"]["decision_source"],
        "bounded_parallel_fanout"
    );
    assert_eq!(
        status_payload["scheduler"]["action"]["kind"],
        "await_parallel_fan_in"
    );
    assert_eq!(
        status_payload["scheduler"]["parallel"]["required_lane_ids"],
        json!(["scout-a", "scout-b"])
    );
    assert!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("cannot cancel host custom subagents directly")
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["host_subagent_state"]["reclaim_replan_recommendation"]["recommended_action"],
        "reclaim_or_replan"
    );
    assert_eq!(
        compact["host_subagent_state"]["reclaim_replan_recommendation"]["needs_operator_attention"],
        true
    );
    assert_eq!(
        compact["host_subagent_state"]["recovery_recommendation"]["recommended_action"],
        "reclaim"
    );
    assert_eq!(
        compact["recovery_lane"]["reclaim_replan_action"],
        "reclaim_or_replan"
    );
    assert_eq!(
        compact["app_panel"]["recovery_lane"]["status"],
        "reclaim_pending"
    );
    assert_eq!(
        compact["captain_action_contract"]["allowed_action"],
        "reclaim_subagent"
    );
    assert_eq!(
        compact["captain_action_contract"]["subagent_capacity_policy"]["fallback_reason"],
        "host_subagent_thread_limit"
    );
    assert_eq!(
        compact["scheduler"]["action"]["kind"],
        "await_parallel_fan_in"
    );
    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains("Recovery: status=reclaim-pending action=reclaim"));
    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Recovery:"));
    assert!(app_panel_text.contains("status=reclaim-pending action=reclaim"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn host_subagent_state_counts_active_provider_and_model_capacity() {
    let payload = crate::host_subagent_lifecycle::create_host_subagent_state_payload(
        &json!({
            "child_agents": [
                {
                    "agent_id": "ccc_raider",
                    "role": "code specialist",
                    "status": "running",
                    "task_card_id": "task-1",
                    "provider": "openai",
                    "model": "gpt-5.5"
                },
                {
                    "agent_id": "ccc_arbiter",
                    "role": "verifier",
                    "status": "acknowledged",
                    "task_card_id": "task-1",
                    "provider": "openai",
                    "model": "gpt-5.5"
                },
                {
                    "agent_id": "external-reviewer",
                    "role": "verifier",
                    "status": "spawned",
                    "task_card_id": "task-1",
                    "provider": "anthropic",
                    "model": "claude-sonnet-4.5"
                },
                {
                    "agent_id": "ccc_scout",
                    "role": "explorer",
                    "status": "running",
                    "task_card_id": "task-2",
                    "provider": "openai",
                    "model": "gpt-5.5"
                },
                {
                    "agent_id": "ccc_scribe",
                    "role": "documenter",
                    "status": "completed",
                    "task_card_id": "task-1",
                    "provider": "openai",
                    "model": "gpt-5.5"
                }
            ]
        }),
        &json!({
            "task_card_id": "task-1",
            "title": "Count provider and model capacity"
        }),
        Some("task-1"),
        &json!({
            "host_subagent_provider_concurrency_limits": {
                "openai": 1,
                "anthropic": 2
            },
            "host_subagent_model_concurrency_limits": {
                "gpt-5.5": 3,
                "claude-sonnet-4.5": 1
            }
        }),
    );

    assert_eq!(payload["active_subagent_count"], 3);
    assert_eq!(
        payload["concurrency"]["schema"],
        "ccc.host_subagent_concurrency.v1"
    );
    assert_eq!(payload["concurrency"]["active_count"], 4);
    assert_eq!(
        payload["concurrency"]["per_provider"]["openai"]["active_count"],
        3
    );
    assert_eq!(payload["concurrency"]["per_provider"]["openai"]["limit"], 1);
    assert_eq!(
        payload["concurrency"]["per_provider"]["openai"]["remaining_capacity"],
        0
    );
    assert_eq!(
        payload["concurrency"]["per_provider"]["openai"]["exceeded"],
        true
    );
    assert_eq!(
        payload["concurrency"]["per_provider"]["anthropic"]["remaining_capacity"],
        1
    );
    assert_eq!(
        payload["concurrency"]["per_model"]["gpt-5.5"]["active_count"],
        3
    );
    assert_eq!(payload["concurrency"]["per_model"]["gpt-5.5"]["limit"], 3);
    assert_eq!(
        payload["concurrency"]["per_model"]["claude-sonnet-4.5"]["exceeded"],
        false
    );
    assert_eq!(payload["concurrency"]["provider_exceeded"], true);
    assert_eq!(payload["concurrency"]["model_exceeded"], false);
    assert_eq!(payload["concurrency"]["exceeded"], true);
}

#[test]
fn host_subagent_state_exposes_default_capacity_and_threshold_visibility() {
    let payload = crate::host_subagent_lifecycle::create_host_subagent_state_payload(
        &json!({
            "child_agents": [
                {
                    "agent_id": "ccc_scribe",
                    "role": "documenter",
                    "status": "running",
                    "task_card_id": "task-1"
                }
            ]
        }),
        &json!({
            "task_card_id": "task-1",
            "title": "Expose default capacity",
            "role_config_snapshot": {
                "model": "gpt-5.4-mini"
            }
        }),
        Some("task-1"),
        &json!({}),
    );

    assert_eq!(payload["concurrency"]["default_provider_limit"], 4);
    assert_eq!(
        payload["concurrency"]["default_provider_limit_source"],
        "default"
    );
    assert_eq!(payload["concurrency"]["default_model_limit"], 2);
    assert_eq!(
        payload["concurrency"]["default_model_limit_source"],
        "default"
    );
    assert_eq!(
        payload["concurrency"]["per_provider"]["openai"]["active_count"],
        1
    );
    assert_eq!(
        payload["concurrency"]["per_provider"]["openai"]["remaining_capacity"],
        3
    );
    assert_eq!(
        payload["concurrency"]["per_provider"]["openai"]["limit_source"],
        "default"
    );
    assert_eq!(
        payload["concurrency"]["per_model"]["gpt-5.4-mini"]["active_count"],
        1
    );
    assert_eq!(
        payload["concurrency"]["per_model"]["gpt-5.4-mini"]["remaining_capacity"],
        1
    );
    assert_eq!(payload["concurrency"]["exceeded"], false);
    assert_eq!(payload["lifecycle_thresholds"]["reclaim_after_ms"], 45_000);
    assert_eq!(
        payload["lifecycle_thresholds"]["reclaim_after_source"],
        "default"
    );
    assert_eq!(payload["lifecycle_thresholds"]["stale_after_ms"], 45_000);
    assert_eq!(
        payload["lifecycle_thresholds"]["stale_after_source"],
        "default"
    );
    assert_eq!(
        payload["reclaim_replan_recommendation"]["stale_after_ms"],
        45_000
    );
}

#[test]
fn ccc_status_uses_shared_runtime_config_for_run_wide_subagent_concurrency() {
    let workspace_dir = create_temp_path("status-subagent-concurrency-config");
    create_dir_all(&workspace_dir).expect("create workspace");
    let config_path = workspace_dir
        .join("config-home")
        .join("ccc")
        .join("ccc-config.toml");
    create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    write(
        &config_path,
        r#"
[runtime]
preferred_specialist_execution_mode = "codex_subagent"
fallback_specialist_execution_mode = "codex_exec"

[runtime.host_subagent_concurrency]
default_provider_concurrency_limit = 1
default_model_concurrency_limit = 3

[runtime.host_subagent_concurrency.provider_concurrency_limits]
openai = 1

[runtime.host_subagent_concurrency.model_concurrency_limits]
"gpt-5.3-codex" = 1
"gpt-5.4-mini" = 1
"#,
    )
    .expect("write runtime config");

    let run_directory = write_test_run_fixture(&workspace_dir, "run-status-concurrency");
    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-status-concurrency",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-running-model",
        "status": "running",
        "summary": "Raider is still running.",
        "observed_model": "gpt-5.3-codex"
    }))
    .expect("record running subagent");

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file).expect("read run record");
    let child_agents = run_record
        .get_mut("child_agents")
        .and_then(Value::as_array_mut)
        .expect("child agents");
    assert_eq!(child_agents[0]["observed_model"], "gpt-5.3-codex");
    child_agents.push(json!({
        "agent_id": "ccc_scout",
        "role": "explorer",
        "status": "spawned",
        "task_card_id": "task-2",
        "thread_id": "thread-other-task",
        "observed_model": "gpt-5.4-mini"
    }));
    write_json_document(&run_file, &run_record).expect("write run record");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-status-concurrency",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let mut session_context = create_session_context();
    session_context.shared_config_path = config_path.to_string_lossy().into_owned();
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    let concurrency = &status_payload["host_subagent_state"]["concurrency"];

    assert_eq!(
        status_payload["host_subagent_state"]["active_subagent_count"],
        1
    );
    assert_eq!(concurrency["active_count"], 2);
    assert_eq!(concurrency["default_provider_limit"], 1);
    assert_eq!(concurrency["default_provider_limit_source"], "explicit");
    assert_eq!(concurrency["default_model_limit"], 3);
    assert_eq!(concurrency["default_model_limit_source"], "explicit");
    assert_eq!(concurrency["per_provider"]["openai"]["active_count"], 2);
    assert_eq!(concurrency["per_provider"]["openai"]["limit"], 1);
    assert_eq!(concurrency["per_provider"]["openai"]["exceeded"], true);
    assert_eq!(concurrency["per_model"]["gpt-5.3-codex"]["active_count"], 1);
    assert_eq!(concurrency["per_model"]["gpt-5.3-codex"]["limit"], 1);
    assert_eq!(concurrency["per_model"]["gpt-5.4-mini"]["active_count"], 1);
    assert_eq!(
        status_payload["runtime_config"]["host_subagent_concurrency"]
            ["provider_concurrency_limits"]["openai"],
        1
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_merged_reopens_captain_advance_and_records_drift() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-merged");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Implement a bounded code change",
        "title": "Apply a bounded code update",
        "intent": "Create a run that defaults to workspace-write mutation",
        "scope": "One bounded mutation task",
        "acceptance": "Return a bounded mutation summary",
        "prompt": "Update the implementation in one bounded pass",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let task_card_file = run_directory.join("task-cards").join(format!(
        "{}.json",
        start_payload["task_card_id"]
            .as_str()
            .expect("task card id")
    ));
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    task_card["assigned_role"] = json!("code specialist");
    task_card["assigned_agent_id"] = json!("raider");
    task_card["sandbox_mode"] = json!("workspace-write");
    task_card["delegation_plan"] = create_specialist_delegation_plan_with_runtime(
        "code specialist",
        &json!({
            "summary": "Bounded code mutation.",
            "model": "gpt-5.3-codex",
            "variant": "high",
            "fast_mode": true,
        }),
        &json!({
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec",
        }),
        "workspace-write",
        "Raider performs bounded mutation work.",
    );
    write_json_document(&task_card_file, &task_card).expect("write task card");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-subagent-merge",
        "status": "merged",
        "summary": "Captain merged the bounded raider result.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:42"],
        "next_action": "advance",
        "open_questions": [],
        "confidence": "high",
        "observed_sandbox_mode": "read-only",
        "fallback_reason": "parent_override_conflict"
    }))
    .expect("merged subagent update");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(status_payload["next_step"], "advance");
    assert_eq!(status_payload["can_advance"], true);
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["status"],
        "merged"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_policy_drift"]["ok"],
        false
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_policy_drift"]["mismatches"][0]["field"],
        "sandbox_mode"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fallback"]["reason"],
        "parent_override_conflict"
    );
    assert_eq!(status_payload["run_state"]["current_phase_name"], "fan_in");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn cap_style_specialist_routes_record_direct_captain_bypass_and_require_acceptance() {
    let session_context = create_session_context();
    let cases = [
        (
            "docs",
            "Use $cap to update README documentation wording only.",
            "documenter",
            "scribe",
        ),
        (
            "translation",
            "Use $cap to translate README.md and docs/install.md into Korean.",
            "documenter",
            "scribe",
        ),
        (
            "review",
            "Use $cap to review rust/ccc-mcp/src/main.rs for regressions and acceptance gaps.",
            "verifier",
            "arbiter",
        ),
        (
            "implementation",
            "Use $cap to implement a bounded fix in rust/ccc-mcp/src/main.rs.",
            "code specialist",
            "raider",
        ),
    ];
    let routing_config = json!({
        "agents": {
            "code specialist": {
                "name": "raider",
                "summary": "Bounded code mutation.",
                "model": "gpt-5.3-codex",
                "variant": "high",
                "fast_mode": true,
                "config_entries": []
            },
            "documenter": {
                "name": "scribe",
                "summary": "Docs and translation updates.",
                "model": "gpt-5.4-mini",
                "variant": "high",
                "fast_mode": true,
                "config_entries": []
            },
            "verifier": {
                "name": "arbiter",
                "summary": "Review and acceptance judgment.",
                "model": "gpt-5.5",
                "variant": "high",
                "fast_mode": true,
                "config_entries": []
            }
        }
    });

    for (label, request, expected_role, expected_agent_id) in cases {
        let route = create_specialist_shortlist_payload_from_config(
            &routing_config,
            request,
            "code specialist",
            None,
        );
        assert_eq!(route["selected_role"], expected_role, "{label}");
        assert_eq!(route["selected_agent_id"], expected_agent_id, "{label}");

        let workspace_dir = create_temp_path(&format!("cap-routing-drift-{label}"));
        create_dir_all(&workspace_dir).expect("create workspace");
        let run_id = format!("run-cap-routing-drift-{label}");
        let run_directory = write_test_run_fixture(&workspace_dir, &run_id);
        let task_card_file = run_directory.join("task-cards").join("task-1.json");
        let mut task_card = read_json_document(&task_card_file).expect("read task card");
        task_card["title"] = json!(format!("Route {label} through specialist"));
        task_card["intent"] =
            json!("Prove captain cannot silently bypass a specialist-owned $cap route");
        task_card["scope"] = json!("One bounded routing-drift regression");
        task_card["prompt"] = json!(request);
        task_card["acceptance"] =
            json!("Direct captain drift is recorded and blocked until review or acceptance.");
        task_card["assigned_role"] = json!(expected_role);
        task_card["assigned_agent_id"] = json!(expected_agent_id);
        task_card["sandbox_mode"] = json!(sandbox_mode_for_role(expected_role));
        task_card["delegation_plan"] = create_specialist_delegation_plan_with_runtime(
            expected_role,
            &routing_config["agents"][expected_role],
            &json!({
                "preferred_specialist_execution_mode": "codex_subagent",
                "fallback_specialist_execution_mode": "codex_exec",
            }),
            sandbox_mode_for_role(expected_role),
            sandbox_rationale_for_role(expected_role),
        );
        task_card["routing_trace"] = json!({
            "selected_role": route["selected_role"].clone(),
            "selected_agent_id": route["selected_agent_id"].clone(),
            "specialist_route": route,
        });
        write_json_document(&task_card_file, &task_card).expect("write task card");

        assert_eq!(task_card["assigned_role"], expected_role, "{label}");
        assert_eq!(task_card["assigned_agent_id"], expected_agent_id, "{label}");
        assert_eq!(
            task_card["routing_trace"]["selected_role"], expected_role,
            "{label}"
        );

        let update_payload = create_ccc_subagent_update_payload(&json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
            "child_agent_id": "captain",
            "thread_id": format!("thread-captain-{label}"),
            "status": "completed",
            "summary": format!("Captain directly handled the {label} request without specialist fan-in."),
            "fan_in_status": "completed",
            "evidence_paths": ["captain/direct.md:1"],
            "next_action": "captain_merge",
            "open_questions": [],
            "confidence": "medium"
        }))
        .expect("direct captain update");
        assert_eq!(update_payload["child_agent_id"], "captain", "{label}");

        let drifted_task_card = read_active_task_card(&run_directory);
        let drift = &drifted_task_card["subagent_policy_drift"];
        assert_eq!(drift["ok"], false, "{label}");
        assert_eq!(drift["direct_captain_bypass"], true, "{label}");
        assert_eq!(drift["observed"]["child_agent_id"], "captain", "{label}");
        assert_eq!(drift["expected"]["role"], expected_role, "{label}");
        assert!(drift["mismatches"]
            .as_array()
            .expect("drift mismatches")
            .iter()
            .any(|mismatch| mismatch["field"] == "child_agent_id"));
        assert_eq!(drift["acceptance_gate"]["state"], "required", "{label}");
        assert_eq!(
            drift["acceptance_gate"]["required_action"], "spawn_or_merge_review",
            "{label}"
        );

        let locator = resolve_run_locator_arguments(
            &json!({
                "run_id": run_id,
                "cwd": workspace_dir.to_string_lossy(),
            }),
            "ccc_status",
        )
        .expect("locator");
        let status_payload =
            create_ccc_status_payload(&session_context, &locator).expect("status payload");
        assert_eq!(
            status_payload["captain_action_contract"]["allowed_action"],
            "captain_drift_acceptance_required",
            "{label}"
        );
        assert_eq!(
            status_payload["captain_action_contract"]["required_action"], "spawn_or_merge_review",
            "{label}"
        );
        assert_eq!(
            status_payload["captain_action_contract"]["direct_finish_allowed"], false,
            "{label}"
        );
        assert_eq!(
            status_payload["captain_action_contract"]["direct_mutation_allowed"], false,
            "{label}"
        );
        assert_eq!(
            status_payload["captain_action_contract"]["direct_file_mutation_policy"]["allowed"],
            false,
            "{label}"
        );
        assert_eq!(
            status_payload["captain_action_contract"]["direct_file_mutation_policy"]
                ["required_route"],
            "specialist_fan_in_then_captain_review_merge",
            "{label}"
        );

        let _ = fs::remove_dir_all(&workspace_dir);
    }
}

#[test]
fn ccc_subagent_update_detects_wrong_specialist_child_agent_id_mismatch() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-wrong-specialist-id");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Implement a bounded code change",
        "title": "Apply bounded mutation",
        "intent": "Ensure wrong specialist identity is recorded as drift",
        "scope": "One bounded mutation task",
        "acceptance": "Wrong specialist child_agent_id is detected without direct-captain acceptance semantics",
        "prompt": "Apply one bounded code mutation and report result",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );

    let update_payload = create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "scribe",
        "thread_id": "thread-wrong-specialist",
        "status": "completed",
        "summary": "Scribe incorrectly reported fan-in for a code specialist task.",
        "fan_in_status": "completed",
        "evidence_paths": ["docs/wrong-specialist.md:1"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "medium"
    }))
    .expect("wrong specialist update");
    assert_eq!(update_payload["child_agent_id"], "scribe");

    let drifted_task_card = read_active_task_card(&run_directory);
    let drift = &drifted_task_card["subagent_policy_drift"];
    assert_eq!(drift["ok"], false);
    assert_eq!(drift["direct_captain_bypass"], false);
    assert_eq!(drift["expected"]["child_agent_id"], "ccc_raider");
    assert_eq!(drift["observed"]["child_agent_id"], "scribe");
    assert!(drift["mismatches"]
        .as_array()
        .expect("drift mismatches")
        .iter()
        .any(|mismatch| mismatch["field"] == "child_agent_id"
            && mismatch["expected"] == "ccc_raider"
            && mismatch["observed"] == "scribe"));
    assert_eq!(drift["acceptance_gate"], Value::Null);

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_ne!(
        status_payload["captain_action_contract"]["allowed_action"],
        "captain_drift_acceptance_required"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_normalizes_raw_host_child_id_into_thread_id() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-normalize-host-id");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Implement a bounded code change",
        "title": "Apply bounded mutation",
        "intent": "Ensure subagent lifecycle identity remains CCC-owned",
        "scope": "One bounded mutation task",
        "acceptance": "Raw host IDs do not replace CCC child_agent_id",
        "prompt": "Apply one bounded code mutation and report result",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let raw_host_session_id = "af0f8a4a-c38a-4a75-8a44-2ee73f16aa72";

    let update_payload = create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": raw_host_session_id,
        "status": "completed",
        "summary": "Completed bounded mutation.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:21"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("subagent update");

    assert_eq!(update_payload["child_agent_id"], "ccc_raider");
    assert_eq!(update_payload["thread_id"], raw_host_session_id);

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    assert_eq!(run_record["active_agent_id"], "captain");
    assert_eq!(run_record["active_thread_id"], Value::Null);
    assert_eq!(
        run_record["host_subagent_handle_archive"][0]["thread_id"],
        raw_host_session_id
    );

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["child_agent_id"],
        "ccc_raider"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["thread_id"],
        raw_host_session_id
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_preserves_explicit_managed_custom_agent_identity() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-preserve-managed-agent");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Review a source task through an explicit arbiter",
        "title": "Explicit arbiter fan-in",
        "intent": "Do not rewrite managed custom agent identities to the source task owner",
        "scope": "One bounded review fan-in",
        "acceptance": "ccc_arbiter remains the recorded child_agent_id",
        "prompt": "Record a review fan-in for the active task.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    let update_payload = create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_arbiter",
        "thread_id": "thread-explicit-arbiter",
        "status": "completed",
        "summary": "Arbiter reviewed the source task evidence.",
        "fan_in_status": "passed",
        "evidence_paths": ["src/lib.rs:10"],
        "next_action": "captain_accept",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("subagent update");

    assert_eq!(update_payload["child_agent_id"], "ccc_arbiter");
    assert_eq!(update_payload["thread_id"], "thread-explicit-arbiter");
    assert_eq!(update_payload["review_outcome"], "passed");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["review_lifecycle"]["child_agent_id"],
        "ccc_arbiter"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"],
        Value::Null
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["current_task_card"]["review_lifecycle"]["child_agent_id"],
        "ccc_arbiter"
    );
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["child_agent_id"],
        "ccc_arbiter"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_records_host_subagent_token_usage_and_route_state() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-host-token-usage");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Record host subagent token usage",
        "title": "Host subagent usage",
        "intent": "Aggregate usage reported by host custom subagents",
        "scope": "One bounded subagent update",
        "acceptance": "Status reports non-zero token usage and route lifecycle state",
        "prompt": "Spawn a custom subagent and record its usage.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");

    let before_status =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        before_status["current_task_card"]["route_enforcement_state"],
        "host_subagent_spawn_required"
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-host-usage",
        "status": "completed",
        "summary": "Completed bounded mutation.",
        "total_token_usage": {
            "input_tokens": 100,
            "cached_input_tokens": 20,
            "output_tokens": 30,
            "reasoning_output_tokens": 7
        },
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("subagent update");

    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["current_task_card"]["route_enforcement_state"],
        "host_subagent_lifecycle_recorded"
    );
    assert_eq!(status_payload["token_usage"]["total_tokens"], 157);
    assert_eq!(
        status_payload["token_usage"]["source"],
        "delegation_and_host_subagent_usage_best_effort"
    );
    assert_eq!(
        status_payload["token_usage_visibility"]["status"],
        "available"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_host_subagent_context_estimates_without_raw_usage() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-host-context-estimate");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Record host subagent context usage",
        "title": "Host subagent context",
        "intent": "Surface context estimates when raw token usage is unavailable",
        "scope": "One bounded subagent update",
        "acceptance": "Status reports per-agent context amount",
        "prompt": "Spawn a custom subagent and record its context estimate.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-host-context",
        "status": "completed",
        "summary": "Completed bounded mutation with host context estimate.",
        "context_tokens": 10500,
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("subagent update");

    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(status_payload["token_usage"]["total_tokens"], 0);
    assert_eq!(status_payload["token_usage"]["total_context_tokens"], 10500);
    assert_eq!(
        status_payload["token_usage"]["by_agent"][0]["context_tokens"],
        10500
    );
    assert_eq!(
        status_payload["token_usage_visibility"]["status"],
        "context_available"
    );

    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Estimated Context: 10.5k"));
    assert!(text.contains("By Agent Context Estimate: ccc_raider 100% (10.5k)"));
    assert!(!text.contains("Token usage: unavailable for host custom subagents"));

    let quiet = create_status_quiet_line(&status_payload);
    assert_eq!(
        quiet,
        format!(
            "run_id={} status=active next=await_fan_in",
            status_payload["run_id"].as_str().expect("run id")
        )
    );
    assert!(!quiet.contains("tokens="));
    assert!(!quiet.contains("token_reason="));
    assert!(!quiet.contains("context_estimate="));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_recommends_long_session_rollover_when_context_pressure_is_high() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("long-session-context-pressure");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Keep long Codex CLI sessions bounded",
        "title": "Long session mitigation",
        "intent": "Recommend operator-approved rollover when context pressure grows",
        "scope": "Status output and compact command templates",
        "acceptance": "Status recommends /compact with checkpoint and resume guidance",
        "prompt": "Continue a long CCC run with a custom subagent context estimate.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-long-context",
        "status": "completed",
        "summary": "Completed bounded mutation after a long session.",
        "context_tokens": 125000,
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("subagent update");

    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["long_session_mitigation"]["recommended"],
        true
    );
    assert_eq!(
        status_payload["long_session_mitigation"]["recommended_action"],
        "/compact"
    );
    assert_eq!(
        status_payload["long_session_mitigation"]["operator_choice_required"],
        true
    );
    assert_eq!(
        status_payload["long_session_mitigation"]["checkpoint_required"],
        true
    );
    assert!(
        status_payload["long_session_mitigation"]["checkpoint_command"]
            .as_str()
            .unwrap()
            .contains("Checkpoint before Codex CLI session rollover.")
    );
    assert_eq!(
        status_payload["long_session_mitigation"]["resume_command"],
        format!("$cap continue {run_id}")
    );
    assert_eq!(
        status_payload["context_health"]["schema"],
        "ccc.context_health.v1"
    );
    assert_eq!(
        status_payload["context_health"]["status"],
        "attention_needed"
    );
    assert_eq!(
        status_payload["context_health"]["safe_action"],
        "checkpoint_then_operator_rollover"
    );
    assert_eq!(
        status_payload["context_health"]["pressure_signals"][0]["kind"],
        "context_pressure"
    );
    assert_eq!(
        status_payload["restart_handoff"]["schema"],
        "ccc.restart_handoff.v1"
    );
    assert_eq!(status_payload["restart_handoff"]["restart_needed"], true);
    assert_eq!(
        status_payload["restart_handoff"]["automatic_restart"],
        false
    );
    assert_eq!(
        status_payload["restart_handoff"]["resume_command"],
        format!("$cap continue {run_id}")
    );
    assert_eq!(
        status_payload["restart_handoff"]["current_longway_state"],
        "active"
    );
    assert_eq!(
        status_payload["restart_handoff"]["next_task"]["task_card_id"],
        status_payload["current_task_card"]["task_card_id"]
    );
    assert_eq!(
        status_payload["app_panel"]["context_health"]["safe_action"],
        "checkpoint_then_operator_rollover"
    );

    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Rollover: recommend /compact"));
    assert!(text.contains("operator choice required"));
    assert!(text.contains(&format!("$cap continue {run_id}")));

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["command_templates"]["session_rollover"]["recommended_action"],
        "/compact"
    );
    assert_eq!(
        compact["context_health"]["safe_action"],
        "checkpoint_then_operator_rollover"
    );
    assert_eq!(compact["restart_handoff"]["restart_needed"], true);
    assert!(
        compact["command_templates"]["session_rollover"]["slash_command_boundary"]
            .as_str()
            .unwrap()
            .contains("does not claim to execute")
    );
    assert!(
        compact["command_templates"]["session_rollover"]["operator_choices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["action"] == "/exit")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_recommends_new_rollover_when_host_subagent_pressure_is_high() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("long-session-resource-pressure");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_id = "run-long-session-resource-pressure";
    let run_directory = write_test_run_fixture(&workspace_dir, run_id);
    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file).expect("read run");
    run_record["child_agents"] = json!([
        {
            "agent_id": "ccc_raider",
            "parent_agent_id": "captain",
            "role": "code specialist",
            "status": "running",
            "task_card_id": "task-1",
            "thread_id": "thread-resource-pressure",
            "summary": "Host subagent has stayed active long enough to require reclaim or replan.",
            "updated_at": "2020-01-01T00:00:00.000Z",
            "created_at": "2020-01-01T00:00:00.000Z"
        }
    ]);
    write_json_document(&run_file, &run_record).expect("write run");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]
            ["needs_operator_attention"],
        true
    );
    assert_eq!(
        status_payload["host_subagent_state"]["reclaim_replan_recommendation"]
            ["recommended_action"],
        "reclaim_or_replan"
    );
    assert_eq!(
        status_payload["long_session_mitigation"]["recommended"],
        true
    );
    assert_eq!(
        status_payload["long_session_mitigation"]["recommended_action"],
        "/new"
    );
    assert_eq!(
        status_payload["context_health"]["safe_action"],
        "checkpoint_then_operator_rollover"
    );
    assert_eq!(
        status_payload["restart_handoff"]["active_conflict_state"]["reclaim_needs_attention"],
        true
    );
    assert!(status_payload["long_session_mitigation"]["signals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|signal| {
            signal["kind"] == "resource_pressure"
                && signal["severity"] == "high"
                && signal["reclaim_needs_attention"] == true
        }));

    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Rollover: recommend /new"));
    assert!(text.contains("reason=resource_pressure"));

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["command_templates"]["session_rollover"]["recommended_action"],
        "/new"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_merged_sparse_preserves_completed_fan_in_details() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-merged-preserve-fan-in");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Preserve completed fan-in details on merged update",
        "title": "Keep completed fan-in fields after merge",
        "intent": "Regression test for sparse merged subagent update payloads",
        "scope": "One bounded subagent update sequence",
        "acceptance": "Merged update keeps prior completed fan-in fields intact",
        "prompt": "Record completed fan-in then send sparse merged update",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-subagent-completed",
        "status": "completed",
        "summary": "Raider completed bounded fan-in payload.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:42", "README.md:12"],
        "next_action": "captain_merge",
        "open_questions": ["Any cleanup needed?"],
        "confidence": "high"
    }))
    .expect("completed subagent update");

    let merged_update = create_ccc_subagent_update_payload(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-subagent-merge",
        "status": "merged",
        "summary": "Captain merged the bounded raider result."
    }))
    .expect("merged subagent update");

    assert_eq!(
        merged_update["fan_in"]["summary"],
        "Raider completed bounded fan-in payload."
    );
    assert_eq!(merged_update["fan_in"]["status"], "completed");
    assert_eq!(
        merged_update["fan_in"]["evidence_paths"][0],
        "src/main.rs:42"
    );
    assert_eq!(merged_update["fan_in"]["next_action"], "captain_merge");
    assert_eq!(
        merged_update["fan_in"]["open_questions"][0],
        "Any cleanup needed?"
    );
    assert_eq!(merged_update["fan_in"]["confidence"], "high");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["status"],
        "merged"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_lifecycle"]["summary"],
        "Captain merged the bounded raider result."
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["summary"],
        "Raider completed bounded fan-in payload."
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["evidence_paths"][0],
        "src/main.rs:42"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["next_action"],
        "captain_merge"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["open_questions"][0],
        "Any cleanup needed?"
    );
    assert_eq!(
        status_payload["current_task_card"]["subagent_fan_in"]["confidence"],
        "high"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_parallel_fan_in_waits_for_all_required_lanes() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-parallel-fan-in");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-parallel-fan-in");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["parallel_fanout"] = json!({
        "mode": "parallel",
        "requested_parallel": true,
        "selection_basis": "explicit_parallel_signal",
        "default_lane_count": 2,
        "max_lane_count": 4,
        "required_lane_ids": ["raider-a", "raider-b"],
        "all_lane_ids": ["raider-a", "raider-b", "raider-c", "raider-d"],
        "disjoint_scope_required": true,
        "disjoint_scope_verified": true,
        "summary": "Parallel fan-out enabled for two disjoint lanes.",
        "lanes": [
            { "lane_id": "raider-a", "required": true, "scope": "src/server/auth.rs", "lifecycle": null, "fan_in": null },
            { "lane_id": "raider-b", "required": true, "scope": "src/server/routes.rs", "lifecycle": null, "fan_in": null }
        ],
        "aggregate": {
            "required_lane_count": 2,
            "active_lane_count": 0,
            "terminal_lane_count": 0,
            "fan_in_ready": false,
            "status": "awaiting_lane_updates",
            "updated_at": "2026-04-22T08:00:00.000Z"
        },
        "recorded_at": "2026-04-22T08:00:00.000Z",
        "updated_at": "2026-04-22T08:00:00.000Z"
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-parallel-fan-in",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "lane_id": "raider-a",
        "thread_id": "thread-parallel-a",
        "status": "completed",
        "summary": "Lane A completed bounded mutation.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/server/auth.rs:40"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("lane a update");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-parallel-fan-in",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_after_lane_a =
        create_ccc_status_payload(&session_context, &locator).expect("status lane a");
    assert_eq!(status_after_lane_a["next_step"], "await_fan_in");
    assert_eq!(
        status_after_lane_a["host_subagent_state"]["fan_in_ready"],
        false
    );
    assert_eq!(
        status_after_lane_a["host_subagent_state"]["parallel_lane_state"]["missing_lane_ids"][0],
        "raider-b"
    );
    assert_eq!(
        status_after_lane_a["scheduler"]["decision_source"],
        "bounded_parallel_fanout"
    );
    assert_eq!(
        status_after_lane_a["scheduler"]["action"]["kind"],
        "await_parallel_fan_in"
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-parallel-fan-in",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_raider",
        "lane_id": "raider-b",
        "thread_id": "thread-parallel-b",
        "status": "failed",
        "summary": "Lane B failed with bounded evidence.",
        "fan_in_status": "failed",
        "evidence_paths": ["src/server/routes.rs:12"],
        "next_action": "captain_merge",
        "open_questions": ["Retry lane B?"],
        "confidence": "medium"
    }))
    .expect("lane b update");

    let status_after_lane_b =
        create_ccc_status_payload(&session_context, &locator).expect("status lane b");
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["fan_in_ready"],
        true
    );
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["parallel_lane_state"]["fan_in_ready"],
        true
    );
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["parallel_lane_state"]["terminal_lane_count"],
        2
    );
    assert_eq!(
        status_after_lane_b["scheduler"]["action"]["kind"],
        "parallel_fan_in_ready"
    );
    assert_eq!(
        status_after_lane_b["scheduler"]["parallel"]["fan_in_ready"],
        true
    );

    let saved_task_card = read_json_document(&task_card_file).expect("saved task card");
    let lanes = saved_task_card["parallel_fanout"]["lanes"]
        .as_array()
        .expect("lanes");
    let lane_a = lanes
        .iter()
        .find(|lane| lane["lane_id"] == "raider-a")
        .expect("lane a");
    let lane_b = lanes
        .iter()
        .find(|lane| lane["lane_id"] == "raider-b")
        .expect("lane b");
    assert_eq!(lane_a["fan_in"]["status"], "completed");
    assert_eq!(lane_b["fan_in"]["status"], "failed");
    assert_eq!(
        saved_task_card["parallel_fanout"]["aggregate"]["fan_in_ready"],
        true
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_parallel_fan_in_supports_scout_lane_ids() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("subagent-parallel-scout-fan-in");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-parallel-scout-fan-in");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["task_kind"] = json!("explore");
    task_card["assigned_role"] = json!("explorer");
    task_card["assigned_agent_id"] = json!("scout");
    task_card["parallel_fanout"] = json!({
        "mode": "parallel",
        "requested_parallel": true,
        "selection_basis": "broad_exploration_signal",
        "default_lane_count": 2,
        "max_lane_count": 4,
        "required_lane_ids": ["scout-a", "scout-b"],
        "all_lane_ids": ["scout-a", "scout-b", "scout-c", "scout-d"],
        "disjoint_scope_required": false,
        "disjoint_scope_verified": false,
        "summary": "Parallel scout fan-out across two lanes.",
        "lanes": [
            { "lane_id": "scout-a", "required": true, "scope": null, "lifecycle": null, "fan_in": null },
            { "lane_id": "scout-b", "required": true, "scope": null, "lifecycle": null, "fan_in": null }
        ],
        "aggregate": {
            "required_lane_count": 2,
            "active_lane_count": 0,
            "terminal_lane_count": 0,
            "fan_in_ready": false,
            "status": "awaiting_lane_updates",
            "updated_at": "2026-04-22T08:00:00.000Z"
        },
        "recorded_at": "2026-04-22T08:00:00.000Z",
        "updated_at": "2026-04-22T08:00:00.000Z"
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-parallel-scout-fan-in",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_scout",
        "lane_id": "scout-a",
        "thread_id": "thread-scout-a",
        "status": "completed",
        "summary": "Scout lane A gathered evidence.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/main.rs:10"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "high"
    }))
    .expect("lane scout-a update");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-parallel-scout-fan-in",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_after_lane_a =
        create_ccc_status_payload(&session_context, &locator).expect("status lane scout-a");
    assert_eq!(
        status_after_lane_a["host_subagent_state"]["fan_in_ready"],
        false
    );
    assert_eq!(
        status_after_lane_a["host_subagent_state"]["parallel_lane_state"]["missing_lane_ids"][0],
        "scout-b"
    );

    create_ccc_subagent_update_payload(&json!({
        "run_id": "run-parallel-scout-fan-in",
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "ccc_scout",
        "lane_id": "scout-b",
        "thread_id": "thread-scout-b",
        "status": "completed",
        "summary": "Scout lane B gathered evidence.",
        "fan_in_status": "completed",
        "evidence_paths": ["src/lib.rs:12"],
        "next_action": "captain_merge",
        "open_questions": [],
        "confidence": "medium"
    }))
    .expect("lane scout-b update");

    let status_after_lane_b =
        create_ccc_status_payload(&session_context, &locator).expect("status lane scout-b");
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["fan_in_ready"],
        true
    );
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["parallel_lane_state"]["fan_in_ready"],
        true
    );
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["parallel_lane_state"]["lane_statuses"]
            ["scout-a"],
        "completed"
    );
    assert_eq!(
        status_after_lane_b["host_subagent_state"]["parallel_lane_state"]["lane_statuses"]
            ["scout-b"],
        "completed"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_rejects_non_code_fallback_reason() {
    let workspace_dir = create_temp_path("subagent-fallback-code");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Create a run for fallback validation",
        "title": "Validate fallback reason",
        "intent": "Reject verbose or unknown fallback reasons",
        "scope": "One bounded validation task",
        "acceptance": "Unknown fallback reason is rejected",
        "prompt": "Validate fallback reason codes only.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    let error = parse_ccc_subagent_update_arguments(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "status": "failed",
        "fallback_reason": "the child was odd and very verbose"
    }))
    .expect_err("unknown fallback reason should be rejected");

    assert!(error.to_string().contains("fallback_reason must be one of"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_rejects_fallback_reason_before_terminal_status() {
    let workspace_dir = create_temp_path("subagent-fallback-terminal");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Create a run for fallback terminal validation",
        "title": "Validate fallback terminal gate",
        "intent": "Reject fallback reasons before specialist terminal state",
        "scope": "One bounded validation task",
        "acceptance": "Fallback requires terminal specialist status",
        "prompt": "Validate fallback terminal gate.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    let error = parse_ccc_subagent_update_arguments(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "status": "running",
        "fallback_reason": "subagent_spawn_unavailable"
    }))
    .expect_err("active fallback reason should be rejected");

    assert!(error
        .to_string()
        .contains("fallback_reason requires terminal specialist status"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_subagent_update_accepts_host_thread_limit_fallback_reason() {
    let workspace_dir = create_temp_path("subagent-thread-limit-fallback");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Create a run for thread-limit fallback validation",
        "title": "Validate thread-limit fallback reason",
        "intent": "Accept a concrete host subagent thread-limit fallback reason",
        "scope": "One bounded validation task",
        "acceptance": "Thread-limit fallback reason is accepted for terminal specialist state",
        "prompt": "Validate fallback reason codes only.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");

    let parsed = parse_ccc_subagent_update_arguments(&json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "status": "failed",
        "fallback_reason": "host_subagent_thread_limit"
    }))
    .expect("thread-limit fallback reason should parse");

    assert_eq!(parsed["fallback_reason"], "host_subagent_thread_limit");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_does_not_allow_degraded_fallback_without_terminal_subagent_state() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("fallback-needs-terminal");
    create_dir_all(&workspace_dir).expect("create workspace");
    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Validate degraded fallback gate",
        "title": "Fallback terminal gate",
        "intent": "Recorded fallback reason alone is not enough",
        "scope": "One bounded fallback gate check",
        "acceptance": "Fallback waits for terminal specialist state",
        "prompt": "Validate fallback gate.",
        "task_kind": "execution"
    }))
    .expect("start payload");
    let run_id = start_payload["run_id"].as_str().expect("run id");
    let task_card_id = start_payload["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    task_card["subagent_fallback"] = json!({
        "reason": "subagent_spawn_unavailable",
        "recorded_at": "2026-04-25T00:00:00.000Z",
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["current_task_card"]["route_enforcement_state"],
        "degraded_direct_host_fallback_pending_terminal_specialist_state"
    );
    assert_eq!(
        status_payload["execution_strategy"]["subagent_fallback_recorded"],
        true
    );
    assert_eq!(
        status_payload["execution_strategy"]["subagent_fallback_ready"],
        false
    );
    assert_eq!(
        status_payload["execution_strategy"]["codex_exec_fallback_allowed"],
        false
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_blocks_degraded_fallback_until_all_required_parallel_lanes_terminal() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("fallback-parallel-required-lanes");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-fallback-parallel-lanes");
    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file).expect("read run");
    run_record["child_agents"] = json!([
        {
            "agent_id": "ccc_raider_a",
            "parent_agent_id": "captain",
            "role": "code specialist",
            "status": "failed",
            "task_card_id": "task-1",
            "lane_id": "raider-a",
            "thread_id": "thread-raider-a",
            "summary": "Lane A failed and recorded a fallback reason.",
            "updated_at": "2026-04-22T08:00:00.000Z",
            "created_at": "2026-04-22T08:00:00.000Z"
        },
        {
            "agent_id": "ccc_raider_b",
            "parent_agent_id": "captain",
            "role": "code specialist",
            "status": "running",
            "task_card_id": "task-1",
            "lane_id": "raider-b",
            "thread_id": "thread-raider-b",
            "summary": "Lane B is still required fan-in.",
            "updated_at": "2020-01-01T00:00:00.000Z",
            "created_at": "2020-01-01T00:00:00.000Z"
        }
    ]);
    write_json_document(&run_file, &run_record).expect("write run");

    let run_state_file = run_directory.join("run-state.json");
    let mut run_state = read_json_document(&run_state_file).expect("read run-state");
    run_state["next_action"] = json!({ "command": "await_fan_in" });
    write_json_document(&run_state_file, &run_state).expect("write run-state");

    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("read task-card");
    task_card["delegation_plan"] = create_specialist_delegation_plan_with_runtime(
        "code specialist",
        &json!({
            "summary": "Bounded parallel code mutation.",
            "model": "gpt-5.3-codex",
            "variant": "high",
            "fast_mode": true,
        }),
        &json!({
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec",
        }),
        "workspace-write",
        "Raider performs bounded mutation work.",
    );
    task_card["subagent_fallback"] = json!({
        "reason": "subagent_spawn_unavailable",
        "recorded_at": "2026-04-25T00:00:00.000Z",
    });
    task_card["parallel_fanout"] = json!({
        "mode": "parallel",
        "requested_parallel": true,
        "selection_basis": "explicit_parallel_signal",
        "default_lane_count": 2,
        "max_lane_count": 4,
        "required_lane_ids": ["raider-a", "raider-b"],
        "all_lane_ids": ["raider-a", "raider-b", "raider-c", "raider-d"],
        "disjoint_scope_required": true,
        "disjoint_scope_verified": true,
        "summary": "Parallel fan-out enabled for two disjoint lanes.",
        "lanes": [
            {
                "lane_id": "raider-a",
                "required": true,
                "scope": "src/server/auth.rs",
                "lifecycle": { "status": "failed" },
                "fan_in": {
                    "summary": "Lane A failed.",
                    "status": "failed",
                    "evidence_paths": [],
                    "next_action": "retry",
                    "open_questions": ["Retry lane A?"],
                    "confidence": "medium"
                }
            },
            {
                "lane_id": "raider-b",
                "required": true,
                "scope": "src/server/routes.rs",
                "lifecycle": { "status": "running" },
                "fan_in": null
            }
        ],
        "aggregate": {
            "required_lane_count": 2,
            "active_lane_count": 1,
            "terminal_lane_count": 1,
            "fan_in_ready": false,
            "status": "awaiting_lane_updates",
            "updated_at": "2026-04-22T08:00:00.000Z"
        },
        "recorded_at": "2026-04-22T08:00:00.000Z",
        "updated_at": "2026-04-22T08:00:00.000Z"
    });
    write_json_document(&task_card_file, &task_card).expect("write task-card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-fallback-parallel-lanes",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["current_task_card"]["route_enforcement_state"],
        "degraded_direct_host_fallback_pending_terminal_specialist_state"
    );
    assert_eq!(
        status_payload["execution_strategy"]["subagent_fallback_recorded"],
        true
    );
    assert_eq!(
        status_payload["execution_strategy"]["subagent_fallback_ready"],
        false
    );
    assert_eq!(
        status_payload["execution_strategy"]["codex_exec_fallback_allowed"],
        false
    );
    assert_eq!(
        status_payload["host_subagent_state"]["parallel_lane_state"]["missing_lane_ids"],
        json!(["raider-b"])
    );
    assert_eq!(
        status_payload["host_subagent_state"]["recovery_recommendation"]["recommended_action"],
        "reclaim"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["allowed_action"],
        "reclaim_subagent"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["required_action"],
        "ccc_subagent_update"
    );
    assert_eq!(
        status_payload["captain_action_contract"]["subagent_capacity_policy"]
            ["direct_specialist_takeover_allowed"],
        false
    );
    assert_eq!(
        status_payload["captain_action_contract"]["subagent_capacity_policy"]
            ["on_capacity_exhausted"],
        "wait_or_cleanup_before_direct_work"
    );
    assert!(
        status_payload["captain_action_contract"]["denied_action_reason"]
            .as_str()
            .expect("denied action reason")
            .contains("close completed host agents")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_streams_prompt_via_stdin_and_records_worker_usage() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("stdin-worker");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_fake_codex_executable_requiring_stdin(
        &workspace_dir,
        "fake-stdin-codex.sh",
        "thread-stdin-test",
        "Worker returned the bounded result from stdin prompt.",
        700,
        50,
        120,
        30,
    );

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Verify delegated codex stdin prompt delivery",
                    "title": "Run one bounded worker",
                    "intent": "Spawn a single delegated worker",
                    "scope": "Single execution task only",
                    "acceptance": "Worker returns a bounded result and usage",
                    "prompt": "Return the bounded result only.",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);

    let orchestrate_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        orchestrate_response.get("error").is_none(),
        "unexpected orchestrate response: {orchestrate_response:?}"
    );

    let delegations_path = run_directory.join("delegations");
    let wait_deadline = SystemTime::now() + Duration::from_secs(3);
    let delegation = loop {
        let maybe_completed = fs::read_dir(&delegations_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| read_json_document(&entry.path()).ok())
            .find(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    == Some("completed")
            });
        if maybe_completed.is_some() || SystemTime::now() >= wait_deadline {
            break maybe_completed;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    .expect("completed delegation");

    assert_eq!(delegation["worker_result"]["status"], "completed");
    assert_eq!(
        delegation["worker_result"]["thread_id"],
        "thread-stdin-test"
    );
    assert_eq!(
        delegation["worker_result"]["assistant_message_preview"],
        "Worker returned the bounded result from stdin prompt."
    );
    assert_eq!(
        delegation["worker_result"]["total_token_usage"]["input_tokens"],
        700
    );
    assert_eq!(
        delegation["worker_result"]["total_token_usage"]["cached_input_tokens"],
        50
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id: run_id.to_string(),
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["token_usage"]["total_tokens"], 900);
    assert_eq!(
        status_payload["latest_delegate_result"]["assistant_message_preview"],
        "Worker returned the bounded result from stdin prompt."
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_records_worker_exit_details_for_failed_worker_output() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("failing-worker");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_failing_fake_codex_executable(
        &workspace_dir,
        "fake-failing-codex.sh",
        17,
        "fatal: synthetic worker failure",
    );

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Verify failed delegated worker diagnostics",
                    "title": "Run one failing bounded worker",
                    "intent": "Spawn a single delegated worker that exits non-zero",
                    "scope": "Single execution task only",
                    "acceptance": "Worker failure details are recorded for captain follow-up",
                    "prompt": "Return the bounded result only.",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);

    let orchestrate_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        orchestrate_response.get("error").is_none(),
        "unexpected orchestrate response: {orchestrate_response:?}"
    );

    let delegations_path = run_directory.join("delegations");
    let wait_deadline = SystemTime::now() + Duration::from_secs(1);
    let delegation = loop {
        let maybe_failed = fs::read_dir(&delegations_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| read_json_document(&entry.path()).ok())
            .find(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    == Some("failed")
            });
        if maybe_failed.is_some() || SystemTime::now() >= wait_deadline {
            break maybe_failed;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    .expect("failed delegation");

    assert_eq!(delegation["worker_result"]["status"], "failed");
    assert_eq!(delegation["worker_result"]["process_exit"]["exit_code"], 17);
    assert_eq!(
        delegation["worker_result"]["process_exit"]["success"],
        false
    );
    assert_eq!(
        delegation["worker_result"]["raw_output_preview"],
        "fatal: synthetic worker failure"
    );
    assert_eq!(delegation["latest_failure"]["reason"], "execution_failed");
    assert!(delegation["latest_failure"]["summary"]
        .as_str()
        .unwrap_or_default()
        .contains("exit_code=17"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_marks_turn_failed_worker_as_failed() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("turn-failed-worker");
    create_dir_all(&workspace_dir).expect("create workspace");
    let fake_codex = create_turn_failed_fake_codex_executable(
        &workspace_dir,
        "fake-turn-failed-codex.sh",
        "thread-turn-failed",
        1,
        "synthetic turn failure",
    );

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 44,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Verify turn.failed worker classification",
                    "title": "Run one turn.failed bounded worker",
                    "intent": "Spawn a worker that emits thread.started and turn.failed",
                    "scope": "Single execution task only",
                    "acceptance": "turn.failed is recorded as a failed worker result",
                    "prompt": "Return the bounded result only.",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);

    let orchestrate_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 45,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": fake_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("orchestrate response");
    assert!(
        orchestrate_response.get("error").is_none(),
        "unexpected orchestrate response: {orchestrate_response:?}"
    );

    let delegations_path = run_directory.join("delegations");
    let delegation = fs::read_dir(&delegations_path)
        .expect("delegations dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find_map(|path| read_json_document(&path).ok())
        .expect("failed delegation");

    assert_eq!(delegation["child_agent"]["status"], "failed");
    assert_eq!(delegation["worker_result"]["status"], "failed");
    assert_eq!(
        delegation["worker_result"]["thread_id"],
        "thread-turn-failed"
    );
    assert_eq!(delegation["worker_result"]["process_exit"]["exit_code"], 1);
    assert_eq!(delegation["latest_failure"]["reason"], "execution_failed");
    assert!(delegation["latest_failure"]["summary"]
        .as_str()
        .unwrap_or_default()
        .contains("terminal_event=turn.failed"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn build_worker_completion_snapshot_classifies_transport_errors_as_execution_failed() {
    let workspace_dir = create_temp_path("worker-transport-error");
    create_dir_all(&workspace_dir).expect("create workspace");
    let raw_events_file = workspace_dir.join("transport-error.jsonl");
    write(
            &raw_events_file,
            "2026-04-23T07:14:11.966922Z ERROR codex_models_manager::manager: failed to refresh available models: stream disconnected before completion\n{\"type\":\"thread.started\",\"thread_id\":\"thread-transport\"}\n{\"type\":\"turn.started\"}\n{\"type\":\"error\",\"message\":\"Reconnecting... 2/5 (stream disconnected before completion: dns error)\"}\n",
        )
        .expect("write transport error raw-events");

    let (_status, summary, _worker_result, latest_failure) =
        build_worker_completion_snapshot(&raw_events_file, "2026-04-23T07:14:15.059Z", None);
    assert!(summary.contains("raw_events_bytes="));
    assert_eq!(latest_failure["reason"], "execution_failed");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn refresh_running_delegation_heartbeat_fails_non_json_raw_events() {
    let workspace_dir = create_temp_path("heartbeat-warning");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-heartbeat-warning");
    create_dir_all(run_directory.join("delegations")).expect("create delegations");
    create_dir_all(run_directory.join("raw-events")).expect("create raw-events");

    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("true")
        .spawn()
        .expect("spawn worker");
    let pid = child.id();
    let _ = child.wait();

    let raw_events_file = run_directory.join("raw-events").join("warning.jsonl");
    write(
            &raw_events_file,
            "WARNING: proceeding, even though we could not update PATH: Operation not permitted (os error 1)\nReading additional input from stdin...\n",
        )
        .expect("write warning raw-events");

    let delegation_file = run_directory.join("delegations").join("warning.json");
    write(
        &delegation_file,
        serde_json::to_vec_pretty(&json!({
            "delegation_id": "warning",
            "run_id": "run-heartbeat-warning",
            "task_card_id": "task-1",
            "delegated_by_role": "orchestrator",
            "review_round": null,
            "summary": "worker warning only",
            "child_agent": {
                "agent_id": "raider",
                "parent_agent_id": "captain",
                "role": "code specialist",
                "status": "running",
                "task_card_id": "task-1"
            },
            "executor": {
                "executor_id": "specialist-executor:raider",
                "status": "running",
                "task_card_id": "task-1",
                "delegation_id": "warning",
                "child_agent_id": "raider"
            },
            "worker_request": {
                "prompt": "Implement the bounded task",
                "acceptance": "Return a bounded result"
            },
            "worker_launch_evidence": {
                "raw_events_file": raw_events_file.to_string_lossy(),
            },
            "worker_lifecycle": {
                "state": "running",
                "reclaim_state": "not_needed",
                "queued_at": "2026-04-22T07:00:00.000Z",
                "launch_requested_at": "2026-04-22T07:00:01.000Z",
                "started_at": "2026-04-22T07:00:02.000Z",
                "process_id": pid,
                "process_started_at": "2026-04-22T07:00:02.000Z",
                "process_last_seen_at": "2026-04-22T07:00:03.000Z",
                "last_progress_at": "2026-04-22T07:00:03.000Z",
                "returned_at": null,
                "stale_at": null,
                "timed_out_at": null,
                "stale_after_ms": 45000,
                "timeout_after_ms": 45000,
                "summary": "running"
            },
            "worker_result": null,
            "result_summary": null,
            "reviewer_outcome": null,
            "latest_failure": null,
            "created_at": "2026-04-22T07:00:00.000Z",
            "updated_at": "2026-04-22T07:00:03.000Z",
            "completed_at": null
        }))
        .expect("serialize warning delegation"),
    )
    .expect("write warning delegation");

    let refreshed = refresh_running_delegation_heartbeat(
        &run_directory,
        &delegation_file,
        read_json_document(&delegation_file).expect("delegation payload"),
    )
    .expect("refresh delegation");

    assert_eq!(refreshed["child_agent"]["status"], "failed");
    assert_eq!(refreshed["executor"]["status"], "failed");
    assert_eq!(refreshed["worker_result"]["status"], "failed");
    assert!(refreshed["worker_result"]["thread_id"].is_null());
    assert!(refreshed["worker_result"]["total_token_usage"].is_null());
    assert!(
        refreshed["worker_result"]["raw_output_bytes"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert_eq!(
            refreshed["worker_result"]["raw_output_preview"],
            "WARNING: proceeding, even though we could not update PATH: Operation not permitted (os error 1) Reading additional input from stdin..."
        );
    assert_eq!(refreshed["latest_failure"]["reason"], "invalid_output");
    assert!(refreshed["latest_failure"]["summary"]
        .as_str()
        .unwrap_or_default()
        .contains("raw_events_bytes="));
    assert_eq!(refreshed["worker_lifecycle"]["state"], "failed");
    assert_eq!(
        create_token_usage_payload(&run_directory).expect("token usage"),
        Value::Null
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_aggregates_run_wide_token_usage_across_agents_and_task_cards() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-token-usage");
    create_dir_all(&workspace_dir).expect("create workspace");
    let raider_codex = create_fake_codex_executable_with_usage(
        &workspace_dir,
        "fake-raider-codex.sh",
        "thread-raider",
        "Raider completed the bounded task.",
        1200,
        300,
        200,
        100,
    );
    let scout_codex = create_fake_codex_executable_with_usage(
        &workspace_dir,
        "fake-scout-codex.sh",
        "thread-scout",
        "Scout gathered the bounded evidence.",
        900,
        100,
        250,
        50,
    );

    let start_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "tools/call",
            "params": {
                "name": "ccc_start",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "goal": "Implement a bounded parser change before a follow-up inspection",
                    "title": "Implement the next bounded step",
                    "intent": "Create a run without invoking Codex",
                    "scope": "Single execution task only",
                    "acceptance": "Persist run bootstrap artifacts",
                    "prompt": "Implement the first bounded parser change",
                    "task_kind": "execution",
                    "sequence": "EXECUTE_SEQUENCE"
                }
            }
        }),
    )
    .expect("start response");
    let run_id = start_response["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id");
    let task_card_id = start_response["result"]["structuredContent"]["task_card_id"]
        .as_str()
        .expect("task card id");
    let run_directory = PathBuf::from(
        start_response["result"]["structuredContent"]["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);

    let first_orchestrate = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 24,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": raider_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("first orchestrate");
    assert!(
        first_orchestrate.get("error").is_none(),
        "unexpected first orchestrate response: {first_orchestrate:?}"
    );

    let delegations_path = run_directory.join("delegations");
    let wait_deadline = SystemTime::now() + Duration::from_secs(3);
    loop {
        let completed = fs::read_dir(&delegations_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| read_json_document(&entry.path()).ok())
            .any(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    == Some("completed")
            });
        if completed || SystemTime::now() >= wait_deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let collapse_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 25,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("collapse response");
    assert!(
        collapse_response.get("error").is_none(),
        "unexpected collapse response: {collapse_response:?}"
    );
    assert_eq!(
        collapse_response["result"]["structuredContent"]["next_step"],
        "advance"
    );

    let replan_response = handle_message(
            &session_context,
            json!({
                "jsonrpc": "2.0",
                "id": 26,
                "method": "tools/call",
                "params": {
                    "name": "ccc_orchestrate",
                    "arguments": {
                        "run_id": run_id,
                        "cwd": workspace_dir.to_string_lossy(),
                        "repair_action": "scout",
                        "replan_prompt": "Inspect the workspace files only and report the result back to captain.",
                        "resolve_summary": "Inspect workspace files"
                    }
                }
            }),
        )
        .expect("replan response");
    assert!(
        replan_response.get("error").is_none(),
        "unexpected replan response: {replan_response:?}"
    );
    assert_eq!(
        replan_response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    let follow_up_task_card_id = replan_response["result"]["structuredContent"]
        ["current_task_card"]["task_card_id"]
        .as_str()
        .expect("follow-up task card id");
    mark_task_card_codex_exec_fallback(&run_directory, follow_up_task_card_id);

    let scout_launch_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 27,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "codex_bin": scout_codex.to_string_lossy(),
                    "progression_mode": "single_step"
                }
            }
        }),
    )
    .expect("scout launch response");
    assert!(
        scout_launch_response.get("error").is_none(),
        "unexpected scout launch response: {scout_launch_response:?}"
    );
    assert_eq!(
        scout_launch_response["result"]["structuredContent"]["next_step"],
        "await_fan_in"
    );

    let wait_deadline = SystemTime::now() + Duration::from_secs(1);
    loop {
        let completed = fs::read_dir(&delegations_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| read_json_document(&entry.path()).ok())
            .filter(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("agent_id"))
                    .and_then(Value::as_str)
                    == Some("scout")
            })
            .any(|delegation| {
                delegation
                    .get("child_agent")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    == Some("completed")
            });
        if completed || SystemTime::now() >= wait_deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let status_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 28,
            "method": "tools/call",
            "params": {
                "name": "ccc_status",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("status response");
    let status = &status_response["result"]["structuredContent"]["status"];
    assert_eq!(status["current_task_card"]["assigned_agent_id"], "scout");
    assert_eq!(status["token_usage"]["total_tokens"], 3100);
    assert_eq!(
        status["token_usage"]["by_agent"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        status["token_usage"]["by_subagent"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        status["token_usage"]["by_agent"][0]["agent_id"]
            .as_str()
            .unwrap_or_default(),
        "raider"
    );
    assert_eq!(status["token_usage"]["by_agent"][0]["total_tokens"], 1800);
    assert_eq!(
        status["token_usage"]["by_agent"][1]["agent_id"]
            .as_str()
            .unwrap_or_default(),
        "scout"
    );
    assert_eq!(status["token_usage"]["by_agent"][1]["total_tokens"], 1300);
    assert_eq!(
        status["token_usage"]["by_subagent"][0]["total_tokens"],
        1800
    );
    assert_eq!(
        status["token_usage"]["by_subagent"][1]["total_tokens"],
        1300
    );

    let status_text = status_response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(!status_text.contains("Tokens: 3.1k used"));
    assert!(!status_text.contains("By Agent:"));
    assert!(!status_text.contains("Gauge:"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_advance_with_replan_creates_follow_up_task_card() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-replan");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-replan");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-replan",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "advance",
                "can_advance": true,
                "summary": "captain checkpoint"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");

    let response = handle_message(
            &session_context,
            json!({
                "jsonrpc": "2.0",
                "id": 24,
                "method": "tools/call",
                "params": {
                    "name": "ccc_orchestrate",
                    "arguments": {
                        "run_id": "run-replan",
                        "cwd": workspace_dir.to_string_lossy(),
                        "repair_action": "scout",
                        "replan_prompt": "Inspect the workspace files only and report the result back to captain.",
                        "resolve_summary": "Inspect workspace files"
                    }
                }
            }),
        )
        .expect("replan response");
    assert!(
        response.get("error").is_none(),
        "unexpected replan response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    assert_eq!(response["result"]["structuredContent"]["can_advance"], true);
    assert_eq!(
        response["result"]["structuredContent"]["scheduler_decision"]["action"]["kind"],
        "replan"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    assert_ne!(active_task_card_id, "task-1");
    assert_eq!(run_record["active_agent_id"], "captain");
    assert_eq!(run_record["active_role"], "orchestrator");

    let follow_up_task = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{active_task_card_id}.json")),
    )
    .expect("follow-up task card");
    assert_eq!(follow_up_task["assigned_role"], "explorer");
    assert_eq!(follow_up_task["assigned_agent_id"], "scout");
    assert_eq!(
        follow_up_task["execution_prompt"],
        "Inspect the workspace files only and report the result back to captain."
    );

    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    assert_eq!(run_state["next_action"]["command"], "execute_task");
    assert_eq!(run_state["current_phase_name"], "inspect");

    let longway = read_json_document(&run_directory.join("longway.json")).expect("LongWay");
    assert_eq!(longway["active_phase_name"], "inspect");
    assert_eq!(longway["phases"].as_array().map(Vec::len), Some(2));
    assert_eq!(longway["phases"][0]["status"], "completed");
    assert_eq!(longway["phases"][1]["status"], "pending");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_advance_with_retry_records_scheduler_action() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-retry-scheduler");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-retry-scheduler");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-retry-scheduler",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "advance",
                "can_advance": true,
                "summary": "captain checkpoint"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 224,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": "run-retry-scheduler",
                    "cwd": workspace_dir.to_string_lossy(),
                    "repair_action": "retry"
                }
            }
        }),
    )
    .expect("retry response");
    assert!(
        response.get("error").is_none(),
        "unexpected retry response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    assert_eq!(
        response["result"]["structuredContent"]["scheduler_decision"]["action"]["kind"],
        "retry"
    );
    assert_eq!(
        response["result"]["structuredContent"]["scheduler_decision"]["blocked"]
            ["retry_current_specialist"],
        true
    );

    let attempt = read_json_document(
        &run_directory
            .join("orchestration")
            .join("attempts")
            .join("attempt-0001.json"),
    )
    .expect("attempt payload");
    assert_eq!(attempt["scheduler_decision"]["action"]["kind"], "retry");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_consumes_queued_same_specialist_follow_up() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-pending-same-specialist");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) =
        start_intervention_test_run(&workspace_dir, "pending-same-specialist");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-pending-same-specialist",
        "status": "completed",
        "summary": "Raider missed one bounded assertion.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": ["Need the missing assertion only."],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "The same specialist can apply one narrowed amendment.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));
    let source_task_card_id = update_payload["task_card_id"]
        .as_str()
        .expect("source task-card id")
        .to_string();
    let dedupe_key = update_payload["captain_intervention"]["pending_follow_up"]["dedupe_key"]
        .as_str()
        .expect("dedupe key")
        .to_string();
    force_run_to_captain_advance(&run_directory);

    let response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 224);
    assert!(
        response.get("error").is_none(),
        "unexpected orchestrate response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    assert_eq!(
        response["result"]["structuredContent"]["consumed_pending_follow_up"]["dedupe_key"],
        dedupe_key
    );
    assert_eq!(
        response["result"]["structuredContent"]["consumed_pending_follow_up"]["status"],
        "consumed"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task-card id");
    assert_ne!(active_task_card_id, source_task_card_id);
    let follow_up_task = read_active_task_card(&run_directory);
    assert_eq!(follow_up_task["assigned_role"], "code specialist");
    assert_eq!(follow_up_task["assigned_agent_id"], "raider");
    assert!(follow_up_task["execution_prompt"]
        .as_str()
        .unwrap_or_default()
        .contains("Bounded same-specialist amendment only."));
    assert_eq!(
        follow_up_task["captain_follow_up"]["source_task_card_id"],
        source_task_card_id
    );
    assert_eq!(follow_up_task["captain_follow_up"]["budget_key"], "retry");
    assert_eq!(
        follow_up_task["captain_follow_up"]["dedupe_key"],
        dedupe_key
    );
    assert_eq!(follow_up_task["captain_follow_up"]["status"], "consumed");

    let source_task = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{source_task_card_id}.json")),
    )
    .expect("source task-card");
    assert_eq!(
        source_task["captain_intervention"]["pending_follow_up"]["status"],
        "consumed"
    );
    assert_eq!(
        source_task["captain_intervention"]["pending_follow_up"]["consumed_task_card_id"],
        active_task_card_id
    );

    let status_payload = create_ccc_status_payload(
        &session_context,
        &ResolvedRunLocator {
            cwd: workspace_dir.clone(),
            run_id,
            run_directory: run_directory.clone(),
        },
    )
    .expect("status payload");
    assert_eq!(status_payload["pending_captain_follow_up"], Value::Null);
    assert_eq!(
        status_payload["latest_captain_intervention"]["pending_follow_up"]["status"],
        "consumed"
    );
    assert!(
        create_ccc_status_text(&status_payload).contains("follow_up=amend-same-worker:consumed")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_never_assigns_repair_follow_up_to_captain_or_orchestrator() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-pending-no-captain-repair");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "no-captain-repair");

    let active_task_card = read_active_task_card(&run_directory);
    let active_task_card_id = active_task_card["task_card_id"]
        .as_str()
        .expect("active task-card id");
    let task_card_file = run_directory
        .join("task-cards")
        .join(format!("{active_task_card_id}.json"));
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["assigned_role"] = json!("orchestrator");
    task_card["assigned_agent_id"] = json!("captain");
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "captain",
        "thread_id": "thread-no-captain-repair",
        "status": "completed",
        "summary": "Captain-owned task reported a repair need.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": ["Route repair to a specialist."],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "Repair must be delegated instead of owned by captain.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_role"],
        "code specialist"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "raider"
    );

    force_run_to_captain_advance(&run_directory);
    let response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 226);
    assert!(
        response.get("error").is_none(),
        "unexpected orchestrate response: {response:?}"
    );
    let follow_up_task = read_active_task_card(&run_directory);
    assert_eq!(follow_up_task["assigned_role"], "code specialist");
    assert_eq!(follow_up_task["assigned_agent_id"], "raider");
    assert_ne!(follow_up_task["assigned_role"], "orchestrator");
    assert_ne!(follow_up_task["assigned_agent_id"], "captain");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_consumes_queued_reassign_follow_up() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-pending-reassign");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) = start_intervention_test_run(&workspace_dir, "pending-reassign");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-pending-reassign",
        "status": "failed",
        "summary": "Raider pursued the wrong ownership boundary.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["docs/project-plan.md:180"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "Captain should reassign this to the explorer.",
        "chosen_next_action": "reassign",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        },
        "reassign_target": {
            "assigned_role": "explorer",
            "assigned_agent_id": "scout",
            "scope": "Inspect ownership boundary before another mutation.",
            "prompt": "Inspect the ownership boundary and report the smallest safe reassignment plan."
        }
    }));
    let source_task_card_id = update_payload["task_card_id"]
        .as_str()
        .expect("source task-card id")
        .to_string();
    let dedupe_key = update_payload["captain_intervention"]["pending_follow_up"]["dedupe_key"]
        .as_str()
        .expect("dedupe key")
        .to_string();
    force_run_to_captain_advance(&run_directory);

    let response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 225);
    assert!(
        response.get("error").is_none(),
        "unexpected orchestrate response: {response:?}"
    );
    let follow_up_task = read_active_task_card(&run_directory);
    assert_eq!(follow_up_task["assigned_role"], "explorer");
    assert_eq!(follow_up_task["assigned_agent_id"], "scout");
    assert_eq!(
        follow_up_task["scope"],
        "Inspect ownership boundary before another mutation."
    );
    assert_eq!(
        follow_up_task["execution_prompt"],
        "Inspect the ownership boundary and report the smallest safe reassignment plan."
    );
    assert_eq!(
        follow_up_task["captain_follow_up"]["source_task_card_id"],
        source_task_card_id
    );
    assert_eq!(
        follow_up_task["captain_follow_up"]["budget_key"],
        "reassign"
    );
    assert_eq!(
        follow_up_task["captain_follow_up"]["dedupe_key"],
        dedupe_key
    );
    assert_eq!(follow_up_task["captain_follow_up"]["status"], "consumed");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_never_assigns_reassign_follow_up_to_captain_or_orchestrator() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-pending-no-captain-reassign");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) =
        start_intervention_test_run(&workspace_dir, "no-captain-reassign");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-no-captain-reassign",
        "status": "failed",
        "summary": "The repair target was incorrectly set to captain.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["docs/project-plan.md:180"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "low",
        "intervention_classification": "direction_or_risk_correction",
        "intervention_rationale": "Captain cannot own the bounded repair follow-up.",
        "chosen_next_action": "reassign",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 1, "remaining": 0 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        },
        "reassign_target": {
            "assigned_role": "orchestrator",
            "assigned_agent_id": "captain",
            "scope": "Repair must remain specialist-owned.",
            "prompt": "Apply only the bounded repair."
        }
    }));
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_role"],
        "code specialist"
    );
    assert_eq!(
        update_payload["captain_intervention"]["pending_follow_up"]["assigned_agent_id"],
        "raider"
    );

    force_run_to_captain_advance(&run_directory);
    let response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 227);
    assert!(
        response.get("error").is_none(),
        "unexpected orchestrate response: {response:?}"
    );
    let follow_up_task = read_active_task_card(&run_directory);
    assert_eq!(follow_up_task["assigned_role"], "code specialist");
    assert_eq!(follow_up_task["assigned_agent_id"], "raider");
    assert_eq!(
        follow_up_task["scope"],
        "Repair must remain specialist-owned."
    );
    assert_eq!(
        follow_up_task["execution_prompt"],
        "Apply only the bounded repair."
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_repeated_consumption_does_not_duplicate_follow_up_task_card() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-pending-no-duplicate");
    create_dir_all(&workspace_dir).expect("create workspace");
    let (run_id, run_directory) =
        start_intervention_test_run(&workspace_dir, "pending-no-duplicate");

    let update_payload = parse_and_record_subagent_update(json!({
        "run_id": run_id,
        "cwd": workspace_dir.to_string_lossy(),
        "child_agent_id": "raider",
        "thread_id": "thread-pending-no-duplicate",
        "status": "completed",
        "summary": "Raider returned an incomplete repair.",
        "fan_in_status": "needs_work",
        "evidence_paths": ["rust/ccc-mcp/src/main.rs:14000"],
        "next_action": "captain_decision",
        "open_questions": [],
        "confidence": "medium",
        "intervention_classification": "bounded_scope_amendment",
        "intervention_rationale": "One same-specialist amendment is enough.",
        "chosen_next_action": "amend_same_worker",
        "budget_snapshot": {
            "retry": { "limit": 1, "used": 0, "remaining": 1 },
            "reassign": { "limit": 1, "used": 0, "remaining": 1 }
        }
    }));
    let dedupe_key = update_payload["captain_intervention"]["pending_follow_up"]["dedupe_key"]
        .as_str()
        .expect("dedupe key")
        .to_string();
    force_run_to_captain_advance(&run_directory);

    let first_response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 226);
    assert!(
        first_response.get("error").is_none(),
        "unexpected first orchestrate response: {first_response:?}"
    );
    let first_active_task_card_id = read_json_document(&run_directory.join("run.json"))
        .expect("run record")["active_task_card_id"]
        .as_str()
        .expect("active task-card id")
        .to_string();
    let second_response = call_ccc_orchestrate_tool(&session_context, &run_id, &workspace_dir, 227);
    assert!(
        second_response.get("error").is_none(),
        "unexpected second orchestrate response: {second_response:?}"
    );
    let second_active_task_card_id = read_json_document(&run_directory.join("run.json"))
        .expect("run record")["active_task_card_id"]
        .as_str()
        .expect("active task-card id")
        .to_string();
    assert_eq!(first_active_task_card_id, second_active_task_card_id);
    assert_eq!(
        task_cards_with_captain_follow_up_dedupe_key(&run_directory, &dedupe_key).len(),
        1
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_advance_with_companion_reader_replan_creates_follow_up_task_card() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-companion-reader-replan");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-companion-reader-replan");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-companion-reader-replan",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "advance",
                "can_advance": true,
                "summary": "captain checkpoint"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");

    let response = handle_message(
            &session_context,
            json!({
                "jsonrpc": "2.0",
                "id": 124,
                "method": "tools/call",
                "params": {
                    "name": "ccc_orchestrate",
                    "arguments": {
                        "run_id": "run-companion-reader-replan",
                        "cwd": workspace_dir.to_string_lossy(),
                        "repair_action": "companion_reader",
                        "replan_prompt": "Inspect the current directory and summarize unnecessary files only.",
                        "resolve_summary": "Inspect workspace files with companion reader"
                    }
                }
            }),
        )
        .expect("replan response");
    assert!(
        response.get("error").is_none(),
        "unexpected replan response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    assert_ne!(active_task_card_id, "task-1");

    let follow_up_task = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{active_task_card_id}.json")),
    )
    .expect("follow-up task card");
    assert_eq!(follow_up_task["assigned_role"], "companion_reader");
    assert_eq!(follow_up_task["assigned_agent_id"], "companion_reader");
    assert_eq!(follow_up_task["sandbox_mode"], "read-only");
    assert_eq!(
        follow_up_task["role_config_snapshot"]["model"],
        expected_role_config_field("companion_reader", "model")
    );
    assert_eq!(
        follow_up_task["role_config_snapshot"]["variant"],
        expected_role_config_field("companion_reader", "variant")
    );

    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    assert_eq!(run_state["next_action"]["command"], "execute_task");
    assert_eq!(run_state["current_phase_name"], "inspect");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_await_fan_in_with_replan_collapses_and_replans_in_one_call() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("await-fan-in-replan");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-await-fan-in-replan");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-await-fan-in-replan",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "await_fan_in",
                "can_advance": true,
                "summary": "waiting on a worker result"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");
    write(
        run_directory.join("run-state.json"),
        serde_json::to_vec_pretty(&json!({
            "current_phase_name": "way",
            "event_count": 1,
            "last_event_id": "event-0001",
            "next_action": {
                "command": "await_fan_in"
            },
            "updated_at": "2026-04-22T07:00:00.000Z"
        }))
        .expect("serialize run-state"),
    )
    .expect("write run-state");
    create_dir_all(run_directory.join("delegations")).expect("create delegations");
    create_dir_all(run_directory.join("raw-events")).expect("create raw-events");
    write(
        run_directory
            .join("raw-events")
            .join("delegation-0001.jsonl"),
        "",
    )
    .expect("write raw-events");
    write(
            run_directory.join("delegations").join("delegation-0001.json"),
            serde_json::to_vec_pretty(&json!({
                "delegation_id": "delegation-0001",
                "run_id": "run-await-fan-in-replan",
                "task_card_id": "task-1",
                "delegated_by_role": "orchestrator",
                "review_round": null,
                "summary": "worker finished",
                "child_agent": {
                    "agent_id": "tactician",
                    "parent_agent_id": "captain",
                    "role": "way",
                    "status": "failed",
                    "task_card_id": "task-1"
                },
                "executor": {
                    "executor_id": "specialist-executor:tactician",
                    "status": "failed",
                    "task_card_id": "task-1",
                    "delegation_id": "delegation-0001",
                    "child_agent_id": "tactician"
                },
                "worker_request": {
                    "prompt": "Inspect and report",
                    "acceptance": "Return a bounded result"
                },
                "worker_launch_evidence": {
                    "raw_events_file": run_directory.join("raw-events").join("delegation-0001.jsonl").to_string_lossy(),
                },
                "worker_lifecycle": {
                    "state": "failed",
                    "reclaim_state": "not_needed",
                    "queued_at": "2026-04-22T07:00:00.000Z",
                    "launch_requested_at": "2026-04-22T07:00:01.000Z",
                    "started_at": "2026-04-22T07:00:02.000Z",
                    "process_id": null,
                    "process_started_at": "2026-04-22T07:00:02.000Z",
                    "process_last_seen_at": "2026-04-22T07:00:03.000Z",
                    "last_progress_at": "2026-04-22T07:00:03.000Z",
                    "returned_at": "2026-04-22T07:00:04.000Z",
                    "stale_at": null,
                    "timed_out_at": null,
                    "stale_after_ms": 45000,
                    "timeout_after_ms": 45000,
                    "summary": "failed and needs captain follow-up"
                },
                "worker_result": {
                    "status": "failed",
                    "recorded_at": "2026-04-22T07:00:04.000Z",
                    "thread_id": null,
                    "assistant_message_preview": null,
                    "total_token_usage": null
                },
                "result_summary": "Worker exited without parseable Codex result or usage artifacts.",
                "reviewer_outcome": null,
                "latest_failure": {
                    "stage": "execution",
                    "reason": "unknown",
                    "summary": "Worker exited without parseable Codex result or usage artifacts.",
                    "recorded_at": "2026-04-22T07:00:04.000Z"
                },
                "created_at": "2026-04-22T07:00:00.000Z",
                "updated_at": "2026-04-22T07:00:04.000Z",
                "completed_at": "2026-04-22T07:00:04.000Z"
            }))
            .expect("serialize delegation"),
        )
        .expect("write delegation");

    let response = handle_message(
            &session_context,
            json!({
                "jsonrpc": "2.0",
                "id": 44,
                "method": "tools/call",
                "params": {
                    "name": "ccc_orchestrate",
                    "arguments": {
                        "run_id": "run-await-fan-in-replan",
                        "cwd": workspace_dir.to_string_lossy(),
                        "repair_action": "scout",
                        "replan_prompt": "Inspect the workspace files only and report the result back to captain.",
                        "resolve_summary": "Inspect workspace files"
                    }
                }
            }),
        )
        .expect("await-fan-in replan response");
    assert!(
        response.get("error").is_none(),
        "unexpected await-fan-in replan response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "execute_task"
    );
    assert_eq!(response["result"]["structuredContent"]["can_advance"], true);
    assert!(response["result"]["structuredContent"]["summary"]
        .as_str()
        .unwrap_or_default()
        .contains("collapsed explicit fan-in"));

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    let active_task_card_id = run_record["active_task_card_id"]
        .as_str()
        .expect("active task card id");
    assert_ne!(active_task_card_id, "task-1");

    let follow_up_task = read_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{active_task_card_id}.json")),
    )
    .expect("follow-up task card");
    assert_eq!(follow_up_task["assigned_role"], "explorer");
    assert_eq!(follow_up_task["assigned_agent_id"], "scout");

    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    assert_eq!(run_state["next_action"]["command"], "execute_task");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_advance_with_resolve_outcome_closes_run() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("advance-resolve");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-resolve");
    let task_card_path = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_path).expect("task card");
    task_card["title"] = Value::String("Implement 0.0.9 pre-release plan".to_string());
    task_card["review_policy"] = json!({
        "decision": "skip",
        "state": "suppressed",
        "risk": "low",
        "summary": "Resolve mechanics test explicitly suppresses follow-up review."
    });
    write_json_document(&task_card_path, &task_card).expect("write task card");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "execute",
            "active_phase_status": "todo",
            "phases": [{
                "phase_name": "way",
                "status": "todo"
            }]
        }),
    )
    .expect("write longway");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": "run-resolve",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "advance",
                "can_advance": true,
                "summary": "captain checkpoint"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 25,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": "run-resolve",
                    "cwd": workspace_dir.to_string_lossy(),
                    "resolve_outcome": "completed",
                    "resolve_summary": "Captain verified the bounded task and closed the run."
                }
            }
        }),
    )
    .expect("resolve response");
    assert!(
        response.get("error").is_none(),
        "unexpected resolve response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "halt_completed"
    );
    assert_eq!(
        response["result"]["structuredContent"]["can_advance"],
        false
    );
    assert_eq!(
        response["result"]["structuredContent"]["scheduler_decision"]["action"]["kind"],
        "complete"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(run_record["status"], "completed");
    assert!(run_record["completed_at"].is_string());
    assert_eq!(
        run_record["latest_orchestrator_synthesis"],
        "Captain verified the bounded task and closed the run."
    );

    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    assert_eq!(run_state["next_action"]["command"], "halt_completed");

    let longway = read_json_document(&run_directory.join("longway.json")).expect("LongWay");
    assert_eq!(longway["lifecycle_state"], "completed");
    assert_eq!(longway["active_phase_status"], "completed");
    assert_eq!(longway["phases"][0]["status"], "completed");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-resolve",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    assert_eq!(
        checklist_payload["checklist"],
        "LongWay\n[x] Implement 0.0.9 pre-release plan"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_resolves_completed_task_card_fan_in_without_delegation_file() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("task-card-fan-in-resolve");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-task-card-fan-in");
    let task_card_path = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_path).expect("task card");
    task_card["title"] = Value::String("0.0.11 installed smoke".to_string());
    task_card["review_policy"] = json!({
        "decision": "skip",
        "state": "suppressed",
        "risk": "low",
        "summary": "Diagnostic fan-in closeout explicitly suppresses follow-up review."
    });
    task_card["verification_state"] = Value::String("passed".to_string());
    task_card["worker_result_envelope"] = json!({
        "schema": "ccc.worker_result_envelope.v1",
        "summary": "Installed CCC smoke passed after Codex app restart.",
        "status": "completed",
        "evidence_paths": ["CCC_LONGWAY_PANEL.md", "CCC_LONGWAY_PANEL.json"],
        "next_action": "complete",
        "open_questions": [],
        "confidence": "high",
        "checks": ["ccc check-install", "ccc status --app-panel"],
        "contract": {
            "captain_consumes_compact_fan_in": true
        }
    });
    task_card["subagent_fan_in"] = task_card["worker_result_envelope"].clone();
    write_json_document(&task_card_path, &task_card).expect("write task card");
    write_json_document(
        &run_directory.join("run-state.json"),
        &json!({
            "version": 1,
            "run_id": "run-task-card-fan-in",
            "updated_at": "2026-05-04T06:41:50.833Z",
            "event_count": 4,
            "last_event_id": "event-0004",
            "current_phase_name": "fan_in",
            "next_action": {
                "command": "await_fan_in"
            }
        }),
    )
    .expect("write run-state");
    write_json_document(
        &run_directory.join("orchestrator-state.json"),
        &json!({
            "run_id": "run-task-card-fan-in",
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "await_fan_in",
                "can_advance": true,
                "summary": "fan-in is ready"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }),
    )
    .expect("write orchestrator-state");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "inspect",
            "active_phase_status": "pending",
            "phases": [{
                "phase_name": "inspect",
                "status": "pending"
            }]
        }),
    )
    .expect("write longway");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 125,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": "run-task-card-fan-in",
                    "cwd": workspace_dir.to_string_lossy(),
                    "resolve_outcome": "completed",
                    "resolve_summary": "Installed smoke passed; close the diagnostic run."
                }
            }
        }),
    )
    .expect("resolve task-card fan-in response");
    assert!(
        response.get("error").is_none(),
        "unexpected task-card fan-in resolve response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["starting_next_step"],
        "await_fan_in"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        "halt_completed"
    );
    assert_eq!(
        response["result"]["structuredContent"]["can_advance"],
        false
    );
    assert_eq!(
        response["result"]["structuredContent"]["scheduler_decision"]["action"]["kind"],
        "complete"
    );
    assert_eq!(
        response["result"]["structuredContent"]["collapsed_fan_in"]["completed"],
        1
    );
    assert_eq!(
        response["result"]["structuredContent"]["consumed_worker_result_envelope"]["source"],
        "current_task_card.worker_result_envelope"
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(run_record["status"], "completed");
    assert!(run_record["completed_at"].is_string());
    let run_state = read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    assert_eq!(run_state["next_action"]["command"], "halt_completed");
    let task_card = read_json_document(&task_card_path).expect("resolved task card");
    assert_eq!(task_card["status"], "completed");
    assert_eq!(task_card["verification_state"], "passed");
    assert!(task_card["completed_at"].is_string());
    let longway = read_json_document(&run_directory.join("longway.json")).expect("LongWay");
    assert_eq!(longway["lifecycle_state"], "completed");

    let status_response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 126,
            "method": "tools/call",
            "params": {
                "name": "ccc_status",
                "arguments": {
                    "run_id": "run-task-card-fan-in",
                    "cwd": workspace_dir.to_string_lossy()
                }
            }
        }),
    )
    .expect("status response");
    assert!(
        status_response.get("error").is_none(),
        "unexpected status response: {status_response:?}"
    );
    assert_eq!(
        status_response["result"]["structuredContent"]["status"]["status"],
        "completed"
    );
    assert_eq!(
        status_response["result"]["structuredContent"]["status"]["app_panel"]["run"]["status"],
        "completed"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

fn assert_resolve_outcome_renders_terminal_checklist_symbol(
    label: &str,
    resolve_outcome: &str,
    expected_run_status: &str,
    expected_next_step: &str,
    expected_phase_status: &str,
    expected_checklist: &str,
) {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path(label);
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_id = format!("run-{label}");
    let run_directory = write_test_run_fixture(&workspace_dir, &run_id);
    let task_card_path = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_path).expect("task card");
    task_card["title"] = Value::String("Implement 0.0.9 pre-release plan".to_string());
    write_json_document(&task_card_path, &task_card).expect("write task card");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "execute",
            "active_phase_status": "todo",
            "phases": [{
                "phase_name": "way",
                "status": "todo"
            }]
        }),
    )
    .expect("write longway");
    write(
        run_directory.join("orchestrator-state.json"),
        serde_json::to_vec_pretty(&json!({
            "run_id": run_id,
            "task_card_id": "task-1",
            "execution_request": null,
            "verification_request": null,
            "decision": {
                "next_step": "advance",
                "can_advance": true,
                "summary": "captain checkpoint"
            },
            "orchestration_policy": {
                "autonomous_research": {
                    "mode": "disabled"
                }
            }
        }))
        .expect("serialize orchestrator-state"),
    )
    .expect("write orchestrator-state");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 25,
            "method": "tools/call",
            "params": {
                "name": "ccc_orchestrate",
                "arguments": {
                    "run_id": run_id,
                    "cwd": workspace_dir.to_string_lossy(),
                    "resolve_outcome": resolve_outcome,
                    "resolve_summary": format!("Captain closed the run as {resolve_outcome}.")
                }
            }
        }),
    )
    .expect("resolve response");
    assert!(
        response.get("error").is_none(),
        "unexpected resolve response: {response:?}"
    );
    assert_eq!(
        response["result"]["structuredContent"]["next_step"],
        expected_next_step
    );

    let run_record = read_json_document(&run_directory.join("run.json")).expect("run payload");
    assert_eq!(run_record["status"], expected_run_status);
    assert!(run_record["completed_at"].is_string());

    let longway = read_json_document(&run_directory.join("longway.json")).expect("LongWay");
    assert_eq!(longway["lifecycle_state"], expected_phase_status);
    assert_eq!(longway["active_phase_status"], expected_phase_status);
    assert_eq!(longway["phases"][0]["status"], expected_phase_status);

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": run_id,
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let checklist_payload =
        create_ccc_checklist_payload(&session_context, &locator).expect("checklist payload");
    assert_eq!(checklist_payload["checklist"], expected_checklist);
    assert!(
        !checklist_payload["checklist"]
            .as_str()
            .unwrap_or_default()
            .contains("[x]"),
        "terminal non-completed resolve should not render completed checklist state"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_orchestrate_failed_resolve_renders_failed_checklist_symbol() {
    assert_resolve_outcome_renders_terminal_checklist_symbol(
        "advance-resolve-failed",
        "failed",
        "failed",
        "halt_failed",
        "failed",
        "LongWay\n[!] Implement 0.0.9 pre-release plan",
    );
}

#[test]
fn ccc_orchestrate_cancelled_resolve_renders_cancelled_checklist_symbol() {
    assert_resolve_outcome_renders_terminal_checklist_symbol(
        "advance-resolve-cancelled",
        "cancelled",
        "cancelled",
        "halt_cancelled",
        "cancelled",
        "LongWay\n[-] Implement 0.0.9 pre-release plan",
    );
}

#[test]
fn build_task_execution_prompt_omits_duplicate_scope_and_acceptance_lines() {
    let repeated =
        "Inspect the workspace files only and report the bounded result back to captain.";
    let prompt = build_task_execution_prompt(&json!({
        "title": "Inspect workspace files",
        "scope": repeated,
        "acceptance": repeated,
        "execution_prompt": repeated,
        "assigned_agent_id": "scout",
        "assigned_role": "explorer",
        "sandbox_mode": "read-only",
        "sandbox_rationale": "Scout work stays read-only."
    }));

    assert!(prompt.contains("Role: scout (explorer)."));
    assert!(prompt.contains("Task:\nInspect the workspace files only"));
    assert!(!prompt.contains("\nScope:"));
    assert!(!prompt.contains("\nAcceptance:"));
}

#[test]
fn build_task_execution_prompt_includes_external_path_guidance() {
    let prompt = build_task_execution_prompt(&json!({
        "title": "Backfill sample fixture",
        "scope": "Copy data from /tmp/sample-fixture.json into docs/examples/sample.json",
        "acceptance": "Use only the provided source path and report if blocked.",
        "execution_prompt": "Copy /tmp/sample-fixture.json into docs/examples/sample.json and summarize what changed.",
        "assigned_agent_id": "raider",
        "assigned_role": "code specialist",
        "sandbox_mode": "workspace-write",
        "sandbox_rationale": "Raider work may change code or config and needs workspace-write."
    }));

    assert!(prompt.contains("External paths:"));
    assert!(prompt.contains("exact path"));
    assert!(prompt.contains("approval needed"));
    assert!(prompt.contains("CCC boundary"));
    assert!(prompt.contains("supervisor persists fan-in"));
    assert!(prompt.contains("Anti-duplication: own only the delegated search/mutation scope"));
    assert!(prompt.contains("validation commands/checks and unresolved risk"));
    assert!(!prompt.contains("bounded task in this workspace"));
}

#[test]
fn build_task_execution_prompt_includes_default_commit_message_guidance() {
    let prompt = build_task_execution_prompt(&json!({
        "title": "Commit bounded fix",
        "scope": "Stage and commit only the bounded update.",
        "acceptance": "Commit output uses CCC default guidance unless the operator overrides it.",
        "execution_prompt": "Create the git commit for the completed slice.",
        "assigned_agent_id": "companion_operator",
        "assigned_role": "companion_operator",
        "sandbox_mode": "workspace-write",
        "sandbox_rationale": "Companion operator work may run bounded git commands."
    }));

    assert!(prompt.contains("Commit message guidance:"));
    assert!(prompt.contains("`fix(hub, worker): 비전 기본 가중치를 metric 0.4 text 0.6으로 조정`"));
    assert!(prompt.contains(
        "If the operator supplies a commit message/style/language instruction, that instruction wins."
    ));
}

#[test]
fn build_task_execution_prompt_includes_task_specific_expertise_framing() {
    let cases = [
        (
            "documenter",
            "scribe",
            "single_scoped_task",
            "release-note documentation and operator-facing clarity",
            "documentation_update",
            "style-and-audience-first",
        ),
        (
            "code specialist",
            "raider",
            "single_scoped_task",
            "bounded implementation, repair, module ownership, and focused validation",
            "bounded_implementation",
            "smallest-defensible-change",
        ),
        (
            "verifier",
            "arbiter",
            "single_scoped_task",
            "code review, regression detection, and acceptance risk",
            "findings_first_review",
            "findings-first-risk-review",
        ),
        (
            "companion_reader",
            "companion_reader",
            "single_scoped_task",
            "bounded operator evidence gathering and source-focused reading",
            "bounded_operator_reader",
            "bounded-reader-evidence-first",
        ),
        (
            "companion_operator",
            "companion_operator",
            "single_scoped_task",
            "bounded operator tool-operation and command-result clarity",
            "bounded_operator_execution",
            "command-boundary-first",
        ),
    ];

    for (
        assigned_role,
        assigned_agent_id,
        task_shape,
        expected_expertise,
        expected_stance,
        expected_thinking_mode,
    ) in cases
    {
        let prompt = build_task_execution_prompt(&json!({
            "title": format!("Exercise {assigned_role} framing"),
            "scope": "One bounded task.",
            "acceptance": "Return compact fan-in.",
            "execution_prompt": "Complete only the assigned bounded task.",
            "assigned_agent_id": assigned_agent_id,
            "assigned_role": assigned_role,
            "sandbox_mode": sandbox_mode_for_role(assigned_role),
            "sandbox_rationale": sandbox_rationale_for_role(assigned_role),
            "expertise_framing": task_expertise_framing_for_role(assigned_role, task_shape)
        }));

        assert!(
            prompt.contains(&format!("Role: {assigned_agent_id} ({assigned_role}).")),
            "{assigned_role}"
        );
        assert!(prompt.contains("Expertise: You are an expert in "));
        assert!(prompt.contains(expected_expertise), "{assigned_role}");
        assert!(
            prompt.contains(&format!(
                "Stance: {expected_stance}; mode: {expected_thinking_mode}."
            )),
            "{assigned_role}"
        );
    }
}

#[test]
fn delegation_contract_surfaces_task_specific_expertise_framing() {
    let cases = [
        (
            "documenter",
            "multi_step_or_unclear",
            "release-note documentation and operator-facing clarity",
            "documentation_update",
        ),
        (
            "code specialist",
            "single_scoped_task",
            "bounded implementation, repair, module ownership, and focused validation",
            "bounded_implementation",
        ),
        (
            "verifier",
            "single_scoped_task",
            "code review, regression detection, and acceptance risk",
            "findings_first_review",
        ),
        (
            "companion_reader",
            "single_scoped_task",
            "bounded operator evidence gathering and source-focused reading",
            "bounded_operator_reader",
        ),
        (
            "companion_operator",
            "single_scoped_task",
            "bounded operator tool-operation and command-result clarity",
            "bounded_operator_execution",
        ),
    ];

    for (assigned_role, task_shape, expected_expertise, expected_stance) in cases {
        let mut task_card = build_task_card_payload_with_role(
            "run-framing",
            "task-framing",
            "Frame subagent work",
            "Prove subagent prompt expertise framing.",
            "One bounded framing contract.",
            "Complete the assigned task.",
            "Return compact fan-in.",
            assigned_role,
            "2026-04-23T00:00:00.000Z",
        );
        apply_task_expertise_framing(&mut task_card, assigned_role, task_shape);

        assert!(
            task_card["expertise_framing"]["expertise_phrase"]
                .as_str()
                .unwrap()
                .contains(expected_expertise),
            "{assigned_role}"
        );
        assert_eq!(
            task_card["expertise_framing"]["task_stance"], expected_stance,
            "{assigned_role}"
        );
        assert_eq!(
            task_card["delegation_plan"]["expertise_framing"], task_card["expertise_framing"],
            "{assigned_role}"
        );
        assert_eq!(
            task_card["delegation_plan"]["subagent_spawn_contract"]["task_stance"], expected_stance,
            "{assigned_role}"
        );
        assert!(
            task_card["delegation_plan"]["subagent_spawn_contract"]["expertise_phrase"]
                .as_str()
                .unwrap()
                .contains(expected_expertise),
            "{assigned_role}"
        );
    }
}

#[test]
fn custom_agent_instructions_include_external_path_guidance() {
    let instructions = custom_agent_developer_instructions_for_role(
        "code specialist",
        &json!({
            "summary": "Bounded code and config mutation for implementation and repair."
        }),
    );
    assert!(instructions.contains("External paths:"));
    assert!(instructions.contains("use exact operator paths"));
    assert!(instructions.contains("approval needed"));
}

#[test]
fn custom_agent_instructions_include_role_specific_icl_guidance() {
    let captain = custom_agent_developer_instructions_for_role(
        "orchestrator",
        &json!({
            "summary": "Captain keeps the LongWay, selects specialists, and closes the loop."
        }),
    );
    assert!(captain.contains("1-3 questions"));
    assert!(captain.contains("proceed with assumptions"));
    assert!(captain.contains("apply_patch"));
    assert!(captain.contains("direct shell mutation"));
    assert!(captain.contains("terminal fallback/operator override"));
    assert!(captain.contains("If trivial/fallback work grows non-trivial"));
    assert!(captain.contains("If subagent capacity is exhausted"));
    assert!(captain.contains("close completed host threads"));
    assert!(captain.contains("delegate the follow-up"));
    assert!(captain.contains("Token discipline"));

    let tactician = custom_agent_developer_instructions_for_role(
        "way",
        &json!({
            "summary": "Way creation and bounded planning when the next move is still unclear."
        }),
    );
    assert!(tactician.contains("Compare options"));
    assert!(tactician.contains("parallel fit"));
    assert!(tactician.contains("operator decisions"));

    let scout = custom_agent_developer_instructions_for_role(
        "explorer",
        &json!({
            "summary": "Read-only repo investigation and evidence gathering."
        }),
    );
    assert!(scout.contains("Prefer primary sources"));
    assert!(scout.contains("avoid long excerpts"));

    let companion_reader = custom_agent_developer_instructions_for_role(
        "companion_reader",
        &json!({
            "summary": "Lightweight tool-routed evidence gathering for files, docs, and read-only inspection."
        }),
    );
    assert!(companion_reader.contains("Prefer primary sources"));
    assert!(companion_reader.contains("Fan-in only"));
    assert!(companion_reader.contains("no full-history dumps"));

    let raider = custom_agent_developer_instructions_for_role(
        "code specialist",
        &json!({
            "summary": "Bounded code and config mutation for implementation and repair."
        }),
    );
    assert!(raider.contains("ownership boundaries"));
    assert!(raider.contains("focused tests"));
    assert!(raider.contains("blocked repair handoff"));

    let arbiter = custom_agent_developer_instructions_for_role(
        "verifier",
        &json!({
            "summary": "Review, regression detection, and acceptance judgment when needed."
        }),
    );
    assert!(arbiter.contains("findings first"));
    assert!(arbiter.contains("security"));
    assert!(arbiter.contains("tests"));

    let scribe = custom_agent_developer_instructions_for_role(
        "documenter",
        &json!({
            "summary": "Docs and operator-facing text updates."
        }),
    );
    assert!(scribe.contains("style/translation fidelity"));
    assert!(scribe.contains("docs-only validation"));

    let sentinel = custom_agent_developer_instructions_for_role(
        "sentinel",
        &json!({
            "summary": "Ownership and execution-path classification for bounded routing decisions."
        }),
    );
    assert!(sentinel.contains("classify owner"));
    assert!(sentinel.contains("lane fit"));
    assert!(sentinel.contains("shared-scope conflict"));

    let companion_operator = custom_agent_developer_instructions_for_role(
        "companion_operator",
        &json!({
            "summary": "Lightweight tool-routed mutation and operator-side execution for git-backed actions."
        }),
    );
    assert!(companion_operator.contains("git/gh/release"));
    assert!(companion_operator.contains("destructive commands without approval"));
    assert!(companion_operator.contains("command outcomes"));
    assert!(companion_operator
        .contains("`fix(hub, worker): 비전 기본 가중치를 metric 0.4 text 0.6으로 조정`"));
    assert!(companion_operator.contains(
        "If the operator supplies a commit message/style/language instruction, that instruction wins."
    ));
}

#[test]
fn run_mutation_lock_is_exclusive_per_run_directory() {
    let run_directory = create_temp_path("run-lock");
    create_dir_all(&run_directory).expect("create run directory");

    let first_lock = acquire_run_mutation_lock(&run_directory, "test-first").expect("first lock");
    let second_attempt = acquire_run_mutation_lock(&run_directory, "test-second");
    assert!(second_attempt.is_err());
    assert_eq!(
        second_attempt.expect_err("second lock error").kind(),
        io::ErrorKind::WouldBlock
    );

    drop(first_lock);

    acquire_run_mutation_lock(&run_directory, "test-third").expect("third lock");
    let _ = fs::remove_dir_all(&run_directory);
}

#[test]
fn create_ccc_status_text_truncates_changed_line_and_exposes_signature() {
    let payload = json!({
        "next_step": "advance",
        "run_truth_surface": {
            "fan_in_ready": false
        },
        "worker_visibility": {
            "active_worker_count": 0
        },
        "longway": {
            "completed_phase_count": 1,
            "phase_count": 3,
            "current_item": "item-2"
        },
        "current_task_card": {
            "assigned_agent_id": "raider"
        },
        "latest_delegate_result": {
            "result_summary": "Captain updated the LongWay and selected the next bounded specialist after a very long summary that should be truncated for cleaner CLI following."
        },
        "output": {
            "verbosity": "default",
            "changed_max_chars": 60,
            "include_agent_loop_when_idle": false
        }
    });

    let text = create_ccc_status_text(&payload);
    assert!(text.starts_with("LongWay"));
    assert!(text.contains("Changed: Captain updated the LongWay"));
    assert!(text.contains("..."));

    let signature = create_visibility_signature(&payload);
    assert_eq!(signature.len(), 16);
    assert!(signature.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn create_ccc_status_text_shows_token_unavailable_reason_for_host_subagents() {
    let payload = json!({
        "next_step": "await_fan_in",
        "run_truth_surface": {
            "fan_in_ready": true
        },
        "worker_visibility": {
            "active_worker_count": 0
        },
        "host_subagent_state": {
            "total_subagent_count": 1,
            "active_subagent_count": 0,
            "fan_in_ready": true
        },
        "token_usage": {
            "total_tokens": 0,
            "by_agent": []
        },
        "longway": {
            "completed_phase_count": 1,
            "phase_count": 2,
            "current_item": "item-2"
        },
        "current_task_card": {
            "assigned_agent_id": "ccc_raider"
        },
        "output": {
            "verbosity": "default",
            "changed_max_chars": 120,
            "include_agent_loop_when_idle": false
        }
    });

    let text = create_ccc_status_text(&payload);
    assert!(!text.contains(
        "Token usage: unavailable for host custom subagents; no raw usage events were supplied"
    ));
    assert!(!text.contains("Gauge: [------------------------]"));
    assert!(!text.contains("Tokens: "));
}

#[test]
fn token_usage_visibility_records_structured_unavailable_reason_for_host_subagents() {
    let visibility = create_token_usage_visibility_payload(
        &json!({
            "total_tokens": 0,
            "by_subagent": []
        }),
        &json!({
            "total_subagent_count": 1,
            "active_subagent_count": 0
        }),
    );

    assert_eq!(visibility["status"], "unavailable");
    assert_eq!(visibility["available"], false);
    assert_eq!(
        visibility["unavailable_reason_code"],
        "host_custom_subagents_no_raw_usage_events"
    );
    assert_eq!(
        visibility["unavailable_reason"],
        "host custom subagents did not supply raw usage events"
    );
}

#[test]
fn token_usage_visibility_records_available_raw_usage_source() {
    let visibility = create_token_usage_visibility_payload(
        &json!({
            "source": "delegation_raw_events_best_effort",
            "captain_tokens_available": false,
            "total_tokens": 1200,
            "by_subagent": [
                {
                    "agent_id": "ccc_raider",
                    "total_tokens": 1200
                }
            ]
        }),
        &json!({
            "total_subagent_count": 1,
            "active_subagent_count": 0
        }),
    );

    assert_eq!(visibility["status"], "available");
    assert_eq!(visibility["available"], true);
    assert_eq!(visibility["source"], "delegation_raw_events_best_effort");
    assert!(visibility["unavailable_reason_code"].is_null());
}

#[test]
fn create_ccc_status_text_shows_token_unavailable_reason_without_usage_events() {
    let payload = json!({
        "next_step": "execute_task",
        "run_truth_surface": {
            "fan_in_ready": false
        },
        "token_usage_visibility": {
            "status": "unavailable",
            "available": false,
            "source": "none",
            "captain_tokens_available": false,
            "unavailable_reason": "host custom subagents did not supply raw usage events",
            "unavailable_reason_code": "host_custom_subagents_no_raw_usage_events"
        },
        "worker_visibility": {
            "active_worker_count": 0
        },
        "longway": {
            "completed_phase_count": 0,
            "phase_count": 1,
            "current_item": "item-1"
        },
        "current_task_card": {
            "assigned_agent_id": "scout"
        },
        "output": {
            "verbosity": "default",
            "changed_max_chars": 120,
            "include_agent_loop_when_idle": false
        }
    });

    let text = create_ccc_status_text(&payload);
    assert!(!text
        .contains("Token usage: unavailable; no raw usage events were captured for this run yet"));
    assert!(!text.contains("Gauge: [------------------------]"));
}

#[test]
fn maybe_create_parallel_fanout_payload_defaults_to_two_raider_lanes_for_disjoint_parallel_work() {
    let payload = maybe_create_parallel_fanout_payload(
        "execution",
        "code specialist",
        "Apply a large parallel mutation",
        "Run bounded fan-out work.",
        "raider-a: mutate src/server/auth.rs only\nraider-b: mutate src/server/routes.rs only",
        "Parallelize the bounded mutation work.",
        Some(&json!({
            "workflow_variant": "parallel_fanout"
        })),
        "2026-04-23T10:00:00.000Z",
    )
    .expect("parallel fanout payload");

    assert_eq!(payload["mode"], "parallel");
    assert_eq!(
        payload["required_lane_ids"],
        json!(["raider-a", "raider-b"])
    );
    assert_eq!(payload["aggregate"]["required_lane_count"], 2);
    assert_eq!(payload["aggregate"]["fan_in_ready"], false);
    assert_eq!(payload["lanes"][0]["lane_id"], "raider-a");
    assert_eq!(payload["lanes"][1]["lane_id"], "raider-b");
}

#[test]
fn maybe_create_parallel_fanout_payload_falls_back_to_sequential_for_single_file_scope() {
    let payload = maybe_create_parallel_fanout_payload(
        "execution",
        "code specialist",
        "Apply parallel mutation",
        "Run in parallel when safe.",
        "Single file scope: src/server/auth.rs only",
        "Parallelize the update.",
        Some(&json!({
            "workflow_variant": "parallel"
        })),
        "2026-04-23T10:00:00.000Z",
    )
    .expect("sequential fallback payload");

    assert_eq!(payload["mode"], "sequential");
    assert_eq!(payload["required_lane_ids"], json!(["raider-a"]));
    assert_eq!(payload["aggregate"]["required_lane_count"], 1);
    assert_eq!(payload["disjoint_scope_verified"], false);
}

#[test]
fn maybe_create_parallel_fanout_payload_defaults_to_two_raider_lanes_for_broad_mutation() {
    let payload = maybe_create_parallel_fanout_payload(
        "execution",
        "code specialist",
        "Apply a multi-file update",
        "Run bounded implementation work.",
        "Touch multiple files across the codebase.",
        "Update the runtime and docs together.",
        None,
        "2026-04-23T10:00:00.000Z",
    )
    .expect("broad mutation fanout payload");

    assert_eq!(payload["mode"], "parallel");
    assert_eq!(
        payload["required_lane_ids"],
        json!(["raider-a", "raider-b"])
    );
    assert_eq!(payload["selection_basis"], "broad_mutation_signal");
    assert_eq!(payload["aggregate"]["required_lane_count"], 2);
}

#[test]
fn maybe_create_parallel_fanout_payload_defaults_to_two_scout_lanes_for_broad_explore() {
    let payload = maybe_create_parallel_fanout_payload(
        "explore",
        "explorer",
        "Repo-wide scan",
        "Gather read-only evidence.",
        "Sweep across the repo for active-run handling gaps.",
        "Inspect broadly and return concise evidence.",
        None,
        "2026-04-23T10:00:00.000Z",
    )
    .expect("broad scout fanout payload");

    assert_eq!(payload["mode"], "parallel");
    assert_eq!(payload["required_lane_ids"], json!(["scout-a", "scout-b"]));
    assert_eq!(payload["selection_basis"], "broad_exploration_signal");
    assert_eq!(payload["aggregate"]["required_lane_count"], 2);
    assert_eq!(payload["default_lane_count"], 2);
    assert_eq!(payload["max_lane_count"], 4);
}

#[test]
fn maybe_create_parallel_fanout_payload_caps_scout_lanes_at_four() {
    let payload = maybe_create_parallel_fanout_payload(
            "explore",
            "explorer",
            "Parallel evidence sweep",
            "Gather read-only evidence in parallel.",
            "scout-a: inspect src/main.rs\nscout-b: inspect src/lib.rs\nscout-c: inspect Cargo.toml\nscout-d: inspect README.md",
            "Run parallel scout lanes for broad repo evidence.",
            Some(&json!({
                "workflow_variant": "parallel_fanout"
            })),
            "2026-04-23T10:00:00.000Z",
        )
        .expect("scout max fanout payload");

    assert_eq!(
        payload["required_lane_ids"],
        json!(["scout-a", "scout-b", "scout-c", "scout-d"])
    );
    assert_eq!(payload["aggregate"]["required_lane_count"], 4);
    assert_eq!(payload["max_lane_count"], 4);
}

#[test]
fn scout_parallel_fan_in_waits_for_all_scout_lanes() {
    let initial = json!({
        "mode": "parallel",
        "requested_parallel": true,
        "selection_basis": "broad_exploration_signal",
        "default_lane_count": 2,
        "max_lane_count": 4,
        "required_lane_ids": ["scout-a", "scout-b"],
        "all_lane_ids": ["scout-a", "scout-b", "scout-c", "scout-d"],
        "disjoint_scope_required": false,
        "disjoint_scope_verified": false,
        "summary": "Parallel scout fan-out enabled.",
        "lanes": [
            { "lane_id": "scout-a", "required": true, "scope": null, "lifecycle": null, "fan_in": null },
            { "lane_id": "scout-b", "required": true, "scope": null, "lifecycle": null, "fan_in": null }
        ],
        "aggregate": {
            "required_lane_count": 2,
            "active_lane_count": 0,
            "terminal_lane_count": 0,
            "fan_in_ready": false,
            "status": "awaiting_lane_updates"
        }
    });
    let after_a = update_parallel_fanout_for_lane(
        Some(&initial),
        "scout-a",
        "completed",
        "ccc_scout",
        Some("thread-a"),
        Some("Scout A complete."),
        &json!({"summary":"Scout A complete.","status":"completed"}),
        "2026-04-23T10:01:00.000Z",
    );
    assert_eq!(after_a["aggregate"]["fan_in_ready"], false);
    assert_eq!(after_a["aggregate"]["missing_lane_ids"], json!(["scout-b"]));

    let after_b = update_parallel_fanout_for_lane(
        Some(&after_a),
        "scout-b",
        "completed",
        "ccc_scout",
        Some("thread-b"),
        Some("Scout B complete."),
        &json!({"summary":"Scout B complete.","status":"completed"}),
        "2026-04-23T10:02:00.000Z",
    );
    assert_eq!(after_b["aggregate"]["fan_in_ready"], true);
    assert_eq!(
        after_b["aggregate"]["terminal_lane_ids"],
        json!(["scout-a", "scout-b"])
    );
}

#[test]
fn parse_cli_command_input_supports_text_and_json_file_modes() {
    let inline = parse_cli_command_input(
        "status",
        &[
            "--text".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-1\"}".to_string(),
        ],
        false,
    )
    .expect("parse inline");
    assert_eq!(inline.output_mode, CliOutputMode::Text);
    assert_eq!(inline.app_panel, false);
    assert_eq!(inline.artifact, false);
    assert_eq!(inline.subagents, false);
    assert_eq!(inline.projection, false);
    assert_eq!(inline.payload["run_id"], "run-1");

    let app_panel = parse_cli_command_input(
        "status",
        &[
            "--app-panel".to_string(),
            "--text".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-panel\"}".to_string(),
        ],
        false,
    )
    .expect("parse app panel");
    assert_eq!(app_panel.output_mode, CliOutputMode::Text);
    assert_eq!(app_panel.app_panel, true);
    assert_eq!(app_panel.artifact, false);
    assert_eq!(app_panel.subagents, false);
    assert_eq!(app_panel.projection, false);
    assert_eq!(app_panel.payload["run_id"], "run-panel");

    let app_panel_artifact = parse_cli_command_input(
        "status",
        &[
            "--app-panel".to_string(),
            "--artifact".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-panel\"}".to_string(),
        ],
        false,
    )
    .expect("parse app panel artifact");
    assert_eq!(app_panel_artifact.output_mode, CliOutputMode::Json);
    assert_eq!(app_panel_artifact.app_panel, true);
    assert_eq!(app_panel_artifact.artifact, true);
    assert_eq!(app_panel_artifact.subagents, false);
    assert_eq!(app_panel_artifact.projection, false);

    let subagents = parse_cli_command_input(
        "status",
        &[
            "--subagents".to_string(),
            "--quiet".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-subagents\"}".to_string(),
        ],
        false,
    )
    .expect("parse subagents");
    assert_eq!(subagents.output_mode, CliOutputMode::Quiet);
    assert_eq!(subagents.subagents, true);
    assert_eq!(subagents.projection, false);
    assert_eq!(subagents.payload["run_id"], "run-subagents");

    let checklist_subagents = parse_cli_command_input(
        "checklist",
        &[
            "--subagents".to_string(),
            "--text".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-subagents\"}".to_string(),
        ],
        false,
    )
    .expect("parse checklist subagents");
    assert_eq!(checklist_subagents.output_mode, CliOutputMode::Text);
    assert_eq!(checklist_subagents.subagents, true);
    assert_eq!(checklist_subagents.projection, false);

    let projection = parse_cli_command_input(
        "status",
        &[
            "--projection".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-projection\"}".to_string(),
        ],
        false,
    )
    .expect("parse projection");
    assert_eq!(projection.output_mode, CliOutputMode::Json);
    assert_eq!(projection.subagents, false);
    assert_eq!(projection.projection, true);
    assert_eq!(projection.payload["run_id"], "run-projection");

    let checklist_projection = parse_cli_command_input(
        "checklist",
        &[
            "--projection".to_string(),
            "--quiet".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-projection\"}".to_string(),
        ],
        false,
    )
    .expect("parse checklist projection");
    assert_eq!(checklist_projection.output_mode, CliOutputMode::Quiet);
    assert_eq!(checklist_projection.projection, true);

    let temp_dir = create_temp_path("cli-command-input");
    create_dir_all(&temp_dir).expect("create temp dir");
    let json_file = temp_dir.join("input.json");
    write(&json_file, "{\"run_id\":\"run-2\"}").expect("write input");

    let from_file = parse_cli_command_input(
        "status",
        &[
            "--quiet".to_string(),
            "--json-file".to_string(),
            json_file.to_string_lossy().to_string(),
        ],
        false,
    )
    .expect("parse json file");
    assert_eq!(from_file.output_mode, CliOutputMode::Quiet);
    assert_eq!(from_file.app_panel, false);
    assert_eq!(from_file.artifact, false);
    assert_eq!(from_file.subagents, false);
    assert_eq!(from_file.projection, false);
    assert_eq!(from_file.payload["run_id"], "run-2");
    assert_eq!(from_file.transient_json_file_path(), None);

    let recommend_entry = parse_cli_command_input(
        "recommend-entry",
        &[
            "--quiet".to_string(),
            "--json".to_string(),
            "{\"request\":\"List files\",\"cwd\":\"/tmp\"}".to_string(),
        ],
        false,
    )
    .expect("parse recommend-entry");
    assert_eq!(recommend_entry.output_mode, CliOutputMode::Quiet);
    let recommend_arguments =
        parse_ccc_recommend_entry_arguments(&recommend_entry.payload).expect("recommend args");
    assert_eq!(recommend_arguments["request"], "List files");
    assert_eq!(recommend_arguments["cwd"], "/tmp");

    let auto_entry = parse_cli_command_input(
        "auto-entry",
        &[
            "--text".to_string(),
            "--json".to_string(),
            "{\"request\":\"List files\",\"cwd\":\"/tmp\",\"codex_bin\":\"codex\"}".to_string(),
        ],
        false,
    )
    .expect("parse auto-entry");
    assert_eq!(auto_entry.output_mode, CliOutputMode::Text);
    let auto_arguments = parse_ccc_auto_entry_arguments(&auto_entry.payload).expect("auto args");
    assert_eq!(auto_arguments["request"], "List files");
    assert_eq!(auto_arguments["codex_bin"], "codex");

    let missing_app_panel = parse_cli_command_input("status", &["--artifact".to_string()], true)
        .expect_err("artifact requires app panel");
    assert!(missing_app_panel
        .to_string()
        .contains("requires `--app-panel`"));

    let subagents_json = parse_cli_command_input(
        "status",
        &[
            "--subagents".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-subagents\"}".to_string(),
        ],
        false,
    )
    .expect_err("subagents rejects json output");
    assert!(subagents_json
        .to_string()
        .contains("requires `--text` or `--quiet`"));

    let checklist_subagents_json = parse_cli_command_input(
        "checklist",
        &[
            "--subagents".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-subagents\"}".to_string(),
        ],
        false,
    )
    .expect_err("checklist subagents rejects json output");
    assert!(checklist_subagents_json
        .to_string()
        .contains("requires `--text` or `--quiet`"));

    let subagents_app_panel = parse_cli_command_input(
        "status",
        &[
            "--subagents".to_string(),
            "--app-panel".to_string(),
            "--text".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-subagents\"}".to_string(),
        ],
        false,
    )
    .expect_err("subagents rejects app panel");
    assert!(subagents_app_panel
        .to_string()
        .contains("cannot be combined"));

    let subagents_artifact = parse_cli_command_input(
        "status",
        &[
            "--subagents".to_string(),
            "--artifact".to_string(),
            "--quiet".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-subagents\"}".to_string(),
        ],
        false,
    )
    .expect_err("subagents rejects artifact");
    assert!(subagents_artifact
        .to_string()
        .contains("cannot be combined"));

    let projection_subagents = parse_cli_command_input(
        "status",
        &[
            "--projection".to_string(),
            "--subagents".to_string(),
            "--text".to_string(),
            "--json".to_string(),
            "{\"run_id\":\"run-projection\"}".to_string(),
        ],
        false,
    )
    .expect_err("projection rejects subagents");
    assert!(projection_subagents
        .to_string()
        .contains("cannot be combined"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn ccc_status_app_panel_artifact_cli_path_is_internal_only() {
    let workspace_dir = create_temp_path("app-panel-artifact-cli-hidden");
    create_dir_all(&workspace_dir).expect("create workspace");
    write_test_run_fixture(&workspace_dir, "run-app-panel-artifact-cli-hidden");

    let input = json!({
        "cwd": workspace_dir.to_string_lossy(),
        "run_id": "run-app-panel-artifact-cli-hidden"
    });
    let error = run_status_command(&[
        "--app-panel".to_string(),
        "--artifact".to_string(),
        "--json".to_string(),
        input.to_string(),
    ])
    .expect_err("app-panel artifact CLI path should be internal-only");
    assert!(error.to_string().contains("internal-only"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn cli_json_file_cleanup_removes_only_ccc_tmp_payload_after_success() {
    let workspace_dir = create_temp_path("cli-json-file-cleanup");
    create_dir_all(workspace_dir.join(".ccc").join("tmp")).expect("create ccc tmp");

    let transient_payload = workspace_dir.join(".ccc").join("tmp").join("memory.json");
    write(
        &transient_payload,
        json!({
            "cwd": workspace_dir.to_string_lossy(),
            "action": "status"
        })
        .to_string(),
    )
    .expect("write transient payload");
    let parsed_transient = parse_cli_command_input(
        "memory",
        &[
            "--json-file".to_string(),
            transient_payload.to_string_lossy().to_string(),
        ],
        false,
    )
    .expect("parse transient payload");
    assert_eq!(
        parsed_transient.transient_json_file_path(),
        Some(transient_payload.as_path())
    );
    run_memory_command(&[
        "--quiet".to_string(),
        "--json-file".to_string(),
        transient_payload.to_string_lossy().to_string(),
    ])
    .expect("memory command succeeds");
    assert!(
        !transient_payload.exists(),
        "successful command removes .ccc/tmp json-file payload"
    );

    let retained_payload = workspace_dir.join("memory.json");
    write(
        &retained_payload,
        json!({
            "cwd": workspace_dir.to_string_lossy(),
            "action": "status"
        })
        .to_string(),
    )
    .expect("write retained payload");
    run_memory_command(&[
        "--quiet".to_string(),
        "--json-file".to_string(),
        retained_payload.to_string_lossy().to_string(),
    ])
    .expect("memory command succeeds");
    assert!(
        retained_payload.exists(),
        "successful command keeps non-.ccc/tmp json-file payload"
    );

    let failing_payload = workspace_dir.join(".ccc").join("tmp").join("failing.json");
    write(
        &failing_payload,
        json!({
            "cwd": workspace_dir.to_string_lossy(),
            "action": "unknown"
        })
        .to_string(),
    )
    .expect("write failing payload");
    let error = run_memory_command(&[
        "--quiet".to_string(),
        "--json-file".to_string(),
        failing_payload.to_string_lossy().to_string(),
    ])
    .expect_err("memory command fails");
    assert!(error.to_string().contains("Unknown memory action"));
    assert!(
        failing_payload.exists(),
        "failed command keeps .ccc/tmp json-file payload"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_compact_payload_keeps_subagent_contract_and_command_templates() {
    let payload = json!({
        "run_id": "run-compact",
        "run_ref": "ccc-run:/tmp/run-compact",
        "status": "active",
        "stage": "execution",
        "next_step": "execute_task",
        "can_advance": true,
        "run_truth_surface": {
            "resume_action": "execute_task"
        },
        "state_contract": {
            "schema": "ccc.state_contract.v1",
            "state": "recovery_pending",
            "active_gate": "recovery",
            "required_artifact": "retry_or_reassign_decision",
            "next_step": "await_fan_in",
            "allowed_next_transitions": ["retry", "reassign", "record_fallback", "close_host_thread"],
            "captain_allowed_action": "recover_subagent",
            "captain_required_action": "ccc_orchestrate",
            "task_card_id": "task-compact",
            "assigned_role": "explorer",
            "assigned_agent_id": "scout"
        },
        "post_fan_in_captain_decision": {
            "schema": "ccc.post_fan_in_captain_decision.v1",
            "precedence": "recovery",
            "allowed_action": "recover_subagent",
            "required_action": "ccc_orchestrate",
            "state": "recovery_pending",
            "active_gate": "recovery",
            "required_artifact": "retry_or_reassign_decision",
            "scheduler_action_kind": "recover_subagent"
        },
        "recovery_lane": {
            "source": "host_subagent_state",
            "status": "recovery_pending",
            "recommended_action": "retry",
            "needs_operator_attention": true,
            "summary": "Terminal ccc_scout state requires retry or reassign before degraded fallback."
        },
        "token_usage_visibility": {
            "status": "unavailable",
            "available": false,
            "source": "none",
            "captain_tokens_available": false,
            "unavailable_reason": "host custom subagents did not supply raw usage events",
            "unavailable_reason_code": "host_custom_subagents_no_raw_usage_events"
        },
        "longway": {
            "completed_phase_count": 0,
            "phase_count": 1,
            "current_item": "item-1",
            "lifecycle_state": "active",
            "planning_context": {
                "workspace_root": {
                    "root": "/tmp/repo",
                    "root_kind": "git_repo",
                    "confidence": "high",
                    "confirmation_required": false,
                    "reason": "Resolved from current workspace git root.",
                    "candidates": []
                }
            },
            "planned_row_count": 2,
            "planned_rows": [
                {
                    "title": "Inspect current directory",
                    "status": "materialized",
                    "planned_role": "explorer",
                    "planned_agent_id": "scout"
                },
                {
                    "title": "Summarize app-panel fallback",
                    "status": "planned",
                    "planned_role": "documenter",
                    "planned_agent_id": "scribe"
                },
                {
                    "title": "Unassigned follow-up work",
                    "status": "planned"
                }
            ],
            "phase_rows": [{
                "id": "item-1",
                "title": "Inspect current directory",
                "status": "pending",
                "owner_agent": "scout"
            }]
        },
        "current_task_card": {
            "task_card_id": "task-compact",
            "title": "Inspect current directory",
            "task_kind": "explore",
            "scope": "Inspect the current directory.",
            "review_of_task_card_ids": ["task-reviewed"],
            "orchestrator_review_gate": "after_child_completion",
            "verification_state": "pending",
            "review_pass_count": 0,
            "assigned_role": "explorer",
            "assigned_agent_id": "scout",
            "execution_prompt": "Inspect the current directory.",
            "subagent_fan_in": {
                "summary": "Bounded fan-in summary",
                "status": "completed",
                "evidence_paths": ["src/main.rs:10"],
                "next_action": "captain_merge",
                "open_questions": [],
                "confidence": "high",
                "recorded_at": "2026-04-22T08:00:00.000Z"
            },
            "parallel_fanout": {
                "mode": "parallel",
                "summary": "Parallel fan-out across two disjoint lanes.",
                "required_lane_ids": ["raider-a", "raider-b"],
                "aggregate": {
                    "required_lane_count": 2,
                    "terminal_lane_count": 1,
                    "fan_in_ready": false
                },
                "lanes": [
                    {
                        "lane_id": "raider-a",
                        "required": true,
                        "scope": "src/main.rs",
                        "lifecycle": {
                            "status": "completed",
                            "child_agent_id": "ccc_raider",
                            "thread_id": "thread-a",
                            "summary": "Lane A complete",
                            "updated_at": "2026-04-22T08:00:00.000Z"
                        },
                        "fan_in": {
                            "summary": "Lane A summary",
                            "status": "completed",
                            "evidence_paths": ["src/main.rs:10"],
                            "next_action": "captain_merge",
                            "open_questions": [],
                            "confidence": "high",
                            "recorded_at": "2026-04-22T08:00:00.000Z"
                        }
                    },
                    {
                        "lane_id": "raider-b",
                        "required": true,
                        "scope": "README.md",
                        "lifecycle": {
                            "status": "running",
                            "child_agent_id": "ccc_raider",
                            "thread_id": "thread-b",
                            "summary": "Lane B running",
                            "updated_at": "2026-04-22T08:05:00.000Z"
                        },
                        "fan_in": null
                    }
                ]
            },
            "subagent_policy_drift": {
                "ok": false,
                "mismatches": [
                    {
                        "field": "sandbox_mode",
                        "expected": "read-only",
                        "observed": "workspace-write"
                    }
                ]
            },
            "delegation_plan": {
                "preferred_execution_mode": "codex_subagent",
                "fallback_execution_mode": "codex_exec",
                "preferred_custom_agent_name": "ccc_scout",
                "preferred_custom_agent_file": "ccc-scout.toml",
                "model": "gpt-5.4-mini",
                "variant": "medium",
                "expertise_framing": {
                    "expertise_phrase": "You are an expert in repository investigation, evidence gathering, and concise source-backed reporting.",
                    "task_stance": "read_only_investigation",
                    "expected_thinking_mode": "evidence-first-read-only",
                    "task_shape": "single_scoped_task"
                },
                "runtime_dispatch": {
                    "source": "config_backed",
                    "execution_mode_source": "runtime_config",
                    "role_profile_source": "role_config_snapshot",
                    "custom_agent_source": "role_mapping",
                    "plan_invariants_source": "delegation_plan_invariants",
                    "preferred_execution_mode": "codex_subagent",
                    "fallback_execution_mode": "codex_exec",
                    "supported_execution_modes": [
                        "codex_subagent",
                        "codex_exec"
                    ],
                    "preferred_custom_agent_name": "ccc_scout",
                    "preferred_custom_agent_file": "ccc-scout.toml",
                    "assigned_role": "explorer",
                    "assigned_agent_id": "scout",
                    "summary": "Read-only repo investigation and evidence gathering.",
                    "expertise_framing": {
                        "expertise_phrase": "You are an expert in repository investigation, evidence gathering, and concise source-backed reporting.",
                        "task_stance": "read_only_investigation",
                        "expected_thinking_mode": "evidence-first-read-only",
                        "task_shape": "single_scoped_task"
                    },
                    "model": "gpt-5.4-mini",
                    "variant": "medium",
                    "fast_mode": false
                },
                "lane_artifact_contract": {
                    "result": {
                        "field": "fan_in",
                        "source": "parallel_fanout.lanes[].fan_in"
                    },
                    "log": {
                        "field": "lifecycle",
                        "source": "parallel_fanout.lanes[].lifecycle"
                    },
                    "recap": {
                        "field": "fan_in.summary",
                        "source": "parallel_fanout.lanes[].fan_in.summary"
                    }
                },
                "verify_retry_recap_report_contract": {
                    "verify": {
                        "field": "verification_state",
                        "states": [
                            "pending",
                            "passed",
                            "needs_work",
                            "blocked"
                        ]
                    },
                    "retry": {
                        "field": "captain_follow_up",
                        "budget_key": "retry",
                        "states": [
                            "queued",
                            "consumed"
                        ]
                    },
                    "recap": {
                        "field": "lane_artifact_contract.recap",
                        "source": "parallel_fanout.lanes[].fan_in.summary"
                    },
                    "report": {
                        "field": "latest_delegate_result.result_summary",
                        "fallback_field": "latest_delegate_result.assistant_message_preview"
                    }
                },
                "sandbox_mode": "read-only",
                "spec_surfaces": {
                    "role_owned": {
                        "owned_by": "role_config_snapshot",
                        "fields": [
                            "summary",
                            "model",
                            "variant",
                            "fast_mode"
                        ]
                    },
                    "sandbox_owned": {
                        "owned_by": "sandbox_policy_helpers",
                        "fields": [
                            "sandbox_mode",
                            "sandbox_rationale"
                        ]
                    },
                    "workflow_owned": {
                        "owned_by": "delegation_plan",
                        "fields": [
                            "routing_source",
                            "preferred_execution_mode",
                            "fallback_execution_mode",
                            "supported_execution_modes",
                            "execution_contract",
                            "runtime_dispatch",
                            "expertise_framing",
                            "fan_in_contract",
                            "lane_artifact_contract",
                            "verify_retry_recap_report_contract",
                            "subagent_spawn_contract",
                            "subagent_update_contract",
                            "fallback_gate"
                        ]
                    },
                    "plan_invariants": {
                        "owned_by": "delegation_plan_invariants",
                        "fields": [
                            "policy_drift_check_required",
                            "captain_checkpoint_required",
                            "fallback_reason_codes"
                        ]
                    }
                },
                "fan_in_contract": {
                    "mode": "structured_summary",
                    "required_fields": [
                        "summary",
                        "status",
                        "evidence_paths",
                        "next_action",
                        "open_questions",
                        "confidence"
                    ]
                },
                "lane_artifact_contract": {
                    "result": {
                        "field": "fan_in",
                        "source": "parallel_fanout.lanes[].fan_in"
                    },
                    "log": {
                        "field": "lifecycle",
                        "source": "parallel_fanout.lanes[].lifecycle"
                    },
                    "recap": {
                        "field": "fan_in.summary",
                        "source": "parallel_fanout.lanes[].fan_in.summary"
                    }
                },
                "subagent_spawn_contract": {
                    "forbid_full_history_fork": true,
                    "expertise_phrase": "You are an expert in repository investigation, evidence gathering, and concise source-backed reporting.",
                    "task_stance": "read_only_investigation",
                    "expected_thinking_mode": "evidence-first-read-only"
                },
                "subagent_update_contract": {
                    "transport": "ccc_cli_subcommand"
                },
                "fallback_gate": {
                    "require_explicit_subagent_fallback_reason": true
                }
            }
        },
        "execution_strategy": {
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec",
            "host_subagent_update_mode": "ccc_cli_subcommand",
            "codex_exec_fallback_allowed": false
        },
        "cost_routing": {
            "status": "configured",
            "subagents": {
                "enabled": true,
                "current_task_role": "explorer",
                "current_task_model": {
                    "model": "gpt-5.4-mini",
                    "variant": "medium",
                    "uses_low_cost_model": true
                }
            },
            "simple_routes_use_low_cost_models": true,
            "token_usage_observation": {
                "status": "unavailable"
            }
        },
        "captain_action_contract": {
            "source": "ccc_status",
            "preflight_guard": "ccc_recommend_entry",
            "preferred_operator_transport": "ccc_cli_quiet_subcommand",
            "preferred_operator_transport_reason": "Keeps repeated CCC lifecycle mutations visible as compact command runs instead of verbose MCP tool-call payloads.",
            "mcp_tool_call_policy": "reserve_for_app_or_structured_inspection_or_cli_unavailable",
            "allowed_action": "spawn_subagent",
            "required_action": "spawn_or_record_specialist",
            "direct_finish_allowed": false,
            "direct_mutation_allowed": false,
            "denied_action_reason": "Current task requires specialist execution before direct captain finish or mutation.",
            "completion_required": false
        },
        "host_subagent_state": {
            "active_subagent_count": 1,
            "subagent_activity": [
                {
                    "child_agent_id": "ccc_scout",
                    "assigned_role": "explorer",
                    "task_card_id": "task-compact",
                    "task_title": "Inspect current directory",
                    "lane_id": "raider-b",
                    "status": "running",
                    "next_action": "await_fan_in",
                    "summary": "Reading files and collecting evidence.",
                    "updated_at": "2026-05-04T10:00:00Z"
                }
            ],
            "active_subagents": [
                {
                    "child_agent_id": "ccc_scout",
                    "assigned_role": "explorer",
                    "task_card_id": "task-compact",
                    "task_title": "Inspect current directory",
                    "lane_id": "raider-b",
                    "status": "running",
                    "next_action": "await_fan_in",
                    "summary": "Reading files and collecting evidence.",
                    "updated_at": "2026-05-04T10:00:00Z"
                }
            ]
        },
        "visibility_signature": "1234567890abcdef"
    });

    let compact = create_ccc_status_compact_payload(&payload);

    assert_eq!(compact["compact"], true);
    assert_eq!(compact["app_panel"]["schema"], "ccc.codex_app_panel.v1");
    assert_eq!(
        compact["app_panel"]["state_contract"],
        payload["state_contract"]
    );
    assert_eq!(
        compact["app_panel"]["state_contract"]["active_gate"],
        "recovery"
    );
    assert_eq!(
        compact["app_panel"]["recovery_lane"],
        payload["recovery_lane"]
    );
    assert_eq!(
        compact["app_panel"]["recovery_lane"]["status"],
        "recovery_pending"
    );
    assert_eq!(
        compact["app_panel"]["post_fan_in_captain_decision"],
        payload["post_fan_in_captain_decision"]
    );
    assert_eq!(
        compact["app_panel"]["render_strategy"]["data_source"],
        "ccc_status_or_ccc_activity"
    );
    assert_eq!(
        compact["app_panel"]["render_strategy"]["fallback"],
        "transcript_status_text"
    );
    assert!(compact["app_panel"]["render_strategy"]
        .get("artifact_fallback")
        .is_none());
    assert_eq!(
        compact["app_panel"]["longway_progress"]["completed_phase_count"],
        0
    );
    assert_eq!(
        compact["app_panel"]["current_task"]["task_card_id"],
        "task-compact"
    );
    assert_eq!(
        compact["app_panel"]["current_task"]["model"],
        "gpt-5.4-mini"
    );
    assert_eq!(compact["app_panel"]["current_task"]["variant"], "medium");
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][1]["display_agent_id"],
        "ccc_scribe"
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][1]["model"],
        expected_role_config_field("documenter", "model")
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][1]["reasoning"],
        expected_role_config_field("documenter", "variant")
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][1]["agent_source"],
        "planned_row_input"
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][1]["model_source"],
        "role_config"
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][1]["reasoning_source"],
        "role_config"
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][2]["display_agent_id"],
        Value::Null
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][2]["display_role"],
        Value::Null
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][2]["agent_source"],
        "unassigned"
    );
    assert_eq!(
        compact["app_panel"]["longway_progress"]["planned_rows"][2]["model_source"],
        "unassigned"
    );
    assert_eq!(
        compact["app_panel"]["specialist_lanes"]["parallel_lanes"][1]["status"],
        "running"
    );
    assert_eq!(
        compact["app_panel"]["specialist_lanes"]["subagent_activity"][0]["child_agent_id"],
        "ccc_scout"
    );
    assert_eq!(
        compact["app_panel"]["specialist_lanes"]["subagent_activity"][0]["task_title"],
        "Inspect current directory"
    );
    assert_eq!(
        compact["app_panel"]["specialist_lanes"]["subagent_activity"][0]["model"],
        "gpt-5.4-mini"
    );
    assert_eq!(
        compact["app_panel"]["specialist_lanes"]["subagent_activity"][0]["variant"],
        "medium"
    );
    assert_eq!(compact["app_panel"]["fan_in"]["ready"], false);
    assert_eq!(
        compact["app_panel"]["next_captain_action"]["allowed_action"],
        "spawn_subagent"
    );
    assert_eq!(
        compact["app_panel"]["next_captain_action"]["direct_file_mutation_policy"]["allowed"],
        false
    );
    assert_eq!(
        compact["current_task_card"]["subagent_contract"]["agent"],
        "ccc_scout"
    );
    assert_eq!(
        compact["current_task_card"]["subagent_contract"]["omit_overrides"],
        true
    );
    assert_eq!(
        compact["current_task_card"]["subagent_policy_drift"]["ok"],
        false
    );
    assert_eq!(
        compact["current_task_card"]["review_of_task_card_ids"],
        json!(["task-reviewed"])
    );
    assert_eq!(
        compact["current_task_card"]["scope"],
        "Inspect the current directory."
    );
    assert_eq!(
        compact["current_task_card"]["launch_visibility"]["lane_ids"],
        json!(["raider-a", "raider-b"])
    );
    assert_eq!(
        compact["current_task_card"]["launch_visibility"]["lane"],
        "raider-a"
    );
    assert_eq!(
        compact["current_task_card"]["launch_visibility"]["expected_fan_in"],
        json!([
            "summary",
            "status",
            "evidence_paths",
            "next_action",
            "open_questions",
            "confidence"
        ])
    );
    assert_eq!(
        compact["current_task_card"]["spec_surfaces"]["role_owned"]["owned_by"],
        "role_config_snapshot"
    );
    assert_eq!(
        compact["current_task_card"]["spec_surfaces"]["sandbox_owned"]["owned_by"],
        "sandbox_policy_helpers"
    );
    assert_eq!(
        compact["current_task_card"]["spec_surfaces"]["workflow_owned"]["fields"],
        json!([
            "routing_source",
            "preferred_execution_mode",
            "fallback_execution_mode",
            "supported_execution_modes",
            "execution_contract",
            "runtime_dispatch",
            "expertise_framing",
            "fan_in_contract",
            "lane_artifact_contract",
            "verify_retry_recap_report_contract",
            "subagent_spawn_contract",
            "subagent_update_contract",
            "fallback_gate"
        ])
    );
    assert_eq!(
        compact["current_task_card"]["spec_surfaces"]["plan_invariants"]["owned_by"],
        "delegation_plan_invariants"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["source"],
        "config_backed"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["execution_mode_source"],
        "runtime_config"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["role_profile_source"],
        "role_config_snapshot"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["custom_agent_source"],
        "role_mapping"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["preferred_execution_mode"],
        "codex_subagent"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["preferred_custom_agent_name"],
        "ccc_scout"
    );
    assert_eq!(
        compact["current_task_card"]["expertise_framing"]["task_stance"],
        "read_only_investigation"
    );
    assert_eq!(
        compact["current_task_card"]["subagent_contract"]["expertise_phrase"],
        "You are an expert in repository investigation, evidence gathering, and concise source-backed reporting."
    );
    assert_eq!(
        compact["current_task_card"]["subagent_contract"]["expected_thinking_mode"],
        "evidence-first-read-only"
    );
    assert_eq!(
        compact["current_task_card"]["runtime_dispatch"]["model"],
        "gpt-5.4-mini"
    );
    assert_eq!(
        compact["current_task_card"]["lane_artifact_contract"]["result"]["field"],
        "fan_in"
    );
    assert_eq!(
        compact["current_task_card"]["lane_artifact_contract"]["log"]["source"],
        "parallel_fanout.lanes[].lifecycle"
    );
    assert_eq!(
        compact["current_task_card"]["lane_artifact_contract"]["recap"]["field"],
        "fan_in.summary"
    );
    assert_eq!(
        compact["current_task_card"]["verify_retry_recap_report_contract"]["verify"]["field"],
        "verification_state"
    );
    assert_eq!(
        compact["current_task_card"]["verify_retry_recap_report_contract"]["retry"]["budget_key"],
        "retry"
    );
    assert_eq!(
        compact["current_task_card"]["verify_retry_recap_report_contract"]["recap"]["field"],
        "lane_artifact_contract.recap"
    );
    assert_eq!(
        compact["current_task_card"]["verify_retry_recap_report_contract"]["report"]["field"],
        "latest_delegate_result.result_summary"
    );
    assert_eq!(
        compact["token_usage_unavailable_reason"],
        "host custom subagents did not supply raw usage events"
    );
    assert_eq!(
        compact["token_usage_unavailable_reason_code"],
        "host_custom_subagents_no_raw_usage_events"
    );
    assert_eq!(
        compact["current_task_card"]["orchestrator_review_gate"],
        "after_child_completion"
    );
    assert_eq!(
        compact["current_task_card"]["verification_state"],
        "pending"
    );
    assert_eq!(compact["current_task_card"]["review_pass_count"], 0);
    assert_eq!(
        compact["execution_strategy"]["codex_exec_fallback_allowed"],
        false
    );
    assert_eq!(
        compact["execution_strategy"]["operator_visible_transport"]["preferred_transport"],
        "ccc_cli_quiet_subcommand"
    );
    assert_eq!(
        compact["execution_strategy"]["operator_visible_transport"]["transcript_signal"],
        "ran"
    );
    assert_eq!(
        compact["execution_strategy"]["operator_visible_transport"]["preferred_command_shapes"]
            ["start"][0],
        "ccc start --quiet --json '{...}'"
    );
    assert_eq!(
        compact["execution_strategy"]["operator_visible_transport"]["preferred_command_shapes"]
            ["memory"][0],
        "ccc memory --quiet --json '{...}'"
    );
    assert_eq!(
        compact["execution_strategy"]["operator_visible_transport"]["preferred_command_shapes"]
            ["projection"][0],
        "ccc status --projection --json '{...}'"
    );
    assert_eq!(
        compact["execution_strategy"]["operator_visible_transport"]["mcp_reserved_for"],
        json!(["app surfaces", "structured inspection", "CLI unavailable"])
    );
    assert_eq!(compact["cost_routing"]["status"], "configured");
    assert_eq!(compact["cost_routing"]["subagents"]["enabled"], true);
    assert_eq!(
        compact["cost_routing"]["simple_routes_use_low_cost_models"],
        true
    );
    assert_eq!(
        compact["captain_action_contract"]["allowed_action"],
        "spawn_subagent"
    );
    assert_eq!(
        compact["captain_action_contract"]["preflight_guard"],
        "ccc_recommend_entry"
    );
    assert_eq!(
        compact["operator_transport_contract"]["preferred"],
        "ccc_cli_quiet_subcommand"
    );
    assert_eq!(
        compact["operator_transport_contract"]["display_expectation"],
        "ran"
    );
    assert_eq!(
        compact["operator_transport_contract"]["avoid_for_lifecycle_mutations"],
        "mcp_tool_call"
    );
    assert_eq!(
        compact["operator_transport_contract"]["default_payload_transport"],
        "inline_json"
    );
    let longway_visibility = compact["operator_transport_contract"]["longway_visibility"]
        .as_str()
        .expect("LongWay visibility guidance");
    assert!(longway_visibility.contains("CCC_LONGWAY_PROJECTION.md"));
    assert!(longway_visibility.contains("ccc status --projection --json '{...}'"));
    assert!(!longway_visibility.contains("ccc checklist --text"));
    assert_eq!(
        compact["command_templates"]["operator_transport"]["preferred"],
        "ccc_cli_quiet_subcommand"
    );
    assert!(
        compact["command_templates"]["operator_transport"]["commands"][2]
            .as_str()
            .expect("operator transport command")
            .contains("ccc subagent-update --quiet --json '{...}'")
    );
    assert_eq!(
        compact["captain_action_contract"]["preferred_operator_transport"],
        "ccc_cli_quiet_subcommand"
    );
    assert_eq!(
        compact["captain_action_contract"]["mcp_tool_call_policy"],
        "reserve_for_app_or_structured_inspection_or_cli_unavailable"
    );
    assert_eq!(
        compact["captain_action_contract"]["direct_mutation_allowed"],
        false
    );
    assert_eq!(
        compact["captain_action_contract"]["direct_file_mutation_policy"]["allowed"],
        false
    );
    assert_eq!(
        compact["captain_action_contract"]["direct_file_mutation_policy"]["applies_to"],
        json!([
            "apply_patch",
            "direct_shell_file_mutation",
            "file_edits",
            "mutation_commands"
        ])
    );
    assert_eq!(
        compact["captain_action_contract"]["direct_file_mutation_policy"]
            ["requires_recorded_exception"],
        "explicit_terminal_fallback_or_operator_override"
    );
    assert_eq!(
        compact["captain_action_contract"]["direct_file_mutation_policy"]["required_route"],
        "specialist_fan_in_then_captain_review_merge"
    );
    assert_eq!(
        compact["captain_action_contract"]["direct_file_mutation_policy"]["merge_gate"],
        "specialist_fan_in_or_explicit_operator_override"
    );
    let text = create_ccc_status_text(&payload);
    assert!(text.contains("Captain Guard: allowed=spawn-subagent"));
    assert!(text.contains("direct_mutation=false"));
    assert!(text.contains(
        "direct_file_mutation_allowed=false route=specialist-fan-in-then-captain-review-merge"
    ));
    assert!(text.contains(
        "Launch: role=Observer(ccc_scout)/explorer lane=raider-a,raider-b scope=\"Inspect the current directory.\" expected_fan_in=summary,status,evidence_paths,next_action,open_questions,confidence"
    ));
    assert!(text.contains(
        "Dispatch: source=config_backed preferred=codex_subagent(runtime_config) fallback=codex_exec(runtime_config) agent=Observer(ccc_scout)(role_mapping) model=gpt-5.4-mini(role_config_snapshot) variant=medium(role_config_snapshot)"
    ));
    assert!(text.contains(
        "Cost Routing: status=configured subagents_enabled=true simple_low_cost_models=true current_role=Observer(ccc_scout)/explorer current_model=gpt-5.4-mini token_usage=unavailable"
    ));
    let app_panel_text = create_codex_app_panel_text(&compact["app_panel"]);
    assert!(app_panel_text.contains("CCC LongWay"));
    assert!(app_panel_text.starts_with("+"));
    assert!(app_panel_text.contains("Progress: 0/1 completed"));
    assert!(app_panel_text.contains("Gauge: [------------------------] 0%"));
    assert!(!app_panel_text.contains("Target Root:"));
    assert!(app_panel_text
        .contains("[ ] Inspect current directory (pending, owner=Observer(ccc_scout))"));
    assert!(
        app_panel_text.contains("[ ] Summarize app-panel fallback -> Adjutant(ccc_scribe) model=")
    );
    assert!(app_panel_text.contains("reasoning="));
    assert!(
        app_panel_text.contains("Observer(ccc_scout) running role=Observer(ccc_scout)/explorer")
    );
    assert!(app_panel_text.contains("model=gpt-5.4-mini variant=medium"));
    assert!(app_panel_text.contains("lane=raider-b"));
    assert!(app_panel_text.contains("next=await_fan_in"));
    let artifact_dir = create_temp_path("app-panel-artifact");
    create_dir_all(&artifact_dir).expect("create artifact dir");
    let artifact =
        write_codex_app_panel_artifact(&artifact_dir, &compact["app_panel"]).expect("artifact");
    let markdown_path = artifact["markdown_path"].as_str().expect("markdown path");
    let markdown = fs::read_to_string(markdown_path).expect("read markdown artifact");
    assert!(markdown.contains("# CCC LongWay Panel"));
    assert!(markdown.contains("## Checklist"));
    assert!(markdown.contains("[ ] Inspect current directory (pending, owner=Observer(ccc_scout))"));
    assert!(markdown.contains("Summarize app-panel fallback -> Adjutant(ccc_scribe)"));
    assert!(markdown.contains("_Planned rows_"));
    assert!(markdown.contains("[ ] Summarize app-panel fallback"));
    assert!(markdown.contains("## Subagents"));
    assert!(markdown.contains("next=await_fan_in"));
    assert!(artifact["json_path"]
        .as_str()
        .expect("json path")
        .ends_with("CCC_LONGWAY_PANEL.json"));
    assert!(artifact["latest_markdown_path"]
        .as_str()
        .expect("latest markdown path")
        .ends_with("CCC_LATEST_PANEL.md"));
    assert!(artifact["latest_json_path"]
        .as_str()
        .expect("latest json path")
        .ends_with("CCC_LATEST_PANEL.json"));
    let _ = fs::remove_dir_all(&artifact_dir);
    assert!(text.contains(
        "Spec Split: role_owned=role_config_snapshot[4] sandbox_owned=sandbox_policy_helpers[2] workflow_owned=delegation_plan[13] plan_invariants=delegation_plan_invariants[3]"
    ));
    assert!(text.contains("Lane Artifacts: result=fan_in log=lifecycle recap=fan_in.summary"));
    assert!(text.contains(
        "Verify/Retry/Recap/Report: verify=pending retry=retry recap=lane_artifact_contract.recap report=latest_delegate_result.result_summary|latest_delegate_result.assistant_message_preview"
    ));
    assert!(text.contains("preflight=internal preflight"));
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["status"],
        "<spawned|acknowledged|running|stalled|completed|failed|merged|reclaimed>"
    );
    assert_eq!(
        compact["command_templates"]["start"]["command"],
        "ccc start --quiet --json '{...}'"
    );
    assert_eq!(
        compact["command_templates"]["status"]["command"],
        "ccc status --quiet --json '{\"run_id\":\"run-compact\"}'"
    );
    assert_eq!(
        compact["command_templates"]["status"]["text_command"],
        "ccc status --text --json '{\"run_id\":\"run-compact\"}'"
    );
    assert_eq!(
        compact["command_templates"]["status"]["projection_command"],
        "ccc status --projection --json '{\"run_id\":\"run-compact\"}'"
    );
    assert_eq!(
        compact["command_templates"]["checklist"]["longway_text_command"],
        "ccc checklist --projection --json '{\"run_id\":\"run-compact\"}'"
    );
    assert_eq!(
        compact["command_templates"]["memory"]["payload"]["entries"][0]["kind"],
        "captain_instruction"
    );
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["compact"],
        true
    );
    assert_eq!(
        compact["command_templates"]["orchestrate"]["payload"]["resolve_outcome"],
        "completed"
    );
    assert_eq!(
        compact["command_templates"]["orchestrate"]["payload"]["repair_action"],
        "<role for replan>"
    );
    assert_eq!(
        compact["current_task_card"]["subagent_fan_in"]["summary"],
        "Bounded fan-in summary"
    );
    assert_eq!(
        compact["current_task_card"]["subagent_fan_in"].get("recorded_at"),
        None
    );
    assert_eq!(
        compact["current_task_card"]["parallel_fanout"]["mode"],
        "parallel"
    );
    assert_eq!(
        compact["current_task_card"]["parallel_fanout"]["lanes"][0]["fan_in"]["status"],
        "completed"
    );
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["lane_id"],
        "raider-a"
    );
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["lane_ids"],
        json!(["raider-a", "raider-b"])
    );
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["lane_payload_template"]
            ["confidence"],
        "<low|medium|high>"
    );
    assert_eq!(
        compact["command_templates"]["subagent_update"]["payload"]["lane_payload_template"]
            ["artifacts"]["result"],
        "<fan_in>"
    );
    assert!(compact.get("server_identity").is_none());
    assert!(compact.get("runtime_config").is_none());
}

#[test]
fn codex_app_panel_text_surfaces_target_root_confirmation_when_ambiguous() {
    let payload = json!({
        "run_id": "run-target-root",
        "status": "active",
        "stage": "planning",
        "next_step": "await_longway_approval",
        "can_advance": false,
        "longway": {
            "completed_phase_count": 1,
            "phase_count": 4,
            "current_item": "item-2",
            "lifecycle_state": "pending_approval",
            "planned_row_count": 0,
            "planned_rows": [],
            "phase_rows": [],
            "planning_context": {
                "workspace_root": {
                    "root": "/tmp/work",
                    "root_kind": "ambiguous_child_git_repos",
                    "confidence": "low",
                    "confirmation_required": true,
                    "reason": "Current workspace is not a git repo and contains multiple child git repos; ask the operator to choose the target path.",
                    "candidates": ["/tmp/work/repo-a", "/tmp/work/repo-b"]
                }
            }
        }
    });

    let app_panel = crate::status_app_panel::create_codex_app_panel_payload(&payload);

    assert_eq!(
        app_panel["target_workspace"]["root_kind"],
        "ambiguous_child_git_repos"
    );
    assert_eq!(app_panel["target_workspace"]["confirmation_required"], true);
    assert_eq!(app_panel["target_workspace"]["candidate_count"], 2);
    let target_root_warning = app_panel["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .find(|warning| warning["kind"] == "target_root_confirmation_required")
        .expect("target root warning");
    assert_eq!(
        target_root_warning["retry_command"],
        "$cap Use target_paths=[\"/tmp/work/repo-a\"] and continue this LongWay."
    );
    let text = create_codex_app_panel_text(&app_panel);
    assert!(text.contains("Progress: 1/4 completed"));
    assert!(text.contains("Gauge: [######------------------] 25%"));
    assert!(text.contains("Target Root:"));
    assert!(text.contains(
        "[!] Confirm target path (ambiguous_child_git_repos, confidence=low, candidates=2)"
    ));
    assert!(text.contains("1. repo-a -> /tmp/work/repo-a"));
    assert!(text.contains("2. repo-b -> /tmp/work/repo-b"));
    assert!(text.contains("Retry: $cap Use target_paths=[\"/tmp/work/repo-a\"]"));
}

#[test]
fn codex_app_panel_text_keeps_resolved_target_root_quiet() {
    let payload = json!({
        "run_id": "run-target-root-quiet",
        "status": "active",
        "stage": "planning",
        "next_step": "await_longway_approval",
        "can_advance": false,
        "longway": {
            "completed_phase_count": 2,
            "phase_count": 4,
            "current_item": "item-3",
            "lifecycle_state": "pending_approval",
            "planned_row_count": 0,
            "planned_rows": [],
            "phase_rows": [],
            "planning_context": {
                "workspace_root": {
                    "root": "/tmp/work/repo",
                    "root_kind": "git_repo",
                    "confidence": "high",
                    "confirmation_required": false,
                    "reason": "Resolved from explicit target path in the request.",
                    "candidates": ["/tmp/work/repo"]
                }
            }
        }
    });

    let app_panel = crate::status_app_panel::create_codex_app_panel_payload(&payload);
    let text = create_codex_app_panel_text(&app_panel);

    assert_eq!(
        app_panel["target_workspace"]["confirmation_required"],
        false
    );
    assert!(!app_panel["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .any(|warning| warning["kind"] == "target_root_confirmation_required"));
    assert!(text.contains("Gauge: [############------------] 50%"));
    assert!(!text.contains("Target Root:"));
    assert!(!text.contains("Retry: $cap Use target_paths="));
}

#[test]
fn codex_app_panel_text_surfaces_workspace_state_when_graph_or_memory_exists() {
    let payload = json!({
        "run_id": "run-workspace-state",
        "status": "active",
        "stage": "planning",
        "next_step": "await_longway_approval",
        "can_advance": false,
        "longway": {
            "completed_phase_count": 1,
            "phase_count": 2,
            "current_item": "item-1",
            "lifecycle_state": "pending_approval",
            "planned_row_count": 0,
            "planned_rows": [],
            "phase_rows": [],
            "planning_context": {
                "workspace_root": {
                    "root": "/tmp/work/docs",
                    "root_kind": "document_root",
                    "confidence": "medium",
                    "confirmation_required": false,
                    "reason": "Resolved from document bundle.",
                    "candidates": []
                },
                "graph": {
                    "available": true,
                    "repo_root": "/tmp/work/docs",
                    "store_path": "/tmp/work/docs/.ccc/graph/store.json",
                    "file_count": 2,
                    "tolaria": {
                        "enabled": true,
                        "available": true,
                        "state": "synced",
                        "relative_note_path": "ccc/repos/docs-abcd/graph.md"
                    }
                },
                "memory": {
                    "available": true,
                    "enabled": true,
                    "workspace": "/tmp/work/docs",
                    "entry_count": 1,
                    "captain_instruction_count": 1,
                    "tolaria": {
                        "enabled": true,
                        "available": true,
                        "state": "memory_loaded",
                        "relative_note_path": "ccc/repos/docs-abcd/memory.md"
                    }
                }
            }
        }
    });

    let app_panel = crate::status_app_panel::create_codex_app_panel_payload(&payload);
    let text = create_codex_app_panel_text(&app_panel);

    assert_eq!(app_panel["workspace_state"]["graph"]["file_count"], 2);
    assert_eq!(
        app_panel["workspace_state"]["graph"]["tolaria"]["relative_note_path"],
        "ccc/repos/docs-abcd/graph.md"
    );
    assert!(text.contains("Workspace State:"));
    assert!(
        text.contains("Graph: available=true files=2 mirror=synced ccc/repos/docs-abcd/graph.md")
    );
    assert!(text.contains(
        "Memory: enabled=true entries=1 mirror=memory_loaded ccc/repos/docs-abcd/memory.md"
    ));
}

#[test]
fn codex_app_panel_text_surfaces_document_target_root_confirmation_candidates() {
    let payload = json!({
        "run_id": "run-document-target-root",
        "status": "active",
        "stage": "planning",
        "next_step": "await_longway_approval",
        "can_advance": false,
        "longway": {
            "completed_phase_count": 0,
            "phase_count": 3,
            "current_item": "item-1",
            "lifecycle_state": "pending_approval",
            "planned_row_count": 0,
            "planned_rows": [],
            "phase_rows": [],
            "planning_context": {
                "workspace_root": {
                    "root": "/tmp/work",
                    "root_kind": "ambiguous_target",
                    "confidence": "low",
                    "confirmation_required": true,
                    "reason": "Request mentions multiple target roots; ask the operator to confirm the intended repo or document root.",
                    "candidates": ["/tmp/work/bundle-a", "/tmp/work/bundle-b"]
                }
            }
        }
    });

    let app_panel = crate::status_app_panel::create_codex_app_panel_payload(&payload);
    let text = create_codex_app_panel_text(&app_panel);

    assert_eq!(
        app_panel["target_workspace"]["root_kind"],
        "ambiguous_target"
    );
    assert_eq!(
        app_panel["warnings"][0]["retry_command"],
        "$cap Use target_paths=[\"/tmp/work/bundle-a\"] and continue this LongWay."
    );
    assert!(
        text.contains("[!] Confirm target path (ambiguous_target, confidence=low, candidates=2)")
    );
    assert!(text.contains("1. bundle-a -> /tmp/work/bundle-a"));
    assert!(text.contains("2. bundle-b -> /tmp/work/bundle-b"));
    assert!(text.contains("Retry: $cap Use target_paths=[\"/tmp/work/bundle-a\"]"));
}

#[test]
fn ccc_status_launch_visibility_shows_agent_for_single_worker_tasks() {
    let payload = json!({
        "run_id": "run-single-launch",
        "status": "active",
        "stage": "execution",
        "next_step": "execute_task",
        "can_advance": false,
        "longway": {
            "completed_phase_count": 0,
            "phase_count": 1,
            "current_item": "item-1",
            "lifecycle_state": "active"
        },
        "current_task_card": {
            "task_card_id": "task-single",
            "title": "Update one file",
            "task_kind": "execution",
            "scope": "Update the single file and stop.",
            "assigned_role": "code specialist",
            "assigned_agent_id": "raider",
            "execution_prompt": "Update the single file and stop.",
            "delegation_plan": {
                "fan_in_contract": {
                    "required_fields": [
                        "summary",
                        "status",
                        "evidence_paths",
                        "next_action",
                        "open_questions",
                        "confidence"
                    ]
                }
            }
        },
        "output": {
            "verbosity": "default",
            "changed_max_chars": 120,
            "include_agent_loop_when_idle": false
        }
    });

    let compact = create_ccc_status_compact_payload(&payload);
    assert!(compact["current_task_card"]["launch_visibility"]["lane"].is_null());
    assert_eq!(
        compact["current_task_card"]["launch_visibility"]["lane_ids"],
        json!([])
    );
    assert_eq!(
        compact["current_task_card"]["launch_visibility"]["expected_fan_in"],
        json!([
            "summary",
            "status",
            "evidence_paths",
            "next_action",
            "open_questions",
            "confidence"
        ])
    );

    let text = create_ccc_status_text(&payload);
    assert!(text.contains(
        "Launch: role=Marauder(ccc_raider)/code specialist agent=Marauder(ccc_raider) scope=\"Update the single file and stop.\" expected_fan_in=summary,status,evidence_paths,next_action,open_questions,confidence"
    ));
    assert!(!text.contains("lane="));
}

#[test]
fn specialist_shortlist_from_config_selects_docs_agent_without_full_roster_scan() {
    let config = json!({
        "routing": {
            "mode": "category_shortlist",
            "categories": {
                "write_docs": {
                    "keywords": ["docs", "readme", "release note"],
                    "intent_types": ["documentation"],
                    "tool_signals": ["filesystem"],
                    "agents": ["scribe", "scout"]
                }
            }
        },
        "agents": {
            "documenter": {
                "name": "scribe",
                "summary": "Docs and release-note updates.",
                "model": "gpt-5.4-mini",
                "variant": "medium",
                "fast_mode": false,
                "config_entries": []
            },
            "explorer": {
                "name": "scout",
                "summary": "Read-only repo investigation.",
                "model": "gpt-5.4-mini",
                "variant": "medium",
                "fast_mode": false,
                "config_entries": []
            }
        }
    });

    let payload = create_specialist_shortlist_payload_from_config(
        &config,
        "Update the README docs for the release and document the migration.",
        "code specialist",
        None,
    );
    assert_eq!(payload["selected_category"], "write_docs");
    assert_eq!(payload["selected_role"], "documenter");
    assert_eq!(payload["selected_agent_id"], "scribe");
    assert_eq!(
        payload["selected_routing_evidence_source"],
        "skill_registry"
    );
    assert_eq!(
        payload["selected_display_metadata_sources"],
        json!({
            "agent": "skill_registry",
            "model": "role_config",
            "variant": "role_config",
            "reasoning": "role_config"
        })
    );
    assert_eq!(
        payload["candidates"][0]["summary"],
        "Docs and release-note updates."
    );
    assert_eq!(
        payload["candidates"][0]["skill_registry"]["scheduling"]["display_agent_id"],
        "ccc_scribe"
    );
}

#[test]
fn specialist_shortlist_routes_translation_requests_to_scribe() {
    let payload = create_specialist_shortlist_payload_from_config(
        &json!({
            "generated_defaults": {
                "version": 9,
                "policy": "ccc-managed-defaults"
            },
            "agents": {
                "documenter": {
                    "name": "scribe",
                    "summary": "Docs and translation updates.",
                    "model": "gpt-5.4-mini",
                    "variant": "medium",
                    "fast_mode": true,
                    "config_entries": []
                }
            }
        }),
        "Translate README.md and docs/install.md into Korean.",
        "code specialist",
        None,
    );

    assert_eq!(payload["selected_category"], "write_docs");
    assert_eq!(payload["selected_role"], "documenter");
    assert_eq!(payload["selected_agent_id"], "scribe");
    assert_eq!(
        payload["selected_routing_evidence_source"],
        "skill_registry"
    );
    assert!(payload["intent_types"]
        .as_array()
        .unwrap()
        .contains(&Value::String("documentation".to_string())));
}

#[test]
fn specialist_shortlist_routes_review_of_release_docs_to_arbiter() {
    let payload = create_specialist_shortlist_payload_from_config(
        &json!({
            "generated_defaults": {
                "version": 9,
                "policy": "ccc-managed-defaults"
            },
            "agents": {
                "documenter": {
                    "name": "scribe",
                    "summary": "Docs and release-note updates.",
                    "model": "gpt-5.4-mini",
                    "variant": "medium",
                    "fast_mode": true,
                    "config_entries": []
                },
                "verifier": {
                    "name": "arbiter",
                    "summary": "Review and acceptance checks.",
                    "model": "gpt-5.4",
                    "variant": "medium",
                    "fast_mode": false,
                    "config_entries": []
                }
            }
        }),
        "Review docs/release-work/0.0.11/PRE_RELEASE_PLAN.md for regressions and acceptance gaps.",
        "verifier",
        None,
    );

    assert_eq!(payload["request_shape"], "review");
    assert!(payload["intent_types"]
        .as_array()
        .unwrap()
        .contains(&Value::String("review".to_string())));
    assert!(payload["intent_types"]
        .as_array()
        .unwrap()
        .contains(&Value::String("documentation".to_string())));
    assert_eq!(payload["selected_category"], "verify");
    assert_eq!(payload["selected_role"], "verifier");
    assert_eq!(payload["selected_agent_id"], "arbiter");
    assert_eq!(
        payload["selected_routing_evidence_source"],
        "skill_registry"
    );
    assert_eq!(
        payload["candidates"][0]["skill_registry"]["scheduling"]["role_family"],
        "review_acceptance"
    );
}

#[test]
fn specialist_shortlist_preserves_explicit_disabled_routing() {
    let payload = create_specialist_shortlist_payload_from_config(
        &json!({
            "routing": {
                "mode": "disabled"
            },
            "agents": {
                "documenter": {
                    "name": "scribe",
                    "summary": "Docs and translation updates.",
                    "model": "gpt-5.4-mini",
                    "variant": "high",
                    "fast_mode": true,
                    "config_entries": []
                }
            }
        }),
        "Translate README.md and docs/install.md into Korean.",
        "code specialist",
        None,
    );

    assert_eq!(payload["mode"], "disabled");
    assert!(payload["selected_role"].is_null());
    assert_eq!(
        payload["summary"],
        "Category shortlist routing is disabled."
    );
}

#[test]
fn specialist_shortlist_prefers_companion_owner_when_shortlist_contains_it() {
    let config = json!({
        "routing": {
            "mode": "category_shortlist",
            "categories": {
                "write_code": {
                    "keywords": ["fix", "patch", "git"],
                    "intent_types": ["mutation"],
                    "tool_signals": ["git"],
                    "agents": ["raider", "companion_operator"]
                }
            }
        },
        "agents": {
            "code specialist": {
                "name": "raider",
                "summary": "Bounded code mutation.",
                "model": "gpt-5.3-codex",
                "variant": "high",
                "fast_mode": true,
                "config_entries": []
            }
        },
        "companion_agents": {
            "companion_reader": {
                "name": "companion_reader",
                "summary": "Read-only companion.",
                "model": "gpt-5.4-mini",
                "variant": "medium",
                "fast_mode": false,
                "config_entries": []
            },
            "companion_operator": {
                "name": "companion_operator",
                "summary": "Git-aware operator.",
                "model": "gpt-5.4-mini",
                "variant": "medium",
                "fast_mode": false,
                "config_entries": []
            }
        }
    });

    let payload = create_specialist_shortlist_payload_from_config(
        &config,
        "Fix the failing hook and create the git commit.",
        "code specialist",
        Some("companion_operator"),
    );
    assert_eq!(payload["selected_category"], "write_code");
    assert_eq!(payload["selected_role"], "companion_operator");
    assert_eq!(payload["selected_agent_id"], "companion_operator");
    assert_eq!(
        payload["selected_routing_evidence_source"],
        "skill_registry"
    );
    assert_eq!(
        payload["candidates"][1]["skill_registry"]["scheduling"]["mutation_allowed"],
        true
    );
}

#[test]
fn specialist_shortlist_does_not_downgrade_way_planning_to_read_repo() {
    let config = json!({
        "routing": {
            "mode": "category_shortlist",
            "categories": {
                "read_repo": {
                    "keywords": ["read", "inspect", "trace", "find", "analyze", "why", "where"],
                    "intent_types": ["read_only", "diagnosis"],
                    "tool_signals": ["filesystem"],
                    "agents": ["scout", "companion_reader"]
                }
            }
        }
    });

    let payload = create_specialist_shortlist_payload_from_config(
        &config,
        "Retry Phase 3 Step 2 with a bounded planning pass",
        "way",
        None,
    );

    assert_eq!(payload["request_shape"], "way");
    assert_eq!(payload["intent_types"], json!(["planning"]));
    assert!(payload["selected_role"].is_null());
    assert_eq!(
        payload["summary"],
        "Category shortlist found no confident match; fell back to way."
    );
}

#[test]
fn routing_trace_protocol_routes_docs_to_scribe_skill() {
    let trace = create_routing_trace_payload(
        "Update README.md docs and release notes for the migration.",
        "code specialist",
    );

    assert_eq!(trace["selected_category"], "write_docs");
    assert_eq!(trace["selected_role"], "documenter");
    assert_eq!(trace["selected_agent_id"], "scribe");
    assert_eq!(trace["selected_skill"]["id"], "ccc_scribe");
    assert_eq!(
        trace["routing_protocol"]["schema"],
        "ccc.internal_routing.v1"
    );
    assert_eq!(
        trace["routing_protocol"]["selected_skill"]["id"],
        "ccc_scribe"
    );
    assert_eq!(trace["risk"], "low");
    assert_eq!(trace["mutation_intent"], "explicit_or_strong");
    assert_eq!(trace["verification_need"], "focused_validation_required");
    assert!(trace["reason"]
        .as_str()
        .expect("reason")
        .contains("Category write_docs selected documenter/scribe"));
}

#[test]
fn routing_trace_protocol_routes_code_mutation_to_raider_skill() {
    let trace = create_routing_trace_payload(
        "Implement a bounded code patch in rust/ccc-mcp/src/request_routing.rs.",
        "code specialist",
    );

    assert_eq!(trace["selected_category"], "write_code");
    assert_eq!(trace["selected_role"], "code specialist");
    assert_eq!(trace["selected_agent_id"], "raider");
    assert_eq!(trace["selected_skill_id"], "ccc_raider");
    assert_eq!(trace["risk"], "medium");
    assert_eq!(
        trace["evidence_need"],
        "changed_files_and_validation_results"
    );
    assert_eq!(trace["verification_need"], "focused_validation_required");
}

#[test]
fn routing_trace_protocol_routes_review_to_arbiter_skill() {
    let trace = create_routing_trace_payload(
        "Review docs/release-work/0.0.15/PRE_RELEASE_PLAN.md for regressions and acceptance gaps.",
        "verifier",
    );

    assert_eq!(trace["request_shape"], "review");
    assert_eq!(trace["selected_category"], "verify");
    assert_eq!(trace["selected_role"], "verifier");
    assert_eq!(trace["selected_agent_id"], "arbiter");
    assert_eq!(trace["selected_skill_name"], "ccc_arbiter");
    assert_eq!(trace["evidence_need"], "findings_and_acceptance_evidence");
    assert_eq!(trace["verification_need"], "review_judgment_required");
}

#[test]
fn status_compact_and_text_show_compact_routing_trace() {
    let payload = json!({
        "run_id": "run-routing-visible",
        "next_step": "execute_task",
        "run_truth_surface": {
            "fan_in_ready": false
        },
        "longway": {
            "planned_rows": []
        },
        "current_task_card": {
            "task_card_id": "task-routing",
            "title": "Implement routing trace visibility",
            "assigned_role": "code specialist",
            "assigned_agent_id": "raider",
            "routing_trace": {
                "source": "task_card",
                "selected_category": "write_code",
                "selected_skill_id": "ccc_raider",
                "selected_skill_name": "ccc_raider",
                "risk": "medium",
                "selected_role": "code specialist",
                "selected_agent_id": "raider",
                "reason": "Category write_code selected code specialist/raider from the shortlist.",
                "specialist_route": {
                    "candidates": ["raw roster should not be needed"]
                }
            }
        }
    });

    let compact = create_ccc_status_compact_payload(&payload);
    assert_eq!(
        compact["current_task_card"]["routing_trace"]["selected_category"],
        "write_code"
    );
    assert_eq!(
        compact["current_task_card"]["routing_trace"]["selected_skill_id"],
        "ccc_raider"
    );
    assert_eq!(
        compact["current_task_card"]["routing_trace"]["reason"],
        "Category write_code selected code specialist/raider from the shortlist."
    );
    assert!(compact["current_task_card"]["routing_trace"]
        .get("specialist_route")
        .is_none());

    let status_text = create_ccc_status_operator_text(&payload);
    assert!(status_text.contains("Routing: category=write_code skill=Marauder(ccc_raider)"));
    assert!(status_text.contains("role=Marauder(ccc_raider)/code specialist"));
    assert!(status_text.contains("agent=Marauder(ccc_raider)"));
    assert!(status_text.contains("reason=\"Category write_code selected code specialist/raider"));
    assert!(!status_text.contains("raw roster"));

    let projection_text = create_operator_longway_projection_text(&payload);
    assert!(projection_text.contains("Routing: category=write_code skill=Marauder(ccc_raider)"));
}

#[test]
fn status_text_renders_companion_callsigns_without_rewriting_machine_ids() {
    let payload = json!({
        "run_id": "run-companion-routing-visible",
        "next_step": "execute_task",
        "current_task_card": {
            "task_card_id": "task-companion",
            "title": "Inspect docs through the companion reader",
            "assigned_role": "companion_reader",
            "assigned_agent_id": "companion_reader",
            "routing_trace": {
                "selected_category": "read_only",
                "selected_skill_id": "ccc_companion_reader",
                "selected_role": "companion_reader",
                "selected_agent_id": "companion_reader",
                "risk": "low"
            }
        }
    });

    let compact = create_ccc_status_compact_payload(&payload);
    assert_eq!(
        compact["current_task_card"]["routing_trace"]["selected_skill_id"],
        "ccc_companion_reader"
    );
    assert_eq!(
        compact["current_task_card"]["routing_trace"]["selected_agent_id"],
        "companion_reader"
    );

    let status_text = create_ccc_status_operator_text(&payload);
    assert!(status_text.contains("skill=Probe(ccc_companion_reader)"));
    assert!(status_text.contains("role=Probe(ccc_companion_reader)/companion_reader"));
    assert!(status_text.contains("agent=Probe(ccc_companion_reader)"));
}

#[test]
fn status_compact_sanitizes_scheduler_selected_planned_row_routing_trace() {
    let payload = json!({
        "run_id": "run-scheduler-planned-row-sanitize",
        "next_step": "execute_task",
        "scheduler": {
            "schema": "ccc.scheduler.v1",
            "selected_planned_row": {
                "row_index": 0,
                "title": "Repair status payload",
                "routing_trace": {
                    "selected_category": "write_code",
                    "selected_skill_id": "ccc_raider",
                    "selected_role": "code specialist",
                    "selected_agent_id": "raider",
                    "risk": "medium",
                    "evidence_need": "focused_diff",
                    "verification_need": "focused_tests",
                    "reason": "Route the bounded repair to the code specialist.",
                    "summary": "Keep scheduler planned-row routing compact.",
                    "specialist_route": {
                        "candidates": ["raider", "scribe", "arbiter"]
                    },
                    "tool_route": {
                        "candidates": ["rg", "cargo test"]
                    },
                    "candidate_roster": ["raw", "roster"],
                    "query_result": {
                        "raw_graph_dump": ["full", "graph", "output"]
                    },
                    "paths": ["rust/ccc-mcp/src/status_payload.rs"],
                    "terms": ["routing_trace"],
                    "query": "review_context"
                }
            },
            "latest_transition": {
                "transition_id": "transition-0001",
                "selected_planned_row": {
                    "row_index": 0,
                    "routing_trace": {
                        "category": "verify",
                        "skill": "ccc_arbiter",
                        "role": "verifier",
                        "agent": "arbiter",
                        "tool_route": {
                            "candidates": ["raw transition route"]
                        }
                    }
                }
            }
        }
    });

    let compact = create_ccc_status_compact_payload(&payload);
    let trace = &compact["scheduler"]["selected_planned_row"]["routing_trace"];
    assert_eq!(trace["selected_category"], "write_code");
    assert_eq!(trace["selected_skill_id"], "ccc_raider");
    assert_eq!(trace["selected_role"], "code specialist");
    assert_eq!(trace["selected_agent_id"], "raider");
    assert_eq!(trace["risk"], "medium");
    assert_eq!(trace["evidence_need"], "focused_diff");
    assert_eq!(trace["verification_need"], "focused_tests");
    assert_eq!(
        trace["reason"],
        "Route the bounded repair to the code specialist."
    );
    assert_eq!(
        trace["summary"],
        "Keep scheduler planned-row routing compact."
    );
    for forbidden in [
        "specialist_route",
        "tool_route",
        "candidate_roster",
        "query_result",
        "paths",
        "terms",
        "query",
    ] {
        assert!(trace.get(forbidden).is_none(), "{forbidden} leaked");
    }
    assert_eq!(
        compact["scheduler"]["latest_transition"]["selected_planned_row"]["routing_trace"]
            ["selected_skill_id"],
        "ccc_arbiter"
    );
    assert!(!serde_json::to_string(&compact["scheduler"])
        .expect("serialize compact scheduler")
        .contains("raw_graph_dump"));
    assert!(!serde_json::to_string(&compact["scheduler"])
        .expect("serialize compact scheduler")
        .contains("raw transition route"));
}

#[test]
fn create_ccc_status_text_shows_exactly_one_routing_line() {
    let payload = json!({
        "run_id": "run-single-routing-line",
        "next_step": "execute_task",
        "run_truth_surface": {
            "fan_in_ready": false
        },
        "current_task_card": {
            "task_card_id": "task-routing",
            "title": "Repair duplicate routing line",
            "assigned_role": "code specialist",
            "assigned_agent_id": "raider",
            "routing_trace": {
                "selected_category": "write_code",
                "selected_skill_id": "ccc_raider",
                "selected_role": "code specialist",
                "selected_agent_id": "raider",
                "risk": "medium",
                "reason": "Status text should only include this routing line once."
            }
        },
        "state_contract": {
            "state": "ready",
            "active_gate": "dispatch",
            "required_artifact": "task_card",
            "next_step": "execute_task"
        },
        "recovery_lane": {
            "status": "clear",
            "recommended_action": "none",
            "reclaim_replan_action": "none",
            "needs_operator_attention": false,
            "target_count": 0,
            "summary": "No recovery needed."
        }
    });

    let text = create_ccc_status_text(&payload);
    let routing_line_count = text
        .lines()
        .filter(|line| line.starts_with("Routing:"))
        .count();
    assert_eq!(routing_line_count, 1, "{text}");
}

#[test]
fn ccc_status_surfaces_active_checkpoint_resume_capsule() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-active-checkpoint");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-active-checkpoint");

    let mut run_record = read_json_document(&run_directory.join("run.json")).expect("run record");
    run_record["approval_state"] = json!("approved_for_task_cards");
    run_record["active_thread_id"] = json!("thread-checkpoint");
    run_record["child_agents"] = json!([{
        "agent_id": "ccc_raider",
        "role": "code specialist",
        "task_card_id": "task-1",
        "lane_id": "raider-a",
        "status": "spawned",
        "thread_id": "thread-checkpoint",
        "summary": "Implement active checkpoint capsule.",
        "created_at": "2099-01-01T00:00:00.000Z",
        "updated_at": "2099-01-01T00:00:00.000Z",
        "execution_mode": "codex_subagent"
    }]);
    write_json_document(&run_directory.join("run.json"), &run_record).expect("write run");

    let mut run_state =
        read_json_document(&run_directory.join("run-state.json")).expect("run-state");
    run_state["next_action"] = json!({ "command": "await_fan_in" });
    write_json_document(&run_directory.join("run-state.json"), &run_state)
        .expect("write run-state");

    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["title"] = json!("Implement checkpoint and resume UX");
    task_card["subagent_lifecycle"] = json!({
        "status": "spawned",
        "child_agent_id": "ccc_raider",
        "thread_id": "thread-checkpoint",
        "summary": "Host subagent is active."
    });
    task_card["late_subagent_output"] = json!({
        "status": "completed",
        "summary": "Late stale output arrived after reclaim.",
        "authority": "stale_output_preserved"
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-active-checkpoint",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["active_checkpoint"]["schema"],
        "ccc.active_checkpoint.v1"
    );
    assert_eq!(
        status_payload["active_checkpoint"]["continuation_command"],
        "$cap continue run-active-checkpoint"
    );
    assert_eq!(
        status_payload["active_checkpoint"]["delegated_work"]["host_subagent_active"],
        1
    );
    assert_eq!(
        status_payload["active_checkpoint"]["late_output"]["count"],
        1
    );
    assert_eq!(
        status_payload["active_checkpoint"]["late_output"]["authority"],
        "stale_output_preserved"
    );
    assert_eq!(
        status_payload["app_panel"]["active_checkpoint"]["task_card_id"],
        "task-1"
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["active_checkpoint"]["resume_action"],
        status_payload["active_checkpoint"]["resume_action"]
    );

    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Checkpoint:"));
    assert!(text.contains("role=Marauder(ccc_raider)/code specialist"));
    assert!(text.contains("agent=Marauder(ccc_raider)"));
    assert!(text.contains("continue=\"$cap continue run-active-checkpoint\""));
    assert!(text.contains("late=completed(1) authority=stale-output-preserved"));

    let projection_text = create_operator_longway_projection_text(&status_payload);
    assert!(projection_text.contains("Checkpoint:"));
    assert!(projection_text.contains("$cap continue run-active-checkpoint"));

    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Checkpoint:"));
    assert!(app_panel_text.contains("$cap continue run-active-checkpoint"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_task_session_state_for_watch_text() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-task-session-state");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-task-session-state");

    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["role_config_snapshot"] = json!({
        "model": "gpt-5.4-mini",
        "variant": "medium"
    });
    task_card["verification_state"] = json!("needs_work");
    task_card["subagent_lifecycle"] = json!({
        "status": "running",
        "child_agent_id": "ccc_raider",
        "summary": "Implementation is active."
    });
    task_card["subagent_fan_in"] = json!({
        "status": "completed",
        "summary": "One focused diff is ready for review.",
        "evidence_paths": ["rust/ccc-mcp/src/status_payload.rs:1"],
        "open_questions": ["Remaining risk needs review."]
    });
    task_card["review_fan_in"] = json!({
        "outcome": "needs_work",
        "unresolved_finding_count": 2,
        "evidence_paths": ["rust/ccc-mcp/src/status_render.rs:1"]
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-task-session-state",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["task_session_state"]["schema"],
        "ccc.task_session_state.v1"
    );
    assert_eq!(
        status_payload["task_session_state"]["public_command_path"],
        false
    );
    assert_eq!(
        status_payload["task_session_state"]["delegated_agent"]["child_agent_id"],
        "ccc_raider"
    );
    assert_eq!(
        status_payload["task_session_state"]["delegated_agent"]["model"],
        "gpt-5.4-mini"
    );
    assert_eq!(status_payload["task_session_state"]["evidence"]["count"], 2);
    assert_eq!(
        status_payload["task_session_state"]["verification"]["unresolved_risk_count"],
        2
    );
    assert_eq!(
        status_payload["app_panel"]["task_session_state"]["active_task"]["task_card_id"],
        "task-1"
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["task_session_state"]["internal_session"]["session_id"],
        status_payload["task_session_state"]["internal_session"]["session_id"]
    );

    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains("Task Session: task=task-1"));
    assert!(status_text.contains("agent=Marauder(ccc_raider)"));
    assert!(status_text.contains("model=gpt-5.4-mini/medium"));
    assert!(status_text.contains("evidence=2 verification=needs-work unresolved_risk=2"));

    let projection_text = create_operator_longway_projection_text(&status_payload);
    assert!(projection_text.contains("Task Session: task=task-1"));
    assert!(projection_text.contains("session=mcp-session-"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_natural_workflow_loop_projection() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-workflow-loop");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-workflow-loop");

    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["assigned_role"] = json!("code specialist");
    task_card["assigned_agent_id"] = json!("raider");
    task_card["routing_trace"] = json!({
        "mutation_intent": "strong_mutation",
        "evidence_need": "focused_repository_inspection"
    });
    task_card["subagent_lifecycle"] = json!({
        "status": "running",
        "child_agent_id": "ccc_raider",
        "summary": "Implementing a focused workflow-loop projection."
    });
    task_card["subagent_fan_in"] = json!({
        "status": "running",
        "summary": "Implementation in progress."
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-workflow-loop",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["workflow_loop"]["schema"],
        "ccc.workflow_loop.v1"
    );
    assert_eq!(status_payload["workflow_loop"]["operator_visible"], true);
    assert_eq!(status_payload["workflow_loop"]["public_commands"], false);
    assert_eq!(
        status_payload["workflow_loop"]["summary"],
        "requirements understanding -> planning -> exploration -> modification -> review -> verification"
    );
    assert_eq!(
        status_payload["workflow_loop"]["current_stage"],
        "modification"
    );
    assert_eq!(
        status_payload["workflow_loop"]["stages"]
            .as_array()
            .map(Vec::len),
        Some(6)
    );
    assert_eq!(
        status_payload["workflow_loop"]["stages"][0]["status"],
        "completed"
    );
    assert_eq!(
        status_payload["workflow_loop"]["stages"][3]["label"],
        "modification"
    );
    assert_eq!(
        status_payload["workflow_loop"]["stages"][3]["status"],
        "active"
    );
    assert_eq!(
        status_payload["app_panel"]["workflow_loop"]["current_stage"],
        "modification"
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(compact["workflow_loop"]["current_stage"], "modification");

    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains("Workflow Loop: status=active current=modification"));
    assert!(status_text.contains("requirements-understanding:completed"));
    assert!(status_text.contains("verification:pending"));

    let projection_text = create_operator_longway_projection_text(&status_payload);
    assert!(projection_text.contains("Workflow Loop: status=active current=modification"));

    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Workflow Loop:"));
    assert!(app_panel_text.contains("current=modification"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_surfaces_verification_capsule_and_delegated_ownership() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-verification-ownership");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-verification-ownership");

    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["acceptance"] = json!("Status includes a concise closeout capsule.");
    task_card["routing_trace"] = json!({
        "mutation_intent": "strong_mutation",
        "paths": ["rust/ccc-mcp/src/status_payload.rs"],
        "terms": ["verification capsule"]
    });
    task_card["subagent_lifecycle"] = json!({
        "status": "completed",
        "child_agent_id": "ccc_raider"
    });
    task_card["subagent_fan_in"] = json!({
        "status": "completed",
        "summary": "Capsule implementation is complete.",
        "evidence_paths": ["rust/ccc-mcp/src/status_payload.rs:1"],
        "checks": ["cargo test ccc_status_surfaces_verification_capsule_and_delegated_ownership"]
    });
    task_card["review_fan_in"] = json!({
        "outcome": "passed",
        "evidence_paths": ["rust/ccc-mcp/src/status_render.rs:1"],
        "unresolved_finding_count": 0
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-verification-ownership",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["current_task_card"]["verification_capsule"]["schema"],
        "ccc.verification_capsule.v1"
    );
    assert_eq!(
        status_payload["current_task_card"]["verification_capsule"]["evidence"]["count"],
        2
    );
    assert_eq!(
        status_payload["current_task_card"]["verification_capsule"]["reviewer_verdict"],
        "passed"
    );
    assert_eq!(
        status_payload["current_task_card"]["verification_capsule"]["validation"]["count"],
        1
    );
    assert_eq!(
        status_payload["current_task_card"]["delegated_ownership"]["repeat_guard"]["policy"],
        "do_not_repeat_delegated_search_or_mutation_without_recorded_reason"
    );
    assert_eq!(
        status_payload["active_checkpoint"]["delegated_work"]["ownership"]["owner"]
            ["assigned_agent_id"],
        "raider"
    );
    assert_eq!(
        status_payload["task_session_state"]["delegated_ownership"]["mutation_ownership"]["active"],
        true
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["current_task_card"]["verification_capsule"]["reviewer_verdict"],
        "passed"
    );

    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains(
        "Verification Capsule: evidence=2 validation=1 reviewer_verdict=passed unresolved_risk=0"
    ));
    assert!(status_text.contains(
        "Delegated Ownership: agent=Marauder(ccc_raider) search_paths=1 search_terms=1 mutation=true"
    ));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn assignment_quality_flags_docs_routed_to_raider_as_drift() {
    let task_card = json!({
        "title": "Update 0.0.10 release plan docs",
        "intent": "Document captain assignment-quality routing drift.",
        "scope": "docs/release-work/0.0.10/PRE_RELEASE_PLAN.md",
        "acceptance": "Release docs explain docs/planning routing.",
        "task_kind": "execution",
        "assigned_role": "code specialist",
        "assigned_agent_id": "ccc_raider"
    });

    let quality = create_assignment_quality_payload(&task_card);

    assert_eq!(quality["state"], "mismatch");
    assert_eq!(quality["expected_family"], "docs_planning");
    assert_eq!(quality["expected_agent_ids"][1], "ccc_scribe");
    assert!(quality["summary"]
        .as_str()
        .unwrap()
        .contains("Routing drift"));
}

#[test]
fn assignment_quality_flags_partial_docs_assignment_mismatches() {
    for (assigned_role, assigned_agent_id) in [
        ("documenter", "ccc_raider"),
        ("code specialist", "ccc_scribe"),
    ] {
        let task_card = json!({
            "title": "Update release plan docs",
            "intent": "Document captain assignment-quality routing drift.",
            "scope": "docs/release-work/0.0.10/PRE_RELEASE_PLAN.md",
            "assigned_role": assigned_role,
            "assigned_agent_id": assigned_agent_id
        });

        let quality = create_assignment_quality_payload(&task_card);

        assert_eq!(quality["state"], "mismatch");
        assert_eq!(quality["expected_family"], "docs_planning");
        assert!(quality["summary"]
            .as_str()
            .unwrap()
            .contains("Routing drift"));
    }
}

#[test]
fn assignment_quality_accepts_expected_specialist_families() {
    let docs_task = json!({
        "title": "Update release plan docs",
        "scope": "docs/release-work/0.0.10/PRE_RELEASE_PLAN.md",
        "assigned_role": "documenter",
        "assigned_agent_id": "ccc_scribe"
    });
    let readonly_task = json!({
        "title": "Investigate current routing behavior",
        "scope": "Read-only design assessment of request_routing.rs",
        "assigned_role": "explorer",
        "assigned_agent_id": "ccc_scout"
    });
    let mutation_task = json!({
        "title": "Implement a small status guard",
        "scope": "Bounded code/config mutation in request_routing.rs",
        "assigned_role": "code specialist",
        "assigned_agent_id": "ccc_raider"
    });
    let review_task = json!({
        "title": "Review assignment-quality guard acceptance",
        "task_kind": "review",
        "assigned_role": "verifier",
        "assigned_agent_id": "ccc_arbiter"
    });
    let operator_task = json!({
        "title": "Create the release git tag",
        "scope": "Run git tag for the validated release.",
        "assigned_role": "companion_operator",
        "assigned_agent_id": "companion_operator"
    });
    let smoke_task = json!({
        "title": "0.0.11 Smoke CLI Status",
        "scope": "Check installed status and Codex app visibility only.",
        "assigned_role": "explorer",
        "assigned_agent_id": "scout"
    });

    assert_eq!(
        create_assignment_quality_payload(&docs_task)["state"],
        "matched"
    );
    assert_eq!(
        create_assignment_quality_payload(&readonly_task)["expected_family"],
        "read_only_investigation_or_design"
    );
    assert_eq!(
        create_assignment_quality_payload(&readonly_task)["state"],
        "matched"
    );
    assert_eq!(
        create_assignment_quality_payload(&mutation_task)["state"],
        "matched"
    );
    assert_eq!(
        create_assignment_quality_payload(&review_task)["state"],
        "matched"
    );
    assert_eq!(
        create_assignment_quality_payload(&operator_task)["expected_family"],
        "operator_side_mutation"
    );
    assert_eq!(
        create_assignment_quality_payload(&operator_task)["state"],
        "matched"
    );
    let smoke_quality = create_assignment_quality_payload(&smoke_task);
    assert_eq!(smoke_quality["expected_family"], "read_only_diagnostic");
    assert_eq!(smoke_quality["state"], "matched");
}

#[test]
fn assignment_quality_prefers_explicit_planned_row_owner_over_smoke_text() {
    let raider_smoke_task = json!({
        "title": "Minimal raider mutation: apply the smallest harmless smoke marker",
        "scope": "Only after captain accepts scout evidence, perform the smallest safe mutation needed to exercise raider routing.",
        "acceptance": "Persist evidence that mutation routing uses code specialist/raider with workspace-write sandbox.",
        "assigned_role": "code specialist",
        "assigned_agent_id": "raider",
        "planned_longway_row": {
            "planned_role": "code specialist",
            "planned_agent_id": "raider"
        }
    });
    let arbiter_smoke_task = json!({
        "title": "Arbiter verification: review scout and raider evidence for the smoke",
        "scope": "Read-only review of persisted run evidence and focused validation output.",
        "assigned_role": "verifier",
        "assigned_agent_id": "arbiter",
        "task_kind": "review",
        "planned_longway_row": {
            "planned_role": "verifier",
            "planned_agent_id": "arbiter"
        }
    });

    let raider_quality = create_assignment_quality_payload(&raider_smoke_task);
    assert_eq!(raider_quality["state"], "matched");
    assert_eq!(raider_quality["expected_family"], "bounded_mutation");
    assert_eq!(raider_quality["drift_severity"], "none");
    assert_eq!(raider_quality["reason"], "Approved LongWay planned-row owner metadata is explicit and takes precedence over text-only diagnostic heuristics.");

    let arbiter_quality = create_assignment_quality_payload(&arbiter_smoke_task);
    assert_eq!(arbiter_quality["state"], "matched");
    assert_eq!(arbiter_quality["expected_family"], "review_acceptance");
    assert_eq!(arbiter_quality["drift_severity"], "none");
}

#[test]
fn assignment_quality_treats_plan_sequence_way_as_phase_aware_match() {
    let task_card = json!({
        "title": "CCC visibility smoke",
        "intent": "Check installed status and Codex app visibility only.",
        "scope": "Read-only diagnostic planning before approved execution.",
        "sequence": "PLAN_SEQUENCE",
        "task_kind": "way",
        "assigned_role": "way",
        "assigned_agent_id": "ccc_tactician"
    });

    let quality = create_assignment_quality_payload(&task_card);

    assert_eq!(quality["state"], "matched");
    assert_eq!(quality["phase"], "planning");
    assert_eq!(quality["expected_family"], "planning_way");
    assert_eq!(quality["drift_severity"], "info");
    assert_eq!(
        quality["route_relation"],
        "planning_route_valid_execution_route_deferred"
    );
    assert_eq!(quality["execution_expected_family"], "read_only_diagnostic");
    assert_eq!(quality["execution_expected_agent_ids"][1], "ccc_scout");
}

#[test]
fn ccc_status_text_surfaces_assignment_quality_warning() {
    let payload = json!({
        "next_step": "execute_task",
        "run_truth_surface": {
            "fan_in_ready": false
        },
        "worker_visibility": {
            "active_worker_count": 0
        },
        "longway": {
            "completed_phase_count": 0,
            "phase_count": 1,
            "current_item": "item-1",
            "phase_rows": []
        },
        "current_task_card": {
            "assigned_role": "code specialist",
            "assigned_agent_id": "ccc_raider",
            "assignment_quality": {
                "state": "mismatch",
                "expected_family": "docs_planning",
                "assigned_role": "code specialist",
                "assigned_agent_id": "ccc_raider",
                "reason": "Documentation, release-plan, and planning-doc updates should route to ccc_scribe."
            }
        },
        "output": {},
        "token_usage": {
            "total_tokens": 0
        }
    });

    let text = create_ccc_status_text(&payload);

    assert!(text.contains(
        "Assignment Warning: routing-drift assigned=Marauder(ccc_raider)/code specialist/Marauder(ccc_raider) expected=docs_planning"
    ));
}

#[test]
fn ccc_status_surfaces_concise_registry_evidence() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-registry-evidence");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-registry-evidence");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("task card");
    task_card["delegation_plan"] = create_specialist_delegation_plan_with_runtime(
        "code specialist",
        &json!({
            "summary": "Bounded code mutation.",
            "model": "gpt-5.5",
            "variant": "high",
            "fast_mode": false
        }),
        &json!({
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec"
        }),
        "workspace-write",
        "Mutation work needs workspace-write.",
    );
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-registry-evidence",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(
        status_payload["registry_evidence"]["schema"],
        "ccc.registry_evidence_status.v1"
    );
    assert_eq!(
        status_payload["registry_evidence"]["source"],
        "skill_registry"
    );
    assert_eq!(
        status_payload["app_panel"]["registry_evidence"]["schema"],
        "ccc.registry_evidence_status.v1"
    );
    assert_eq!(
        status_payload["registry_evidence"]["agent_name"],
        "ccc_raider"
    );
    let text = create_ccc_status_text(&status_payload);
    assert!(text.contains("Registry: Marauder(ccc_raider) status="));
    assert!(text.contains("ssl="));
    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Registry: Marauder(ccc_raider) status="));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn create_ccc_status_operator_text_defaults_to_checklist_sequence_registry_and_next() {
    let payload = json!({
        "next_step": "await_operator",
        "run_truth_surface": {
            "fan_in_ready": false
        },
        "worker_visibility": {
            "active_worker_count": 1
        },
        "sequence": "PLAN_SEQUENCE",
        "stage": "planning",
        "approval_state": "pending_longway_approval",
        "longway": {
            "current_item": "item-1",
            "phase_rows": [{
                "title": "Draft bounded plan",
                "status": "in_progress",
                "owner_agent": "ccc_tactician"
            }]
        },
        "registry_evidence": {
            "agent_name": "ccc_tactician",
            "status": "ok",
            "manifest_status": "ok",
            "advisory_only": true
        },
        "token_usage": {
            "total_tokens": 12345,
            "by_agent": [{
                "agent_id": "ccc_tactician",
                "total_tokens": 12345,
                "context_tokens_estimate": 45678
            }]
        },
        "code_graph": {
            "available": true,
            "file_count": 3,
            "resolution": "repo graph"
        },
        "memory": {
            "configured": true,
            "enabled": true,
            "entry_count": 2,
            "path": ".ccc/memory.json"
        },
        "host_subagent_state": {
            "reclaim_replan_recommendation": {
                "summary": "internal reclaim details"
            },
            "active_handle_cleanup": {
                "state": "active",
                "summary": "internal handle details"
            }
        },
        "current_task_card": {
            "assigned_role": "way",
            "assigned_agent_id": "ccc_tactician",
            "scope": "Planning smoke only",
            "delegation_plan": {
                "runtime_dispatch": {
                    "preferred_execution_mode": "codex_subagent",
                    "fallback_execution_mode": "codex_exec",
                    "preferred_custom_agent_name": "ccc_tactician",
                    "model": "gpt-5.5",
                    "variant": "medium"
                },
                "transport_guidance": {
                    "preferred": "host_subagent",
                    "fallback": "codex_exec",
                    "summary": "internal transport"
                },
                "cost_routing": {
                    "state": "ok",
                    "summary": "internal cost"
                },
                "spec_split": {
                    "state": "not_required",
                    "summary": "internal spec"
                },
                "lane_artifact_contract": {
                    "result_field": "result",
                    "log_field": "log",
                    "recap_field": "recap"
                },
                "verify_retry_recap_report_contract": {
                    "verify": "pending",
                    "recap_field": "recap"
                }
            },
            "assignment_quality": {
                "state": "mismatch",
                "expected_family": "read_only_diagnostic",
                "assigned_role": "way",
                "assigned_agent_id": "ccc_tactician"
            },
            "review_policy": {
                "state": "recommended",
                "decision": "recommend_single",
                "risk": "moderate",
                "active_reviewers": 0,
                "reviewer_cap": 1
            },
            "completion_discipline": {
                "state": "required"
            }
        },
        "captain_action_contract": {
            "allowed_action": "blocked",
            "required_action": "await_longway_approval"
        },
        "latest_captain_intervention": {
            "classification": "stale_output",
            "chosen_next_action": "await_operator"
        },
        "output": {
            "verbosity": "default",
            "include_agent_loop_when_idle": true
        }
    });

    let text = create_ccc_status_operator_text(&payload);

    assert!(text.starts_with("LongWay\n[>] Draft bounded plan [Executor(ccc_tactician)]"));
    assert!(
        text.contains("Sequence: PLAN_SEQUENCE stage=planning approval=pending_longway_approval")
    );
    assert!(text.contains("Registry: Executor(ccc_tactician) status=ok ssl=ok advisory=true"));
    assert!(text.ends_with("Next: operator"));
    for noisy_label in [
        "Tokens:",
        "By Agent:",
        "Estimated Context:",
        "Gauge:",
        "Launch:",
        "Dispatch:",
        "Transport:",
        "Cost:",
        "Assignment Warning:",
        "Spec Split:",
        "Lane Artifacts:",
        "Verify/Retry/Recap/Report:",
        "Graph:",
        "Graph Warning:",
        "Memory:",
        "Completion:",
        "Review:",
        "Captain Guard:",
        "Intervention:",
        "Agent Loop:",
        "Spawned:",
        "Host Subagents:",
        "Host Handles:",
    ] {
        assert!(
            !text.contains(noisy_label),
            "{noisy_label} leaked in:\n{text}"
        );
    }
}

#[test]
fn create_ccc_status_operator_text_uses_projection_status_when_diff_flow_is_active() {
    let payload = json!({
        "run_id": "run-projection-text",
        "next_step": "await_operator",
        "operator_longway_projection": {
            "kind": "ccc_longway_projection",
            "path": "/tmp/work/CCC_LONGWAY_PROJECTION.md",
            "stable": true,
            "diff_visibility": {
                "status": "git_intent_to_add",
                "diff_command": "git diff -- CCC_LONGWAY_PROJECTION.md"
            }
        },
        "sequence": "PLAN_SEQUENCE",
        "stage": "planning",
        "approval_state": "pending_longway_approval",
        "longway": {
            "phase_rows": [{
                "title": "Draft bounded plan",
                "status": "in_progress",
                "owner_agent": "ccc_tactician"
            }]
        }
    });

    let text = create_ccc_status_operator_text(&payload);

    assert!(text.starts_with("LongWay Projection: CCC_LONGWAY_PROJECTION.md"));
    assert!(text.contains(
        "Progress: view CCC_LONGWAY_PROJECTION.md or refresh with ccc status --projection --json '{\"run_id\":\"run-projection-text\"}'"
    ));
    assert!(text.contains("Diff: git diff -- CCC_LONGWAY_PROJECTION.md"));
    assert!(
        text.contains("Sequence: PLAN_SEQUENCE stage=planning approval=pending_longway_approval")
    );
    assert!(text.ends_with("Next: operator"));
    assert!(!text.contains("[>] Draft bounded plan"));
    assert_eq!(
        payload["longway"]["phase_rows"][0]["title"],
        "Draft bounded plan"
    );
}

#[test]
fn create_subagents_text_renders_parallel_lanes_as_separate_rows() {
    let payload = json!({
        "current_task_card": {
            "task_card_id": "task-lanes",
            "title": "Implement lane-only status",
            "assigned_role": "code specialist",
            "parallel_fanout": {
                "lanes": [
                    {
                        "lane_id": "raider-a",
                        "lifecycle": {
                            "status": "running",
                            "child_agent_id": "ccc_raider_a",
                            "summary": "Editing the renderer."
                        }
                    },
                    {
                        "lane_id": "raider-b",
                        "lifecycle": {
                            "status": "completed",
                            "child_agent_id": "ccc_raider_b"
                        },
                        "fan_in": {
                            "status": "completed",
                            "summary": "Parser and command routing are covered."
                        }
                    },
                    {
                        "lane_id": "raider-c",
                        "required": true,
                        "scope": null,
                        "lifecycle": null,
                        "fan_in": null
                    }
                ]
            }
        },
        "sequence": "EXECUTE_SEQUENCE",
        "registry_evidence": {
            "agent_name": "ccc_raider",
            "status": "ok"
        },
        "next_step": "advance"
    });

    let text = create_subagents_text(&payload);
    let lines = text.lines().collect::<Vec<_>>();

    assert_eq!(lines[0], "Subagents");
    assert_eq!(
        lines[1],
        "[>] raider-a running child=Marauder(ccc_raider_a) role=Marauder(ccc_raider)/code specialist task=\"Implement lane-only status\""
    );
    assert_eq!(
        lines[2],
        "[x] raider-b completed child=Marauder(ccc_raider_b) role=Marauder(ccc_raider)/code specialist task=\"Implement lane-only status\""
    );
    assert_eq!(
        lines[3],
        "[ ] raider-c not-started child=unassigned role=Marauder(ccc_raider)/code specialist task=\"Implement lane-only status\""
    );
    assert_eq!(lines.len(), 4);
    assert!(!text.contains("summary="));
    assert!(!text.contains("Registry:"));
    assert!(!text.contains("Sequence:"));
    assert!(!text.contains("Next:"));
}

#[test]
fn create_subagents_text_falls_back_to_host_subagent_activity() {
    let payload = json!({
        "host_subagent_state": {
            "subagent_activity": [{
                "child_agent_id": "ccc_scout",
                "assigned_role": "explorer",
                "task_card_id": "task-host",
                "task_title": "Inspect current directory",
                "lane_id": "scout-a",
                "status": "running",
                "summary": "Collecting evidence."
            }]
        }
    });

    let text = create_subagents_text(&payload);

    assert_eq!(
        text,
        "Subagents\n[>] scout-a running child=Observer(ccc_scout) role=Observer(ccc_scout)/explorer task=\"Inspect current directory\""
    );
    assert!(!text.contains("summary="));
}

#[test]
fn create_operator_longway_projection_text_localizes_for_korean_prompt() {
    let payload = json!({
        "run_id": "run-ko",
        "next_step": "advance",
        "current_task_card": {
            "task_card_id": "task-ko",
            "title": "상태 투영 구현",
            "execution_prompt": "상태/checklist에서 LongWay 투영 파일을 만들어줘",
            "assigned_role": "code specialist",
            "parallel_fanout": {
                "lanes": [{
                    "lane_id": "raider-a",
                    "lifecycle": {
                        "status": "running",
                        "child_agent_id": "ccc_raider"
                    }
                }]
            }
        }
    });

    let text = create_operator_longway_projection_text(&payload);

    assert!(text.starts_with("LongWay 투영\n"));
    assert!(text.contains("실행: run-ko"));
    assert!(text.contains("다음: advance"));
    assert!(text.contains("서브에이전트"));
    assert!(text.contains(
        "[>] raider-a running 자식=Marauder(ccc_raider) 역할=Marauder(ccc_raider)/code specialist 작업=\"상태 투영 구현\""
    ));
    assert!(!text.contains("Subagents"));
    assert!(!text.contains("child="));
    assert!(!text.contains("role="));
    assert!(!text.contains("task=\""));
}

#[test]
fn create_operator_longway_projection_text_stays_short_with_many_lanes() {
    let lanes = (0..14)
        .map(|index| {
            json!({
                "lane_id": format!("raider-{index}"),
                "lifecycle": {
                    "status": "running",
                    "child_agent_id": format!("ccc_raider_{index}")
                }
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "run_id": "run-many",
        "current_task_card": {
            "title": "Implement bounded projection",
            "assigned_role": "code specialist",
            "parallel_fanout": {
                "lanes": lanes
            }
        }
    });

    let text = create_operator_longway_projection_text(&payload);
    let subagent_rows = text
        .lines()
        .filter(|line| line.starts_with("["))
        .collect::<Vec<_>>();

    assert_eq!(subagent_rows.len(), 13);
    assert!(text.contains("[ ] ... 2 more rows omitted"));
    assert!(!text.contains("raider-13"));
}

#[test]
fn write_operator_longway_projection_overwrites_one_stable_workspace_file() {
    let workspace_dir = create_temp_path("operator-longway-projection");
    create_dir_all(&workspace_dir).expect("create workspace");
    let git_init = Command::new("git")
        .arg("-C")
        .arg(&workspace_dir)
        .arg("init")
        .output()
        .expect("run git init");
    assert!(
        git_init.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&git_init.stderr)
    );
    let run_one = workspace_dir.join(".ccc").join("runs").join("run-one");
    let run_two = workspace_dir.join(".ccc").join("runs").join("run-two");
    let payload_one = json!({
        "run_id": "run-one",
        "current_task_card": {
            "title": "First projection",
            "assigned_role": "code specialist",
            "parallel_fanout": {
                "lanes": [{
                    "lane_id": "raider-a",
                    "lifecycle": {
                        "status": "running",
                        "child_agent_id": "ccc_raider_a"
                    }
                }]
            }
        }
    });
    let payload_two = json!({
        "run_id": "run-two",
        "current_task_card": {
            "title": "Second projection",
            "assigned_role": "explorer",
            "parallel_fanout": {
                "lanes": [{
                    "lane_id": "scout-a",
                    "fan_in": {
                        "status": "completed",
                        "child_agent_id": "ccc_scout"
                    }
                }]
            }
        }
    });

    let first = write_operator_longway_projection(&workspace_dir, &run_one, &payload_one)
        .expect("write first");
    let first_path = PathBuf::from(first["path"].as_str().expect("first path"));
    let expected_path = workspace_dir.join("CCC_LONGWAY_PROJECTION.md");
    assert_eq!(
        first_path,
        fs::canonicalize(&expected_path).expect("canonical projection path")
    );
    assert!(!first_path.to_string_lossy().contains(".ccc"));
    assert!(!first_path.to_string_lossy().contains("run-one"));
    let first_text = fs::read_to_string(&first_path).expect("read first projection");
    assert!(first_text.contains("Run: run-one"));
    assert!(first_text.contains("First projection"));
    assert_eq!(first["diff_visibility"]["status"], "git_intent_to_add");
    let diff_name = Command::new("git")
        .arg("-C")
        .arg(&workspace_dir)
        .arg("diff")
        .arg("--name-only")
        .arg("--")
        .arg("CCC_LONGWAY_PROJECTION.md")
        .output()
        .expect("run git diff");
    assert!(
        diff_name.status.success(),
        "git diff failed: {}",
        String::from_utf8_lossy(&diff_name.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&diff_name.stdout).trim(),
        "CCC_LONGWAY_PROJECTION.md"
    );

    let second = write_operator_longway_projection(&workspace_dir, &run_two, &payload_two)
        .expect("write second");
    let second_path = PathBuf::from(second["path"].as_str().expect("second path"));
    assert_eq!(second_path, first_path);
    let second_text = fs::read_to_string(&second_path).expect("read second projection");
    assert!(second_text.contains("Run: run-two"));
    assert!(second_text.contains("Second projection"));
    assert!(!second_text.contains("First projection"));
    assert_eq!(second["stable"], true);

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn captain_direct_mutation_guard_flags_dirty_paths_since_run_start_baseline() {
    let workspace_dir = create_temp_path("captain-direct-mutation-guard");
    create_dir_all(&workspace_dir).expect("create workspace");
    let git_init = Command::new("git")
        .arg("-C")
        .arg(&workspace_dir)
        .arg("init")
        .output()
        .expect("run git init");
    assert!(
        git_init.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&git_init.stderr)
    );
    write(
        workspace_dir.join("preexisting.txt"),
        "dirty before CCC start\n",
    )
    .expect("write preexisting dirty file");

    let start_payload = create_ccc_start_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "goal": "Guard direct mutation",
        "title": "Detect direct mutation",
        "intent": "Persist a baseline and surface unrecorded direct mutation drift",
        "scope": "Single status guard test",
        "acceptance": "Status surfaces changed dirty paths",
        "prompt": "Detect mutation drift only.",
        "task_kind": "execution",
        "sequence": "EXECUTE_SEQUENCE"
    }))
    .expect("start run");
    let run_directory = PathBuf::from(
        start_payload["run_directory"]
            .as_str()
            .expect("run directory"),
    );
    let run_id = start_payload["run_id"]
        .as_str()
        .expect("run id")
        .to_string();
    let run_record = read_json_document(&run_directory.join("run.json")).expect("read run");
    assert_eq!(
        run_record["worktree_mutation_baseline"]["status"],
        "available"
    );
    assert_eq!(
        run_record["worktree_mutation_baseline"]["dirty_paths"],
        json!(["preexisting.txt"])
    );

    write(workspace_dir.join("direct.txt"), "dirty after CCC start\n")
        .expect("write direct dirty file");
    write(
        workspace_dir.join("preexisting.txt"),
        "dirty before CCC start, then changed after\n",
    )
    .expect("mutate preexisting dirty file");
    let locator = ResolvedRunLocator {
        cwd: workspace_dir.clone(),
        run_id: run_id.clone(),
        run_directory: run_directory.clone(),
    };
    let session_context = create_session_context();
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(
        status_payload["captain_direct_mutation_guard"]["state"],
        "blocked_unrecorded_direct_mutation"
    );
    assert_eq!(
        status_payload["captain_direct_mutation_guard"]["changed_paths"],
        json!(["direct.txt", "preexisting.txt"])
    );
    assert_eq!(
        status_payload["captain_direct_mutation_guard"]["required_action"],
        "record_terminal_fallback_or_operator_override_before_captain_merge"
    );

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(
        compact["captain_direct_mutation_guard"]["state"],
        "blocked_unrecorded_direct_mutation"
    );
    assert_eq!(
        status_payload["app_panel"]["captain_direct_mutation_guard"]["changed_path_count"],
        2
    );
    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains("Captain Direct Mutation Guard"));
    assert!(status_text.contains("paths=direct.txt"));
    assert!(status_text.contains("preexisting.txt"));
    let projection_text = create_operator_longway_projection_text(&status_payload);
    assert!(projection_text.contains("Captain Direct Mutation Guard"));
    assert!(projection_text.contains("paths=direct.txt"));
    let app_panel_text = create_codex_app_panel_text(&status_payload["app_panel"]);
    assert!(app_panel_text.contains("Mutation Guard:"));
    assert!(app_panel_text.contains("paths=direct.txt"));

    let task_card_id = status_payload["active_task_card_id"]
        .as_str()
        .expect("task card id");
    mark_task_card_codex_exec_fallback(&run_directory, task_card_id);
    let fallback_status =
        create_ccc_status_payload(&session_context, &locator).expect("fallback status payload");
    assert_eq!(
        fallback_status["captain_direct_mutation_guard"]["state"],
        "exception_recorded"
    );
    assert_eq!(
        fallback_status["captain_direct_mutation_guard"]["changed_path_count"],
        0
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn operator_longway_projection_renders_plan_rows_and_cleanup_on_completion() {
    let workspace_dir = create_temp_path("operator-longway-projection-cleanup");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_dir = workspace_dir.join(".ccc").join("runs").join("run-plan");
    create_dir_all(&run_dir).expect("create run dir");
    let pending_payload = json!({
        "run_id": "run-plan",
        "stage": "planning",
        "sequence": "PLAN_SEQUENCE",
        "approval_state": "pending_longway_approval",
        "next_step": "await_longway_approval",
        "run_state": {
            "next_action": {
                "command": "await_longway_approval"
            }
        },
        "longway": {
            "phase_count": 1,
            "current_item": "item-1",
            "active_phase_name": "plan",
            "active_phase_status": "pending_longway_approval",
            "planned_rows": [{
                "title": "Patch Way-first LongWay projection",
                "planned_role": "code specialist",
                "planned_agent_id": "raider",
                "scope": "Implement projection lifecycle",
                "acceptance": "Codex diff shows the LongWay plan",
                "status": "planned"
            }]
        },
        "current_task_card": {
            "title": "Draft bounded LongWay",
            "assigned_role": "way"
        }
    });

    let projection = sync_operator_longway_projection(&workspace_dir, &run_dir, &pending_payload)
        .expect("write pending projection");
    let projection_path = PathBuf::from(projection["path"].as_str().expect("projection path"));
    let text = fs::read_to_string(&projection_path).expect("read projection");
    assert!(text.contains("LongWay Projection"));
    assert!(text.contains("Planned: Patch Way-first LongWay projection"));
    assert!(text.contains("Approval"));
    assert!(text.contains("Confirm whether to execute this LongWay plan"));

    let completed_payload = json!({
        "run_id": "run-plan",
        "status": "completed",
        "run_state": {
            "next_action": {
                "command": "halt_completed"
            }
        }
    });
    let cleanup = sync_operator_longway_projection(&workspace_dir, &run_dir, &completed_payload)
        .expect("cleanup terminal projection");
    assert_eq!(cleanup["status"], "removed");
    assert!(!projection_path.exists());

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_payload_marks_operator_language_from_korean_prompt() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("operator-language-korean");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-operator-language");
    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file).expect("read run record");
    run_record["prompt"] = Value::String("서브에이전트별 LongWay 투영을 보여줘".to_string());
    run_record["prompt_refinement"] = json!({
        "schema": "ccc.prompt_refinement.v1",
        "state": "disabled",
        "enabled": false,
        "execution_mode": "internal",
        "owner": "captain",
        "captain_gate": "accept_adjust_reject",
        "longway_materialization_allowed": false,
        "task_card_creation_allowed": false,
        "source": "ccc_promptsmith",
        "task_card_id": run_record
            .get("active_task_card_id")
            .and_then(Value::as_str)
            .unwrap_or("task-1"),
        "created_at": run_record
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or("2024-01-01T00:00:00.000Z"),
        "consumed_at": Value::Null,
        "recorded_at": run_record
            .get("updated_at")
            .and_then(Value::as_str)
            .unwrap_or("2024-01-01T00:00:00.000Z")
    });
    write_json_document(&run_file, &run_record).expect("write run record");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-operator-language",
            "cwd": workspace_dir.to_string_lossy(),
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(status_payload["operator_language"], "ko");

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn specialist_delegation_plan_defaults_to_subagent_with_structured_fan_in() {
    let runtime_config = json!({
        "preferred_specialist_execution_mode": "codex_subagent",
        "fallback_specialist_execution_mode": "codex_exec",
    });
    let snapshot = json!({
        "summary": "Read-only repo investigation and evidence gathering.",
        "model": "gpt-5.4-mini",
        "variant": "medium",
        "fast_mode": false,
    });

    let payload = create_specialist_delegation_plan_with_runtime(
        "explorer",
        &snapshot,
        &runtime_config,
        "read-only",
        "Scout work is evidence gathering and should not mutate the workspace.",
    );

    assert_eq!(payload["preferred_execution_mode"], "codex_subagent");
    assert_eq!(payload["fallback_execution_mode"], "codex_exec");
    assert_eq!(payload["preferred_custom_agent_name"], "ccc_scout");
    assert_eq!(payload["preferred_custom_agent_file"], "ccc-scout.toml");
    assert_eq!(payload["runtime_dispatch"]["source"], "config_backed");
    assert_eq!(
        payload["runtime_dispatch"]["execution_mode_source"],
        "runtime_config"
    );
    assert_eq!(
        payload["runtime_dispatch"]["role_profile_source"],
        "role_config_snapshot"
    );
    assert_eq!(
        payload["runtime_dispatch"]["custom_agent_source"],
        "role_mapping"
    );
    assert_eq!(
        payload["runtime_dispatch"]["preferred_custom_agent_name"],
        "ccc_scout"
    );
    assert_eq!(
        payload["runtime_dispatch"]["plan_invariants_source"],
        "delegation_plan_invariants"
    );
    assert_eq!(
        payload["spec_surfaces"]["workflow_owned"]["fields"],
        json!([
            "routing_source",
            "preferred_execution_mode",
            "fallback_execution_mode",
            "supported_execution_modes",
            "execution_contract",
            "runtime_dispatch",
            "expertise_framing",
            "fan_in_contract",
            "lane_artifact_contract",
            "verify_retry_recap_report_contract",
            "subagent_spawn_contract",
            "subagent_update_contract",
            "fallback_gate"
        ])
    );
    assert_eq!(payload["fan_in_contract"]["mode"], "structured_summary");
    assert_eq!(
        payload["fan_in_contract"]["required_fields"],
        json!([
            "summary",
            "status",
            "evidence_paths",
            "next_action",
            "open_questions",
            "confidence"
        ])
    );
    assert_eq!(
        payload["lane_artifact_contract"]["result"]["field"],
        "fan_in"
    );
    assert_eq!(
        payload["lane_artifact_contract"]["recap"]["source"],
        "parallel_fanout.lanes[].fan_in.summary"
    );
    assert_eq!(
        payload["verify_retry_recap_report_contract"]["verify"]["field"],
        "verification_state"
    );
    assert_eq!(
        payload["verify_retry_recap_report_contract"]["retry"]["budget_key"],
        "retry"
    );
    assert_eq!(
        payload["verify_retry_recap_report_contract"]["report"]["field"],
        "latest_delegate_result.result_summary"
    );
    assert_eq!(
        payload["spec_surfaces"]["role_owned"]["fields"],
        json!(["summary", "model", "variant", "fast_mode"])
    );
    assert_eq!(
        payload["spec_surfaces"]["sandbox_owned"]["fields"],
        json!(["sandbox_mode", "sandbox_rationale"])
    );
    assert_eq!(
        payload["spec_surfaces"]["plan_invariants"]["fields"],
        json!([
            "policy_drift_check_required",
            "captain_checkpoint_required",
            "fallback_reason_codes"
        ])
    );
    assert_eq!(
        payload["spec_surfaces"]["skill_ssl_manifest"]["owned_by"],
        "skill_registry"
    );
    assert_eq!(payload["skill_ssl_manifest"]["blocking"], false);
    assert_eq!(payload["skill_ssl_manifest"]["advisory_only"], true);
    assert_eq!(payload["skill_registry"]["schema"], "ccc.skill_registry.v1");
    assert_eq!(payload["skill_registry"]["runtime_truth"], false);
    assert_eq!(
        payload["runtime_dispatch"]["skill_registry"]["blocking"],
        false
    );
    assert_eq!(
        payload["runtime_dispatch"]["skill_ssl_manifest"]["blocking"],
        false
    );
    assert_eq!(
        payload["subagent_spawn_contract"]["forbid_full_history_fork"],
        true
    );
    assert_eq!(
        payload["subagent_spawn_contract"]["omit_model_override"],
        true
    );
    assert_eq!(
        payload["subagent_update_contract"]["transport"],
        "ccc_cli_subcommand"
    );
    assert_eq!(
        payload["subagent_update_contract"]["command"],
        "ccc subagent-update --quiet --json '{...}'"
    );
    assert_eq!(
        payload["subagent_update_contract"]["default_payload_transport"],
        "inline_json"
    );
    assert_eq!(
        payload["subagent_update_contract"]["inline_command"],
        "ccc subagent-update --quiet --json '{...}'"
    );
    assert_eq!(
        payload["fallback_gate"]["require_explicit_subagent_fallback_reason"],
        true
    );
    assert_eq!(payload["policy_drift_check_required"], true);
}

#[test]
fn execution_contract_registry_describes_configured_ccc_roles() {
    let config = json!({
        "runtime": {
            "preferred_specialist_execution_mode": "codex_subagent",
            "fallback_specialist_execution_mode": "codex_exec"
        },
        "agents": {
            "code specialist": {
                "summary": "Bounded code and config mutation for implementation and repair.",
                "display_name": "Marauder",
                "callsign": "Marauder",
                "theme": "starcraft_display_callsign",
                "inspired_by": ["oh-my-openagent"],
                "recommended_workflows": ["remove-deadcode", "ai-slop-remover", "lsp-safe-refactor"],
                "lsp_capabilities": ["lsp_diagnostics", "lsp_rename"],
                "model": "gpt-5.5",
                "variant": "high",
                "fast_mode": true,
                "config_entries": ["model_reasoning_effort=\"high\""]
            },
            "verifier": {
                "summary": "Review, regression detection, and acceptance judgment when needed.",
                "model": "gpt-5.5",
                "variant": "high",
                "fast_mode": true,
                "config_entries": []
            },
            "explorer": {
                "summary": "Read-only repo investigation and evidence gathering.",
                "model": "gpt-5.4-mini",
                "variant": "medium",
                "fast_mode": true,
                "config_entries": []
            }
        }
    });

    let registry =
        crate::execution_contract::create_execution_contract_registry_from_config(&config);
    let roles = registry["roles"].as_array().expect("contract roles");
    let find_role = |role: &str| {
        roles
            .iter()
            .find(|contract| {
                contract
                    .pointer("/role_identity/role")
                    .and_then(Value::as_str)
                    == Some(role)
            })
            .unwrap_or_else(|| panic!("missing contract for {role}"))
    };
    let raider = find_role("code specialist");
    let scout = find_role("explorer");
    let arbiter = find_role("verifier");

    assert_eq!(registry["schema"], "ccc.execution_contract.v1");
    assert_eq!(registry["advisory_only"], true);
    assert_eq!(
        registry["runtime_truth"],
        "persisted_run_and_task_card_state"
    );
    assert_eq!(registry["role_count"], 8);
    assert_eq!(raider["role_identity"]["agent_id"], "raider");
    assert_eq!(raider["role_identity"]["custom_agent_name"], "ccc_raider");
    assert_eq!(raider["role_identity"]["callsign"], "Marauder");
    assert_eq!(
        raider["role_identity"]["theme"],
        "starcraft_display_callsign"
    );
    assert_eq!(raider["cost_tier"], "high_tier");
    assert_eq!(raider["model_policy"]["model"], "gpt-5.5");
    assert_eq!(raider["model_policy"]["source"], "shared_role_config");
    assert_eq!(
        raider["model_policy"]["recommended_workflows"],
        json!(["remove-deadcode", "ai-slop-remover", "lsp-safe-refactor"])
    );
    assert_eq!(
        raider["model_policy"]["lsp_capabilities"],
        json!(["lsp_diagnostics", "lsp_rename"])
    );
    assert_eq!(
        raider["supported_modes"],
        json!(["codex_subagent", "codex_exec"])
    );
    assert_eq!(raider["mutation_capability"]["can_mutate_workspace"], true);
    assert_eq!(raider["review_capability"]["can_gate_acceptance"], false);
    assert_eq!(scout["mutation_capability"]["can_mutate_workspace"], false);
    assert_eq!(scout["tool_restrictions"]["sandbox_mode"], "read-only");
    assert_eq!(arbiter["review_capability"]["can_gate_acceptance"], true);
    assert!(raider["fallback_policy"]["reason_codes"]
        .as_array()
        .expect("reason codes")
        .iter()
        .any(|code| code.as_str() == Some("child_timeout")));
}

#[test]
fn skill_ssl_manifest_parser_accepts_low_risk_scout_manifest() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .join("skills")
        .join("ssl")
        .join("ccc_scout.skill.ssl.json");

    let payload =
        crate::skill_manifest::load_skill_ssl_manifest_from_path("ccc_scout", &manifest_path);

    assert_eq!(payload["status"], "available");
    assert_eq!(payload["blocking"], false);
    assert_eq!(payload["advisory_only"], true);
    assert_eq!(payload["runtime_truth"], false);
    assert_eq!(payload["display_name"], "Observer");
    assert_eq!(payload["callsign"], "Observer");
    assert_eq!(payload["theme"], "starcraft_display_callsign");
    assert_eq!(
        payload["recommended_workflows"],
        json!(["github-triage", "get-unpublished-changes"])
    );
    assert_eq!(
        payload["lsp_capabilities"],
        json!(["lsp_diagnostics", "lsp_references", "lsp_definition"])
    );
    assert_eq!(payload["scheduling"]["display_agent_id"], "ccc_scout");
    assert_eq!(payload["logical"]["external_side_effects"], false);
    assert_eq!(payload["logical"]["actions"][0]["risk"], "low");
}

#[test]
fn skill_ssl_manifest_parser_treats_invalid_manifest_as_non_blocking() {
    let manifest_dir = create_temp_path("invalid-skill-ssl");
    create_dir_all(&manifest_dir).expect("create manifest dir");
    let manifest_path = manifest_dir.join("ccc_scout.skill.ssl.json");
    write(
        &manifest_path,
        r#"{
          "skill_id": "ccc_scout",
          "version": "0.1",
          "scheduling": {},
          "logical": {}
        }"#,
    )
    .expect("write invalid manifest");

    let payload =
        crate::skill_manifest::load_skill_ssl_manifest_from_path("ccc_scout", &manifest_path);

    assert_eq!(payload["status"], "invalid");
    assert_eq!(payload["blocking"], false);
    assert_eq!(payload["advisory_only"], true);
    assert_eq!(payload["fallback"], "SKILL.md + ccc-config.toml");
    assert!(payload["reason"]
        .as_str()
        .expect("reason")
        .contains("structural"));

    let _ = fs::remove_dir_all(&manifest_dir);
}

#[test]
fn skill_ssl_manifest_parser_reports_stale_version_as_non_blocking() {
    let manifest_dir = create_temp_path("stale-skill-ssl");
    create_dir_all(&manifest_dir).expect("create manifest dir");
    let manifest_path = manifest_dir.join("ccc_scout.skill.ssl.json");
    write(
        &manifest_path,
        r#"{
          "skill_id": "ccc_scout",
          "version": "0.0",
          "scheduling": {
            "role_family": "read_only_exploration",
            "display_agent_id": "ccc_scout",
            "intent_signatures": [],
            "expected_inputs": [],
            "expected_outputs": [],
            "mutation_allowed": false
          },
          "structural": { "scenes": [] },
          "logical": {
            "actions": [],
            "requires_operator_approval": false,
            "external_side_effects": false
          }
        }"#,
    )
    .expect("write stale manifest");

    let payload =
        crate::skill_manifest::load_skill_ssl_manifest_from_path("ccc_scout", &manifest_path);

    assert_eq!(payload["status"], "stale");
    assert_eq!(payload["blocking"], false);
    assert!(payload["reason"]
        .as_str()
        .expect("reason")
        .contains("supported version"));

    let _ = fs::remove_dir_all(&manifest_dir);
}

#[test]
fn skill_ssl_manifest_parser_reports_skill_id_drift_as_non_blocking() {
    let manifest_dir = create_temp_path("drift-skill-ssl");
    create_dir_all(&manifest_dir).expect("create manifest dir");
    let manifest_path = manifest_dir.join("ccc_scout.skill.ssl.json");
    write(
        &manifest_path,
        r#"{
          "skill_id": "ccc_scribe",
          "version": "0.1",
          "scheduling": {
            "role_family": "read_only_exploration",
            "display_agent_id": "ccc_scout",
            "intent_signatures": [],
            "expected_inputs": [],
            "expected_outputs": [],
            "mutation_allowed": false
          },
          "structural": { "scenes": [] },
          "logical": {
            "actions": [],
            "requires_operator_approval": false,
            "external_side_effects": false
          }
        }"#,
    )
    .expect("write drifted manifest");

    let payload =
        crate::skill_manifest::load_skill_ssl_manifest_from_path("ccc_scout", &manifest_path);

    assert_eq!(payload["status"], "drift_detected");
    assert_eq!(payload["blocking"], false);
    assert!(payload["reason"]
        .as_str()
        .expect("reason")
        .contains("does not match"));

    let _ = fs::remove_dir_all(&manifest_dir);
}

#[test]
fn skill_ssl_manifest_parser_requires_minimal_routing_and_risk_fields() {
    let manifest_dir = create_temp_path("missing-required-skill-ssl");
    create_dir_all(&manifest_dir).expect("create manifest dir");
    let manifest_path = manifest_dir.join("ccc_scout.skill.ssl.json");
    write(
        &manifest_path,
        r#"{
          "skill_id": "ccc_scout",
          "version": "0.1",
          "scheduling": {
            "display_agent_id": "ccc_scout",
            "mutation_allowed": false
          },
          "structural": {},
          "logical": {
            "actions": []
          }
        }"#,
    )
    .expect("write incomplete manifest");

    let payload =
        crate::skill_manifest::load_skill_ssl_manifest_from_path("ccc_scout", &manifest_path);

    assert_eq!(payload["status"], "invalid");
    assert_eq!(payload["blocking"], false);
    let reason = payload["reason"].as_str().expect("reason");
    assert!(reason.contains("scheduling.role_family"));
    assert!(reason.contains("structural.scenes"));
    assert!(reason.contains("logical.requires_operator_approval"));

    let _ = fs::remove_dir_all(&manifest_dir);
}

#[test]
fn skill_registry_wraps_available_manifest_with_config_evidence() {
    let manifest = json!({
        "status": "available",
        "blocking": false,
        "runtime_truth": false,
        "advisory_only": true,
        "scheduling": { "display_agent_id": "ccc_scout" },
        "structural": { "scenes": [] },
        "logical": { "actions": [] }
    });
    let role_config = json!({
        "model": "gpt-5.4-mini",
        "variant": "high",
        "fast_mode": true,
        "summary": "Read-only repo investigation."
    });

    let payload =
        crate::skill_registry::build_skill_registry_payload("ccc_scout", manifest, &role_config);

    assert_eq!(payload["schema"], "ccc.skill_registry.v1");
    assert_eq!(payload["status"], "available");
    assert_eq!(payload["blocking"], false);
    assert_eq!(payload["runtime_truth"], false);
    assert_eq!(payload["source_priority"][0], "persisted_run_state");
    assert_eq!(payload["role_config"]["model"], "gpt-5.4-mini");
    assert_eq!(
        payload["skill_ssl_manifest"]["scheduling"]["display_agent_id"],
        "ccc_scout"
    );
}

#[test]
fn skill_registry_keeps_missing_manifest_non_blocking() {
    let manifest = json!({
        "status": "missing",
        "blocking": false,
        "runtime_truth": false,
        "advisory_only": true,
        "reason": "not found"
    });

    let payload =
        crate::skill_registry::build_skill_registry_payload("ccc_raider", manifest, &json!({}));

    assert_eq!(payload["status"], "missing");
    assert_eq!(payload["blocking"], false);
    assert_eq!(payload["fallback"], "SKILL.md + ccc-config.toml");
    assert_eq!(payload["evidence_sources"][2]["source"], "skill_ssl_json");
    assert_eq!(payload["evidence_sources"][2]["available"], false);
}

#[test]
fn skill_ssl_manifest_parser_covers_all_managed_custom_agents() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf();
    let agents = [
        "ccc_tactician",
        "ccc_scout",
        "ccc_raider",
        "ccc_scribe",
        "ccc_arbiter",
        "ccc_sentinel",
        "ccc_companion_reader",
        "ccc_companion_operator",
    ];

    for agent in agents {
        let manifest_path = repo_root
            .join("skills")
            .join("ssl")
            .join(format!("{agent}.skill.ssl.json"));
        let payload =
            crate::skill_manifest::load_skill_ssl_manifest_from_path(agent, &manifest_path);

        assert_eq!(payload["status"], "available", "{agent}");
        assert_eq!(payload["blocking"], false, "{agent}");
        assert!(
            payload["scheduling"]["mutation_allowed"].is_boolean(),
            "{agent}"
        );
        assert!(payload["structural"]["scenes"].is_array(), "{agent}");
        assert!(payload["logical"]["actions"].is_array(), "{agent}");
    }
}

#[test]
fn build_task_card_payload_with_role_includes_delegation_plan() {
    let payload = build_task_card_payload_with_role(
        "run-123",
        "task-123",
        "Inspect auth flow",
        "Gather bounded evidence for captain.",
        "Trace the auth path and return concise findings.",
        "Inspect the auth modules and identify the failure boundary.",
        "Return structured evidence to captain.",
        "explorer",
        "2026-04-23T00:00:00.000Z",
    );

    assert_eq!(
        payload["delegation_plan"]["preferred_execution_mode"],
        "codex_subagent"
    );
    assert_eq!(
        payload["delegation_plan"]["preferred_custom_agent_name"],
        "ccc_scout"
    );
    assert_eq!(
        payload["delegation_plan"]["execution_contract"]["role_identity"]["custom_agent_name"],
        "ccc_scout"
    );
    assert_eq!(
        payload["delegation_plan"]["runtime_dispatch"]["execution_contract"]["advisory_only"],
        true
    );
    assert_eq!(
        payload["delegation_plan"]["fan_in_contract"]["mode"],
        "structured_summary"
    );
    assert_eq!(
        payload["delegation_plan"]["lane_artifact_contract"]["log"]["field"],
        "lifecycle"
    );
    assert_eq!(
        payload["delegation_plan"]["verify_retry_recap_report_contract"]["recap"]["field"],
        "lane_artifact_contract.recap"
    );
    assert_eq!(
        payload["delegation_plan"]["spec_surfaces"]["workflow_owned"]["owned_by"],
        "delegation_plan"
    );
    assert_eq!(
        payload["delegation_plan"]["spec_surfaces"]["sandbox_owned"]["owned_by"],
        "sandbox_policy_helpers"
    );
    assert_eq!(
        payload["delegation_plan"]["subagent_update_contract"]["transport"],
        "ccc_cli_subcommand"
    );
    assert_eq!(
        payload["delegation_plan"]["fallback_gate"]["must_attempt_preferred_subagent_first"],
        true
    );
}

#[test]
fn ccc_status_projects_compact_lifecycle_sync_onto_longway_rows() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("status-row-lifecycle-sync");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-row-lifecycle-sync");
    write_json_document(
        &run_directory.join("longway.json"),
        &json!({
            "lifecycle_state": "active",
            "active_phase_name": "execute",
            "active_phase_status": "in_progress",
            "phases": [{
                "phase_name": "execute",
                "title": "Implement compact lifecycle row sync",
                "status": "in_progress",
                "owner_agent": "ccc_raider",
                "task_card_id": "task-1"
            }]
        }),
    )
    .expect("write longway");
    let task_card_file = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_file).expect("read task card");
    task_card["subagent_lifecycle"] = json!({
        "status": "running",
        "child_agent_id": "ccc_raider",
        "summary": "Implementation is in progress.",
        "updated_at": "2026-04-22T08:02:00.000Z"
    });
    task_card["parallel_fanout"] = json!({
        "lanes": [
            { "lane_id": "raider-a", "lifecycle": { "status": "running" } },
            { "lane_id": "raider-b", "lifecycle": { "status": "completed" } }
        ]
    });
    write_json_document(&task_card_file, &task_card).expect("write task card");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-row-lifecycle-sync",
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    let row = &status_payload["longway"]["phase_rows"][0];
    assert_eq!(row["lifecycle_sync"]["available"], true);
    assert_eq!(row["lifecycle_sync"]["status"], "running");
    assert_eq!(row["lifecycle_sync"]["active_lane_count"], 1);
    assert_eq!(row["lifecycle_sync"]["terminal_lane_count"], 1);
    assert_eq!(
        row["lifecycle_sync"]["lane_statuses"]["raider-b"],
        "completed"
    );
    assert_eq!(row["lifecycle_sync"]["details"][0]["label"], "raider-a");
    assert_eq!(row["lifecycle_sync"]["details"][0]["status"], "running");
    assert_eq!(row["lifecycle_sync"]["details"][1]["label"], "raider-b");
    assert_eq!(row["lifecycle_sync"]["details"][1]["status"], "completed");
    let status_text = create_ccc_status_text(&status_payload);
    assert!(status_text.contains(
        "[>] Implement compact lifecycle row sync [Marauder(ccc_raider)] units=raider-a:ccc_raider,raider-b:ccc_raider lifecycle=running"
    ));
    assert!(status_text.contains("- raider-a running Marauder(ccc_raider)"));
    assert!(status_text.contains("- raider-b completed Marauder(ccc_raider)"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_persists_schema_ignores_generated_dirs_and_reuses_unchanged_metadata() {
    let workspace_dir = create_temp_path("code-graph-persist");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    create_dir_all(workspace_dir.join("target")).expect("create target");
    create_dir_all(workspace_dir.join("node_modules/pkg")).expect("create node_modules");
    create_dir_all(workspace_dir.join(".config/generated")).expect("create config");

    write(
        workspace_dir.join("src").join("lib.rs"),
        "use crate::service::Worker;\npub fn run_app() { Worker::new(); helper(); }\nfn helper() {}\n",
    )
    .expect("write lib");
    write(
        workspace_dir.join("src").join("service.rs"),
        "pub struct Worker;\nimpl Worker { pub fn new() -> Self { Worker } }\n",
    )
    .expect("write service");
    write(
        workspace_dir.join("target").join("generated.rs"),
        "pub fn ignored_target() {}\n",
    )
    .expect("write ignored target");
    write(
        workspace_dir.join("node_modules/pkg").join("generated.js"),
        "export function ignoredNodeModule() {}\n",
    )
    .expect("write ignored node_modules");
    write(
        workspace_dir.join(".config/generated").join("ignored.py"),
        "def ignored_config(): pass\n",
    )
    .expect("write ignored config");

    let detected = code_graph::detect_repo_root(&workspace_dir.join("src")).expect("detect root");
    assert_eq!(
        detected,
        fs::canonicalize(&workspace_dir).expect("canonical workspace")
    );

    let store_path = code_graph::default_graph_store_path(&workspace_dir);
    let first_store =
        code_graph::update_code_graph_store_at(&workspace_dir, &store_path).expect("index graph");
    assert_eq!(
        first_store.schema_version,
        code_graph::CODE_GRAPH_SCHEMA_VERSION
    );
    assert!(first_store.files.contains_key("src/lib.rs"));
    assert!(first_store.files.contains_key("src/service.rs"));
    assert!(!first_store.files.contains_key("target/generated.rs"));
    assert!(!first_store
        .files
        .contains_key("node_modules/pkg/generated.js"));
    assert!(!first_store
        .files
        .contains_key(".config/generated/ignored.py"));

    let persisted = read_json_document(&store_path).expect("read persisted graph");
    assert_eq!(
        persisted["schema_version"],
        json!(code_graph::CODE_GRAPH_SCHEMA_VERSION)
    );

    let mut edited_store = first_store.clone();
    edited_store
        .files
        .get_mut("src/service.rs")
        .expect("service file")
        .symbols
        .push(code_graph::CodeGraphSymbol {
            name: "PersistedWithoutReparse".to_string(),
            kind: "function".to_string(),
            line: 99,
        });
    code_graph::write_code_graph_store(&store_path, &edited_store).expect("write edited graph");

    let second_store =
        code_graph::update_code_graph_store_at(&workspace_dir, &store_path).expect("reindex graph");
    assert!(second_store
        .files
        .get("src/service.rs")
        .expect("service file")
        .symbols
        .iter()
        .any(|symbol| symbol.name == "PersistedWithoutReparse"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_queries_summary_impact_review_context_and_python_symbols() {
    let workspace_dir = create_temp_path("code-graph-query");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    create_dir_all(workspace_dir.join("tests")).expect("create tests");
    create_dir_all(workspace_dir.join("tools")).expect("create tools");

    write(
        workspace_dir.join("src").join("service.ts"),
        "export class Service {}\nexport function loadUser() { return new Service(); }\n",
    )
    .expect("write service ts");
    write(
        workspace_dir.join("src").join("controller.ts"),
        "import { loadUser, Service } from './service';\nexport function handleRequest() {\n  const service = new Service();\n  return loadUser();\n}\n",
    )
    .expect("write controller ts");
    write(
        workspace_dir.join("tests").join("service.test.ts"),
        "import { loadUser } from '../src/service';\ntest('loadUser', () => { loadUser(); });\n",
    )
    .expect("write service test");
    write(
        workspace_dir.join("tools").join("review.py"),
        "from package.module import Thing\nclass Review:\n    pass\ndef score():\n    return Thing\n",
    )
    .expect("write python");

    let store =
        code_graph::update_code_graph_store_for_repo(&workspace_dir).expect("index graph store");
    let query = code_graph::CodeGraphQuery::new(&store);

    let service_summary = query
        .file_summary(Path::new("src/service.ts"))
        .expect("service summary");
    assert_eq!(service_summary.language, "typescript");
    assert!(service_summary
        .symbols
        .contains(&"class:Service".to_string()));
    assert!(service_summary
        .symbols
        .contains(&"function:loadUser".to_string()));

    let python_summary = query
        .file_summary(Path::new("tools/review.py"))
        .expect("python summary");
    assert_eq!(python_summary.imports, vec!["package.module".to_string()]);
    assert!(python_summary.symbols.contains(&"class:Review".to_string()));
    assert!(python_summary
        .symbols
        .contains(&"function:score".to_string()));

    let callers = query.callers_for_file(Path::new("src/service.ts"));
    assert!(callers
        .iter()
        .any(|relation| relation.path == "src/controller.ts"
            && relation
                .reasons
                .iter()
                .any(|reason| reason.contains("imports src/service.ts"))));
    assert!(callers
        .iter()
        .any(|relation| relation.path == "tests/service.test.ts"));

    let callees = query.callees_for_file(Path::new("src/controller.ts"));
    assert!(callees
        .iter()
        .any(|relation| relation.path == "src/service.ts"));

    let related_tests = query.related_tests_for_file(Path::new("src/service.ts"));
    assert!(related_tests
        .iter()
        .any(|relation| relation.path == "tests/service.test.ts"));

    let changed = vec![PathBuf::from("src/service.ts")];
    let blast_radius = query.blast_radius_for_changed_paths(&changed);
    assert_eq!(
        blast_radius.changed_files,
        vec!["src/service.ts".to_string()]
    );
    assert!(blast_radius
        .impacted_files
        .iter()
        .any(|relation| relation.path == "src/controller.ts"));
    assert!(blast_radius
        .related_tests
        .iter()
        .any(|relation| relation.path == "tests/service.test.ts"));
    assert!(blast_radius.risk_score >= 25);
    assert!(["low", "medium", "high"].contains(&blast_radius.risk_level.as_str()));

    let review_context = query.minimal_review_context(&changed);
    assert!(review_context
        .summaries
        .iter()
        .any(|summary| summary.path == "src/service.ts"));
    assert!(review_context
        .callers
        .iter()
        .any(|relation| relation.path == "src/controller.ts"));
    assert!(review_context
        .callees
        .iter()
        .all(|relation| relation.path != "src/service.ts"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_second_wave_queries_cover_flow_criticality_architecture_and_search() {
    let workspace_dir = create_temp_path("code-graph-second-wave");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src/api")).expect("create api");
    create_dir_all(workspace_dir.join("src/domain")).expect("create domain");
    create_dir_all(workspace_dir.join("tests")).expect("create tests");
    write(
        workspace_dir.join("src/domain").join("service.ts"),
        "export class PaymentService {}\nexport function chargeUser() { return new PaymentService(); }\n",
    )
    .expect("write service");
    write(
        workspace_dir.join("src/api").join("handler.ts"),
        "import { chargeUser, PaymentService } from '../domain/service';\nexport function handlePayment() { const svc = new PaymentService(); return chargeUser(); }\n",
    )
    .expect("write handler");
    write(
        workspace_dir.join("tests").join("service.test.ts"),
        "import { chargeUser } from '../src/domain/service';\ntest('chargeUser', () => chargeUser());\n",
    )
    .expect("write test");

    let flow_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "flow_trace",
        "paths": ["src/domain/service.ts"],
        "direction": "both",
        "max_depth": 2,
        "update": true
    }))
    .expect("flow payload");
    assert!(flow_payload["query_result"]["flow_trace"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["from"] == "src/api/handler.ts" && edge["to"] == "src/domain/service.ts"));

    let criticality_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "criticality",
        "paths": ["src/domain/service.ts"],
        "update": false
    }))
    .expect("criticality payload");
    assert_eq!(
        criticality_payload["query_result"]["criticality_scores"][0]["path"],
        "src/domain/service.ts"
    );
    assert!(
        criticality_payload["query_result"]["criticality_scores"][0]["score"]
            .as_u64()
            .unwrap()
            >= 30
    );

    let architecture_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "architecture_overview",
        "update": false
    }))
    .expect("architecture payload");
    assert!(
        architecture_payload["query_result"]["architecture_overview"]["communities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|community| community["id"] == "src/domain")
    );

    let search_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "full_text_search",
        "text": "chargeUser",
        "limit": 10,
        "update": false
    }))
    .expect("search payload");
    assert!(search_payload["query_result"]["search"]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"] == "src/domain/service.ts"));

    let second_repo = create_temp_path("code-graph-second-wave-repo");
    create_dir_all(second_repo.join(".git")).expect("create second git marker");
    create_dir_all(second_repo.join("src")).expect("create second src");
    write(
        second_repo.join("src").join("billing.ts"),
        "export function chargeUserAgain() { return 'chargeUser'; }\n",
    )
    .expect("write second repo");
    let multi_repo_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "multi_repo_search",
        "text": "chargeUser",
        "repos": [
            { "cwd": workspace_dir.to_string_lossy(), "update": false },
            { "cwd": second_repo.to_string_lossy(), "update": true }
        ]
    }))
    .expect("multi repo payload");
    assert_eq!(
        multi_repo_payload["query_result"]["multi_repo_search"]["repos"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        multi_repo_payload["query_result"]["multi_repo_search"]["repos"][1]["search"]["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "src/billing.ts")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
    let _ = fs::remove_dir_all(&second_repo);
}

#[test]
fn code_graph_omitted_update_reads_existing_store_without_rewriting() {
    let workspace_dir = create_temp_path("code-graph-read-only-default");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("existing.rs"),
        "pub fn existing_entry() {}\n",
    )
    .expect("write existing file");

    let initial_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "architecture_overview",
        "update": true
    }))
    .expect("initial graph update");
    assert_eq!(initial_payload["updated"], true);
    assert_eq!(initial_payload["file_count"], 1);

    let store_path = code_graph::default_graph_store_path(&workspace_dir);
    let before_omitted_update = fs::read(&store_path).expect("read graph store before query");
    write(
        workspace_dir.join("src").join("added.rs"),
        "pub fn added_entry() {}\n",
    )
    .expect("write added file");

    let read_only_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "architecture_overview"
    }))
    .expect("read-only graph query");
    assert_eq!(read_only_payload["updated"], false);
    assert_eq!(read_only_payload["file_count"], 1);
    assert_eq!(
        fs::read(&store_path).expect("read graph store after query"),
        before_omitted_update
    );

    let refreshed_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "file_summary",
        "paths": ["src/added.rs"],
        "update": true
    }))
    .expect("explicit graph update");
    assert_eq!(refreshed_payload["updated"], true);
    assert_eq!(refreshed_payload["file_count"], 2);
    assert_ne!(
        fs::read(&store_path).expect("read graph store after explicit update"),
        before_omitted_update
    );
    assert_eq!(
        refreshed_payload["query_result"]["file_summaries"][0]["path"],
        "src/added.rs"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_optionally_mirrors_to_tolaria_and_loads_missing_local_store() {
    let workspace_dir = create_temp_path("code-graph-tolaria-mirror");
    let vault_dir = create_temp_path("code-graph-tolaria-vault");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    create_dir_all(&vault_dir).expect("create vault");
    write(vault_dir.join("AGENTS.md"), "# Tolaria Vault\n").expect("write vault marker");
    write(
        workspace_dir.join("src").join("mirror.rs"),
        "pub fn mirrored_entry() {}\n",
    )
    .expect("write mirror source");

    let update_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "file_summary",
        "paths": ["src/mirror.rs"],
        "update": true,
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("update graph with Tolaria mirror");
    assert_eq!(update_payload["tolaria"]["state"], "synced");
    let note_path = PathBuf::from(
        update_payload["tolaria"]["note_path"]
            .as_str()
            .expect("Tolaria note path"),
    );
    assert!(note_path.exists());
    assert!(note_path.to_string_lossy().contains("/ccc/repos/"));
    assert!(note_path.to_string_lossy().ends_with("/graph.md"));
    assert_eq!(
        update_payload["tolaria"]["relative_note_path"]
            .as_str()
            .unwrap_or_default()
            .split('/')
            .next_back(),
        Some("graph.md")
    );
    let note = fs::read_to_string(&note_path).expect("read Tolaria note");
    assert!(note.contains("CCC Code Graph"));
    assert!(note.contains("src/mirror.rs"));
    assert!(note.contains("\"files\""));

    let store_path = code_graph::default_graph_store_path(&workspace_dir);
    fs::remove_file(&store_path).expect("remove local graph store");
    let loaded_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "file_summary",
        "paths": ["src/mirror.rs"],
        "update": false,
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("load graph from Tolaria mirror");
    assert_eq!(loaded_payload["tolaria"]["state"], "loaded_from_tolaria");
    assert_eq!(
        loaded_payload["query_result"]["file_summaries"][0]["symbols"][0],
        "function:mirrored_entry"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
    let _ = fs::remove_dir_all(&vault_dir);
}

#[test]
fn code_graph_multi_repo_search_omitted_update_reads_existing_store() {
    let workspace_dir = create_temp_path("code-graph-multi-repo-read-only-default");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("existing.rs"),
        "pub fn existing_multi_repo_entry() {}\n",
    )
    .expect("write existing file");
    code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "architecture_overview",
        "update": true
    }))
    .expect("initial graph update");

    let store_path = code_graph::default_graph_store_path(&workspace_dir);
    let before_omitted_update = fs::read(&store_path).expect("read graph store before search");
    write(
        workspace_dir.join("src").join("added.rs"),
        "pub fn multi_repo_added_entry() { let needle = \"fresh-multi-repo-term\"; }\n",
    )
    .expect("write added file");

    let read_only_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "multi_repo_search",
        "text": "fresh-multi-repo-term",
        "repos": [
            { "cwd": workspace_dir.to_string_lossy() }
        ]
    }))
    .expect("read-only multi repo search");
    assert!(
        read_only_payload["query_result"]["multi_repo_search"]["repos"][0]["search"]["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::read(&store_path).expect("read graph store after search"),
        before_omitted_update
    );

    let refreshed_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "query": "multi_repo_search",
        "text": "fresh-multi-repo-term",
        "repos": [
            { "cwd": workspace_dir.to_string_lossy(), "update": true }
        ]
    }))
    .expect("explicit multi repo refresh");
    assert!(
        refreshed_payload["query_result"]["multi_repo_search"]["repos"][0]["search"]["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "src/added.rs")
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_cli_input_uses_payload_and_text_helpers() {
    let workspace_dir = create_temp_path("code-graph-cli");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("lib.rs"),
        "pub fn cli_graph_entry() {}\n",
    )
    .expect("write lib");

    let parsed = parse_cli_command_input(
        "graph",
        &[
            "--text".to_string(),
            "--json".to_string(),
            json!({
                "cwd": workspace_dir.to_string_lossy(),
                "query": "file_summary",
                "paths": ["src/lib.rs"],
                "update": true
            })
            .to_string(),
        ],
        false,
    )
    .expect("parse graph cli");
    assert_eq!(parsed.output_mode, CliOutputMode::Text);

    let payload = code_graph::create_code_graph_payload(&parsed.payload).expect("graph payload");
    assert_eq!(payload["query"], "file_summary");
    assert_eq!(
        payload["query_result"]["file_summaries"][0]["path"],
        "src/lib.rs"
    );
    let graph_text = code_graph::create_code_graph_text(&payload);
    assert!(graph_text.contains("Graph: Way referenced query=file_summary paths=src/lib.rs"));
    assert!(graph_text.contains("found 1 indexed files; graph-informed planning next step."));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_mcp_tool_returns_text_and_structured_content() {
    let config_home = create_temp_path("code-graph-mcp-config");
    let config_path = config_home.join("ccc").join("ccc-config.toml");
    write_graph_context_routing_config(&config_path, false, None, None);
    let mut session_context = create_session_context();
    session_context.shared_config_path = config_path.to_string_lossy().into_owned();
    let workspace_dir = create_temp_path("code-graph-mcp");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("service.ts"),
        "export function loadUser() { return 1; }\n",
    )
    .expect("write service");

    let response = handle_message(
        &session_context,
        json!({
            "jsonrpc": "2.0",
            "id": 88,
            "method": "tools/call",
            "params": {
                "name": "ccc_code_graph",
                "arguments": {
                    "cwd": workspace_dir.to_string_lossy(),
                    "query": "file_summary",
                    "paths": ["src/service.ts"],
                    "update": true
                }
            }
        }),
    )
    .expect("response");

    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("Graph: Way referenced query=file_summary paths=src/service.ts"));
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("found 1 indexed files; graph-informed planning next step."));
    assert_eq!(
        response["result"]["structuredContent"]["code_graph"]["query_result"]["file_summaries"][0]
            ["symbols"][0],
        "function:loadUser"
    );

    let tools = crate::mcp_tools::create_tools_list_response(Some(json!(1)));
    assert!(tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "ccc_code_graph"));

    let _ = fs::remove_dir_all(&workspace_dir);
    let _ = fs::remove_dir_all(&config_home);
}

#[test]
fn code_graph_status_payload_text_and_compact_show_quiet_summary_when_store_exists() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("code-graph-status");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(workspace_dir.join("src")).expect("create src");
    write(
        workspace_dir.join("src").join("main.rs"),
        "fn main() { helper(); }\nfn helper() {}\n",
    )
    .expect("write main");
    code_graph::update_code_graph_store_for_repo(&workspace_dir).expect("index graph");
    write_test_run_fixture(&workspace_dir, "run-code-graph-status");

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-code-graph-status",
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");

    assert_eq!(status_payload["code_graph"]["available"], true);
    assert_eq!(status_payload["code_graph"]["file_count"], 1);
    assert!(status_payload["code_graph"]["evidence_note"]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("availability=store_loaded"));
    assert!(create_ccc_status_text(&status_payload).contains(
        "Graph: Way referenced repo graph; found 1 indexed files in src:1; graph-informed planning next step."
    ));

    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(compact["code_graph"]["available"], true);
    assert_eq!(
        compact["command_templates"]["graph"]["payload"]["query"],
        "review_context"
    );
    assert_eq!(
        compact["command_templates"]["graph"]["payload"]["update"],
        false
    );
    assert!(compact["command_templates"]["graph"]["command"]
        .as_str()
        .unwrap_or_default()
        .contains("\"update\":false"));
    assert!(!compact["command_templates"]["graph"]["command"]
        .as_str()
        .unwrap_or_default()
        .contains("\"update\":true"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn code_graph_parent_cwd_uses_single_child_graph_store_and_rebases_paths() {
    let parent_dir = create_temp_path("code-graph-parent-cwd");
    let repo_dir = parent_dir.join("Codex-Cli-Captain");
    create_dir_all(&parent_dir).expect("create parent");
    write(parent_dir.join("package.json"), "{}\n").expect("write parent marker");
    create_dir_all(repo_dir.join(".git")).expect("create git marker");
    create_dir_all(repo_dir.join("rust/ccc-mcp/src")).expect("create nested src");
    write(repo_dir.join("rust/ccc-mcp/Cargo.toml"), "[package]\n").expect("write nested marker");
    write(
        repo_dir.join("rust/ccc-mcp/src").join("lib.rs"),
        "pub fn parent_cwd_graph_entry() {}\n",
    )
    .expect("write lib");
    code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index child graph");

    let payload = code_graph::create_code_graph_payload(&json!({
        "cwd": parent_dir.to_string_lossy(),
        "query": "file_summary",
        "paths": ["Codex-Cli-Captain/rust/ccc-mcp/src/lib.rs"],
        "update": false
    }))
    .expect("graph payload");
    assert!(payload["repo_root"]
        .as_str()
        .unwrap_or_default()
        .ends_with("Codex-Cli-Captain"));
    assert_eq!(
        payload["query_result"]["file_summaries"][0]["path"],
        "rust/ccc-mcp/src/lib.rs"
    );

    let status = code_graph::create_code_graph_status_payload(&parent_dir);
    assert_eq!(status["available"], true);
    assert!(status["repo_root"]
        .as_str()
        .unwrap_or_default()
        .ends_with("Codex-Cli-Captain"));
    assert_eq!(status["resolution"], "single_child_graph_store");

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn code_graph_parent_cwd_reports_actionable_errors_for_missing_or_ambiguous_child_store() {
    let empty_parent = create_temp_path("code-graph-parent-empty");
    create_dir_all(&empty_parent).expect("create empty parent");
    let missing_error = code_graph::create_code_graph_payload(&json!({
        "cwd": empty_parent.to_string_lossy(),
        "query": "file_summary",
        "update": false
    }))
    .expect_err("missing graph target");
    let missing_message = missing_error.to_string();
    assert!(missing_message.contains("cwd must be a git repo when no code graph store exists"));
    assert!(missing_message.contains("Pass cwd as the target repo path"));

    let parent_dir = create_temp_path("code-graph-parent-ambiguous");
    for name in ["repo-one", "repo-two"] {
        let repo_dir = parent_dir.join(name);
        create_dir_all(repo_dir.join(".git")).expect("create git marker");
        create_dir_all(repo_dir.join("src")).expect("create src");
        write(
            repo_dir.join("src").join("lib.rs"),
            format!("pub fn {}() {{}}\n", name.replace('-', "_")),
        )
        .expect("write lib");
        code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index child graph");
    }

    let ambiguous_error = code_graph::create_code_graph_payload(&json!({
        "cwd": parent_dir.to_string_lossy(),
        "query": "architecture_overview",
        "update": false
    }))
    .expect_err("ambiguous graph target");
    let ambiguous_message = ambiguous_error.to_string();
    assert!(ambiguous_message.contains("multiple child code graph stores"));
    assert!(ambiguous_message.contains("Pass cwd as the target repo path"));

    let status = code_graph::create_code_graph_status_payload(&parent_dir);
    assert_eq!(status["available"], false);
    assert_eq!(status["diagnostic_severity"], "warning");
    assert_eq!(status["blocking"], false);
    assert!(status["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("multiple child code graph stores"));

    let _ = fs::remove_dir_all(&empty_parent);
    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn code_graph_indexes_non_git_document_root_markdown_files() {
    let document_root = create_temp_path("code-graph-document-root");
    create_dir_all(document_root.join("docs")).expect("create docs");
    write(
        document_root.join("docs").join("guide.md"),
        "# Release Guide\nSee [Install](install.md) for setup.\n",
    )
    .expect("write guide");
    write(
        document_root.join("docs").join("install.md"),
        "# Install\nRun the installer.\n",
    )
    .expect("write install");

    let payload = code_graph::create_code_graph_payload(&json!({
        "cwd": document_root.to_string_lossy(),
        "query": "file_summary",
        "paths": ["docs/guide.md"],
        "update": true
    }))
    .expect("document graph payload");
    assert_eq!(payload["file_count"], 2);
    assert!(payload["repo_root"]
        .as_str()
        .unwrap_or_default()
        .contains("code-graph-document-root"));
    assert_eq!(
        payload["query_result"]["file_summaries"][0]["language"],
        "markdown"
    );
    assert!(payload["query_result"]["file_summaries"][0]["symbols"]
        .as_array()
        .unwrap()
        .contains(&json!("heading:Release_Guide")));
    assert!(payload["query_result"]["file_summaries"][0]["imports"]
        .as_array()
        .unwrap()
        .contains(&json!("install.md")));

    let status = code_graph::create_code_graph_status_payload(&document_root);
    assert_eq!(status["available"], true);
    assert_eq!(status["resolution"], "document_graph_store");

    let _ = fs::remove_dir_all(&document_root);
}

#[test]
fn code_graph_mirrors_document_root_graph_to_tolaria_repo_namespace() {
    let document_root = create_temp_path("code-graph-document-tolaria-root");
    let vault_dir = create_temp_path("code-graph-document-tolaria-vault");
    create_dir_all(document_root.join("docs")).expect("create docs");
    create_dir_all(&vault_dir).expect("create vault");
    write(vault_dir.join("AGENTS.md"), "# Tolaria Vault\n").expect("write vault marker");
    write(
        document_root.join("docs").join("guide.md"),
        "# Guide\nSee [Install](install.md).\n",
    )
    .expect("write guide");
    write(document_root.join("docs").join("install.md"), "# Install\n").expect("write install");

    let update_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": document_root.to_string_lossy(),
        "query": "file_summary",
        "paths": ["docs/guide.md"],
        "update": true,
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("update document graph with Tolaria mirror");

    assert_eq!(update_payload["tolaria"]["state"], "synced");
    let note_path = PathBuf::from(
        update_payload["tolaria"]["note_path"]
            .as_str()
            .expect("Tolaria document graph note path"),
    );
    assert!(note_path.to_string_lossy().contains("/ccc/repos/"));
    assert!(note_path.to_string_lossy().ends_with("/graph.md"));
    assert_eq!(
        update_payload["tolaria"]["repo_folder"],
        note_path
            .parent()
            .expect("Tolaria repo folder")
            .to_string_lossy()
            .to_string()
    );

    let store_path = code_graph::default_graph_store_path(&document_root);
    fs::remove_file(&store_path).expect("remove local document graph store");
    let loaded_payload = code_graph::create_code_graph_payload(&json!({
        "cwd": document_root.to_string_lossy(),
        "query": "file_summary",
        "paths": ["docs/guide.md"],
        "update": false,
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("load document graph from Tolaria mirror");
    assert_eq!(loaded_payload["tolaria"]["state"], "loaded_from_tolaria");
    assert_eq!(
        loaded_payload["query_result"]["file_summaries"][0]["language"],
        "markdown"
    );

    let _ = fs::remove_dir_all(&document_root);
    let _ = fs::remove_dir_all(&vault_dir);
}

#[test]
fn code_graph_parent_cwd_uses_query_paths_to_disambiguate_child_repos() {
    let parent_dir = create_temp_path("code-graph-parent-disambiguated");
    for name in ["repo-one", "repo-two"] {
        let repo_dir = parent_dir.join(name);
        create_dir_all(repo_dir.join(".git")).expect("create git marker");
        create_dir_all(repo_dir.join("src")).expect("create src");
        write(
            repo_dir.join("src").join("lib.rs"),
            format!("pub fn {}() {{}}\n", name.replace('-', "_")),
        )
        .expect("write lib");
        code_graph::update_code_graph_store_for_repo(&repo_dir).expect("index child graph");
    }

    let payload = code_graph::create_code_graph_payload(&json!({
        "cwd": parent_dir.to_string_lossy(),
        "query": "file_summary",
        "paths": ["repo-one/src/lib.rs"],
        "update": false
    }))
    .expect("disambiguated graph payload");
    assert!(payload["repo_root"]
        .as_str()
        .unwrap_or_default()
        .ends_with("repo-one"));
    assert_eq!(
        payload["query_result"]["file_summaries"][0]["path"],
        "src/lib.rs"
    );

    let _ = fs::remove_dir_all(&parent_dir);
}

#[test]
fn ccc_memory_previews_filters_and_requires_stale_write_guard() {
    let workspace_dir = create_temp_path("memory-preview-write");
    create_dir_all(&workspace_dir).expect("create workspace");

    let preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "preview",
        "entries": [
            {
                "kind": "user_preference",
                "text": "Prefer focused Rust tests for narrow CLI behavior.",
                "source_kind": "operator_confirmation"
            },
            {
                "kind": "verified_project_fact",
                "text": "LongWay run state says this happened.",
                "source_kind": "run_state",
                "evidence_paths": ["run-state.json"]
            },
            {
                "kind": "verified_project_fact",
                "text": "A project fact without evidence.",
                "source_kind": "project_file"
            }
        ]
    }))
    .expect("preview memory");

    assert_eq!(preview["written"], false);
    assert_eq!(preview["accepted_entries"].as_array().unwrap().len(), 1);
    assert_eq!(preview["rejected_entries"].as_array().unwrap().len(), 2);
    assert!(!memory::default_memory_store_path(&workspace_dir).exists());
    assert_eq!(
        preview["next_write"]["expected_updated_at_unix_ms"],
        Value::Null
    );
    assert!(preview["next_write"]["preview_token"]
        .as_str()
        .unwrap()
        .starts_with("ccc-memory-preview-v1:"));
    let preview_text = memory::create_memory_text(&preview);
    assert!(preview_text.contains("Memory: preview written=false accepted=1 rejected=2"));
    assert!(preview_text.contains("diff=before=null after={"));
    assert!(preview_text.contains("preview_token=ccc-memory-preview-v1:"));
    assert!(preview_text.contains("accepted_entries=[user_preference: Prefer focused Rust tests"));
    assert!(preview_text.contains("rejected_entries=[verified_project_fact: LongWay run state"));
    assert!(preview_text.contains("reason=run_state is not an allowed memory truth source"));

    let missing_ack = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "expected_updated_at_unix_ms": null,
        "entries": [{
            "kind": "repeated_rule",
            "text": "Never treat latest work result as memory truth."
        }]
    }));
    assert!(missing_ack
        .unwrap_err()
        .to_string()
        .contains("preview_ack=true"));

    let write_entries = json!([{
        "kind": "verified_project_fact",
        "text": "CCC memory stores workspace data in .ccc/memory.json.",
        "source_kind": "project_file",
        "evidence_paths": ["rust/ccc-mcp/src/memory.rs"]
    }]);
    let direct_ack_only = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "preview_ack": true,
        "expected_updated_at_unix_ms": null,
        "entries": write_entries.clone()
    }));
    assert!(direct_ack_only
        .unwrap_err()
        .to_string()
        .contains("preview_token"));

    let mismatched_token = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "preview_ack": true,
        "preview_token": preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": null,
        "entries": write_entries.clone()
    }));
    assert!(mismatched_token
        .unwrap_err()
        .to_string()
        .contains("preview_token"));

    let write_preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "preview",
        "entries": write_entries.clone()
    }))
    .expect("preview write memory");
    let written = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "preview_ack": true,
        "preview_token": write_preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": null,
        "entries": write_entries
    }))
    .expect("write memory");
    assert_eq!(written["written"], true);
    assert_eq!(written["memory"]["enabled"], true);
    assert_eq!(written["memory"]["entry_count"], 1);

    let stale_write = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "preview_ack": true,
        "preview_token": write_preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": null,
        "entries": [{
            "kind": "repeated_rule",
            "text": "This stale write must not land."
        }]
    }));
    assert!(stale_write
        .unwrap_err()
        .to_string()
        .contains("stale memory write"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_memory_normalizes_source_kind_allowlist_and_rejects_unsafe_variants() {
    let workspace_dir = create_temp_path("memory-source-kind-allowlist");
    create_dir_all(&workspace_dir).expect("create workspace");

    let preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "preview",
        "entries": [
            {
                "kind": "verified_project_fact",
                "text": "Project files are acceptable memory evidence.",
                "source_kind": "Project File",
                "evidence_paths": ["Cargo.toml"]
            },
            {
                "kind": "repeated_rule",
                "text": "Test results are acceptable repeated-rule evidence.",
                "source_kind": "test-result",
                "generally_useful": true,
                "evidence_paths": ["test-a.log", "test-b.log"]
            },
            {
                "kind": "repeated_rule",
                "text": "A single observation should not become durable guidance.",
                "source_kind": "test_result",
                "evidence_paths": ["one-test.log"]
            },
            {
                "kind": "verified_project_fact",
                "text": "LongWay is not a durable truth source.",
                "source_kind": "LONG WAY",
                "evidence_paths": ["longway.json"]
            },
            {
                "kind": "verified_project_fact",
                "text": "Latest worker output is not a durable truth source.",
                "source_kind": "latest work result",
                "evidence_paths": ["worker.json"]
            },
            {
                "kind": "repeated_rule",
                "text": "Inference-only observations are not durable truth.",
                "source_kind": "inference_only"
            }
        ]
    }))
    .expect("preview source kinds");

    let accepted = preview["accepted_entries"].as_array().unwrap();
    assert_eq!(accepted.len(), 2);
    assert_eq!(accepted[0]["source_kind"], "project_file");
    assert_eq!(accepted[1]["source_kind"], "test_result");
    assert_eq!(accepted[1]["certainty"], "verified_repeated");
    assert_eq!(preview["rejected_entries"].as_array().unwrap().len(), 4);

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_memory_rejects_public_store_path_override() {
    let workspace_dir = create_temp_path("memory-store-path-public");
    create_dir_all(&workspace_dir).expect("create workspace");

    let rejected = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "store_path": workspace_dir.join("elsewhere.json").to_string_lossy(),
        "action": "status"
    }));
    assert!(rejected.unwrap_err().to_string().contains("store_path"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_memory_accepts_captain_instruction_entries_and_status_counts() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("memory-captain-instruction");
    create_dir_all(&workspace_dir).expect("create workspace");
    write_test_run_fixture(&workspace_dir, "run-memory-captain-instruction");

    let entries = json!([
        {
            "kind": "captain_instruction",
            "text": "When a request is broad or risky, ask concise clarification questions before mutation.",
            "source_kind": "operator_confirmation",
            "source": "operator"
        }
    ]);
    let preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "preview",
        "entries": entries.clone()
    }))
    .expect("preview captain instruction");
    assert_eq!(
        preview["accepted_entries"][0]["kind"],
        "captain_instruction"
    );
    assert_eq!(
        preview["accepted_entries"][0]["source_kind"],
        "operator_confirmation"
    );

    let written = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "preview_ack": true,
        "preview_token": preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": null,
        "entries": entries
    }))
    .expect("write captain instruction");

    assert_eq!(written["memory"]["entry_count"], 1);
    assert_eq!(written["memory"]["entry_counts"]["captain_instruction"], 1);
    assert_eq!(
        written["memory"]["entry_source_counts"]["operator_confirmation"],
        1
    );
    assert_eq!(written["memory"]["captain_instruction_count"], 1);
    assert_eq!(written["memory"]["captain_instruction_status"], "active");
    assert_eq!(
        written["memory"]["captain_instruction_source"],
        "ccc_memory"
    );
    assert_eq!(
        written["memory"]["captain_instruction_source_counts"]["operator_confirmation"],
        1
    );
    assert_eq!(
        written["memory"]["captain_instruction_source_summary"],
        "operator_confirmation:1"
    );
    assert!(memory::create_memory_text(&written)
        .contains("captain_instructions=1 captain_instruction_status=active captain_instruction_source=operator_confirmation:1"));

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-memory-captain-instruction",
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(status_payload["memory"]["captain_instruction_count"], 1);
    assert_eq!(
        status_payload["memory"]["captain_instruction_status"],
        "active"
    );
    assert_eq!(
        status_payload["memory"]["captain_instruction_source_summary"],
        "operator_confirmation:1"
    );
    assert!(create_ccc_status_text(&status_payload).contains(
        "Memory: enabled entries=1 captain_instructions=1 captain_instruction_status=active captain_instruction_source=operator_confirmation:1"
    ));
    assert!(create_ccc_status_text(&status_payload).contains(".ccc/memory.json"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_memory_optionally_mirrors_repo_memory_to_tolaria() {
    let workspace_dir = create_temp_path("memory-tolaria-mirror");
    let vault_dir = create_temp_path("memory-tolaria-vault");
    create_dir_all(workspace_dir.join(".git")).expect("create git marker");
    create_dir_all(&vault_dir).expect("create vault");
    write(vault_dir.join("AGENTS.md"), "# Tolaria Vault\n").expect("write vault marker");

    let entry = json!({
        "kind": "user_preference",
        "text": "Prefer repo-scoped CCC memory mirrors in Tolaria when enabled.",
        "source_kind": "operator_confirmation"
    });
    let preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "preview",
        "entries": [entry.clone()],
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("preview Tolaria memory");
    let written = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "entries": [entry],
        "preview_ack": true,
        "preview_token": preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": preview["next_write"]["expected_updated_at_unix_ms"].clone(),
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("write Tolaria memory");

    assert_eq!(written["tolaria"]["state"], "synced");
    assert_eq!(written["memory"]["tolaria"]["enabled"], true);
    let note_path = PathBuf::from(
        written["tolaria"]["note_path"]
            .as_str()
            .expect("Tolaria memory note path"),
    );
    assert!(note_path.exists());
    assert!(note_path.to_string_lossy().contains("/ccc/repos/"));
    let note = fs::read_to_string(&note_path).expect("read Tolaria memory note");
    assert!(note.contains("CCC Memory"));
    assert!(note.contains("Prefer repo-scoped CCC memory mirrors"));
    assert!(note.contains("\"entries\""));

    fs::remove_file(memory::default_memory_store_path(&workspace_dir))
        .expect("remove local memory store");
    let restored = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "status",
        "tolaria_enabled": true,
        "tolaria_vault_path": vault_dir.to_string_lossy()
    }))
    .expect("restore memory status from Tolaria mirror");
    assert_eq!(restored["memory"]["available"], true);
    assert_eq!(restored["memory"]["entry_count"], 1);
    assert_eq!(restored["memory"]["tolaria"]["available"], true);

    let _ = fs::remove_dir_all(&workspace_dir);
    let _ = fs::remove_dir_all(&vault_dir);
}

#[test]
fn ccc_memory_off_status_and_ccc_status_surface_are_opt_in() {
    let session_context = create_session_context();
    let workspace_dir = create_temp_path("memory-status-surface");
    create_dir_all(&workspace_dir).expect("create workspace");
    write_test_run_fixture(&workspace_dir, "run-memory-status");

    let write_entries = json!([
        {
            "kind": "user_preference",
            "text": "Keep CCC memory opt in.",
            "source_kind": "operator_confirmation"
        }
    ]);
    let write_preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "preview",
        "entries": write_entries.clone()
    }))
    .expect("preview memory write");
    let written = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "write",
        "preview_ack": true,
        "preview_token": write_preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": null,
        "entries": write_entries
    }))
    .expect("write memory");
    let expected_updated = written["memory"]["updated_at_unix_ms"].clone();

    let locator = resolve_run_locator_arguments(
        &json!({
            "run_id": "run-memory-status",
            "cwd": workspace_dir.to_string_lossy()
        }),
        "ccc_status",
    )
    .expect("locator");
    let status_payload =
        create_ccc_status_payload(&session_context, &locator).expect("status payload");
    assert_eq!(status_payload["memory"]["enabled"], true);
    assert_eq!(status_payload["memory"]["entry_count"], 1);
    assert!(create_ccc_status_text(&status_payload).contains("Memory: enabled entries=1"));
    let compact = create_ccc_status_compact_payload(&status_payload);
    assert_eq!(compact["memory"]["enabled"], true);

    let off_preview = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "off"
    }))
    .expect("off preview");
    assert_eq!(off_preview["written"], false);
    assert_eq!(off_preview["next_write"]["apply"], true);

    let direct_off_apply = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "off",
        "apply": true,
        "expected_updated_at_unix_ms": expected_updated
    }));
    assert!(direct_off_apply
        .unwrap_err()
        .to_string()
        .contains("preview_token"));

    let off = memory::create_memory_payload(&json!({
        "cwd": workspace_dir.to_string_lossy(),
        "action": "off",
        "apply": true,
        "preview_token": off_preview["next_write"]["preview_token"].clone(),
        "expected_updated_at_unix_ms": expected_updated
    }))
    .expect("turn memory off");
    assert_eq!(off["written"], true);
    assert_eq!(off["memory"]["enabled"], false);
    assert!(memory::create_memory_text(&off).contains("Memory: off entries=1"));

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn ccc_status_locator_accepts_persisted_run_record_payload() {
    let payload = status_locator_payload_from_cli_payload(&json!({
        "run_id": "run-from-record",
        "active_agent_id": "captain",
        "active_task_card_id": "task-1",
        "status": "active"
    }));

    assert_eq!(payload, json!({ "run_id": "run-from-record" }));
}

#[test]
fn ccc_subagent_update_invalid_task_card_error_names_missing_card() {
    let workspace_dir = create_temp_path("subagent-update-missing-task-card");
    create_dir_all(&workspace_dir).expect("create workspace");
    let _run_directory = write_test_run_fixture(&workspace_dir, "run-missing-task-card");

    let parsed = parse_ccc_subagent_update_arguments(&json!({
        "run_id": "run-missing-task-card",
        "cwd": workspace_dir.to_string_lossy(),
        "task_card_id": "missing-task-card",
        "child_agent_id": "ccc_scout",
        "status": "completed",
        "summary": "done"
    }))
    .expect("parse subagent update");
    let error = create_ccc_subagent_update_payload(&parsed).expect_err("missing task card error");
    let message = error.to_string();
    assert!(
        message.contains("task_card_id=missing-task-card"),
        "missing task card id in error: {message}"
    );
    assert!(
        message.contains("task-cards/missing-task-card.json"),
        "missing task card path in error: {message}"
    );

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn completion_review_gate_skips_already_passed_task_card() {
    let workspace_dir = create_temp_path("completion-review-skip-passed");
    create_dir_all(&workspace_dir).expect("create workspace");
    let run_directory = write_test_run_fixture(&workspace_dir, "run-review-skip-passed");
    let task_card_path = run_directory.join("task-cards").join("task-1.json");
    let mut task_card = read_json_document(&task_card_path).expect("task card");
    task_card["verification_state"] = Value::String("passed".to_string());
    write_json_document(&task_card_path, &task_card).expect("write task card");

    let review = maybe_require_arbiter_review_before_completion(
        &run_directory,
        &task_card,
        "2026-05-06T00:00:00.000Z",
    )
    .expect("review gate");
    assert!(review.is_none());

    let _ = fs::remove_dir_all(&workspace_dir);
}

#[test]
fn planned_row_git_mutation_infers_companion_operator_before_docs() {
    let role = planned_row_text_inferred_role(
        "Commit README.md and README.ja.md with a concise documentation message.",
        "",
        "",
    );
    assert_eq!(role, Some("companion_operator"));

    let stage_role = planned_row_text_inferred_role("Stage display cleanup review.", "", "");
    assert_ne!(stage_role, Some("companion_operator"));
}

#[test]
fn routing_trace_does_not_treat_stage_as_git_tag() {
    let route = create_routing_trace_payload("Stage display cleanup", "code specialist");

    assert_ne!(route["selected_role"], "companion_operator");
    assert_ne!(
        route["companion_tool_route"]["owner_role"],
        "companion_operator"
    );
}

#[test]
fn run_locator_global_fallback_resolves_run_id_from_different_cwd() {
    let real_workspace = create_temp_path("global-run-real-workspace");
    let wrong_workspace = create_temp_path("global-run-wrong-workspace");
    let global_root = create_temp_path("global-run-root").join("workspaces");
    create_dir_all(&real_workspace).expect("create real workspace");
    create_dir_all(&wrong_workspace).expect("create wrong workspace");
    create_dir_all(&global_root).expect("create global root");

    let run_id = "run-global-fallback";
    let central_ccc_dir = global_root.join(crate::run_locator::compute_workspace_storage_key(
        &real_workspace,
    ));
    let central_run_directory = central_ccc_dir.join("runs").join(run_id);
    crate::run_locator::ensure_run_paths_for_start(&real_workspace, &central_run_directory)
        .expect("create central run paths");
    write_json_document(
        &central_run_directory.join("run.json"),
        &json!({
            "run_id": run_id,
            "goal": "Resolve globally by run_id",
            "status": "active",
            "stage": "execution"
        }),
    )
    .expect("write run record");

    let locator = crate::run_locator::resolve_run_id_locator_with_global_fallback_in_roots(
        &wrong_workspace,
        run_id,
        &[global_root.clone()],
    )
    .expect("resolve run id globally");

    assert_eq!(locator.run_id, run_id);
    assert_eq!(
        locator.cwd,
        crate::run_locator::normalize_path(&real_workspace)
    );
    assert_eq!(
        locator.run_directory,
        crate::run_locator::normalize_path(&central_run_directory)
    );

    let _ = fs::remove_dir_all(&real_workspace);
    let _ = fs::remove_dir_all(&wrong_workspace);
    let _ = fs::remove_dir_all(global_root.parent().unwrap_or(&global_root));
}
