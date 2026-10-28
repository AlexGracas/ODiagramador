//! Configuration snippets the apps copy or install for MCP clients (Claude Code, Codex, Cursor, Claude Desktop, Kiro, Antigravity, …).

use std::path::{Path, PathBuf};
use serde_json::{json, Value};

/// Returns the server configuration value for a client.
pub fn server_config(client: &str, exe: &str, workspace: &str, url: Option<&str>, token: Option<&str>) -> Value {
    if let Some(url) = url {
        let mut server = json!({"type": "http", "url": url});
        if let Some(t) = token {
            server["headers"] = json!({"Authorization": format!("Bearer {t}")});
        }
        server
    } else {
        match client {
            "kiro" => json!({
                "command": exe,
                "args": ["mcp", "--workspace", workspace],
                "disabled": false,
                "autoApprove": []
            }),
            "claude-code" => json!({
                "type": "stdio",
                "command": exe,
                "args": ["mcp", "--workspace", workspace],
                "env": {}
            }),
            _ => json!({
                "command": exe,
                "args": ["mcp", "--workspace", workspace]
            }),
        }
    }
}

/// Local (stdio) snippets, plus HTTP ones when `url` is given (the app exposes MCP on a port).
/// Keys: json, claude_code, codex and, with `url`, remote_json, remote_claude_code, remote_codex.
pub fn snippets(exe: &str, workspace: &str, url: Option<&str>, token: Option<&str>) -> Value {
    let generic = json!({"mcpServers": {"odiagramador": {"command": exe, "args": ["mcp", "--workspace", workspace]}}});
    let kiro_val = json!({"mcpServers": {"odiagramador": server_config("kiro", exe, workspace, None, None)}});
    let mut out = json!({
        "json": serde_json::to_string_pretty(&generic).unwrap(),
        "kiro": serde_json::to_string_pretty(&kiro_val).unwrap(),
        "claude_code": format!("claude mcp add odiagramador -- \"{exe}\" mcp --workspace \"{workspace}\""),
        "codex": format!("[mcp_servers.odiagramador]\ncommand = {exe:?}\nargs = [\"mcp\", \"--workspace\", {workspace:?}]"),
    });
    if let Some(url) = url {
        let mut server = json!({"type": "http", "url": url});
        if let Some(t) = token {
            server["headers"] = json!({"Authorization": format!("Bearer {t}")});
        }
        let header = token.map(|t| format!(" --header \"Authorization: Bearer {t}\"")).unwrap_or_default();
        out["remote_json"] = json!(serde_json::to_string_pretty(&json!({"mcpServers": {"odiagramador": server}})).unwrap());
        out["remote_claude_code"] = json!(format!("claude mcp add --transport http odiagramador {url}{header}"));
        out["remote_codex"] = json!(match token {
            Some(t) => format!("[mcp_servers.odiagramador]\nurl = {url:?}\nhttp_headers = {{ \"Authorization\" = \"Bearer {t}\" }}"),
            None => format!("[mcp_servers.odiagramador]\nurl = {url:?}"),
        });
    }
    out
}

/// The snippet for one client id: claude-code, codex, kiro, or anything else (JSON: Cursor, Claude Desktop, Antigravity, …).
pub fn snippet_for(all: &Value, client: &str, remote: bool) -> String {
    let key = match (client, remote) {
        ("claude-code", false) => "claude_code",
        ("codex", false) => "codex",
        ("kiro", false) => "kiro",
        ("claude-code", true) => "remote_claude_code",
        ("codex", true) => "remote_codex",
        (_, true) => "remote_json",
        _ => "json",
    };
    all[key].as_str().or_else(|| all["json"].as_str()).unwrap_or_default().to_string()
}

/// Finds candidate paths for a client configuration on the local machine (including WSL -> Windows mapping if available).
pub fn candidate_config_paths(client: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(h) = std::env::var("HOME") {
        roots.push(PathBuf::from(h));
    }
    if let Ok(u) = std::env::var("USERPROFILE") {
        roots.push(PathBuf::from(u));
    }
    // If on WSL, check Windows user profile directory as well
    if Path::new("/mnt/c/Users").is_dir() {
        if let Ok(out) = std::process::Command::new("cmd.exe").args(["/c", "echo %USERPROFILE%"]).output() {
            let win_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if win_path.len() >= 3 && win_path.chars().nth(1) == Some(':') {
                let drive = win_path.chars().next().unwrap().to_ascii_lowercase();
                let rest = win_path[3..].replace('\\', "/");
                let wsl_path = PathBuf::from(format!("/mnt/{drive}/{rest}"));
                if wsl_path.is_dir() && !roots.contains(&wsl_path) {
                    roots.push(wsl_path);
                }
            }
        }
    }

    let mut paths = Vec::new();
    for root in &roots {
        match client {
            "kiro" => {
                paths.push(root.join(".kiro/settings/mcp.json"));
            }
            "claude-desktop" => {
                paths.push(root.join("AppData/Roaming/Claude/claude_desktop_config.json"));
                paths.push(root.join(".config/Claude/claude_desktop_config.json"));
                paths.push(root.join("Library/Application Support/Claude/claude_desktop_config.json"));
            }
            "antigravity" => {
                paths.push(root.join(".gemini/antigravity/mcp_config.json"));
                paths.push(root.join(".gemini/config/mcp_config.json"));
            }
            "claude-code" => {
                paths.push(root.join(".claude.json"));
            }
            "cursor" => {
                paths.push(root.join(".cursor/mcp.json"));
            }
            _ => {}
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        if client == "claude-desktop" {
            paths.push(PathBuf::from(appdata).join("Claude/claude_desktop_config.json"));
        }
    }
    paths
}

/// Directly injects the `odiagramador` entry into the client's configuration file.
/// Returns Ok(path_written) if successful, or Err(msg) if direct installation was not possible.
pub fn install_to_client(client: &str, exe: &str, workspace: &str, url: Option<&str>, token: Option<&str>) -> Result<PathBuf, String> {
    let targets = candidate_config_paths(client);
    if targets.is_empty() {
        return Err("No candidate configuration path found for client".into());
    }

    // Try target paths where the file or its parent directory already exists
    let mut last_err = String::from("No existing configuration directory found for client");
    for path in targets {
        if path.exists() || path.parent().is_some_and(|p| p.exists()) {
            match update_json_mcp_config(&path, client, exe, workspace, url, token) {
                Ok(()) => return Ok(path),
                Err(e) => last_err = e,
            }
        }
    }
    Err(last_err)
}

fn update_json_mcp_config(path: &Path, client: &str, exe: &str, workspace: &str, url: Option<&str>, token: Option<&str>) -> Result<(), String> {
    let mut root_val: Value = if path.exists() {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !root_val.is_object() {
        root_val = json!({});
    }

    let server_entry = server_config(client, exe, workspace, url, token);

    let obj = root_val.as_object_mut().unwrap();
    let mcp_servers = obj.entry("mcpServers").or_insert_with(|| json!({}));
    if !mcp_servers.is_object() {
        *mcp_servers = json!({});
    }
    mcp_servers.as_object_mut().unwrap().insert("odiagramador".to_string(), server_entry);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    let formatted = serde_json::to_string_pretty(&root_val).map_err(|e| e.to_string())?;
    std::fs::write(path, formatted).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_snippets_carry_the_token() {
        let all = snippets("/bin/odg", "/w", Some("http://127.0.0.1:7410/mcp"), Some("abc"));
        assert!(snippet_for(&all, "claude-code", true).contains("--transport http"));
        assert!(snippet_for(&all, "cursor", true).contains("Bearer abc"));
        assert!(snippet_for(&all, "kiro", false).contains("autoApprove"));
        assert!(snippet_for(&all, "codex", false).contains("command = \"/bin/odg\""));
        assert!(snippets("/bin/odg", "/w", None, None).get("remote_json").is_none());
    }

    #[test]
    fn updates_json_mcp_config_preserves_existing_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("mcp.json");
        std::fs::write(&file, r#"{"mcpServers":{"other":{"command":"foo"}},"customKey":123}"#).unwrap();

        update_json_mcp_config(&file, "kiro", "/bin/odg", "/my/ws", None, None).unwrap();

        let updated: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(updated["customKey"], 123);
        assert_eq!(updated["mcpServers"]["other"]["command"], "foo");
        assert_eq!(updated["mcpServers"]["odiagramador"]["command"], "/bin/odg");
        assert_eq!(updated["mcpServers"]["odiagramador"]["args"][1], "--workspace");
        assert_eq!(updated["mcpServers"]["odiagramador"]["args"][2], "/my/ws");
        assert_eq!(updated["mcpServers"]["odiagramador"]["disabled"], false);
    }
}
