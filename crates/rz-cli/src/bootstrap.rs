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

You have `rz` at `{rz_path}`. Use it to talk to other agents:

```bash
# Send a message to another agent (use surface ID)
{rz_path} send <surface_id> "your message"

# Send and block until reply (timeout in seconds)
{rz_path} send --wait 30 <surface_id> "question"

# Reply to a specific message (threading)
{rz_path} send --ref <message_id> <surface_id> "your response"

# List all agents
{rz_path} list

# Session overview with message counts
{rz_path} status

# Read another agent's scrollback (last N lines)
{rz_path} dump <surface_id> --last 50

# View protocol messages only
{rz_path} log <surface_id>

# Broadcast to all agents
{rz_path} broadcast "message"

# Set a timer — you'll get an @@RZ: Timer message when it fires
{rz_path} timer <seconds> "label"
```

{workspace_section}### Active agents

{peers}
### Protocol

When you receive a message starting with `@@RZ:` it is a protocol envelope.
The JSON inside has `from`, `kind`, and `ts` fields. Reply with
`{rz_path} send --ref <message_id> <from_surface_id> "your response"`.

### Working patterns

**Messages vs files.** Keep `rz send` messages short (status updates, questions,
results). Write large outputs (research, code drafts, audit reports) to the
workspace `shared/` directory and send the file path instead.

**Parallel work.** When multiple agents edit code simultaneously, divide by
**file** not by feature. Two agents editing the same file causes conflicts.
Claim your files, finish, then hand off.

**Spawning sub-agents.** You can spawn your own helpers for sub-tasks:
`{rz_path} spawn --name subtask-name -p "focused task description" claude`
Give sub-agents narrow scope. They report back to you; you report to your caller.

**Situational awareness.** Run `{rz_path} status` or `{rz_path} list` to see
who else is active. Check `{rz_path} log <surface_id>` to catch up on what
another agent has been doing.

**Timers.** Use `{rz_path} timer 300 "check build"` for periodic monitoring,
build checks, or goal reviews. No polling — the hub wakes you up.

**Audits and reviews.** Write findings to the workspace (`shared/audit-*.md`).
Send a short summary via message with the file path. Do NOT fix code outside
your assigned scope — report issues and let the responsible agent fix them.
This prevents merge conflicts and respects file ownership.

### Browser automation

You can open and control a real browser. Every browser command targets a
**surface ID** returned by `browser open`.

```bash
# Open a URL — returns a browser surface ID
B=$({rz_path} browser open https://example.com)

# Wait for the page to finish loading (always do this after open/navigate)
{rz_path} browser wait $B

# Navigate to a new URL (--wait blocks until loaded)
{rz_path} browser navigate $B https://other.com --wait

# Take a screenshot and save as PNG
{rz_path} browser screenshot $B --output /tmp/page.png

# Get the page source / DOM snapshot
{rz_path} browser snapshot $B

# Run JavaScript and get the result
{rz_path} browser eval $B "document.title"

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
