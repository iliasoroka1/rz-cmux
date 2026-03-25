//! Bootstrap message sent to newly spawned agents.

use eyre::Result;

use crate::cmux;

/// Build bootstrap instructions for a newly spawned agent.
///
/// Includes: identity, how to communicate, who else is running.
pub fn build(surface_id: &str, name: Option<&str>, rz_path: &str) -> Result<String> {
    let surfaces = cmux::list_surfaces()?;
    let identity = name.unwrap_or(surface_id);

    let mut peers = String::new();
    for s in &surfaces {
        if s.surface_type == "browser" || s.id == surface_id {
            continue;
        }
        let title = if s.title.is_empty() { "shell" } else { &s.title };
        let ws = s.workspace_name.as_deref().unwrap_or("-");
        peers.push_str(&format!("  - {} ({}, workspace: {})\n", s.id, title, ws));
    }
    if peers.is_empty() {
        peers.push_str("  (none)\n");
    }

    // Check if workspace exists.
    let workspace = std::env::var("CMUX_SOCKET_PATH")
        .ok()
        .and_then(|sock| {
            // Derive workspace dir from socket path: /tmp/rz-cmux-<name>
            let stem = std::path::Path::new(&sock)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cmux")
                .to_string();
            Some(format!("/tmp/rz-{stem}"))
        })
        .filter(|p| std::path::Path::new(p).exists());

    let workspace_section = if let Some(ref ws) = workspace {
        format!(
            r#"### Workspace

Shared workspace: `{ws}/shared/` — write large outputs here, not in messages.

**Project files** (read these first, update as you work):

- **`{ws}/goals.md`** — Session objectives. READ on start. Add sub-goals you discover.
- **`{ws}/context.md`** — Decisions and discoveries. APPEND entries as you learn things others should know.
- **`{ws}/agents.md`** — Who's doing what. UPDATE with your surface ID, name, and current task when you start or switch tasks.
"#
        )
    } else {
        String::new()
    };

    Ok(format!(
        r#"## Multi-Agent Environment

You are agent "{identity}" (surface: {surface_id}) in a multi-agent cmux session.

You are **long-lived** — you will receive multiple tasks over time, not just one.
After completing a task, report back and wait for the next one. Your context
and knowledge accumulate across tasks, making you more valuable over time.
Do not exit after finishing a task.

### Communication

You have `rz` at `{rz_path}`. Your surface ID is `{surface_id}`.

```bash
# --- Identity & discovery ---
{rz_path} id                              # print your own surface ID
{rz_path} ps                              # all active surfaces (alias: list)
{rz_path} status                          # surface count + message counts per agent
{rz_path} ping <surface_id>               # check if an agent is alive, measure RTT

# --- Sending messages ---
{rz_path} send <surface_id> "message"     # send a structured @@RZ: message
{rz_path} send --wait 30 <surface_id> "?" # block until the agent replies (30s timeout)
{rz_path} send --ref <msg_id> <surface_id> "reply"  # reply to a specific message (threading)
{rz_path} broadcast "message"             # send to all other agents at once

# --- Reading output ---
{rz_path} logs <surface_id> --last 50    # read raw terminal scrollback (alias: dump)
{rz_path} log <surface_id>               # show only @@RZ: protocol messages
{rz_path} log <surface_id> --last 10     # last 10 protocol messages

# --- Spawning sub-agents ---
{rz_path} run claude --name worker -p "do X"   # spawn agent, send it a task (alias: spawn)
{rz_path} run --no-bootstrap claude            # spawn without bootstrap (bare shell)
{rz_path} kill <surface_id>                    # close a surface when done (alias: close)

# --- Timers (no polling loops) ---
{rz_path} timer 60 "check build"         # wake yourself up after 60s with a Timer message
```

{workspace_section}### Active agents

{peers}
### Incoming messages (@@RZ: protocol)

Messages arrive pasted into your terminal input. They look like:

```
@@RZ:{{"id":"a1b20000","from":"<sender-surface-id>","kind":{{"kind":"chat","body":{{"text":"hello"}}}},"ts":1234567890}}
```

Parse the envelope: `id` is the message ID (use it in `--ref` to reply), `from` is the sender's surface ID.

To reply:
```bash
{rz_path} send --ref <id> <from> "your response"
```

### Working patterns

**Report back when done.** After completing a task, send a message to whoever assigned it. Include what you did and any blockers. Then wait for the next task — do not exit.

**Messages vs files.** Keep `rz send` messages short (status, questions, results). Write large outputs (code, research, reports) to the workspace `shared/` directory and send the file path.

**Parallel work.** When multiple agents edit code simultaneously, divide by **file** not by feature. Two agents editing the same file causes merge conflicts. Claim your files in `agents.md`, finish, then hand off.

**Spawning sub-agents.** Delegate sub-tasks with `rz spawn`. Give sub-agents a narrow scope and a clear `--prompt`. They report back to you; you report to your caller.

**Situational awareness.** Run `{rz_path} status` on start to see who's active. Use `{rz_path} log <surface_id>` to catch up on what another agent has been doing.

**Timers.** Use `{rz_path} timer 300 "label"` for periodic checks. The timer delivers an `@@RZ:` Timer message to you — no polling loops needed.

**Audits and reviews.** Write findings to `shared/audit-*.md`. Send the file path in a message. Do NOT fix code outside your assigned scope — report and let the owner fix it.

### Browser automation

You can open and control a real browser. Every browser command targets a
**surface ID** returned by `browser open`.

```bash
# Open a URL — returns a browser surface ID
B=$({rz_path} browser open https://example.com)

# Wait for the page to finish loading (always do this after open/goto)
{rz_path} browser waitfor $B              # alias: wait

# Navigate to a new URL (--wait blocks until loaded)
{rz_path} browser goto $B https://other.com --wait   # alias: navigate

# Take a screenshot and save as PNG
{rz_path} browser snap $B --output /tmp/page.png    # alias: screenshot

# Get the page source / DOM snapshot
{rz_path} browser content $B             # alias: snapshot

# Run JavaScript and get the result
{rz_path} browser exec $B "document.title"   # alias: eval

# Click an element by CSS selector
{rz_path} browser click $B "button.submit"

# Fill a form field
{rz_path} browser fill $B "input[name='q']" "search term"

# Get current URL
{rz_path} browser url $B

# Close the browser surface when done
{rz_path} browser close $B
```

**Important rules:**
- Always call `browser wait $B` after `open` or `navigate` before using `eval`, `fill`, or `click`. Navigation is async — the page may not be loaded yet.
- Use `browser close $B` when finished to keep the workspace tidy.
- `fill` sets the input value directly. If the site uses React/Vue and needs keyboard events, use `eval` to dispatch an `input` event after filling.
- Screenshots are saved as real PNG files when you use `--output`. Without `--output`, the raw JSON (including base64) is printed.
- `snapshot` returns the full page HTML — good for reading content without running JS."#
    ))
}
