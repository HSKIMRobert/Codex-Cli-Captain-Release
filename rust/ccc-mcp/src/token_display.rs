#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
const TOKEN_USAGE_GAUGE_WIDTH: usize = 24;

pub(crate) fn format_compact_token_count(token_count: u64) -> String {
    if token_count >= 1_000_000 {
        format!("{:.1}m", token_count as f64 / 1_000_000.0)
    } else if token_count >= 1_000 {
        format!("{:.1}k", token_count as f64 / 1_000.0)
    } else {
        token_count.to_string()
    }
}

#[cfg(test)]
pub(crate) fn build_token_usage_bar(by_agent: &[Value], total_tokens: u64) -> Option<String> {
    build_agent_count_bar(by_agent, total_tokens, "total_tokens")
}

#[cfg(test)]
pub(crate) fn build_context_usage_bar(
    by_agent: &[Value],
    total_context_tokens: u64,
) -> Option<String> {
    build_agent_count_bar(by_agent, total_context_tokens, "context_tokens")
}

#[cfg(test)]
fn build_agent_count_bar(
    by_agent: &[Value],
    total_tokens: u64,
    count_field: &str,
) -> Option<String> {
    if total_tokens == 0 {
        return None;
    }
    let mut segments = by_agent
        .iter()
        .filter_map(|agent| {
            let tokens = agent.get(count_field)?.as_u64()?;
            if tokens == 0 {
                return None;
            }
            Some((
                agent.get("agent_id")?.as_str()?.to_string(),
                tokens.min(total_tokens),
            ))
        })
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return Some(format!("[{}]", "█".repeat(TOKEN_USAGE_GAUGE_WIDTH)));
    }

    segments.sort_by(|left, right| right.1.cmp(&left.1));
    let total_width = TOKEN_USAGE_GAUGE_WIDTH;
    let palette = ['█', '▓', '▒', '░', '■', '▦', '▨', '▧'];
    let last_index = segments.len().saturating_sub(1);
    let mut remaining = total_width;
    let mut parts = Vec::new();
    for (index, (_, tokens)) in segments.iter().enumerate() {
        let width = if index == last_index {
            remaining
        } else {
            let proportional =
                ((*tokens as f64 / total_tokens as f64) * total_width as f64).round() as usize;
            let bounded = proportional
                .max(1)
                .min(remaining.saturating_sub(last_index - index));
            remaining = remaining.saturating_sub(bounded);
            bounded
        };
        let fill = palette[index % palette.len()];
        parts.push(fill.to_string().repeat(width.max(1)));
    }
    Some(format!("[{}]", parts.join("")))
}

#[cfg(test)]
pub(crate) fn build_token_usage_breakdown(by_agent: &[Value], total_tokens: u64) -> Option<String> {
    build_agent_count_breakdown(by_agent, total_tokens, "total_tokens")
}

#[cfg(test)]
pub(crate) fn build_context_usage_breakdown(
    by_agent: &[Value],
    total_context_tokens: u64,
) -> Option<String> {
    build_agent_count_breakdown(by_agent, total_context_tokens, "context_tokens")
}

#[cfg(test)]
fn build_agent_count_breakdown(
    by_agent: &[Value],
    total_tokens: u64,
    count_field: &str,
) -> Option<String> {
    if by_agent.is_empty() || total_tokens == 0 {
        return None;
    }
    let mut parts = by_agent
        .iter()
        .filter_map(|agent| {
            let agent_id = agent.get("agent_id")?.as_str()?;
            let agent_tokens = agent.get(count_field)?.as_u64()?;
            Some((agent_id.to_string(), agent_tokens))
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    parts.sort_by(|left, right| right.1.cmp(&left.1));
    Some(
        parts
            .into_iter()
            .map(|(agent_id, agent_tokens)| {
                let percentage = (agent_tokens as f64 / total_tokens as f64) * 100.0;
                format!(
                    "{agent_id} {:.0}% ({})",
                    percentage,
                    format_compact_token_count(agent_tokens)
                )
            })
            .collect::<Vec<_>>()
            .join(" | "),
    )
}

#[cfg(test)]
pub(crate) fn token_usage_by_agent(payload: &Value) -> Option<&[Value]> {
    payload
        .get("token_usage")
        .and_then(|value| value.get("by_subagent"))
        .and_then(Value::as_array)
        .or_else(|| {
            payload
                .get("token_usage")
                .and_then(|value| value.get("by_agent"))
                .and_then(Value::as_array)
        })
        .map(Vec::as_slice)
}

#[cfg(test)]
pub(crate) fn token_context_total(payload: &Value) -> Option<u64> {
    payload
        .get("token_usage")
        .and_then(|value| value.get("total_context_tokens"))
        .and_then(Value::as_u64)
}
