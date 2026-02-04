# Domain Model (v1)

## WatchTarget
用于描述“监控什么”。

Fields:
- id: string (stable identifier, e.g. "github:owner/repo:release")
- source: github / npm / whatsapp-web
- kind:
  - github_release { repo: RepoId }
  - github_branch { repo: RepoId, branch: string }
  - npm_latest { package: string }
  - whatsapp_web_version { } (v1 reserved)
- labels: string[] (e.g. ["whatsapp"])
- enabled: bool

## Event
一次“变化”被检测到后的事实记录

Fields:
- event_id: string (idempotency key)
- type: github_release | github_branch | npm_latest | whatsapp_web_version
- source: github | npm | whatsapp-web
- subject: string ("owner/repo" or "package")
- old_value: string | null
- new_value: string
- occurred_at: datetime (from upstream when possible; else datection time)
- detected_at: datetime (local)
- url: string | null
- meta: map<string, string> (optional)

Invariants:
- event_id must be stable for the same detected change
- event should be verifiable (url points to source of truth)

## Policy (v1)
### CooldownPolicy
Fields:
- cooldown_seconds: int
- scope:
  - by_target (same target)
  - by_target_and_type (same target + event type)