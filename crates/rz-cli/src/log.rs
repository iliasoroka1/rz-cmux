//! Helpers for extracting and formatting protocol messages from scrollback.

use rz_cmux_protocol::{Envelope, MessageKind, SENTINEL};

/// Scan scrollback text for `@@RZ:` lines and parse each into an [`Envelope`].
///
/// Handles terminal line-wrapping by joining continuation lines until the
/// JSON parses successfully (up to 20 lines lookahead).
pub fn extract_messages(scrollback: &str) -> Vec<Envelope> {
    let lines: Vec<&str> = scrollback.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if let Some(idx) = lines[i].find(SENTINEL) {
            let mut candidate = lines[i][idx..].to_string();
            if let Ok(env) = Envelope::decode(&candidate) {
                result.push(env);
                i += 1;
                continue;
            }
            // Try joining wrapped continuation lines.
            for j in 1..20 {
                if i + j >= lines.len() {
                    break;
                }
                candidate.push_str(lines[i + j]);
                if let Ok(env) = Envelope::decode(&candidate) {
                    result.push(env);
                    i += j;
                    break;
                }
            }
        }
        i += 1;
    }

    result
}

/// Format an envelope as a human-readable one-liner: `[HH:MM:SS] from_id> text`
pub fn format_message(envelope: &Envelope) -> String {
    let secs = envelope.ts / 1000;
    let h = (secs / 3600) % 24;
    let m = (secs % 3600) / 60;
    let s = secs % 60;

    let text = match &envelope.kind {
        MessageKind::Chat { text } => text.as_str(),
        MessageKind::Hello { name, pane_id } => {
            return format!("[{h:02}:{m:02}:{s:02}] {}> hello ({name}, {pane_id})", envelope.from);
        }
        MessageKind::Ping => "ping",
        MessageKind::Pong => "pong",
        MessageKind::Error { message } => {
            return format!("[{h:02}:{m:02}:{s:02}] {}> error: {message}", envelope.from);
        }
        MessageKind::Timer { label } => {
            return format!("[{h:02}:{m:02}:{s:02}] {}> timer: {label}", envelope.from);
        }
    };

    format!("[{h:02}:{m:02}:{s:02}] {}> {text}", envelope.from)
}
