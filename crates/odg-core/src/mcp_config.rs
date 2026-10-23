//! Configuration snippets the apps copy for MCP clients (Claude Code, Codex, Cursor, Claude Desktop, …).

use serde_json::{json, Value};

/// Local (stdio) snippets, plus HTTP ones when `url` is given (the app exposes MCP on a port).
/// Keys: json, claude_code, codex and, with `url`, remote_json, remote_claude_code, remote_codex.
pub fn snippets(exe: &str, workspace: &str, url: Option<&str>, token: Option<&str>) -> Value {
    let generic = json!({"mcpServers": {"odiagramador": {"command": exe, "args": ["mcp", "--workspace", workspace]}}});
    let mut out = json!({
        "json": serde_json::to_string_pretty(&generic).unwrap(),
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

/// The snippet for one client id: claude-code, codex, or anything else (JSON: Cursor, Claude Desktop, …).
pub fn snippet_for(all: &Value, client: &str, remote: bool) -> String {
    let key = match (client, remote) {
        ("claude-code", false) => "claude_code",
        ("codex", false) => "codex",
        ("claude-code", true) => "remote_claude_code",
        ("codex", true) => "remote_codex",
        (_, true) => "remote_json",
        _ => "json",
    };
    all[key].as_str().or_else(|| all["json"].as_str()).unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_snippets_carry_the_token() {
        let all = snippets("/bin/odg", "/w", Some("http://127.0.0.1:7410/mcp"), Some("abc"));
        assert!(snippet_for(&all, "claude-code", true).contains("--transport http"));
        assert!(snippet_for(&all, "cursor", true).contains("Bearer abc"));
        assert!(snippet_for(&all, "codex", false).contains("command = \"/bin/odg\""));
        assert!(snippets("/bin/odg", "/w", None, None).get("remote_json").is_none());
    }
}
