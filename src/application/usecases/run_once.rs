use crate::application::usecases::HandleEventUseCase;
use crate::application::{AppResult, TargetRepository, WatchProvider};

pub struct RunOnceUseCase<'a> {
    pub targets: &'a dyn TargetRepository,
    pub provider: &'a dyn WatchProvider,
    pub handle_event: HandleEventUseCase<'a>,
}

impl<'a> RunOnceUseCase<'a> {
    pub async fn execute(&self) -> AppResult<()> {
        let targets = self.targets.list_enabled_targets().await?;
        for t in targets {
            if !t.enabled {
                continue;
            }

            if let Some(event) = self.provider.check(&t).await? {
                self.handle_event.execute(&event, &t.id).await?;
            }
        }
        Ok(())
    }
}
