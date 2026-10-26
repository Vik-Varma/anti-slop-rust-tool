//! `uninit-memory`, `mem-forget`. `static-mut` and the UB cases are checked under Miri and
//! with old and new compilers; see `miri/` and docs/EVIDENCE.md.

use anti_slop_experiments::run_helper;

/// Modern std refuses `mem::uninitialized` for types with invalid bit patterns at run time, by
/// aborting the process (a panic that can't unwind). For the remaining types it is still UB.
#[test]
fn mem_uninitialized_aborts_for_references() {
    let out = run_helper(env!("CARGO_BIN_EXE_uninit_ref"), &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stdout.contains("caught"), "catch_unwind never got control: {stdout}");
    assert!(stderr.contains("attempted to leave type `&u8` uninitialized, which is invalid"), "{stderr}");
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(out.status.signal(), Some(6), "SIGABRT");
    }
}

/// `mem::forget` on a guard leaves the lock held for good: every later `lock()` would block.
#[test]
fn forgetting_a_guard_keeps_the_lock() {
    let m = std::sync::Mutex::new(0u8);
    std::mem::forget(m.lock().expect("lock"));
    assert!(m.try_lock().is_err(), "nobody can take the lock again");
}

/// `mem::forget` on a file leaks the descriptor.
#[cfg(target_os = "linux")]
#[test]
fn forgetting_a_file_leaks_its_descriptor() {
    let count = || std::fs::read_dir("/proc/self/fd").expect("procfs").count();
    let before = count();
    for _ in 0..50 {
        std::mem::forget(std::fs::File::open("/proc/self/stat").expect("open"));
    }
    assert!(count() >= before + 50);
}
