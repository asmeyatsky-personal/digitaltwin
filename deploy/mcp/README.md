# MCP servers

Each Rust service in `backend/services/*` exposes an MCP server over
`POST /mcp` (JSON-RPC 2.0). Tools are write-side use cases; resources are
read-side projections. This matches `Architectural Rules — 2026.md` §3.5:
"One MCP server per bounded context."

## Registering with Claude Desktop

Copy `claude_desktop_config.json` into your Claude Desktop config directory
(`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS),
replacing `<HOST>` with your Cloud Run URL suffix (or `localhost:PORT` for
local dev).

## Methods supported

Every server implements the same JSON-RPC methods:

- `initialize` — protocol handshake
- `tools/list` — list the write-side tools
- `tools/call` — invoke a tool (see each service's `resources/list` output
  for the exact tool name + input schema)
- `resources/list` — list read-side resources
- `resources/read` — fetch a resource by URI

## Local dev

```sh
# With docker-compose Postgres up:
cd backend
IDENTITY_DATABASE_URL=... IDENTITY_JWT_PRIVATE_KEY_PEM=... ... cargo run -p identity-service
```

Then point Claude Desktop at `http://localhost:8080/mcp`.
