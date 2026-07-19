use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct NewToolCall {
    pub sequence: u64,
    pub occurred_at_ms: i64,
    pub tool_name: String,
    pub input: Value,
    pub subject_path: Option<String>,
    pub status: String,
    pub duration_ms: u64,
    pub delivery_mode: String,
    pub decision_reason: Option<String>,
    pub baseline_hash: Option<String>,
    pub current_hash: Option<String>,
    pub original_text: String,
    pub intercepted_text: String,
    pub input_tokens: u64,
    pub original_output_tokens: u64,
    pub intercepted_output_tokens: u64,
    pub original_bytes: u64,
    pub intercepted_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session: SessionSummary,
    pub totals: TokenTotals,
    pub tool_calls: Vec<ToolCallSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub started_at_ms: i64,
    pub workspace_root: String,
    pub context_window_tokens: Option<u64>,
    pub token_counter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenTotals {
    pub tool_input_tokens: u64,
    pub original_output_tokens: u64,
    pub intercepted_output_tokens: u64,
    pub without_runtime_tokens: u64,
    pub with_runtime_tokens: u64,
    pub saved_tokens: i64,
    pub savings_percent: f64,
    pub without_runtime_context_percent: Option<f64>,
    pub with_runtime_context_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallSummary {
    pub id: i64,
    pub sequence: u64,
    pub occurred_at_ms: i64,
    pub tool_name: String,
    pub subject_path: Option<String>,
    pub status: String,
    pub delivery_mode: String,
    pub input_tokens: u64,
    pub original_output_tokens: u64,
    pub intercepted_output_tokens: u64,
    pub saved_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallDetail {
    pub id: i64,
    pub sequence: u64,
    pub occurred_at_ms: i64,
    pub tool_name: String,
    pub subject_path: Option<String>,
    pub status: String,
    pub duration_ms: u64,
    pub input: Value,
    pub decision: DeliveryDecision,
    pub original: ToolPayload,
    pub intercepted: ToolPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryDecision {
    pub mode: String,
    pub reason: Option<String>,
    pub baseline_hash: Option<String>,
    pub current_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPayload {
    pub text: String,
    pub bytes: u64,
    pub tokens: u64,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReadRequest {
    #[schemars(description = "Path to the UTF-8 text file, relative to the workspace root")]
    pub path: String,
    #[schemars(description = "One-indexed line number to start reading from")]
    pub offset: Option<usize>,
    #[schemars(description = "Maximum number of lines to return")]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WriteRequest {
    #[schemars(description = "New path to create, relative to the workspace root")]
    pub path: String,
    #[schemars(description = "Complete UTF-8 content for the new file")]
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EditRequest {
    #[schemars(description = "Path to edit, relative to the workspace root")]
    pub path: String,
    #[schemars(description = "Exact text that must occur once in the current file")]
    pub old_text: String,
    #[schemars(description = "Replacement text")]
    pub new_text: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BashRequest {
    #[schemars(description = "Allowlisted executable name, without a path")]
    pub program: String,
    #[serde(default)]
    #[schemars(
        description = "Arguments passed directly to the executable; shell syntax is unsupported"
    )]
    pub args: Vec<String>,
    #[schemars(
        description = "Working directory relative to the workspace root; defaults to the root"
    )]
    pub cwd: Option<String>,
}
