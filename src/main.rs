mod detail;
mod sidebar;
mod state;

use std::collections::HashMap;

use dioxus::prelude::*;
use llm_tasks::db::Task;

use crate::detail::Detail;
use crate::sidebar::Sidebar;
use crate::state::{AgentInfo, Project, TaskDetail};

const STYLE: &str = include_str!("../assets/style.css");

async fn refresh_detail(
    sel: Option<String>,
    project: Option<Project>,
    mut task_detail: Signal<Option<TaskDetail>>,
) {
    match (sel, project) {
        (Some(id), Some(proj)) => {
            if let Some(db) = state::open_db_for(&proj).await {
                task_detail.set(state::load_detail(&db, &id).await);
            }
        }
        _ => task_detail.set(None),
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_disable_context_menu(true)
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("llm-tasks")
                        .with_decorations(false)
                        .with_inner_size(dioxus::desktop::LogicalSize::new(960, 640)),
                ),
        )
        .launch(App);
}

fn setup_project_refresh(
    active_project: Signal<Option<Project>>,
    mut tasks: Signal<Vec<Task>>,
) {
    use_effect(move || {
        let proj = active_project();
        spawn(async move {
            if let Some(proj) = proj {
                if let Some(db) = state::open_db_for(&proj).await {
                    if let Ok(new_tasks) = db.list_tasks(None, None).await {
                        tasks.set(new_tasks);
                    }
                }
            } else {
                tasks.set(Vec::new());
            }
        });
    });
}

fn setup_detail_refresh(
    selected: Signal<Option<String>>,
    active_project: Signal<Option<Project>>,
    tasks: Signal<Vec<Task>>,
    task_detail: Signal<Option<TaskDetail>>,
) {
    use_effect(move || {
        let sel = selected();
        let proj = active_project();
        let _tasks = tasks();
        spawn(async move { refresh_detail(sel, proj, task_detail).await });
    });
}

#[component]
fn App() -> Element {
    let tasks: Signal<Vec<Task>> = use_signal(Vec::new);
    let selected: Signal<Option<String>> = use_signal(|| None);
    let filter: Signal<Option<String>> = use_signal(|| None);
    let task_detail: Signal<Option<TaskDetail>> = use_signal(|| None);
    let agent_statuses: Signal<HashMap<String, AgentInfo>> = use_signal(HashMap::new);
    let projects = use_signal(state::discover_projects);
    let active_project: Signal<Option<Project>> = use_signal(|| {
        state::discover_projects().into_iter().next()
    });

    use_future(move || poll_tasks(active_project, tasks));
    use_future(move || poll_agent_status(active_project, agent_statuses));
    setup_project_refresh(active_project, tasks);
    setup_detail_refresh(selected, active_project, tasks, task_detail);

    rsx! {
        style { "{STYLE}" }
        div { class: "app",
            div { class: "drag-region" }
            div { class: "app-body",
                Sidebar { tasks, selected, filter, projects, active_project }
                Detail { detail: task_detail, selected, active_project, tasks, agent_statuses }
            }
        }
    }
}

async fn poll_tasks(
    active_project: Signal<Option<Project>>,
    mut tasks: Signal<Vec<Task>>,
) {
    loop {
        if let Some(proj) = active_project() {
            if let Some(db) = state::open_db_for(&proj).await {
                if let Ok(new_tasks) = db.list_tasks(None, None).await {
                    if state::tasks_changed(&tasks.read(), &new_tasks) {
                        tasks.set(new_tasks);
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn poll_agent_status(
    active_project: Signal<Option<Project>>,
    mut agent_statuses: Signal<HashMap<String, AgentInfo>>,
) {
    loop {
        if let Some(proj) = active_project() {
            let new_statuses = state::fetch_agent_status(&proj).await;
            if *agent_statuses.read() != new_statuses {
                agent_statuses.set(new_statuses);
            }
        } else if !agent_statuses.read().is_empty() {
            agent_statuses.set(HashMap::new());
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
