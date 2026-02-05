use repopulse::application::usecases::{HandleEventUseCase, RunOnceUseCase};
use repopulse::domain::{RepoId, WatchKind, WatchTarget};
use repopulse::infrastructure::feishu_notifier::FeishuNotifier;
use repopulse::infrastructure::github_release_provider::GitHubReleaseProvider;
use repopulse::infrastructure::multi_notifier::MultiNotifier;
use repopulse::infrastructure::sqlite_store::SqliteEventStore;
use repopulse::infrastructure::{
    console_notifier::ConsoleNotifier, memory_store::InMemoryTargetRepository,
};

#[tokio::main]
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
