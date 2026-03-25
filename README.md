# rz-cmux

Inter-agent messaging for [cmux](https://github.com/manaflow-ai/cmux) terminal sessions.

**Fork of [rz](https://github.com/HodlOg/rz)** ([crates.io](https://crates.io/crates/rz-cli)) by [@HodlOg](https://github.com/HodlOg) — the original Zellij inter-agent messaging tool. This fork replaces the Zellij transport with cmux's v2 JSON-RPC socket API and adds cmux-native features (browser automation, notifications, workspace management). The `@@RZ:` wire protocol is unchanged — agents communicate identically regardless of which multiplexer they're running in.

## What it does

- Spawn Claude (or any agent) in a new cmux split with bootstrap instructions
- Send structured `@@RZ:` messages between surfaces with threading and reply-waiting
- Read another agent's terminal scrollback
- Open browser splits, take screenshots, run JavaScript
- Send notifications, manage workspaces
- Coordinate multi-agent sessions with a shared workspace (`goals.md`, `agents.md`, `context.md`)

## Install

```bash
git clone https://github.com/YOUR_USERNAME/rz-cmux
cd rz-cmux
cargo build --release
cp target/release/rz ~/.local/bin/rz
```

Requires [cmux](https://github.com/manaflow-ai/cmux) running with `CMUX_SOCKET_PATH` and `CMUX_SURFACE_ID` set (automatically available inside cmux terminals).

## Quick start

```bash
# Spawn a Claude sub-agent in a new split
rz spawn --name researcher -p "find all TODOs in the codebase" claude --dangerously-skip-permissions

# Send it a follow-up message
rz send <surface_id> "focus on the auth module only"

# Read what it's been doing
rz dump <surface_id> --last 50

# See all protocol messages
rz log <surface_id>

# Broadcast to all agents
rz broadcast "wrapping up, push your changes"

# Open a browser split
rz browser open https://docs.rs/eyre

# Take a screenshot
rz browser screenshot <browser_surface_id> --output screenshot.png

# Send a macOS notification
rz notify "Build complete" --body "all tests passed"
```

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
| `rz send --raw <id> "text"` | Send plain text |
| `rz send --wait 30 <id> "msg"` | Block until reply arrives |
| `rz send --ref <msg_id> <id> "msg"` | Reply to a specific message |
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

## Protocol

Every message is a single line: `@@RZ:<json>`

```json
@@RZ:{"id":"a1b20000","from":"surface-uuid","kind":{"kind":"chat","body":{"text":"hello"}},"ts":1774298000000}
```

Agents receive messages pasted into their terminal input. They parse `@@RZ:` lines from their own scrollback using `rz log`.

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

The only cmux-specific code is `cmux.rs` — everything else is transport-agnostic.

## Differences from rz (Zellij)

| Feature | rz (Zellij) | rz-cmux |
|---|---|---|
| Transport | `zellij action paste` | cmux v2 JSON-RPC socket |
| Surface IDs | `terminal_3` | UUIDs |
| Browser automation | ✗ | ✓ (60+ methods) |
| Notifications | ✗ | ✓ (macOS native) |
| Workspace management | ✗ | ✓ |
| Hub/WASM plugin | Optional | Not needed |
| Timer | Via hub | `sh -c "sleep N && rz send"` |

## Credits

Forked from [rz](https://github.com/HodlOg/rz) ([crates.io/crates/rz-cli](https://crates.io/crates/rz-cli)) by [@HodlOg](https://github.com/HodlOg). The `@@RZ:` protocol, bootstrap design, and core messaging architecture are from the original project.

## License

MIT OR Apache-2.0
