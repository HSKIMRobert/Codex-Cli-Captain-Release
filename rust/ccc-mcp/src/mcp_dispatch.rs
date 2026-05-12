use crate::activity_view::{create_ccc_activity_payload, create_ccc_activity_text};
use crate::code_graph::{create_code_graph_payload, create_code_graph_text};
use crate::entry_arguments::{
    parse_ccc_auto_entry_arguments, parse_ccc_orchestrate_arguments,
    parse_ccc_recommend_entry_arguments, parse_ccc_run_arguments, parse_ccc_start_arguments,
    parse_ccc_subagent_update_arguments,
};
use crate::entry_policy::{
    create_ccc_auto_entry_payload, create_ccc_auto_entry_text, create_ccc_recommend_entry_payload,
    create_ccc_recommend_entry_text,
};
use crate::graph_context::{
    create_graph_context_code_graph_text,
    create_graph_context_mcp_code_graph_payload_for_config_path,
};
use crate::install_check::{
    create_install_check_payload, create_server_identity_payload, create_server_identity_text,
};
use crate::mcp_tools::{
    create_orchestrate_tool_structured_content, create_orchestrate_tool_text,
    create_run_tool_structured_content, create_run_tool_text, create_start_tool_structured_content,
    create_start_tool_text, create_subagent_update_tool_structured_content,
    create_subagent_update_tool_text, create_tools_list_response, tool_call_arguments, tool_error,
    tool_result, tool_result_with_meta,
};
use crate::run_bootstrap::{create_ccc_run_payload, create_ccc_start_payload};
use crate::run_locator::resolve_run_locator_arguments;
use crate::status_app_panel::{
    create_codex_app_panel_resource_html, create_codex_app_panel_text,
    write_codex_app_panel_artifact, CCC_APP_PANEL_MIME_TYPE, CCC_APP_PANEL_RESOURCE_URI,
};
use crate::{
    create_ccc_orchestrate_payload, create_ccc_status_operator_text, create_ccc_status_payload,
    create_ccc_subagent_update_payload, SessionContext, MCP_PROTOCOL_VERSION, SERVER_NAME,
};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) fn handle_message(session_context: &SessionContext, message: Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str)?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "instructions": format!(
                    "Rust-only CCC runtime for the current {} release.",
                    env!("CARGO_PKG_VERSION")
                )
            }
        })),
        "notifications/initialized" => None,
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        })),
        "tools/list" => Some(create_tools_list_response(id)),
        "tools/call" => handle_tool_call(session_context, &message, id),
        "resources/list" => Some(create_resources_list_response(id)),
        "resources/read" => Some(handle_resource_read(&message, id)),
        _ => Some(tool_error(id, -32601, format!("Unknown method: {method}"))),
    }
}

fn create_resources_list_response(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "resources": [
                {
                    "uri": CCC_APP_PANEL_RESOURCE_URI,
                    "name": "CCC LongWay Panel",
                    "description": "HTML template for rendering the CCC LongWay/status app panel.",
                    "mimeType": CCC_APP_PANEL_MIME_TYPE,
                    "_meta": {
                        "ui": {
                            "prefersBorder": true
                        }
                    }
                }
            ]
        }
    })
}

fn handle_resource_read(message: &Value, id: Option<Value>) -> Value {
    let uri = message
        .get("params")
        .and_then(|params| params.get("uri"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    if uri != CCC_APP_PANEL_RESOURCE_URI {
        return tool_error(id, -32602, format!("Unknown resource: {uri}"));
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "contents": [
                {
                    "uri": CCC_APP_PANEL_RESOURCE_URI,
                    "mimeType": CCC_APP_PANEL_MIME_TYPE,
                    "text": create_codex_app_panel_resource_html(),
                    "_meta": {
                        "ui": {
                            "prefersBorder": true
                        }
                    }
                }
            ]
        }
    })
}

fn handle_tool_call(
    session_context: &SessionContext,
    message: &Value,
    id: Option<Value>,
) -> Option<Value> {
    let tool_name = message
        .get("params")
        .and_then(|params| params.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    match tool_name {
        "ccc_server_identity" => Some(tool_result(
            id,
            create_server_identity_text(session_context),
            json!({
                "server_identity": create_server_identity_payload(session_context),
                "install_check": create_install_check_payload(session_context)
            }),
        )),
        "ccc_recommend_entry" => {
            let arguments = tool_call_arguments(message);

            match parse_ccc_recommend_entry_arguments(&arguments).map(|parsed| {
                let payload = create_ccc_recommend_entry_payload(&parsed);
                tool_result(
                    id.clone(),
                    create_ccc_recommend_entry_text(&payload),
                    json!({
                        "recommendation": payload
                    }),
                )
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_auto_entry" => {
            let arguments = tool_call_arguments(message);

            match parse_ccc_auto_entry_arguments(&arguments).and_then(|parsed| {
                let payload = create_ccc_auto_entry_payload(session_context, &parsed)?;
                Ok(tool_result(
                    id.clone(),
                    create_ccc_auto_entry_text(&payload),
                    json!({
                        "auto_entry": payload
                    }),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_status" => {
            let arguments = tool_call_arguments(message);

            match resolve_run_locator_arguments(&arguments, "ccc_status").and_then(|locator| {
                let payload = create_ccc_status_payload(session_context, &locator)?;
                Ok(tool_result(
                    id.clone(),
                    create_ccc_status_operator_text(&payload),
                    json!({
                        "status": payload
                    }),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_activity" => {
            let arguments = tool_call_arguments(message);

            match resolve_run_locator_arguments(&arguments, "ccc_activity").and_then(|locator| {
                let payload = create_ccc_activity_payload(session_context, &locator)?;
                Ok(tool_result(
                    id.clone(),
                    create_ccc_activity_text(&payload),
                    json!({
                        "activity": payload
                    }),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_render_app_panel" => {
            let arguments = tool_call_arguments(message);

            match resolve_run_locator_arguments(&arguments, "ccc_render_app_panel").and_then(
                |locator| {
                    let payload = create_ccc_status_payload(session_context, &locator)?;
                    let app_panel = payload.get("app_panel").cloned().unwrap_or(Value::Null);
                    let artifact =
                        write_codex_app_panel_artifact(&locator.run_directory, &app_panel)?;
                    Ok(tool_result_with_meta(
                        id.clone(),
                        create_codex_app_panel_text(&app_panel),
                        json!({
                            "app_panel": app_panel,
                            "artifact": artifact
                        }),
                        json!({
                            "ui.resourceUri": CCC_APP_PANEL_RESOURCE_URI,
                            "openai/outputTemplate": CCC_APP_PANEL_RESOURCE_URI
                        }),
                    ))
                },
            ) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_code_graph" => {
            let arguments = tool_call_arguments(message);

            match create_routed_code_graph_payload(session_context, &arguments).map(|payload| {
                tool_result(
                    id.clone(),
                    create_routed_code_graph_text(&payload),
                    json!({
                        "code_graph": payload
                    }),
                )
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_start" => {
            let arguments = tool_call_arguments(message);

            match parse_ccc_start_arguments(&arguments).and_then(|parsed| {
                let start_payload = create_ccc_start_payload(&parsed)?;
                let locator = resolve_run_locator_arguments(
                    &json!({
                        "run_id": start_payload.get("run_id").cloned().unwrap_or(Value::Null),
                        "cwd": start_payload.get("cwd").cloned().unwrap_or(Value::Null),
                    }),
                    "ccc_status",
                )?;
                let status_payload = create_ccc_status_payload(session_context, &locator)?;
                Ok(tool_result(
                    id.clone(),
                    create_start_tool_text(&start_payload, &status_payload),
                    create_start_tool_structured_content(&start_payload, &status_payload),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_run" => {
            let arguments = tool_call_arguments(message);

            match parse_ccc_run_arguments(&arguments).and_then(|parsed| {
                let run_payload = create_ccc_run_payload(&parsed)?;
                let locator = resolve_run_locator_arguments(
                    &json!({
                        "run_id": run_payload.get("run_id").cloned().unwrap_or(Value::Null),
                        "cwd": run_payload.get("cwd").cloned().unwrap_or(Value::Null),
                    }),
                    "ccc_status",
                )?;
                let status_payload = create_ccc_status_payload(session_context, &locator)?;
                Ok(tool_result(
                    id.clone(),
                    create_run_tool_text(&run_payload, &status_payload),
                    create_run_tool_structured_content(&run_payload, &status_payload),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_orchestrate" => {
            let arguments = tool_call_arguments(message);

            match parse_ccc_orchestrate_arguments(&arguments).and_then(|parsed| {
                let orchestrate_payload = create_ccc_orchestrate_payload(&parsed)?;
                let locator = resolve_run_locator_arguments(
                    &json!({
                        "run_id": orchestrate_payload.get("run_id").cloned().unwrap_or(Value::Null),
                        "cwd": orchestrate_payload.get("cwd").cloned().unwrap_or(Value::Null),
                    }),
                    "ccc_status",
                )?;
                let status_payload = create_ccc_status_payload(session_context, &locator)?;
                Ok(tool_result(
                    id.clone(),
                    create_orchestrate_tool_text(&orchestrate_payload),
                    create_orchestrate_tool_structured_content(
                        &orchestrate_payload,
                        &status_payload,
                    ),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        "ccc_subagent_update" => {
            let arguments = tool_call_arguments(message);

            match parse_ccc_subagent_update_arguments(&arguments).and_then(|parsed| {
                let update_payload = create_ccc_subagent_update_payload(&parsed)?;
                let locator = resolve_run_locator_arguments(
                    &json!({
                        "run_id": update_payload.get("run_id").cloned().unwrap_or(Value::Null),
                        "cwd": update_payload.get("cwd").cloned().unwrap_or(Value::Null),
                    }),
                    "ccc_status",
                )?;
                let status_payload = create_ccc_status_payload(session_context, &locator)?;
                Ok(tool_result(
                    id.clone(),
                    create_subagent_update_tool_text(&update_payload),
                    create_subagent_update_tool_structured_content(
                        &update_payload,
                        &status_payload,
                    ),
                ))
            }) {
                Ok(response) => Some(response),
                Err(error) => Some(tool_error(id, -32602, error.to_string())),
            }
        }
        _ => Some(tool_error(id, -32601, format!("Unknown tool: {tool_name}"))),
    }
}

fn create_routed_code_graph_payload(
    session_context: &SessionContext,
    arguments: &Value,
) -> std::io::Result<Value> {
    create_graph_context_mcp_code_graph_payload_for_config_path(
        arguments,
        Path::new(&session_context.shared_config_path),
    )?
    .map(Ok)
    .unwrap_or_else(|| create_code_graph_payload(arguments))
}

fn create_routed_code_graph_text(payload: &Value) -> String {
    if payload.get("schema").and_then(Value::as_str) == Some("ccc.graph_context_code_graph.v1") {
        create_graph_context_code_graph_text(payload)
    } else {
        create_code_graph_text(payload)
    }
}
