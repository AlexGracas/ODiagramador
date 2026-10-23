import AppKit
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // when launched from `swift run` there is no bundle: still behave like a regular app
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        MainActor.assumeIsolated { RemoteMCP.current?.stop() }
    }
}

@main
struct ODiagramadorApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var state = AppState()

    var body: some Scene {
        // single window: odiagramador:// links (open_in_viewer) must reuse it, never spawn another
        Window("ODiagramador", id: "main") {
            ContentView()
                .environmentObject(state)
                .frame(minWidth: 900, minHeight: 560)
                .onOpenURL { state.handle(url: $0) }
        }
        .commands {
            CommandGroup(replacing: .appSettings) {
                SettingsCommands(state: state)
            }
            CommandGroup(replacing: .newItem) {
                Button(state.t("ui-open-folder")) { state.chooseWorkspace() }.keyboardShortcut("o")
            }
            CommandGroup(replacing: .saveItem) {
                Button(state.t("ui-save")) { state.flushSave() }.keyboardShortcut("s")
                Button(state.t("ui-export-png")) { state.export(png: true) }.keyboardShortcut("e")
                Button(state.t("ui-export-svg")) { state.export(png: false) }.keyboardShortcut("e", modifiers: [.command, .shift])
            }
        }

        Window(state.t("ui-settings"), id: "settings") {
            SettingsView()
                .environmentObject(state)
        }
        .windowResizability(.contentSize)
    }
}

/// App menu: Settings… (⌘,) and Check for Updates…
struct SettingsCommands: View {
    @ObservedObject var state: AppState
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Button(state.t("ui-settings") + "…") {
            state.settingsTab = "general"
            openWindow(id: "settings")
        }
        .keyboardShortcut(",")
        Button(state.t("ui-check-updates")) {
            Task { await state.updater.check(userInitiated: true) { k, a in state.t(k, a) } }
        }
    }
}
