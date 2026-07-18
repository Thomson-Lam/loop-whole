use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use tokio::sync::Mutex;

const MAX_LINES: usize = 2_000;
const MAX_BYTES: usize = 50 * 1024;

#[derive(Debug, Clone)]
pub struct FileTools {
    root: Arc<PathBuf>,
    write_locks: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>,
}

#[derive(Debug)]
pub struct ReadOutput {
    pub text: String,
}

impl FileTools {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
            write_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn read(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<ReadOutput> {
        if matches!(offset, Some(0)) {
            bail!("offset must be one-indexed and greater than zero");
        }
        if matches!(limit, Some(0)) {
            bail!("limit must be greater than zero");
        }
        let absolute = self.resolve_existing(path).await?;
        let content = tokio::fs::read_to_string(&absolute)
            .await
            .with_context(|| format!("failed to read {}", display_path(path)))?;
        let all_lines: Vec<&str> = content.split('\n').collect();
        let start = offset.unwrap_or(1) - 1;
        if start >= all_lines.len() {
            bail!(
                "offset {} is beyond end of file ({} lines total)",
                offset.unwrap_or(1),
                all_lines.len()
            );
        }

        let end = limit
            .map(|limit| start.saturating_add(limit).min(all_lines.len()))
            .unwrap_or(all_lines.len());
        let selected = all_lines[start..end].join("\n");
        let truncated = truncate_head(&selected);
        let mut output = truncated.text;
        let first_line = start + 1;
        let last_line = first_line + truncated.lines.saturating_sub(1);

        if truncated.first_line_too_large {
            bail!("line {first_line} exceeds the {MAX_BYTES} byte read limit");
        }
        if truncated.was_truncated {
            output.push_str(&format!(
                "\n\n[Showing lines {first_line}-{last_line} of {}. Use offset={} to continue.]",
                all_lines.len(),
                last_line + 1
            ));
        } else if end < all_lines.len() {
            output.push_str(&format!(
                "\n\n[{} more lines in file. Use offset={} to continue.]",
                all_lines.len() - end,
                end + 1
            ));
        }

        Ok(ReadOutput { text: output })
    }

    pub async fn write(&self, path: &str, content: &str) -> Result<String> {
        let absolute = self.resolve_for_write(path).await?;
        let lock = {
            let mut locks = self.write_locks.lock().await;
            // ponytail: process-local locks are enough for one gateway process.
            locks
                .entry(absolute.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;
        let parent = absolute
            .parent()
            .context("write target has no parent directory")?;
        tokio::fs::create_dir_all(parent).await.with_context(|| {
            format!(
                "failed to create parent directory for {}",
                display_path(path)
            )
        })?;
        tokio::fs::write(&absolute, content)
            .await
            .with_context(|| format!("failed to write {}", display_path(path)))?;
        Ok(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            display_path(path)
        ))
    }

    async fn resolve_existing(&self, raw: &str) -> Result<PathBuf> {
        let candidate = self.lexical_candidate(raw)?;
        let canonical = tokio::fs::canonicalize(&candidate)
            .await
            .with_context(|| format!("path does not exist: {}", display_path(raw)))?;
        self.ensure_inside_root(&canonical, raw)?;
        Ok(canonical)
    }

    async fn resolve_for_write(&self, raw: &str) -> Result<PathBuf> {
        let candidate = self.lexical_candidate(raw)?;
        if tokio::fs::try_exists(&candidate).await? {
            let canonical = tokio::fs::canonicalize(&candidate).await?;
            self.ensure_inside_root(&canonical, raw)?;
            return Ok(canonical);
        }

        let mut existing = candidate.as_path();
        let mut missing = Vec::new();
        while !tokio::fs::try_exists(existing).await? {
            let name = existing
                .file_name()
                .context("could not resolve write target")?
                .to_os_string();
            missing.push(name);
            existing = existing
                .parent()
                .context("could not resolve write parent")?;
        }
        let mut canonical = tokio::fs::canonicalize(existing).await?;
        self.ensure_inside_root(&canonical, raw)?;
        for component in missing.into_iter().rev() {
            canonical.push(component);
        }
        Ok(canonical)
    }

    fn lexical_candidate(&self, raw: &str) -> Result<PathBuf> {
        let raw = raw.strip_prefix('@').unwrap_or(raw);
        if raw.trim().is_empty() {
            bail!("path must not be empty");
        }
        let path = Path::new(raw);
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let normalized = normalize(&joined)?;
        if !normalized.starts_with(self.root.as_path()) {
            bail!("path escapes workspace root: {}", display_path(raw));
        }
        Ok(normalized)
    }

    fn ensure_inside_root(&self, canonical: &Path, raw: &str) -> Result<()> {
        if !canonical.starts_with(self.root.as_path()) {
            bail!("path escapes workspace root: {}", display_path(raw));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Truncated {
    text: String,
    lines: usize,
    was_truncated: bool,
    first_line_too_large: bool,
}

fn truncate_head(content: &str) -> Truncated {
    if content.is_empty() {
        return Truncated {
            text: String::new(),
            lines: 0,
            was_truncated: false,
            first_line_too_large: false,
        };
    }
    let lines: Vec<&str> = content.split('\n').collect();
    if lines[0].len() > MAX_BYTES {
        return Truncated {
            text: String::new(),
            lines: 0,
            was_truncated: true,
            first_line_too_large: true,
        };
    }

    let mut selected = Vec::new();
    let mut bytes = 0;
    for line in &lines {
        if selected.len() == MAX_LINES {
            break;
        }
        let extra = line.len() + usize::from(!selected.is_empty());
        if bytes + extra > MAX_BYTES {
            break;
        }
        selected.push(*line);
        bytes += extra;
    }
    let selected_len = selected.len();
    Truncated {
        text: selected.join("\n"),
        lines: selected_len,
        was_truncated: selected_len < lines.len(),
        first_line_too_large: false,
    }
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

fn display_path(path: &str) -> &str {
    path.strip_prefix('@').unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_without_splitting_lines() {
        let content = (0..2_001).map(|_| "x").collect::<Vec<_>>().join("\n");
        let output = truncate_head(&content);
        assert!(output.was_truncated);
        assert_eq!(output.lines, 2_000);
        assert_eq!(output.text.lines().count(), 2_000);
    }

    #[tokio::test]
    async fn blocks_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "nope").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(outside.path(), root.path().join("escape")).unwrap();

        let root_path = tokio::fs::canonicalize(root.path()).await.unwrap();
        let tools = FileTools::new(root_path);
        assert!(tools.read("escape/secret", None, None).await.is_err());
        assert!(tools.write("escape/new", "nope").await.is_err());
    }
}
