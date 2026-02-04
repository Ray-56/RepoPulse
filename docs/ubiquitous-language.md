# Ubiquitous Language (RepoPulse)

## Core Concepts

### Watch Target
一个需要被监控的对象。可能是：
- GitHub 仓库（release / branch）
- npm 包（latest version）
- WhatsApp Web（web version）

### Source
变化来源系统：github / npm / whatsapp-web

### Check
对某个 Watch Target 的一次检测动作，结果可能产生 0 或 1 个 Event。

### Event
一次“有意义的变化”的结构化记录（可追溯、可验证、可去重）。
示例：
- GitHub Release: v1.27.0 -> v1.28.0
- Branch Head: sha_old -> sha_new
- npm latest: 1.0.0 -> 1.1.0
- WhatsApp Web version: 2.240x -> 2.241x

### Event Id (Idempotency Key)
用于去重的唯一标识。通常由：
(event_type + subject + new_value) 计算得到。

### Subject
事件的主体标识：
- GitHub repo: "owner/repo"
- npm package: "package_name"
- WhatsApp Web: "whatsapp-web"

### Event Store
用于持久化事件与去重状态的存储（v1: SQLite）。

### Notification
把 Event 或 Digest 发送到外部渠道的动作。

### Notifier
具体的通知渠道实现（Feishu / Email / Telegram / ...）。

### Policy
决定“是否要通知”的规则集合（v1: cooldown）。

### Cooldown
冷却时间：同一类事件在一段时间内不重复通知（避免刷屏）。

### Digest
摘要通知：对一段时间的多个事件做聚合汇总（v1.5+）。