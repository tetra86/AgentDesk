# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is AgentDesk

AgentDesk is a single-binary AI agent orchestration platform written in Rust. It manages multiple AI agents (Claude, Codex) through Discord, orchestrates task dispatch via a Kanban board, and uses hot-reloadable JavaScript policies for business logic. All state lives in a single SQLite database.

## Documentation

When modifying code or investigating issues, refer to these documents first:

- **`ARCHITECTURE.md`** — Code navigation guide. Directory structure, call chains for common scenarios ("message not processing" → which files), API endpoint list, session lifecycle, DB schema.
- **`FEATURES.md`** — Feature specification. Kanban state flow, auto-queue, review automation, timeout rules, dispatch system, Discord commands, policy engine hooks and bridge functions.

## Build & Run

```bash
# Build (debug)
cargo build

# Build (release — optimized for size, LTO enabled)
cargo build --release

# Run the server (loads agentdesk.yaml from working dir or ~/.agentdesk/)
cargo run

# Dashboard (React frontend)
cd dashboard && npm install && npm run dev    # dev server
cd dashboard && npm run build                 # production build → dist/
```

## CLI Subcommands

The binary doubles as a CLI. Subcommands are parsed before the tokio runtime starts:

- `--dcserver [token]` — start the Discord bot server standalone
- `--init` / `--reconfigure` — interactive setup wizard
- `--doctor` — system diagnostics
- `--tmux-wrapper` / `--codex-tmux-wrapper` — process wrappers for agent providers
- `--discord-sendfile`, `--discord-sendmessage`, `--discord-senddm` — Discord message utilities
- `--restart-dcserver` — restart with crash report context

Default (no subcommand) starts the full HTTP server + policy engine + Discord gateway.

## Architecture

### Core Components (src/)

- **`main.rs`** — Entry point. CLI dispatch, then tokio runtime for the server.
- **`config.rs`** — Parses `agentdesk.yaml` (YAML config).
- **`db/`** — SQLite via rusqlite. Schema migrations in `schema.rs`. Shared as `Arc<Mutex<Connection>>`.
- **`server/`** — Axum HTTP server + WebSocket broadcast (`ws.rs`). 30+ route modules under `routes/`.
- **`engine/`** — QuickJS (rquickjs) policy engine:
  - `mod.rs` — JS runtime init, injects `agentdesk.*` global namespace
  - `ops.rs` — ~30 Rust↔JS bridge functions (db queries, Discord sends, kanban transitions, dispatch creation)
  - `loader.rs` — File watcher for hot-reloading `policies/*.js`
  - `hooks.rs` — Lifecycle hooks: `OnSessionStatusChange`, `OnCardTransition`, `OnDispatchCompleted`, `OnCardTerminal`, `OnTick`, `OnReviewEnter`, `OnReviewVerdict`
- **`dispatch/`** — Task dispatch creation, routing to agents, result handling.
- **`github/`** — GitHub issue sync (`sync.rs`), auto-triage (`triage.rs`), DoD mirroring (`dod.rs`). Uses `gh` CLI.
- **`services/discord/`** — Serenity/Poise Discord bot. Key files:
  - `router.rs` — Message routing, intake dedup, mention filtering
  - `turn_bridge.rs` — Agent turn lifecycle, heartbeat, watchdog
  - `tmux.rs` — Session output watcher, orphan cleanup, session lifecycle
  - `meeting.rs` — Round-table meetings
  - `recovery.rs` — Inflight turn recovery after restart
  - `commands/` — Slash commands (/start, /stop, /clear, /model, /skill, /diagnostics, /meeting, /help)
- **`services/`** — Provider integrations:
  - `claude.rs` / `codex.rs` — AI provider streaming execution
  - `session_backend.rs` — ProcessBackend: child process spawning, stdin pipe IPC
  - `tmux_common.rs` — Temp file paths, owner markers
  - `tmux_diagnostics.rs` — Exit reason tracking, death diagnostics
  - `tmux_wrapper.rs` / `codex_tmux_wrapper.rs` — Process execution wrappers
  - `provider.rs` — ProviderKind enum, session name construction
  - `platform/` — Cross-platform abstractions: binary resolver, shell commands

### Policies (policies/)

JavaScript files loaded by the QuickJS engine. Each exports a default object with lifecycle hook handlers. They are hot-reloaded on file change when `policies.hot_reload: true` in config.

Current policies: `kanban-rules.js`, `review-automation.js`, `auto-queue.js`, `pipeline.js`, `triage-rules.js`, `timeouts.js`

### Dashboard (dashboard/)

React 19 + TypeScript + Vite + Tailwind. Uses Pixi.js for the office visualization. Connects to the backend API at the same host.

## Key Patterns

- **AppState** — `(Db, PolicyEngine)` tuple passed to all Axum route handlers.
- **Db** — `Arc<Mutex<rusqlite::Connection>>` — lock before any query.
- **PolicyEngine** — Thread-safe wrapper around QuickJS. Call hooks via `engine.fire_hook(hook, payload)`.
- **SessionKey** — Format: `"hostname:session-name"` — uniquely identifies agent sessions.
- **ProviderKind** — Enum (`Claude` / `Codex`) for routing to the correct AI backend.
- **ProcessBackend** — Spawns agent CLI as child process with stdin pipe for input.
- **Config** is loaded once at startup from `agentdesk.yaml` (searched in CWD, then `~/.agentdesk/`).

## Environment Variables

- `AGENTDESK_TOKEN` — Auth token for the HTTP server
- `AGENTDESK_ROOT_DIR` — Override data directory (default: `~/.agentdesk`)
- `AGENTDESK_STATUS_INTERVAL_SECS` — Status polling interval (default: 5)
- `AGENTDESK_TURN_TIMEOUT_SECS` — Turn watchdog timeout (default: 3600)
- `RUST_LOG` — Standard tracing filter (default directive: `agentdesk=info`)

## Config

See `agentdesk.example.yaml` for the full config structure. Key sections: `server`, `discord.bots`, `agents`, `github`, `policies`, `data`, `kanban`.
