use std::collections::HashMap;

use dioxus::prelude::*;

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
/// Rendered outside AgentLogSection so it can stick to the viewport bottom.
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
        ChatInput { project, agent_name }
    }
}

fn render_log_entry(entry: &LogEntry) -> Element {
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

fn truncate_log_text(text: &str, max: usize) -> &str {
    if text.len() <= max {
        text
    } else {
        &text[..max]
    }
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
    mut sending: Signal<bool>,
    mut error: Signal<Option<String>>,
) {
    spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            crate::ipc::send_message(&project_name, &agent, &text)
        })
        .await;
        match result {
            Ok(Ok(())) => {
                scroll_detail_to_bottom();
            }
            Ok(Err(e)) => error.set(Some(e)),
            Err(e) => error.set(Some(format!("Task failed: {e}"))),
        }
        sending.set(false);
    });
}

#[component]
fn ChatInput(project: Project, agent_name: String) -> Element {
    let mut input_text = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    let on_submit = move |_| {
        let text = input_text().trim().to_string();
        if text.is_empty() || sending() {
            return;
        }
        sending.set(true);
        error.set(None);
        input_text.set(String::new());
        spawn_send_message(project.name.clone(), agent_name.clone(), text, sending, error);
    };

    rsx! {
        div { class: "chat-input-area",
            if let Some(err) = error() {
                div { class: "chat-error", "{err}" }
            }
            ChatTextarea { input_text, sending, on_submit }
        }
    }
}

#[component]
fn ChatTextarea(
    input_text: Signal<String>,
    sending: Signal<bool>,
    on_submit: EventHandler,
) -> Element {
    rsx! {
        div { class: "chat-input-row",
            textarea {
                class: "chat-textarea",
                placeholder: "Message agent...",
                rows: "2",
                value: "{input_text}",
                disabled: sending(),
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
                disabled: sending() || input_text().trim().is_empty(),
                onclick: move |_| on_submit.call(()),
                if sending() { "..." } else { "Send" }
            }
        }
    }
}
