use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    time::timeout,
};

use crate::schema::BashRequest;

const MAX_STREAM_BYTES: usize = 256 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const CAPTURE_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct CommandTools {
    root: Arc<PathBuf>,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub baseline_cwd: String,
    pub original_text: String,
    pub raw_output_hash: String,
    pub exit_code: Option<i32>,
    pub was_truncated: bool,
    pub timed_out: bool,
    stdout: String,
    stderr: String,
}

#[derive(Debug)]
pub struct CanonicalCommandOutput {
    pub adapter_kind: &'static str,
    pub text: String,
    pub hash: String,
}

impl CommandTools {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
        }
    }

    pub async fn run(&self, request: &BashRequest) -> Result<CommandOutput> {
        validate_allowlist(request)?;
        let cwd = self
            .resolve_cwd(request.cwd.as_deref().unwrap_or("."))
            .await?;
        let baseline_cwd = cwd.to_string_lossy().into_owned();
        let mut command = Command::new(&request.program);
        command
            .args(&request.args)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if request.program == "rg" {
            command.env_remove("RIPGREP_CONFIG_PATH");
        }
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .spawn()
            .with_context(|| format!("failed to start {}", request.program))?;
        let stdout = child.stdout.take().context("failed to capture stdout")?;
        let stderr = child.stderr.take().context("failed to capture stderr")?;
        let stdout_task = tokio::spawn(capture_stream(stdout));
        let stderr_task = tokio::spawn(capture_stream(stderr));

        let (status, timed_out, wait_error) = match timeout(COMMAND_TIMEOUT, child.wait()).await {
            Ok(Ok(status)) => (Some(status), false, None),
            Ok(Err(error)) => {
                kill_process_tree(&mut child).await;
                (None, false, Some(error))
            }
            Err(_) => {
                kill_process_tree(&mut child).await;
                let _ = child.wait().await;
                (None, true, None)
            }
        };
        let (stdout, stderr) = await_captures(stdout_task, stderr_task).await?;
        if let Some(error) = wait_error {
            return Err(error).context("failed to wait for command");
        }
        let exit_code = status.and_then(|status| status.code());
        let raw_output_hash = hash_parts(&[
            &stdout.hash,
            &stderr.hash,
            &format!("exit={exit_code:?};timeout={timed_out}"),
        ]);
        let stdout_text = stdout.render();
        let stderr_text = stderr.render();
        let was_truncated = stdout.was_truncated() || stderr.was_truncated();
        let original_text = format_result(&stdout_text, &stderr_text, exit_code, timed_out);

        Ok(CommandOutput {
            baseline_cwd,
            original_text,
            raw_output_hash,
            exit_code,
            was_truncated,
            timed_out,
            stdout: stdout_text,
            stderr: stderr_text,
        })
    }

    async fn resolve_cwd(&self, raw: &str) -> Result<PathBuf> {
        let raw = raw.strip_prefix('@').unwrap_or(raw);
        let candidate = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.root.join(raw)
        };
        let candidate = normalize(&candidate)?;
        let canonical = tokio::fs::canonicalize(&candidate)
            .await
            .with_context(|| format!("working directory does not exist: {raw}"))?;
        if !canonical.starts_with(self.root.as_path()) {
            bail!("working directory escapes workspace root: {raw}");
        }
        if !tokio::fs::metadata(&canonical).await?.is_dir() {
            bail!("working directory is not a directory: {raw}");
        }
        Ok(canonical)
    }
}

impl CommandOutput {
    pub fn completed(&self) -> bool {
        !self.timed_out && self.exit_code.is_some()
    }

    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out
    }
}

pub fn canonicalize(request: &BashRequest, output: &CommandOutput) -> CanonicalCommandOutput {
    let cleaned = format_result(
        &clean_output(&output.stdout),
        &clean_output(&output.stderr),
        output.exit_code,
        output.timed_out,
    );
    let (adapter_kind, text) = if is_cargo_test(request) && !output.was_truncated {
        match cargo_test_projection(&cleaned, output.exit_code) {
            Some(projected) => ("cargo_test", projected),
            None => ("generic", cleaned),
        }
    } else {
        ("generic", cleaned)
    };
    CanonicalCommandOutput {
        hash: hash_parts(&[&text]),
        adapter_kind,
        text,
    }
}

fn validate_allowlist(request: &BashRequest) -> Result<()> {
    if request.program.contains('/') || request.program.contains('\\') {
        bail!("program must be an allowlisted executable name without a path");
    }
    if request
        .args
        .iter()
        .any(|argument| argument_escapes_root(argument))
    {
        bail!("absolute paths and parent-directory arguments are not allowed");
    }

    let subcommand = request.args.first().map(String::as_str);
    let allowed = match request.program.as_str() {
        "cargo" => matches!(
            subcommand,
            Some("build" | "check" | "clippy" | "fmt" | "test")
        ),
        "npm" => match subcommand {
            Some("test") => true,
            Some("run") => matches!(
                request.args.get(1).map(String::as_str),
                Some("build" | "check" | "lint" | "test")
            ),
            _ => false,
        },
        "git" => {
            matches!(subcommand, Some("diff" | "log" | "show" | "status"))
                && !request.args.iter().any(|argument| {
                    argument == "--ext-diff"
                        || argument == "--textconv"
                        || argument == "--output"
                        || argument == "-o"
                        || argument.starts_with("--output=")
                })
        }
        "grep" => true,
        "rg" => !request.args.iter().any(|argument| {
            argument == "--pre"
                || argument.starts_with("--pre=")
                || argument == "--pre-glob"
                || argument.starts_with("--pre-glob=")
        }),
        _ => false,
    };
    if !allowed {
        bail!("command is not in the demo allowlist");
    }
    Ok(())
}

fn argument_escapes_root(argument: &str) -> bool {
    [
        Some(argument),
        argument.split_once('=').map(|(_, value)| value),
    ]
    .into_iter()
    .flatten()
    .any(|value| {
        Path::new(value).is_absolute()
            || value.starts_with("~/")
            || Path::new(value)
                .components()
                .any(|component| component == Component::ParentDir)
    })
}

#[derive(Debug)]
struct CapturedStream {
    head: Vec<u8>,
    tail: Vec<u8>,
    total_bytes: usize,
    hash: String,
}

impl CapturedStream {
    fn was_truncated(&self) -> bool {
        self.total_bytes > self.head.len() + self.tail.len()
    }

    fn render(&self) -> String {
        let head = String::from_utf8_lossy(&self.head);
        let tail = String::from_utf8_lossy(&self.tail);
        if !self.was_truncated() {
            return format!("{head}{tail}");
        }
        let omitted = self.total_bytes - self.head.len() - self.tail.len();
        format!("{head}\n\n[... {omitted} output bytes omitted ...]\n\n{tail}")
    }
}

async fn await_captures(
    mut stdout_task: tokio::task::JoinHandle<Result<CapturedStream>>,
    mut stderr_task: tokio::task::JoinHandle<Result<CapturedStream>>,
) -> Result<(CapturedStream, CapturedStream)> {
    match timeout(CAPTURE_DRAIN_TIMEOUT, async {
        let stdout = (&mut stdout_task)
            .await
            .context("stdout capture task failed")??;
        let stderr = (&mut stderr_task)
            .await
            .context("stderr capture task failed")??;
        Ok((stdout, stderr))
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            stdout_task.abort();
            stderr_task.abort();
            bail!("command output streams did not close after termination");
        }
    }
}

async fn kill_process_tree(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(id) = child.id() {
        // SAFETY: the child starts in a process group whose id equals its pid.
        unsafe {
            libc::kill(-(id as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill().await;
}

async fn capture_stream(mut stream: impl AsyncRead + Unpin) -> Result<CapturedStream> {
    let half = MAX_STREAM_BYTES / 2;
    let mut head = Vec::with_capacity(half);
    let mut tail = Vec::with_capacity(half);
    let mut total_bytes = 0;
    let mut hasher = DefaultHasher::new();
    let mut buffer = [0_u8; 8 * 1024];

    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let bytes = &buffer[..read];
        hasher.write(bytes);
        total_bytes += read;

        let head_remaining = half.saturating_sub(head.len());
        let to_head = head_remaining.min(bytes.len());
        head.extend_from_slice(&bytes[..to_head]);
        tail.extend_from_slice(&bytes[to_head..]);
        if tail.len() > half {
            tail.drain(..tail.len() - half);
        }
    }

    Ok(CapturedStream {
        head,
        tail,
        total_bytes,
        hash: format!("{:016x}", hasher.finish()),
    })
}

fn format_result(stdout: &str, stderr: &str, exit_code: Option<i32>, timed_out: bool) -> String {
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(stdout.trim_end());
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str("[stderr]\n");
        result.push_str(stderr.trim_end());
    }
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    if timed_out {
        result.push_str("[Command timed out after 120 seconds]");
    } else {
        result.push_str(&format!("[Exit code: {}]", exit_code.unwrap_or(-1)));
    }
    result
}

fn clean_output(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut cleaned = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'[') {
            index += 2;
            while index < bytes.len() {
                let byte = bytes[index];
                index += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
        } else if bytes[index] == b'\r' {
            cleaned.push(b'\n');
            index += 1;
        } else {
            cleaned.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&cleaned).into_owned()
}

fn is_cargo_test(request: &BashRequest) -> bool {
    request.program == "cargo" && request.args.first().is_some_and(|arg| arg == "test")
}

fn cargo_test_projection(text: &str, exit_code: Option<i32>) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let summaries: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.trim_start().starts_with("test result:"))
        .collect();
    if summaries.is_empty()
        || lines.iter().any(|line| {
            let line = line.trim_start();
            line.starts_with("warning:") || (line.starts_with("error:") && exit_code != Some(0))
        })
    {
        return None;
    }

    let mut projected = format!(
        "Cargo test: {}\n",
        if exit_code == Some(0) {
            "PASSED"
        } else {
            "FAILED"
        }
    );
    if exit_code != Some(0)
        && let Some(start) = lines.iter().position(|line| line.trim() == "failures:")
    {
        for line in &lines[start..] {
            if !line.starts_with("[Exit code:") {
                projected.push_str(line);
                projected.push('\n');
            }
        }
    } else {
        for summary in summaries {
            projected.push_str(summary.trim());
            projected.push('\n');
        }
    }
    projected.push_str(&format!("[Exit code: {}]", exit_code.unwrap_or(-1)));
    Some(projected)
}

fn hash_parts(parts: &[&str]) -> String {
    let mut hasher = DefaultHasher::new();
    for part in parts {
        hasher.write_usize(part.len());
        hasher.write(part.as_bytes());
    }
    format!("{:016x}", hasher.finish())
}

fn normalize(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    bail!("invalid parent path component");
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(program: &str, args: &[&str]) -> BashRequest {
        BashRequest {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            cwd: None,
        }
    }

    #[test]
    fn allowlist_accepts_demo_commands_and_rejects_shells() {
        assert!(validate_allowlist(&request("cargo", &["test"])).is_ok());
        assert!(validate_allowlist(&request("git", &["status"])).is_ok());
        assert!(validate_allowlist(&request("sudo", &["cargo", "test"])).is_err());
        assert!(validate_allowlist(&request("rm", &["-rf", "."])).is_err());
        assert!(validate_allowlist(&request("cargo", &["test", "../outside"])).is_err());
        assert!(
            validate_allowlist(&request(
                "cargo",
                &["test", "--manifest-path=/tmp/evil/Cargo.toml"]
            ))
            .is_err()
        );
        assert!(validate_allowlist(&request("git", &["diff", "--output=/tmp/diff"])).is_err());
        assert!(validate_allowlist(&request("git", &["diff", "--output=diff.txt"])).is_err());
        assert!(validate_allowlist(&request("rg", &["--pre=sh", "needle"])).is_err());
    }

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(clean_output("\u{1b}[31mfailed\u{1b}[0m"), "failed");
    }

    #[test]
    fn projects_successful_cargo_test_output() {
        let output = "running 1 test\ntest example ... ok\n\ntest result: ok. 1 passed; 0 failed\n\n[Exit code: 0]";
        let projection = cargo_test_projection(output, Some(0)).unwrap();
        assert!(projection.contains("Cargo test: PASSED"));
        assert!(projection.contains("1 passed; 0 failed"));
        assert!(!projection.contains("example ... ok"));
    }
}
