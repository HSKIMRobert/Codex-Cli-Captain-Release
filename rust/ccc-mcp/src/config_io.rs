use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn resolve_shared_config_path() -> PathBuf {
    if let Some(config_root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_root)
            .join("ccc")
            .join("ccc-config.toml");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ccc")
            .join("ccc-config.toml");
    }

    PathBuf::from("ccc-config.toml")
}

fn resolve_previous_shared_config_path() -> PathBuf {
    if let Some(config_root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_root)
            .join("ccc")
            .join("ccc-config.toml");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ccc")
            .join("ccc-config.toml");
    }

    PathBuf::from("ccc-config.toml")
}

pub(crate) fn resolve_previous_shared_config_path_for(config_path: &Path) -> Option<PathBuf> {
    if config_path.file_name().and_then(|value| value.to_str()) != Some("ccc-config.toml") {
        return None;
    }

    let parent = config_path.parent()?;
    if parent.file_name().and_then(|value| value.to_str()) != Some("ccc") {
        return None;
    }

    Some(parent.parent()?.join("ccc").join("ccc-config.toml"))
}

pub(crate) fn resolve_legacy_shared_toml_config_path_for(config_path: &Path) -> Option<PathBuf> {
    if config_path.file_name().and_then(|value| value.to_str()) != Some("ccc-config.toml") {
        return None;
    }

    let parent = config_path.parent()?;
    if parent.file_name().and_then(|value| value.to_str()) != Some("ccc") {
        return None;
    }

    Some(parent.parent()?.join("ccc").join("ccc-config.toml"))
}

pub(crate) fn resolve_legacy_shared_json_config_path_for(config_path: &Path) -> Option<PathBuf> {
    if config_path.file_name().and_then(|value| value.to_str()) != Some("ccc-config.toml") {
        return None;
    }

    let parent = config_path.parent()?;
    if parent.file_name().and_then(|value| value.to_str()) != Some("ccc") {
        return None;
    }

    Some(parent.parent()?.join("ccc").join("ccc-config.json"))
}

pub(crate) fn resolve_legacy_shared_toml_config_path() -> PathBuf {
    if let Some(config_root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_root)
            .join("ccc")
            .join("ccc-config.toml");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ccc")
            .join("ccc-config.toml");
    }

    PathBuf::from("ccc-config.toml")
}

pub(crate) fn resolve_legacy_shared_json_config_path() -> PathBuf {
    if let Some(config_root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_root)
            .join("ccc")
            .join("ccc-config.json");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ccc")
            .join("ccc-config.json");
    }

    PathBuf::from("ccc-config.json")
}

fn resolve_shared_config_home() -> io::Result<PathBuf> {
    if let Some(config_root) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_root));
    }

    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".config"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Unable to resolve the shared CCC config home. Set XDG_CONFIG_HOME or HOME.",
    ))
}

pub(crate) fn resolve_ccc_config_directory() -> io::Result<PathBuf> {
    Ok(resolve_shared_config_home()?.join("ccc"))
}

pub(crate) fn resolve_codex_home() -> io::Result<PathBuf> {
    if let Some(configured) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(configured));
    }

    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".codex"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Unable to resolve Codex home. Set CODEX_HOME or HOME.",
    ))
}

pub(crate) fn resolve_custom_agent_install_directory() -> io::Result<PathBuf> {
    Ok(resolve_codex_home()?.join("agents"))
}

pub(crate) fn write_string_atomic(path: &Path, content: &str) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no parent directory.", path.display()),
        )
    })?;
    fs::create_dir_all(parent)?;
    let temp_path = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp"),
        generate_uuid_like_id()
    ));
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

pub(crate) fn read_json_document(path: &Path) -> io::Result<Value> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON in {}: {error}", path.display()),
        )
    })
}

pub(crate) fn read_optional_json_document(path: &Path) -> io::Result<Option<Value>> {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).map(Some).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSON in {}: {error}", path.display()),
            )
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn read_optional_toml_document(path: &Path) -> io::Result<Option<Value>> {
    match fs::read_to_string(path) {
        Ok(content) => {
            let parsed = toml::from_str::<toml::Value>(&content).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid TOML in {}: {error}", path.display()),
                )
            })?;
            serde_json::to_value(parsed).map(Some).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("TOML conversion failed for {}: {error}", path.display()),
                )
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn read_optional_shared_config_document() -> io::Result<Option<(PathBuf, Value)>> {
    let preferred_path = resolve_shared_config_path();
    if let Some(value) = read_optional_toml_document(&preferred_path)? {
        return Ok(Some((preferred_path, value)));
    }

    let previous_path = resolve_previous_shared_config_path();
    if previous_path != preferred_path {
        if let Some(value) = read_optional_toml_document(&previous_path)? {
            return Ok(Some((previous_path, value)));
        }
    }

    let legacy_toml_path = resolve_legacy_shared_toml_config_path();
    if let Some(value) = read_optional_toml_document(&legacy_toml_path)? {
        return Ok(Some((legacy_toml_path, value)));
    }

    let legacy_json_path = resolve_legacy_shared_json_config_path();
    if let Some(value) = read_optional_json_document(&legacy_json_path)? {
        return Ok(Some((legacy_json_path, value)));
    }

    Ok(None)
}

pub(crate) fn write_toml_document(path: &Path, value: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let sanitized =
        sanitize_value_for_toml(value).unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let content = toml::to_string_pretty(&sanitized).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode TOML {}: {error}", path.display()),
        )
    })?;
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn timestamped_backup_path_for(path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.6fZ");
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ccc-config.toml");
    path.with_file_name(format!("{file_name}.{timestamp}.bak"))
}

pub(crate) fn create_timestamped_backup(path: &Path) -> io::Result<PathBuf> {
    let mut backup_path = timestamped_backup_path_for(path);
    let mut suffix = 1;
    while backup_path.exists() {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("ccc-config.toml");
        backup_path = path.with_file_name(format!(
            "{file_name}.{}.{}.bak",
            Utc::now().format("%Y%m%dT%H%M%S%.6fZ"),
            suffix
        ));
        suffix += 1;
    }
    fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

pub(crate) fn sanitize_value_for_toml(value: &Value) -> Option<Value> {
    match value {
        Value::Null => None,
        Value::Bool(_) | Value::Number(_) | Value::String(_) => Some(value.clone()),
        Value::Array(values) => Some(Value::Array(
            values.iter().filter_map(sanitize_value_for_toml).collect(),
        )),
        Value::Object(entries) => Some(Value::Object(
            entries
                .iter()
                .filter_map(|(key, nested)| {
                    sanitize_value_for_toml(nested).map(|sanitized| (key.clone(), sanitized))
                })
                .collect(),
        )),
    }
}

pub(crate) fn generate_uuid_like_id() -> String {
    let seed = format!(
        "{}:{}:{}",
        process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let digest = Sha256::digest(seed.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub(crate) fn write_json_document(path: &Path, value: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_vec_pretty(value).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("encode JSON {}: {error}", path.display()),
        )
    })?;
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn is_permission_error(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(1)
}
