pub(crate) fn summarize_text_for_visibility(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>()
        + "..."
}

fn normalize_compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn compact_prompt_text(text: &str, max_chars: usize) -> String {
    summarize_text_for_visibility(text, max_chars.max(24))
}

pub(crate) fn prompt_fields_match(left: &str, right: &str) -> bool {
    !left.trim().is_empty() && normalize_compact_text(left) == normalize_compact_text(right)
}
