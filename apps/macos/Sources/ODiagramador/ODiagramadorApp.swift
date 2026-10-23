import AppKit
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // when launched from `swift run` there is no bundle: still behave like a regular app
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
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
            CommandGroup(replacing: .newItem) {
                Button(state.t("ui-open-folder")) { state.chooseWorkspace() }.keyboardShortcut("o")
            }
            CommandGroup(replacing: .saveItem) {
                Button(state.t("ui-save")) { state.flushSave() }.keyboardShortcut("s")
                Button(state.t("ui-export-png")) { state.export(png: true) }.keyboardShortcut("e")
                Button(state.t("ui-export-svg")) { state.export(png: false) }.keyboardShortcut("e", modifiers: [.command, .shift])
            }
        }
    }
}
