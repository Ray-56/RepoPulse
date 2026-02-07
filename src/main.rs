use clap::Parser;
use tracing_subscriber::EnvFilter;

use repopulse::application::usecases::{HandleEventUseCase, RunOnceUseCase};
use repopulse::infrastructure::{
    composite_provider::CompositeWatchProvider, console_notifier::ConsoleNotifier,
    feishu_notifier::FeishuNotifier, github_branch_provider::GitHubBranchProvider,
    github_release_provider::GitHubReleaseProvider, memory_store::InMemoryTargetRepository,
    multi_notifier::MultiNotifier, npm_latest_provider::NpmLatestProvider,
    sqlite_store::SqliteEventStore,
};
use repopulse::interfaces::config::Config;

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
    // let provider = GitHubReleaseProvider::new(std::env::var("GITHUB_TOKEN").ok());
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

    // 3) usecases
    let handle_event = HandleEventUseCase {
        store: &store,
        notifier: &notifier,
        cooldown_seconds: cooldown,
    };
    let run_once = RunOnceUseCase {
        targets: &target_repo,
        provider: &provider,
        handle_event,
    };

    // 4) run
    if args.once {
        if let Err(e) = run_once.execute().await {
            tracing::error!("RunOnce failed: {e}");
            std::process::exit(1);
        }
        tracing::info!("run once completed");
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
