import AppKit
import SwiftUI

/// Brand marks for the MCP menu (Simple Icons, CC0 — see assets/icons/README.md).
enum BrandIcon: String {
    case mcp = "modelcontextprotocol"
    case claude
    case claudeCode = "claudecode"
    case cursor
    case kiro

    /// Claude marks keep their orange; the others are template images that follow light/dark mode.
    private var tint: String? {
        switch self {
        case .claude, .claudeCode: return "#D97757"
        default: return nil
        }
    }

    private static var cache: [String: NSImage] = [:]

    static var directory: URL {
        if let res = Bundle.main.resourceURL?.appendingPathComponent("icons"),
           FileManager.default.fileExists(atPath: res.appendingPathComponent("modelcontextprotocol.svg").path) {
            return res
        }
        return RenderKit.repoRoot.appendingPathComponent("assets/icons")
    }

    func nsImage(size: CGFloat = 16) -> NSImage {
        let key = "\(rawValue)@\(size)"
        if let img = BrandIcon.cache[key] { return img }
        var image: NSImage?
        if var svg = try? String(contentsOf: BrandIcon.directory.appendingPathComponent("\(rawValue).svg"), encoding: .utf8) {
            if let tint { svg = svg.replacingOccurrences(of: "<path ", with: "<path fill=\"\(tint)\" ") }
            image = NSImage(data: Data(svg.utf8))
        }
        let result = image ?? NSImage(systemSymbolName: "puzzlepiece.extension", accessibilityDescription: nil) ?? NSImage()
        result.size = NSSize(width: size, height: size)
        result.isTemplate = tint == nil
        BrandIcon.cache[key] = result
        return result
    }

    var image: Image { Image(nsImage: nsImage()) }
}
