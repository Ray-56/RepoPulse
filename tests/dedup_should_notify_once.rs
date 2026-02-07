use repopulse::application::usecases::{HandleEventUseCase, RunOnceUseCase};
use repopulse::application::{AppResult, Notifier};
use repopulse::domain::Event;
use repopulse::domain::{RepoId, WatchKind, WatchTarget};
use repopulse::infrastructure::{
    fake_provider::FakeWatchProvider,
    memory_store::{InMemoryEventStore, InMemoryTargetRepository},
};

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct CountingNotifier {
    count: Arc<Mutex<u32>>,
}

impl CountingNotifier {
    fn new() -> Self {
        Self::default()
    }
    fn get(&self) -> u32 {
        *self.count.lock().unwrap()
    }
}

#[async_trait]
impl Notifier for CountingNotifier {
    async fn notify(&self, _event: &Event) -> AppResult<()> {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        Ok(())
    }
}

#[tokio::test]
async fn should_notify_only_once_for_same_event() {
    let repo = RepoId::parse("pedroslopez/whatsapp-web.js").unwrap();
    let targets = vec![WatchTarget {
        id: "github:pedroslopez/whatsapp-web.js:release".to_string(),
        enabled: true,
        labels: vec![],
        kind: WatchKind::GitHubRelease { repo },
    }];

    let target_repo = InMemoryTargetRepository::new(targets);
    let provider = FakeWatchProvider::new();
    let store = InMemoryEventStore::new();
    let notifier = CountingNotifier::new();

    let handle_event = HandleEventUseCase {
        store: &store,
        notifier: &notifier,
        cooldown_seconds: 0,
    };
    let run_once = RunOnceUseCase {
        targets: &target_repo,
        provider: &provider,
        handle_event,
    };

    // 第一次执行 通知 1 次
    run_once.execute().await.unwrap();
    // 第二次执行 同一个 fake event（event_id 相同） 不通知
    run_once.execute().await.unwrap();

    assert_eq!(notifier.get(), 1);
}
