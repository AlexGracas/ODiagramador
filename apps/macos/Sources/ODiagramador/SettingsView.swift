import AppKit
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        TabView(selection: $state.settingsTab) {
            GeneralSettings()
                .tabItem { Label(state.t("ui-settings-general"), systemImage: "gearshape") }
                .tag("general")
            EditorSettings()
                .tabItem { Label(state.t("ui-settings-editor"), systemImage: "textformat") }
                .tag("editor")
            MCPSettings(remote: state.remote)
                .tabItem { Label { Text(state.t("ui-settings-mcp")) } icon: { BrandIcon.mcp.image } }
                .tag("mcp")
        }
        .padding(20)
        .frame(width: 720)
        // confirmations (e.g. "MCP config copied") here too: the main window's toast is behind this window
        .overlay(alignment: .bottom) {
            if let toast = state.toast {
                Label(toast, systemImage: "checkmark.circle.fill")
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
                    .background(.regularMaterial, in: Capsule())
                    .shadow(radius: 4)
                    .padding(.bottom, 12)
                    .transition(.opacity)
            }
        }
        .animation(.easeInOut(duration: 0.2), value: state.toast)
    }
}

private struct GeneralSettings: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        Form {
            Picker(state.t("ui-language"), selection: Binding(get: { state.lang }, set: { state.setLanguage($0) })) {
                Text("Português (Brasil)").tag("pt-BR")
                Text("English").tag("en")
            }
            Picker(state.t("ui-appearance"), selection: $state.appearance) {
                Text(state.t("ui-appearance-system")).tag("system")
                Text(state.t("ui-appearance-light")).tag("light")
                Text(state.t("ui-appearance-dark")).tag("dark")
            }
            .pickerStyle(.radioGroup)
            Section(state.t("ui-updates")) {
                UpdateSettings(updater: state.updater)
            }
        }
        .formStyle(.grouped)
    }
}

private struct UpdateSettings: View {
    @EnvironmentObject var state: AppState
    @ObservedObject var updater: Updater

    var body: some View {
        Toggle(state.t("ui-auto-update"), isOn: Binding(get: { updater.auto }, set: { updater.auto = $0 }))
        Toggle(state.t("ui-auto-install"), isOn: Binding(get: { updater.autoInstall }, set: { updater.autoInstall = $0 }))
            .disabled(!updater.auto)
        HStack {
            Text(state.t("ui-version", ["version": updater.currentVersion])).foregroundStyle(.secondary)
            Spacer()
            Button(state.t("ui-check-updates")) {
                Task { await updater.check(userInitiated: true) { k, a in state.t(k, a) } }
            }
        }
    }
}

private struct EditorSettings: View {
    @EnvironmentObject var state: AppState

    private let editorFonts = ["", "Menlo", "Monaco", "SF Mono", "Courier New", "JetBrains Mono", "Fira Code"]
    private let diagramFonts = ["", "Helvetica", "Verdana", "Trebuchet MS", "Georgia", "Times New Roman", "Courier New"]

    var body: some View {
        Form {
            Picker(state.t("ui-editor-font"), selection: $state.editorFontName) {
                ForEach(editorFonts.filter { $0.isEmpty || NSFont(name: $0, size: 12) != nil }, id: \.self) { f in
                    Text(f.isEmpty ? "System Monospaced" : f).tag(f)
                }
            }
            Stepper(value: $state.editorFontSize, in: 9...28, step: 1) {
                Text("\(state.t("ui-editor-font-size")): \(Int(state.editorFontSize)) pt")
            }
            Section {
                Picker(state.t("ui-diagram-font"), selection: $state.diagramFont) {
                    ForEach(diagramFonts, id: \.self) { f in
                        Text(f.isEmpty ? state.t("ui-font-default") : f).tag(f)
                    }
                }
                .disabled(state.workspace == nil)
                Text(state.t("ui-diagram-font-hint")).font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }
}

private struct MCPSettings: View {
    @EnvironmentObject var state: AppState
    @ObservedObject var remote: RemoteMCP
    @State private var portText = ""

    var body: some View {
        Form {
            Section(state.t("ui-mcp-local")) {
                MCPClientButtons(remote: false)
                Text(state.t("ui-mcp-local-hint")).font(.caption).foregroundStyle(.secondary)
            }
            Section(state.t("ui-mcp-remote")) {
                Toggle(state.t("ui-mcp-remote-enable"), isOn: $remote.enabled)
                Picker(state.t("ui-mcp-scope"), selection: $remote.lan) {
                    Text(state.t("ui-mcp-scope-local")).tag(false)
                    Text(state.t("ui-mcp-scope-lan")).tag(true)
                }
                TextField(state.t("ui-mcp-port"), text: $portText)
                    .onAppear { portText = String(remote.port) }
                    .onSubmit { if let p = Int(portText), (1...65535).contains(p) { remote.port = p } }
                HStack {
                    Text(state.t("ui-mcp-token"))
                    Spacer()
                    Text(remote.token.prefix(8) + "…").font(.body.monospaced()).foregroundStyle(.secondary)
                    Button {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(remote.token, forType: .string)
                    } label: { Image(systemName: "doc.on.doc") }
                    Button(state.t("ui-mcp-regenerate")) { remote.token = RemoteMCP.newToken() }
                }
                statusRow
                if remote.enabled {
                    Text(state.t("ui-mcp-copy-remote")).font(.caption).foregroundStyle(.secondary)
                    MCPClientButtons(remote: true)
                }
                Label(state.t("ui-mcp-remote-warning"), systemImage: "exclamationmark.shield")
                    .font(.caption)
                    .foregroundStyle(.orange)
            }
        }
        .formStyle(.grouped)
    }

    @ViewBuilder private var statusRow: some View {
        switch remote.status {
        case .running(let url):
            HStack {
                Image(systemName: "circle.fill").foregroundStyle(.green).font(.caption2)
                Text(state.t("ui-mcp-running", ["url": url])).textSelection(.enabled)
            }
        case .failed(let detail):
            HStack {
                Image(systemName: "xmark.octagon.fill").foregroundStyle(.red)
                Text(state.t("ui-mcp-error", ["detail": detail])).textSelection(.enabled)
            }
        case .stopped:
            HStack {
                Image(systemName: "circle").foregroundStyle(.secondary).font(.caption2)
                Text(state.t("ui-mcp-stopped")).foregroundStyle(.secondary)
            }
        }
    }
}

/// One button per agent, with its logo.
struct MCPClientButtons: View {
    @EnvironmentObject var state: AppState
    let remote: Bool

    var body: some View {
        HStack {
            ForEach(AppState.mcpClients, id: \.id) { client in
                Button { state.copyMcpConfig(client.id, remote: remote) } label: {
                    Label { Text(client.name) } icon: { MCPClientIcon(id: client.id) }
                }
            }
            Button { state.copyMcpConfig("json", remote: remote) } label: {
                Label(state.t("ui-mcp-other"), systemImage: "curlybraces")
            }
        }
        .disabled(state.workspace == nil)
    }
}

struct MCPClientIcon: View {
    let id: String

    var body: some View {
        switch id {
        case "claude-code": BrandIcon.claudeCode.image
        case "claude-desktop": BrandIcon.claude.image
        case "cursor": BrandIcon.cursor.image
        case "codex": Image(systemName: "terminal")
        default: Image(systemName: "curlybraces")
        }
    }
}

/// "ODiagramador X is available" / progress / up to date.
struct UpdateSheet: View {
    @EnvironmentObject var state: AppState
    @ObservedObject var updater: Updater

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                Image(systemName: updater.available != nil ? "arrow.down.circle.fill" : "checkmark.circle.fill")
                    .font(.largeTitle)
                    .foregroundStyle(updater.available != nil ? Color.accentColor : Color.green)
                if let info = updater.available {
                    Text(state.t("ui-update-available", ["version": info.latest, "current": info.current])).font(.headline)
                } else if let m = updater.message {
                    Text(m).font(.headline)
                }
            }
            if let info = updater.available, !info.notes.isEmpty {
                ScrollView { Text(info.notes).font(.callout).frame(maxWidth: .infinity, alignment: .leading).textSelection(.enabled) }
                    .frame(height: 180)
            }
            if updater.available != nil, let m = updater.message {
                HStack {
                    if updater.busy { ProgressView().controlSize(.small) }
                    Text(m).font(.callout).foregroundStyle(.secondary)
                }
            }
            HStack {
                if let info = updater.available {
                    Button(state.t("ui-update-release-page")) { NSWorkspace.shared.open(URL(string: info.pageUrl)!) }
                }
                Spacer()
                Button(updater.available != nil ? state.t("ui-update-later") : "OK") { updater.showSheet = false }
                    .keyboardShortcut(.cancelAction)
                    .disabled(updater.busy)
                if updater.available != nil {
                    Button(state.t("ui-update-now")) {
                        Task { await updater.install { k, a in state.t(k, a) } }
                    }
                    .keyboardShortcut(.defaultAction)
                    .disabled(updater.busy)
                }
            }
        }
        .padding(20)
        .frame(width: 460)
    }
}
