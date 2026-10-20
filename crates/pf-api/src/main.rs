//! `pf-api` — PackForge HTTP API server.
//!
//! Configuration (environment):
//! - `PF_BIND`          listen address (default `0.0.0.0:8080`)
//! - `PF_KEYS_FILE`     TOML file with hashed API keys (required unless `PF_DEV_NO_AUTH=1`)
//! - `PF_DEV_NO_AUTH`   `1` disables authentication — local development only
//! - `PF_CORS_ORIGINS`  comma-separated origins allowed to call the API from browsers
//! - `PF_MAX_BODY`      maximum request body in bytes (default 262144)
//! - `RUST_LOG`         log filter (default `info`)

use clap::{Parser, Subcommand};
use pf_api::keys::{generate_secret, sha256_hex, KeyStore};
use pf_api::{app, Settings};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "pf-api", version, about = "PackForge HTTP API server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the server (default).
    Serve,
    /// Create a new API key and print the secret plus the entry for the keys file.
    GenKey {
        /// Key id, e.g. "shop-prod"
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "enterprise")]
        plan: String,
        #[arg(long, default_value_t = 120)]
        rate_per_minute: u32,
        /// Secret prefix: pk_live or pk_test
        #[arg(long, default_value = "pk_live")]
        prefix: String,
    },
    /// Print the keys-file hash for an existing secret.
    HashKey { secret: String },
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn settings() -> Result<Settings, String> {
    let keys = if env("PF_DEV_NO_AUTH").as_deref() == Some("1") {
        tracing::warn!("PF_DEV_NO_AUTH=1: authentication is DISABLED");
        None
    } else {
        let path = env("PF_KEYS_FILE")
            .ok_or("PF_KEYS_FILE is not set (or set PF_DEV_NO_AUTH=1 for local development)")?;
        let src = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
        let store = KeyStore::from_toml(&src).map_err(|e| format!("{path}: {e}"))?;
        if store.is_empty() {
            return Err(format!("{path} contains no [[key]] entries"));
        }
        tracing::info!(keys = store.len(), "loaded API keys");
        Some(store)
    };
    let cors_origins = env("PF_CORS_ORIGINS")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let max_body_bytes = match env("PF_MAX_BODY") {
        Some(v) => v
            .parse()
            .map_err(|_| format!("PF_MAX_BODY must be a number, got '{v}'"))?,
        None => 256 * 1024,
    };
    Ok(Settings {
        keys,
        cors_origins,
        max_body_bytes,
    })
}

async fn serve() -> Result<(), String> {
    let settings = settings()?;
    let bind = env("PF_BIND").unwrap_or_else(|| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| format!("cannot bind {bind}: {e}"))?;
    tracing::info!(%bind, "pf-api listening");
    axum::serve(listener, app(settings))
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|e| e.to_string())
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutting down");
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let result = match Cli::parse().command.unwrap_or(Command::Serve) {
        Command::Serve => serve().await,
        Command::GenKey {
            id,
            plan,
            rate_per_minute,
            prefix,
        } => generate_secret(&prefix)
            .map(|secret| {
                println!("# Secret (give this to the client; it is not stored anywhere):");
                println!("# {secret}");
                println!();
                println!("[[key]]");
                println!("id = \"{id}\"");
                println!("hash = \"sha256:{}\"", sha256_hex(secret.as_bytes()));
                println!("plan = \"{plan}\"");
                println!("rate_per_minute = {rate_per_minute}");
            })
            .map_err(|e| format!("random number generator failed: {e}")),
        Command::HashKey { secret } => {
            println!("sha256:{}", sha256_hex(secret.as_bytes()));
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
