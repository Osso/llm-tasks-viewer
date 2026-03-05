mod detail;
mod sidebar;
mod state;

use dioxus::prelude::*;
use llm_tasks::db::Task;

use crate::detail::Detail;
use crate::sidebar::Sidebar;
use crate::state::TaskDetail;

const STYLE: &str = include_str!("../assets/style.css");

async fn refresh_detail(sel: Option<String>, mut task_detail: Signal<Option<TaskDetail>>) {
    if let Some(id) = sel {
        if let Some(db) = state::open_db().await {
            task_detail.set(state::load_detail(&db, &id).await);
        }
    } else {
        task_detail.set(None);
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

#[component]
fn App() -> Element {
    let mut tasks: Signal<Vec<Task>> = use_signal(Vec::new);
    let selected: Signal<Option<String>> = use_signal(|| None);
    let filter: Signal<Option<String>> = use_signal(|| None);
    let task_detail: Signal<Option<TaskDetail>> = use_signal(|| None);

    // Poll DB every 2 seconds
    use_future(move || async move {
        let Some(db) = state::open_db().await else {
            tracing::error!("failed to open database");
            return;
        };
        loop {
            if let Ok(new_tasks) = db.list_tasks(None, None).await {
                if state::tasks_changed(&tasks.read(), &new_tasks) {
                    tasks.set(new_tasks);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    // Reload detail when selection or tasks change
    use_effect(move || {
        let sel = selected();
        let _tasks = tasks();
        spawn(async move { refresh_detail(sel, task_detail).await });
    });

    rsx! {
        style { "{STYLE}" }
        div { class: "app",
            div { class: "drag-region" }
            div { class: "app-body",
                Sidebar { tasks, selected, filter }
                Detail { detail: task_detail, selected }
            }
        }
    }
}
