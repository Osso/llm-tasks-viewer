use llm_tasks::db::{Database, Event, Task};

#[derive(Clone, Debug, PartialEq)]
pub struct TaskDetail {
    pub task: Task,
    pub depends_on: Vec<(String, String, String)>,
    pub blocks: Vec<(String, String, String)>,
    pub events: Vec<Event>,
}

pub async fn open_db() -> Option<Database> {
    let data_dir = dirs::data_dir()?;
    let db_path = data_dir.join("llm-tasks/tasks.db");
    if !db_path.exists() {
        tracing::warn!("database not found: {}", db_path.display());
        return None;
    }
    Database::open(&db_path).await.ok()
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

    let mut depends_on = Vec::new();
    for dep in &deps {
        let (title, status) = match db.get_task(&dep.depends_on).await {
            Ok(t) => (t.title, t.status),
            Err(_) => (dep.depends_on.clone(), "unknown".into()),
        };
        depends_on.push((dep.depends_on.clone(), title, status));
    }

    let mut blocks = Vec::new();
    for dep in &rev_deps {
        let (title, status) = match db.get_task(&dep.task_id).await {
            Ok(t) => (t.title, t.status),
            Err(_) => (dep.task_id.clone(), "unknown".into()),
        };
        blocks.push((dep.task_id.clone(), title, status));
    }

    Some(TaskDetail {
        task,
        depends_on,
        blocks,
        events,
    })
}
