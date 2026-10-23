import Foundation
import Security

/// Runs the bundled `odg mcp --http` so agents on other machines (or other tools) can reach this
/// workspace over the network. Off by default; LAN access always requires a bearer token.
@MainActor
final class RemoteMCP: ObservableObject {
    static weak var current: RemoteMCP?

    enum Status: Equatable {
        case stopped
        case running(String)
        case failed(String)
    }

    @Published var enabled: Bool { didSet { save(); restart() } }
    @Published var port: Int { didSet { save(); restart() } }
    @Published var lan: Bool { didSet { save(); restart() } }
    @Published var token: String { didSet { save(); restart() } }
    @Published private(set) var status: Status = .stopped

    private var process: Process?
    private var workspace: URL?
    private var odgPath = ""
    private var restartTask: Task<Void, Never>?

    init() {
        let d = UserDefaults.standard
        enabled = d.bool(forKey: "mcpRemote")
        port = d.object(forKey: "mcpPort") as? Int ?? 7410
        lan = d.bool(forKey: "mcpLan")
        token = d.string(forKey: "mcpToken") ?? RemoteMCP.newToken()
        RemoteMCP.current = self
    }

    static func newToken() -> String {
        var bytes = [UInt8](repeating: 0, count: 24)
        _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        return bytes.map { String(format: "%02x", $0) }.joined()
    }

    private func save() {
        let d = UserDefaults.standard
        d.set(enabled, forKey: "mcpRemote")
        d.set(port, forKey: "mcpPort")
        d.set(lan, forKey: "mcpLan")
        d.set(token, forKey: "mcpToken")
    }

    /// Address clients should use (first LAN IPv4 when shared on the network).
    var url: String {
        let host = lan ? (RemoteMCP.lanAddresses().first ?? "127.0.0.1") : "127.0.0.1"
        return "http://\(host):\(port)/mcp"
    }

    func configure(workspace: URL?, odgPath: String) {
        self.workspace = workspace
        self.odgPath = odgPath
        restart()
    }

    func restart() {
        restartTask?.cancel()
        restartTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 400_000_000) // debounce typing in the port field
            guard !Task.isCancelled else { return }
            self?.stop()
            self?.start()
        }
    }

    private func start() {
        guard enabled, let ws = workspace, FileManager.default.isExecutableFile(atPath: odgPath), (1...65535).contains(port) else { return }
        let p = Process()
        p.executableURL = URL(fileURLWithPath: odgPath)
        p.arguments = ["mcp", "--workspace", ws.path, "--http", "\(lan ? "0.0.0.0" : "127.0.0.1"):\(port)"]
        var env = ProcessInfo.processInfo.environment
        env["ODG_MCP_TOKEN"] = token
        p.environment = env
        let errPipe = Pipe()
        p.standardError = errPipe
        p.standardOutput = FileHandle.nullDevice
        p.terminationHandler = { [weak self] proc in
            let text = String(decoding: errPipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
            let line = text.split(separator: "\n").last.map(String.init) ?? "exit \(proc.terminationStatus)"
            Task { @MainActor in
                guard let self, self.process === proc else { return }
                self.process = nil
                self.status = .failed(line.replacingOccurrences(of: "odg: ", with: ""))
            }
        }
        do {
            try p.run()
            process = p
            status = .running(url)
        } catch {
            status = .failed(error.localizedDescription)
        }
    }

    func stop() {
        if let p = process {
            process = nil
            p.terminationHandler = nil
            p.terminate()
        }
        status = .stopped
    }

    static func lanAddresses() -> [String] {
        var out: [String] = []
        var ifaddr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddr) == 0, let first = ifaddr else { return out }
        defer { freeifaddrs(ifaddr) }
        for ptr in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let ifa = ptr.pointee
            guard let addr = ifa.ifa_addr, addr.pointee.sa_family == UInt8(AF_INET),
                  (ifa.ifa_flags & UInt32(IFF_LOOPBACK)) == 0, (ifa.ifa_flags & UInt32(IFF_UP)) != 0 else { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            if getnameinfo(addr, socklen_t(addr.pointee.sa_len), &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 {
                out.append(String(cString: host))
            }
        }
        return out
    }
}
