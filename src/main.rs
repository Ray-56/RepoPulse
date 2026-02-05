use repopulse::application::usecases::{HandleEventUseCase, RunOnceUseCase};
use repopulse::domain::{RepoId, WatchKind, WatchTarget};
use repopulse::infrastructure::{
    console_notifier::ConsoleNotifier,
    fake_provider::FakeWatchProvider,
    memory_store::{InMemoryEventStore, InMemoryTargetRepository},
};

#[tokio::main]
async fn main() {
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
    let provider = FakeWatchProvider::new(); // 每次 check 返回固定 event (用于跑通)
    let store = InMemoryEventStore::new();
    let notifier = ConsoleNotifier::new();

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
