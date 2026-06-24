use dioxus::prelude::*;
use llm_tasks::db::Task;
use llm_tasks::db::TaskUpdates;

use crate::state::{Project, SelectedTask, TaskDetail};

#[derive(Clone, Copy)]
struct EditSignals {
    title: Signal<String>,
    description: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    assignee: Signal<String>,
}

fn edit_signals(task: &Task) -> EditSignals {
    EditSignals {
        title: use_signal(|| task.title.clone()),
        description: use_signal(|| task.description.clone().unwrap_or_default()),
        status: use_signal(|| task.status.clone()),
        priority: use_signal(|| task.priority.to_string()),
        assignee: use_signal(|| task.assignee.clone().unwrap_or_default()),
    }
}

async fn persist_task_update(
    project: &Project,
    task_id: &str,
    title: &str,
    description: &str,
    status: &str,
    priority: &str,
    assignee: &str,
) -> Result<(), String> {
    let priority = priority.parse::<u8>().unwrap_or(0);
    let description = if description.is_empty() {
        None
    } else {
        Some(description)
    };
    let assignee = if assignee.is_empty() {
        None
    } else {
        Some(assignee)
    };

    let updates = TaskUpdates {
        title: Some(title),
        description,
        status: Some(status),
        priority: Some(priority),
        assignee,
        ..Default::default()
    };

    let db = crate::state::open_db_for(project)
        .await
        .ok_or("Failed to open database")?;
    db.update_task(task_id, updates, "viewer")
        .await
        .map_err(|e| format!("{e}"))
}

async fn persist_status_update(
    project: &Project,
    task_id: &str,
    status: &str,
) -> Result<(), String> {
    let updates = TaskUpdates {
        status: Some(status),
        ..Default::default()
    };

    let db = crate::state::open_db_for(project)
        .await
        .ok_or("Failed to open database")?;
    db.update_task(task_id, updates, "viewer")
        .await
        .map_err(|e| format!("{e}"))
}

fn spawn_status_switch(
    project: Project,
    task_id: String,
    next_status: String,
    mut selected: Signal<Option<SelectedTask>>,
    mut switching: Signal<bool>,
    mut error: Signal<Option<String>>,
) {
    spawn(async move {
        switching.set(true);
        error.set(None);
        match persist_status_update(&project, &task_id, &next_status).await {
            Ok(_) => {
                let current = selected();
                selected.set(None);
                selected.set(current);
            }
            Err(err) => error.set(Some(err)),
        }
        switching.set(false);
    });
}

#[component]
fn QuickStatusButton(
    status: String,
    project: Project,
    task_id: String,
    selected: Signal<Option<SelectedTask>>,
    switching: Signal<bool>,
    error: Signal<Option<String>>,
) -> Element {
    let next_status = status.clone();
    let button_project = project.clone();
    let button_task_id = task_id.clone();

    rsx! {
        button {
            class: "status-quick-btn",
            disabled: switching(),
            onclick: move |_| {
                spawn_status_switch(
                    button_project.clone(),
                    button_task_id.clone(),
                    next_status.clone(),
                    selected,
                    switching,
                    error,
                )
            },
            "{super::status_label(&status)}"
        }
    }
}

#[component]
pub(crate) fn StatusQuickSwitch(
    project: Project,
    task_id: String,
    current_status: String,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    let switching = use_signal(|| false);
    let error = use_signal(|| Option::<String>::None);
    let targets = super::quick_status_targets(&current_status);

    rsx! {
        div { class: "status-switcher",
            QuickStatusButtons { targets, project, task_id, selected, switching, error }
            OptionalMessage {
                class_name: "status-switch-error",
                message: error(),
                inline: true,
            }
        }
    }
}

#[component]
fn QuickStatusButtons(
    targets: Vec<&'static str>,
    project: Project,
    task_id: String,
    selected: Signal<Option<SelectedTask>>,
    switching: Signal<bool>,
    error: Signal<Option<String>>,
) -> Element {
    rsx! {
        for status in targets {
            QuickStatusButton {
                key: "{status}",
                status: status.to_string(),
                project: project.clone(),
                task_id: task_id.clone(),
                selected,
                switching,
                error,
            }
        }
    }
}

#[component]
fn TextFieldEdit(label: String, value: Signal<String>) -> Element {
    rsx! {
        div { class: "edit-field",
            label { "{label}" }
            input {
                r#type: "text",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn TextAreaEdit(label: String, value: Signal<String>) -> Element {
    rsx! {
        div { class: "edit-field",
            label { "{label}" }
            textarea {
                rows: "4",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn StatusSelect(status: Signal<String>) -> Element {
    let open = use_signal(|| false);

    rsx! {
        div { class: "edit-field",
            label { "Status" }
            div { class: "dropdown",
                StatusSelectTrigger { status, open }
                StatusDropdownMenu { is_open: open(), status, open }
            }
        }
    }
}

#[component]
fn StatusSelectTrigger(status: Signal<String>, mut open: Signal<bool>) -> Element {
    rsx! {
        div {
            class: "dropdown-trigger",
            onclick: move |_| open.set(!open()),
            span { class: "dropdown-value", "{super::status_label(&status())}" }
            span { class: "dropdown-chevron", "▾" }
        }
    }
}

#[component]
fn StatusDropdownMenu(is_open: bool, status: Signal<String>, open: Signal<bool>) -> Element {
    if !is_open {
        return rsx! {};
    }

    rsx! {
        div { class: "dropdown-list",
            StatusDropdownList { status, open }
        }
    }
}

#[component]
fn StatusDropdownList(status: Signal<String>, open: Signal<bool>) -> Element {
    rsx! {
        for value in super::STATUSES {
            StatusDropdownItem {
                key: "{value}",
                value: value.to_string(),
                status,
                open,
            }
        }
    }
}

#[component]
fn StatusDropdownItem(value: String, status: Signal<String>, open: Signal<bool>) -> Element {
    let is_active = status() == value;
    let selected_value = value.clone();

    rsx! {
        div {
            class: if is_active { "dropdown-item active" } else { "dropdown-item" },
            onclick: move |_| {
                status.set(selected_value.clone());
                open.set(false);
            },
            "{super::status_label(&value)}"
        }
    }
}

#[component]
fn NumberFieldEdit(label: String, value: Signal<String>, min: String, max: String) -> Element {
    rsx! {
        div { class: "edit-field",
            label { "{label}" }
            input {
                r#type: "number",
                min: "{min}",
                max: "{max}",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn EditFields(
    title: Signal<String>,
    description: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    assignee: Signal<String>,
) -> Element {
    rsx! {
        TextFieldEdit { label: "Title", value: title }
        TextAreaEdit { label: "Description", value: description }
        div { class: "edit-row",
            StatusSelect { status }
            NumberFieldEdit { label: "Priority", value: priority, min: "0", max: "9" }
            TextFieldEdit { label: "Assignee", value: assignee }
        }
    }
}

fn spawn_save(
    project: Project,
    task_id: String,
    title: Signal<String>,
    description: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    assignee: Signal<String>,
    mut saving: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut editing: Signal<bool>,
    mut selected: Signal<Option<SelectedTask>>,
) {
    spawn(async move {
        saving.set(true);
        error.set(None);
        match persist_task_update(
            &project,
            &task_id,
            &title(),
            &description(),
            &status(),
            &priority(),
            &assignee(),
        )
        .await
        {
            Ok(_) => {
                editing.set(false);
                let current = selected();
                selected.set(None);
                selected.set(current);
            }
            Err(err) => error.set(Some(err)),
        }
        saving.set(false);
    });
}

#[component]
pub(crate) fn EditForm(
    detail: TaskDetail,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    let task = &detail.task;
    let project = detail.project.clone();
    let task_id = task.id.clone();
    let signals = edit_signals(task);
    let saving = use_signal(|| false);
    let error = use_signal(|| Option::<String>::None);

    let on_save = move |_| {
        spawn_save(
            project.clone(),
            task_id.clone(),
            signals.title,
            signals.description,
            signals.status,
            signals.priority,
            signals.assignee,
            saving,
            error,
            editing,
            selected,
        );
    };

    rsx! {
        EditFormBody {
            title: signals.title,
            description: signals.description,
            status: signals.status,
            priority: signals.priority,
            assignee: signals.assignee,
            error: error(),
            saving,
            editing,
            on_save,
        }
    }
}

#[component]
fn EditFormBody(
    title: Signal<String>,
    description: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    assignee: Signal<String>,
    error: Option<String>,
    saving: Signal<bool>,
    editing: Signal<bool>,
    on_save: EventHandler,
) -> Element {
    rsx! {
        div { class: "edit-form",
            EditFields { title, description, status, priority, assignee }
            OptionalMessage {
                class_name: "edit-error",
                message: error,
                inline: false,
            }
            EditActionButtons { saving, editing, on_save }
        }
    }
}

#[component]
fn OptionalMessage(class_name: String, message: Option<String>, inline: bool) -> Element {
    let Some(message) = message else {
        return rsx! {};
    };

    if inline {
        return rsx! {
            span { class: "{class_name}", "{message}" }
        };
    }

    rsx! {
        div { class: "{class_name}", "{message}" }
    }
}

#[component]
fn EditActionButtons(
    saving: Signal<bool>,
    editing: Signal<bool>,
    on_save: EventHandler,
) -> Element {
    rsx! {
        div { class: "edit-actions",
            button {
                class: "btn-save",
                disabled: saving(),
                onclick: move |_| on_save.call(()),
                if saving() { "Saving..." } else { "Save" }
            }
            button {
                class: "btn-cancel",
                disabled: saving(),
                onclick: move |_| editing.set(false),
                "Cancel"
            }
        }
    }
}
