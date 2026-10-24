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
        .modifier(Overlays(updater: state.updater))
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
            SourceEditor(text: $state.source, errorLines: lines("error"), warningLines: lines("warning"), scrollToLine: $state.scrollToLine, font: state.editorFont)
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
    /// Context sent with the comment: regenerated when the diagram / selection changes, unless edited.
    @State private var context = ""
    @State private var contextEdited = false
    @State private var includeContext = true
    @State private var showContext = true

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
                        if let c = f.context {
                            Text(c).font(.caption2.monospaced()).foregroundStyle(.tertiary).lineLimit(4).textSelection(.enabled)
                        }
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
            .padding(.horizontal, 6)
            .padding(.top, 6)
            DisclosureGroup(isExpanded: $showContext) {
                VStack(alignment: .leading, spacing: 4) {
                    TextEditor(text: Binding(get: { context }, set: { context = $0; contextEdited = true }))
                        .font(.caption.monospaced())
                        .frame(height: 78)
                        .scrollContentBackground(.hidden)
                        .padding(4)
                        .background(.quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 6))
                        .disabled(!includeContext)
                        .opacity(includeContext ? 1 : 0.5)
                    HStack {
                        Toggle(state.t("ui-feedback-context-include"), isOn: $includeContext).toggleStyle(.checkbox)
                        Button(state.t("ui-feedback-context-reset")) { regenerate(force: true) }.controlSize(.small)
                        Spacer()
                        Text(state.t("ui-feedback-context-hint")).font(.caption2).foregroundStyle(.secondary).lineLimit(1)
                    }
                }
            } label: {
                Text(state.t("ui-feedback-context")).font(.caption).foregroundStyle(.secondary)
            }
            .padding(.horizontal, 8)
            .padding(.bottom, 6)
        }
        .onAppear { regenerate(force: true) }
        .onChange(of: state.selectedPath) { _ in regenerate(force: false) }
        .onChange(of: state.selectedElement) { _ in regenerate(force: false) }
    }

    private func regenerate(force: Bool) {
        guard force || !contextEdited else { return }
        context = state.feedbackContext()
        contextEdited = false
    }

    private func submit() {
        state.addFeedback(draft, context: includeContext ? context : nil)
        draft = ""
        regenerate(force: true)
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
    @Environment(\.openWindow) private var openWindow

    var body: some ToolbarContent {
        ToolbarItemGroup {
            Button { state.chooseWorkspace() } label: { Label(state.t("ui-open-folder"), systemImage: "folder.badge.gearshape") }
                .help(state.t("ui-open-folder"))
            Button { showingNew = true } label: { Label(state.t("ui-new-diagram"), systemImage: "doc.badge.plus") }
                .help(state.t("ui-new-diagram"))
                .disabled(state.workspace == nil)
            Menu {
                Button { state.export(png: true) } label: { Label("PNG", systemImage: "photo") }
                Button { state.export(png: false) } label: { Label("SVG", systemImage: "chevron.left.forwardslash.chevron.right") }
            } label: { Label(state.t("ui-export"), systemImage: "square.and.arrow.up") }
                .help(state.t("ui-export"))
                .disabled(state.selectedPath == nil)
            Menu {
                Section(state.t("ui-mcp-copy-for")) {
                    ForEach(AppState.mcpClients, id: \.id) { client in
                        Button { state.copyMcpConfig(client.id) } label: {
                            Label { Text(client.name) } icon: { MCPClientIcon(id: client.id) }
                        }
                    }
                    Button { state.copyMcpConfig("json") } label: { Label(state.t("ui-mcp-other"), systemImage: "curlybraces") }
                }
                Divider()
                Button {
                    state.settingsTab = "mcp"
                    openWindow(id: "settings")
                } label: { Label(state.t("ui-mcp-settings"), systemImage: "gearshape") }
            } label: { Label { Text(state.t("ui-mcp-configure")) } icon: { BrandIcon.mcp.image } }
                .help(state.t("ui-mcp-configure"))
                .accessibilityLabel(state.t("ui-mcp-configure"))
            UpdateBadge(updater: state.updater)
            Button {
                state.settingsTab = "general"
                openWindow(id: "settings")
            } label: { Label(state.t("ui-settings"), systemImage: "gearshape") }
                .help(state.t("ui-settings"))
        }
    }
}

/// Shows up in the toolbar only when a newer release exists.
struct UpdateBadge: View {
    @EnvironmentObject var state: AppState
    @ObservedObject var updater: Updater

    var body: some View {
        if let info = updater.available {
            Button { updater.showSheet = true } label: {
                Label(state.t("ui-update-available", ["version": info.latest, "current": info.current]), systemImage: "arrow.down.circle.fill")
            }
            .foregroundStyle(Color.accentColor)
            .help(state.t("ui-update-now"))
        }
    }
}

/// Toast + update sheet host.
struct Overlays: ViewModifier {
    @EnvironmentObject var state: AppState
    @ObservedObject var updater: Updater

    func body(content: Content) -> some View {
        content
            .overlay(alignment: .bottom) {
                if let toast = state.toast {
                    Label(toast, systemImage: "checkmark.circle.fill")
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(.regularMaterial, in: Capsule())
                        .shadow(radius: 4)
                        .padding(.bottom, 24)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .animation(.easeInOut(duration: 0.2), value: state.toast)
            .sheet(isPresented: $updater.showSheet) { UpdateSheet(updater: updater) }
    }
}

struct NewDiagramSheet: View {
    @EnvironmentObject var state: AppState
    @Binding var isPresented: Bool
    @State private var name = ""
    @State private var kind = "flowchart"

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(state.t("ui-new-diagram")).font(.headline)
            Picker(state.t("ui-new-diagram-type"), selection: $kind) {
                ForEach(state.diagramKinds, id: \.kind) { k in
                    Text(k.label).tag(k.kind)
                }
            }
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
        state.createDiagram(named: name, kind: kind)
        isPresented = false
    }
}
