use crate::captain_intervention::{
    is_valid_intervention_classification, is_valid_intervention_next_action,
};
use crate::host_subagent_lifecycle::is_terminal_or_merged_host_subagent_status;
use crate::review_policy::{canonical_review_outcome, is_valid_review_outcome};
use crate::specialist_roles::SUBAGENT_FALLBACK_REASON_CODES;
use std::io;

pub(crate) const SENTINEL_INTERVENTION_CLASSIFICATIONS: &[&str] = &["observe", "warn", "enforce"];

pub(crate) fn canonical_subagent_review_outcome(outcome: &str) -> io::Result<&'static str> {
    if is_valid_review_outcome(outcome) {
        Ok(canonical_review_outcome(outcome).expect("valid review outcome has canonical form"))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update review_outcome must be one of: passed, needs_work, unsatisfactory, blocked, stalled, reclaimed.",
        ))
    }
}

pub(crate) fn canonical_subagent_fan_in_status(status: &str) -> String {
    canonical_review_outcome(status)
        .unwrap_or(status)
        .to_string()
}

pub(crate) fn validate_subagent_intervention_classification(
    classification: &str,
) -> io::Result<()> {
    if is_valid_intervention_classification(classification) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update intervention_classification must be one of: clarification_only, bounded_scope_amendment, direction_or_risk_correction.",
        ))
    }
}

pub(crate) fn validate_subagent_chosen_next_action(action: &str) -> io::Result<()> {
    if is_valid_intervention_next_action(action) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update chosen_next_action must be one of: amend_same_worker, reclaim, reassign, close, clarify, no_action.",
        ))
    }
}

pub(crate) fn validate_sentinel_intervention_classification(
    classification: &str,
) -> io::Result<()> {
    if SENTINEL_INTERVENTION_CLASSIFICATIONS.contains(&classification) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update sentinel_classification must be one of: observe, warn, enforce.",
        ))
    }
}

pub(crate) fn is_valid_subagent_fallback_reason(reason: &str) -> bool {
    SUBAGENT_FALLBACK_REASON_CODES.contains(&reason)
}

pub(crate) fn validate_subagent_fallback_reason_for_status(
    reason: &str,
    status: &str,
) -> io::Result<()> {
    if !is_valid_subagent_fallback_reason(reason) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "ccc_subagent_update fallback_reason must be one of: {}.",
                SUBAGENT_FALLBACK_REASON_CODES.join(", ")
            ),
        ));
    }
    if !is_terminal_or_merged_host_subagent_status(status) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ccc_subagent_update fallback_reason requires terminal specialist status: completed, failed, stalled, merged, or reclaimed.",
        ));
    }
    Ok(())
}
