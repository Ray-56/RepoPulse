# RepoPulse MCP integration (OpenClaw)

RepoPulse can run as an MCP server over stdio (JSON-RPC). This allows OpenClaw / agents to query RepoPulse as a trusted source of repo updates.

## Run RepoPulse as MCP server

```bash
export API_TOKEN="your-secret"
export DATABASE_URL="sqlite:./state.db"   # or sqlite:/data/state.db in docker
cargo run -- --mcp
```

RepoPulse will read JSON-RPC requests from STDIN and write responses to STDOUT.

## Tools
	•	health(token?)
	•	list_targets(token?)
	•	get_events(token?, since?, label?, type?, subject?, limit?)

If API_TOKEN is set, every tools/call must include token in arguments.

## Example: manual requests

list tools
```json
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

get events (last 24h, whatsapp label)
```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_events","arguments":{"token":"your-secret","since":"24h","label":"whatsapp","limit":50}}}
```

## OpenClaw usage idea

In OpenClaw, configure an MCP client that launches RepoPulse:
	•	command: repopulse
	•	args: ["--mcp"]
	•	env: API_TOKEN=..., DATABASE_URL=...

Then in chat, you can ask:
	•	“Show me repo updates in the last 24 hours”
	•	“Any release updates for whatsapp related repos?”