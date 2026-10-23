//! Preferences window (General · Editor and diagram · MCP), brand icons and icon popover menus.

use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, glib, pango};
use odg_core::{i18n, Lang};

use crate::prefs::{new_token, RemoteMcp, RemoteStatus};
use crate::{apply_appearance, apply_editor_font, check_updates, copy_mcp_config, restart_remote, set_diagram_font, set_language, Ui};

/// MCP clients offered in the menus: (id, name). "json" = any other client.
pub const MCP_CLIENTS: &[(&str, &str)] = &[("claude-code", "Claude Code"), ("codex", "Codex"), ("cursor", "Cursor"), ("claude-desktop", "Claude Desktop")];

const CLAUDE_ORANGE: &str = "#D97757";

fn brand_svg(name: &str) -> Option<&'static str> {
    Some(match name {
        "modelcontextprotocol" => include_str!("../../../assets/icons/modelcontextprotocol.svg"),
        "claude" => include_str!("../../../assets/icons/claude.svg"),
        "claudecode" => include_str!("../../../assets/icons/claudecode.svg"),
        "cursor" => include_str!("../../../assets/icons/cursor.svg"),
        _ => return None,
    })
}

/// Brand mark (Simple Icons, CC0) as a 16px image; monochrome marks follow the light/dark style.
pub fn brand_image(name: &str, fill: Option<&str>) -> Option<gtk::Image> {
    let svg = brand_svg(name)?;
    let dark = adw::StyleManager::default().is_dark();
    let color = fill.unwrap_or(if dark { "#f2f2f2" } else { "#1f1f1f" });
    let svg = svg.replacen("<path ", &format!("<path fill=\"{color}\" "), 1);
    let texture = gdk::Texture::from_bytes(&glib::Bytes::from_owned(svg.into_bytes())).ok()?;
    let image = gtk::Image::from_paintable(Some(&texture));
    image.set_pixel_size(16);
    Some(image)
}

pub fn client_icon(id: &str) -> gtk::Image {
    let brand = match id {
        "claude-code" => brand_image("claudecode", Some(CLAUDE_ORANGE)),
        "claude-desktop" => brand_image("claude", Some(CLAUDE_ORANGE)),
        "cursor" => brand_image("cursor", None),
        "mcp" => brand_image("modelcontextprotocol", None),
        _ => None,
    };
    brand.unwrap_or_else(|| gtk::Image::from_icon_name(if id == "codex" { "utilities-terminal-symbolic" } else { "text-x-generic-symbolic" }))
}

/// First icon name the current theme has (icon names differ between themes).
pub fn themed_icon(candidates: &[&str]) -> String {
    let theme = gdk::Display::default().map(|d| gtk::IconTheme::for_display(&d));
    candidates
        .iter()
        .find(|n| theme.as_ref().is_some_and(|t| t.has_icon(n)))
        .unwrap_or(&candidates[candidates.len() - 1])
        .to_string()
}

pub struct MenuEntry {
    pub icon: Option<gtk::Image>,
    pub label: String,
    pub sensitive: bool,
    pub action: Option<Box<dyn Fn()>>,
}

impl MenuEntry {
    pub fn new(icon: gtk::Image, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        MenuEntry { icon: Some(icon), label: label.into(), sensitive: true, action: Some(Box::new(action)) }
    }

    pub fn heading(label: impl Into<String>) -> Self {
        MenuEntry { icon: None, label: label.into(), sensitive: false, action: None }
    }

    pub fn separator() -> Self {
        MenuEntry { icon: None, label: String::new(), sensitive: false, action: None }
    }
}

/// A popover of flat icon+label buttons (GMenu popovers do not reliably show item icons).
pub fn icon_popover(entries: Vec<MenuEntry>) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
    column.set_margin_top(4);
    column.set_margin_bottom(4);
    for entry in entries {
        if entry.label.is_empty() {
            column.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            continue;
        }
        if entry.action.is_none() {
            let label = gtk::Label::builder().label(&entry.label).xalign(0.0).css_classes(["dim-label", "caption-heading"]).margin_start(10).margin_top(4).margin_bottom(2).build();
            column.append(&label);
            continue;
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        if let Some(icon) = entry.icon {
            row.append(&icon);
        }
        row.append(&gtk::Label::builder().label(&entry.label).xalign(0.0).build());
        let button = gtk::Button::builder().child(&row).css_classes(["flat"]).sensitive(entry.sensitive).build();
        let action = entry.action.unwrap();
        let pop = popover.downgrade();
        button.connect_clicked(move |_| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
            action();
        });
        column.append(&button);
    }
    popover.set_child(Some(&column));
    popover
}

// ------------------------------------------------------------------ preferences window

pub fn show_preferences(ui: &Ui, page: &str) {
    let (lang, prefs, diagram_font) = {
        let s = ui.state.borrow();
        (s.lang, s.prefs.clone(), s.diagram_font.clone())
    };
    let t = |k: &str| i18n::t(lang, k, &[]);
    let window = adw::PreferencesWindow::builder().title(t("ui-settings")).transient_for(&ui.window).modal(false).default_width(720).default_height(640).build();

    // ---- General
    let general = adw::PreferencesPage::builder().title(t("ui-settings-general")).icon_name("emblem-system-symbolic").name("general").build();
    let group = adw::PreferencesGroup::new();
    let language = adw::ComboRow::builder().title(t("ui-language")).model(&gtk::StringList::new(&["English", "Português (Brasil)"])).build();
    language.set_selected(if lang == Lang::PtBr { 1 } else { 0 });
    let (u, w) = (ui.clone(), window.downgrade());
    language.connect_selected_notify(move |row| {
        set_language(&u, if row.selected() == 1 { Lang::PtBr } else { Lang::En });
        // rebuild in the new language
        if let Some(w) = w.upgrade() {
            w.close();
        }
        show_preferences(&u, "general");
    });
    group.add(&language);
    let appearance = adw::ComboRow::builder()
        .title(t("ui-appearance"))
        .model(&gtk::StringList::new(&[&t("ui-appearance-system"), &t("ui-appearance-light"), &t("ui-appearance-dark")]))
        .build();
    appearance.set_selected(match prefs.appearance.as_str() { "light" => 1, "dark" => 2, _ => 0 });
    let u = ui.clone();
    appearance.connect_selected_notify(move |row| {
        let value = ["system", "light", "dark"][row.selected().min(2) as usize];
        u.state.borrow_mut().prefs.appearance = value.into();
        apply_appearance(&u);
    });
    group.add(&appearance);
    general.add(&group);

    let updates = adw::PreferencesGroup::builder().title(t("ui-updates")).description(i18n::t(lang, "ui-version", &[("version", odg_core::VERSION.into())])).build();
    let auto = adw::SwitchRow::builder().title(t("ui-auto-update")).active(prefs.auto_update).build();
    let u = ui.clone();
    auto.connect_active_notify(move |row| {
        let mut s = u.state.borrow_mut();
        s.prefs.auto_update = row.is_active();
        s.prefs.save();
    });
    updates.add(&auto);
    let check = gtk::Button::builder().label(t("ui-check-updates")).valign(gtk::Align::Center).build();
    let u = ui.clone();
    check.connect_clicked(move |_| check_updates(&u, true));
    let check_row = adw::ActionRow::builder().title(t("ui-check-updates")).build();
    check_row.add_suffix(&check);
    updates.add(&check_row);
    general.add(&updates);
    window.add(&general);

    // ---- Editor and diagram
    let editor = adw::PreferencesPage::builder().title(t("ui-settings-editor")).icon_name("accessories-text-editor-symbolic").name("editor").build();
    let group = adw::PreferencesGroup::new();
    let font_button = gtk::FontDialogButton::new(Some(gtk::FontDialog::new()));
    font_button.set_valign(gtk::Align::Center);
    font_button.set_font_desc(&pango::FontDescription::from_string(if prefs.editor_font.is_empty() { "Monospace 11" } else { &prefs.editor_font }));
    let u = ui.clone();
    font_button.connect_font_desc_notify(move |b| {
        if let Some(desc) = b.font_desc() {
            u.state.borrow_mut().prefs.editor_font = desc.to_string();
            apply_editor_font(&u);
        }
    });
    let font_row = adw::ActionRow::builder().title(t("ui-editor-font")).build();
    font_row.add_suffix(&font_button);
    group.add(&font_row);
    editor.add(&group);
    let diagram = adw::PreferencesGroup::builder().title(t("ui-diagram-font")).description(t("ui-diagram-font-hint")).build();
    let font_entry = adw::EntryRow::builder().title(t("ui-font-default")).text(&diagram_font).show_apply_button(true).build();
    let u = ui.clone();
    font_entry.connect_apply(move |row| set_diagram_font(&u, row.text().trim()));
    diagram.add(&font_entry);
    editor.add(&diagram);
    window.add(&editor);

    // ---- MCP
    let mcp = adw::PreferencesPage::builder().title(t("ui-settings-mcp")).icon_name("network-server-symbolic").name("mcp").build();
    let local = adw::PreferencesGroup::builder().title(t("ui-mcp-local")).description(t("ui-mcp-local-hint")).build();
    add_client_rows(ui, &local, false, lang);
    mcp.add(&local);

    let remote = adw::PreferencesGroup::builder().title(t("ui-mcp-remote")).description(format!("⚠ {}", t("ui-mcp-remote-warning"))).build();
    let enable = adw::SwitchRow::builder().title(t("ui-mcp-remote-enable")).active(prefs.mcp_remote).build();
    let port = adw::SpinRow::with_range(1.0, 65535.0, 1.0);
    port.set_title(&t("ui-mcp-port"));
    port.set_value(prefs.mcp_port as f64);
    let scope = adw::ComboRow::builder().title(t("ui-mcp-scope")).model(&gtk::StringList::new(&[&t("ui-mcp-scope-local"), &t("ui-mcp-scope-lan")])).build();
    scope.set_selected(if prefs.mcp_lan { 1 } else { 0 });
    let token = adw::ActionRow::builder().title(t("ui-mcp-token")).subtitle(&prefs.mcp_token).subtitle_selectable(true).build();
    let regenerate = gtk::Button::builder().icon_name("view-refresh-symbolic").tooltip_text(t("ui-mcp-regenerate")).valign(gtk::Align::Center).css_classes(["flat"]).build();
    token.add_suffix(&regenerate);
    let status = adw::ActionRow::new();
    remote.add(&enable);
    remote.add(&port);
    remote.add(&scope);
    remote.add(&token);
    remote.add(&status);
    let details: Vec<gtk::Widget> = vec![port.clone().upcast(), scope.clone().upcast(), token.clone().upcast()];
    let set_sensitive = move |on: bool| details.iter().for_each(|w| w.set_sensitive(on));
    set_sensitive(prefs.mcp_remote);

    let u = ui.clone();
    enable.connect_active_notify(move |row| {
        u.state.borrow_mut().prefs.mcp_remote = row.is_active();
        set_sensitive(row.is_active());
        restart_remote(&u);
    });
    let u = ui.clone();
    port.connect_value_notify(move |row| {
        let value = row.value() as u16;
        if u.state.borrow().prefs.mcp_port != value {
            u.state.borrow_mut().prefs.mcp_port = value;
            restart_remote(&u);
        }
    });
    let u = ui.clone();
    scope.connect_selected_notify(move |row| {
        u.state.borrow_mut().prefs.mcp_lan = row.selected() == 1;
        restart_remote(&u);
    });
    let (u, token_row) = (ui.clone(), token.downgrade());
    regenerate.connect_clicked(move |_| {
        let new = new_token();
        u.state.borrow_mut().prefs.mcp_token = new.clone();
        if let Some(r) = token_row.upgrade() {
            r.set_subtitle(&new);
        }
        restart_remote(&u);
    });
    mcp.add(&remote);

    let copy_remote = adw::PreferencesGroup::builder().title(t("ui-mcp-copy-remote")).build();
    add_client_rows(ui, &copy_remote, true, lang);
    mcp.add(&copy_remote);
    window.add(&mcp);

    // keep the server status current while the window is open
    let (u, status_row) = (ui.clone(), status.downgrade());
    let refresh = move || -> glib::ControlFlow {
        let Some(row) = status_row.upgrade() else { return glib::ControlFlow::Break };
        let s = u.state.borrow();
        let (icon, text) = match &s.remote.status {
            RemoteStatus::Running(_) => ("●", i18n::t(s.lang, "ui-mcp-running", &[("url", RemoteMcp::url(&s.prefs).as_str().into())])),
            RemoteStatus::Failed(d) => ("●", i18n::t(s.lang, "ui-mcp-error", &[("detail", d.as_str().into())])),
            RemoteStatus::Stopped => ("○", i18n::t(s.lang, "ui-mcp-stopped", &[])),
        };
        row.set_title(&format!("{icon}  {text}"));
        row.set_css_classes(match &s.remote.status {
            RemoteStatus::Running(_) => &["success"],
            RemoteStatus::Failed(_) => &["error"],
            RemoteStatus::Stopped => &["dim-label"],
        });
        glib::ControlFlow::Continue
    };
    refresh();
    glib::timeout_add_local(Duration::from_millis(800), refresh);

    window.set_visible_page_name(page);
    window.present();
}

fn add_client_rows(ui: &Ui, group: &adw::PreferencesGroup, remote: bool, lang: Lang) {
    let other = i18n::t(lang, "ui-mcp-other", &[]);
    let clients = MCP_CLIENTS.iter().map(|(id, name)| (*id, name.to_string())).chain(std::iter::once(("json", other)));
    for (id, name) in clients {
        let row = adw::ActionRow::builder().title(&name).activatable(true).build();
        row.add_prefix(&client_icon(id));
        row.add_suffix(&gtk::Image::from_icon_name("edit-copy-symbolic"));
        let u = ui.clone();
        row.connect_activated(move |_| copy_mcp_config(&u, id, remote));
        group.add(&row);
    }
}
