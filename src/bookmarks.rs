// Browser bookmarks layer: reads Chromium `Bookmarks` JSON (Dia / Chrome /
// Arc / Edge / Brave / Vivaldi share this format) from every profile found on
// the machine, flattens the tree, and buckets bookmarks into smart categories.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct BookmarkFile {
    pub roots: std::collections::HashMap<String, Node>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    #[serde(rename = "type")]
    pub node_type: String,
    pub name: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub date_added: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub name: String,
    pub url: String,
    pub host: String,
    pub folder: String,
    pub browser: String,
    pub date_added: Option<String>,
}

/// Chromium-based browsers that keep a `Bookmarks` file under these roots.
pub fn browser_roots() -> Vec<(String, PathBuf)> {
    let home = dirs_home();
    let base = home.join("Library/Application Support");
    let candidates = [
        ("Dia", "Dia/User Data"),
        ("Arc", "Arc/User Data"),
        ("Chrome", "Google/Chrome"),
        ("Edge", "Microsoft Edge"),
        ("Brave", "BraveSoftware/Brave-Browser"),
        ("Vivaldi", "Vivaldi"),
    ];
    let mut out = Vec::new();
    for (name, rel) in candidates {
        let dir = base.join(rel);
        if !dir.exists() {
            continue;
        }
        let mut profiles: Vec<PathBuf> = Vec::new();
        if dir.join("Default/Bookmarks").exists() {
            profiles.push(dir.join("Default"));
        }
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.join("Bookmarks").exists()
                    && p.file_name()
                        .map(|n| n.to_string_lossy().starts_with("Profile"))
                        .unwrap_or(false)
                {
                    profiles.push(p);
                }
            }
        }
        for p in profiles {
            out.push((name.to_string(), p.join("Bookmarks")));
        }
    }
    out
}

pub fn load_bookmarks() -> Vec<Bookmark> {
    let mut all = Vec::new();
    for (browser, path) in browser_roots() {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<BookmarkFile>(&text) else {
            continue;
        };
        let mut flat = Vec::new();
        for root in parsed.roots.values() {
            flatten(root, "", &mut flat);
        }
        for b in flat {
            all.push(Bookmark {
                browser: browser.clone(),
                ..b
            });
        }
    }
    // dedupe by URL, keep first, sort by folder then name
    let mut seen = std::collections::HashSet::new();
    all.retain(|b| seen.insert(b.url.clone()));
    all.sort_by(|a, b| {
        a.folder
            .cmp(&b.folder)
            .then_with(|| a.name.cmp(&b.name))
    });
    all
}

fn flatten(node: &Node, folder: &str, out: &mut Vec<Bookmark>) {
    if node.node_type == "folder" {
        let f = node
            .name
            .as_deref()
            .map(|n| {
                if folder.is_empty() {
                    n.to_string()
                } else {
                    format!("{folder}/{n}")
                }
            })
            .unwrap_or_else(|| folder.to_string());
        for c in &node.children {
            flatten(c, &f, out);
        }
    } else if node.node_type == "url" {
        if let Some(url) = &node.url {
            out.push(Bookmark {
                host: host_of(url),
                folder: folder.to_string(),
                name: node.name.clone().unwrap_or_else(|| url.clone()),
                url: url.clone(),
                browser: String::new(),
                date_added: node.date_added.clone(),
            });
        }
    }
}

pub fn host_of(url: &str) -> String {
    let u = url.trim_start_matches("https://").trim_start_matches("http://");
    let u = u.split('/').next().unwrap_or(u);
    let u = u.split('?').next().unwrap_or(u);
    u.trim_start_matches("www.").to_string()
}

/// Smart bucketing by host + folder + URL keyword. First match wins,
/// checked in order, no fall-through.
pub fn categorize(b: &Bookmark) -> String {
    let host = b.host.to_lowercase();
    let folder = b.folder.to_lowercase();
    let url = b.url.to_lowercase();
    let host_is = |d: &str| host == d || host.ends_with(&format!(".{d}"));
    let folder_has = |k: &str| folder.contains(k);
    let url_has = |k: &str| url.contains(k);

    // Vault / second-brain knowledge links
    if folder_has("vault") || url_has("monk97.me") || host_is("monk97.me") {
        return "Vault & Docs".into();
    }
    // Home lab / infra (matches the vault's 05-References runbooks)
    if host_is("lab.monk97.me")
        || url_has("tailscale.com")
        || url_has("synology")
        || url_has("portainer")
        || url_has("proxmox")
        || url_has("cloudflare")
        || host_is("github.com") && url_has("infrastructure")
    {
        return "Home Lab / Infra".into();
    }
    // Google / Firebase / AI tooling
    if host_is("google.com")
        || host_is("firebase.google.com")
        || host_is("cloud.google.com")
        || host_is("gdg.community.dev")
        || host_is("jules.google.com")
        || folder_has("firebase")
        || folder_has("genkit")
        || url_has("vertex-ai")
        || url_has("gemini")
    {
        return "Google / Firebase / AI".into();
    }
    // Code hosting / dev
    if host_is("github.com")
        || host_is("gitlab.com")
        || host_is("code.visualstudio.com")
        || host_is("stackoverflow.com")
        || host_is("dev.to")
    {
        return "Code / Dev".into();
    }
    // Learning sites
    if host_is("edx.org")
        || host_is("coursera.org")
        || host_is("udemy.com")
        || host_is("docs.python.org")
        || host_is("developer.mozilla.org")
        || host_is("wikipedia.org")
        || folder_has("gyaan")
        || folder_has("learning")
    {
        return "Learning".into();
    }
    // Video
    if host_is("youtube.com") || host_is("youtu.be") || host_is("vimeo.com") {
        return "Video".into();
    }
    // Social / mail / productivity
    if host_is("gmail.com")
        || host_is("mail.google.com")
        || host_is("notion.site")
        || host_is("notion.so")
        || host_is("x.com")
        || host_is("twitter.com")
        || host_is("instagram.com")
        || host_is("linkedin.com")
        || host_is("reddit.com")
    {
        return "Social / Productivity".into();
    }
    // Everything else, grouped by folder intent when present
    if folder_has("bookmarks bar") && !folder_has("personal") {
        return "Misc".into();
    }
    "Unsorted".into()
}

pub fn grouped(bookmarks: &[Bookmark]) -> Vec<(String, Vec<Bookmark>)> {
    let mut map: BTreeMap<String, Vec<Bookmark>> = BTreeMap::new();
    for b in bookmarks {
        let key = categorize(b);
        map.entry(key).or_default().push(b.clone());
    }
    map.into_iter().collect()
}

/// "What exactly is this?" — short human description from host + path guess.
pub fn describe(b: &Bookmark) -> String {
    let mut s = b.host.clone();
    let path = b
        .url
        .split(&b.host)
        .nth(1)
        .unwrap_or("")
        .trim_start_matches('/')
        .replace(['-', '_'], " ")
        .to_lowercase();
    if !path.is_empty() {
        let short: String = path.chars().take(56).collect();
        s.push_str(&format!(" · {short}"));
    }
    s
}

pub fn by_browser(bookmarks: &[Bookmark]) -> Vec<(String, usize)> {
    let mut map: BTreeMap<String, usize> = BTreeMap::new();
    for b in bookmarks {
        *map.entry(b.browser.clone()).or_default() += 1;
    }
    map.into_iter().collect()
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/Users/art"))
}
