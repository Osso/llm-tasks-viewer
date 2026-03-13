use std::collections::HashMap;

use dioxus::prelude::*;
use serde_json::Value;

use crate::state::{AgentInfo, LogEntry, Project};

#[component]
pub fn AgentLogSection(
    project: Project,
    task_id: String,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let mut entries = use_signal(Vec::<LogEntry>::new);
    let tid = task_id.clone();
    let poll_project = project.clone();

    use_future(move || {
        let tid = tid.clone();
        let project = poll_project.clone();
        async move {
            loop {
                let logs = crate::state::read_agent_log(&project, &tid, 50);
                entries.set(logs);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    });

    let logs = entries.read();
    let key = crate::state::task_key(&project, &task_id);
    let agent_name = agent_statuses.read().get(&key).map(|a| a.name.clone());
    let has_agent = agent_name.is_some();

    if logs.is_empty() && !has_agent {
        return rsx! {};
    }

    rsx! {
        crate::detail::CollapsibleSection {
            title: "AGENT LOG",
            class: "detail-agent-log",
            header_class: "agent-log-header",
            div { class: "agent-log-entries",
                for entry in logs.iter() {
                    { render_log_entry(entry) }
                }
            }
        }
    }
}

/// Sticky chat input shown at the bottom of the detail area.
#[component]
pub fn StickyChat(
    project: Project,
    task_id: String,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let key = crate::state::task_key(&project, &task_id);
    let agent_name = agent_statuses.read().get(&key).map(|a| a.name.clone());
    let Some(agent_name) = agent_name else {
        return rsx! {};
    };

    rsx! {
        ChatInput { project, task_id, agent_name }
    }
}

fn render_log_entry(entry: &LogEntry) -> Element {
    if entry.kind == "tool_call" {
        return rsx! { ToolCallEntry { entry: entry.clone() } };
    }

    render_plain_log_entry(entry)
}

#[component]
fn ToolCallEntry(entry: LogEntry) -> Element {
    let mut open = use_signal(|| false);
    let Some(tool_call) = parse_tool_call_text(&entry.text) else {
        return render_plain_log_entry(&entry);
    };

    let time = entry.timestamp.get(11..19).unwrap_or("");
    let chevron = if open() { "▾" } else { "▸" };
    let preview = tool_call_preview(tool_call.arguments);
    let body = truncate_owned(format_tool_call_body(tool_call.arguments), 2000);

    rsx! {
        div { class: "log-entry log-entry-tool_call",
            div {
                class: "log-tool-header",
                onclick: move |_| open.set(!open()),
                if !time.is_empty() {
                    span { class: "log-time", "{time}" }
                }
                span { class: "log-tool-chevron", "{chevron}" }
                span { class: "log-tool-name", "{tool_call.name}" }
                span { class: "log-tool-id", "{tool_call.id}" }
                if let Some(preview) = preview {
                    span { class: "log-tool-preview", "{preview}" }
                }
            }
            if open() {
                pre { class: "log-tool-body", "{body}" }
            }
        }
    }
}

fn render_plain_log_entry(entry: &LogEntry) -> Element {
    let time = entry.timestamp.get(11..19).unwrap_or("");
    let kind_class = format!("log-entry log-entry-{}", entry.kind);
    let text = truncate_log_text(&entry.text, 500);

    rsx! {
        div { class: "{kind_class}",
            if !time.is_empty() {
                span { class: "log-time", "{time}" }
            }
            span { class: "log-kind", "{entry.kind}" }
            span { class: "log-text", "{text}" }
        }
    }
}

struct ParsedToolCall<'a> {
    name: &'a str,
    id: &'a str,
    arguments: &'a str,
}

fn parse_tool_call_text(text: &str) -> Option<ParsedToolCall<'_>> {
    let (name, rest) = text.split_once(" [")?;
    let (id, arguments) = rest.split_once("] ")?;
    Some(ParsedToolCall {
        name: name.trim(),
        id: id.trim(),
        arguments: arguments.trim(),
    })
}

fn tool_call_preview(arguments: &str) -> Option<String> {
    let json = serde_json::from_str::<Value>(arguments).ok()?;
    let object = json.as_object()?;

    for key in [
        "command",
        "cmd",
        "expression",
        "q",
        "location",
        "ticker",
        "team",
        "url",
        "path",
    ] {
        if let Some(value) = object.get(key).and_then(|value| value.as_str()) {
            return Some(format!("{key}: {}", truncate_inline(value, 96)));
        }
    }

    Some(format!(
        "{} arg{}",
        object.len(),
        if object.len() == 1 { "" } else { "s" }
    ))
}

fn format_tool_call_body(arguments: &str) -> String {
    match serde_json::from_str::<Value>(arguments) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| arguments.to_string()),
        Err(_) => arguments.to_string(),
    }
}

fn truncate_log_text(text: &str, max: usize) -> &str {
    if text.len() <= max {
        text
    } else {
        &text[..max]
    }
}

fn truncate_inline(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{truncated}…")
}

fn truncate_owned(text: String, max: usize) -> String {
    if text.chars().count() <= max {
        return text;
    }

    let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{truncated}…")
}

fn scroll_detail_to_bottom() {
    let js = r#"
        let el = document.querySelector('.detail-area');
        if (el) el.scrollTop = el.scrollHeight;
    "#;
    document::eval(js);
}

fn spawn_send_message(
    project_name: String,
    agent: String,
    text: String,
    mut waiting: Signal<Option<String>>,
    mut error: Signal<Option<String>>,
) {
    let echo_text = text.clone();
    spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            crate::ipc::send_message(&project_name, &agent, &text)
        })
        .await;
        match result {
            Ok(Ok(())) => {
                waiting.set(Some(echo_text));
                scroll_detail_to_bottom();
            }
            Ok(Err(e)) => {
                waiting.set(None);
                error.set(Some(e));
            }
            Err(e) => {
                waiting.set(None);
                error.set(Some(format!("Task failed: {e}")));
            }
        }
    });
}

fn poll_for_echo(project: Project, task_id: String, mut waiting: Signal<Option<String>>) {
    spawn(async move {
        for _ in 0..15 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let expected = waiting.read().clone();
            let Some(ref text) = expected else { return };
            let logs = crate::state::read_agent_log(&project, &task_id, 10);
            let found = logs.iter().any(|e| e.kind == "user" && e.text == *text);
            if found {
                waiting.set(None);
                scroll_detail_to_bottom();
                return;
            }
        }
        waiting.set(None);
    });
}

#[component]
fn ChatInput(project: Project, task_id: String, agent_name: String) -> Element {
    let mut input_text = use_signal(String::new);
    let waiting: Signal<Option<String>> = use_signal(|| None);
    let mut error = use_signal(|| Option::<String>::None);
    let is_waiting = waiting.read().is_some();

    let on_submit = move |_| {
        let text = input_text().trim().to_string();
        if text.is_empty() || is_waiting {
            return;
        }
        error.set(None);
        input_text.set(String::new());
        spawn_send_message(
            project.name.clone(),
            agent_name.clone(),
            text,
            waiting,
            error,
        );
        poll_for_echo(project.clone(), task_id.clone(), waiting);
    };

    rsx! {
        div { class: "chat-input-area",
            if let Some(err) = error() {
                div { class: "chat-error", "{err}" }
            }
            ChatTextarea { input_text, disabled: is_waiting, on_submit }
        }
    }
}

#[component]
fn ChatTextarea(input_text: Signal<String>, disabled: bool, on_submit: EventHandler) -> Element {
    rsx! {
        div { class: "chat-input-row",
            textarea {
                class: "chat-textarea",
                placeholder: "Message agent...",
                rows: "2",
                value: "{input_text}",
                disabled,
                oninput: move |e| input_text.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Enter && !e.modifiers().shift() {
                        e.prevent_default();
                        on_submit.call(());
                    }
                },
            }
            button {
                class: "chat-send-btn",
                disabled: disabled || input_text().trim().is_empty(),
                onclick: move |_| on_submit.call(()),
                if disabled { "..." } else { "Send" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_call_text_extracts_name_id_and_arguments() {
        let parsed = parse_tool_call_text(r#"Bash [call_1] {"command":"pwd"}"#).unwrap();

        assert_eq!(parsed.name, "Bash");
        assert_eq!(parsed.id, "call_1");
        assert_eq!(parsed.arguments, r#"{"command":"pwd"}"#);
    }

    #[test]
    fn tool_call_preview_prefers_command_like_fields() {
        let preview = tool_call_preview(r#"{"command":"git status --short"}"#).unwrap();

        assert_eq!(preview, "command: git status --short");
    }

    #[test]
    fn format_tool_call_body_pretty_prints_json() {
        let body = format_tool_call_body(r#"{"command":"pwd","cwd":"/tmp"}"#);

        assert!(body.contains("\"command\": \"pwd\""));
        assert!(body.contains('\n'));
    }
}
