//! ODiagramador for Linux: GTK4 + libadwaita UI, WebKitGTK preview, GtkSourceView editor.
//! Uses odg-core directly (no FFI) and embeds the shared render-kit.
//!
//!   odiagramador [--workspace DIR] [odiagramador://open?workspace=…&path=…]
#![allow(deprecated)] // adw::MessageDialog keeps us compatible with libadwaita 1.4 (Ubuntu 24.04)

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use base64::Engine as _;
use gtk::{gdk, gio, glib};
use odg_core::analyze::Analysis;
use odg_core::lint::LintConfig;
use odg_core::model::RawRender;
use odg_core::workspace::{Actor, DiagramInfo, Event, Feedback};
use odg_core::{build_report, i18n, source, Lang, Report, Workspace};
use serde_json::{json, Value};
use sourceview::prelude::*;
use webkit::prelude::*;

mod prefs;
mod settings;

use prefs::{Prefs, RemoteMcp};
use settings::{client_icon, icon_popover, themed_icon, MenuEntry, MCP_CLIENTS};

const APP_ID: &str = "io.odiagramador.app";

const ASSETS: &[(&str, &[u8], &str)] = &[
    ("/index.html", include_bytes!("../../../render-kit/index.html"), "text/html"),
    ("/bridge.js", include_bytes!("../../../render-kit/bridge.js"), "text/javascript"),
    ("/palette.js", include_bytes!("../../../render-kit/palette.js"), "text/javascript"),
    ("/renderers.js", include_bytes!("../../../render-kit/renderers.js"), "text/javascript"),
    ("/viewer.css", include_bytes!("../../../render-kit/viewer.css"), "text/css"),
    ("/vendor/mermaid-odg.min.js", include_bytes!("../../../render-kit/vendor/mermaid-odg.min.js"), "text/javascript"),
];

struct State {
    ws: Option<Workspace>,
    lang: Lang,
    strings: BTreeMap<String, String>,
    diagrams: Vec<DiagramInfo>,
    selected: Option<String>,
    saved_source: String,
    report: Option<Report>,
    selected_element: Option<String>,
    render_seq: u64,
    render_source: String,
    edit_gen: u64,
    loading: bool,
    viewer_ready: bool,
    pending: Vec<Value>,
    last_svg: Option<String>,
    last_png: Option<String>,
    modified: BTreeMap<String, String>,
    fit_next: bool,
    dark: bool,
    prefs: Prefs,
    remote: RemoteMcp,
    diagram_font: String,
    update: Option<odg_core::update::UpdateInfo>,
    /// Read position in the agents' message log (None = viewer just opened on this folder).
    inbox_cursor: Option<u64>,
    presence_at: i64,
    /// What the human sees (from the viewer): zoom, element in the centre, elements on screen.
    view_zoom: Option<f64>,
    view_center: Option<String>,
    view_visible: Vec<String>,
    /// The comment context was edited by the human: keep it until sent or regenerated.
    context_edited: bool,
    context_setting: bool,
    collapsed_groups: std::collections::HashSet<String>,
}

impl State {
    fn t(&self, key: &str) -> String {
        self.strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

#[derive(Clone)]
struct Ui {
    window: adw::ApplicationWindow,
    title: adw::WindowTitle,
    list: gtk::ListBox,
    group_dd: gtk::DropDown,
    sort_dd: gtk::DropDown,
    /// set while the list options are being (re)filled, so their change handlers stay quiet
    list_opts_busy: Rc<Cell<bool>>,
    collapse_list: gtk::Button,
    buffer: sourceview::Buffer,
    editor: sourceview::View,
    web: webkit::WebView,
    problems: gtk::ListBox,
    feedback: gtk::ListBox,
    activity: gtk::ListBox,
    notebook: gtk::Notebook,
    feedback_entry: gtk::Entry,
    feedback_button: gtk::Button,
    tab_help: Vec<gtk::Label>,
    /// strips left by the collapsed code / bottom panel (button, name label)
    code_rail: Collapsible,
    bottom_rail: Collapsible,
    list_rail: Collapsible,
    context_expander: gtk::Expander,
    context_view: gtk::TextView,
    context_include: gtk::CheckButton,
    context_regen: gtk::Button,
    context_hint: gtk::Label,
    status: gtk::Label,
    agent_badge: gtk::Label,
    open_button: gtk::Button,
    new_button: gtk::Button,
    properties_button: gtk::Button,
    export_button: gtk::MenuButton,
    mcp_button: gtk::MenuButton,
    update_button: gtk::Button,
    settings_button: gtk::Button,
    toast: adw::ToastOverlay,
    editor_css: gtk::CssProvider,
    state: Rc<RefCell<State>>,
}

thread_local! {
    /// The window of this (single) instance; later launches reuse it.
    static MAIN_UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

fn main() -> glib::ExitCode {
    // one instance per session: agents open the viewer through odiagramador:// links, and each link used to
    // start another window; GApplication now hands later command lines to the running instance
    let app = adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE).build();
    app.connect_command_line(|app, cmd| {
        let args: Vec<String> = cmd.arguments().iter().skip(1).map(|a| a.to_string_lossy().to_string()).collect();
        match MAIN_UI.with(|u| u.borrow().clone()) {
            Some(ui) => handle_external_launch(&ui, &args),
            None => build_ui(app, &args),
        }
        glib::ExitCode::SUCCESS
    });
    app.run()
}

/// Double-click on the paned's bar collapses one side (`end`: the end child, else the start child) and
/// restores it to its previous size; the bar shows a grip icon and is wider so it is easy to hit.
/// Thickness of the strip a collapsed pane leaves.
const RAIL: i32 = 26;

/// Double-click the bar to collapse / restore one child of `paned` (the start one, or the end one). A
/// collapsed child leaves a labelled strip (its name and an arrow) that brings it back when clicked, so it
/// is clear what is hidden. Returns the strip, its label (filled with the localised name later) and a
/// function that collapses (true) or restores (false) the child.
#[derive(Clone)]
struct Collapsible {
    rail: gtk::Button,
    label: gtk::Label,
    set: Rc<dyn Fn(bool)>,
}

fn make_collapsible(paned: &gtk::Paned, end: bool) -> Collapsible {
    paned.set_wide_handle(true);
    paned.add_css_class("odg-split");
    paned.set_shrink_start_child(true);
    paned.set_shrink_end_child(true);
    let horizontal = paned.orientation() == gtk::Orientation::Horizontal;
    let content = if end { paned.end_child() } else { paned.start_child() }.expect("paned child");
    // strip: arrow + name (letters stacked when the strip is vertical)
    let label = gtk::Label::builder().css_classes(["caption-heading", "dim-label"]).justify(gtk::Justification::Center).build();
    let arrow = gtk::Image::from_icon_name(match (horizontal, end) {
        (true, false) => "go-next-symbolic",
        (true, true) => "go-previous-symbolic",
        (false, true) => "go-up-symbolic",
        (false, false) => "go-down-symbolic",
    });
    let inner = gtk::Box::new(if horizontal { gtk::Orientation::Vertical } else { gtk::Orientation::Horizontal }, 8);
    inner.append(&arrow);
    inner.append(&label);
    inner.set_valign(if horizontal { gtk::Align::Start } else { gtk::Align::Center });
    inner.set_halign(if horizontal { gtk::Align::Center } else { gtk::Align::Start });
    inner.set_margin_top(if horizontal { 10 } else { 0 });
    inner.set_margin_start(if horizontal { 0 } else { 10 });
    let rail = gtk::Button::builder().child(&inner).css_classes(["flat"]).visible(false).build();
    rail.set_cursor_from_name(Some("pointer"));
    if horizontal { rail.set_width_request(RAIL) } else { rail.set_height_request(RAIL) }
    let wrap = gtk::Box::new(if horizontal { gtk::Orientation::Horizontal } else { gtk::Orientation::Vertical }, 0);
    if end { paned.set_end_child(None::<&gtk::Widget>) } else { paned.set_start_child(None::<&gtk::Widget>) }
    content.set_hexpand(true);
    content.set_vexpand(true);
    wrap.append(&rail);
    wrap.append(&content);
    if end { paned.set_end_child(Some(&wrap)) } else { paned.set_start_child(Some(&wrap)) }

    let saved = Rc::new(Cell::new(0i32));
    let set_collapsed: Rc<dyn Fn(bool)> = {
        let (p, rail, content, saved) = (paned.clone(), rail.clone(), content.clone(), saved.clone());
        Rc::new(move |collapse| {
            let max = p.max_position();
            if collapse {
                saved.set(p.position());
                p.set_position(if end { max - RAIL } else { RAIL });
            } else {
                let back = saved.get();
                p.set_position(if back > RAIL * 2 && back < max - RAIL * 2 { back } else if end { max * 3 / 4 } else { max / 3 });
            }
            content.set_visible(!collapse);
            rail.set_visible(collapse);
        })
    };
    let f = set_collapsed.clone();
    rail.connect_clicked(move |_| f(false));
    let set = set_collapsed.clone();
    let click = gtk::GestureClick::builder().button(1).propagation_phase(gtk::PropagationPhase::Capture).build();
    let (p, r) = (paned.clone(), rail.clone());
    click.connect_pressed(move |g, n, x, y| {
        let along = if horizontal { x } else { y };
        if n != 2 || (along - p.position() as f64).abs() > 10.0 {
            return; // not on the bar: let the children have the click
        }
        g.set_state(gtk::EventSequenceState::Claimed);
        set_collapsed(!r.is_visible());
    });
    paned.add_controller(click);
    Collapsible { rail, label, set }
}

/// A later launch (an agent's odiagramador:// link, or the user starting the app again) reuses this window.
fn handle_external_launch(ui: &Ui, args: &[String]) {
    let (ws, path) = parse_args(args);
    if let Some(dir) = ws.filter(|d| d.is_dir()) {
        let current = ui.state.borrow().ws.as_ref().map(|w| w.root().to_path_buf());
        if current.as_deref() != std::fs::canonicalize(&dir).ok().as_deref() {
            open_workspace(ui, dir);
        }
    }
    if let Some(p) = path {
        refresh_list(ui);
        if ui.state.borrow().diagrams.iter().any(|d| d.path == p) {
            select(ui, Some(p.clone()));
            let mut i = 0;
            while let Some(row) = ui.list.row_at_index(i) {
                if row.widget_name() == p.as_str() {
                    ui.list.select_row(Some(&row));
                    break;
                }
                i += 1;
            }
        }
    }
    ui.window.present();
}

/// [--workspace DIR] [odiagramador://open?workspace=…&path=…] → (workspace, diagram).
fn parse_args(args: &[String]) -> (Option<PathBuf>, Option<String>) {
    let mut ws: Option<PathBuf> = None;
    let mut path = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--workspace" {
            ws = it.next().map(PathBuf::from);
        } else if a.starts_with("odiagramador://") {
            let (w, p) = parse_open_url(a);
            ws = w.map(PathBuf::from).or(ws);
            path = p;
        }
    }
    (ws, path)
}

/// The app icon, installed in the user's icon theme so windows, the dock and the .desktop entry can use it.
fn install_icon() {
    const SVG: &[u8] = include_bytes!("../../../assets/app-icon/odiagramador.svg");
    let dir = glib::user_data_dir().join("icons/hicolor/scalable/apps");
    let path = dir.join(format!("{APP_ID}.svg"));
    if std::fs::read(&path).ok().as_deref() != Some(SVG) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&path, SVG);
    }
    if let Some(display) = gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_search_path(glib::user_data_dir().join("icons"));
    }
    gtk::Window::set_default_icon_name(APP_ID);
}

fn build_ui(app: &adw::Application, args: &[String]) {
    install_icon();
    let prefs = Prefs::load();
    let lang = prefs.language.as_deref().map(Lang::parse).unwrap_or_else(Lang::from_env);
    let state = Rc::new(RefCell::new(State {
        ws: None,
        lang,
        strings: i18n::all_strings(lang),
        diagrams: vec![],
        selected: None,
        saved_source: String::new(),
        report: None,
        selected_element: None,
        render_seq: 0,
        render_source: String::new(),
        edit_gen: 0,
        loading: false,
        viewer_ready: false,
        pending: vec![],
        last_svg: None,
        last_png: None,
        modified: BTreeMap::new(),
        fit_next: true,
        dark: adw::StyleManager::default().is_dark(),
        prefs,
        remote: RemoteMcp::default(),
        diagram_font: String::new(),
        update: None,
        inbox_cursor: None,
        presence_at: 0,
        view_zoom: None,
        view_center: None,
        view_visible: vec![],
        context_edited: false,
        context_setting: false,
        collapsed_groups: std::collections::HashSet::new(),
    }));

    // ---- header
    let title = adw::WindowTitle::new("ODiagramador", "");
    let header = adw::HeaderBar::builder().title_widget(&title).build();
    let open_button = gtk::Button::from_icon_name("folder-open-symbolic");
    let new_button = gtk::Button::from_icon_name("document-new-symbolic");
    let properties_button = gtk::Button::builder().icon_name("document-properties-symbolic").sensitive(false).build();
    let export_button = gtk::MenuButton::builder()
        .icon_name(themed_icon(&["document-export-symbolic", "document-send-symbolic", "document-save-as-symbolic"]))
        .build();
    let mcp_button = gtk::MenuButton::builder().child(&client_icon("mcp")).build();
    let update_button = gtk::Button::builder().icon_name("software-update-available-symbolic").css_classes(["suggested-action"]).visible(false).build();
    let settings_button = gtk::Button::from_icon_name("emblem-system-symbolic");
    header.pack_start(&open_button);
    header.pack_start(&new_button);
    header.pack_start(&properties_button);
    header.pack_start(&export_button);
    header.pack_end(&settings_button);
    header.pack_end(&update_button);
    header.pack_end(&mcp_button);

    // ---- sidebar
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Single).css_classes(["navigation-sidebar"]).build();
    let list_scroll = gtk::ScrolledWindow::builder().child(&list).vexpand(true).build();
    // how the list is organised: group (none / type / folder) and sort (name / newest first)
    let group_dd = gtk::DropDown::builder().hexpand(true).css_classes(["caption"]).build();
    let sort_dd = gtk::DropDown::builder().hexpand(true).css_classes(["caption"]).build();
    let pickers = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(4).homogeneous(true).hexpand(true).build();
    pickers.append(&group_dd);
    pickers.append(&sort_dd);
    // hides the list: it leaves a "DIAGRAMS" strip that brings it back
    let collapse_list = gtk::Button::builder().icon_name("go-first-symbolic").css_classes(["flat"]).build();
    let list_opts = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(4).margin_start(6).margin_end(6).margin_top(6).margin_bottom(2).build();
    list_opts.append(&pickers);
    list_opts.append(&collapse_list);
    let sidebar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).width_request(220).build();
    sidebar.append(&list_opts);
    sidebar.append(&list_scroll);

    // ---- editor
    let buffer = sourceview::Buffer::new(None);
    buffer.create_tag(Some("error-line"), &[("background", &"rgba(209,36,47,0.22)")]);
    buffer.create_tag(Some("warning-line"), &[("background", &"rgba(217,119,6,0.16)")]);
    let editor = sourceview::View::with_buffer(&buffer);
    editor.set_show_line_numbers(true);
    editor.set_highlight_current_line(true);
    editor.set_monospace(true);
    editor.set_top_margin(6);
    editor.add_css_class("odg-editor");
    let split_css = gtk::CssProvider::new();
    split_css.load_from_string(
        "paned.odg-split.horizontal > separator { min-width: 9px; background-image: -gtk-icontheme(\"view-more-symbolic\"); background-repeat: no-repeat; background-position: center; background-size: 12px 12px; }\n\
         paned.odg-split.vertical > separator { min-height: 9px; background-image: -gtk-icontheme(\"view-more-horizontal-symbolic\"); background-repeat: no-repeat; background-position: center; background-size: 12px 12px; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &split_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
    let editor_css = gtk::CssProvider::new();
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &editor_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
    let editor_scroll = gtk::ScrolledWindow::builder().child(&editor).vexpand(true).hexpand(true).build();
    let agent_badge = gtk::Label::builder().css_classes(["accent", "caption"]).halign(gtk::Align::End).visible(false).build();
    let editor_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor_box.append(&agent_badge);
    editor_box.append(&editor_scroll);

    // ---- preview (shared render-kit in WebKitGTK)
    let ucm = webkit::UserContentManager::new();
    ucm.register_script_message_handler("odg", None);
    let context = webkit::WebContext::new();
    context.register_uri_scheme("odg", serve_render_kit);
    if let Some(sec) = context.security_manager() {
        sec.register_uri_scheme_as_secure("odg");
        sec.register_uri_scheme_as_cors_enabled("odg");
    }
    let web = webkit::WebView::builder().user_content_manager(&ucm).web_context(&context).hexpand(true).vexpand(true).build();
    web.load_uri("odg://localhost/index.html?mode=viewer");

    let hpaned = gtk::Paned::builder().orientation(gtk::Orientation::Horizontal).start_child(&editor_box).end_child(&web).position(420).build();

    // ---- bottom panel
    let notebook = gtk::Notebook::new();
    let problems = gtk::ListBox::builder().css_classes(["boxed-list-separate"]).build();
    let feedback = gtk::ListBox::new();
    let activity = gtk::ListBox::new();
    let feedback_entry = gtk::Entry::builder().hexpand(true).build();
    let feedback_button = gtk::Button::new();
    let fb_row = gtk::Box::builder().spacing(6).margin_start(6).margin_end(6).margin_bottom(6).margin_top(6).build();
    fb_row.append(&feedback_entry);
    fb_row.append(&feedback_button);
    // context sent with the comment (diagram, selected element, its lines), editable
    let context_view = gtk::TextView::builder().monospace(true).wrap_mode(gtk::WrapMode::WordChar).top_margin(4).left_margin(4).build();
    let context_include = gtk::CheckButton::builder().active(true).build();
    let context_regen = gtk::Button::builder().css_classes(["flat"]).build();
    let context_hint = gtk::Label::builder().css_classes(["dim-label", "caption"]).ellipsize(gtk::pango::EllipsizeMode::End).xalign(0.0).hexpand(true).build();
    let context_tools = gtk::Box::builder().spacing(8).build();
    context_tools.append(&context_include);
    context_tools.append(&context_regen);
    context_tools.append(&context_hint);
    let context_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    context_box.append(&gtk::ScrolledWindow::builder().child(&context_view).min_content_height(72).build());
    context_box.append(&context_tools);
    let context_expander = gtk::Expander::builder().expanded(true).child(&context_box).margin_start(6).margin_end(6).margin_bottom(6).build();
    let fb_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    fb_box.append(&gtk::ScrolledWindow::builder().child(&feedback).vexpand(true).build());
    fb_box.append(&fb_row);
    fb_box.append(&context_expander);
    // each page starts with one line saying what it is for
    let tab_help: Vec<gtk::Label> = (0..3)
        .map(|_| gtk::Label::builder().xalign(0.0).wrap(true).css_classes(["dim-label", "caption"]).margin_start(8).margin_end(8).margin_top(4).margin_bottom(4).build())
        .collect();
    let page = |help: &gtk::Label, content: &gtk::Widget| {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
        b.append(help);
        b.append(content);
        b
    };
    notebook.append_page(&page(&tab_help[0], gtk::ScrolledWindow::builder().child(&problems).vexpand(true).build().upcast_ref()), Some(&gtk::Label::new(None)));
    notebook.append_page(&page(&tab_help[1], fb_box.upcast_ref()), Some(&gtk::Label::new(None)));
    notebook.append_page(&page(&tab_help[2], gtk::ScrolledWindow::builder().child(&activity).vexpand(true).build().upcast_ref()), Some(&gtk::Label::new(None)));
    let status = gtk::Label::builder().xalign(0.0).ellipsize(gtk::pango::EllipsizeMode::End).css_classes(["dim-label", "caption"]).margin_start(8).build();
    let bottom = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bottom.append(&status);
    bottom.append(&notebook);

    let vpaned = gtk::Paned::builder().orientation(gtk::Orientation::Vertical).start_child(&hpaned).end_child(&bottom).position(470).build();
    let main_paned = gtk::Paned::builder().orientation(gtk::Orientation::Horizontal).start_child(&sidebar).end_child(&vpaned).position(230).build();
    // double-click a bar to collapse / restore the code (start of hpaned) or the bottom panel (end of vpaned)
    let code_rail = make_collapsible(&hpaned, false);
    let bottom_rail = make_collapsible(&vpaned, true);
    let list_rail = make_collapsible(&main_paned, false);
    let f = list_rail.set.clone();
    collapse_list.connect_clicked(move |_| f(true));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&header);
    content.append(&main_paned);
    let toast = adw::ToastOverlay::new();
    toast.set_child(Some(&content));
    let window = adw::ApplicationWindow::builder().application(app).default_width(1280).default_height(820).content(&toast).build();

    let ui = Ui {
        window,
        title,
        list,
        group_dd,
        sort_dd,
        list_opts_busy: Rc::new(Cell::new(false)),
        collapse_list,
        buffer,
        editor,
        web,
        problems,
        feedback,
        activity,
        notebook,
        feedback_entry,
        feedback_button,
        tab_help,
        code_rail,
        bottom_rail,
        list_rail,
        context_expander,
        context_view,
        context_include,
        context_regen,
        context_hint,
        status,
        agent_badge,
        open_button,
        new_button,
        properties_button,
        export_button,
        mcp_button,
        update_button,
        settings_button,
        toast,
        editor_css,
        state,
    };
    apply_appearance(&ui);
    apply_editor_font(&ui);
    apply_strings(&ui);
    wire(&ui, &ucm);

    MAIN_UI.with(|u| *u.borrow_mut() = Some(ui.clone()));
    // workspace from args / URL / last session
    let (ws_arg, open_path) = parse_args(args);
    let ws_arg = ws_arg.or_else(|| ui.state.borrow().prefs.last_workspace.clone().map(PathBuf::from));
    if let Some(dir) = ws_arg.filter(|d| d.is_dir()) {
        open_workspace(&ui, dir);
        if let Some(p) = open_path {
            select(&ui, Some(p));
        }
    }

    let ui2 = ui.clone();
    glib::timeout_add_local(Duration::from_millis(1000), move || {
        poll(&ui2);
        glib::ControlFlow::Continue
    });
    ui.window.present();

    let due = {
        let p = &ui.state.borrow().prefs;
        p.auto_update && prefs::now() - p.last_update_check > 24 * 3600
    };
    if due {
        check_updates(&ui, false);
    }
}

fn parse_open_url(url: &str) -> (Option<String>, Option<String>) {
    let query = url.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut w = None;
    let mut p = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = glib::Uri::unescape_string(v, None::<&str>).map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
        match k {
            "workspace" => w = Some(v),
            "path" => p = Some(v),
            _ => {}
        }
    }
    (w, p)
}

fn serve_render_kit(req: &webkit::URISchemeRequest) {
    let path = req.path().map(|p| p.to_string()).unwrap_or_default();
    match ASSETS.iter().find(|(p, _, _)| *p == path) {
        Some((_, bytes, mime)) => {
            let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_static(bytes));
            req.finish(&stream, bytes.len() as i64, Some(mime));
        }
        None => {
            let mut err = glib::Error::new(gio::IOErrorEnum::NotFound, "not found");
            req.finish_error(&mut err);
        }
    }
}

// ------------------------------------------------------------------ strings

fn apply_strings(ui: &Ui) {
    let s = ui.state.borrow();
    ui.open_button.set_tooltip_text(Some(&s.t("ui-open-folder")));
    ui.new_button.set_tooltip_text(Some(&s.t("ui-new-diagram")));
    ui.properties_button.set_tooltip_text(Some(&s.t("ui-properties")));
    ui.export_button.set_tooltip_text(Some(&s.t("ui-export")));
    ui.mcp_button.set_tooltip_text(Some(&s.t("ui-mcp-configure")));
    ui.settings_button.set_tooltip_text(Some(&s.t("ui-settings")));
    ui.feedback_button.set_label(&s.t("ui-add-feedback"));
    ui.context_expander.set_label(Some(&s.t("ui-feedback-context")));
    ui.context_include.set_label(Some(&s.t("ui-feedback-context-include")));
    ui.context_regen.set_label(&s.t("ui-feedback-context-reset"));
    ui.context_hint.set_label(&s.t("ui-feedback-context-hint"));
    ui.feedback_entry.set_placeholder_text(Some(&s.t("ui-feedback-placeholder")));
    ui.agent_badge.set_label(&format!("✦ {}", s.t("ui-changed-by-agent")));
    for (i, key) in ["ui-problems", "ui-feedback", "ui-activity"].iter().enumerate() {
        if let Some(page) = ui.notebook.nth_page(Some(i as u32)) {
            ui.notebook.set_tab_label_text(&page, &s.t(key));
            let help = s.t(&format!("{key}-help"));
            ui.tab_help[i].set_label(&help);
            if let Some(label) = ui.notebook.tab_label(&page) {
                label.set_tooltip_text(Some(&help));
            }
        }
    }
    let (u1, u2) = (ui.clone(), ui.clone());
    ui.export_button.set_popover(Some(&icon_popover(vec![
        MenuEntry::new(gtk::Image::from_icon_name("image-x-generic-symbolic"), "PNG", move || export(&u1, true)),
        MenuEntry::new(gtk::Image::from_icon_name(&themed_icon(&["x-office-drawing-symbolic", "image-x-generic-symbolic"])), "SVG", move || export(&u2, false)),
    ])));
    let mut mcp = vec![MenuEntry::heading(s.t("ui-mcp-copy-for"))];
    for (id, name) in MCP_CLIENTS.iter().copied().chain(std::iter::once(("json", ""))) {
        let u = ui.clone();
        let label = if id == "json" { s.t("ui-mcp-other") } else { name.to_string() };
        mcp.push(MenuEntry::new(client_icon(id), label, move || {
            let msg = copy_mcp_config(&u, id, false);
            u.toast.add_toast(adw::Toast::new(&msg));
        }));
    }
    mcp.push(MenuEntry::separator());
    let u = ui.clone();
    mcp.push(MenuEntry::new(gtk::Image::from_icon_name("emblem-system-symbolic"), s.t("ui-mcp-settings"), move || settings::show_preferences(&u, "mcp")));
    ui.mcp_button.set_popover(Some(&icon_popover(mcp)));
    let (group_labels, sort_labels) = (
        ["ui-group-project", "ui-group-none", "ui-group-kind", "ui-group-folder"].map(|k| s.t(k)),
        ["ui-sort-name", "ui-sort-date"].map(|k| s.t(k)),
    );
    let (group, sort) = (s.prefs.list_group.clone(), s.prefs.list_sort.clone());
    ui.group_dd.set_tooltip_text(Some(&s.t("ui-group-by")));
    // the vertical strip stacks the letters of the code pane's name
    let code = s.t("ui-source");
    let stacked = |text: String| text.to_uppercase().chars().map(String::from).collect::<Vec<_>>().join("\n");
    ui.code_rail.label.set_label(&stacked(code));
    ui.code_rail.rail.set_tooltip_text(Some(&s.t("ui-split-code-show")));
    ui.bottom_rail.label.set_label(&s.t("ui-bottom-panel").to_uppercase());
    ui.bottom_rail.rail.set_tooltip_text(Some(&s.t("ui-split-bottom-show")));
    ui.list_rail.label.set_label(&stacked(s.t("ui-diagrams")));
    ui.list_rail.rail.set_tooltip_text(Some(&s.t("ui-split-list-show")));
    ui.collapse_list.set_tooltip_text(Some(&s.t("ui-split-list-hide")));
    ui.sort_dd.set_tooltip_text(Some(&s.t("ui-sort-by")));
    drop(s);
    ui.list_opts_busy.set(true);
    ui.group_dd.set_model(Some(&gtk::StringList::new(&group_labels.iter().map(String::as_str).collect::<Vec<_>>())));
    ui.group_dd.set_selected(LIST_GROUPS.iter().position(|g| *g == group).unwrap_or(0) as u32);
    ui.sort_dd.set_model(Some(&gtk::StringList::new(&sort_labels.iter().map(String::as_str).collect::<Vec<_>>())));
    ui.sort_dd.set_selected(if sort == "date" { 1 } else { 0 });
    ui.list_opts_busy.set(false);
    refresh_list(ui); // diagram types and group titles in the current language
    let all = ui.state.borrow().strings.clone();
    send(ui, json!({"op": "strings", "strings": all}));
}

const LIST_GROUPS: [&str; 4] = ["project", "none", "kind", "folder"];

// ------------------------------------------------------------------ wiring

fn wire(ui: &Ui, ucm: &webkit::UserContentManager) {
    let u = ui.clone();
    ucm.connect_script_message_received(Some("odg"), move |_, value| {
        if let Ok(msg) = serde_json::from_str::<Value>(&value.to_str()) {
            on_viewer_message(&u, msg);
        }
    });

    for dd in [&ui.group_dd, &ui.sort_dd] {
        let u = ui.clone();
        dd.connect_selected_notify(move |_| {
            if u.list_opts_busy.get() {
                return;
            }
            {
                let mut s = u.state.borrow_mut();
                s.prefs.list_group = LIST_GROUPS.get(u.group_dd.selected() as usize).unwrap_or(&"none").to_string();
                s.prefs.list_sort = if u.sort_dd.selected() == 1 { "date" } else { "name" }.into();
                s.prefs.save();
            }
            refresh_list(&u);
        });
    }

    let u = ui.clone();
    ui.list.connect_row_selected(move |_, row| {
        let Some(row) = row else { return };
        let path = row.widget_name().to_string();
        if u.state.borrow().selected.as_deref() != Some(path.as_str()) {
            select(&u, Some(path));
        }
    });

    let u = ui.clone();
    ui.buffer.connect_changed(move |_| {
        if u.state.borrow().loading {
            return;
        }
        let gen = {
            let mut s = u.state.borrow_mut();
            s.edit_gen += 1;
            s.edit_gen
        };
        let u2 = u.clone();
        glib::timeout_add_local_once(Duration::from_millis(300), move || {
            if u2.state.borrow().edit_gen == gen {
                render(&u2);
            }
        });
        let u3 = u.clone();
        glib::timeout_add_local_once(Duration::from_millis(900), move || {
            if u3.state.borrow().edit_gen == gen {
                flush_save(&u3);
            }
        });
    });

    let u = ui.clone();
    ui.open_button.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder().title(u.state.borrow().t("ui-open-folder")).build();
        let u2 = u.clone();
        dialog.select_folder(Some(&u.window), gio::Cancellable::NONE, move |res| {
            if let Ok(folder) = res {
                if let Some(path) = folder.path() {
                    open_workspace(&u2, path);
                }
            }
        });
    });

    let u = ui.clone();
    ui.new_button.connect_clicked(move |_| new_diagram_dialog(&u));

    let u = ui.clone();
    ui.properties_button.connect_clicked(move |_| show_properties_dialog(&u));

    let u = ui.clone();
    let add = move || {
        let text = u.feedback_entry.text().to_string();
        let s = u.state.borrow();
        if let (Some(ws), Some(path)) = (&s.ws, &s.selected) {
            if !text.trim().is_empty() {
                let author = glib::real_name().to_string_lossy().to_string();
                let buf = u.context_view.buffer();
                let context = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
                let context = (u.context_include.is_active() && !context.trim().is_empty()).then_some(context);
                let _ = ws.add_feedback(path, &text, s.selected_element.as_deref(), None, &author, context.as_deref());
            }
        }
        drop(s);
        u.feedback_entry.set_text("");
        refresh_feedback_context(&u, true);
        refresh_side(&u);
    };
    let add2 = add.clone();
    let u = ui.clone();
    ui.context_view.buffer().connect_changed(move |_| {
        let mut s = u.state.borrow_mut();
        if !s.context_setting {
            s.context_edited = true;
        }
    });
    let u = ui.clone();
    ui.context_regen.connect_clicked(move |_| refresh_feedback_context(&u, true));
    let u = ui.clone();
    ui.context_include.connect_toggled(move |b| u.context_view.set_sensitive(b.is_active()));
    ui.feedback_button.connect_clicked(move |_| add());
    ui.feedback_entry.connect_activate(move |_| add2());

    let u = ui.clone();
    ui.problems.connect_row_activated(move |_, row| {
        let index: usize = row.widget_name().parse().unwrap_or(0);
        select_problem(&u, index);
    });

    let u = ui.clone();
    ui.settings_button.connect_clicked(move |_| settings::show_preferences(&u, "general"));
    let u = ui.clone();
    ui.update_button.connect_clicked(move |_| show_update(&u));
    let u = ui.clone();
    ui.window.connect_close_request(move |_| {
        flush_save(&u);
        let mut s = u.state.borrow_mut();
        // agents should no longer count on the human seeing their messages
        if let Some(ws) = &s.ws {
            let _ = ws.clear_presence();
        }
        s.remote.stop();
        s.prefs.save();
        glib::Propagation::Proceed
    });

    let u = ui.clone();
    adw::StyleManager::default().connect_dark_notify(move |sm| {
        u.state.borrow_mut().dark = sm.is_dark();
        send(&u, json!({"op": "theme", "theme": if sm.is_dark() { "dark" } else { "light" }}));
        apply_strings(&u); // monochrome brand marks follow the style
        render(&u);
    });
}

// ------------------------------------------------------------------ viewer bridge

fn send(ui: &Ui, msg: Value) {
    let mut s = ui.state.borrow_mut();
    if !s.viewer_ready {
        if msg["op"] == "process" {
            s.pending.retain(|m| m["op"] != "process");
        }
        s.pending.push(msg);
        return;
    }
    drop(s);
    let js = format!("window.odg.handle({msg});");
    ui.web.evaluate_javascript(&js, None, None, gio::Cancellable::NONE, |_| {});
}

fn on_viewer_message(ui: &Ui, msg: Value) {
    match msg["op"].as_str().unwrap_or("") {
        "ready" => {
            let pending = {
                let mut s = ui.state.borrow_mut();
                s.viewer_ready = true;
                std::mem::take(&mut s.pending)
            };
            let dark = ui.state.borrow().dark;
            send(ui, json!({"op": "theme", "theme": if dark { "dark" } else { "light" }}));
            apply_strings(ui);
            for m in pending.into_iter().filter(|m| m["op"] != "process") {
                send(ui, m);
            }
            render(ui);
        }
        "result" => {
            let id = msg["id"].as_u64().unwrap_or(0);
            render_finished(ui, id, msg["result"].clone());
        }
        "view" => {
            let mut s = ui.state.borrow_mut();
            s.view_zoom = msg["zoom"].as_f64();
            s.view_center = msg["center_element"].as_str().map(str::to_string);
            s.view_visible = msg["visible_elements"].as_array().map(|v| v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            s.presence_at = 0; // tell agents on the next poll
        }
        // undo / redo in the viewer: apply and save like any edit
        "set-source" => {
            if let Some(src) = msg["source"].as_str().map(str::to_string) {
                apply_layout_edit(ui, move |_| Ok(src));
            }
        }
        // view mode: the source can be read and selected, not typed into
        "edit-mode" => ui.editor.set_editable(msg["editing"].as_bool().unwrap_or(true)),
        "answer" => {
            let (Some(id), Some(text)) = (msg["question_id"].as_str(), msg["text"].as_str()) else { return };
            if let Some(ws) = &ui.state.borrow().ws {
                let _ = ws.answer(id, text);
            }
            refresh_side(ui);
        }
        "select" => {
            let el = msg["element_id"].as_str().map(str::to_string);
            ui.state.borrow_mut().selected_element = el.clone();
            refresh_feedback_context(ui, false);
            if let Some(el) = el {
                let line = source::find_line(&buffer_text(ui), &el);
                if let Some(l) = line {
                    scroll_to_line(ui, l);
                }
            }
        }
        "select-problem" => select_problem(ui, msg["index"].as_u64().unwrap_or(0) as usize),
        "move" => {
            if let (Some(id), Some(x), Some(y)) = (msg["element_id"].as_str(), msg["x"].as_f64(), msg["y"].as_f64()) {
                apply_layout_edit(ui, |src| source::set_node_position(src, id, x, y));
            }
        }
        "layout-mode" => {
            if let Some(mode) = msg["mode"].as_str() {
                apply_layout_edit(ui, |src| source::set_layout_mode(src, mode));
            }
        }
        "reset-positions" => apply_layout_edit(ui, |src| Ok(source::clear_positions(src, None))),
        "edit" => {
            if let Some(action) = msg["action"].as_str() {
                let params = msg["params"].clone();
                apply_layout_edit(ui, |src| odg_core::edit::apply(src, action, &params));
            }
        }
        "construct" => {
            if let Some(c) = msg["construct"].as_str() {
                let params = msg["params"].clone();
                apply_layout_edit(ui, |src| odg_core::construct::apply(src, c, &params));
            }
        }
        _ => {}
    }
}

/// Drag & drop / layout menu: rewrite the `%% @odg` comments, save and re-render.
fn apply_layout_edit(ui: &Ui, edit: impl FnOnce(&str) -> Result<String, String>) {
    if ui.state.borrow().selected.is_none() {
        return;
    }
    let current = buffer_text(ui);
    if let Ok(updated) = edit(&current) {
        if updated != current {
            set_buffer(ui, &updated);
            flush_save(ui);
        }
    }
}

fn buffer_text(ui: &Ui) -> String {
    let (a, b) = ui.buffer.bounds();
    ui.buffer.text(&a, &b, true).to_string()
}

fn set_buffer(ui: &Ui, text: &str) {
    ui.state.borrow_mut().loading = true;
    ui.buffer.set_text(text);
    ui.state.borrow_mut().loading = false;
}

fn render(ui: &Ui) {
    let text = buffer_text(ui);
    let (seq, dark, has, font) = {
        let mut s = ui.state.borrow_mut();
        s.render_seq += 1;
        s.render_source = text.clone();
        (s.render_seq, s.dark, s.selected.is_some(), s.diagram_font.clone())
    };
    if !has {
        send(ui, json!({"op": "process", "id": 0, "source": ""}));
        return;
    }
    let doc = ui.state.borrow().selected.clone().unwrap_or_default();
    send(ui, json!({"op": "process", "id": seq, "source": text, "doc": doc, "theme": if dark { "dark" } else { "default" }, "png": true, "scale": 2, "font": font}));
}

fn render_finished(ui: &Ui, id: u64, result: Value) {
    let report = {
        let mut s = ui.state.borrow_mut();
        if id != s.render_seq {
            return;
        }
        let Ok(raw) = serde_json::from_value::<RawRender>(result) else { return };
        let src = s.render_source.clone();
        let report = build_report(&src, &raw, "mermaid", s.lang, &LintConfig::default(), raw.duration_ms);
        s.last_svg = raw.svg.clone();
        s.last_png = raw.png.clone();
        if let (Some(ws), Some(path)) = (&s.ws, &s.selected) {
            if src == s.saved_source {
                let mut a = Analysis {
                    report: report.clone(),
                    svg: raw.svg.clone(),
                    png: raw.png.as_ref().and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok()),
                    annotated_png: None,
                    raw,
                };
                let _ = ws.save_analysis(path, &mut a, None);
            }
        }
        s.report = Some(report.clone());
        report
    };
    let problems: Vec<Value> = report
        .problems
        .iter()
        .map(|p| json!({"index": p.index, "severity": p.severity, "code": p.code, "message": p.message, "bbox": p.bbox}))
        .collect();
    send(ui, json!({"op": "show", "problems": problems}));
    if std::mem::take(&mut ui.state.borrow_mut().fit_next) {
        send(ui, json!({"op": "fit"}));
    }
    show_report(ui, &report);
}

fn show_report(ui: &Ui, report: &Report) {
    ui.status.set_label(&report.summary);
    // tint lines in the editor
    let (a, b) = ui.buffer.bounds();
    ui.buffer.remove_tag_by_name("error-line", &a, &b);
    ui.buffer.remove_tag_by_name("warning-line", &a, &b);
    for p in &report.problems {
        let Some(line) = p.line else { continue };
        let tag = match p.severity {
            odg_core::Severity::Error => "error-line",
            odg_core::Severity::Warning => "warning-line",
            _ => continue,
        };
        if let Some(start) = ui.buffer.iter_at_line(line as i32 - 1) {
            let mut end = start;
            end.forward_to_line_end();
            ui.buffer.apply_tag_by_name(tag, &start, &end);
        }
    }
    // problems list
    while let Some(child) = ui.problems.first_child() {
        ui.problems.remove(&child);
    }
    let s = ui.state.borrow();
    if report.problems.is_empty() {
        ui.problems.append(&gtk::Label::builder().label(s.t("ui-no-problems")).css_classes(["dim-label"]).margin_top(24).build());
    }
    for p in &report.problems {
        let icon = match p.severity {
            odg_core::Severity::Error => "dialog-error-symbolic",
            odg_core::Severity::Warning => "dialog-warning-symbolic",
            _ => "dialog-information-symbolic",
        };
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&format!("{}. {}", p.index, p.message)).as_str())
            .subtitle(glib::markup_escape_text(&p.hint).as_str())
            .activatable(true)
            .name(p.index.to_string())
            .build();
        row.add_prefix(&gtk::Image::from_icon_name(icon));
        let loc = p.line.map(|l| format!("{} · {}", p.code, i18n::t(s.lang, "ui-line", &[("line", l.into())]))).unwrap_or_else(|| p.code.clone());
        row.add_suffix(&gtk::Label::builder().label(loc).css_classes(["dim-label", "caption"]).build());
        ui.problems.append(&row);
    }
    if let Some(page) = ui.notebook.nth_page(Some(0)) {
        ui.notebook.set_tab_label_text(&page, &format!("{} ({})", s.t("ui-problems"), report.problems.len()));
    }
    let valid = report.valid;
    let selected = s.selected.clone();
    drop(s);
    if let Some(path) = selected {
        update_row_icon(ui, &path, Some((valid, report.warning_count)));
    }
}

fn select_problem(ui: &Ui, index: usize) {
    let p = ui.state.borrow().report.as_ref().and_then(|r| r.problems.iter().find(|p| p.index == index).cloned());
    let Some(p) = p else { return };
    if let Some(l) = p.line {
        scroll_to_line(ui, l);
    }
    if let Some(el) = p.element_id {
        ui.state.borrow_mut().selected_element = Some(el.clone());
        send(ui, json!({"op": "highlight", "element_id": el}));
    }
}

fn scroll_to_line(ui: &Ui, line: u32) {
    if let Some(mut it) = ui.buffer.iter_at_line(line as i32 - 1) {
        ui.buffer.place_cursor(&it);
        ui.editor.scroll_to_iter(&mut it, 0.2, false, 0.0, 0.0);
    }
}

// ------------------------------------------------------------------ workspace

// ------------------------------------------------------------------ agent ⇄ human (MCP show_diagram, highlight_elements, notify_user, ask_user)

/// Tells agents what the human is looking at and delivers their messages to the viewer.
fn sync_agents(ui: &Ui) {
    let inbox = {
        let mut s = ui.state.borrow_mut();
        let Some(ws) = s.ws.clone() else { return };
        if prefs::now() - s.presence_at >= 2 {
            s.presence_at = prefs::now();
            let _ = ws.set_presence(&odg_core::channel::Presence {
                app: "ODiagramador".into(),
                version: odg_core::VERSION.into(),
                platform: "linux".into(),
                pid: std::process::id(),
                updated: 0,
                path: s.selected.clone(),
                element_id: s.selected_element.clone(),
                zoom: s.view_zoom,
                center_element: s.view_center.clone(),
                visible_elements: s.view_visible.clone(),
            });
        }
        let Ok(inbox) = ws.viewer_inbox(s.inbox_cursor) else { return };
        s.inbox_cursor = Some(inbox.cursor);
        inbox
    };
    for m in &inbox.messages {
        deliver_agent_message(ui, m);
    }
    if !inbox.messages.is_empty() {
        refresh_side(ui);
    }
}

fn deliver_agent_message(ui: &Ui, m: &odg_core::channel::Message) {
    let mut switched = false;
    if let Some(path) = &m.path {
        let wants_switch = matches!(m.kind.as_str(), "show" | "highlight" | "view" | "select") || !m.element_ids.is_empty();
        let current = ui.state.borrow().selected.clone();
        if wants_switch && current.as_deref() != Some(path.as_str()) {
            if !ui.state.borrow().diagrams.iter().any(|d| &d.path == path) {
                refresh_list(ui);
            }
            if ui.state.borrow().diagrams.iter().any(|d| &d.path == path) {
                select(ui, Some(path.clone()));
                let mut i = 0;
                while let Some(row) = ui.list.row_at_index(i) {
                    if row.widget_name() == path.as_str() {
                        ui.list.select_row(Some(&row));
                        break;
                    }
                    i += 1;
                }
                switched = true;
            }
        }
    }
    send(ui, json!({"op": "agent", "message": m, "switched": switched}));
    if m.kind != "notify" {
        ui.window.present();
    }
}

fn open_workspace(ui: &Ui, dir: PathBuf) {
    let Ok(ws) = Workspace::open(&dir) else { return };
    {
        let mut s = ui.state.borrow_mut();
        if let Some(old) = s.ws.as_ref().filter(|o| o.root() != ws.root()) {
            let _ = old.clear_presence();
        }
        s.inbox_cursor = None;
        s.presence_at = 0;
    }
    let config = ws.config();
    {
        let mut s = ui.state.borrow_mut();
        s.prefs.last_workspace = Some(ws.root().to_string_lossy().to_string());
        s.prefs.save();
        s.diagram_font = config.font.clone().unwrap_or_default();
        if let Some(l) = &config.lang {
            s.lang = Lang::parse(l);
            s.strings = i18n::all_strings(s.lang);
        }
    }
    ui.title.set_subtitle(&ws.root().to_string_lossy());
    {
        let mut s = ui.state.borrow_mut();
        s.ws = Some(ws);
        s.selected = None;
        s.modified.clear();
    }
    apply_strings(ui);
    restart_remote(ui);
    refresh_list(ui);
    let first = ui.state.borrow().diagrams.first().map(|d| d.path.clone());
    select(ui, first);
}

fn format_short_date(rfc3339: &str) -> String {
    if rfc3339.len() >= 10 && rfc3339.chars().nth(4) == Some('-') && rfc3339.chars().nth(7) == Some('-') {
        let month = &rfc3339[5..7];
        let day = &rfc3339[8..10];
        format!("{day}/{month}")
    } else {
        String::new()
    }
}

fn refresh_list(ui: &Ui) {
    let (list, lang, group, sort, unknown, root, no_project, collapsed_groups) = {
        let s = ui.state.borrow();
        let Some(ws) = &s.ws else { return };
        (
            ws.list().unwrap_or_default(),
            s.lang,
            s.prefs.list_group.clone(),
            s.prefs.list_sort.clone(),
            s.t("ui-kind-unknown"),
            s.t("ui-root-folder"),
            s.t("ui-no-project"),
            s.collapsed_groups.clone(),
        )
    };
    while let Some(child) = ui.list.first_child() {
        ui.list.remove(&child);
    }
    let kind_label = |d: &odg_core::workspace::DiagramInfo| d.kind.as_deref().and_then(odg_core::kinds::find).map(|k| k.label(lang));
    let folder = |d: &odg_core::workspace::DiagramInfo| d.path.rfind('/').map(|i| d.path[..i].to_string()).unwrap_or_default();
    let group_of = |d: &odg_core::workspace::DiagramInfo| match group.as_str() {
        "project" => Some(d.project.as_deref().filter(|p| !p.trim().is_empty()).map(str::to_string).unwrap_or_else(|| no_project.clone())),
        "kind" => Some(kind_label(d).map(str::to_string).unwrap_or_else(|| unknown.clone())),
        "folder" => Some(Some(folder(d)).filter(|f| !f.is_empty()).map(|f| f + "/").unwrap_or_else(|| root.clone())),
        _ => None,
    };
    let mut ordered: Vec<&odg_core::workspace::DiagramInfo> = list.iter().collect();
    ordered.sort_by(|a, b| {
        let by_group = group_of(a).map(|g| g.to_lowercase()).cmp(&group_of(b).map(|g| g.to_lowercase()));
        let within = if sort == "date" { b.modified.cmp(&a.modified) } else { a.name.to_lowercase().cmp(&b.name.to_lowercase()) };
        by_group.then(within).then(a.path.cmp(&b.path))
    });
    let mut last_group: Option<String> = None;
    for d in ordered {
        if let Some(g) = group_of(d) {
            let is_collapsed = collapsed_groups.contains(&g);
            if last_group.as_ref() != Some(&g) {
                let count = list.iter().filter(|x| group_of(x).as_ref() == Some(&g)).count();
                let btn = gtk::Button::builder()
                    .has_frame(false)
                    .css_classes(["flat"])
                    .margin_top(4)
                    .margin_bottom(2)
                    .build();
                let hbox = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(6).build();
                let chevron = gtk::Image::from_icon_name(if is_collapsed { "pan-end-symbolic" } else { "pan-down-symbolic" });
                chevron.set_pixel_size(12);
                let lbl = gtk::Label::builder()
                    .label(format!("{g}  ·  {count}"))
                    .xalign(0.0)
                    .css_classes(["caption-heading", "dim-label"])
                    .hexpand(true)
                    .build();
                hbox.append(&chevron);
                hbox.append(&lbl);
                btn.set_child(Some(&hbox));

                let u = ui.clone();
                let g_name = g.clone();
                btn.connect_clicked(move |_| {
                    {
                        let mut s = u.state.borrow_mut();
                        if s.collapsed_groups.contains(&g_name) {
                            s.collapsed_groups.remove(&g_name);
                        } else {
                            s.collapsed_groups.insert(g_name.clone());
                        }
                    }
                    refresh_list(&u);
                });

                ui.list.append(&gtk::ListBoxRow::builder().child(&btn).selectable(false).activatable(false).build());
                last_group = Some(g.clone());
            }

            if is_collapsed {
                continue;
            }
        }
        // second line: coloured diagram type + folder (each left out when it is the group)
        let mut subtitle = String::new();
        if group != "kind" {
            if let Some(k) = kind_label(d) {
                subtitle.push_str(&format!("<span foreground=\"{}\" weight=\"bold\">{}</span>", kind_colour(d.kind.as_deref().unwrap_or("")), glib::markup_escape_text(k)));
            }
        }
        let f = folder(d);
        if group != "folder" && !f.is_empty() {
            if !subtitle.is_empty() {
                subtitle.push_str("  ");
            }
            subtitle.push_str(&glib::markup_escape_text(&format!("{f}/")));
        }
        if group != "project" {
            if let Some(proj) = &d.project {
                if !proj.is_empty() {
                    if !subtitle.is_empty() {
                        subtitle.push_str("  ·  ");
                    }
                    subtitle.push_str(&glib::markup_escape_text(proj));
                }
            }
        }
        let display_title = d.title.as_deref().filter(|t| !t.trim().is_empty()).unwrap_or(&d.name);
        let mut tooltip = d.path.clone();
        if let Some(desc) = &d.description {
            if !desc.trim().is_empty() {
                tooltip.push_str("\n\n");
                tooltip.push_str(desc.trim());
            }
        }
        if !d.modified.is_empty() {
            tooltip.push_str("\n\n");
            tooltip.push_str(&d.modified);
        }
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(display_title).as_str()).subtitle(subtitle.as_str()).tooltip_text(tooltip.as_str()).build();
        let icon = gtk::Image::from_icon_name(status_icon(d.status.as_ref().map(|s| (s.valid, s.warnings))));
        icon.set_widget_name("status");
        row.add_prefix(&icon);

        let suffix_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::End)
            .spacing(2)
            .build();

        if d.open_feedback > 0 {
            suffix_box.append(&gtk::Label::builder()
                .label(format!("💬 {}", d.open_feedback))
                .css_classes(["caption"])
                .halign(gtk::Align::End)
                .valign(gtk::Align::Start)
                .build());
        }

        let short_date = format_short_date(&d.modified);
        if !short_date.is_empty() {
            let date_lbl = gtk::Label::builder()
                .label(short_date)
                .css_classes(["caption", "dim-label"])
                .halign(gtk::Align::End)
                .valign(gtk::Align::End)
                .vexpand(true)
                .build();
            suffix_box.append(&date_lbl);
        }
        row.add_suffix(&suffix_box);

        let lrow = gtk::ListBoxRow::builder().child(&row).name(d.path.as_str()).build();
        ui.list.append(&lrow);
        if ui.state.borrow().selected.as_deref() == Some(d.path.as_str()) {
            ui.list.select_row(Some(&lrow));
        }
    }
    let mut s = ui.state.borrow_mut();
    for d in &list {
        s.modified.entry(d.path.clone()).or_insert_with(|| d.modified.clone());
    }
    s.diagrams = list;
}

/// Colour of the diagram-type label, by family of diagram (same as the macOS and Windows apps).
fn kind_colour(kind: &str) -> &'static str {
    match kind {
        "flowchart" | "block" | "architecture" | "c4" | "swimlane" | "agentflow" | "requirement" => "#1A73E8",
        "sequence" | "zenuml" | "state" | "usecase" | "eventmodeling" => "#8250DF",
        "class" | "er" => "#0E8A8A",
        "gantt" | "timeline" | "journey" | "kanban" | "gitgraph" => "#D9730D",
        "mindmap" | "treeview" | "ishikawa" => "#C2185B",
        _ => "#1E8E3E",
    }
}

fn status_icon(status: Option<(bool, usize)>) -> &'static str {
    match status {
        Some((false, _)) => "dialog-error-symbolic",
        Some((true, w)) if w > 0 => "dialog-warning-symbolic",
        Some(_) => "emblem-ok-symbolic",
        None => "content-loading-symbolic",
    }
}

fn update_row_icon(ui: &Ui, path: &str, status: Option<(bool, usize)>) {
    let mut child = ui.list.first_child();
    while let Some(row) = child {
        if row.widget_name() == path {
            if let Some(action_row) = row.first_child() {
                set_icon_recursive(&action_row, status_icon(status));
            }
        }
        child = row.next_sibling();
    }
}

fn set_icon_recursive(w: &gtk::Widget, icon: &str) {
    if let Some(img) = w.downcast_ref::<gtk::Image>() {
        if img.widget_name() == "status" {
            img.set_icon_name(Some(icon));
            return;
        }
    }
    let mut c = w.first_child();
    while let Some(ch) = c {
        set_icon_recursive(&ch, icon);
        c = ch.next_sibling();
    }
}

fn refresh_side(ui: &Ui) {
    let (fb, ev, lang) = {
        let s = ui.state.borrow();
        let Some(ws) = &s.ws else { return };
        (ws.feedback(s.selected.as_deref(), true).unwrap_or_default(), ws.events(40, None).unwrap_or_default(), s.lang)
    };
    fill_feedback(ui, &fb, lang);
    fill_activity(ui, &ev);
}

fn fill_feedback(ui: &Ui, items: &[Feedback], lang: Lang) {
    while let Some(child) = ui.feedback.first_child() {
        ui.feedback.remove(&child);
    }
    for f in items {
        let sub = format!(
            "{}{}{}",
            f.author,
            f.element_id.as_ref().map(|e| format!(" · {e}")).unwrap_or_default(),
            f.resolution.as_ref().map(|r| format!(" · {}: {r}", i18n::t(lang, "ui-resolved", &[]))).unwrap_or_default()
        );
        let sub = match &f.context {
            Some(c) => format!("{sub}\n{c}"),
            None => sub,
        };
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(&f.text).as_str()).subtitle(glib::markup_escape_text(&sub).as_str()).subtitle_lines(5).build();
        row.add_prefix(&gtk::Image::from_icon_name(if f.resolved { "emblem-ok-symbolic" } else { "mail-unread-symbolic" }));
        if !f.resolved {
            let btn = gtk::Button::builder().label(i18n::t(lang, "ui-mark-resolved", &[])).valign(gtk::Align::Center).build();
            let (u, id) = (ui.clone(), f.id.clone());
            btn.connect_clicked(move |_| {
                if let Some(ws) = &u.state.borrow().ws {
                    let _ = ws.resolve_feedback(&id, None, Actor::Human);
                }
                refresh_side(&u);
            });
            row.add_suffix(&btn);
        }
        ui.feedback.append(&row);
    }
}

fn fill_activity(ui: &Ui, events: &[Event]) {
    while let Some(child) = ui.activity.first_child() {
        ui.activity.remove(&child);
    }
    for e in events {
        let actor = match e.actor {
            Actor::Agent => "✦",
            Actor::Human => "👤",
            Actor::Cli => "⌘",
        };
        let valid = e.valid.map(|v| if v { " ✓" } else { " ✗" }).unwrap_or("");
        let ts: String = e.ts.replace('T', " ").chars().take(19).collect();
        let text = format!("{actor}  {:<9} {}{valid}   {}   {ts}", e.action, e.path, e.client.clone().unwrap_or_default());
        ui.activity.append(&gtk::Label::builder().label(text).xalign(0.0).css_classes(["monospace"]).margin_start(8).build());
    }
}

fn select(ui: &Ui, path: Option<String>) {
    flush_save(ui);
    ui.properties_button.set_sensitive(path.is_some());
    let Some(path) = path else { return };
    let text = {
        let s = ui.state.borrow();
        let Some(ws) = &s.ws else { return };
        ws.read(&path).unwrap_or_default()
    };
    {
        let mut s = ui.state.borrow_mut();
        s.selected = Some(path.clone());
        s.saved_source = text.clone();
        s.report = None;
        s.selected_element = None;
        s.fit_next = true;
    }
    ui.agent_badge.set_visible(false);
    ui.window.set_title(Some(&format!("{path} — ODiagramador")));
    set_buffer(ui, &text);
    refresh_feedback_context(ui, false);
    refresh_side(ui);
    render(ui);
}

/// Regenerates the comment context (diagram, selected element, its lines) unless the human edited it.
fn refresh_feedback_context(ui: &Ui, force: bool) {
    let text = {
        let s = ui.state.borrow();
        if !force && s.context_edited {
            return;
        }
        match (&s.ws, &s.selected) {
            (Some(ws), Some(path)) => ws
                .read(path)
                .map(|src| odg_core::feedback_context::build(&src, path, s.selected_element.as_deref(), s.lang))
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    ui.state.borrow_mut().context_setting = true;
    ui.context_view.buffer().set_text(&text);
    let mut s = ui.state.borrow_mut();
    s.context_setting = false;
    s.context_edited = false;
}

fn flush_save(ui: &Ui) {
    let text = buffer_text(ui);
    let mut s = ui.state.borrow_mut();
    let (Some(ws), Some(path)) = (s.ws.clone(), s.selected.clone()) else { return };
    if text == s.saved_source {
        return;
    }
    if ws.write(&path, &text, Actor::Human, None).is_ok() {
        s.saved_source = text;
        drop(s);
        render(ui);
    }
}

fn poll(ui: &Ui) {
    ui.state.borrow_mut().remote.poll();
    sync_agents(ui);
    let (list, selected) = {
        let s = ui.state.borrow();
        let Some(ws) = &s.ws else { return };
        (ws.list().unwrap_or_default(), s.selected.clone())
    };
    let changed: Vec<String> = {
        let s = ui.state.borrow();
        list.iter().filter(|d| s.modified.get(&d.path) != Some(&d.modified)).map(|d| d.path.clone()).collect()
    };
    let structure_changed = list.iter().map(|d| (&d.path, d.open_feedback)).ne(ui.state.borrow().diagrams.iter().map(|d| (&d.path, d.open_feedback)));
    if changed.is_empty() && !structure_changed {
        return;
    }
    {
        let mut s = ui.state.borrow_mut();
        for d in &list {
            s.modified.insert(d.path.clone(), d.modified.clone());
        }
    }
    refresh_list(ui);
    refresh_side(ui);
    if let Some(sel) = selected.filter(|p| changed.contains(p)) {
        let (disk, saved, by_agent) = {
            let s = ui.state.borrow();
            let ws = s.ws.as_ref().unwrap();
            let by_agent = ws.events(1, Some(&sel)).ok().and_then(|e| e.into_iter().next()).is_some_and(|e| e.actor == Actor::Agent);
            (ws.read(&sel).unwrap_or_default(), s.saved_source.clone(), by_agent)
        };
        if disk != saved {
            ui.state.borrow_mut().saved_source = disk.clone();
            ui.agent_badge.set_visible(by_agent);
            set_buffer(ui, &disk);
            render(ui);
        }
    }
}

fn show_properties_dialog(ui: &Ui) {
    let (sel, src, lang) = {
        let s = ui.state.borrow();
        let Some(ws) = &s.ws else { return };
        let Some(sel) = &s.selected else { return };
        let src = ws.read(sel).unwrap_or_default();
        (sel.clone(), src, s.lang)
    };
    let fm = odg_core::source::parse_front_matter(&src);

    let dialog = adw::MessageDialog::new(Some(&ui.window), Some(&i18n::t(lang, "ui-properties", &[])), None);
    let box_content = gtk::Box::new(gtk::Orientation::Vertical, 8);

    // Project entry (Required)
    box_content.append(&gtk::Label::builder().label(i18n::t(lang, "ui-diagram-project", &[])).xalign(0.0).build());
    let proj_entry = gtk::Entry::builder().text(fm.project.as_deref().unwrap_or("")).placeholder_text(i18n::t(lang, "ui-new-diagram-project-placeholder", &[])).build();
    box_content.append(&proj_entry);

    // Title entry (Optional)
    box_content.append(&gtk::Label::builder().label(i18n::t(lang, "ui-diagram-title", &[])).xalign(0.0).build());
    let title_entry = gtk::Entry::builder().text(fm.title.as_deref().unwrap_or("")).build();
    box_content.append(&title_entry);

    // Description entry (Optional)
    box_content.append(&gtk::Label::builder().label(i18n::t(lang, "ui-diagram-description", &[])).xalign(0.0).build());
    let desc_buffer = gtk::TextBuffer::new(None);
    if let Some(desc) = &fm.description {
        desc_buffer.set_text(desc);
    }
    let desc_view = gtk::TextView::builder().buffer(&desc_buffer).wrap_mode(gtk::WrapMode::Word).height_request(80).build();
    let desc_scroll = gtk::ScrolledWindow::builder().child(&desc_view).height_request(80).build();
    box_content.append(&desc_scroll);

    dialog.set_extra_child(Some(&box_content));
    dialog.add_response("cancel", &i18n::t(lang, "ui-cancel", &[]));
    dialog.add_response("save", &i18n::t(lang, "ui-save", &[]));
    dialog.set_default_response(Some("save"));
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);

    let u = ui.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "save" {
            return;
        }
        let proj = proj_entry.text().trim().to_string();
        if proj.is_empty() {
            return;
        }
        let tit = title_entry.text().trim().to_string();
        let tit_opt = if tit.is_empty() { None } else { Some(tit) };
        let (start, end) = desc_buffer.bounds();
        let desc = desc_buffer.text(&start, &end, false).trim().to_string();
        let desc_opt = if desc.is_empty() { None } else { Some(desc) };

        let updated = {
            let s = u.state.borrow();
            let Some(ws) = &s.ws else { return };
            let current_src = ws.read(&sel).unwrap_or_default();
            let new_src = odg_core::source::update_front_matter(&current_src, &odg_core::source::DiagramFrontMatter {
                title: tit_opt,
                description: desc_opt,
                project: Some(proj),
            });
            ws.write(&sel, &new_src, Actor::Human, None).ok().map(|_| new_src)
        };

        if let Some(new_src) = updated {
            u.state.borrow_mut().saved_source = new_src.clone();
            u.buffer.set_text(&new_src);
            refresh_list(&u);
            u.toast.add_toast(adw::Toast::new(&i18n::t(lang, "ui-properties-saved", &[])));
        }
    });
    dialog.present();
}

fn new_diagram_dialog(ui: &Ui) {
    let (lang, active_proj) = {
        let s = ui.state.borrow();
        if s.ws.is_none() {
            return;
        }
        let active_proj = s.selected.as_ref().and_then(|sel| {
            s.diagrams.iter().find(|d| &d.path == sel).and_then(|d| d.project.clone())
        }).unwrap_or_else(|| "default".to_string());
        (s.lang, active_proj)
    };
    let dialog = adw::MessageDialog::new(Some(&ui.window), Some(&i18n::t(lang, "ui-new-diagram", &[])), None);
    let extra = gtk::Box::new(gtk::Orientation::Vertical, 8);

    // Kind
    let kind_list = odg_core::kinds::list(lang);
    let kinds: Vec<&str> = kind_list.iter().map(|k| k.kind).collect();
    let labels: Vec<String> = kind_list.iter().map(|k| k.label.to_string()).collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let kind_drop = gtk::DropDown::from_strings(&label_refs);
    extra.append(&gtk::Label::builder().label(i18n::t(lang, "ui-new-diagram-type", &[])).xalign(0.0).build());
    extra.append(&kind_drop);

    // Project (Required)
    extra.append(&gtk::Label::builder().label(i18n::t(lang, "ui-diagram-project", &[])).xalign(0.0).build());
    let proj_entry = gtk::Entry::builder().text(&active_proj).placeholder_text(i18n::t(lang, "ui-new-diagram-project-placeholder", &[])).build();
    extra.append(&proj_entry);

    // File name (slug)
    extra.append(&gtk::Label::builder().label(i18n::t(lang, "ui-new-diagram-name", &[])).xalign(0.0).build());
    let entry = gtk::Entry::builder().placeholder_text("checkout").activates_default(true).build();
    extra.append(&entry);

    // Title (Optional)
    extra.append(&gtk::Label::builder().label(i18n::t(lang, "ui-diagram-title", &[])).xalign(0.0).build());
    let title_entry = gtk::Entry::builder().build();
    extra.append(&title_entry);

    // Description (Optional)
    extra.append(&gtk::Label::builder().label(i18n::t(lang, "ui-diagram-description", &[])).xalign(0.0).build());
    let desc_buffer = gtk::TextBuffer::new(None);
    let desc_view = gtk::TextView::builder().buffer(&desc_buffer).wrap_mode(gtk::WrapMode::Word).height_request(60).build();
    let desc_scroll = gtk::ScrolledWindow::builder().child(&desc_view).height_request(60).build();
    extra.append(&desc_scroll);

    dialog.set_extra_child(Some(&extra));
    dialog.add_response("cancel", &i18n::t(lang, "ui-cancel", &[]));
    dialog.add_response("create", &i18n::t(lang, "ui-create", &[]));
    dialog.set_default_response(Some("create"));
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);

    let u = ui.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "create" {
            return;
        }
        let proj = proj_entry.text().trim().to_string();
        if proj.is_empty() {
            return;
        }
        let name = entry.text().to_string();
        let slug: String = name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let path = format!("diagrams/{}.mmd", if slug.is_empty() { "diagram".into() } else { slug });
        let kind = kinds.get(kind_drop.selected() as usize).copied().unwrap_or("flowchart");
        let tit = title_entry.text().trim().to_string();
        let tit_opt = if tit.is_empty() { None } else { Some(tit) };
        let (start, end) = desc_buffer.bounds();
        let desc = desc_buffer.text(&start, &end, false).trim().to_string();
        let desc_opt = if desc.is_empty() { None } else { Some(desc) };

        let created = {
            let s = u.state.borrow();
            let template_body = odg_core::construct::template(kind, s.lang).unwrap_or_default();
            let with_fm = odg_core::source::update_front_matter(&template_body, &odg_core::source::DiagramFrontMatter {
                title: tit_opt,
                description: desc_opt,
                project: Some(proj),
            });
            s.ws.as_ref().and_then(|ws| ws.write(&path, &with_fm, Actor::Human, None).ok())
        };
        if let Some(rel) = created {
            refresh_list(&u);
            select(&u, Some(rel));
        }
    });
    dialog.present();
}

fn export(ui: &Ui, png: bool) {
    let (data, name) = {
        let s = ui.state.borrow();
        let data: Option<Vec<u8>> = if png {
            s.last_png.as_ref().and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
        } else {
            s.last_svg.clone().map(String::into_bytes)
        };
        let stem = s.selected.as_ref().map(|p| p.rsplit('/').next().unwrap_or(p).trim_end_matches(".mmd").to_string()).unwrap_or_else(|| "diagram".into());
        (data, format!("{stem}.{}", if png { "png" } else { "svg" }))
    };
    let Some(data) = data else { return };
    let dialog = gtk::FileDialog::builder().initial_name(name).build();
    dialog.save(Some(&ui.window), gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                let _ = std::fs::write(path, &data);
            }
        }
    });
}

/// Copies the config for `client` and returns the confirmation for the caller to show where the user clicked.
pub(crate) fn copy_mcp_config(ui: &Ui, client: &str, remote: bool) -> String {
    let s = ui.state.borrow();
    let Some(ws) = &s.ws else { return s.t("ui-mcp-need-folder").split(':').next().unwrap_or_default().to_string() };
    let w = ws.root().to_string_lossy().to_string();
    let url = remote.then(|| RemoteMcp::url(&s.prefs));
    let token = remote.then_some(s.prefs.mcp_token.as_str());
    let all = odg_core::mcp_config::snippets(&prefs::odg_exe(), &w, url.as_deref(), token);
    let text = odg_core::mcp_config::snippet_for(&all, client, remote);
    let name = MCP_CLIENTS.iter().find(|(id, _)| *id == client).map(|(_, n)| n.to_string()).unwrap_or_else(|| s.t("ui-mcp-other"));
    match gdk::Display::default() {
        Some(display) => {
            display.clipboard().set_text(&text);
            i18n::t(s.lang, "ui-mcp-copied", &[("app", name.as_str().into())])
        }
        None => s.t("ui-mcp-copy-failed"),
    }
}

// ------------------------------------------------------------------ settings, appearance, updates

pub(crate) fn set_language(ui: &Ui, lang: Lang) {
    {
        let mut s = ui.state.borrow_mut();
        if s.lang == lang {
            return;
        }
        s.lang = lang;
        s.strings = i18n::all_strings(lang);
        s.prefs.language = Some(lang.code().into());
        s.prefs.save();
        if let Some(ws) = &s.ws {
            let mut cfg = ws.config();
            cfg.lang = Some(lang.code().into());
            let _ = ws.set_config(&cfg);
        }
    }
    apply_strings(ui);
    render(ui);
    refresh_side(ui);
}

pub(crate) fn apply_appearance(ui: &Ui) {
    let scheme = match ui.state.borrow().prefs.appearance.as_str() {
        "dark" => adw::ColorScheme::ForceDark,
        "light" => adw::ColorScheme::ForceLight,
        _ => adw::ColorScheme::Default,
    };
    ui.state.borrow().prefs.save();
    adw::StyleManager::default().set_color_scheme(scheme);
}

pub(crate) fn apply_editor_font(ui: &Ui) {
    let font = ui.state.borrow().prefs.editor_font.clone();
    ui.state.borrow().prefs.save();
    let css = if font.is_empty() {
        String::new()
    } else {
        let desc = gtk::pango::FontDescription::from_string(&font);
        let family = desc.family().map(|f| f.to_string()).unwrap_or_else(|| "monospace".into());
        let size = if desc.size() > 0 { desc.size() as f64 / gtk::pango::SCALE as f64 } else { 11.0 };
        format!("textview.odg-editor {{ font-family: \"{}\", monospace; font-size: {size}pt; }}", family.replace('"', ""))
    };
    ui.editor_css.load_from_string(&css);
}

pub(crate) fn set_diagram_font(ui: &Ui, font: &str) {
    {
        let mut s = ui.state.borrow_mut();
        s.diagram_font = font.to_string();
        if let Some(ws) = &s.ws {
            let mut cfg = ws.config();
            cfg.font = (!font.is_empty()).then(|| font.to_string());
            let _ = ws.set_config(&cfg);
        }
    }
    render(ui);
}

pub(crate) fn restart_remote(ui: &Ui) {
    let mut s = ui.state.borrow_mut();
    s.prefs.save();
    let ws = s.ws.as_ref().map(|w| w.root().to_path_buf());
    let prefs = s.prefs.clone();
    s.remote.restart(&prefs, ws.as_deref());
}

pub(crate) fn check_updates(ui: &Ui, user_initiated: bool) {
    let u = ui.clone();
    prefs::check_updates(move |result| {
        let lang = u.state.borrow().lang;
        match result {
            Ok(info) => {
                {
                    let mut s = u.state.borrow_mut();
                    s.prefs.last_update_check = prefs::now();
                    s.prefs.save();
                    s.update = info.available.then(|| info.clone());
                }
                u.update_button.set_visible(info.available);
                if info.available {
                    let text = i18n::t(lang, "ui-update-available", &[("version", info.latest.as_str().into()), ("current", info.current.as_str().into())]);
                    u.update_button.set_tooltip_text(Some(&text));
                    show_update(&u);
                } else if user_initiated {
                    u.toast.add_toast(adw::Toast::new(&i18n::t(lang, "ui-up-to-date", &[("version", odg_core::VERSION.into())])));
                }
            }
            Err(e) if user_initiated => {
                u.toast.add_toast(adw::Toast::new(&i18n::t(lang, "ui-update-failed", &[("detail", e.as_str().into())])));
            }
            Err(_) => {}
        }
    });
}

/// Linux builds come from source or distro packages: offer the release page instead of replacing files.
fn show_update(ui: &Ui) {
    let s = ui.state.borrow();
    let Some(info) = s.update.clone() else { return };
    let heading = i18n::t(s.lang, "ui-update-available", &[("version", info.latest.as_str().into()), ("current", info.current.as_str().into())]);
    let dialog = adw::MessageDialog::new(Some(&ui.window), Some(&heading), None);
    // release notes are Markdown: shown as Pango markup in a scrollable label
    let notes = gtk::Label::builder()
        .use_markup(true)
        .label(odg_core::markdown::to_pango(&info.notes))
        .wrap(true)
        .xalign(0.0)
        .selectable(true)
        .build();
    let scroll = gtk::ScrolledWindow::builder().child(&notes).min_content_height(260).max_content_height(360).propagate_natural_height(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    dialog.set_extra_child(Some(&scroll));
    dialog.add_response("later", &s.t("ui-update-later"));
    dialog.add_response("page", &s.t("ui-update-release-page"));
    dialog.set_response_appearance("page", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("page"));
    let (window, url) = (ui.window.clone(), info.page_url.clone());
    dialog.connect_response(None, move |_, response| {
        if response == "page" {
            gtk::UriLauncher::new(&url).launch(Some(&window), gio::Cancellable::NONE, |_| {});
        }
    });
    dialog.present();
}
