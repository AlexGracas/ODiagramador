import Darwin
import Foundation

/// On Apple silicon the app may be started as Intel (Rosetta) — by an Intel parent process or "Open using
/// Rosetta" — and macOS then warns it "won't work with future versions". Replace the process with the arm64
/// slice of this same universal executable, before any window or state exists.
enum NativeRelaunch {
    static func ifTranslated() {
        var translated: Int32 = 0
        var size = MemoryLayout<Int32>.size
        guard sysctlbyname("sysctl.proc_translated", &translated, &size, nil, 0) == 0, translated == 1,
              let exe = Bundle.main.executablePath else { return }
        var attr: posix_spawnattr_t?
        guard posix_spawnattr_init(&attr) == 0 else { return }
        defer { posix_spawnattr_destroy(&attr) }
        var pref: [cpu_type_t] = [CPU_TYPE_ARM64]
        var count = 0
        posix_spawnattr_setbinpref_np(&attr, 1, &pref, &count)
        posix_spawnattr_setflags(&attr, Int16(POSIX_SPAWN_SETEXEC))
        let args = CommandLine.arguments.map { strdup($0) } + [nil]
        var pid: pid_t = 0
        // only returns on failure (no arm64 slice): then keep running as Intel
        _ = posix_spawn(&pid, exe, nil, &attr, args, environ)
        args.forEach { free($0) }
    }
}
