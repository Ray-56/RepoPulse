#[derive(Clone, Debug)]
pub struct CooldownPolicy {
    pub cooldown_seconds: u64,
    pub scope: CooldownScope,
}

#[derive(Clone, Debug)]
pub enum CooldownScope {
    ByTarget,        // same watch target
    ByTargetAndType, // same watch target + event type
}
