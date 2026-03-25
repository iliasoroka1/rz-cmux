//! `rz` — inter-agent messaging over cmux.

use clap::{Parser, Subcommand};
use eyre::{Result, bail};

use rz_cmux_protocol::{Envelope, MessageKind};
use rz_cli::{bootstrap, cmux, log, status};

#[derive(Subcommand)]
enum BrowserCmd {
    /// Open a URL in a new browser split.
    Open {
        /// URL to open.
        url: String,
        /// Surface to split from (defaults to current).
        #[arg(long)]
        surface: Option<String>,
    },
    /// Navigate an existing browser surface to a URL.
    Navigate {
        /// Browser surface ID.
        surface: String,
        /// URL to navigate to.
        url: String,
    },
    /// Take a screenshot of a browser surface.
    Screenshot {
        /// Browser surface ID.
        surface: String,
        /// Capture full page (not just viewport).
        #[arg(long)]
        full_page: bool,
        /// Save to file path instead of printing base64.
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Get a DOM/accessibility tree snapshot (text representation).
    Snapshot {
        /// Browser surface ID.
        surface: String,
        /// Include interactive elements only.
        #[arg(long)]
        interactive: bool,
    },
    /// Execute JavaScript in the browser and print the result.
    Eval {
        /// Browser surface ID.
        surface: String,
        /// JavaScript code to execute.
        script: String,
    },
    /// Get the current URL of a browser surface.
    Url {
        /// Browser surface ID.
        surface: String,
    },
    /// Click an element by CSS selector.
    Click {
        /// Browser surface ID.
        surface: String,
        /// CSS selector.
        selector: String,
    },
    /// Fill a form field by CSS selector.
    Fill {
        /// Browser surface ID.
        surface: String,
        /// CSS selector.
        selector: String,
        /// Text to fill.
        text: String,
    },
}

#[derive(Subcommand)]
enum WorkspaceCmd {
    /// Create a new workspace.
    Create {
        /// Workspace name.
        #[arg(long)]
        name: Option<String>,
        /// Working directory for the new workspace.
        #[arg(long)]
        cwd: Option<String>,
    },
    /// List all workspaces.
    List,
}

/// Agent-to-agent messaging over cmux surfaces.
///
/// Uses cmux's socket API for direct, targeted communication between processes
/// running in cmux surfaces. No files, no focus switching — just surface IDs.
///
/// Quick start:
///   rz spawn claude                     # start an agent, get its surface ID
///   rz send <surface_id> "do something" # send it a message
///   rz list                             # see all running surfaces
///   rz dump <surface_id>                # read what it's been doing
///   rz broadcast "status update"        # message all agents
#[derive(Parser)]
#[command(name = "rz", version, about, long_about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print this surface's ID.
    Id,

    /// Initialize a shared workspace for this session.
    ///
    /// Creates a directory at /tmp/rz-cmux-<workspace_id>/ with a shared/ folder
    /// and prints the path. Agents can write files there instead of
    /// sending large messages. Idempotent — safe to call multiple times.
    Init,

    /// Print the session workspace path.
    ///
    /// Fails if `rz init` hasn't been run yet.
    Dir,

    /// Spawn an agent in a new surface with communication instructions.
    ///
    /// Creates a new cmux surface, waits for it to start, then sends
    /// bootstrap instructions (identity, rz usage, active peers).
    ///
    /// Examples:
    ///   rz spawn claude
    ///   rz spawn --name researcher -p "find all TODOs" claude
    ///   rz spawn --no-bootstrap python agent.py
    Spawn {
        /// Command to run.
        command: String,
        /// Surface name.
        #[arg(short, long)]
        name: Option<String>,
        /// Skip bootstrap instructions.
        #[arg(long)]
        no_bootstrap: bool,
        /// Seconds to wait for process to be ready before bootstrapping.
        #[arg(long, default_value = "45")]
        wait: u64,
        /// Task prompt to send after bootstrap.
        #[arg(short, long)]
        prompt: Option<String>,
        /// Extra arguments passed to the command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Send a message to a surface.
    ///
    /// By default wraps the message in an @@RZ: protocol envelope with
    /// sender ID and timestamp. Use --raw for plain text.
    ///
    /// Examples:
    ///   rz send <surface_id> "research this topic"
    ///   rz send --raw <surface_id> "ls -la"
    ///   rz send --ref abc123 <surface_id> "replying to your message"
    ///   rz send --wait 30 <surface_id> "do this and reply"
    Send {
        /// Target surface ID.
        pane: String,
        /// Message text.
        message: String,
        /// Send plain text instead of @@RZ: envelope.
        #[arg(long)]
        raw: bool,
        /// Sender identity. Defaults to CMUX_SURFACE_ID.
        #[arg(long)]
        from: Option<String>,
        /// Reference a previous message ID (for threading).
        #[arg(long)]
        r#ref: Option<String>,
        /// Block until a reply (with matching ref) arrives in own scrollback.
        /// Value is timeout in seconds.
        #[arg(long)]
        wait: Option<u64>,
    },

    /// Broadcast a message to all other terminal surfaces.
    Broadcast {
        /// Message text.
        message: String,
        /// Send plain text instead of @@RZ: envelopes.
        #[arg(long)]
        raw: bool,
    },

    /// List all surfaces with their info and status.
    List,

    /// Show a summary of the session: surface counts and per-surface status.
    ///
    /// Includes message counts from each surface's scrollback.
    Status,

    /// Dump a surface's scrollback to stdout.
    ///
    /// Examples:
    ///   rz dump <surface_id>              # full scrollback
    ///   rz dump <surface_id> --last 50    # last 50 lines only
    Dump {
        /// Target surface ID.
        pane: String,
        /// Only show the last N lines.
        #[arg(long)]
        last: Option<usize>,
    },

    /// Show @@RZ: protocol messages from a surface's scrollback.
    ///
    /// Extracts and formats all protocol envelopes, filtering out
    /// normal shell output.
    ///
    /// Examples:
    ///   rz log <surface_id>
    ///   rz log <surface_id> --last 10
    Log {
        /// Target surface ID.
        pane: String,
        /// Only show the last N messages.
        #[arg(long)]
        last: Option<usize>,
    },

    /// Close a surface.
    Close {
        /// Target surface ID.
        pane: String,
    },

    /// Ping a surface and measure round-trip time.
    ///
    /// Sends a Ping envelope and waits for a Pong reply (up to --timeout
    /// seconds). Useful for checking if an agent is alive and responsive.
    ///
    /// Examples:
    ///   rz ping <surface_id>
    ///   rz ping <surface_id> --timeout 5
    Ping {
        /// Target surface ID.
        pane: String,
        /// Seconds to wait for a Pong reply.
        #[arg(long, default_value = "3")]
        timeout: u64,
    },

    /// Set a timer — delivers @@RZ: Timer message when it fires.
    ///
    /// Spawns a background thread that sleeps and then sends a Timer
    /// envelope to self when it expires.
    ///
    /// Examples:
    ///   rz timer 30 "check build"     # 30s timer with label
    ///   rz timer 5                     # 5s timer, empty label
    Timer {
        /// Delay in seconds.
        seconds: f64,
        /// Timer label (delivered in the Timer message).
        #[arg(default_value = "")]
        label: String,
    },

    /// Browser automation commands.
    ///
    /// Open, navigate, screenshot, and interact with browser surfaces.
    ///
    /// Examples:
    ///   rz browser open https://docs.rs
    ///   rz browser screenshot <surface_id>
    ///   rz browser snapshot <surface_id>
    ///   rz browser eval <surface_id> "document.title"
    ///   rz browser url <surface_id>
    ///   rz browser click <surface_id> "button.submit"
    ///   rz browser fill <surface_id> "input#search" "query text"
    ///   rz browser navigate <surface_id> https://example.com
    Browser {
        #[command(subcommand)]
        action: BrowserCmd,
    },

    /// Send a notification to the user.
    ///
    /// Creates a cmux notification that appears in the sidebar and
    /// as a macOS system notification.
    ///
    /// Examples:
    ///   rz notify "Build complete"
    ///   rz notify "Test failed" --body "3 tests failed in auth module"
    ///   rz notify "Done" --surface <surface_id>
    Notify {
        /// Notification title.
        title: String,
        /// Notification body text.
        #[arg(long)]
        body: Option<String>,
        /// Associate with a specific surface.
        #[arg(long)]
        surface: Option<String>,
    },

    /// Workspace management commands.
    ///
    /// Examples:
    ///   rz workspace create --name "research"
    ///   rz workspace list
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCmd,
    },

    /// Show full system tree (windows, workspaces, surfaces).
    ///
    /// Displays the hierarchical structure of the cmux session.
    Tree,
}

fn rz_path() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "rz".into())
}

fn workspace_path() -> Result<std::path::PathBuf> {
    let workspace_id = std::env::var("CMUX_WORKSPACE_ID")
        .map_err(|_| eyre::eyre!("CMUX_WORKSPACE_ID not set — not inside cmux?"))?;
    Ok(std::path::PathBuf::from(format!("/tmp/rz-cmux-{workspace_id}")))
}

fn sender_id(from: Option<&str>) -> String {
    from.map(String::from)
        .or_else(|| cmux::own_surface_id().ok())
        .unwrap_or_else(|| "unknown".into())
}

/// Poll own scrollback for a reply referencing `msg_id`, with timeout.
fn wait_for_reply(msg_id: &str, timeout_secs: u64) -> Result<()> {
    let own = cmux::own_surface_id()?;
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(timeout_secs);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if std::time::Instant::now() >= deadline {
            bail!("timeout ({timeout_secs}s) — no reply to {msg_id}");
        }
        let scrollback = cmux::read_text(&own)?;
        let messages = log::extract_messages(&scrollback);
        if let Some(reply) = messages.iter().rev().find(|m| {
            m.r#ref.as_deref() == Some(msg_id)
        }) {
            println!("{}", log::format_message(reply));
            return Ok(());
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Cmd::Id => {
            println!("{}", cmux::own_surface_id()?);
        }

        Cmd::Init => {
            let ws = workspace_path()?;
            std::fs::create_dir_all(ws.join("shared"))?;

            // Create project coordination files (idempotent — don't overwrite).
            let goals = ws.join("goals.md");
            if !goals.exists() {
                std::fs::write(&goals, "\
# Session Goals

> Agents: read this file when you start. Add sub-goals as you discover them.

## Goal
_Fill in the session's primary objective._

## Sub-goals
-

## Completed
-
")?;
            }

            let context = ws.join("context.md");
            if !context.exists() {
                std::fs::write(&context, "\
# Session Context

> Agents: append here, never delete. Prefix entries with the date.

## Decisions

## Discoveries

## Open Questions
-
")?;
            }

            let agents = ws.join("agents.md");
            if !agents.exists() {
                std::fs::write(&agents, "\
# Active Agents

> Agents: update your row when starting or finishing a task.

| Surface | Name | Current Task | Status |
|---------|------|--------------|--------|
")?;
            }

            println!("{}", ws.display());
        }

        Cmd::Dir => {
            let ws = workspace_path()?;
            if !ws.exists() {
                bail!("workspace not initialized — run `rz init` first");
            }
            println!("{}", ws.display());
        }

        Cmd::Spawn {
            command,
            name,
            no_bootstrap,
            wait,
            prompt,
            args,
        } => {
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let surface_id = cmux::spawn(&command, &arg_refs, name.as_deref())?;

            if !no_bootstrap {
                // Phase 2: wait up to `wait` secs for Claude (or any agent)
                // to appear, then settle 5s before sending bootstrap.
                cmux::wait_for_stable_output(&surface_id, wait, 5);

                let msg = bootstrap::build(&surface_id, name.as_deref(), &rz_path())?;
                cmux::send(&surface_id, &msg)?;

                if let Some(task) = prompt {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    cmux::send(&surface_id, &task)?;
                }
            }

            println!("{surface_id}");
        }

        Cmd::Send { pane, message, raw, from, r#ref, wait } => {
            if raw {
                if wait.is_some() {
                    bail!("--wait requires protocol mode (cannot use with --raw)");
                }
                cmux::send(&pane, &message)?;
            } else {
                let mut envelope = Envelope::new(
                    sender_id(from.as_deref()),
                    MessageKind::Chat { text: message },
                );
                if let Some(r) = r#ref {
                    envelope = envelope.with_ref(r);
                }
                let msg_id = envelope.id.clone();
                cmux::send(&pane, &envelope.encode()?)?;

                if let Some(timeout_secs) = wait {
                    wait_for_reply(&msg_id, timeout_secs)?;
                }
            }
        }

        Cmd::Broadcast { message, raw } => {
            let from = sender_id(None);
            let peers = cmux::list_surface_ids()?;
            let own = cmux::own_surface_id().ok();
            let mut sent = 0;

            for peer in &peers {
                if own.as_deref() == Some(peer.as_str()) {
                    continue;
                }
                if raw {
                    cmux::send(peer, &message)?;
                } else {
                    let envelope = Envelope::new(
                        &from,
                        MessageKind::Chat { text: message.clone() },
                    );
                    cmux::send(peer, &envelope.encode()?)?;
                }
                sent += 1;
            }
            eprintln!("broadcast to {sent} surfaces");
        }

        Cmd::List => {
            let surfaces = cmux::list_surfaces()?;
            let own = cmux::own_surface_id().ok();
            println!("{:<38} {:<20} {:<38} {:<8}",
                "SURFACE_ID", "TITLE", "WORKSPACE", "TYPE");
            for s in &surfaces {
                let marker = if own.as_deref() == Some(s.id.as_str()) { " *" } else { "" };
                let title = if s.title.is_empty() { "-" } else { &s.title };
                let ws = s.workspace_name.as_deref().unwrap_or(&s.workspace_id);
                println!("{:<38} {:<20} {:<38} {:<8}{}",
                    s.id, title, ws, s.surface_type, marker);
            }
        }

        Cmd::Status => {
            let surfaces = cmux::list_surfaces()?;
            let summary = status::summarize(&surfaces, |id| cmux::read_text(id).ok());
            print!("{}", status::format_summary(&summary));
        }

        Cmd::Dump { pane, last } => {
            let text = cmux::read_text(&pane)?;
            if let Some(n) = last {
                let lines: Vec<&str> = text.lines().collect();
                let skip = lines.len().saturating_sub(n);
                for line in lines.into_iter().skip(skip) {
                    println!("{}", line);
                }
            } else {
                print!("{}", text);
            }
        }

        Cmd::Log { pane, last } => {
            let scrollback = cmux::read_text(&pane)?;
            let mut messages = log::extract_messages(&scrollback);
            if let Some(n) = last {
                let skip = messages.len().saturating_sub(n);
                messages = messages.into_iter().skip(skip).collect();
            }
            for msg in &messages {
                println!("{}", log::format_message(msg));
            }
        }

        Cmd::Close { pane } => {
            cmux::close(&pane)?;
        }

        Cmd::Ping { pane, timeout } => {
            let own = cmux::own_surface_id()?;
            let from = sender_id(None);
            let envelope = Envelope::new(&from, MessageKind::Ping);
            let ping_id = envelope.id.clone();
            let sent = std::time::Instant::now();

            cmux::send(&pane, &envelope.encode()?)?;

            let deadline = sent + std::time::Duration::from_secs(timeout);
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if std::time::Instant::now() >= deadline {
                    println!("timeout ({timeout}s) — no pong from {pane}");
                    std::process::exit(1);
                }
                let scrollback = cmux::read_text(&own)?;
                let messages = log::extract_messages(&scrollback);
                let got_pong = messages.iter().any(|m| {
                    matches!(m.kind, MessageKind::Pong)
                        && m.r#ref.as_deref() == Some(&ping_id)
                });
                if got_pong {
                    let rtt = sent.elapsed();
                    println!("pong from {pane} in {:.1}ms", rtt.as_secs_f64() * 1000.0);
                    break;
                }
            }
        }

        Cmd::Timer { seconds, label } => {
            let own = cmux::own_surface_id()?;
            let encoded = Envelope::new("timer", MessageKind::Timer { label }).encode()?;

            // Spawn a detached child process: sleep then send the timer envelope to self.
            let script = format!(
                "sleep {} && {} send --raw {} {}",
                seconds,
                shell_escape(&rz_path()),
                shell_escape(&own),
                shell_escape(&encoded),
            );

            std::process::Command::new("sh")
                .args(["-c", &script])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;

            eprintln!("timer set for {seconds}s");
        }

        Cmd::Browser { action } => {
            match action {
                BrowserCmd::Open { url, surface } => {
                    let sid = cmux::browser_open(&url, surface.as_deref())?;
                    println!("{sid}");
                }
                BrowserCmd::Navigate { surface, url } => {
                    cmux::browser_navigate(&surface, &url)?;
                }
                BrowserCmd::Screenshot { surface, full_page, output } => {
                    let result = cmux::browser_screenshot(&surface, full_page)?;
                    if let Some(path) = output {
                        // If result has base64 data, decode and write to file
                        if let Some(data) = result.get("data").and_then(|v| v.as_str()) {
                            use std::io::Write;
                            let bytes = base64_decode(data)?;
                            let mut f = std::fs::File::create(&path)?;
                            f.write_all(&bytes)?;
                            eprintln!("saved to {path}");
                        } else {
                            // Write raw JSON result
                            std::fs::write(&path, serde_json::to_string_pretty(&result)?)?;
                            eprintln!("saved to {path}");
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                }
                BrowserCmd::Snapshot { surface, interactive } => {
                    let result = cmux::browser_snapshot(&surface, interactive)?;
                    // Print as text if string, otherwise JSON
                    if let Some(text) = result.as_str() {
                        println!("{text}");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                }
                BrowserCmd::Eval { surface, script } => {
                    let result = cmux::browser_eval(&surface, &script)?;
                    if let Some(text) = result.as_str() {
                        println!("{text}");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                }
                BrowserCmd::Url { surface } => {
                    println!("{}", cmux::browser_url(&surface)?);
                }
                BrowserCmd::Click { surface, selector } => {
                    cmux::browser_click(&surface, &selector)?;
                }
                BrowserCmd::Fill { surface, selector, text } => {
                    cmux::browser_fill(&surface, &selector, &text)?;
                }
            }
        }

        Cmd::Notify { title, body, surface } => {
            cmux::notify(&title, body.as_deref(), surface.as_deref())?;
        }

        Cmd::Workspace { action } => {
            match action {
                WorkspaceCmd::Create { name, cwd } => {
                    let ws_id = cmux::workspace_create(name.as_deref(), cwd.as_deref())?;
                    println!("{ws_id}");
                }
                WorkspaceCmd::List => {
                    let result = cmux::workspace_list()?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }

        Cmd::Tree => {
            let result = cmux::system_tree()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

/// Simple shell escaping for single-quoted strings.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    // Simple base64 decoder (no external crate needed)
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buf = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in input.as_bytes() {
        if byte == b'=' || byte == b'\n' || byte == b'\r' { continue; }
        let val = table.iter().position(|&b| b == byte)
            .ok_or_else(|| eyre::eyre!("invalid base64"))? as u32;
        acc = (acc << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            buf.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(buf)
}
