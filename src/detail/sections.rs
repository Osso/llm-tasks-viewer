use std::collections::HashMap;

use dioxus::prelude::*;

use crate::state::{AgentInfo, Project, SelectedTask, TaskDetail};

#[component]
pub(crate) fn DependenciesSection(
    detail: TaskDetail,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    let has_dependencies = !detail.depends_on.is_empty() || !detail.blocks.is_empty();
    if !has_dependencies {
        return rsx! {};
    }

    let project = detail.project.clone();

    rsx! {
        super::CollapsibleSection {
            title: "DEPENDENCIES",
            class: "detail-deps",
            header_class: "deps-header",
            DependencyPanel {
                label: "Depends on:",
                entries: detail.depends_on.clone(),
                project: project.clone(),
                selected,
            }
            DependencyPanel {
                label: "Blocks:",
                entries: detail.blocks.clone(),
                project,
                selected,
            }
        }
    }
}

#[component]
fn DependencyPanel(
    label: String,
    entries: Vec<(String, String, String)>,
    project: Project,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    if entries.is_empty() {
        return rsx! {};
    }

    rsx! {
        DependencyGroup { label, entries, project, selected }
    }
}

#[component]
fn DependencyGroup(
    label: String,
    entries: Vec<(String, String, String)>,
    project: Project,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    rsx! {
        div { class: "dep-group",
            span { class: "dep-label", "{label}" }
            DependencyLinks { entries, project, selected }
        }
    }
}

#[component]
fn DependencyLinks(
    entries: Vec<(String, String, String)>,
    project: Project,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    rsx! {
        for (id, title, status) in entries {
            DepLink {
                key: "{id}",
                project: project.clone(),
                id,
                title,
                status,
                selected,
            }
        }
    }
}

#[component]
fn DepLink(
    project: Project,
    id: String,
    title: String,
    status: String,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    let nav_id = id.clone();
    let nav_project = project.clone();
    let status_class = format!("dep-status dep-status-{status}");

    rsx! {
        span {
            class: "dep-link",
            onclick: move |_| {
                selected.set(Some(SelectedTask {
                    project: nav_project.clone(),
                    task_id: nav_id.clone(),
                }))
            },
            span { class: "{status_class}" }
            "[{id}] {title}"
        }
    }
}

#[component]
pub(crate) fn CommentsSection(detail: TaskDetail) -> Element {
    if detail.comments.is_empty() {
        return rsx! {};
    }

    rsx! {
        super::CollapsibleSection {
            title: "COMMENTS",
            class: "detail-comments",
            header_class: "comments-header",
            CommentRows { comments: detail.comments.clone() }
        }
    }
}

#[component]
fn CommentRows(comments: Vec<llm_tasks::db::Comment>) -> Element {
    rsx! {
        for comment in comments {
            CommentRow { comment }
        }
    }
}

#[component]
fn CommentRow(comment: llm_tasks::db::Comment) -> Element {
    rsx! {
        div { class: "comment-row",
            CommentMeta {
                actor: comment.actor,
                created_at: comment.created_at,
            }
            div { class: "comment-content", "{comment.content}" }
        }
    }
}

#[component]
fn CommentMeta(actor: String, created_at: String) -> Element {
    rsx! {
        div { class: "comment-meta",
            span { class: "comment-actor", "{actor}" }
            span { class: "comment-time", "{super::format_timestamp(&created_at)}" }
        }
    }
}

#[component]
pub(crate) fn EventTimeline(detail: TaskDetail) -> Element {
    if detail.events.is_empty() {
        return rsx! {};
    }

    rsx! {
        super::CollapsibleSection {
            title: "EVENTS",
            class: "detail-timeline",
            header_class: "timeline-header",
            EventRows { events: detail.events.clone() }
        }
    }
}

#[component]
fn EventRows(events: Vec<llm_tasks::db::Event>) -> Element {
    rsx! {
        for event in events {
            EventRow { event }
        }
    }
}

#[component]
fn EventRow(event: llm_tasks::db::Event) -> Element {
    let time = event.timestamp.get(11..16).unwrap_or("??:??");
    let description = format_event(&event);

    rsx! {
        div { class: "timeline-row",
            span { class: "timeline-time", "{time}" }
            span { class: "timeline-actor", "{event.actor}" }
            span { class: "timeline-desc", "{description}" }
        }
    }
}

fn format_event(event: &llm_tasks::db::Event) -> String {
    match event.action.as_str() {
        "created" => "created".into(),
        "claimed" => "claimed".into(),
        "closed" => "completed".into(),
        "updated" => {
            let field = event.field.as_deref().unwrap_or("?");
            let value = event.new_value.as_deref().unwrap_or("?");
            format!("{field} → {value}")
        }
        other => other.into(),
    }
}

#[component]
pub(crate) fn AgentStatusBadge(
    project: Project,
    task_id: String,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let statuses = agent_statuses.read();
    let key = crate::state::task_key(&project, &task_id);
    let Some(agent) = statuses.get(&key) else {
        return rsx! {};
    };

    rsx! {
        span { class: "agent-badge agent-running", "{agent.name}" }
    }
}
