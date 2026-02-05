# RepoPulse
RepoPulse monitors meaningful changes across GitHub repositories and dependencies, and delivers reliable signals to humans and AI agents.

## Run with Docker

1. Create `.env`:

- `GITHUB_TOKEN`: GitHub personal access token
- `FEISHU_WEBHOOK`: Feishu webhook URL

2. Start:

```bash
mkdir -p data
docker compose up -d build
```
