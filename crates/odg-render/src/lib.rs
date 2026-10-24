//! Headless renderer: a hidden OS webview (WKWebView / WebView2 / WebKitGTK through tao + wry)
//! running the shared render-kit, embedded in the binary so everything works offline.
//!
//! The webview needs the main thread (macOS requirement), so callers hand their work to
//! [`run`]: it starts the event loop on the current (main) thread and executes the closure on a
//! worker thread with a [`Renderer`] handle that implements [`odg_core::RenderBackend`].

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

use base64::Engine as _;
use odg_core::{ProcessRequest, RawRender, RenderBackend};
use serde_json::{json, Value};
use tao::dpi::LogicalSize;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::http::{header::CONTENT_TYPE, Request, Response};
use wry::WebViewBuilder;

/// The render-kit, compiled in. Same files the desktop apps bundle.
const ASSETS: &[(&str, &[u8], &str)] = &[
    ("/index.html", include_bytes!("../../../render-kit/index.html"), "text/html; charset=utf-8"),
    ("/bridge.js", include_bytes!("../../../render-kit/bridge.js"), "text/javascript; charset=utf-8"),
    ("/palette.js", include_bytes!("../../../render-kit/palette.js"), "text/javascript; charset=utf-8"),
    ("/renderers.js", include_bytes!("../../../render-kit/renderers.js"), "text/javascript; charset=utf-8"),
    ("/viewer.css", include_bytes!("../../../render-kit/viewer.css"), "text/css; charset=utf-8"),
    ("/vendor/mermaid-odg.min.js", include_bytes!("../../../render-kit/vendor/mermaid-odg.min.js"), "text/javascript; charset=utf-8"),
];

#[cfg(windows)]
const START_URL: &str = "http://odg.localhost/index.html?mode=headless";
#[cfg(not(windows))]
const START_URL: &str = "odg://localhost/index.html?mode=headless";

enum UserEvent {
    Eval(String),
    Exit,
}

type Pending = Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>;
type Ready = Arc<(Mutex<bool>, Condvar)>;

/// Handle to the hidden webview. Cheap to clone; calls block until the page answers.
#[derive(Clone)]
pub struct Renderer {
    proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
    pending: Pending,
    ready: Ready,
    next_id: Arc<AtomicU64>,
    timeout: Duration,
}

impl Renderer {
    fn wait_ready(&self) -> Result<(), String> {
        let (lock, cv) = &*self.ready;
        let guard = lock.lock().unwrap();
        let (guard, res) = cv.wait_timeout_while(guard, Duration::from_secs(30), |ready| !*ready).unwrap();
        if res.timed_out() && !*guard {
            return Err("the webview did not load the render-kit within 30s".into());
        }
        Ok(())
    }

    /// Sends `{op, ...}` to `odg.handle` and waits for the reply carrying the same id.
    pub fn call(&self, mut msg: Value) -> Result<Value, String> {
        self.wait_ready()?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        msg["id"] = json!(id);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let js = format!("window.odg.handle({});", msg);
        if self.proxy.lock().unwrap().send_event(UserEvent::Eval(js)).is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err("renderer event loop is gone".into());
        }
        rx.recv_timeout(self.timeout).map_err(|_| {
            self.pending.lock().unwrap().remove(&id);
            format!("renderer timed out after {}s", self.timeout.as_secs())
        })
    }
}

impl RenderBackend for Renderer {
    fn process(&self, req: &ProcessRequest) -> Result<RawRender, String> {
        let mut msg = serde_json::to_value(req).map_err(|e| e.to_string())?;
        msg["op"] = json!("process");
        let reply = self.call(msg)?;
        serde_json::from_value(reply.get("result").cloned().unwrap_or(Value::Null)).map_err(|e| format!("bad render result: {e}"))
    }

    fn rasterize(&self, svg: &str, scale: f64, background: &str) -> Result<Vec<u8>, String> {
        let reply = self.call(json!({"op": "rasterize", "svg": svg, "scale": scale, "background": background}))?;
        if let Some(err) = reply.get("error").and_then(Value::as_str) {
            return Err(err.to_string());
        }
        let b64 = reply.get("png").and_then(Value::as_str).ok_or("rasterize returned no png")?;
        base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| e.to_string())
    }
}

fn serve(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    match ASSETS.iter().find(|(p, _, _)| *p == path) {
        Some((_, bytes, mime)) => Response::builder()
            .header(CONTENT_TYPE, *mime)
            .body(Cow::Borrowed(*bytes))
            .unwrap(),
        None => Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).unwrap(),
    }
}

fn on_ipc(body: &str, pending: &Pending, ready: &Ready) {
    let Ok(msg) = serde_json::from_str::<Value>(body) else { return };
    if msg.get("op").and_then(Value::as_str) == Some("ready") {
        let (lock, cv) = &**ready;
        *lock.lock().unwrap() = true;
        cv.notify_all();
        return;
    }
    if let Some(id) = msg.get("id").and_then(Value::as_u64) {
        if let Some(tx) = pending.lock().unwrap().remove(&id) {
            let _ = tx.send(msg);
        }
    }
}

/// Runs `work` with a renderer. Must be called from the main thread; returns `work`'s result.
pub fn run<F, R>(work: F) -> Result<R, String>
where
    F: FnOnce(Renderer) -> R + Send + 'static,
    R: Send + 'static,
{
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Prohibited);
        event_loop.set_dock_visibility(false);
        event_loop.set_activate_ignoring_other_apps(false);
    }

    let window = WindowBuilder::new()
        .with_title("odg-render")
        .with_visible(false)
        .with_inner_size(LogicalSize::new(1400.0, 1000.0))
        .build(&event_loop)
        .map_err(|e| format!("cannot create window: {e}"))?;

    let pending: Pending = Arc::default();
    let ready: Ready = Arc::new((Mutex::new(false), Condvar::new()));
    let (p2, r2) = (pending.clone(), ready.clone());
    let builder = WebViewBuilder::new()
        .with_custom_protocol("odg".into(), |_id, req| serve(req))
        .with_ipc_handler(move |req: Request<String>| on_ipc(req.body(), &p2, &r2))
        .with_url(START_URL);

    #[cfg(not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
    let webview = builder.build(&window).map_err(|e| format!("cannot create webview: {e}"))?;
    #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().ok_or("no gtk container")?;
        builder.build_gtk(vbox).map_err(|e| format!("cannot create webview: {e}"))?
    };

    let renderer = Renderer {
        proxy: Arc::new(Mutex::new(event_loop.create_proxy())),
        pending,
        ready,
        next_id: Arc::new(AtomicU64::new(1)),
        timeout: Duration::from_secs(60),
    };
    let exit_proxy = event_loop.create_proxy();
    let worker = std::thread::spawn(move || {
        let out = work(renderer);
        let _ = exit_proxy.send_event(UserEvent::Exit);
        out
    });

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep_alive = &window;
        if let Event::UserEvent(ev) = event {
            match ev {
                UserEvent::Eval(js) => {
                    if let Err(e) = webview.evaluate_script(&js) {
                        eprintln!("odg-render: evaluate_script failed: {e}");
                    }
                }
                UserEvent::Exit => *control_flow = ControlFlow::Exit,
            }
        }
    });

    worker.join().map_err(|_| "renderer worker panicked".to_string())
}
