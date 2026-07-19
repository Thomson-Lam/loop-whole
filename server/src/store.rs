use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::{Component, Path},
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    // ponytail: session calls are expected to be serial; add per-key locks for concurrent agents.
    read_baselines: HashMap<ReadBaselineKey, ReadBaseline>,
    command_baselines: HashMap<CommandBaselineKey, CommandBaseline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReadBaselineKey {
    pub path: String,
    pub offset: usize,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ReadBaseline {
    pub text: String,
    pub view_hash: String,
    pub file_hash: String,
    pub was_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandBaselineKey {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Clone)]
pub struct CommandBaseline {
    pub exit_code: i32,
    pub raw_output_hash: String,
    pub canonical_text: String,
    pub canonical_hash: String,
    pub output_was_truncated: bool,
    pub adapter_kind: String,
}

#[derive(Debug, Clone)]
struct StoredToolCall {
    id: i64,
    call: NewToolCall,
}

const SESSION_FORMAT_VERSION: u32 = 1;
const TOKEN_COUNTER: &str = "chars_div_4_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSession {
    #[serde(default)]
    pub format_version: Option<u32>,
    pub session: PersistedSessionMeta,
    pub totals: TokenTotals,
    pub tool_calls: Vec<PersistedToolCall>,
    #[serde(default)]
    pub baselines: PersistedBaselines,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSessionMeta {
    pub id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub workspace_root: String,
    pub context_window_tokens: Option<u64>,
    pub token_counter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedBaselines {
    #[serde(default)]
    pub reads: Vec<PersistedReadBaseline>,
    #[serde(default)]
    pub commands: Vec<PersistedCommandBaseline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedReadBaseline {
    pub path: String,
    pub offset: usize,
    pub limit: Option<usize>,
    pub text: String,
    pub view_hash: String,
    pub file_hash: String,
    pub was_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedCommandBaseline {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub exit_code: i32,
    pub raw_output_hash: String,
    pub canonical_text: String,
    pub canonical_hash: String,
    pub output_was_truncated: bool,
    pub adapter_kind: String,
}

impl SessionStore {
    pub fn new(session: SessionSummary) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StoreData {
                session,
                next_id: 1,
                tool_calls: Vec::new(),
                read_baselines: HashMap::new(),
                command_baselines: HashMap::new(),
            })),
        }
    }

    pub fn load_from_path(
        path: &Path,
        expected_root: &Path,
        expected_session_id: &str,
        expected_context_window_tokens: Option<u64>,
    ) -> Result<(Self, u64)> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("failed to read session dump {}", path.display()))?;
        let persisted: PersistedSession = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse session dump {}", path.display()))?;

        if let Some(version) = persisted.format_version
            && version != SESSION_FORMAT_VERSION
        {
            bail!("unsupported session format version {version}");
        }
        if persisted.session.id != expected_session_id {
            bail!(
                "session dump ID {:?} does not match requested ID {:?}",
                persisted.session.id,
                expected_session_id
            );
        }
        if Path::new(&persisted.session.workspace_root) != expected_root {
            bail!(
                "session workspace {} does not match configured root {}",
                persisted.session.workspace_root,
                expected_root.display()
            );
        }
        if persisted.session.token_counter != TOKEN_COUNTER {
            bail!(
                "unsupported token counter {:?}",
                persisted.session.token_counter
            );
        }
        if let Some(expected) = expected_context_window_tokens
            && persisted.session.context_window_tokens != Some(expected)
        {
            bail!(
                "session context window {:?} does not match configured value {expected}",
                persisted.session.context_window_tokens
            );
        }

        let mut ids = HashSet::new();
        let mut sequences = HashSet::new();
        let mut max_id = 0_i64;
        let mut max_sequence = 0_u64;
        let mut tool_calls = Vec::with_capacity(persisted.tool_calls.len());
        for call in persisted.tool_calls {
            if call.id <= 0 || !ids.insert(call.id) {
                bail!(
                    "session dump contains an invalid or duplicate call ID {}",
                    call.id
                );
            }
            if call.sequence == 0 || !sequences.insert(call.sequence) {
                bail!(
                    "session dump contains an invalid or duplicate sequence {}",
                    call.sequence
                );
            }
            max_id = max_id.max(call.id);
            max_sequence = max_sequence.max(call.sequence);
            let input_text = serde_json::to_string(&call.input)?;
            tool_calls.push(StoredToolCall {
                id: call.id,
                call: NewToolCall {
                    sequence: call.sequence,
                    occurred_at_ms: call.occurred_at_ms,
                    tool_name: call.tool_name,
                    input: call.input,
                    subject_path: call.subject_path,
                    status: call.status,
                    duration_ms: call.duration_ms,
                    delivery_mode: call.delivery_mode,
                    decision_reason: call.decision_reason,
                    baseline_hash: call.baseline_hash,
                    current_hash: call.current_hash,
                    original_text: call.original.text,
                    intercepted_text: call.intercepted.text,
                    input_tokens: estimate_tokens(&input_text),
                    original_output_tokens: call.original.tokens,
                    intercepted_output_tokens: call.intercepted.tokens,
                    original_bytes: call.original.bytes,
                    intercepted_bytes: call.intercepted.bytes,
                },
            });
        }

        let mut read_baselines = HashMap::new();
        for baseline in persisted.baselines.reads {
            validate_baseline_path(&baseline.path, expected_root, "read path")?;
            let key = ReadBaselineKey {
                path: baseline.path,
                offset: baseline.offset,
                limit: baseline.limit,
            };
            let value = ReadBaseline {
                text: baseline.text,
                view_hash: baseline.view_hash,
                file_hash: baseline.file_hash,
                was_truncated: baseline.was_truncated,
            };
            if read_baselines.insert(key, value).is_some() {
                bail!("session dump contains a duplicate read baseline");
            }
        }

        let mut command_baselines = HashMap::new();
        for baseline in persisted.baselines.commands {
            validate_baseline_path(&baseline.cwd, expected_root, "command cwd")?;
            let key = CommandBaselineKey {
                program: baseline.program,
                args: baseline.args,
                cwd: baseline.cwd,
            };
            let value = CommandBaseline {
                exit_code: baseline.exit_code,
                raw_output_hash: baseline.raw_output_hash,
                canonical_text: baseline.canonical_text,
                canonical_hash: baseline.canonical_hash,
                output_was_truncated: baseline.output_was_truncated,
                adapter_kind: baseline.adapter_kind,
            };
            if command_baselines.insert(key, value).is_some() {
                bail!("session dump contains a duplicate command baseline");
            }
        }

        let next_id = max_id
            .checked_add(1)
            .context("session call ID space is exhausted")?;
        let next_sequence = max_sequence
            .checked_add(1)
            .context("session sequence space is exhausted")?;
        let session = SessionSummary {
            id: persisted.session.id,
            started_at_ms: persisted.session.started_at_ms,
            workspace_root: persisted.session.workspace_root,
            context_window_tokens: persisted.session.context_window_tokens,
            token_counter: persisted.session.token_counter,
        };

        Ok((
            Self {
                inner: Arc::new(RwLock::new(StoreData {
                    session,
                    next_id,
                    tool_calls,
                    read_baselines,
                    command_baselines,
                })),
            },
            next_sequence,
        ))
    }

    pub fn record(&self, call: NewToolCall) -> i64 {
        let mut store = self.inner.write().expect("session store lock poisoned");
        let id = store.next_id;
        store.next_id += 1;
        store.tool_calls.push(StoredToolCall { id, call });
        id
    }

    pub fn read_baseline(&self, key: &ReadBaselineKey) -> Option<ReadBaseline> {
        self.inner
            .read()
            .expect("session store lock poisoned")
            .read_baselines
            .get(key)
            .cloned()
    }

    pub fn set_read_baseline(&self, key: ReadBaselineKey, baseline: ReadBaseline) {
        self.inner
            .write()
            .expect("session store lock poisoned")
            .read_baselines
            .insert(key, baseline);
    }

    pub fn command_baseline(&self, key: &CommandBaselineKey) -> Option<CommandBaseline> {
        self.inner
            .read()
            .expect("session store lock poisoned")
            .command_baselines
            .get(key)
            .cloned()
    }

    pub fn set_command_baseline(&self, key: CommandBaselineKey, baseline: CommandBaseline) {
        self.inner
            .write()
            .expect("session store lock poisoned")
            .command_baselines
            .insert(key, baseline);
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
        let payload = self.persisted_session(ended_at_ms);
        let parent = path.parent().context("session dump path has no parent")?;
        std::fs::create_dir_all(parent)?;
        let resolved_parent = std::fs::canonicalize(parent)?;
        let workspace_root = Path::new(&payload.session.workspace_root);
        if !resolved_parent.starts_with(workspace_root) {
            bail!(
                "session dump directory {} resolves outside workspace {}",
                parent.display(),
                workspace_root.display()
            );
        }
        if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            bail!("session dump may not be a symlink: {}", path.display());
        }

        let file_name = path
            .file_name()
            .context("session dump path has no file name")?
            .to_string_lossy();
        let temp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;
            serde_json::to_writer_pretty(&mut file, &payload)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            std::fs::rename(&temp_path, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
        }
        result
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
        let mut reads = store
            .read_baselines
            .iter()
            .map(|(key, baseline)| PersistedReadBaseline {
                path: key.path.clone(),
                offset: key.offset,
                limit: key.limit,
                text: baseline.text.clone(),
                view_hash: baseline.view_hash.clone(),
                file_hash: baseline.file_hash.clone(),
                was_truncated: baseline.was_truncated,
            })
            .collect::<Vec<_>>();
        reads.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.offset.cmp(&right.offset))
                .then(left.limit.cmp(&right.limit))
        });
        let mut commands = store
            .command_baselines
            .iter()
            .map(|(key, baseline)| PersistedCommandBaseline {
                program: key.program.clone(),
                args: key.args.clone(),
                cwd: key.cwd.clone(),
                exit_code: baseline.exit_code,
                raw_output_hash: baseline.raw_output_hash.clone(),
                canonical_text: baseline.canonical_text.clone(),
                canonical_hash: baseline.canonical_hash.clone(),
                output_was_truncated: baseline.output_was_truncated,
                adapter_kind: baseline.adapter_kind.clone(),
            })
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| {
            left.program
                .cmp(&right.program)
                .then(left.args.cmp(&right.args))
                .then(left.cwd.cmp(&right.cwd))
        });

        PersistedSession {
            format_version: Some(SESSION_FORMAT_VERSION),
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
            baselines: PersistedBaselines { reads, commands },
        }
    }
}

fn validate_baseline_path(path: &str, expected_root: &Path, label: &str) -> Result<()> {
    let path = Path::new(path);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        || !path.starts_with(expected_root)
    {
        bail!(
            "session {label} {} is outside workspace {}",
            path.display(),
            expected_root.display()
        );
    }
    Ok(())
}

fn estimate_tokens(text: &str) -> u64 {
    text.chars().count().div_ceil(4) as u64
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

    #[test]
    fn keeps_runtime_baselines_in_session_memory() {
        let store = SessionStore::new(SessionSummary {
            id: "test".to_string(),
            started_at_ms: 1,
            workspace_root: "/tmp".to_string(),
            context_window_tokens: None,
            token_counter: "test".to_string(),
        });
        let key = ReadBaselineKey {
            path: "src/main.rs".to_string(),
            offset: 1,
            limit: Some(10),
        };
        store.set_read_baseline(
            key.clone(),
            ReadBaseline {
                text: "fn main() {}".to_string(),
                view_hash: "view".to_string(),
                file_hash: "file".to_string(),
                was_truncated: false,
            },
        );

        assert_eq!(store.read_baseline(&key).unwrap().view_hash, "view");
        assert!(store.snapshot().tool_calls.is_empty());
    }

    #[test]
    fn persists_and_restores_calls_counters_and_baselines() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let store = SessionStore::new(SessionSummary {
            id: "demo".to_string(),
            started_at_ms: 1,
            workspace_root: root.to_string_lossy().into_owned(),
            context_window_tokens: Some(200_000),
            token_counter: TOKEN_COUNTER.to_string(),
        });
        store.record(sample_call(3));

        let read_key = ReadBaselineKey {
            path: root.join("src/main.rs").to_string_lossy().into_owned(),
            offset: 1,
            limit: Some(500),
        };
        store.set_read_baseline(
            read_key.clone(),
            ReadBaseline {
                text: "fn main() {}".to_string(),
                view_hash: "view".to_string(),
                file_hash: "file".to_string(),
                was_truncated: false,
            },
        );
        let command_key = CommandBaselineKey {
            program: "cargo".to_string(),
            args: vec!["test".to_string()],
            cwd: root.to_string_lossy().into_owned(),
        };
        store.set_command_baseline(
            command_key.clone(),
            CommandBaseline {
                exit_code: 0,
                raw_output_hash: "raw".to_string(),
                canonical_text: "ok".to_string(),
                canonical_hash: "canonical".to_string(),
                output_was_truncated: false,
                adapter_kind: "cargo_test".to_string(),
            },
        );

        let path = root.join(".loopwhole/sessions/demo.json");
        store.persist_to_path(&path, 10).unwrap();
        let (loaded, next_sequence) =
            SessionStore::load_from_path(&path, &root, "demo", Some(200_000)).unwrap();

        assert_eq!(next_sequence, 4);
        assert_eq!(loaded.snapshot().tool_calls.len(), 1);
        assert_eq!(loaded.read_baseline(&read_key).unwrap().view_hash, "view");
        assert_eq!(
            loaded
                .command_baseline(&command_key)
                .unwrap()
                .canonical_text,
            "ok"
        );
        assert_eq!(loaded.record(sample_call(next_sequence)), 2);
        assert_eq!(loaded.tool_call(2).unwrap().sequence, 4);
    }

    #[test]
    fn loads_legacy_dump_without_runtime_baselines() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let store = SessionStore::new(SessionSummary {
            id: "legacy".to_string(),
            started_at_ms: 1,
            workspace_root: root.to_string_lossy().into_owned(),
            context_window_tokens: None,
            token_counter: TOKEN_COUNTER.to_string(),
        });
        store.record(sample_call(1));
        let path = root.join("legacy.json");
        store.persist_to_path(&path, 2).unwrap();

        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("formatVersion");
        json.as_object_mut().unwrap().remove("baselines");
        std::fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();

        let (loaded, next_sequence) =
            SessionStore::load_from_path(&path, &root, "legacy", None).unwrap();
        assert_eq!(next_sequence, 2);
        assert_eq!(loaded.snapshot().tool_calls.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_session_dump_symlink_escapes() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_path = outside.path().canonicalize().unwrap();
        let store = SessionStore::new(SessionSummary {
            id: "demo".to_string(),
            started_at_ms: 1,
            workspace_root: root_path.to_string_lossy().into_owned(),
            context_window_tokens: None,
            token_counter: TOKEN_COUNTER.to_string(),
        });

        symlink(&outside_path, root_path.join(".loopwhole")).unwrap();
        let dump = root_path.join(".loopwhole/sessions/demo.json");
        assert!(store.persist_to_path(&dump, 2).is_err());

        std::fs::remove_file(root_path.join(".loopwhole")).unwrap();
        std::fs::create_dir_all(dump.parent().unwrap()).unwrap();
        let outside_file = outside_path.join("outside.json");
        std::fs::write(&outside_file, "unchanged").unwrap();
        symlink(&outside_file, &dump).unwrap();
        assert!(store.persist_to_path(&dump, 2).is_err());
        assert_eq!(std::fs::read_to_string(outside_file).unwrap(), "unchanged");
    }

    fn sample_call(sequence: u64) -> NewToolCall {
        NewToolCall {
            sequence,
            occurred_at_ms: 2,
            tool_name: "read".to_string(),
            input: json!({"path": "a.rs"}),
            subject_path: Some("a.rs".to_string()),
            status: "success".to_string(),
            duration_ms: 1,
            delivery_mode: "full".to_string(),
            decision_reason: Some("no_read_baseline".to_string()),
            baseline_hash: None,
            current_hash: Some("hash".to_string()),
            original_text: "content".to_string(),
            intercepted_text: "content".to_string(),
            input_tokens: 4,
            original_output_tokens: 2,
            intercepted_output_tokens: 2,
            original_bytes: 7,
            intercepted_bytes: 7,
        }
    }
}
