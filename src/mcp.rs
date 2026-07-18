use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde::Serialize;
use serde_json::Value;

use crate::{
    schema::{NewToolCall, ReadRequest, WriteRequest},
    store::SessionStore,
    tools::FileTools,
};

#[derive(Debug)]
pub struct GatewayState {
    pub store: SessionStore,
    pub files: FileTools,
    pub sequence: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct Gateway {
    state: Arc<GatewayState>,
    tool_router: ToolRouter<Self>,
}

struct ToolOutcome {
    subject_path: Option<String>,
    status: &'static str,
    mode: &'static str,
    reason: &'static str,
    original: String,
    intercepted: String,
}

impl Gateway {
    pub fn new(state: Arc<GatewayState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    fn record<T: Serialize>(
        &self,
        tool_name: &str,
        input: &T,
        started: Instant,
        outcome: ToolOutcome,
    ) {
        let input = serde_json::to_value(input).unwrap_or(Value::Null);
        let input_text = serde_json::to_string(&input).unwrap_or_default();
        let call = NewToolCall {
            sequence: self.state.sequence.fetch_add(1, Ordering::Relaxed),
            occurred_at_ms: now_ms(),
            tool_name: tool_name.to_string(),
            input,
            subject_path: outcome.subject_path,
            status: outcome.status.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            delivery_mode: outcome.mode.to_string(),
            decision_reason: Some(outcome.reason.to_string()),
            baseline_hash: None,
            current_hash: None,
            input_tokens: estimate_tokens(&input_text),
            original_output_tokens: estimate_tokens(&outcome.original),
            intercepted_output_tokens: estimate_tokens(&outcome.intercepted),
            original_bytes: outcome.original.len() as u64,
            intercepted_bytes: outcome.intercepted.len() as u64,
            original_text: outcome.original,
            intercepted_text: outcome.intercepted,
        };
        self.state.store.record(call);
    }
}

#[tool_router(router = tool_router)]
impl Gateway {
    #[tool(
        description = "Read a UTF-8 text file inside the workspace. Output is limited to 2,000 lines or 50KB; use offset and limit to continue large files."
    )]
    async fn read(&self, Parameters(request): Parameters<ReadRequest>) -> CallToolResult {
        let started = Instant::now();
        match self
            .state
            .files
            .read(&request.path, request.offset, request.limit)
            .await
        {
            Ok(output) => {
                let text = output.text;
                self.record(
                    "read",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "success",
                        mode: "full",
                        reason: "state_optimization_not_enabled",
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::success(vec![ContentBlock::text(text)])
            }
            Err(error) => {
                let text = format!("Error reading {}: {error:#}", request.path);
                self.record(
                    "read",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "error",
                        mode: "error",
                        reason: "tool_execution_failed",
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::error(vec![ContentBlock::text(text)])
            }
        }
    }

    #[tool(
        description = "Write complete UTF-8 content to a file inside the workspace. Creates parent directories and overwrites an existing file."
    )]
    async fn write(&self, Parameters(request): Parameters<WriteRequest>) -> CallToolResult {
        let started = Instant::now();
        match self
            .state
            .files
            .write(&request.path, &request.content)
            .await
        {
            Ok(text) => {
                self.record(
                    "write",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "success",
                        mode: "passthrough",
                        reason: "state_optimization_not_enabled",
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::success(vec![ContentBlock::text(text)])
            }
            Err(error) => {
                let text = format!("Error writing {}: {error:#}", request.path);
                self.record(
                    "write",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "error",
                        mode: "error",
                        reason: "tool_execution_failed",
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::error(vec![ContentBlock::text(text)])
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Gateway {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Workspace-scoped read and write tools with observable tool results.",
        )
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn estimate_tokens(text: &str) -> u64 {
    text.chars().count().div_ceil(4) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimate_rounds_up() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }
}
