//! Agent ⇄ human channel through the workspace (no sockets, works with any number of agents and viewers).
//!
//! - `.odiagramador/messages.jsonl` — append-only conversation. Agent entries are also commands for the
//!   desktop viewer: `show` (open a diagram), `highlight` (put elements in evidence), `view` (zoom / centre on
//!   an element), `notify` (a message), `ask` (a question with optional choices). The human's replies are `answer` entries (`reply_to` = question id).
//! - `.odiagramador/viewer.json` — presence: the desktop app rewrites it every few seconds with what the human
//!   is looking at, so an agent knows whether anyone will see its message.

use std::fs;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::workspace::{Actor, Event, Workspace, WsResult};
use crate::{now_rfc3339, sha256_hex};

/// Presence older than this means the viewer is closed (it refreshes every ~2 s).
pub const PRESENCE_TTL_SECS: u64 = 10;
/// Agent messages this recent are still delivered to a viewer that opens afterwards (e.g. launched by `show`).
pub const LATE_DELIVERY_SECS: u64 = 120;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub ts: String,
    /// Seconds since the Unix epoch (for ages and ordering).
    pub epoch: u64,
    pub from: Option<Actor>,
    /// show | highlight | view | notify | ask | answer
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// info | success | warning | error
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Choices offered by an `ask`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// `ask`: whether a free-text reply is allowed besides the options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_text: Option<bool>,
    /// `view`: zoom factor (1 = 100 %).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
    /// `view`: fit the whole diagram in the window (before zoom / centre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit: Option<bool>,
    /// `view`: element to bring to the centre of the screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_on: Option<String>,
    /// `answer`: the question it replies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// Agent/client name, e.g. "claude-code".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Presence {
    pub app: String,
    pub version: String,
    pub platform: String,
    pub pid: u32,
    /// Seconds since the Unix epoch of the last refresh.
    pub updated: u64,
    /// Diagram on screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Element the human selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    /// Current zoom factor of the preview (1 = 100 %).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
    /// Element closest to the centre of the preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_element: Option<String>,
    /// Elements (at least partly) on screen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visible_elements: Vec<String>,
}

impl Presence {
    pub fn is_live(&self) -> bool {
        self.updated > 0 && epoch_now().saturating_sub(self.updated) <= PRESENCE_TTL_SECS
    }
}

/// New agent messages for the viewer, and where to continue reading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inbox {
    pub cursor: u64,
    pub messages: Vec<Message>,
}

pub fn epoch_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Workspace {
    fn messages_file(&self) -> PathBuf {
        self.state_dir().join("messages.jsonl")
    }

    fn presence_file(&self) -> PathBuf {
        self.state_dir().join("viewer.json")
    }

    /// Appends a message (id and timestamps are filled in) and records it in the activity log.
    pub fn post_message(&self, mut msg: Message) -> WsResult<Message> {
        if let Some(p) = &msg.path {
            msg.path = Some(self.rel(p)?);
        }
        msg.ts = now_rfc3339();
        msg.epoch = epoch_now();
        let seed = format!("{}{}{:?}{}", msg.ts, msg.kind, msg.text, std::process::id());
        let prefix = if msg.kind == "ask" { "q" } else { "msg" };
        msg.id = format!("{prefix}-{}", &sha256_hex(seed.as_bytes())[..10]);
        fs::create_dir_all(self.state_dir())?;
        let mut f = fs::OpenOptions::new().create(true).append(true).open(self.messages_file())?;
        writeln!(f, "{}", serde_json::to_string(&msg)?)?;
        self.log(Event {
            ts: msg.ts.clone(),
            actor: msg.from.unwrap_or(Actor::Agent),
            action: msg.kind.clone(),
            path: msg.path.clone().unwrap_or_default(),
            valid: None,
            errors: None,
            warnings: None,
            note: msg.text.as_ref().map(|t| t.chars().take(200).collect()),
            client: msg.client.clone(),
        })?;
        Ok(msg)
    }

    /// The human's reply to a question.
    pub fn answer(&self, question_id: &str, text: &str) -> WsResult<Message> {
        let question = self.all_messages()?.into_iter().find(|m| m.id == question_id);
        self.post_message(Message {
            from: Some(Actor::Human),
            kind: "answer".into(),
            text: Some(text.to_string()),
            reply_to: Some(question_id.to_string()),
            path: question.as_ref().and_then(|q| q.path.clone()),
            ..Default::default()
        })
    }

    pub fn all_messages(&self) -> WsResult<Vec<Message>> {
        match fs::read_to_string(self.messages_file()) {
            Ok(t) => Ok(t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    /// Most recent `limit` messages, oldest first.
    pub fn recent_messages(&self, limit: usize) -> WsResult<Vec<Message>> {
        let all = self.all_messages()?;
        Ok(all[all.len().saturating_sub(limit)..].to_vec())
    }

    pub fn answer_to(&self, question_id: &str) -> WsResult<Option<Message>> {
        Ok(self.all_messages()?.into_iter().find(|m| m.kind == "answer" && m.reply_to.as_deref() == Some(question_id)))
    }

    /// Agent messages for the viewer. Without a cursor (viewer just opened): recent messages and every
    /// question still unanswered; with a cursor: whatever the agents wrote since.
    pub fn viewer_inbox(&self, cursor: Option<u64>) -> WsResult<Inbox> {
        let file = self.messages_file();
        let Ok(mut f) = fs::File::open(&file) else { return Ok(Inbox::default()) };
        let len = f.metadata()?.len();
        let from_agent = |m: &Message| m.from != Some(Actor::Human) && m.kind != "answer";
        match cursor {
            Some(c) if c <= len => {
                f.seek(SeekFrom::Start(c))?;
                let mut text = String::new();
                f.read_to_string(&mut text)?;
                // only complete lines: a writer may be in the middle of one
                let complete = text.rfind('\n').map(|i| i + 1).unwrap_or(0);
                let messages = text[..complete].lines().filter_map(|l| serde_json::from_str::<Message>(l).ok()).filter(from_agent).collect();
                Ok(Inbox { cursor: c + complete as u64, messages })
            }
            _ => {
                let all = self.all_messages()?;
                let now = epoch_now();
                let answered: Vec<&str> = all.iter().filter_map(|m| m.reply_to.as_deref()).collect();
                let messages = all
                    .iter()
                    .filter(|m| from_agent(m))
                    .filter(|m| {
                        if m.kind == "ask" {
                            // questions stay until answered (for a day), however old
                            return !answered.contains(&m.id.as_str()) && now.saturating_sub(m.epoch) <= 24 * 3600;
                        }
                        now.saturating_sub(m.epoch) <= LATE_DELIVERY_SECS
                    })
                    .cloned()
                    .collect();
                Ok(Inbox { cursor: len, messages })
            }
        }
    }

    pub fn set_presence(&self, presence: &Presence) -> WsResult<()> {
        fs::create_dir_all(self.state_dir())?;
        let mut p = presence.clone();
        p.updated = epoch_now();
        fs::write(self.presence_file(), serde_json::to_vec_pretty(&p)?)?;
        Ok(())
    }

    pub fn clear_presence(&self) -> WsResult<()> {
        match fs::remove_file(self.presence_file()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// The viewer's presence, live or not (check [`Presence::is_live`]).
    pub fn presence(&self) -> Option<Presence> {
        fs::read_to_string(self.presence_file()).ok().and_then(|t| serde_json::from_str(&t).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> (tempfile::TempDir, Workspace) {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        (dir, ws)
    }

    fn agent(kind: &str, text: &str) -> Message {
        Message { from: Some(Actor::Agent), kind: kind.into(), text: Some(text.into()), ..Default::default() }
    }

    #[test]
    fn inbox_delivers_new_agent_messages_once() {
        let (_d, ws) = ws();
        ws.post_message(agent("notify", "old")).unwrap();
        let first = ws.viewer_inbox(None).unwrap();
        assert_eq!(first.messages.len(), 1, "recent messages reach a viewer that just opened");
        let q = ws.post_message(Message { options: vec!["Sim".into(), "Não".into()], ..agent("ask", "Posso?") }).unwrap();
        assert!(q.id.starts_with("q-"));
        let next = ws.viewer_inbox(Some(first.cursor)).unwrap();
        assert_eq!(next.messages.len(), 1);
        assert_eq!(next.messages[0].kind, "ask");
        ws.answer(&q.id, "Sim").unwrap();
        let after = ws.viewer_inbox(Some(next.cursor)).unwrap();
        assert!(after.messages.is_empty(), "the human's own answer is not sent back to the viewer");
        let reopened = ws.viewer_inbox(None).unwrap();
        assert!(reopened.messages.iter().all(|m| m.kind != "ask"), "an answered question is not asked again");
        assert_eq!(ws.answer_to(&q.id).unwrap().unwrap().text.as_deref(), Some("Sim"));
        assert_eq!(ws.events(10, None).unwrap()[0].action, "answer");
    }

    #[test]
    fn presence_expires() {
        let (_d, ws) = ws();
        assert!(ws.presence().is_none());
        ws.set_presence(&Presence { app: "test".into(), path: Some("diagrams/a.mmd".into()), ..Default::default() }).unwrap();
        assert!(ws.presence().unwrap().is_live());
        let stale = Presence { updated: epoch_now() - PRESENCE_TTL_SECS - 5, ..Default::default() };
        assert!(!stale.is_live());
        ws.clear_presence().unwrap();
        assert!(ws.presence().is_none());
    }
}
