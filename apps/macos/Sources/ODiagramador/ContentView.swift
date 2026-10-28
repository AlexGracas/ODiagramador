import SwiftUI

struct ContentView: View {
    @EnvironmentObject var state: AppState
    @Environment(\.colorScheme) private var colorScheme
    @State private var newName = ""
    @State private var showingNew = false
    @AppStorage("codeWidth") private var codeWidth = 380.0
    @AppStorage("codeCollapsed") private var codeCollapsed = false
    @AppStorage("bottomHeight") private var bottomHeight = 200.0
    @AppStorage("bottomCollapsed") private var bottomCollapsed = false
    @AppStorage("listCollapsed") private var listCollapsed = false

    @ViewBuilder
    private var detail: some View {
        if state.workspace == nil {
            Welcome()
        } else {
            // double-click a bar (or click its chevron) to collapse / restore the code or the bottom panel
            CollapsibleSplit(axis: .vertical, collapses: .second, size: $bottomHeight, collapsed: $bottomCollapsed,
                             minSize: 120, minOther: 260, help: state.t("ui-split-bottom-tip"),
                             collapsedTitle: state.t("ui-bottom-panel"), collapsedIcon: "rectangle.bottomthird.inset.filled",
                             collapsedHelp: state.t("ui-split-bottom-show")) {
                CollapsibleSplit(axis: .horizontal, collapses: .first, size: $codeWidth, collapsed: $codeCollapsed,
                                 minSize: 220, minOther: 320, help: state.t("ui-split-code-tip"),
                                 collapsedTitle: state.t("ui-source"), collapsedIcon: "chevron.left.forwardslash.chevron.right",
                                 collapsedHelp: state.t("ui-split-code-show")) {
                    SourcePane()
                } second: {
                    PreviewPane()
                }
            } second: {
                BottomPanel()
            }
        }
    }

    var body: some View {
        // the diagram list hides with the toolbar's sidebar button (or by dragging it closed) and leaves a
        // labelled strip, like the code and the bottom panel
        NavigationSplitView(columnVisibility: Binding(get: { listCollapsed ? .detailOnly : .all },
                                                      set: { listCollapsed = $0 == .detailOnly })) {
            Sidebar(showingNew: $showingNew)
                .navigationSplitViewColumnWidth(min: 200, ideal: 240)
        } detail: {
            HStack(spacing: 0) {
                if listCollapsed {
                    CollapsedStrip(axis: .horizontal, title: state.t("ui-diagrams"), icon: "list.bullet.rectangle",
                                   chevron: "chevron.compact.right", help: state.t("ui-split-list-show")) {
                        withAnimation(.easeInOut(duration: 0.18)) { listCollapsed = false }
                    }
                }
                detail
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
    @AppStorage("listGroup") private var group = "none" // none | kind | folder
    @AppStorage("listSort") private var sort = "name"   // name | date
    @AppStorage("listCollapsed") private var listCollapsed = false
    @State private var collapsedGroups: Set<String> = []

    private struct Group: Identifiable {
        let id: String
        let title: String
        let items: [DiagramInfo]
    }

    private func sorted(_ list: [DiagramInfo]) -> [DiagramInfo] {
        sort == "date"
            ? list.sorted { $0.modified != $1.modified ? $0.modified > $1.modified : $0.path < $1.path } // newest first
            : list.sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }

    private var groups: [Group] {
        switch group {
        case "project":
            return Dictionary(grouping: state.diagrams) { $0.project?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false ? $0.project! : state.t("ui-no-project") }
                .sorted { $0.key.localizedStandardCompare($1.key) == .orderedAscending }
                .map { Group(id: "p:" + $0.key, title: $0.key, items: sorted($0.value)) }
        case "kind":
            return Dictionary(grouping: state.diagrams) { $0.kindLabel ?? state.t("ui-kind-unknown") }
                .sorted { $0.key.localizedStandardCompare($1.key) == .orderedAscending }
                .map { Group(id: "k:" + $0.key, title: $0.key, items: sorted($0.value)) }
        case "folder":
            return Dictionary(grouping: state.diagrams, by: \.folder)
                .sorted { $0.key.localizedStandardCompare($1.key) == .orderedAscending }
                .map { Group(id: "f:" + $0.key, title: $0.key.isEmpty ? state.t("ui-root-folder") : $0.key + "/", items: sorted($0.value)) }
        default:
            return [Group(id: "all", title: state.t("ui-diagrams"), items: sorted(state.diagrams))]
        }
    }

    var body: some View {
        List(selection: Binding(get: { state.selectedPath }, set: { state.select($0) })) {
            ForEach(groups) { g in
                let isCollapsed = collapsedGroups.contains(g.id)
                Section {
                    if !isCollapsed {
                        ForEach(g.items) { d in
                            DiagramRow(d: d, showKind: group != "kind", showFolder: group != "folder")
                                .tag(d.path)
                        }
                    }
                } header: {
                    if group != "none" {
                        Button {
                            withAnimation(.easeInOut(duration: 0.15)) {
                                if isCollapsed {
                                    collapsedGroups.remove(g.id)
                                } else {
                                    collapsedGroups.insert(g.id)
                                }
                            }
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: isCollapsed ? "chevron.right" : "chevron.down")
                                    .font(.system(size: 9, weight: .bold))
                                    .foregroundStyle(.secondary)
                                    .frame(width: 12)
                                Text(g.title)
                                Spacer()
                                Text("\(g.items.count)").monospacedDigit()
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    } else {
                        HStack {
                            Text(g.title)
                            Spacer()
                        }
                    }
                }
            }
        }
        .safeAreaInset(edge: .top, spacing: 0) {
            // how the list is organised: one menu with both choices, its label says the current ones
            HStack {
                Menu {
                    Picker(state.t("ui-group-by"), selection: $group) {
                        Text(state.t("ui-group-project")).tag("project")
                        Text(state.t("ui-group-none")).tag("none")
                        Text(state.t("ui-group-kind")).tag("kind")
                        Text(state.t("ui-group-folder")).tag("folder")
                    }
                    .pickerStyle(.inline)
                    Picker(state.t("ui-sort-by"), selection: $sort) {
                        Text(state.t("ui-sort-name")).tag("name")
                        Text(state.t("ui-sort-date")).tag("date")
                    }
                    .pickerStyle(.inline)
                } label: {
                    Label(state.t(["project": "ui-group-project", "kind": "ui-group-kind", "folder": "ui-group-folder"][group] ?? "ui-group-none")
                          + " · " + state.t(sort == "date" ? "ui-sort-date" : "ui-sort-name"),
                          systemImage: "line.3.horizontal.decrease.circle")
                }
                .menuStyle(.borderlessButton)
                .fixedSize(horizontal: false, vertical: true)
                .help(state.t("ui-group-by") + " / " + state.t("ui-sort-by"))
                Spacer(minLength: 0)
                // hide the list: it leaves a "DIAGRAMAS" strip on the left that brings it back
                Button {
                    withAnimation(.easeInOut(duration: 0.18)) { listCollapsed = true }
                } label: {
                    Image(systemName: "chevron.left.2")
                        .font(.caption.weight(.semibold))
                        .frame(width: 20, height: 18)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.borderless)
                .help(state.t("ui-split-list-hide"))
            }
            .font(.caption)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .disabled(state.diagrams.isEmpty)
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

struct DiagramRow: View {
    let d: DiagramInfo
    let showKind: Bool
    let showFolder: Bool

    private var shortDate: String {
        if d.modified.count >= 10 {
            let m = d.modified
            let startMonth = m.index(m.startIndex, offsetBy: 5)
            let endMonth = m.index(m.startIndex, offsetBy: 7)
            let startDay = m.index(m.startIndex, offsetBy: 8)
            let endDay = m.index(m.startIndex, offsetBy: 10)
            return "\(m[startDay..<endDay])/\(m[startMonth..<endMonth])"
        }
        return ""
    }

    var body: some View {
        HStack(spacing: 8) {
            StatusIcon(status: d.status)
            VStack(alignment: .leading, spacing: 2) {
                Text(d.title?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false ? d.title! : d.name).lineLimit(1)
                HStack(spacing: 5) {
                    if showKind, let k = d.kindLabel { KindTag(kind: d.kind ?? "", label: k) }
                    if showFolder && !d.folder.isEmpty {
                        Text(d.folder + "/").font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.head)
                    }
                }
            }
            Spacer(minLength: 4)
            VStack(alignment: .trailing, spacing: 2) {
                if d.openFeedback > 0 {
                    Label("\(d.openFeedback)", systemImage: "text.bubble")
                        .labelStyle(.titleAndIcon)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
                if !shortDate.isEmpty {
                    Text(shortDate)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }
            }
        }
        .help(d.description?.isEmpty == false ? "\(d.path)\n\n\(d.description!)" : (d.kindLabel.map { "\(d.path) · \($0)" } ?? d.path))
    }
}

/// Diagram type as a small coloured capsule (the colour follows the family of the type).
struct KindTag: View {
    let kind: String
    let label: String

    private var colour: Color {
        switch kind {
        case "flowchart", "block", "architecture", "c4", "swimlane", "agentflow", "requirement": return .blue
        case "sequence", "zenuml", "state", "usecase", "eventmodeling": return .purple
        case "class", "er": return .teal
        case "gantt", "timeline", "journey", "kanban", "gitgraph": return .orange
        case "mindmap", "treeview", "ishikawa": return .pink
        default: return .green // charts and maps
        }
    }

    var body: some View {
        Text(label)
            .font(.caption2.weight(.medium))
            .lineLimit(1)
            .fixedSize()
            .padding(.horizontal, 5)
            .padding(.vertical, 1)
            .foregroundStyle(colour)
            .background(colour.opacity(0.14), in: Capsule())
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
            SourceEditor(text: $state.source, errorLines: lines("error"), warningLines: lines("warning"), scrollToLine: $state.scrollToLine, font: state.editorFont, editable: state.editing)
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
                .help(state.t(["ui-problems-help", "ui-feedback-help", "ui-activity-help"][tab]))
                Spacer()
                if let r = state.report {
                    Text(r.summary).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
            }
            .padding(6)
            // what this tab is for, in one line
            HStack(spacing: 6) {
                Image(systemName: ["exclamationmark.triangle", "text.bubble", "clock.arrow.circlepath"][tab])
                Text(state.t(["ui-problems-help", "ui-feedback-help", "ui-activity-help"][tab])).lineLimit(2)
                Spacer()
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 8)
            .padding(.bottom, 6)
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
    @State private var project = ""
    @State private var title = ""
    @State private var description = ""
    @State private var kind = "flowchart"

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(state.t("ui-new-diagram")).font(.headline)
            Picker(state.t("ui-new-diagram-type"), selection: $kind) {
                ForEach(state.diagramKinds, id: \.kind) { k in
                    Text(k.label).tag(k.kind)
                }
            }
            TextField(state.t("ui-diagram-project"), text: $project)
            TextField(state.t("ui-new-diagram-name"), text: $name).onSubmit(create)
            TextField(state.t("ui-diagram-title"), text: $title)
            TextField(state.t("ui-diagram-description"), text: $description)
            HStack {
                Spacer()
                Button(state.t("ui-cancel")) { isPresented = false }.keyboardShortcut(.cancelAction)
                Button(state.t("ui-create"), action: create)
                    .keyboardShortcut(.defaultAction)
                    .disabled(name.isEmpty || project.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding()
        .frame(width: 360)
        .onAppear {
            if project.isEmpty {
                project = state.selectedDiagram?.project ?? "default"
            }
        }
    }

    private func create() {
        guard !name.isEmpty, !project.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        state.createDiagram(named: name, kind: kind, project: project, title: title.isEmpty ? nil : title, description: description.isEmpty ? nil : description)
        isPresented = false
    }
}
