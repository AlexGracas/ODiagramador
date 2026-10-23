import SwiftUI
import WebKit

/// Locates the shared render-kit: inside the app bundle when packaged, in the repo when run from source.
enum RenderKit {
    static let repoRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent() // ODiagramador
        .deletingLastPathComponent() // Sources
        .deletingLastPathComponent() // macos
        .deletingLastPathComponent() // apps
        .deletingLastPathComponent() // repo

    static var directory: URL {
        if let res = Bundle.main.resourceURL?.appendingPathComponent("render-kit"),
           FileManager.default.fileExists(atPath: res.appendingPathComponent("index.html").path) {
            return res
        }
        return repoRoot.appendingPathComponent("render-kit")
    }
}

/// Serves render-kit files on odg://localhost/… (same origin model as the headless renderer).
final class RenderKitSchemeHandler: NSObject, WKURLSchemeHandler {
    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        guard let url = task.request.url else { return }
        let rel = url.path.hasPrefix("/") ? String(url.path.dropFirst()) : url.path
        let file = RenderKit.directory.appendingPathComponent(rel).standardized
        guard file.path.hasPrefix(RenderKit.directory.standardized.path), let data = try? Data(contentsOf: file) else {
            task.didFailWithError(URLError(.fileDoesNotExist))
            return
        }
        let mime: String
        switch file.pathExtension {
        case "html": mime = "text/html"
        case "js": mime = "text/javascript"
        case "css": mime = "text/css"
        default: mime = "application/octet-stream"
        }
        task.didReceive(URLResponse(url: url, mimeType: mime, expectedContentLength: data.count, textEncodingName: "utf-8"))
        task.didReceive(data)
        task.didFinish()
    }

    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}
}

/// Owns the conversation with bridge.js. Messages sent before the page is ready are queued.
@MainActor
final class ViewerBridge: NSObject, WKScriptMessageHandler {
    weak var webView: WKWebView?
    private var ready = false
    private var queue: [[String: Any]] = []

    var onReady: (() -> Void)?
    var onResult: ((Int, [String: Any]) -> Void)?
    var onSelect: ((String?) -> Void)?
    var onSelectProblem: ((Int) -> Void)?

    func send(_ message: [String: Any]) {
        guard ready, let webView else {
            // only the latest render request matters while loading
            if message["op"] as? String == "process" { queue.removeAll { $0["op"] as? String == "process" } }
            queue.append(message)
            return
        }
        guard let data = try? JSONSerialization.data(withJSONObject: message) else { return }
        let json = String(decoding: data, as: UTF8.self)
        webView.evaluateJavaScript("window.odg.handle(\(json));", completionHandler: nil)
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let text = message.body as? String,
              let obj = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any],
              let op = obj["op"] as? String else { return }
        switch op {
        case "ready":
            ready = true
            let pending = queue
            queue = []
            onReady?()
            pending.filter { $0["op"] as? String != "process" }.forEach(send)
        case "result":
            if let id = obj["id"] as? Int, let result = obj["result"] as? [String: Any] { onResult?(id, result) }
        case "select":
            onSelect?(obj["element_id"] as? String)
        case "select-problem":
            if let i = obj["index"] as? Int { onSelectProblem?(i) }
        default:
            break
        }
    }
}

struct ViewerWebView: NSViewRepresentable {
    let bridge: ViewerBridge

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.setURLSchemeHandler(RenderKitSchemeHandler(), forURLScheme: "odg")
        config.userContentController.add(bridge, name: "odg")
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.setValue(false, forKey: "drawsBackground")
        #if DEBUG
        if #available(macOS 13.3, *) { webView.isInspectable = true }
        #endif
        bridge.webView = webView
        webView.load(URLRequest(url: URL(string: "odg://localhost/index.html?mode=viewer")!))
        return webView
    }

    func updateNSView(_ nsView: WKWebView, context: Context) {}
}
