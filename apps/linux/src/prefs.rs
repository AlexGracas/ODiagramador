//! Per-user settings (~/.config/odiagramador/settings.json), the remote MCP server and update checks.

use std::fs::File;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use gtk::{gio, glib};
use odg_core::update::{self, UpdateInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub language: Option<String>,
    /// system | light | dark
    pub appearance: String,
    /// Pango font description ("Monospace 11"); empty = system monospace.
    pub editor_font: String,
    pub auto_update: bool,
    pub last_update_check: i64,
    pub mcp_remote: bool,
    pub mcp_port: u16,
    pub mcp_lan: bool,
    pub mcp_token: String,
    pub last_workspace: Option<String>,
    /// Diagram list: "project" | "none" | "kind" | "folder", and "name" | "date".
    pub list_group: String,
    pub list_sort: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            language: None,
            appearance: "system".into(),
            editor_font: String::new(),
            auto_update: true,
            last_update_check: 0,
            mcp_remote: false,
            mcp_port: 7410,
            mcp_lan: false,
            mcp_token: new_token(),
            last_workspace: None,
            list_group: "project".into(),
            list_sort: "name".into(),
        }
    }
}

fn dir() -> PathBuf {
    glib::user_config_dir().join("odiagramador")
}

impl Prefs {
    pub fn load() -> Prefs {
        if let Ok(text) = std::fs::read_to_string(dir().join("settings.json")) {
            return serde_json::from_str(&text).unwrap_or_default();
        }
        // migrate the workspace remembered by 0.1.x
        let last = std::fs::read_to_string(dir().join("last-workspace")).ok().map(|s| s.trim().to_string());
        Prefs { last_workspace: last, ..Prefs::default() }
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(dir());
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir().join("settings.json"), text);
        }
    }
}

pub fn new_token() -> String {
    let mut t = glib::uuid_string_random().replace('-', "");
    t.push_str(&glib::uuid_string_random().replace('-', ""));
    t.truncate(48);
    t
}

/// The bundled `odg` next to this executable, else the one on PATH.
pub fn odg_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("odg")))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "odg".into())
}

/// Address other machines use to reach us (no packet is sent: connect only picks the route).
pub fn lan_address() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

// ------------------------------------------------------------------ remote MCP

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteStatus {
    Stopped,
    Running(String),
    Failed(String),
}

/// `odg mcp --http` exposed on a port so agents on other machines can reach this workspace.
/// Off by default; LAN access always requires the bearer token (odg refuses otherwise).
pub struct RemoteMcp {
    child: Option<Child>,
    pub status: RemoteStatus,
}

impl Default for RemoteMcp {
    fn default() -> Self {
        RemoteMcp { child: None, status: RemoteStatus::Stopped }
    }
}

fn log_file() -> PathBuf {
    dir().join("mcp-remote.log")
}

impl RemoteMcp {
    pub fn url(prefs: &Prefs) -> String {
        let host = if prefs.mcp_lan { lan_address().unwrap_or_else(|| "127.0.0.1".into()) } else { "127.0.0.1".into() };
        format!("http://{host}:{}/mcp", prefs.mcp_port)
    }

    pub fn restart(&mut self, prefs: &Prefs, workspace: Option<&Path>) {
        self.stop();
        let Some(ws) = workspace else { return };
        if !prefs.mcp_remote || prefs.mcp_port == 0 {
            return;
        }
        let bind = format!("{}:{}", if prefs.mcp_lan { "0.0.0.0" } else { "127.0.0.1" }, prefs.mcp_port);
        let _ = std::fs::create_dir_all(dir());
        // stderr goes to a file: a pipe nobody drains could block the server
        let log = File::create(log_file()).map(Stdio::from).unwrap_or_else(|_| Stdio::null());
        let spawned = Command::new(odg_exe())
            .args(["mcp", "--workspace"])
            .arg(ws)
            .args(["--http", &bind])
            .env("ODG_MCP_TOKEN", &prefs.mcp_token)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn();
        match spawned {
            Ok(child) => {
                self.child = Some(child);
                self.status = RemoteStatus::Running(Self::url(prefs));
            }
            Err(e) => self.status = RemoteStatus::Failed(e.to_string()),
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.status = RemoteStatus::Stopped;
    }

    /// Notices a server that exited (port in use, …); true when the status changed.
    pub fn poll(&mut self) -> bool {
        let Some(child) = &mut self.child else { return false };
        match child.try_wait() {
            Ok(Some(_)) => {
                self.child = None;
                let log = std::fs::read_to_string(log_file()).unwrap_or_default();
                let last = log.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("exited").replace("odg: ", "");
                self.status = RemoteStatus::Failed(last);
                true
            }
            _ => false,
        }
    }
}

// ------------------------------------------------------------------ updates

/// Asks GitHub for the latest release (curl, asynchronously) and compares it with this version.
pub fn check_updates(done: impl FnOnce(Result<UpdateInfo, String>) + 'static) {
    let ua = format!("User-Agent: ODiagramador/{}", odg_core::VERSION);
    let argv = ["curl", "-fsSL", "--max-time", "20", "-H", ua.as_str(), "-H", "Accept: application/vnd.github+json", update::RELEASES_API];
    let argv: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    let proc = match gio::Subprocess::newv(&argv, gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE) {
        Ok(p) => p,
        Err(e) => return done(Err(e.to_string())),
    };
    proc.communicate_utf8_async(None, gio::Cancellable::NONE, move |res| {
        let result = match res {
            Ok((Some(out), _)) if !out.is_empty() => update::evaluate(&out, odg_core::VERSION, "linux"),
            Ok(_) => Err("no answer from GitHub".into()),
            Err(e) => Err(e.to_string()),
        };
        done(result);
    });
}

pub fn now() -> i64 {
    glib::real_time() / 1_000_000
}
