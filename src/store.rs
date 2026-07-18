use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use serde::Serialize;

use crate::schema::{
    DeliveryDecision, NewToolCall, SessionSnapshot, SessionSummary, TokenTotals, ToolCallDetail,
    ToolCallSummary, ToolPayload,
};

#[derive(Debug, Clone)]
pub struct SessionStore {
    inner: Arc<RwLock<StoreData>>,
}

#[derive(Debug)]
struct StoreData {
    session: SessionSummary,
    next_id: i64,
    tool_calls: Vec<StoredToolCall>,
}

#[derive(Debug, Clone)]
struct StoredToolCall {
    id: i64,
    call: NewToolCall,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSession {
    pub session: PersistedSessionMeta,
    pub totals: TokenTotals,
    pub tool_calls: Vec<PersistedToolCall>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSessionMeta {
    pub id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub workspace_root: String,
    pub context_window_tokens: Option<u64>,
    pub token_counter: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedToolCall {
    pub id: i64,
    pub sequence: u64,
    pub occurred_at_ms: i64,
    pub tool_name: String,
    pub subject_path: Option<String>,
    pub status: String,
    pub duration_ms: u64,
    pub delivery_mode: String,
    pub decision_reason: Option<String>,
    pub baseline_hash: Option<String>,
    pub current_hash: Option<String>,
    pub input: serde_json::Value,
    pub original: ToolPayload,
    pub intercepted: ToolPayload,
}

impl SessionStore {
    pub fn new(session: SessionSummary) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StoreData {
                session,
                next_id: 1,
                tool_calls: Vec::new(),
            })),
        }
    }

    pub fn record(&self, call: NewToolCall) -> i64 {
        let mut store = self.inner.write().expect("session store lock poisoned");
        let id = store.next_id;
        store.next_id += 1;
        store.tool_calls.push(StoredToolCall { id, call });
        id
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let store = self.inner.read().expect("session store lock poisoned");
        let input = store
            .tool_calls
            .iter()
            .map(|stored| stored.call.input_tokens)
            .sum::<u64>();
        let original = store
            .tool_calls
            .iter()
            .map(|stored| stored.call.original_output_tokens)
            .sum::<u64>();
        let intercepted = store
            .tool_calls
            .iter()
            .map(|stored| stored.call.intercepted_output_tokens)
            .sum::<u64>();
        let without_runtime = input + original;
        let with_runtime = input + intercepted;
        let saved = without_runtime as i64 - with_runtime as i64;
        let context_window = store.session.context_window_tokens;

        SessionSnapshot {
            session: store.session.clone(),
            totals: TokenTotals {
                tool_input_tokens: input,
                original_output_tokens: original,
                intercepted_output_tokens: intercepted,
                without_runtime_tokens: without_runtime,
                with_runtime_tokens: with_runtime,
                saved_tokens: saved,
                savings_percent: percent(saved, without_runtime),
                without_runtime_context_percent: context_window
                    .map(|window| ratio(without_runtime, window)),
                with_runtime_context_percent: context_window
                    .map(|window| ratio(with_runtime, window)),
            },
            tool_calls: store
                .tool_calls
                .iter()
                .map(|stored| ToolCallSummary {
                    id: stored.id,
                    sequence: stored.call.sequence,
                    occurred_at_ms: stored.call.occurred_at_ms,
                    tool_name: stored.call.tool_name.clone(),
                    subject_path: stored.call.subject_path.clone(),
                    status: stored.call.status.clone(),
                    delivery_mode: stored.call.delivery_mode.clone(),
                    input_tokens: stored.call.input_tokens,
                    original_output_tokens: stored.call.original_output_tokens,
                    intercepted_output_tokens: stored.call.intercepted_output_tokens,
                    saved_tokens: stored.call.original_output_tokens as i64
                        - stored.call.intercepted_output_tokens as i64,
                })
                .collect(),
        }
    }

    pub fn persist_to_path(&self, path: &Path, ended_at_ms: i64) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let payload = self.persisted_session(ended_at_ms);
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &payload)?;
        Ok(())
    }

    pub fn tool_call(&self, id: i64) -> Option<ToolCallDetail> {
        let store = self.inner.read().expect("session store lock poisoned");
        let stored = store.tool_calls.iter().find(|stored| stored.id == id)?;
        let call = &stored.call;
        Some(ToolCallDetail {
            id: stored.id,
            sequence: call.sequence,
            occurred_at_ms: call.occurred_at_ms,
            tool_name: call.tool_name.clone(),
            subject_path: call.subject_path.clone(),
            status: call.status.clone(),
            duration_ms: call.duration_ms,
            input: call.input.clone(),
            decision: DeliveryDecision {
                mode: call.delivery_mode.clone(),
                reason: call.decision_reason.clone(),
                baseline_hash: call.baseline_hash.clone(),
                current_hash: call.current_hash.clone(),
            },
            original: ToolPayload {
                text: call.original_text.clone(),
                bytes: call.original_bytes,
                tokens: call.original_output_tokens,
            },
            intercepted: ToolPayload {
                text: call.intercepted_text.clone(),
                bytes: call.intercepted_bytes,
                tokens: call.intercepted_output_tokens,
            },
        })
    }

    fn persisted_session(&self, ended_at_ms: i64) -> PersistedSession {
        let store = self.inner.read().expect("session store lock poisoned");
        let input = store
            .tool_calls
            .iter()
            .map(|stored| stored.call.input_tokens)
            .sum::<u64>();
        let original = store
            .tool_calls
            .iter()
            .map(|stored| stored.call.original_output_tokens)
            .sum::<u64>();
        let intercepted = store
            .tool_calls
            .iter()
            .map(|stored| stored.call.intercepted_output_tokens)
            .sum::<u64>();
        let without_runtime = input + original;
        let with_runtime = input + intercepted;
        let saved = without_runtime as i64 - with_runtime as i64;
        let context_window = store.session.context_window_tokens;
        PersistedSession {
            session: PersistedSessionMeta {
                id: store.session.id.clone(),
                started_at_ms: store.session.started_at_ms,
                ended_at_ms,
                workspace_root: store.session.workspace_root.clone(),
                context_window_tokens: context_window,
                token_counter: store.session.token_counter.clone(),
            },
            totals: TokenTotals {
                tool_input_tokens: input,
                original_output_tokens: original,
                intercepted_output_tokens: intercepted,
                without_runtime_tokens: without_runtime,
                with_runtime_tokens: with_runtime,
                saved_tokens: saved,
                savings_percent: percent(saved, without_runtime),
                without_runtime_context_percent: context_window
                    .map(|window| ratio(without_runtime, window)),
                with_runtime_context_percent: context_window
                    .map(|window| ratio(with_runtime, window)),
            },
            tool_calls: store
                .tool_calls
                .iter()
                .map(|stored| PersistedToolCall {
                    id: stored.id,
                    sequence: stored.call.sequence,
                    occurred_at_ms: stored.call.occurred_at_ms,
                    tool_name: stored.call.tool_name.clone(),
                    subject_path: stored.call.subject_path.clone(),
                    status: stored.call.status.clone(),
                    duration_ms: stored.call.duration_ms,
                    delivery_mode: stored.call.delivery_mode.clone(),
                    decision_reason: stored.call.decision_reason.clone(),
                    baseline_hash: stored.call.baseline_hash.clone(),
                    current_hash: stored.call.current_hash.clone(),
                    input: stored.call.input.clone(),
                    original: ToolPayload {
                        text: stored.call.original_text.clone(),
                        bytes: stored.call.original_bytes,
                        tokens: stored.call.original_output_tokens,
                    },
                    intercepted: ToolPayload {
                        text: stored.call.intercepted_text.clone(),
                        bytes: stored.call.intercepted_bytes,
                        tokens: stored.call.intercepted_output_tokens,
                    },
                })
                .collect(),
        }
    }
}

fn percent(saved: i64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        saved as f64 * 100.0 / total as f64
    }
}

fn ratio(tokens: u64, context_window: u64) -> f64 {
    if context_window == 0 {
        0.0
    } else {
        tokens as f64 * 100.0 / context_window as f64
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn records_and_summarizes_calls() {
        let store = SessionStore::new(SessionSummary {
            id: "test".to_string(),
            started_at_ms: 1,
            workspace_root: "/tmp".to_string(),
            context_window_tokens: Some(100),
            token_counter: "test".to_string(),
        });
        store.record(NewToolCall {
            sequence: 1,
            occurred_at_ms: 2,
            tool_name: "read".to_string(),
            input: json!({"path": "a.rs"}),
            subject_path: Some("a.rs".to_string()),
            status: "success".to_string(),
            duration_ms: 1,
            delivery_mode: "unchanged".to_string(),
            decision_reason: None,
            baseline_hash: None,
            current_hash: None,
            original_text: "12345678".to_string(),
            intercepted_text: "1234".to_string(),
            input_tokens: 2,
            original_output_tokens: 2,
            intercepted_output_tokens: 1,
            original_bytes: 8,
            intercepted_bytes: 4,
        });

        let snapshot = store.snapshot();
        assert_eq!(snapshot.tool_calls.len(), 1);
        assert_eq!(snapshot.totals.saved_tokens, 1);
        assert_eq!(store.tool_call(1).unwrap().original.text, "12345678");
    }
}
