use dioxus::prelude::*;

use crate::state::TaskDetail;

#[component]
fn TaskHeader(detail: TaskDetail) -> Element {
    let task = &detail.task;
    let status_class = format!("status-badge status-{}", task.status);
    let status_label = match task.status.as_str() {
        "in_progress" => "In Progress",
        other => other,
    };

    rsx! {
        div { class: "detail-header",
            div { class: "detail-title-row",
                span { class: "detail-title", "{task.title}" }
                span { class: "detail-id", "{task.id}" }
            }
            div { class: "detail-meta-row",
                span { class: "{status_class}", "{status_label}" }
                if task.priority > 0 {
                    span { class: "badge-priority", "P{task.priority}" }
                }
                if let Some(ref assignee) = task.assignee {
                    span { class: "detail-assignee", "@{assignee}" }
                }
            }
        }
    }
}

#[component]
fn DepLink(id: String, title: String, status: String, selected: Signal<Option<String>>) -> Element {
    let nav_id = id.clone();
    let status_class = format!("dep-status dep-status-{status}");

    rsx! {
        span {
            class: "dep-link",
            onclick: move |_| selected.set(Some(nav_id.clone())),
            span { class: "{status_class}" }
            "[{id}] {title}"
        }
    }
}

#[component]
fn DependenciesSection(detail: TaskDetail, selected: Signal<Option<String>>) -> Element {
    let has_deps = !detail.depends_on.is_empty() || !detail.blocks.is_empty();
    if !has_deps {
        return rsx! {};
    }

    rsx! {
        div { class: "detail-deps",
            if !detail.depends_on.is_empty() {
                div { class: "dep-group",
                    span { class: "dep-label", "Depends on:" }
                    for (id, title, status) in &detail.depends_on {
                        DepLink {
                            key: "{id}",
                            id: id.clone(),
                            title: title.clone(),
                            status: status.clone(),
                            selected,
                        }
                    }
                }
            }
            if !detail.blocks.is_empty() {
                div { class: "dep-group",
                    span { class: "dep-label", "Blocks:" }
                    for (id, title, status) in &detail.blocks {
                        DepLink {
                            key: "{id}",
                            id: id.clone(),
                            title: title.clone(),
                            status: status.clone(),
                            selected,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EventTimeline(detail: TaskDetail) -> Element {
    if detail.events.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "detail-timeline",
            div { class: "timeline-header", "EVENTS" }
            for event in &detail.events {
                {
                    let time = event.timestamp.get(11..16).unwrap_or("??:??");
                    let desc = format_event(event);
                    rsx! {
                        div { class: "timeline-row",
                            span { class: "timeline-time", "{time}" }
                            span { class: "timeline-actor", "{event.actor}" }
                            span { class: "timeline-desc", "{desc}" }
                        }
                    }
                }
            }
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
            let new = event.new_value.as_deref().unwrap_or("?");
            format!("{field} → {new}")
        }
        other => other.into(),
    }
}

#[component]
pub fn Detail(detail: Signal<Option<TaskDetail>>, selected: Signal<Option<String>>) -> Element {
    let Some(d) = detail() else {
        return rsx! {
            div { class: "detail-empty", "Select a task" }
        };
    };

    rsx! {
        div { class: "detail-area",
            TaskHeader { detail: d.clone() }
            if let Some(ref desc) = d.task.description {
                div { class: "detail-description", "{desc}" }
            }
            DependenciesSection { detail: d.clone(), selected }
            EventTimeline { detail: d }
        }
    }
}
