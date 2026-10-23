//! "Is there a newer ODiagramador?" — the apps fetch GitHub's latest-release JSON themselves
//! (native HTTP stack, proxy settings, no TLS dependency in the core) and hand it here to decide
//! whether to offer an update and which package fits the platform.

use serde::{Deserialize, Serialize};

pub const RELEASES_API: &str = "https://api.github.com/repos/AlexGracas/ODiagramador/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/AlexGracas/ODiagramador/releases/latest";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current: String,
    pub latest: String,
    /// Release page (always set, for "see what's new" or manual download).
    pub page_url: String,
    /// Package for this platform, when the release has one.
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
    /// What changed since `current` (the release's "what's new" sections newer than it), Markdown.
    pub notes: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// `v1.2.3`, `1.2`, `0.1.1-beta` → (1, 2, 3) / (1, 2, 0) / (0, 1, 1).
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.trim().trim_start_matches(['v', 'V']).split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>());
    let major = it.next()?.ok()?;
    let minor = it.next().unwrap_or(Ok(0)).ok()?;
    let patch = it.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

fn asset_matches(name: &str, platform: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".zip")
        && match platform {
            "macos" => n.contains("macos"),
            "windows" => n.contains("windows"),
            "linux" => n.contains("linux"),
            _ => false,
        }
}

/// `platform`: `macos` | `windows` | `linux`.
pub fn evaluate(release_json: &str, current: &str, platform: &str) -> Result<UpdateInfo, String> {
    let rel: Release = serde_json::from_str(release_json).map_err(|e| format!("unexpected release data: {e}"))?;
    let latest = rel.tag_name.trim_start_matches(['v', 'V']).to_string();
    let newer = match (parse_version(&rel.tag_name), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    };
    // prefer the universal macOS build when several match
    let mut assets: Vec<&Asset> = rel.assets.iter().filter(|a| asset_matches(&a.name, platform)).collect();
    assets.sort_by_key(|a| !a.name.contains("universal"));
    let asset = assets.first();
    Ok(UpdateInfo {
        available: newer && !rel.draft && !rel.prerelease,
        current: current.trim_start_matches(['v', 'V']).to_string(),
        latest,
        page_url: if rel.html_url.is_empty() { RELEASES_PAGE.to_string() } else { rel.html_url },
        asset_name: asset.map(|a| a.name.clone()),
        asset_url: asset.map(|a| a.browser_download_url.clone()),
        asset_size: asset.map(|a| a.size),
        notes: changes_since(&rel.body, current).chars().take(6000).collect(),
    })
}

/// The `## …` sections of release notes whose heading names a version newer than `current`
/// ("Novidades da 0.3.0", "What's new in 0.3.0"), newest first as written. Without such sections,
/// the whole text.
pub fn changes_since(notes: &str, current: &str) -> String {
    let Some(cur) = parse_version(current) else { return notes.to_string() };
    let mut out = String::new();
    let mut keep = false;
    for line in notes.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            let version = title.split_whitespace().find_map(|w| parse_version(w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.')).filter(|_| w.contains('.')));
            keep = version.is_some_and(|v| v > cur);
        } else if line.starts_with("# ") {
            keep = false;
        }
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if out.trim().is_empty() {
        notes.to_string()
    } else {
        out.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REL: &str = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/x/y/releases/tag/v0.2.0","body":"notes",
        "draft":false,"prerelease":false,"assets":[
        {"name":"ODiagramador-0.2.0-windows-x64.zip","browser_download_url":"https://dl/w.zip","size":10},
        {"name":"ODiagramador-0.2.0-macos-arm64.zip","browser_download_url":"https://dl/a.zip","size":5},
        {"name":"ODiagramador-0.2.0-macos-universal.zip","browser_download_url":"https://dl/m.zip","size":20}]}"#;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.1.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("0.3.0-beta.1"), Some((0, 3, 0)));
        assert_eq!(parse_version("abc"), None);
        assert!(parse_version("0.10.0") > parse_version("0.9.9"));
    }

    #[test]
    fn picks_platform_asset() {
        let mac = evaluate(REL, "0.1.1", "macos").unwrap();
        assert!(mac.available);
        assert_eq!(mac.latest, "0.2.0");
        assert_eq!(mac.asset_url.as_deref(), Some("https://dl/m.zip"));
        let win = evaluate(REL, "v0.1.1", "windows").unwrap();
        assert_eq!(win.asset_name.as_deref(), Some("ODiagramador-0.2.0-windows-x64.zip"));
        let linux = evaluate(REL, "0.1.1", "linux").unwrap();
        assert!(linux.available && linux.asset_url.is_none());
        assert!(!evaluate(REL, "0.2.0", "macos").unwrap().available);
        assert!(!evaluate(REL, "0.3.0", "macos").unwrap().available);
        assert!(evaluate("not json", "0.1.0", "macos").is_err());
    }

    #[test]
    fn notes_keep_only_newer_changes() {
        let notes = "Intro\n\n## Downloads\n| a | b |\n\n## Novidades da 0.3.0\n- agent\n\n## Novidades da 0.2.2\n- icon\n\n## Novidades da 0.2.1\n- log\n\n## Funcionalidades\n- all";
        assert_eq!(changes_since(notes, "0.2.1"), "## Novidades da 0.3.0\n- agent\n\n## Novidades da 0.2.2\n- icon");
        assert_eq!(changes_since("plain notes", "0.1.0"), "plain notes");
    }
}
