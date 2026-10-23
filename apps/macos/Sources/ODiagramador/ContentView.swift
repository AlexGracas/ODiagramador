import SwiftUI

struct ContentView: View {
    @EnvironmentObject var state: AppState
    @Environment(\.colorScheme) private var colorScheme
    @State private var newName = ""
    @State private var showingNew = false

    var body: some View {
        NavigationSplitView {
            Sidebar(showingNew: $showingNew)
                .navigationSplitViewColumnWidth(min: 200, ideal: 240)
        } detail: {
            if state.workspace == nil {
                Welcome()
            } else {
                VSplitView {
                    HSplitView {
                        SourcePane()
                            .frame(minWidth: 260, idealWidth: 380)
                        PreviewPane()
                            .frame(minWidth: 320)
                    }
                    .frame(minHeight: 260)
                    BottomPanel()
                        .frame(minHeight: 130, idealHeight: 200)
                }
            }
        }
        .toolbar { Toolbar(showingNew: $showingNew) }
        .navigationTitle(state.selectedPath ?? state.t("ui-app-title"))
        .sheet(isPresented: $showingNew) { NewDiagramSheet(isPresented: $showingNew) }
        .onAppear { state.setDarkMode(colorScheme == .dark) }
        .onChange(of: colorScheme) { state.setDarkMode($0 == .dark) }
        .alert(state.errorMessage ?? "", isPresented: Binding(get: { state.errorMessage != nil }, set: { if !$0 { state.errorMessage = nil } })) {
            Button("OK", role: .cancel) {}
        }
    }
}

// MARK: - Sidebar

struct Sidebar: View {
    @EnvironmentObject var state: AppState
    @Binding var showingNew: Bool

    var body: some View {
        List(selection: Binding(get: { state.selectedPath }, set: { state.select($0) })) {
            Section(state.t("ui-diagrams")) {
                ForEach(state.diagrams) { d in
                    HStack(spacing: 8) {
                        StatusIcon(status: d.status)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(d.name).lineLimit(1)
                            Text(d.path).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                        Spacer()
                        if d.openFeedback > 0 {
                            Label("\(d.openFeedback)", systemImage: "text.bubble")
                                .labelStyle(.titleAndIcon)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .tag(d.path)
                }
            }
        }
        .overlay {
            if state.workspace != nil && state.diagrams.isEmpty {
                Text(state.t("ui-no-diagrams"))
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
                    .padding()
            }
        }
    }
}

struct StatusIcon: View {
    let status: DiagramStatus?

    var body: some View {
        if let s = status {
            if !s.valid {
                Image(systemName: "xmark.octagon.fill").foregroundStyle(.red)
            } else if s.warnings > 0 {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
            } else {
                Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
            }
        } else {
            Image(systemName: "circle.dashed").foregroundStyle(.secondary)
        }
    }
}

// MARK: - Panes

struct Welcome: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "point.3.connected.trianglepath.dotted")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
            Text(state.t("ui-no-workspace"))
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
                .frame(maxWidth: 380)
            Button(state.t("ui-open-folder")) { state.chooseWorkspace() }
                .keyboardShortcut("o")
                .controlSize(.large)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct SourcePane: View {
    @EnvironmentObject var state: AppState

    private func lines(_ severity: String) -> Set<Int> {
        Set(state.report?.problems.filter { $0.severity == severity }.compactMap(\.line) ?? [])
    }

    var body: some View {
        VStack(spacing: 0) {
            PaneHeader(title: state.t("ui-source")) {
                if state.agentChangedAt != nil {
                    Label(state.t("ui-changed-by-agent"), systemImage: "sparkles")
                        .font(.caption)
                        .foregroundStyle(.purple)
                }
            }
            SourceEditor(text: $state.source, errorLines: lines("error"), warningLines: lines("warning"), scrollToLine: $state.scrollToLine)
                .disabled(state.selectedPath == nil)
        }
    }
}

struct PreviewPane: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        VStack(spacing: 0) {
            PaneHeader(title: state.t("ui-preview")) {
                if let r = state.report {
                    Text(r.valid ? state.t("ui-valid") : state.t("ui-invalid"))
                        .font(.caption.bold())
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(r.valid ? Color.green.opacity(0.2) : Color.red.opacity(0.2), in: Capsule())
                }
            }
            ViewerWebView(bridge: state.viewer)
        }
    }
}

struct PaneHeader<Accessory: View>: View {
    let title: String
    @ViewBuilder var accessory: Accessory

    var body: some View {
        HStack {
            Text(title).font(.caption).foregroundStyle(.secondary).textCase(.uppercase)
            Spacer()
            accessory
        }
        .padding(.horizontal, 10)
        .frame(height: 26)
        .background(.bar)
    }
}

// MARK: - Bottom panel (problems, comments for agents, activity)

struct BottomPanel: View {
    @EnvironmentObject var state: AppState
    @State private var tab = 0

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Picker("", selection: $tab) {
                    Text("\(state.t("ui-problems")) (\(state.report?.problems.count ?? 0))").tag(0)
                    Text(state.t("ui-feedback")).tag(1)
                    Text(state.t("ui-activity")).tag(2)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .fixedSize()
                Spacer()
                if let r = state.report {
                    Text(r.summary).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
            }
            .padding(6)
            Divider()
            switch tab {
            case 0: ProblemsList()
            case 1: FeedbackList()
            default: ActivityList()
            }
        }
    }
}

struct ProblemsList: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let problems = state.report?.problems ?? []
        if problems.isEmpty {
            Text(state.t("ui-no-problems")).foregroundStyle(.secondary).frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            List(problems, selection: Binding(get: { state.selectedProblem }, set: { if let i = $0 { state.selectProblem(i) } })) { p in
                HStack(alignment: .top, spacing: 8) {
                    SeverityIcon(severity: p.severity)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(p.index). \(p.message)")
                        Text(p.hint).font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    VStack(alignment: .trailing, spacing: 2) {
                        Text(p.code).font(.caption.monospaced()).foregroundStyle(.secondary)
                        if let l = p.line { Text(state.t("ui-line", ["line": l])).font(.caption).foregroundStyle(.secondary) }
                    }
                }
                .tag(p.index)
            }
        }
    }
}

struct SeverityIcon: View {
    let severity: String

    var body: some View {
        switch severity {
        case "error": Image(systemName: "xmark.octagon.fill").foregroundStyle(.red)
        case "warning": Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
        default: Image(systemName: "info.circle.fill").foregroundStyle(.blue)
        }
    }
}

struct FeedbackList: View {
    @EnvironmentObject var state: AppState
    @State private var draft = ""

    var body: some View {
        VStack(spacing: 0) {
            List(state.feedback) { f in
                HStack(alignment: .top) {
                    Image(systemName: f.resolved ? "checkmark.circle" : "text.bubble")
                        .foregroundStyle(f.resolved ? Color.green : Color.accentColor)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(f.text).strikethrough(f.resolved)
                        HStack(spacing: 6) {
                            Text(f.author)
                            if let el = f.elementId { Text("· \(el)").monospaced() }
                            if let r = f.resolution { Text("· \(state.t("ui-resolved")): \(r)") }
                        }
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    }
                    Spacer()
                    if !f.resolved {
                        Button(state.t("ui-mark-resolved")) { state.resolveFeedback(f.id) }.controlSize(.small)
                    }
                }
            }
            HStack {
                if let el = state.selectedElement {
                    Text(el).font(.caption.monospaced()).padding(.horizontal, 6).background(.quaternary, in: Capsule())
                }
                TextField(state.t("ui-feedback-placeholder"), text: $draft)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(submit)
                Button(state.t("ui-add-feedback"), action: submit).disabled(draft.isEmpty || state.selectedPath == nil)
            }
            .padding(6)
        }
    }

    private func submit() {
        state.addFeedback(draft)
        draft = ""
    }
}

struct ActivityList: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        List(state.events) { e in
            HStack(spacing: 8) {
                Image(systemName: e.actor == "agent" ? "sparkles" : e.actor == "human" ? "person" : "terminal")
                    .foregroundStyle(e.actor == "agent" ? Color.purple : Color.secondary)
                    .frame(width: 18)
                Text(e.action).font(.caption.monospaced()).frame(width: 64, alignment: .leading)
                Text(e.path).lineLimit(1)
                if let v = e.valid { Image(systemName: v ? "checkmark" : "xmark").foregroundStyle(v ? .green : .red) }
                if let c = e.client { Text(c).font(.caption).foregroundStyle(.secondary) }
                Spacer()
                Text(e.ts.replacingOccurrences(of: "T", with: " ").prefix(19)).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
            }
        }
    }
}

// MARK: - Toolbar & sheets

struct Toolbar: ToolbarContent {
    @EnvironmentObject var state: AppState
    @Binding var showingNew: Bool

    var body: some ToolbarContent {
        ToolbarItemGroup {
            Button { state.chooseWorkspace() } label: { Label(state.t("ui-open-folder"), systemImage: "folder") }
            Button { showingNew = true } label: { Label(state.t("ui-new-diagram"), systemImage: "plus") }
                .disabled(state.workspace == nil)
            Menu {
                Button(state.t("ui-export-png")) { state.export(png: true) }
                Button(state.t("ui-export-svg")) { state.export(png: false) }
            } label: { Label("Export", systemImage: "square.and.arrow.up") }
                .disabled(state.selectedPath == nil)
            Menu {
                Button("Claude Code") { state.copyMcpConfig("claude") }
                Button("Codex") { state.copyMcpConfig("codex") }
                Button("JSON (Cursor, Claude Desktop…)") { state.copyMcpConfig("json") }
            } label: { Label(state.t("ui-copy-mcp-config"), systemImage: "cpu") }
                .disabled(state.workspace == nil)
            Picker(state.t("ui-language"), selection: Binding(get: { state.lang }, set: { state.setLanguage($0) })) {
                Text("Português").tag("pt-BR")
                Text("English").tag("en")
            }
            .pickerStyle(.menu)
        }
    }
}

struct NewDiagramSheet: View {
    @EnvironmentObject var state: AppState
    @Binding var isPresented: Bool
    @State private var name = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(state.t("ui-new-diagram")).font(.headline)
            TextField(state.t("ui-new-diagram-name"), text: $name).onSubmit(create)
            HStack {
                Spacer()
                Button(state.t("ui-cancel")) { isPresented = false }.keyboardShortcut(.cancelAction)
                Button(state.t("ui-create"), action: create).keyboardShortcut(.defaultAction).disabled(name.isEmpty)
            }
        }
        .padding()
        .frame(width: 360)
    }

    private func create() {
        guard !name.isEmpty else { return }
        state.createDiagram(named: name)
        isPresented = false
    }
}
