use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use repopulse::application::usecases::{HandleEventUseCase, RunOnceUseCase};
use repopulse::infrastructure::{
    composite_provider::CompositeWatchProvider, console_notifier::ConsoleNotifier,
    feishu_notifier::FeishuNotifier, github_branch_provider::GitHubBranchProvider,
    github_release_provider::GitHubReleaseProvider, memory_store::InMemoryTargetRepository,
    multi_notifier::MultiNotifier, npm_latest_provider::NpmLatestProvider,
    sqlite_store::SqliteEventStore,
};
use repopulse::interfaces::{
    config::Config,
    http_api::{ApiState, build_router},
};

#[derive(Parser, Debug)]
#[command(name = "repopulse")]
struct Args {
    /// Path to config.yaml
    #[arg(long, default_value = "config.yaml")]
    config: String,

    /// Run once and exit
    #[arg(long)]
    once: bool,

    /// Do not send external notifications (console only)
    #[arg(long)]
    dry_run: bool,

    /// Enable HTTP API  service at address (e.g. 0.0.0.0:8080). If not set, HTTP is disabled.
    #[arg(long)]
    http_addr: Option<String>,

    /// Enable MCP server (stdio)
    #[arg(long)]
    mcp: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("repopulse=info".parse().unwrap()),
        )
        .init();
    if dotenvy::dotenv().is_err() {
        let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    }
    let args = Args::parse();
    let mut http_enabled = false;

    // 1) load config
    let cfg = match Config::load_from_file(&args.config) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load config {}: {}", args.config, e);
            std::process::exit(1);
        }
    };

    let targets = match cfg.to_watch_targets() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Invalid targets in config: {e}");
            std::process::exit(1);
        }
    };

    let poll_interval = cfg.poll_interval_seconds;

    // 2) build infra
    let target_repo = InMemoryTargetRepository::new(targets);
    let token = std::env::var("GITHUB_TOKEN").ok();
    let provider = CompositeWatchProvider::new(
        Box::new(GitHubReleaseProvider::new(token.clone())),
        Box::new(GitHubBranchProvider::new(token.clone())),
        Box::new(NpmLatestProvider::new()),
    );
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:/data/state.db".to_string());
    let store = SqliteEventStore::new(&db_url).await.expect("sqlite store");

    // notifiers fanout
    let mut notifiers: Vec<Box<dyn repopulse::application::Notifier>> = vec![];
    notifiers.push(Box::new(ConsoleNotifier::new()));

    if !args.dry_run {
        if let Ok(hook) = std::env::var("FEISHU_WEBHOOK") {
            notifiers.push(Box::new(FeishuNotifier::new(hook)));
        } else {
            tracing::warn!("FEISHU_WEBHOOK not set, FeishuNotifier disabled");
        }
    } else {
        tracing::warn!("--dry-run enabled: only console output");
    }

    let notifier = MultiNotifier::new(notifiers);
    let cooldown = cfg.cooldown_seconds.unwrap_or(0);

    let target_repo = Arc::new(target_repo);
    let store = Arc::new(store);

    // 3) usecases
    let handle_event = HandleEventUseCase {
        store: store.as_ref(),
        notifier: &notifier,
        cooldown_seconds: cooldown,
    };
    let run_once: RunOnceUseCase<'_> = RunOnceUseCase {
        targets: target_repo.as_ref(),
        provider: &provider,
        handle_event,
    };

    if let Some(addr) = args.http_addr.clone() {
        let state = ApiState {
            store: store.clone(),
            targets: target_repo.clone(),
        };
        let app = build_router(state);

        tracing::info!(%addr, "http api enabled");

        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .expect("bind http addr");
            axum::serve(listener, app).await.expect("http serve");
        });
        http_enabled = true;
    }

    // 4) run
    if args.mcp {
        tracing::info!("starting mcp server (stdio)");
        let server = repopulse::interfaces::mcp::McpServer {
            store: store.clone(),
            targets: target_repo.clone(),
        };
        if let Err(e) = server.serve().await {
            tracing::error!("mcp server error: {e}");
            std::process::exit(1);
        }
        return;
    }

    if args.once {
        if let Err(e) = run_once.execute().await {
            tracing::error!("RunOnce failed: {e}");
            std::process::exit(1);
        }
        tracing::info!("run once completed");

        if http_enabled {
            tracing::info!("http server is running; press Ctrl+C to exit");
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("Ctrl+C received, shutting down");
        }
        return;
    }

    tracing::info!(poll_interval = poll_interval, "polling started");
    loop {
        if let Err(e) = run_once.execute().await {
            tracing::error!("RunOnce failed: {e}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
    }
}
