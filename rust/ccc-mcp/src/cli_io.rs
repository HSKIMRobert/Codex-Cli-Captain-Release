use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CliOutputMode {
    Json,
    Text,
    Quiet,
}

#[derive(Debug)]
pub(crate) struct CliCommandInput {
    pub(crate) payload: Value,
    pub(crate) output_mode: CliOutputMode,
    pub(crate) app_panel: bool,
    pub(crate) artifact: bool,
    pub(crate) subagents: bool,
    pub(crate) projection: bool,
    transient_json_file: Option<PathBuf>,
}

impl CliCommandInput {
    pub(crate) fn cleanup_transient_json_file_after_success(&self) {
        cleanup_transient_json_file(&self.transient_json_file);
    }

    #[cfg(test)]
    pub(crate) fn transient_json_file_path(&self) -> Option<&Path> {
        self.transient_json_file.as_deref()
    }
}

#[derive(Debug)]
pub(crate) struct CliJsonInput {
    pub(crate) payload: Value,
    transient_json_file: Option<PathBuf>,
}

impl CliJsonInput {
    pub(crate) fn cleanup_transient_json_file_after_success(&self) {
        cleanup_transient_json_file(&self.transient_json_file);
    }
}

pub(crate) fn parse_cli_command_input(
    command: &str,
    args: &[String],
    allow_empty: bool,
) -> io::Result<CliCommandInput> {
    let visibility_usage = if matches!(command, "status" | "checklist") {
        " [--subagents|--projection]"
    } else {
        ""
    };
    let usage = format!(
        "Usage: ccc {command} [--text|--quiet]{visibility_usage} [--json '{{...}}' | --json-file /path/to/input.json]"
    );
    let mut output_mode = CliOutputMode::Json;
    let mut inline_json: Option<&str> = None;
    let mut json_file: Option<&str> = None;
    let mut app_panel = false;
    let mut artifact = false;
    let mut subagents = false;
    let mut projection = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                if index + 1 >= args.len() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
                }
                if inline_json.is_some() || json_file.is_some() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
                }
                inline_json = Some(args[index + 1].as_str());
                index += 2;
            }
            "--json-file" => {
                if index + 1 >= args.len() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
                }
                if inline_json.is_some() || json_file.is_some() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
                }
                json_file = Some(args[index + 1].as_str());
                index += 2;
            }
            "--text" => {
                if output_mode != CliOutputMode::Json {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
                }
                output_mode = CliOutputMode::Text;
                index += 1;
            }
            "--quiet" => {
                if output_mode != CliOutputMode::Json {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
                }
                output_mode = CliOutputMode::Quiet;
                index += 1;
            }
            "--app-panel" if command == "status" => {
                app_panel = true;
                index += 1;
            }
            "--artifact" if command == "status" => {
                artifact = true;
                index += 1;
            }
            "--subagents" if matches!(command, "status" | "checklist") => {
                subagents = true;
                index += 1;
            }
            "--projection" if matches!(command, "status" | "checklist") => {
                projection = true;
                index += 1;
            }
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, usage.clone()));
            }
        }
    }

    if subagents {
        if app_panel || artifact {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "`ccc status --subagents` cannot be combined with `--app-panel` or `--artifact`.",
            ));
        }
        if output_mode == CliOutputMode::Json {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "`ccc {command} --subagents` requires `--text` or `--quiet`; JSON output is not supported."
                ),
            ));
        }
    }

    if projection {
        if subagents || app_panel || artifact {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "`ccc {command} --projection` cannot be combined with `--subagents`, `--app-panel`, or `--artifact`."
                ),
            ));
        }
    }

    if artifact && !app_panel {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "`ccc status --artifact` requires `--app-panel`.",
        ));
    }

    let payload = if let Some(raw) = inline_json {
        let parsed = serde_json::from_str::<Value>(raw).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid JSON for {command}: {error}"),
            )
        })?;
        if !parsed.is_object() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{command} requires a JSON object."),
            ));
        }
        parsed
    } else if let Some(path) = json_file {
        let raw = fs::read_to_string(path).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unable to read JSON file for {command}: {error}"),
            )
        })?;
        let parsed = serde_json::from_str::<Value>(&raw).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid JSON file for {command}: {error}"),
            )
        })?;
        if !parsed.is_object() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{command} requires a JSON object."),
            ));
        }
        parsed
    } else if allow_empty {
        json!({})
    } else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, usage));
    };

    Ok(CliCommandInput {
        payload,
        output_mode,
        app_panel,
        artifact,
        subagents,
        projection,
        transient_json_file: json_file.and_then(transient_json_file_cleanup_path),
    })
}

pub(crate) fn parse_cli_json_argument(
    command: &str,
    args: &[String],
    allow_empty: bool,
) -> io::Result<CliJsonInput> {
    match args {
        [] if allow_empty => Ok(CliJsonInput {
            payload: json!({}),
            transient_json_file: None,
        }),
        [flag, raw] if flag == "--json" => {
            let parsed = serde_json::from_str::<Value>(raw).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid JSON for {command}: {error}"),
                )
            })?;
            if parsed.is_object() {
                Ok(CliJsonInput {
                    payload: parsed,
                    transient_json_file: None,
                })
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{command} requires a JSON object."),
                ))
            }
        }
        [flag, path] if flag == "--json-file" => {
            let raw = fs::read_to_string(path).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unable to read JSON file for {command}: {error}"),
                )
            })?;
            let parsed = serde_json::from_str::<Value>(&raw).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid JSON file for {command}: {error}"),
                )
            })?;
            if parsed.is_object() {
                Ok(CliJsonInput {
                    payload: parsed,
                    transient_json_file: transient_json_file_cleanup_path(path),
                })
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{command} requires a JSON object."),
                ))
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Usage: ccc {command} [--json '{{...}}' | --json-file /path/to/input.json]"),
        )),
    }
}

fn cleanup_transient_json_file(path: &Option<PathBuf>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}

fn transient_json_file_cleanup_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let parent = path.parent()?;
    let components = normalized_path_components(parent)?;
    let is_ccc_tmp_parent = if path.is_absolute() {
        components.len() >= 2
            && components[components.len() - 2] == ".ccc"
            && components[components.len() - 1] == "tmp"
    } else {
        components == [".ccc", "tmp"]
    };
    is_ccc_tmp_parent.then(|| path.to_path_buf())
}

fn normalized_path_components(path: &Path) -> Option<Vec<String>> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => components.push(value.to_string_lossy().to_string()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::ParentDir => return None,
        }
    }
    Some(components)
}

pub(crate) fn print_json_payload(payload: &Value) -> io::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(payload).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize JSON payload: {error}"),
            )
        })?
    );
    Ok(())
}

pub(crate) fn print_text_line(text: &str) -> io::Result<()> {
    println!("{text}");
    Ok(())
}
