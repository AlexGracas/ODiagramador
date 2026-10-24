import COdg
import Foundation

/// Thin Swift wrapper over the Rust core (`odg_invoke`, JSON in / JSON out).
enum Core {
    struct Failure: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    private struct Envelope<T: Decodable>: Decodable {
        let ok: Bool
        let result: T?
        let error: String?
    }

    static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()

    /// Calls `method` and returns the raw JSON envelope text.
    private static func raw(_ method: String, _ args: [String: Any]) throws -> Data {
        let data = try JSONSerialization.data(withJSONObject: args)
        let json = String(decoding: data, as: UTF8.self)
        guard let ptr = odg_invoke(method, json) else { throw Failure(message: "odg_invoke returned NULL") }
        defer { odg_free(ptr) }
        return Data(String(cString: ptr).utf8)
    }

    static func call<T: Decodable>(_ method: String, _ args: [String: Any] = [:], as type: T.Type = T.self) throws -> T {
        let env = try decoder.decode(Envelope<T>.self, from: raw(method, args))
        guard env.ok else { throw Failure(message: env.error ?? "unknown error") }
        guard let result = env.result else { throw Failure(message: "\(method) returned nothing") }
        return result
    }

    /// For methods whose result may be null or is irrelevant.
    static func run(_ method: String, _ args: [String: Any] = [:]) throws {
        let obj = try JSONSerialization.jsonObject(with: raw(method, args)) as? [String: Any]
        if obj?["ok"] as? Bool != true {
            throw Failure(message: obj?["error"] as? String ?? "unknown error")
        }
    }

    /// The result as plain JSON (dictionaries/arrays), for values forwarded untouched to the viewer.
    static func json(_ method: String, _ args: [String: Any] = [:]) throws -> Any? {
        let obj = try JSONSerialization.jsonObject(with: raw(method, args)) as? [String: Any]
        if obj?["ok"] as? Bool != true {
            throw Failure(message: obj?["error"] as? String ?? "unknown error")
        }
        return obj?["result"]
    }

    static func optional<T: Decodable>(_ method: String, _ args: [String: Any] = [:], as type: T.Type = T.self) throws -> T? {
        let env = try decoder.decode(Envelope<T>.self, from: raw(method, args))
        guard env.ok else { throw Failure(message: env.error ?? "unknown error") }
        return env.result
    }
}

// MARK: - Models (mirror odg-core's JSON)

struct DiagramKind: Codable, Hashable {
    var kind: String
    var label: String
    var graph: Bool
}

struct BBox: Codable, Hashable {
    var x: Double
    var y: Double
    var w: Double
    var h: Double
}

struct Problem: Codable, Hashable, Identifiable {
    var index: Int
    var severity: String
    var code: String
    var message: String
    var hint: String
    var line: Int?
    var column: Int?
    var elementId: String?
    var related: [String]?
    var bbox: BBox?
    var id: Int { index }
}

struct Stats: Codable, Hashable {
    var nodes: Int
    var edges: Int
    var groups: Int
    var width: Double
    var height: Double
}

struct Report: Codable, Hashable {
    var path: String?
    var diagramType: String?
    var valid: Bool
    var errorCount: Int
    var warningCount: Int
    var infoCount: Int
    var summary: String
    var problems: [Problem]
    var stats: Stats
}

struct DiagramStatus: Codable, Hashable {
    var valid: Bool
    var errors: Int
    var warnings: Int
    var stale: Bool
    var summary: String
}

struct DiagramInfo: Codable, Hashable, Identifiable {
    var path: String
    var name: String
    var engine: String
    var size: Int
    var modified: String
    var status: DiagramStatus?
    var openFeedback: Int
    var id: String { path }
}

struct Feedback: Codable, Hashable, Identifiable {
    var id: String
    var path: String
    var elementId: String?
    var line: Int?
    var text: String
    var context: String?
    var author: String
    var createdAt: String
    var resolved: Bool
    var resolution: String?
}

struct ActivityEvent: Codable, Hashable, Identifiable {
    var ts: String
    var actor: String
    var action: String
    var path: String
    var valid: Bool?
    var errors: Int?
    var warnings: Int?
    var note: String?
    var client: String?
    var id: String { ts + action + path }
}

struct WorkspaceConfig: Codable {
    var lang: String?
    var theme: String?
    var font: String?
}

struct McpConfig: Codable {
    var json: String
    var claudeCode: String
    var codex: String
    var remoteJson: String?
    var remoteClaudeCode: String?
    var remoteCodex: String?
}

struct UpdateInfo: Codable, Equatable {
    var available: Bool
    var current: String
    var latest: String
    var pageUrl: String
    var assetName: String?
    var assetUrl: String?
    var assetSize: Int?
    var notes: String
}

struct CoreVersion: Codable {
    var version: String
    var mermaid: String
    var releasesApi: String
    var releasesPage: String
}
