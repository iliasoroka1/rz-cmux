# rz-cmux

**Run a swarm of AI agents inside your terminal — all talking to each other natively.**

rz-cmux gives every [cmux](https://github.com/manaflow-ai/cmux) terminal session a native inter-agent messaging layer. Spawn Claude (or any agent) into a split, send it work, read its output, wait for replies, broadcast to the whole swarm — all from a single `rz` binary that any agent can call itself.

No servers. No sidecars. No configuration. If you're inside cmux, it just works.

**Fork of [rz](https://github.com/HodlOg/rz)** ([crates.io](https://crates.io/crates/rz-cli)) by [@HodlOg](https://github.com/HodlOg) — the original Zellij inter-agent messaging tool. This fork replaces the Zellij transport with cmux's v2 JSON-RPC socket API and adds cmux-native features: browser automation, MPI-style collective operations, notifications, and workspace management. The `@@RZ:` wire protocol is unchanged — agents communicate identically regardless of multiplexer.

---

## Why rz-cmux

Modern AI coding workflows involve more than one agent. You might have a researcher, a coder, a reviewer, and a QA agent all running in parallel. Without a messaging layer they're blind to each other — duplicating work, stepping on each other's files, unable to hand off tasks.

rz-cmux solves this with one small binary:

- **Agents can spawn agents.** A lead agent delegates sub-tasks to helpers, waits for replies, and reports back — all autonomously.
- **Structured messaging with threading.** Every message has an ID. Agents reply to specific messages with `--ref`, building proper conversation threads across splits.
- **MPI-style collective operations.** `rz ask` for synchronous request/reply. `rz gather` to fan-in status from a group of parallel workers in one call.
- **Shared workspace.** A `goals.md`, `agents.md`, and `context.md` give the whole swarm shared memory. Agents write discoveries, claim files, and coordinate without collision.
- **Full situational awareness.** Any agent can read another agent's terminal scrollback with `rz logs`, see who's active with `rz ps`, and catch up on protocol messages with `rz log`.
- **Browser automation built in.** Open URLs in cmux browser splits, take screenshots, run JavaScript, click elements, fill forms — all scriptable from any agent.
- **Timers for autonomous operation.** Set a timer and get woken up when it fires. No polling loops needed.
- **Familiar command names.** Docker-style aliases (`ps`, `logs`, `run`, `kill`) and Playwright-style aliases (`goto`, `exec`, `snap`, `content`, `waitfor`) so agents don't have to learn new vocabulary.

The result: a team of agents that coordinates, delegates, and delivers — living entirely inside your terminal.

---

## What it does

- Spawn Claude (or any agent) into a new cmux split with a full bootstrap — identity, peers, workspace, all commands
- Send structured `@@RZ:` messages between surfaces with threading and reply-waiting
- Ask an agent a question and block until it replies (`rz ask`)
- Gather the last message from multiple agents in one call (`rz gather`)
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
rz run --name lead -p "refactor the auth module, spawn helpers as needed" claude --dangerously-skip-permissions
```

From that point, agents run autonomously. They spawn each other, send messages, wait for replies, read each other's scrollback, and write to the shared workspace — all using `rz` commands themselves. You can observe or intervene at any time:

```bash
rz ps                              # see who's active (alias: list)
rz logs <surface_id> --last 50    # read raw terminal output (alias: dump)
rz gather <id1> <id2> <id3>       # see last message from each worker at once
rz ask <surface_id> "what's your status?"
rz broadcast "wrapping up — push your changes"
```

---

## How agent coordination works

When you spawn an agent with `rz run`, it receives a bootstrap message telling it:

- Its own surface ID and name
- Which other agents are running and their IDs
- Where the shared workspace is (`goals.md`, `agents.md`, `context.md`)
- Every available command with examples — messaging, browser, timers, spawning

From that point the agent is self-sufficient. It can delegate, report back, wait for replies, and spawn sub-agents — all using the same `rz` binary it was told about at startup.

A typical swarm looks like this:

```
lead-agent
├── rz run → researcher   "find all auth-related TODOs"
├── rz run → coder        "implement the session token fix"
└── rz run → reviewer     "review coder's diff when done"
      └── rz ask <coder> "ready for review?"   ← blocks until coder replies
          rz gather <researcher> <coder>        ← fan-in status from both
```

Each agent writes large outputs to the shared workspace and sends file paths via messages — keeping protocol messages short and the shared context up to date.

---

## Commands

### Agent lifecycle
| Command | Alias | Description |
|---|---|---|
| `rz id` | | Print this surface's ID |
| `rz spawn <cmd>` | `rz run` | Spawn agent in new split with bootstrap |
| `rz close <id>` | `rz kill` | Close a surface |
| `rz list` | `rz ps` | List all surfaces |
| `rz status` | | Surface counts and message counts |
| `rz tree` | | Full window/workspace/surface hierarchy |

### Messaging
| Command | Alias | Description |
|---|---|---|
| `rz send <id> "msg"` | | Send `@@RZ:` envelope to a surface |
| `rz send --raw <id> "text"` | | Send plain text (no envelope) |
| `rz ask <id> "msg"` | | Send and block until reply (default 60s) |
| `rz ask <id> "msg" --timeout 120` | | Same with custom timeout |
| `rz gather <id1> <id2>...` | | Last message from each agent (fan-in) |
| `rz gather <id1> <id2> --last 3` | | Last N messages from each |
| `rz send --ref <msg_id> <id> "msg"` | | Reply to a specific message (threading) |
| `rz broadcast "msg"` | | Send to all other surfaces |
| `rz log <id>` | | Show `@@RZ:` messages from scrollback |
| `rz dump <id>` | `rz logs` | Full terminal scrollback |
| `rz ping <id>` | | Ping and measure round-trip time (default 60s) |
| `rz timer 30 "label"` | | Self-deliver a Timer message after N seconds |

### Workspace
| Command | Description |
|---|---|
| `rz init` | Create shared workspace (`/tmp/rz-cmux-<id>/`) |
| `rz dir` | Print workspace path |
| `rz workspace create --name "research"` | New cmux workspace |
| `rz workspace list` | List workspaces |

### Browser
| Command | Alias | Description |
|---|---|---|
| `rz browser open <url>` | | Open URL in new browser split, returns surface ID |
| `rz browser wait <id>` | `waitfor` | Block until page finishes loading (`--timeout N`) |
| `rz browser navigate <id> <url>` | `goto` | Navigate existing browser (`--wait` to block until loaded) |
| `rz browser screenshot <id>` | `snap` | Screenshot (`--output file.png` saves PNG, `--full-page`) |
| `rz browser snapshot <id>` | `content` | Full page HTML / DOM tree |
| `rz browser eval <id> "js"` | `exec` | Execute JavaScript, returns result as JSON |
| `rz browser url <id>` | | Get current URL |
| `rz browser click <id> "selector"` | | Click element by CSS selector |
| `rz browser fill <id> "sel" "text"` | | Fill form field by CSS selector |
| `rz browser close <id>` | | Close the browser surface |

### Notifications
| Command | Description |
|---|---|
| `rz notify "title"` | macOS notification via cmux |

---

## Protocol

Every message is a single line: `@@RZ:<json>`

```
@@RZ:{"id":"a1b20000","from":"surface-uuid","kind":{"kind":"chat","body":{"text":"hello"}},"ts":1774298000000}
```

Agents receive messages pasted into their terminal input. They parse `@@RZ:` lines from their own scrollback using `rz log`. Reply to a message with `--ref <id>` to create a thread. The protocol is transport-agnostic — the same envelope format works in Zellij (original rz) and cmux (this fork).

---

## Architecture

```
rz-cmux
├── crates/
│   ├── rz-protocol/   # @@RZ: wire protocol (transport-agnostic)
│   └── rz-cli/
│       ├── cmux.rs       # cmux socket client (v2 JSON-RPC over Unix socket)
│       ├── bootstrap.rs  # bootstrap message builder
│       ├── log.rs        # extract @@RZ: messages from scrollback
│       ├── status.rs     # surface status summary
│       └── main.rs       # CLI entry point
```

The only cmux-specific code is `cmux.rs` — everything else is transport-agnostic and could be ported to any multiplexer with a socket API.

---

## Credits

Forked from [rz](https://github.com/HodlOg/rz) ([crates.io/crates/rz-cli](https://crates.io/crates/rz-cli)) by [@HodlOg](https://github.com/HodlOg). The `@@RZ:` protocol, bootstrap design, and core messaging architecture are from the original project.

## License

MIT OR Apache-2.0
