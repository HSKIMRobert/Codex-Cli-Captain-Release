use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportMode {
    Framed,
    JsonStream,
}

fn detect_transport_mode<R: BufRead>(reader: &mut R) -> io::Result<Option<TransportMode>> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(None);
        }

        let first = buffer[0];
        if first.is_ascii_whitespace() {
            reader.consume(1);
            continue;
        }

        return Ok(Some(if first == b'{' || first == b'[' {
            TransportMode::JsonStream
        } else {
            TransportMode::Framed
        }));
    }
}

fn read_json_stream_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Value>> {
    match Value::deserialize(&mut serde_json::Deserializer::from_reader(reader)) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.is_eof() => Ok(None),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON stream payload: {error}"),
        )),
    }
}

fn read_framed_mcp_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Value>> {
    let mut headers = BTreeMap::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);

        if trimmed.is_empty() {
            break;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid MCP header line: {trimmed}"),
            ));
        };

        headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    let content_length = headers
        .get("content-length")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header"))?
        .parse::<usize>()
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid Content-Length header: {error}"),
            )
        })?;

    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body)?;
    let message = serde_json::from_slice::<Value>(&body).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON payload: {error}"),
        )
    })?;

    Ok(Some(message))
}

pub(crate) fn read_mcp_message<R: BufRead>(
    reader: &mut R,
    transport_mode: &mut Option<TransportMode>,
) -> io::Result<Option<Value>> {
    let mode = match transport_mode {
        Some(mode) => *mode,
        None => {
            let Some(detected) = detect_transport_mode(reader)? else {
                return Ok(None);
            };
            *transport_mode = Some(detected);
            detected
        }
    };

    match mode {
        TransportMode::Framed => read_framed_mcp_message(reader),
        TransportMode::JsonStream => read_json_stream_message(reader),
    }
}

pub(crate) fn write_mcp_message<W: Write>(
    writer: &mut W,
    value: &Value,
    transport_mode: TransportMode,
) -> io::Result<()> {
    let payload = serde_json::to_vec(value).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("encode error: {error}"))
    })?;
    match transport_mode {
        TransportMode::Framed => {
            write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
            writer.write_all(&payload)?;
        }
        TransportMode::JsonStream => {
            writer.write_all(&payload)?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{BufReader, Cursor};

    #[test]
    fn framed_transport_round_trips_message() {
        let message = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});
        let mut output = Vec::new();

        write_mcp_message(&mut output, &message, TransportMode::Framed).expect("write frame");

        assert!(String::from_utf8_lossy(&output).starts_with("Content-Length: "));
        let mut reader = BufReader::new(Cursor::new(output));
        let mut mode = Some(TransportMode::Framed);

        assert_eq!(
            read_mcp_message(&mut reader, &mut mode).expect("read frame"),
            Some(message)
        );
    }

    #[test]
    fn json_stream_transport_round_trips_message() {
        let message = json!({"jsonrpc": "2.0", "id": 2, "result": "ok"});
        let mut output = Vec::new();

        write_mcp_message(&mut output, &message, TransportMode::JsonStream)
            .expect("write json stream");

        assert!(output.ends_with(b"\n"));
        let mut reader = BufReader::new(Cursor::new(output));
        let mut mode = None;

        assert_eq!(
            read_mcp_message(&mut reader, &mut mode).expect("read json stream"),
            Some(message)
        );
        assert_eq!(mode, Some(TransportMode::JsonStream));
    }

    #[test]
    fn transport_detection_skips_leading_whitespace() {
        let mut reader = BufReader::new(Cursor::new(b"\n \t{\"ok\":true}".to_vec()));
        let mut mode = None;

        assert_eq!(
            read_mcp_message(&mut reader, &mut mode).expect("read json stream"),
            Some(json!({"ok": true}))
        );
        assert_eq!(mode, Some(TransportMode::JsonStream));
    }

    #[test]
    fn framed_transport_rejects_missing_content_length() {
        let mut reader = BufReader::new(Cursor::new(b"X-Test: 1\r\n\r\n{}".to_vec()));
        let mut mode = Some(TransportMode::Framed);
        let error = read_mcp_message(&mut reader, &mut mode).expect_err("missing length");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("missing Content-Length"));
    }
}
