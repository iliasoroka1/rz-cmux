# rz-cmux

**Run a swarm of AI agents inside your terminal — all talking to each other natively.**

rz-cmux gives every [cmux](https://github.com/manaflow-ai/cmux) terminal session a native inter-agent messaging layer. Spawn Claude (or any agent) into a split, send it work, read its output, wait for replies, broadcast to the whole swarm — all from a single `rz` binary that any agent can call itself.

No servers. No sidecars. No configuration. If you're inside cmux, it just works.

**Fork of [rz](https://github.com/HodlOg/rz)** ([crates.io](https://crates.io/crates/rz-cli)) by [@HodlOg](https://github.com/HodlOg) — the original Zellij inter-agent messaging tool. This fork replaces the Zellij transport with cmux's v2 JSON-RPC socket API and adds cmux-native features: browser automation, notifications, and workspace management. The `@@RZ:` wire protocol is unchanged — agents communicate identically regardless of multiplexer.

---

## Why rz-cmux

Modern AI coding workflows involve more than one agent. You might have a researcher, a coder, a reviewer, and a QA agent all running in parallel. Without a messaging layer they're blind to each other — duplicating work, stepping on each other's files, unable to hand off tasks.

rz-cmux solves this with one small binary:

- **Agents can spawn agents.** A lead agent can delegate sub-tasks to helpers, wait for their replies, and report back — all autonomously.
- **Structured messaging with threading.** Every message has an ID. Agents reply to specific messages with `--ref`, building proper conversation threads across splits.
- **Shared workspace.** A `goals.md`, `agents.md`, and `context.md` give the whole swarm a shared memory. Agents write discoveries, claim files, and coordinate without collision.
- **Full situational awareness.** Any agent can read another agent's terminal scrollback with `rz dump`, see who's active with `rz status`, and catch up on protocol messages with `rz log`.
- **Browser automation built in.** Open URLs in cmux browser splits, take screenshots, run JavaScript, click elements, fill forms — all scriptable from any agent.
- **Timers for autonomous operation.** Set a timer and get woken up when it fires. No polling loops needed.

The result: a team of agents that coordinates, delegates, and delivers — living entirely inside your terminal.

---

## What it does

- Spawn Claude (or any agent) into a new cmux split with a full bootstrap — identity, peers, workspace, communication instructions
- Send structured `@@RZ:` messages between surfaces with threading and reply-waiting
- Broadcast to all active agents at once
- Read another agent's terminal scrollback
- Ping surfaces, measure round-trip latency
- Open browser splits, take screenshots, run JavaScript, click and fill forms
- Send macOS notifications when work completes
- Create and manage cmux workspaces
- Coordinate entire swarms with a shared workspace (`goals.md`, `agents.md`, `context.md`)

---

## Install

```bash
cargo install rz-cmux
```

Or build from source:

```bash
git clone https://github.com/iliasoroka1/rz-cmux
cd rz-cmux
cargo build --release
cp target/release/rz ~/.local/bin/rz
```

Requires [cmux](https://github.com/manaflow-ai/cmux) running with `CMUX_SOCKET_PATH` and `CMUX_SURFACE_ID` set (automatically available inside cmux terminals).

---

## Quick start

You start the swarm. Your agents do the rest.

```bash
# Spawn a lead agent — it gets a full bootstrap and takes it from there
rz spawn --name lead -p "refactor the auth module, spawn helpers as needed" claude --dangerously-skip-permissions
```

From that point, agents run autonomously. They spawn each other, send messages, wait for replies, read each other's scrollback, and write to the shared workspace — all using `rz` commands themselves. You can observe or intervene at any time:

```bash
rz status                          # see who's active and message counts
rz log <surface_id>                # read protocol messages from a surface
rz dump <surface_id> --last 50     # read raw terminal output
rz send <surface_id> "new priority: focus on the login flow only"
rz broadcast "wrapping up — push your changes"
```

---

## How agent coordination works

When you spawn an agent with `rz spawn`, it receives a bootstrap message telling it:

- Its own surface ID and name
- Which other agents are running and their IDs
- Where the shared workspace is (`goals.md`, `agents.md`, `context.md`)
- How to send messages, reply with threading, broadcast, and spawn its own helpers

From that point the agent is self-sufficient. It can delegate, report back, wait for replies, and spawn sub-agents — all using the same `rz` binary it was told about at startup.

A typical swarm looks like this:

```
lead-agent
├── spawns researcher  → "find all auth-related TODOs"
├── spawns coder       → "implement the session token fix"
└── spawns reviewer    → "review coder's diff when done"
     └── sends rz send --wait 60 <coder> "ready for review?"
```

Each agent writes large outputs to the shared workspace and sends file paths via messages — keeping protocol messages short and the shared context up to date.

---

## Commands

### Agent lifecycle
| Command | Description |
|---|---|
| `rz id` | Print this surface's ID |
| `rz spawn <cmd>` | Spawn agent in new split with bootstrap |
| `rz close <id>` | Close a surface |
| `rz list` | List all surfaces |
| `rz status` | Surface counts and message counts |
| `rz tree` | Full window/workspace/surface hierarchy |

### Messaging
| Command | Description |
|---|---|
| `rz send <id> "msg"` | Send `@@RZ:` envelope to a surface |
| `rz send --raw <id> "text"` | Send plain text (no envelope) |
| `rz send --wait 30 <id> "msg"` | Block until reply arrives |
| `rz send --ref <msg_id> <id> "msg"` | Reply to a specific message (threading) |
| `rz broadcast "msg"` | Send to all other surfaces |
| `rz log <id>` | Show `@@RZ:` messages from scrollback |
| `rz dump <id>` | Full terminal scrollback |
| `rz ping <id>` | Ping and measure round-trip time |
| `rz timer 30 "label"` | Self-deliver a Timer message after N seconds |

### Workspace
| Command | Description |
|---|---|
| `rz init` | Create shared workspace (`/tmp/rz-cmux-<id>/`) |
| `rz dir` | Print workspace path |
| `rz workspace create --name "research"` | New cmux workspace |
| `rz workspace list` | List workspaces |

### Browser
| Command | Description |
|---|---|
| `rz browser open <url>` | Open URL in new browser split |
| `rz browser navigate <id> <url>` | Navigate existing browser |
| `rz browser screenshot <id>` | Screenshot (--output file.png) |
| `rz browser snapshot <id>` | DOM/accessibility tree |
| `rz browser eval <id> "js"` | Execute JavaScript |
| `rz browser url <id>` | Get current URL |
| `rz browser click <id> "selector"` | Click element |
| `rz browser fill <id> "sel" "text"` | Fill form field |

### Notifications
| Command | Description |
|---|---|
| `rz notify "title"` | macOS notification via cmux |

---

## Protocol

Every message is a single line: `@@RZ:<json>`

```json
@@RZ:{"id":"a1b20000","from":"surface-uuid","kind":{"kind":"chat","body":{"text":"hello"}},"ts":1774298000000}
```

Agents receive messages pasted into their terminal input. They parse `@@RZ:` lines from their own scrollback using `rz log`. The protocol is transport-agnostic — the same envelope format works in Zellij (original rz) and cmux (this fork).

---

## Architecture

```
rz-cmux
├── crates/
│   ├── rz-protocol/   # @@RZ: wire protocol (transport-agnostic)
│   └── rz-cli/
│       ├── cmux.rs    # cmux socket client (v2 JSON-RPC over Unix socket)
│       ├── bootstrap.rs  # bootstrap message builder
│       ├── log.rs     # extract @@RZ: messages from scrollback
│       ├── status.rs  # surface status summary
│       └── main.rs    # CLI entry point
```

The only cmux-specific code is `cmux.rs` — everything else is transport-agnostic and could be ported to any multiplexer with a socket API.

---

## Credits

Forked from [rz](https://github.com/HodlOg/rz) ([crates.io/crates/rz-cli](https://crates.io/crates/rz-cli)) by [@HodlOg](https://github.com/HodlOg). The `@@RZ:` protocol, bootstrap design, and core messaging architecture are from the original project.

## License

MIT OR Apache-2.0
