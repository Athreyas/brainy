// Vault data layer: scans the Obsidian vault, parses markdown + frontmatter,
// groups notes by folder, extracts open checkboxes, daily notes, calendar data.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate};
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Note {
    pub path: PathBuf,
    pub rel_path: String,
    pub title: String,
    pub folder: String,
    pub note_type: String,
    pub tags: Vec<String>,
    pub updated: Option<NaiveDate>,
    pub author: String,
    pub body: String,
    pub heading_preview: String,
    pub checkboxes: Vec<Checkbox>,
}

#[derive(Debug, Clone)]
pub struct Checkbox {
    pub done: bool,
    pub text: String,
    pub line_no: usize,
    pub note_title: String,
    pub has_id: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Vault {
    pub notes: Vec<Note>,
    pub folders: Vec<String>,
    pub daily_notes: Vec<Note>,
    pub weekly_notes: Vec<Note>,
}

pub fn vault_path() -> PathBuf {
    if let Ok(p) = std::env::var("BRAINY_VAULT") {
        return PathBuf::from(p);
    }
    PathBuf::from("/Users/art/Library/Mobile Documents/iCloud~md~obsidian/Documents/home-vault")
}

pub fn load_vault(root: &Path) -> Result<Vault> {
    let mut vault = Vault::default();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
    {
        let path = entry.path();
        if path.components().any(|c| {
            matches!(c.as_os_str().to_str(), Some(".obsidian") | Some(".git"))
        }) {
            continue;
        }
        if let Some(n) = parse_note(root, path) {
            vault.notes.push(n);
        }
    }
    vault.notes.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let mut folders: Vec<String> = vault
        .notes
        .iter()
        .map(|n| n.folder.clone())
        .collect();
    folders.sort();
    folders.dedup();
    vault.folders = folders;
    vault.daily_notes = vault
        .notes
        .iter()
        .filter(|n| n.rel_path.starts_with("09-Planning/Daily/"))
        .cloned()
        .collect();
    vault.weekly_notes = vault
        .notes
        .iter()
        .filter(|n| n.rel_path.starts_with("09-Planning/Weekly/"))
        .cloned()
        .collect();
    Ok(vault)
}

fn parse_note(root: &Path, path: &Path) -> Option<Note> {
    let content = fs::read_to_string(path).ok()?;
    let rel = path.strip_prefix(root).ok()?.to_string_lossy().to_string();
    let folder = Path::new(&rel)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| "(root)".into());

    let (meta, body) = split_frontmatter(&content);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let get = |k: &str| -> String {
        meta.get(k)
            .map(|v| {
                match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => String::new(),
                }
            })
            .unwrap_or_default()
    };
    let tags: Vec<String> = match meta.get("tags") {
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) => s.split(',').map(|t| t.trim().to_string()).collect(),
        _ => vec![],
    };
    let updated = get("updated")
        .parse::<NaiveDate>()
        .ok()
        .or_else(|| {
            get("created")
                .parse::<NaiveDate>()
                .ok()
        });

    let title = {
        let t = get("title");
        if t.is_empty() { stem.clone() } else { t }
    };

    let mut checkboxes = Vec::new();
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- [") {
            if let Some(end) = rest.find(']') {
                let mark = &rest[..end];
                let done = mark == "x" || mark == "X";
                let text = rest[end + 1..].trim().to_string();
                checkboxes.push(Checkbox {
                    done,
                    has_id: text.contains("🆔"),
                    text,
                    line_no: i,
                    note_title: title.clone(),
                });
            }
        }
    }

    let heading_preview = body
        .lines()
        .map(|l| l.trim())
        .find(|l| l.starts_with('#') && !l.starts_with("##"))
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .unwrap_or_default();

    Some(Note {
        path: path.to_path_buf(),
        rel_path: rel,
        title,
        folder,
        note_type: get("type"),
        tags,
        updated,
        author: get("author"),
        body,
        heading_preview,
        checkboxes,
    })
}

pub fn split_frontmatter(content: &str) -> (Value, String) {
    let empty = Value::Mapping(serde_yaml::Mapping::new());
    if !content.starts_with("---") {
        return (empty, content.to_string());
    }
    let mut parts = content.splitn(3, "---");
    let _ = parts.next();
    let yaml = parts.next().unwrap_or("");
    let body = parts.next().unwrap_or(content).to_string();
    let meta: Value = serde_yaml::from_str(yaml).unwrap_or(empty);
    (meta, body)
}

/// Counts (year, month, day) -> number of daily-note actions (checkboxes + entries).
pub fn calendar_activity(vault: &Vault) -> Vec<(NaiveDate, String)> {
    let mut out = Vec::new();
    for n in &vault.daily_notes {
        let date_str = Path::new(&n.rel_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Ok(d) = date_str.parse::<NaiveDate>() {
            out.push((d, n.title.clone()));
        }
    }
    out.sort_by_key(|(d, _)| *d);
    out
}

pub fn current_month_days() -> Vec<Option<NaiveDate>> {
    let today = chrono::Local::now().date_naive();
    let mut days = Vec::new();
    for d in 1..=31 {
        if let Some(date) = NaiveDate::from_ymd_opt(today.year(), today.month(), d) {
            days.push(Some(date));
        }
    }
    days
}

pub fn fmt_date(d: &NaiveDate) -> String {
    d.format("%a %b %d").to_string()
}

/// Search across title, tags, body, and checkbox text.
pub fn search(vault: &Vault, query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    let mut hits = Vec::new();
    for n in &vault.notes {
        if n.title.to_lowercase().contains(&q)
            || n.rel_path.to_lowercase().contains(&q)
            || n.body.to_lowercase().contains(&q)
            || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
        {
            hits.push(format!("{}  ·  {}", n.title, n.rel_path));
        }
    }
    hits
}

/// Aggregate checkboxes across the whole vault (excluding inbox/memory/archive).
/// Only high-signal sections: Planning / Projects / Learning / Personal.
/// Truncated "]" parser fragments are skipped — they are table syntax, not tasks.
pub fn all_open_actions(vault: &Vault) -> Vec<(String, Checkbox)> {
    use std::collections::BTreeMap;
    let mut by_note: BTreeMap<String, Vec<Checkbox>> = BTreeMap::new();
    for n in &vault.notes {
        let keep = n.rel_path.starts_with("09-Planning/")
            || n.rel_path.starts_with("01-Projects/")
            || n.rel_path.starts_with("03-Learning/")
            || n.rel_path.starts_with("04-Personal/");
        if !keep {
            continue;
        }
        for c in &n.checkboxes {
            // skip parser artefacts: lines that are just "]" or table fragments
            let txt = c.text.trim();
            if txt.is_empty() || txt.starts_with(']') || txt.starts_with("|") {
                continue;
            }
            by_note.entry(n.rel_path.clone()).or_default().push(c.clone());
        }
    }
    let mut out = Vec::new();
    for (path, items) in by_note {
        for c in items {
            if !c.done {
                out.push((path.clone(), c));
            }
        }
    }
    out
}

pub fn count_by_folder(vault: &Vault) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = vault
        .folders
        .iter()
        .map(|f| {
            (
                f.clone(),
                vault.notes.iter().filter(|n| &n.folder == f).count(),
            )
        })
        .collect();
    counts.sort_by(|a, b| a.0.cmp(&b.0));
    counts
}

pub fn read_note_body(root: &Path, rel: &str) -> Result<String> {
    fs::read_to_string(root.join(rel)).context("read note")
}
