use std::path::PathBuf;

use llm_tasks::db::{Database, Event, Task};

#[derive(Clone, Debug, PartialEq)]
pub struct TaskDetail {
    pub task: Task,
    pub depends_on: Vec<(String, String, String)>,
    pub blocks: Vec<(String, String, String)>,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub name: String,
    pub db_path: PathBuf,
}

pub fn discover_projects() -> Vec<Project> {
    let Some(data_dir) = dirs::data_dir() else {
        return Vec::new();
    };
    let mut projects = Vec::new();

    // Legacy llm-tasks location
    let legacy = data_dir.join("llm-tasks/tasks.db");
    if legacy.exists() {
        projects.push(Project {
            name: "llm-tasks".into(),
            db_path: legacy,
        });
    }

    // agent-orchestrator per-project DBs
    let orch_dir = data_dir.join("agent-orchestrator");
    if let Ok(entries) = std::fs::read_dir(&orch_dir) {
        for entry in entries.flatten() {
            let db_path = entry.path().join("tasks.db");
            if db_path.exists() {
                let name = entry.file_name().to_string_lossy().into_owned();
                projects.push(Project { name, db_path });
            }
        }
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

pub async fn open_db_for(project: &Project) -> Option<Database> {
    Database::open(&project.db_path).await.ok()
}

pub fn tasks_changed(old: &[Task], new: &[Task]) -> bool {
    if old.len() != new.len() {
        return true;
    }
    old.iter()
        .zip(new.iter())
        .any(|(a, b)| a.id != b.id || a.updated_at != b.updated_at)
}

pub async fn load_detail(db: &Database, task_id: &str) -> Option<TaskDetail> {
    let task = db.get_task(task_id).await.ok()?;
    let deps = db.get_dependencies(task_id).await.unwrap_or_default();
    let rev_deps = db
        .get_reverse_dependencies(task_id)
        .await
        .unwrap_or_default();
    let events = db.get_events(task_id).await.unwrap_or_default();

    let depends_on = collect_dep_details(db, &deps, |d| &d.depends_on).await;
    let blocks = collect_dep_details(db, &rev_deps, |d| &d.task_id).await;

    Some(TaskDetail {
        task,
        depends_on,
        blocks,
        events,
    })
}

async fn collect_dep_details(
    db: &Database,
    deps: &[llm_tasks::db::Dependency],
    id_fn: fn(&llm_tasks::db::Dependency) -> &String,
) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for dep in deps {
        let id = id_fn(dep);
        let (title, status) = match db.get_task(id).await {
            Ok(t) => (t.title, t.status),
            Err(_) => (id.clone(), "unknown".into()),
        };
        out.push((id.clone(), title, status));
    }
    out
}
