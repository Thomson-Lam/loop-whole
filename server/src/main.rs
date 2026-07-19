mod api;
mod commands;
mod logging;
mod mcp;
mod schema;
mod store;
mod tools;

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, atomic::AtomicU64},
};

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::ServiceExt;
use uuid::Uuid;

use crate::{
    api::ApiState,
    commands::CommandTools,
    logging::{init as init_logging, log_line},
    mcp::{Gateway, GatewayState, now_ms},
    schema::SessionSummary,
    store::SessionStore,
    tools::FileTools,
};

#[derive(Debug, Parser)]
#[command(about = "Workspace-scoped MCP read/write gateway with an observability API")]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[arg(long, default_value = "127.0.0.1:8787")]
    api_addr: SocketAddr,

    #[arg(long, conflicts_with = "resume_session")]
    session_id: Option<String>,

    #[arg(long, conflicts_with = "session_id")]
    resume_session: Option<String>,

    #[arg(long)]
    context_window_tokens: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let root = tokio::fs::canonicalize(&args.root)
        .await
        .with_context(|| format!("failed to resolve workspace root {}", args.root.display()))?;
    let resume_session = args.resume_session;
    let session_id = resume_session
        .clone()
        .or(args.session_id)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    validate_session_id(&session_id)?;
    let log_path = root.join("logs").join(format!("{session_id}.log"));
    let dump_path = root
        .join(".loopwhole")
        .join("sessions")
        .join(format!("{session_id}.json"));
    validate_output_path(&log_path, &root, "log file")?;
    init_logging(&log_path)
        .with_context(|| format!("failed to initialize log file {}", log_path.display()))?;

    let (store, next_sequence) = if resume_session.is_some() {
        validate_resume_path(&dump_path, &root)?;
        SessionStore::load_from_path(&dump_path, &root, &session_id, args.context_window_tokens)
            .with_context(|| format!("failed to resume session {session_id}"))?
    } else {
        (
            SessionStore::new(SessionSummary {
                id: session_id.clone(),
                started_at_ms: now_ms(),
                workspace_root: root.to_string_lossy().into_owned(),
                context_window_tokens: args.context_window_tokens,
                token_counter: "chars_div_4_v1".to_string(),
            }),
            1,
        )
    };

    let gateway_state = Arc::new(GatewayState {
        store: store.clone(),
        files: FileTools::new(root.clone()),
        commands: CommandTools::new(root.clone()),
        sequence: AtomicU64::new(next_sequence),
    });
    let api_state = Arc::new(ApiState {
        store: store.clone(),
    });

    let listener = tokio::net::TcpListener::bind(args.api_addr)
        .await
        .with_context(|| format!("failed to bind dashboard API to {}", args.api_addr))?;
    log_line(format!("dashboard API: http://{}", args.api_addr));
    log_line(format!("workspace root: {}", root.display()));
    log_line(format!("session: {session_id}"));
    if resume_session.is_some() {
        log_line(format!(
            "resumed {} prior tool calls; next sequence: {next_sequence}",
            store.snapshot().tool_calls.len()
        ));
    }
    log_line(format!("log file: {}", log_path.display()));

    let api_task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, api::router(api_state)).await {
            log_line(format!("dashboard API stopped: {error}"));
        }
    });

    let service = Gateway::new(gateway_state)
        .serve(rmcp::transport::stdio())
        .await
        .context("failed to start MCP stdio server")?;

    let shutdown_reason = tokio::select! {
        result = service.waiting() => {
            result.context("MCP server stopped with an error")?;
            "mcp_client_disconnected"
        }
        _ = shutdown_signal() => {
            log_line("shutdown signal received");
            "signal"
        }
    };

    if let Err(error) = store.persist_to_path(&dump_path, now_ms()) {
        log_line(format!(
            "failed to save session dump to {}: {error:#}",
            dump_path.display()
        ));
    } else {
        log_line(format!(
            "saved session dump ({shutdown_reason}): {}",
            dump_path.display()
        ));
    }

    api_task.abort();
    let _ = api_task.await;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn validate_session_id(session_id: &str) -> Result<()> {
    if session_id.is_empty()
        || !session_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        anyhow::bail!("session ID must contain only ASCII letters, numbers, '.', '_', or '-'");
    }
    Ok(())
}

fn validate_output_path(path: &std::path::Path, root: &std::path::Path, label: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{label} has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)?;
    let resolved_parent = std::fs::canonicalize(parent)?;
    if !resolved_parent.starts_with(root) {
        anyhow::bail!(
            "{label} directory {} resolves outside workspace {}",
            parent.display(),
            root.display()
        );
    }
    if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        anyhow::bail!("{label} may not be a symlink: {}", path.display());
    }
    Ok(())
}

fn validate_resume_path(path: &std::path::Path, root: &std::path::Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to find session dump {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        anyhow::bail!("session dump may not be a symlink: {}", path.display());
    }
    let resolved = std::fs::canonicalize(path)
        .with_context(|| format!("failed to resolve session dump {}", path.display()))?;
    if !resolved.starts_with(root) {
        anyhow::bail!(
            "session dump {} resolves outside workspace {}",
            path.display(),
            root.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_session_id;

    #[test]
    fn accepts_only_filename_safe_session_ids() {
        assert!(validate_session_id("pitch-demo_1.0").is_ok());
        assert!(validate_session_id("").is_err());
        assert!(validate_session_id("pitch/demo").is_err());
        assert!(validate_session_id("pitch demo").is_err());
    }
}
