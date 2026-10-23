import AppKit
import Foundation

/// Checks GitHub for a newer release and replaces this .app with it.
/// The core decides availability and picks the package (odg-core/src/update.rs).
@MainActor
final class Updater: ObservableObject {
    @Published var available: UpdateInfo?
    @Published var busy = false
    @Published var message: String?
    @Published var showSheet = false

    var auto: Bool {
        get { UserDefaults.standard.object(forKey: "autoUpdate") as? Bool ?? true }
        set { UserDefaults.standard.set(newValue, forKey: "autoUpdate"); objectWillChange.send() }
    }

    /// Install without asking when a newer release is found on launch (the app restarts).
    var autoInstall: Bool {
        get { UserDefaults.standard.bool(forKey: "autoInstallUpdates") }
        set { UserDefaults.standard.set(newValue, forKey: "autoInstallUpdates"); objectWillChange.send() }
    }

    var currentVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
            ?? (try? Core.call("version", as: CoreVersion.self).version) ?? "0.0.0"
    }

    /// Automatic check, at most once a day.
    func checkOnLaunch(t: @escaping (String, [String: Any]) -> String) {
        guard auto else { return }
        let last = UserDefaults.standard.double(forKey: "lastUpdateCheck")
        guard Date().timeIntervalSince1970 - last > 24 * 3600 else { return }
        Task { await check(userInitiated: false, t: t) }
    }

    func check(userInitiated: Bool, t: @escaping (String, [String: Any]) -> String) async {
        guard let version = try? Core.call("version", as: CoreVersion.self), let url = URL(string: version.releasesApi) else { return }
        var req = URLRequest(url: url, timeoutInterval: 20)
        req.setValue("ODiagramador/\(currentVersion)", forHTTPHeaderField: "User-Agent")
        req.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        do {
            let (data, _) = try await URLSession.shared.data(for: req)
            UserDefaults.standard.set(Date().timeIntervalSince1970, forKey: "lastUpdateCheck")
            let info = try Core.call("update_evaluate", [
                "release_json": String(decoding: data, as: UTF8.self),
                "current": currentVersion,
                "platform": "macos",
            ], as: UpdateInfo.self)
            if info.available {
                available = info
                message = nil
                showSheet = true
                if autoInstall && !userInitiated {
                    await install(t: t)
                }
            } else if userInitiated {
                available = nil
                message = t("ui-up-to-date", ["version": currentVersion])
                showSheet = true
            }
        } catch {
            if userInitiated {
                message = t("ui-update-failed", ["detail": error.localizedDescription])
                showSheet = true
            }
        }
    }

    /// Downloads the new .app, swaps it in after we quit, and relaunches.
    func install(t: @escaping (String, [String: Any]) -> String) async {
        guard let info = available else { return }
        let appURL = Bundle.main.bundleURL
        let parent = appURL.deletingLastPathComponent()
        guard appURL.pathExtension == "app",
              FileManager.default.isWritableFile(atPath: parent.path),
              let asset = info.assetUrl.flatMap(URL.init(string:)) else {
            message = t("ui-update-manual", [:])
            NSWorkspace.shared.open(URL(string: info.pageUrl)!)
            return
        }
        busy = true
        message = t("ui-updating", [:])
        do {
            let (zip, _) = try await URLSession.shared.download(from: asset)
            let work = FileManager.default.temporaryDirectory.appendingPathComponent("odg-update-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: work, withIntermediateDirectories: true)
            let unzip = Process()
            unzip.executableURL = URL(fileURLWithPath: "/usr/bin/ditto")
            unzip.arguments = ["-x", "-k", zip.path, work.path]
            try unzip.run()
            unzip.waitUntilExit()
            let newApp = work.appendingPathComponent("ODiagramador.app")
            guard unzip.terminationStatus == 0, FileManager.default.fileExists(atPath: newApp.path) else {
                throw NSError(domain: "odg", code: 1, userInfo: [NSLocalizedDescriptionKey: "package has no ODiagramador.app"])
            }
            let script = work.appendingPathComponent("swap.sh")
            try """
            #!/bin/sh
            PID="$1"; OLD="$2"; NEW="$3"
            while kill -0 "$PID" 2>/dev/null; do sleep 0.3; done
            BACKUP="$OLD.previous"
            rm -rf "$BACKUP"
            if mv "$OLD" "$BACKUP" && mv "$NEW" "$OLD"; then
              rm -rf "$BACKUP"
            elif [ -d "$BACKUP" ] && [ ! -d "$OLD" ]; then
              mv "$BACKUP" "$OLD"
            fi
            xattr -dr com.apple.quarantine "$OLD" 2>/dev/null
            open "$OLD"
            """.write(to: script, atomically: true, encoding: .utf8)
            let swap = Process()
            swap.executableURL = URL(fileURLWithPath: "/bin/sh")
            swap.arguments = [script.path, String(ProcessInfo.processInfo.processIdentifier), appURL.path, newApp.path]
            try swap.run()
            message = t("ui-update-restart", [:])
            try? await Task.sleep(nanoseconds: 800_000_000)
            RemoteMCP.current?.stop()
            NSApp.terminate(nil)
            // terminate can be refused (open sheet, modal panel); the swap script is waiting for us
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            exit(0)
        } catch {
            busy = false
            message = t("ui-update-failed", ["detail": error.localizedDescription])
        }
    }
}
