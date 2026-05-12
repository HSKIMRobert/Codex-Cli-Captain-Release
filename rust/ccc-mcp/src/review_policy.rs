use crate::parallel_fanout::has_large_mutation_signal;
use crate::request_routing::{
    combine_request_text_for_routing, infer_request_shape, infer_task_shape,
};
use crate::skill_registry::load_skill_registry_for_agent;
use crate::{
    build_task_card_payload_with_role, generate_uuid_like_id, read_json_document,
    write_json_document,
};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::thread;

const MAX_REVIEWER_CAP: u64 = 1;
const LOW_MEMORY_AVAILABLE_PERCENT: u64 = 10;
const LOGICAL_PRECHECK_AGENTS: [&str; 2] = ["ccc_sentinel", "ccc_arbiter"];

#[derive(Clone, Debug, Default)]
pub(crate) struct RuntimeReviewPressureSnapshot {
    pub(crate) source: String,
    pub(crate) stale_worker_count: u64,
    pub(crate) timed_out_worker_count: u64,
    pub(crate) reclaim_needed_worker_count: u64,
    pub(crate) active_run_count: u64,
    pub(crate) token_total: u64,
    pub(crate) token_soft_limit: Option<u64>,
    pub(crate) cpu_available_parallelism: Option<u64>,
    pub(crate) memory_total_kib: Option<u64>,
    pub(crate) memory_available_kib: Option<u64>,
    pub(crate) memory_available_percent: Option<u64>,
    pub(crate) pressure_reason: Option<String>,
}

impl RuntimeReviewPressureSnapshot {
    pub(crate) fn has_observed_pressure(&self) -> bool {
        self.stale_worker_count > 0
            || self.timed_out_worker_count > 0
            || self.reclaim_needed_worker_count > 0
            || self.active_run_count > 0
            || self
                .token_soft_limit
                .map(|limit| limit > 0 && self.token_total >= limit)
                .unwrap_or(false)
            || self.cpu_available_parallelism.is_some()
            || self.memory_available_percent.is_some()
            || self.pressure_reason.is_some()
    }

    fn high_pressure_reason(&self) -> Option<String> {
        if let Some(reason) = self
            .pressure_reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(reason.to_string());
        }
        if self.timed_out_worker_count > 0 {
            return Some(format!(
                "{} timed-out worker(s) require bounded reclaim before adding reviewer load.",
                self.timed_out_worker_count
            ));
        }
        if self.stale_worker_count > 0 || self.reclaim_needed_worker_count > 0 {
            return Some(format!(
                "{} worker(s) need reclaim pressure handling before adding reviewer load.",
                self.reclaim_needed_worker_count
                    .max(self.stale_worker_count)
            ));
        }
        if self.active_run_count > 0 {
            return Some(format!(
                "{} active prior run(s) require continuity handling before adding reviewer load.",
                self.active_run_count
            ));
        }
        if let Some(limit) = self.token_soft_limit {
            if limit > 0 && self.token_total >= limit {
                return Some(format!(
                    "Run token usage ({}) reached configured review soft limit ({}).",
                    self.token_total, limit
                ));
            }
        }
        if let Some(percent) = self.memory_available_percent {
            if percent <= LOW_MEMORY_AVAILABLE_PERCENT {
                return Some(format!(
                    "OS memory availability is low ({}% available), so reviewer load is suppressed.",
                    percent
                ));
            }
        }
        if self.cpu_available_parallelism == Some(1) {
            return Some(
                "Only one available CPU thread was reported, so reviewer load is suppressed."
                    .to_string(),
            );
        }
        None
    }

    fn to_policy_metadata(&self) -> Value {
        json!({
            "source": if self.source.trim().is_empty() { "runtime_snapshot" } else { self.source.as_str() },
            "high_pressure": self.high_pressure_reason().is_some(),
            "pressure_reason": self.high_pressure_reason().map(Value::String).unwrap_or(Value::Null),
            "stale_worker_count": self.stale_worker_count,
            "timed_out_worker_count": self.timed_out_worker_count,
            "reclaim_needed_worker_count": self.reclaim_needed_worker_count,
            "active_run_count": self.active_run_count,
            "token_total": self.token_total,
            "token_soft_limit": self.token_soft_limit.map(Value::from).unwrap_or(Value::Null),
            "cpu_available_parallelism": self.cpu_available_parallelism.map(Value::from).unwrap_or(Value::Null),
            "memory_total_kib": self.memory_total_kib.map(Value::from).unwrap_or(Value::Null),
            "memory_available_kib": self.memory_available_kib.map(Value::from).unwrap_or(Value::Null),
            "memory_available_percent": self.memory_available_percent.map(Value::from).unwrap_or(Value::Null),
        })
    }
}

pub(crate) fn read_u64_field(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

pub(crate) fn runtime_review_pressure_snapshot_from_value(
    value: Option<&Value>,
    source: &str,
) -> Option<RuntimeReviewPressureSnapshot> {
    let value = value.filter(|value| value.is_object())?;
    let snapshot = RuntimeReviewPressureSnapshot {
        source: source.to_string(),
        stale_worker_count: read_u64_field(value, "stale_worker_count"),
        timed_out_worker_count: read_u64_field(value, "timed_out_worker_count"),
        reclaim_needed_worker_count: read_u64_field(value, "reclaim_needed_worker_count"),
        active_run_count: read_u64_field(value, "active_run_count")
            .max(read_u64_field(value, "fresh_active_run_count")),
        token_total: read_u64_field(value, "token_total")
            .max(read_u64_field(value, "total_tokens")),
        token_soft_limit: value
            .get("token_soft_limit")
            .or_else(|| value.get("review_token_soft_limit"))
            .and_then(Value::as_u64)
            .filter(|limit| *limit > 0),
        cpu_available_parallelism: value
            .get("cpu_available_parallelism")
            .or_else(|| value.get("available_parallelism"))
            .and_then(Value::as_u64)
            .filter(|value| *value > 0),
        memory_total_kib: value
            .get("memory_total_kib")
            .or_else(|| value.get("total_memory_kib"))
            .and_then(Value::as_u64),
        memory_available_kib: value
            .get("memory_available_kib")
            .or_else(|| value.get("available_memory_kib"))
            .and_then(Value::as_u64),
        memory_available_percent: value
            .get("memory_available_percent")
            .or_else(|| value.get("available_memory_percent"))
            .and_then(Value::as_u64),
        pressure_reason: value
            .get("pressure_reason")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(str::to_string),
    };
    snapshot.has_observed_pressure().then_some(snapshot)
}

fn merge_runtime_review_pressure_snapshots(
    first: Option<&RuntimeReviewPressureSnapshot>,
    second: Option<&RuntimeReviewPressureSnapshot>,
) -> Option<RuntimeReviewPressureSnapshot> {
    match (first, second) {
        (Some(first), Some(second)) => Some(RuntimeReviewPressureSnapshot {
            source: "combined_runtime_snapshot".to_string(),
            stale_worker_count: first.stale_worker_count.max(second.stale_worker_count),
            timed_out_worker_count: first
                .timed_out_worker_count
                .max(second.timed_out_worker_count),
            reclaim_needed_worker_count: first
                .reclaim_needed_worker_count
                .max(second.reclaim_needed_worker_count),
            active_run_count: first.active_run_count.max(second.active_run_count),
            token_total: first.token_total.max(second.token_total),
            token_soft_limit: match (first.token_soft_limit, second.token_soft_limit) {
                (Some(first), Some(second)) => Some(first.min(second)),
                (Some(limit), None) | (None, Some(limit)) => Some(limit),
                (None, None) => None,
            },
            cpu_available_parallelism: match (
                first.cpu_available_parallelism,
                second.cpu_available_parallelism,
            ) {
                (Some(first), Some(second)) => Some(first.min(second)),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
            memory_total_kib: first.memory_total_kib.or(second.memory_total_kib),
            memory_available_kib: match (first.memory_available_kib, second.memory_available_kib) {
                (Some(first), Some(second)) => Some(first.min(second)),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
            memory_available_percent: match (
                first.memory_available_percent,
                second.memory_available_percent,
            ) {
                (Some(first), Some(second)) => Some(first.min(second)),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
            pressure_reason: first
                .pressure_reason
                .clone()
                .or_else(|| second.pressure_reason.clone()),
        }),
        (Some(snapshot), None) | (None, Some(snapshot)) => Some(snapshot.clone()),
        (None, None) => None,
    }
}

pub(crate) fn runtime_review_pressure_snapshot_from_start_scan(
    active_run_scan: &Value,
) -> Option<RuntimeReviewPressureSnapshot> {
    runtime_review_pressure_snapshot_from_value(Some(active_run_scan), "active_run_scan")
}

fn linux_memory_snapshot() -> Option<(u64, u64)> {
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = None;
    let mut available = None;
    for line in meminfo.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("MemTotal:") => total = parts.next().and_then(|value| value.parse::<u64>().ok()),
            Some("MemAvailable:") => {
                available = parts.next().and_then(|value| value.parse::<u64>().ok())
            }
            _ => {}
        }
    }
    Some((total?, available?))
}

#[cfg(target_os = "macos")]
fn macos_memory_snapshot() -> Option<(u64, u64)> {
    let total_bytes = String::from_utf8(
        Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?
            .stdout,
    )
    .ok()?
    .trim()
    .parse::<u64>()
    .ok()?;

    let page_size = String::from_utf8(Command::new("pagesize").output().ok()?.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;

    let vm_stat = String::from_utf8(Command::new("vm_stat").output().ok()?.stdout).ok()?;
    let mut free_pages = 0_u64;
    let mut inactive_pages = 0_u64;
    let mut speculative_pages = 0_u64;
    for line in vm_stat.lines() {
        let value = line
            .split(':')
            .nth(1)
            .map(|part| part.trim().trim_end_matches('.').replace('.', ""))
            .and_then(|part| part.parse::<u64>().ok())
            .unwrap_or(0);
        if line.starts_with("Pages free:") {
            free_pages = value;
        } else if line.starts_with("Pages inactive:") {
            inactive_pages = value;
        } else if line.starts_with("Pages speculative:") {
            speculative_pages = value;
        }
    }

    let available_bytes = free_pages
        .saturating_add(inactive_pages)
        .saturating_add(speculative_pages)
        .saturating_mul(page_size);
    Some((total_bytes / 1024, available_bytes / 1024))
}

#[cfg(not(target_os = "macos"))]
fn macos_memory_snapshot() -> Option<(u64, u64)> {
    None
}

pub(crate) fn collect_os_review_pressure_snapshot() -> Option<RuntimeReviewPressureSnapshot> {
    let cpu_available_parallelism = thread::available_parallelism()
        .ok()
        .map(|value| value.get() as u64);
    let memory = linux_memory_snapshot().or_else(macos_memory_snapshot);
    let memory_total_kib = memory.map(|(total, _)| total);
    let memory_available_kib = memory.map(|(_, available)| available);
    let memory_available_percent = memory.and_then(|(total, available)| {
        (total > 0).then_some(available.saturating_mul(100) / total)
    });

    if cpu_available_parallelism.is_none() && memory_available_percent.is_none() {
        return None;
    }

    Some(RuntimeReviewPressureSnapshot {
        source: "os_resource_snapshot".to_string(),
        stale_worker_count: 0,
        timed_out_worker_count: 0,
        reclaim_needed_worker_count: 0,
        active_run_count: 0,
        token_total: 0,
        token_soft_limit: None,
        cpu_available_parallelism,
        memory_total_kib,
        memory_available_kib,
        memory_available_percent,
        pressure_reason: None,
    })
}

pub(crate) fn canonical_review_outcome(outcome: &str) -> Option<&'static str> {
    match outcome.trim() {
        "passed" => Some("passed"),
        "needs_work" | "unsatisfactory" => Some("needs_work"),
        "blocked" => Some("blocked"),
        "stalled" => Some("stalled"),
        "reclaimed" => Some("reclaimed"),
        _ => None,
    }
}

pub(crate) fn is_valid_review_outcome(outcome: &str) -> bool {
    canonical_review_outcome(outcome).is_some()
}

pub(crate) fn task_card_is_review(task_card: &Value) -> bool {
    task_card.get("task_kind").and_then(Value::as_str) == Some("review")
        || task_card.get("assigned_role").and_then(Value::as_str) == Some("verifier")
        || task_card.get("assigned_agent_id").and_then(Value::as_str) == Some("arbiter")
        || task_card.get("assigned_agent_id").and_then(Value::as_str) == Some("ccc_arbiter")
        || task_card
            .get("review_of_task_card_ids")
            .and_then(Value::as_array)
            .map(|values| !values.is_empty())
            .unwrap_or(false)
}

pub(crate) fn infer_review_outcome(
    task_card: &Value,
    status: &str,
    child_agent_id: Option<&str>,
    fan_in_status: Option<&str>,
    explicit_review_outcome: Option<&str>,
) -> Option<String> {
    if let Some(outcome) = explicit_review_outcome.and_then(canonical_review_outcome) {
        return Some(outcome.to_string());
    }
    if let Some(outcome) = fan_in_status.and_then(canonical_review_outcome) {
        return Some(outcome.to_string());
    }
    let lifecycle_child_agent_id = task_card
        .pointer("/subagent_lifecycle/child_agent_id")
        .and_then(Value::as_str);
    if child_agent_id == Some("ccc_arbiter") || lifecycle_child_agent_id == Some("ccc_arbiter") {
        return match status {
            "completed" => Some("passed".to_string()),
            "failed" => Some("needs_work".to_string()),
            "stalled" => Some("stalled".to_string()),
            "reclaimed" => Some("reclaimed".to_string()),
            _ => None,
        };
    }
    if !task_card_is_review(task_card) {
        return None;
    }

    match status {
        "completed" => Some("passed".to_string()),
        "failed" => Some("needs_work".to_string()),
        "stalled" => Some("stalled".to_string()),
        "reclaimed" => Some("reclaimed".to_string()),
        _ => None,
    }
}

pub(crate) fn verification_state_for_review_outcome(outcome: &str) -> &'static str {
    match outcome {
        "passed" => "passed",
        "needs_work" => "needs_work",
        "blocked" | "stalled" | "reclaimed" => "blocked",
        _ => "pending",
    }
}

pub(crate) fn review_pass_cap_for_task_card(task_card: &Value) -> u64 {
    task_card
        .get("review_policy")
        .and_then(|policy| policy.get("reviewer_cap"))
        .and_then(Value::as_u64)
        .unwrap_or(1)
}

pub(crate) fn push_review_cap_finding(value: &mut Value, field: Option<&str>, finding: &str) {
    let finding = Value::String(finding.to_string());
    if let Some(field) = field {
        if let Some(object) = value.as_object_mut() {
            let mut findings = object
                .get(field)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            findings.push(finding);
            object.insert(field.to_string(), Value::Array(findings));
        }
    } else if let Some(findings) = value.as_array_mut() {
        findings.push(finding);
    } else {
        *value = Value::Array(vec![finding]);
    }
}

pub(crate) fn create_review_fan_in_payload(
    task_card: &Value,
    outcome: &str,
    fan_in_compact: &Value,
    findings: Value,
    timestamp: &str,
) -> Value {
    let open_questions = fan_in_compact
        .get("open_questions")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let unresolved_findings = if findings.as_array().map(Vec::is_empty) == Some(false) {
        findings
    } else if matches!(outcome, "needs_work" | "blocked" | "stalled" | "reclaimed") {
        open_questions.clone()
    } else {
        Value::Array(Vec::new())
    };
    let unresolved_finding_count = unresolved_findings
        .as_array()
        .map(Vec::len)
        .unwrap_or_default();
    let next_action = fan_in_compact
        .get("next_action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("captain_decision");

    json!({
        "outcome": outcome,
        "summary": fan_in_compact.get("summary").cloned().unwrap_or(Value::Null),
        "status": fan_in_compact.get("status").cloned().unwrap_or(Value::Null),
        "evidence_paths": fan_in_compact.get("evidence_paths").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "next_action": next_action,
        "open_questions": open_questions,
        "confidence": fan_in_compact.get("confidence").cloned().unwrap_or(Value::Null),
        "reviewed_task_card_ids": task_card.get("review_of_task_card_ids").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "unresolved_findings": unresolved_findings,
        "unresolved_finding_count": unresolved_finding_count,
        "captain_next_decision": next_action,
        "authority": "captain_decides_after_review",
        "recorded_at": timestamp,
    })
}

pub(crate) fn create_review_policy_payload(
    request: &str,
    request_shape: &str,
    _task_shape: &str,
    recorded_at: Option<&str>,
    runtime_pressure: Option<&RuntimeReviewPressureSnapshot>,
) -> Value {
    let logical_precheck = create_logical_risk_precheck_payload(request, request_shape);
    let manifest_requires_review = logical_precheck
        .get("manifest_requires_review")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let high_risk = request_shape == "review"
        || has_large_mutation_signal(request)
        || has_failed_validation_review_signal(request)
        || has_high_risk_review_signal(request)
        || manifest_requires_review;
    let moderate_risk = request_shape == "mutation";
    let resource_limited = has_resource_limit_review_signal(request);
    let runtime_pressure_reason =
        runtime_pressure.and_then(RuntimeReviewPressureSnapshot::high_pressure_reason);
    let runtime_pressure_metadata = runtime_pressure
        .map(RuntimeReviewPressureSnapshot::to_policy_metadata)
        .unwrap_or(Value::Null);

    let (
        decision,
        state,
        risk,
        required,
        recommended_reviewers,
        reviewer_cap,
        reason_code,
        summary,
    ) = if resource_limited {
        (
            "suppress_for_resource_limit",
            "suppressed",
            if moderate_risk { "moderate" } else { "low" },
            false,
            0,
            0,
            "reviewer_resource_limit",
            "Review was suppressed by explicit reviewer resource limits; captain authority remains unchanged.",
        )
    } else if let Some(reason) = runtime_pressure_reason.as_deref() {
        (
            "suppress_for_runtime_pressure",
            "suppressed",
            if high_risk {
                "high"
            } else if moderate_risk {
                "moderate"
            } else {
                "low"
            },
            false,
            0,
            0,
            "runtime_review_pressure",
            reason,
        )
    } else if high_risk {
        (
            "require",
            "required",
            "high",
            true,
            1,
            MAX_REVIEWER_CAP,
            if has_failed_validation_review_signal(request) {
                "failed_validation_or_unresolved_acceptance"
            } else {
                "high_risk_or_explicit_review"
            },
            if has_failed_validation_review_signal(request) {
                "Failed validation or unresolved acceptance requires a bounded review gate before completion."
            } else {
                "High-risk or explicit review signals require a bounded review gate before completion."
            },
        )
    } else if moderate_risk {
        (
            "recommend_single",
            "recommended",
            "moderate",
            false,
            1,
            MAX_REVIEWER_CAP,
            "moderate_risk_mutation",
            "Moderate-risk work recommends one bounded review without spawning a reviewer in this slice.",
        )
    } else {
        (
            "skip",
            "skipped",
            "low",
            false,
            0,
            MAX_REVIEWER_CAP,
            "low_risk_read_only",
            "Low-risk read-only work skips the review gate.",
        )
    };

    json!({
        "decision": decision,
        "state": state,
        "risk": risk,
        "required": required,
        "recommended_reviewers": recommended_reviewers,
        "reviewer_cap": reviewer_cap,
        "active_reviewers": 0,
        "reason_code": reason_code,
        "summary": summary,
        "risk_evidence_source": logical_precheck.get("source").cloned().unwrap_or_else(|| Value::String("fallback_heuristic".to_string())),
        "logical_risk_precheck": logical_precheck,
        "resource_pressure": runtime_pressure_metadata,
        "recorded_at": recorded_at.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null),
    })
}

fn create_logical_risk_precheck_payload(request: &str, request_shape: &str) -> Value {
    let agent_evidence = LOGICAL_PRECHECK_AGENTS
        .into_iter()
        .map(create_logical_precheck_agent_evidence)
        .collect::<Vec<_>>();
    let available_agent_count = agent_evidence
        .iter()
        .filter(|agent| agent.get("status").and_then(Value::as_str) == Some("available"))
        .count();
    let manifest_requires_review = agent_evidence.iter().any(logical_evidence_requires_review);
    let selected_review_owner = if request_shape == "review"
        || manifest_requires_review
        || has_failed_validation_review_signal(request)
        || has_high_risk_review_signal(request)
        || has_large_mutation_signal(request)
    {
        Value::String("ccc_arbiter".to_string())
    } else {
        Value::Null
    };
    let source = if available_agent_count == LOGICAL_PRECHECK_AGENTS.len() {
        "skill_registry"
    } else {
        "fallback_heuristic"
    };

    json!({
        "schema": "ccc.logical_risk_precheck.v1",
        "source": source,
        "blocking": false,
        "advisory_only": true,
        "classifier_agent": "ccc_sentinel",
        "review_agent": "ccc_arbiter",
        "selected_review_owner": selected_review_owner,
        "manifest_requires_review": manifest_requires_review,
        "agents": agent_evidence,
    })
}

fn create_logical_precheck_agent_evidence(agent_name: &str) -> Value {
    let registry = load_skill_registry_for_agent(agent_name, &json!({}));
    let status = registry
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let logical = registry
        .get("skill_ssl_manifest")
        .and_then(|manifest| manifest.get("logical"))
        .cloned()
        .unwrap_or(Value::Null);

    // Logical manifests are advisory risk evidence for Sentinel/Arbiter. They
    // can raise a review gate, but they never replace persisted task state.
    json!({
        "agent_name": agent_name,
        "status": status,
        "source": if status == "available" && logical.is_object() { "skill_registry" } else { "fallback_heuristic" },
        "logical": logical,
    })
}

fn logical_evidence_requires_review(agent: &Value) -> bool {
    let logical = agent.get("logical").unwrap_or(&Value::Null);
    logical
        .get("requires_operator_approval")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || logical
            .get("external_side_effects")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || logical
            .get("actions")
            .and_then(Value::as_array)
            .map(|actions| {
                actions.iter().any(|action| {
                    matches!(
                        action.get("risk").and_then(Value::as_str),
                        Some("high" | "critical")
                    )
                })
            })
            .unwrap_or(false)
}

pub(crate) fn review_policy_for_start_payload(
    parsed: &Value,
    timestamp: &str,
    runtime_pressure: Option<&RuntimeReviewPressureSnapshot>,
) -> Value {
    if let Some(policy) = parsed
        .get("review_policy")
        .filter(|value| value.is_object())
    {
        let mut policy = policy.clone();
        if let Some(object) = policy.as_object_mut() {
            object.insert(
                "recorded_at".to_string(),
                Value::String(timestamp.to_string()),
            );
        }
        return policy;
    }

    let request_text = combine_request_text_for_routing(parsed);
    let request_shape = infer_request_shape(&request_text);
    let task_shape = infer_task_shape(&request_text, request_shape);
    let payload_runtime_pressure = runtime_review_pressure_snapshot_from_value(
        parsed.get("runtime_pressure"),
        "request_payload",
    );
    let os_runtime_pressure = collect_os_review_pressure_snapshot();
    let merged_runtime_pressure = merge_runtime_review_pressure_snapshots(
        runtime_pressure,
        payload_runtime_pressure.as_ref(),
    );
    let merged_runtime_pressure = merge_runtime_review_pressure_snapshots(
        merged_runtime_pressure.as_ref(),
        os_runtime_pressure.as_ref(),
    );
    create_review_policy_payload(
        &request_text,
        request_shape,
        task_shape,
        Some(timestamp),
        merged_runtime_pressure.as_ref(),
    )
}

fn review_policy_suppresses_captain_review(review_policy: &Value) -> bool {
    let state = review_policy
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or_default();
    matches!(state, "skipped" | "suppressed")
}

fn review_policy_requests_captain_review(review_policy: &Value) -> bool {
    if review_policy_suppresses_captain_review(review_policy) {
        return false;
    }

    let decision = review_policy
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or_default();
    matches!(decision, "require" | "recommend_single")
}

fn task_card_requires_mutation_review_follow_up(task_card: &Value) -> bool {
    let assigned_role = task_card
        .get("assigned_role")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let assigned_agent = task_card
        .get("assigned_agent_id")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let task_kind = task_card
        .get("task_kind")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let request_shape = task_card
        .pointer("/routing_trace/request_shape")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let mutation_intent = task_card
        .pointer("/routing_trace/mutation_intent")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();

    matches!(
        assigned_role.as_str(),
        "code specialist" | "implementation_specialist" | "implementation specialist"
    ) || matches!(assigned_agent.as_str(), "raider" | "ccc_raider")
        || matches!(request_shape.as_str(), "mutation" | "implementation")
        || matches!(mutation_intent.as_str(), "explicit_or_strong" | "weak")
        || task_kind == "execution"
            && task_card
                .get("mutation_intent")
                .and_then(Value::as_str)
                .is_some()
}

pub(crate) fn task_card_reviews_source(task_card: &Value, source_task_card_id: &str) -> bool {
    task_card
        .get("review_of_task_card_ids")
        .and_then(Value::as_array)
        .map(|source_ids| {
            source_ids
                .iter()
                .any(|source_id| source_id.as_str() == Some(source_task_card_id))
        })
        .unwrap_or(false)
}

pub(crate) fn review_task_card_for_source(
    run_directory: &Path,
    source_task_card_id: &str,
) -> io::Result<Option<Value>> {
    let task_cards_directory = run_directory.join("task-cards");
    let entries = match fs::read_dir(&task_cards_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let task_card = read_json_document(&path)?;
        if task_card_reviews_source(&task_card, source_task_card_id) {
            return Ok(Some(task_card));
        }
    }

    Ok(None)
}

fn review_task_card_exists(run_directory: &Path, source_task_card_id: &str) -> io::Result<bool> {
    Ok(review_task_card_for_source(run_directory, source_task_card_id)?.is_some())
}

pub(crate) fn review_task_card_has_passed_fan_in(task_card: &Value) -> bool {
    task_card
        .pointer("/review_fan_in/outcome")
        .and_then(Value::as_str)
        == Some("passed")
}

fn build_captain_owned_review_task_card(
    source_task_card: &Value,
    review_task_card_id: &str,
    timestamp: &str,
) -> io::Result<Value> {
    let run_id = source_task_card
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "source task card is missing run_id",
            )
        })?;
    let source_task_card_id = source_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "source task card is missing task_card_id",
            )
        })?;
    let source_title = source_task_card
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("source task");
    let source_scope = source_task_card
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("Review the source task output.");
    let source_acceptance = source_task_card
        .get("acceptance")
        .and_then(Value::as_str)
        .unwrap_or("Return a bounded review outcome to captain.");
    let title = format!("Review {source_title}");
    let intent = format!("Verify task-card {source_task_card_id} after child completion.");
    let execution_prompt = format!(
        "Review task-card {source_task_card_id} after child completion. Verify the source task output against acceptance and report a bounded review outcome with concise findings."
    );
    let mut task_card = build_task_card_payload_with_role(
        run_id,
        review_task_card_id,
        &title,
        &intent,
        source_scope,
        &execution_prompt,
        source_acceptance,
        "verifier",
        timestamp,
    );

    if let Some(object) = task_card.as_object_mut() {
        object.insert(
            "review_of_task_card_ids".to_string(),
            json!([source_task_card_id]),
        );
        object.insert("review_fan_in".to_string(), Value::Null);
    }

    Ok(task_card)
}

pub(crate) fn maybe_create_captain_owned_review_task_card(
    run_directory: &Path,
    source_task_card: &Value,
    timestamp: &str,
) -> io::Result<Option<Value>> {
    if task_card_is_review(source_task_card) {
        return Ok(None);
    }

    let review_policy = source_task_card
        .get("review_policy")
        .filter(|value| value.is_object())
        .unwrap_or(&Value::Null);
    let needs_mutation_follow_up = task_card_requires_mutation_review_follow_up(source_task_card)
        && !review_policy_suppresses_captain_review(review_policy);
    if !review_policy_requests_captain_review(review_policy) && !needs_mutation_follow_up {
        return Ok(None);
    }

    let source_task_card_id = source_task_card
        .get("task_card_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "source task card is missing task_card_id",
            )
        })?;
    if review_task_card_exists(run_directory, source_task_card_id)? {
        return Ok(None);
    }

    let review_task_card_id = generate_uuid_like_id();
    let review_task_card =
        build_captain_owned_review_task_card(source_task_card, &review_task_card_id, timestamp)?;
    write_json_document(
        &run_directory
            .join("task-cards")
            .join(format!("{review_task_card_id}.json")),
        &review_task_card,
    )?;

    let run_file = run_directory.join("run.json");
    let mut run_record = read_json_document(&run_file)?;
    let run_object = run_record
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "run.json must be an object."))?;
    let mut task_card_ids = run_object
        .get("task_card_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !task_card_ids
        .iter()
        .any(|value| value.as_str() == Some(review_task_card_id.as_str()))
    {
        task_card_ids.push(Value::String(review_task_card_id));
    }
    run_object.insert("task_card_ids".to_string(), Value::Array(task_card_ids));
    run_object.insert(
        "updated_at".to_string(),
        Value::String(timestamp.to_string()),
    );
    write_json_document(&run_file, &run_record)?;

    Ok(Some(review_task_card))
}

fn has_high_risk_review_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "high risk",
        "high-risk",
        "security",
        "auth",
        "permission",
        "payment",
        "data loss",
        "migration",
        "production",
        "breaking",
        "regression",
        "required review",
        "review required",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

fn has_failed_validation_review_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "failed validation",
        "validation failed",
        "failed verification",
        "verification failed",
        "acceptance failed",
        "failed acceptance",
        "unresolved acceptance",
        "acceptance unresolved",
        "tests failed",
        "test failed",
        "failing test",
        "failing validation",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

fn has_resource_limit_review_signal(request: &str) -> bool {
    let normalized = request.to_ascii_lowercase();
    [
        "resource limit",
        "resource-limit",
        "reviewer cap",
        "review cap",
        "budget cap",
        "review budget exhausted",
        "no reviewer budget",
        "suppress review",
        "skip review due to budget",
        "too many open files",
        "too many files open",
        "os error 24",
        "emfile",
        "file descriptor pressure",
        "file-descriptor pressure",
        "file descriptor exhaustion",
        "file-descriptor exhaustion",
        "file handle pressure",
        "file-handle pressure",
        "file handle exhaustion",
        "file-handle exhaustion",
        "open file limit",
        "open-file limit",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}
