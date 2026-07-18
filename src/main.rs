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

    #[arg(long)]
    session_id: Option<String>,

    #[arg(long)]
    context_window_tokens: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let root = tokio::fs::canonicalize(&args.root)
        .await
        .with_context(|| format!("failed to resolve workspace root {}", args.root.display()))?;
    let session_id = args
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let log_path = root
        .join("logs")
        .join(format!("{}.log", safe_session_id(&session_id)));
    init_logging(&log_path)
        .with_context(|| format!("failed to initialize log file {}", log_path.display()))?;

    let store = SessionStore::new(SessionSummary {
        id: session_id.clone(),
        started_at_ms: now_ms(),
        workspace_root: root.to_string_lossy().into_owned(),
        context_window_tokens: args.context_window_tokens,
        token_counter: "chars_div_4_v1".to_string(),
    });

    let gateway_state = Arc::new(GatewayState {
        store: store.clone(),
        files: FileTools::new(root.clone()),
        commands: CommandTools::new(root.clone()),
        sequence: AtomicU64::new(1),
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

    let dump_path = root
        .join(".loopwhole")
        .join("sessions")
        .join(format!("{}.json", safe_session_id(&session_id)));
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

fn safe_session_id(session_id: &str) -> String {
    let mut safe = String::with_capacity(session_id.len());
    for ch in session_id.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            safe.push(ch);
        } else {
            safe.push('_');
        }
    }
    if safe.is_empty() {
        "session".to_string()
    } else {
        safe
    }
}
