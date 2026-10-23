// swift-tools-version:5.9
// ODiagramador for macOS — SwiftUI + WKWebView over the shared Rust core (odg-ffi).
// Build the core first:  cargo build --release -p odg-ffi -p odg-cli   (or run ./build-app.sh)
import Foundation
import PackageDescription

let repoRoot = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../..").standardized
let staticLib = ProcessInfo.processInfo.environment["ODG_FFI_LIB"]
    ?? repoRoot.appendingPathComponent("target/release/libodg_ffi.a").path

let package = Package(
    name: "ODiagramador",
    platforms: [.macOS(.v13)],
    targets: [
        .systemLibrary(name: "COdg", path: "Sources/COdg"),
        .executableTarget(
            name: "ODiagramador",
            dependencies: ["COdg"],
            path: "Sources/ODiagramador",
            linkerSettings: [
                .unsafeFlags([staticLib]),
                .linkedFramework("WebKit"),
            ]
        ),
    ]
)
