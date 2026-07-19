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
use serde_json::{Value, json};
use similar::TextDiff;

const MAX_COMMAND_DIFF_BYTES: usize = 512 * 1024;

use crate::{
    commands::{CommandTools, canonicalize},
    logging::log_line,
    schema::{BashRequest, EditRequest, NewToolCall, ReadRequest, WriteRequest},
    store::{CommandBaseline, CommandBaselineKey, ReadBaseline, ReadBaselineKey, SessionStore},
    tools::FileTools,
};

#[derive(Debug)]
pub struct GatewayState {
    pub store: SessionStore,
    pub files: FileTools,
    pub commands: CommandTools,
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
    baseline_hash: Option<String>,
    current_hash: Option<String>,
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
            baseline_hash: outcome.baseline_hash,
            current_hash: outcome.current_hash,
            input_tokens: estimate_tokens(&input_text),
            original_output_tokens: estimate_tokens(&outcome.original),
            intercepted_output_tokens: estimate_tokens(&outcome.intercepted),
            original_bytes: outcome.original.len() as u64,
            intercepted_bytes: outcome.intercepted.len() as u64,
            original_text: outcome.original,
            intercepted_text: outcome.intercepted,
        };
        let mut log = tool_call_log(&call);
        let id = self.state.store.record(call);
        log["id"] = json!(id);
        log_line(log.to_string());
    }
}

#[tool_router(router = tool_router)]
impl Gateway {
    #[tool(
        description = "Read a UTF-8 text file inside the workspace. The first request returns the requested view; repeated identical requests return only changes or an unchanged marker. Output is limited to 2,000 lines or 50KB."
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
                let display_path = request.path.strip_prefix('@').unwrap_or(&request.path);
                let key = ReadBaselineKey {
                    path: output.baseline_path.clone(),
                    offset: request.offset.unwrap_or(1),
                    limit: request.limit,
                };
                let previous = self.state.store.read_baseline(&key);
                let original = output.text.clone();
                let (intercepted, mode, reason, baseline_hash) = match &previous {
                    Some(previous) if previous.view_hash == output.view_hash => {
                        let (message, reason) = if previous.file_hash == output.file_hash {
                            (
                                format!(
                                    "No changes in the requested view of {}.\nCurrent file hash: {}",
                                    display_path, output.file_hash
                                ),
                                "requested_view_unchanged",
                            )
                        } else {
                            (
                                format!(
                                    "No changes in the requested view of {}. Other portions of the file changed.\nCurrent file hash: {}",
                                    display_path, output.file_hash
                                ),
                                "requested_view_unchanged_file_changed",
                            )
                        };
                        (
                            message,
                            "unchanged",
                            reason,
                            Some(previous.file_hash.clone()),
                        )
                    }
                    Some(previous) => {
                        let diff = render_diff(&previous.text, &output.text);
                        let compact = format!(
                            "Changes in the requested view of {}:\n\n{}\nCurrent file hash: {}",
                            display_path, diff, output.file_hash
                        );
                        if estimate_tokens(&compact) < estimate_tokens(&original) {
                            let reason = if previous.was_truncated || output.was_truncated {
                                "partial_requested_view_changed"
                            } else {
                                "requested_view_changed"
                            };
                            (compact, "diff", reason, Some(previous.file_hash.clone()))
                        } else {
                            (
                                original.clone(),
                                "full",
                                "diff_not_smaller_than_current_view",
                                Some(previous.file_hash.clone()),
                            )
                        }
                    }
                    None => (original.clone(), "full", "no_read_baseline", None),
                };
                self.state.store.set_read_baseline(
                    key,
                    ReadBaseline {
                        text: output.text,
                        view_hash: output.view_hash,
                        file_hash: output.file_hash.clone(),
                        was_truncated: output.was_truncated,
                    },
                );
                let response = intercepted.clone();
                self.record(
                    "read",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "success",
                        mode,
                        reason,
                        baseline_hash,
                        current_hash: Some(output.file_hash),
                        original,
                        intercepted,
                    },
                );
                CallToolResult::success(vec![ContentBlock::text(response)])
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
                        baseline_hash: None,
                        current_hash: None,
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::error(vec![ContentBlock::text(text)])
            }
        }
    }

    #[tool(
        description = "Create a new UTF-8 file inside the workspace. This refuses to overwrite an existing file; use edit for existing content."
    )]
    async fn write(&self, Parameters(request): Parameters<WriteRequest>) -> CallToolResult {
        let started = Instant::now();
        match self
            .state
            .files
            .write(&request.path, &request.content)
            .await
        {
            Ok(output) => {
                self.record(
                    "write",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "success",
                        mode: "passthrough",
                        reason: "create_only_write",
                        baseline_hash: None,
                        current_hash: Some(output.current_hash),
                        original: output.text.clone(),
                        intercepted: output.text.clone(),
                    },
                );
                CallToolResult::success(vec![ContentBlock::text(output.text)])
            }
            Err(error) => {
                let text = format!("Error creating {}: {error:#}", request.path);
                self.record(
                    "write",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "error",
                        mode: "error",
                        reason: "tool_execution_failed",
                        baseline_hash: None,
                        current_hash: None,
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::error(vec![ContentBlock::text(text)])
            }
        }
    }

    #[tool(
        description = "Replace one exact, unique text occurrence in an existing UTF-8 file inside the workspace."
    )]
    async fn edit(&self, Parameters(request): Parameters<EditRequest>) -> CallToolResult {
        let started = Instant::now();
        match self
            .state
            .files
            .edit(&request.path, &request.old_text, &request.new_text)
            .await
        {
            Ok(output) => {
                self.record(
                    "edit",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "success",
                        mode: "passthrough",
                        reason: "exact_text_replaced",
                        baseline_hash: Some(output.baseline_hash),
                        current_hash: Some(output.current_hash),
                        original: output.text.clone(),
                        intercepted: output.text.clone(),
                    },
                );
                CallToolResult::success(vec![ContentBlock::text(output.text)])
            }
            Err(error) => {
                let text = format!("Error editing {}: {error:#}", request.path);
                self.record(
                    "edit",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.path.clone()),
                        status: "error",
                        mode: "error",
                        reason: "tool_execution_failed",
                        baseline_hash: None,
                        current_hash: None,
                        original: text.clone(),
                        intercepted: text.clone(),
                    },
                );
                CallToolResult::error(vec![ContentBlock::text(text)])
            }
        }
    }

    #[tool(
        description = "Run an allowlisted developer command without shell expansion. Supported command families include cargo build/check/clippy/fmt/test, selected npm scripts, read-only git commands, grep, and rg."
    )]
    async fn bash(&self, Parameters(request): Parameters<BashRequest>) -> CallToolResult {
        let started = Instant::now();
        match self.state.commands.run(&request).await {
            Ok(output) => {
                let canonical = canonicalize(&request, &output);
                let key = CommandBaselineKey {
                    program: request.program.clone(),
                    args: request.args.clone(),
                    cwd: output.baseline_cwd.clone(),
                };
                let previous = self.state.store.command_baseline(&key);
                let (intercepted, mode, reason, baseline_hash) = if !output.completed() {
                    (
                        canonical.text.clone(),
                        "error",
                        "command_did_not_complete",
                        None,
                    )
                } else {
                    match &previous {
                        Some(previous)
                            if previous.raw_output_hash == output.raw_output_hash
                                && previous.exit_code == output.exit_code.unwrap_or(-1) =>
                        {
                            (
                                format!(
                                    "Command returned the same output as last run.\n[Exit code: {}]",
                                    output.exit_code.unwrap_or(-1)
                                ),
                                "unchanged",
                                "command_output_unchanged",
                                Some(previous.raw_output_hash.clone()),
                            )
                        }
                        Some(previous)
                            if previous.adapter_kind == canonical.adapter_kind
                                && previous.canonical_hash == canonical.hash
                                && previous.exit_code == output.exit_code.unwrap_or(-1)
                                && !previous.output_was_truncated
                                && !output.was_truncated =>
                        {
                            (
                                format!(
                                    "Command had no relevant result changes after normalization.\n[Exit code: {}]",
                                    output.exit_code.unwrap_or(-1)
                                ),
                                "unchanged",
                                "canonical_command_output_unchanged",
                                Some(previous.raw_output_hash.clone()),
                            )
                        }
                        Some(previous) if previous.adapter_kind == canonical.adapter_kind => (
                            bound_text(
                                &format!(
                                    "Command output changes since the last run:\n\n{}",
                                    render_diff(&previous.canonical_text, &canonical.text)
                                ),
                                MAX_COMMAND_DIFF_BYTES,
                            ),
                            "diff",
                            "command_output_changed",
                            Some(previous.raw_output_hash.clone()),
                        ),
                        Some(previous) => (
                            canonical.text.clone(),
                            "compressed",
                            "command_adapter_changed",
                            Some(previous.raw_output_hash.clone()),
                        ),
                        None => (
                            canonical.text.clone(),
                            "compressed",
                            "no_command_baseline",
                            None,
                        ),
                    }
                };

                if output.completed() {
                    self.state.store.set_command_baseline(
                        key,
                        CommandBaseline {
                            exit_code: output.exit_code.unwrap_or(-1),
                            raw_output_hash: output.raw_output_hash.clone(),
                            canonical_text: canonical.text,
                            canonical_hash: canonical.hash,
                            output_was_truncated: output.was_truncated,
                            adapter_kind: canonical.adapter_kind.to_string(),
                        },
                    );
                }
                let completed = output.completed();
                let status = if output.succeeded() {
                    "success"
                } else {
                    "error"
                };
                let response = intercepted.clone();
                self.record(
                    "bash",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.cwd.clone().unwrap_or_else(|| ".".to_string())),
                        status,
                        mode,
                        reason,
                        baseline_hash,
                        current_hash: Some(output.raw_output_hash),
                        original: output.original_text,
                        intercepted,
                    },
                );
                if completed {
                    CallToolResult::success(vec![ContentBlock::text(response)])
                } else {
                    CallToolResult::error(vec![ContentBlock::text(response)])
                }
            }
            Err(error) => {
                let text = format!("Error running {}: {error:#}", request.program);
                self.record(
                    "bash",
                    &request,
                    started,
                    ToolOutcome {
                        subject_path: Some(request.cwd.clone().unwrap_or_else(|| ".".to_string())),
                        status: "error",
                        mode: "error",
                        reason: "command_execution_failed",
                        baseline_hash: None,
                        current_hash: None,
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
            "Workspace-scoped read, create, edit, and allowlisted command tools with observable context optimization.",
        )
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn tool_call_log(call: &NewToolCall) -> Value {
    let saved_tokens = call.original_output_tokens as i64 - call.intercepted_output_tokens as i64;
    let without_runtime_tokens = call.input_tokens + call.original_output_tokens;
    json!({
        "event": "tool_call",
        "sequence": call.sequence,
        "occurredAtMs": call.occurred_at_ms,
        "toolName": &call.tool_name,
        "subjectPath": call.subject_path.as_deref(),
        "status": &call.status,
        "durationMs": call.duration_ms,
        "deliveryMode": &call.delivery_mode,
        "decisionReason": call.decision_reason.as_deref(),
        "baselineHash": call.baseline_hash.as_deref(),
        "currentHash": call.current_hash.as_deref(),
        "inputTokens": call.input_tokens,
        "originalOutputTokens": call.original_output_tokens,
        "interceptedOutputTokens": call.intercepted_output_tokens,
        "savedTokens": saved_tokens,
        "contextSavingsPercent": percentage(saved_tokens, without_runtime_tokens),
        "outputSavingsPercent": percentage(saved_tokens, call.original_output_tokens),
        "originalBytes": call.original_bytes,
        "interceptedBytes": call.intercepted_bytes,
    })
}

fn percentage(saved: i64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        saved as f64 * 100.0 / total as f64
    }
}

fn render_diff(previous: &str, current: &str) -> String {
    TextDiff::from_lines(previous, current)
        .unified_diff()
        .header("previous", "current")
        .to_string()
}

fn bound_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let payload_bytes = max_bytes.saturating_sub(128);
    let mut head_end = payload_bytes / 2;
    while !text.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = text.len() - payload_bytes / 2;
    while !text.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let omitted = tail_start - head_end;
    format!(
        "{}\n\n[... {omitted} diff bytes omitted ...]\n\n{}",
        &text[..head_end],
        &text[tail_start..]
    )
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

    #[test]
    fn logs_per_call_token_reduction() {
        let call = NewToolCall {
            sequence: 1,
            occurred_at_ms: 2,
            tool_name: "read".to_string(),
            input: json!({"path": "example.rs"}),
            subject_path: Some("example.rs".to_string()),
            status: "success".to_string(),
            duration_ms: 3,
            delivery_mode: "unchanged".to_string(),
            decision_reason: Some("requested_view_unchanged".to_string()),
            baseline_hash: Some("old".to_string()),
            current_hash: Some("new".to_string()),
            original_text: "abcdefgh".to_string(),
            intercepted_text: "same".to_string(),
            input_tokens: 2,
            original_output_tokens: 8,
            intercepted_output_tokens: 2,
            original_bytes: 8,
            intercepted_bytes: 4,
        };

        let log = tool_call_log(&call);
        assert_eq!(log["savedTokens"], 6);
        assert_eq!(log["contextSavingsPercent"], 60.0);
        assert_eq!(log["outputSavingsPercent"], 75.0);
        assert!(log.get("originalText").is_none());
    }

    #[test]
    fn renders_line_diff() {
        let diff = render_diff("one\ntwo\n", "one\nthree\n");
        assert!(diff.contains("-two"));
        assert!(diff.contains("+three"));
    }

    #[test]
    fn bounds_large_command_diffs() {
        let bounded = bound_text(&"x".repeat(1_000), 256);
        assert!(bounded.len() <= 256);
        assert!(bounded.contains("diff bytes omitted"));
    }

    #[tokio::test]
    async fn compacts_repeated_read_views() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("example.rs");
        let original = (1..=100)
            .map(|line| format!("let value_{line} = {line};"))
            .collect::<Vec<_>>()
            .join("\n");
        tokio::fs::write(&path, &original).await.unwrap();
        let root_path = tokio::fs::canonicalize(root.path()).await.unwrap();
        let store = SessionStore::new(crate::schema::SessionSummary {
            id: "test".to_string(),
            started_at_ms: 1,
            workspace_root: root_path.to_string_lossy().into_owned(),
            context_window_tokens: None,
            token_counter: "test".to_string(),
        });
        let gateway = Gateway::new(Arc::new(GatewayState {
            store: store.clone(),
            files: FileTools::new(root_path.clone()),
            commands: CommandTools::new(root_path),
            sequence: AtomicU64::new(1),
        }));
        let request = || {
            Parameters(ReadRequest {
                path: "example.rs".to_string(),
                offset: None,
                limit: None,
            })
        };

        gateway.read(request()).await;
        let unchanged = gateway.read(request()).await;
        let unchanged = serde_json::to_value(unchanged).unwrap();
        assert!(
            unchanged["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("No changes")
        );

        tokio::fs::write(&path, original.replace("value_50 = 50", "value_50 = 500"))
            .await
            .unwrap();
        let changed = gateway.read(request()).await;
        let changed = serde_json::to_value(changed).unwrap();
        assert!(
            changed["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Changes in the requested view")
        );
        assert_eq!(
            store
                .snapshot()
                .tool_calls
                .into_iter()
                .map(|call| call.delivery_mode)
                .collect::<Vec<_>>(),
            ["full", "unchanged", "diff"]
        );
    }
}
