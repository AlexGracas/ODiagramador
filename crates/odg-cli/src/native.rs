//! On Apple silicon, a universal `odg` started by a program that runs under Rosetta (an Intel terminal or
//! MCP client) would run as Intel too, and macOS warns that the app "won't work with future versions".
//! When that happens, replace this process with the arm64 slice of the same binary (same arguments,
//! stdio and environment), before anything else starts.

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub fn ensure_native() {
    use std::ffi::{c_char, c_int, c_void, CString};
    use std::os::unix::ffi::OsStrExt;

    #[allow(non_camel_case_types)]
    type posix_spawnattr_t = *mut c_void;
    const POSIX_SPAWN_SETEXEC: i16 = 0x0040;
    const CPU_TYPE_ARM64: c_int = 0x0100_000c;
    extern "C" {
        fn sysctlbyname(name: *const c_char, old: *mut c_void, oldlen: *mut usize, new: *mut c_void, newlen: usize) -> c_int;
        fn posix_spawnattr_init(attr: *mut posix_spawnattr_t) -> c_int;
        fn posix_spawnattr_setflags(attr: *mut posix_spawnattr_t, flags: i16) -> c_int;
        fn posix_spawnattr_setbinpref_np(attr: *mut posix_spawnattr_t, count: usize, pref: *const c_int, ocount: *mut usize) -> c_int;
        fn posix_spawn(pid: *mut c_int, path: *const c_char, actions: *const c_void, attr: *const posix_spawnattr_t,
            argv: *const *const c_char, envp: *const *const c_char) -> c_int;
        static environ: *const *const c_char;
    }

    let mut translated: c_int = 0;
    let mut size = std::mem::size_of::<c_int>();
    let name = CString::new("sysctl.proc_translated").unwrap();
    // SAFETY: plain sysctl read into an int
    let rc = unsafe { sysctlbyname(name.as_ptr(), &mut translated as *mut c_int as *mut c_void, &mut size, std::ptr::null_mut(), 0) };
    if rc != 0 || translated != 1 {
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let Ok(path) = CString::new(exe.as_os_str().as_bytes()) else { return };
    let args: Vec<CString> = std::env::args_os().filter_map(|a| CString::new(a.as_bytes()).ok()).collect();
    let mut argv: Vec<*const c_char> = args.iter().map(|a| a.as_ptr()).collect();
    argv.push(std::ptr::null());
    // SAFETY: posix_spawn with SETEXEC replaces this process (stdio and environment inherited); it only
    // returns on failure (e.g. no arm64 slice), and then we simply keep running as Intel.
    unsafe {
        let mut attr: posix_spawnattr_t = std::ptr::null_mut();
        if posix_spawnattr_init(&mut attr) != 0 {
            return;
        }
        let pref = [CPU_TYPE_ARM64];
        let mut set = 0usize;
        posix_spawnattr_setbinpref_np(&mut attr, 1, pref.as_ptr(), &mut set);
        posix_spawnattr_setflags(&mut attr, POSIX_SPAWN_SETEXEC);
        let mut pid: c_int = 0;
        posix_spawn(&mut pid, path.as_ptr(), std::ptr::null(), &attr, argv.as_ptr(), environ);
    }
}

#[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
pub fn ensure_native() {}
