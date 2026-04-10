use crate::ast::*;
use std::path::Path;

/// A task entry extracted from a .gibber task file.
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub status: String,
    pub depends: Vec<String>,
}

/// Scan a directory for T*.gibber task files and extract their info.
pub fn scan_tasks(tasks_dir: &Path) -> Vec<TaskInfo> {
    let mut tasks = Vec::new();

    let entries = match std::fs::read_dir(tasks_dir) {
        Ok(e) => e,
        Err(_) => return tasks,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Only process T*.gibber files (task files)
        if !name.starts_with('T') || !name.ends_with(".gibber") {
            continue;
        }

        if let Some(info) = extract_task_info(&path) {
            tasks.push(info);
        }
    }

    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    tasks
}

/// Extract task info from a single .gibber file.
fn extract_task_info(path: &Path) -> Option<TaskInfo> {
    let content = std::fs::read_to_string(path).ok()?;
    let file = crate::parse(&content).ok()?;
    let form = file.root.as_form()?;

    if form.head != "task" {
        return None;
    }

    let id = form
        .field_value("id")
        .and_then(|v| match v {
            GibberValue::Ident(s) | GibberValue::Symbol(s) => Some(s.clone()),
            _ => None,
        })
        .or_else(|| file.frontmatter.get("id").cloned())
        .unwrap_or_default();

    let title = form
        .field_value("title")
        .and_then(|v| match v {
            GibberValue::Str(s) => Some(s.clone()),
            GibberValue::Symbol(s) | GibberValue::Ident(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let status = form
        .field_value("status")
        .and_then(|v| match v {
            GibberValue::Symbol(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "queued".to_string());

    let depends = form
        .field_value("depends")
        .and_then(|v| v.as_list())
        .map(|list| {
            list.iter()
                .filter_map(|v| match v {
                    GibberValue::Ident(s) | GibberValue::Symbol(s) => Some(s.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    Some(TaskInfo {
        id,
        title,
        status,
        depends,
    })
}

/// Generate a tasks_index.gibber from scanned tasks.
pub fn generate_index(tasks: &[TaskInfo]) -> String {
    let today = chrono_today();

    let active: Vec<&TaskInfo> = tasks
        .iter()
        .filter(|t| t.status == "wip" || t.status == "queued")
        .collect();

    let completed: Vec<&TaskInfo> = tasks.iter().filter(|t| t.status == "done").collect();

    let mut out = String::new();
    out.push_str("---\nid: INDEX\ngibber_dict: meta/v2\n---\n\n");
    out.push_str("(§index §id:INDEX\n");
    out.push_str(&format!("  §updated:{}\n", today));

    // Next task: first wip, or first queued
    let next = tasks
        .iter()
        .find(|t| t.status == "wip")
        .or_else(|| tasks.iter().find(|t| t.status == "queued"));
    if let Some(n) = next {
        out.push_str(&format!("  §next:{}\n", n.id));
    }

    // Active
    let active_ids: Vec<&str> = active.iter().map(|t| t.id.as_str()).collect();
    out.push_str(&format!("  §active:[{}]\n", active_ids.join(" ")));

    // Completed
    let completed_ids: Vec<&str> = completed.iter().map(|t| t.id.as_str()).collect();
    out.push_str(&format!("  §completed:[{}])\n", completed_ids.join(" ")));

    out
}

/// Rebuild tasks_index.gibber from the task files in a directory.
pub fn rebuild_index(tasks_dir: &Path) -> Result<String, String> {
    let tasks = scan_tasks(tasks_dir);
    if tasks.is_empty() {
        return Err("no task files found".to_string());
    }
    let index = generate_index(&tasks);

    let index_path = tasks_dir.join("tasks_index.gibber");
    std::fs::write(&index_path, &index)
        .map_err(|e| format!("failed to write index: {}", e))?;

    Ok(format!(
        "(§result §tool:gibber §cmd:rebuild-index §outcome:§passed §tasks:{} §file:\"{}\")",
        tasks.len(),
        index_path.display()
    ))
}

fn chrono_today() -> String {
    // Simple date without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days = now / 86400;
    let years = (days as f64 / 365.25) as u64;
    let year = 1970 + years;
    // Approximate — good enough for an index timestamp
    let remaining_days = days - (years as f64 * 365.25) as u64;
    let month = (remaining_days / 30).min(11) + 1;
    let day = (remaining_days % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}
