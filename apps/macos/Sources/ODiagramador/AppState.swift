import AppKit
import Foundation
import UniformTypeIdentifiers

/// Everything the window shows. Diagrams live on disk; agents change them through the MCP
/// server, so the state polls the workspace and reloads whatever changed underneath.
@MainActor
final class AppState: ObservableObject {
    @Published var workspace: URL?
    @Published var diagrams: [DiagramInfo] = []
    @Published var selectedPath: String?
    @Published var source: String = "" {
        didSet { if source != oldValue && !loading { sourceEdited() } }
    }
    @Published var report: Report?
    @Published var feedback: [Feedback] = []
    @Published var events: [ActivityEvent] = []
    @Published var selectedElement: String?
    @Published var selectedProblem: Int?
    @Published var lang: String = "en"
    @Published var strings: [String: String] = [:]
    @Published var darkMode = false
    @Published var agentChangedAt: Date?
    @Published var errorMessage: String?
    @Published var scrollToLine: Int?

    let viewer = ViewerBridge()
    private var savedSource = ""
    private var loading = false
    private var renderTask: Task<Void, Never>?
    private var saveTask: Task<Void, Never>?
    private var pollTimer: Timer?
    private var lastModified: [String: String] = [:]
    private var fitNext = true
    private var lastSvg: String?
    private var lastPng: String?

    init() {
        let saved = UserDefaults.standard.string(forKey: "lang")
        let system = Locale.preferredLanguages.first ?? "en"
        lang = saved ?? (system.hasPrefix("pt") ? "pt-BR" : "en")
        loadStrings()
        viewer.onResult = { [weak self] id, result in self?.renderFinished(id: id, result: result) }
        viewer.onSelect = { [weak self] element in self?.selectElement(element) }
        viewer.onSelectProblem = { [weak self] index in self?.selectProblem(index) }
        viewer.onReady = { [weak self] in
            guard let self else { return }
            self.viewer.send(["op": "strings", "strings": ["fit": self.t("ui-fit"), "empty": self.t("ui-empty")]])
            self.viewer.send(["op": "theme", "theme": self.darkMode ? "dark" : "light"])
            self.render()
        }
        if let path = UserDefaults.standard.string(forKey: "workspace"), FileManager.default.fileExists(atPath: path) {
            openWorkspace(URL(fileURLWithPath: path))
        }
    }

    // MARK: strings

    func t(_ key: String, _ args: [String: Any] = [:]) -> String {
        if args.isEmpty { return strings[key] ?? key }
        return (try? Core.call("t", ["lang": lang, "key": key, "args": args], as: String.self)) ?? key
    }

    private func loadStrings() {
        strings = (try? Core.call("strings", ["lang": lang], as: [String: String].self)) ?? [:]
    }

    func setLanguage(_ code: String) {
        lang = code
        UserDefaults.standard.set(code, forKey: "lang")
        loadStrings()
        if let ws = workspace {
            try? Core.run("set_config", ["workspace": ws.path, "lang": code])
        }
        viewer.send(["op": "strings", "strings": ["fit": t("ui-fit"), "empty": t("ui-empty")]])
        render()
    }

    func setDarkMode(_ dark: Bool) {
        guard dark != darkMode else { return }
        darkMode = dark
        viewer.send(["op": "theme", "theme": dark ? "dark" : "light"])
        render()
    }

    // MARK: workspace

    func chooseWorkspace() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.prompt = t("ui-open-folder")
        if panel.runModal() == .OK, let url = panel.url {
            openWorkspace(url)
        }
    }

    func openWorkspace(_ url: URL) {
        workspace = url
        UserDefaults.standard.set(url.path, forKey: "workspace")
        if let cfg = try? Core.call("config", ["workspace": url.path], as: WorkspaceConfig.self), let l = cfg.lang, l != lang {
            lang = l
            loadStrings()
        }
        selectedPath = nil
        lastModified = [:]
        refreshList()
        if let first = diagrams.first { select(first.path) }
        pollTimer?.invalidate()
        pollTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.poll() }
        }
    }

    func refreshList() {
        guard let ws = workspace else { return }
        do {
            diagrams = try Core.call("list", ["workspace": ws.path], as: [DiagramInfo].self)
            for d in diagrams where lastModified[d.path] == nil { lastModified[d.path] = d.modified }
        } catch {
            errorMessage = error.localizedDescription
        }
        refreshSideData()
    }

    private func refreshSideData() {
        guard let ws = workspace else { return }
        var args: [String: Any] = ["workspace": ws.path, "include_resolved": true]
        if let p = selectedPath { args["path"] = p }
        feedback = (try? Core.call("feedback", args, as: [Feedback].self)) ?? []
        events = (try? Core.call("events", ["workspace": ws.path, "limit": 40], as: [ActivityEvent].self)) ?? []
    }

    /// Picks up changes made by agents (or any editor) on disk.
    private func poll() {
        guard let ws = workspace,
              let list = try? Core.call("list", ["workspace": ws.path], as: [DiagramInfo].self) else { return }
        let changed = list.filter { lastModified[$0.path] != $0.modified }
        if list.map(\.path) != diagrams.map(\.path) || !changed.isEmpty || list != diagrams {
            diagrams = list
        }
        for d in changed { lastModified[d.path] = d.modified }
        if let sel = selectedPath, changed.contains(where: { $0.path == sel }) {
            reloadFromDisk(sel)
        }
        if !changed.isEmpty { refreshSideData() }
        if selectedPath != nil, !list.contains(where: { $0.path == selectedPath }) {
            selectedPath = nil
            setSource("")
        }
    }

    private func reloadFromDisk(_ path: String) {
        guard let ws = workspace,
              let disk = try? Core.call("read", ["workspace": ws.path, "path": path], as: String.self) else { return }
        guard disk != savedSource else { return }
        let agentEvent = (try? Core.call("events", ["workspace": ws.path, "limit": 1, "path": path], as: [ActivityEvent].self))?.first
        if agentEvent?.actor == "agent" { agentChangedAt = Date() }
        savedSource = disk
        // the agent's version wins over unsaved local typing; the debounce window is short
        saveTask?.cancel()
        setSource(disk)
        render()
    }

    private func setSource(_ text: String) {
        loading = true
        source = text
        loading = false
    }

    func select(_ path: String?) {
        guard let ws = workspace, let path else { return }
        flushSave()
        selectedPath = path
        fitNext = true
        selectedElement = nil
        selectedProblem = nil
        report = nil
        agentChangedAt = nil
        do {
            let text = try Core.call("read", ["workspace": ws.path, "path": path], as: String.self)
            savedSource = text
            setSource(text)
        } catch {
            errorMessage = error.localizedDescription
        }
        refreshSideData()
        render()
    }

    func createDiagram(named name: String) {
        guard let ws = workspace else { return }
        let slug = name.lowercased()
            .components(separatedBy: CharacterSet.alphanumerics.inverted)
            .filter { !$0.isEmpty }
            .joined(separator: "-")
        let path = "diagrams/\(slug.isEmpty ? "diagram" : slug).mmd"
        let template = "flowchart LR\n    start([Start]) --> step[Do something]\n    step --> done([Done])\n"
        do {
            let rel = try Core.call("write", ["workspace": ws.path, "path": path, "content": template, "actor": "human"], as: String.self)
            refreshList()
            select(rel)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: editing

    private func sourceEdited() {
        renderTask?.cancel()
        renderTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 300_000_000)
            guard !Task.isCancelled else { return }
            self?.render()
        }
        saveTask?.cancel()
        saveTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 900_000_000)
            guard !Task.isCancelled else { return }
            self?.flushSave()
        }
    }

    func flushSave() {
        saveTask?.cancel()
        guard let ws = workspace, let path = selectedPath, source != savedSource else { return }
        do {
            _ = try Core.call("write", ["workspace": ws.path, "path": path, "content": source, "actor": "human"], as: String.self)
            savedSource = source
            render() // stores the fresh report next to the file
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: rendering

    private var renderSeq = 0
    private var renderSources: [Int: String] = [:]

    func render() {
        guard selectedPath != nil else {
            viewer.send(["op": "process", "id": 0, "source": ""])
            return
        }
        renderSeq += 1
        renderSources = [renderSeq: source]
        viewer.send([
            "op": "process", "id": renderSeq, "source": source,
            "theme": darkMode ? "dark" : "default", "png": true, "scale": 2,
        ])
    }

    private func renderFinished(id: Int, result: [String: Any]) {
        guard id == renderSeq, let src = renderSources[id], let path = selectedPath, let ws = workspace else { return }
        lastSvg = result["svg"] as? String
        lastPng = result["png"] as? String
        let onDisk = src == savedSource
        do {
            let r = try Core.call("analyze", [
                "workspace": ws.path, "path": path, "source": src, "raw": result,
                "lang": lang, "save": onDisk,
            ], as: Report.self)
            report = r
            let problems = r.problems.map { p -> [String: Any] in
                var d: [String: Any] = ["index": p.index, "severity": p.severity, "code": p.code, "message": p.message]
                if let b = p.bbox { d["bbox"] = ["x": b.x, "y": b.y, "w": b.w, "h": b.h] }
                return d
            }
            viewer.send(["op": "show", "problems": problems])
            if fitNext {
                viewer.send(["op": "fit"])
                fitNext = false
            }
            if let el = selectedElement { viewer.send(["op": "highlight", "element_id": el]) }
            if onDisk, let i = diagrams.firstIndex(where: { $0.path == path }) {
                diagrams[i].status = DiagramStatus(valid: r.valid, errors: r.errorCount, warnings: r.warningCount, stale: false, summary: r.summary)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: selection

    func selectElement(_ element: String?) {
        selectedElement = element
        guard let element else { return }
        if let p = report?.problems.first(where: { $0.elementId == element }) { selectedProblem = p.index }
        if let line = try? Core.call("find_line", ["source": source, "id": element], as: Int.self) {
            scrollToLine = line
        }
    }

    func selectProblem(_ index: Int) {
        selectedProblem = index
        guard let p = report?.problems.first(where: { $0.index == index }) else { return }
        if let line = p.line { scrollToLine = line }
        if let el = p.elementId {
            selectedElement = el
            viewer.send(["op": "highlight", "element_id": el])
        }
    }

    // MARK: feedback

    func addFeedback(_ text: String) {
        guard let ws = workspace, let path = selectedPath, !text.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        var args: [String: Any] = ["workspace": ws.path, "path": path, "text": text, "author": NSFullUserName()]
        if let el = selectedElement { args["element_id"] = el }
        do {
            _ = try Core.call("add_feedback", args, as: Feedback.self)
            refreshList()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func resolveFeedback(_ id: String) {
        guard let ws = workspace else { return }
        try? Core.run("resolve_feedback", ["workspace": ws.path, "id": id, "actor": "human"])
        refreshList()
    }

    // MARK: export / integration

    func export(png: Bool) {
        guard let name = selectedPath.map({ URL(fileURLWithPath: $0).deletingPathExtension().lastPathComponent }) else { return }
        let data: Data?
        if png {
            data = lastPng.flatMap { Data(base64Encoded: $0) }
        } else {
            data = lastSvg.map { Data($0.utf8) }
        }
        guard let data else { NSSound.beep(); return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "\(name).\(png ? "png" : "svg")"
        panel.allowedContentTypes = [png ? .png : .svg]
        if panel.runModal() == .OK, let url = panel.url {
            try? data.write(to: url)
        }
    }

    /// Path of the bundled `odg` CLI (inside the .app), or the repo build when running from source.
    var odgPath: String {
        let bundled = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/odg")
        if FileManager.default.isExecutableFile(atPath: bundled.path) { return bundled.path }
        return RenderKit.repoRoot.appendingPathComponent("target/release/odg").path
    }

    func copyMcpConfig(_ kind: String) {
        guard let ws = workspace,
              let cfg = try? Core.call("mcp_config", ["odg_path": odgPath, "workspace": ws.path], as: McpConfig.self) else { return }
        let text = kind == "claude" ? cfg.claudeCode : kind == "codex" ? cfg.codex : cfg.json
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    /// odiagramador://open?workspace=/abs/path&path=diagrams/x.mmd
    func handle(url: URL) {
        guard url.scheme == "odiagramador",
              let items = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems else { return }
        let value = { (k: String) in items.first(where: { $0.name == k })?.value }
        if let w = value("workspace"), workspace?.path != w {
            openWorkspace(URL(fileURLWithPath: w))
        }
        if let p = value("path") {
            refreshList()
            select(p)
        }
        NSApp.activate(ignoringOtherApps: true)
    }
}
