use clap::Parser;

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
    if dotenvy::dotenv().is_err() {
        let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    }
    let args = Args::parse();

    // 1) load config
    let cfg = match Config::load_from_file(&args.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config {}: {e}", args.config);
            std::process::exit(1);
        }
    };

    let targets = match cfg.to_watch_targets() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Invalid targets in config: {e}");
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
            eprintln!("FEISHU_WEBHOOK not set, FeishuNotifier disabled");
        }
    } else {
        eprintln!("--dry-run enabled: only console output");
    }

    let notifier = MultiNotifier::new(notifiers);

    // 3) usecases
    let handle_event = HandleEventUseCase {
        store: &store,
        notifier: &notifier,
    };
    let run_once = RunOnceUseCase {
        targets: &target_repo,
        provider: &provider,
        handle_event,
    };

    // 4) run
    if args.once {
        if let Err(e) = run_once.execute().await {
            eprintln!("RunOnce failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    loop {
        if let Err(e) = run_once.execute().await {
            eprintln!("RunOnce failed: {e}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
    }
}

/* #[tokio::main]
async fn main() {
    // Load `.env` into process env (best-effort).
    // - first try current working directory
    // - then fallback to project root (useful when running from subdirs)
    if dotenvy::dotenv().is_err() {
        let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    }

    // 1) 准备 targets(之后会来自 config/DB)
    let repo = RepoId::parse("pedroslopez/whatsapp-web.js").expect("valid repo");
    let targets = vec![WatchTarget {
        id: "github:pedroslopez/whatsapp-web.js:release".to_string(),
        enabled: true,
        labels: vec!["whatsapp".to_string()],
        kind: WatchKind::GitHubRelease { repo },
    }];

    // 2) 组装依赖 (infrastructure)
    let target_repo = InMemoryTargetRepository::new(targets);
    // let provider = FakeWatchProvider::new(); // 每次 check 返回固定 event (用于跑通)
    let provider = GitHubReleaseProvider::new(std::env::var("GITHUB_TOKEN").ok());
    // let store = InMemoryEventStore::new();
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:/data/state.db".to_string());

    let store = SqliteEventStore::new(&db_url).await.expect("sqlite store");
    // let notifier = ConsoleNotifier::new();

    let feishu_webhook = std::env::var("FEISHU_WEBHOOK").ok();
    let mut notifiers: Vec<Box<dyn repopulse::application::Notifier>> = vec![];
    notifiers.push(Box::new(ConsoleNotifier::new()));
    if let Some(hook) = feishu_webhook {
        notifiers.push(Box::new(FeishuNotifier::new(hook)));
    } else {
        eprintln!("FEISHU_WEBHOOK not set, skipping Feishu notification");
    }
    let notifier = MultiNotifier::new(notifiers);

    // 3) 组装用例 (application)
    let handle_event = HandleEventUseCase {
        store: &store,
        notifier: &notifier,
    };
    let run_once = RunOnceUseCase {
        targets: &target_repo,
        provider: &provider,
        handle_event,
    };

    // 4) 执行一次
    if let Err(e) = run_once.execute().await {
        eprintln!("RunOnce failed: {e}");
    }
}
 */
