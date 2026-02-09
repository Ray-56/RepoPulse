use tracing::{info, warn};

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

            let target_id = t.id.clone();

            match self.provider.check(&t).await {
                Ok(Some(event)) => {
                    info!(target_id = %target_id, event_id = %event.event_id, "event detected");
                    let labels = t.labels.clone();
                    if let Err(e) = self.handle_event.execute(&event, &target_id, &labels).await {
                        warn!(target_id = %target_id, error = %e, "handle event failed");
                    }
                }
                Ok(None) => {
                    // 正常：无变化
                }
                Err(e) => {
                    warn!(target_id = %target_id, error = %e, "provider check failed");
                }
            }
        }
        Ok(())
    }
}
