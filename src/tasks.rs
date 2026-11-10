// Taskwarrior layer: shells out to `task export`, parses JSON, sorts by
// urgency/due, and can complete or add tasks back via the CLI.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct TaskItem {
    pub id: i64,
    pub description: String,
    pub status: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub urgency: f64,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
}

impl TaskItem {
    pub fn due_date(&self) -> Option<NaiveDate> {
        self.due
            .as_ref()
            .map(|d| d[..8].to_string())
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y%m%d").ok())
    }

    pub fn prio_rank(&self) -> u8 {
        match self.priority.as_deref() {
            Some("H") => 0,
            Some("M") => 1,
            Some("L") => 2,
            _ => 3,
        }
    }

    pub fn is_overdue(&self) -> bool {
        if let Some(d) = self.due_date() {
            (d - chrono::Local::now().date_naive()).num_days() < 0
        } else {
            false
        }
    }

    pub fn is_due_this_week(&self) -> bool {
        if let Some(d) = self.due_date() {
            let days = (d - chrono::Local::now().date_naive()).num_days();
            days >= 0 && days <= 7
        } else {
            false
        }
    }
}

pub fn fetch_tasks() -> Result<Vec<TaskItem>> {
    let out = Command::new("task")
        .args(["export"])
        .output()
        .context("run `task export`")?;
    if !out.status.success() {
        anyhow::bail!("task export failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let mut tasks: Vec<TaskItem> =
        serde_json::from_slice(&out.stdout).context("parse task export JSON")?;
    tasks.sort_by(|a, b| {
        // overdue/due first, then urgency desc
        a.due.cmp(&b.due)
            .then_with(|| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal))
    });
    Ok(tasks)
}

pub fn pending_tasks(tasks: &[TaskItem]) -> Vec<TaskItem> {
    tasks
        .iter()
        .filter(|t| t.status == "pending")
        .cloned()
        .collect()
}

pub fn complete_task(id: i64) -> Result<()> {
    let out = Command::new("task")
        .args([&id.to_string(), "done"])
        .output()
        .context("run `task <id> done`")?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

pub fn add_task(description: &str, project: &str, priority: &str) -> Result<()> {
    let mut args = vec![description.to_string()];
    if !project.is_empty() {
        args.push(format!("project:{}", project));
    }
    if !priority.is_empty() {
        args.push(format!("priority:{}", priority));
    }
    args.push("+vault".into());
    let out = Command::new("task")
        .args(&args)
        .output()
        .context("run `task add`")?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub overdue: usize,
    pub due_this_week: usize,
    pub high_prio: usize,
}

pub fn stats(tasks: &[TaskItem]) -> TaskStats {
    let today = chrono::Local::now().date_naive();
    let mut s = TaskStats::default();
    for t in tasks {
        s.total += 1;
        if t.status == "pending" {
            s.pending += 1;
            if t.prio_rank() == 0 {
                s.high_prio += 1;
            }
            if let Some(d) = t.due_date() {
                let days = (d - today).num_days();
                if days < 0 {
                    s.overdue += 1;
                } else if days <= 7 {
                    s.due_this_week += 1;
                }
            }
        }
    }
    s
}

pub fn group_by_project(tasks: &[TaskItem]) -> Vec<(String, Vec<TaskItem>)> {
    let mut map: std::collections::BTreeMap<String, Vec<TaskItem>> = Default::default();
    for t in tasks.iter().filter(|t| t.status == "pending") {
        let key = if t.project.is_empty() {
            "(no project)".into()
        } else {
            t.project.clone()
        };
        map.entry(key).or_default().push(t.clone());
    }
    map.into_iter().collect()
}
