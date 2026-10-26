//! `ignored-io-error`, `unused-io-amount`, `process-exit-in-library`.

use std::fs;
#[cfg(target_os = "linux")]
use std::fs::{File, OpenOptions};
#[cfg(target_os = "linux")]
use std::io::BufWriter;
use std::io::{self, Write};

use anti_slop_experiments::run_helper;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("anti-slop-experiments");
    fs::create_dir_all(&dir).expect("scratch dir");
    dir.join(name)
}

/// On a full disk `write_all` fails; with `let _ =` the program carries on as if it had saved.
#[cfg(target_os = "linux")]
#[test]
fn ignored_write_error_loses_data_silently() {
    let mut full = OpenOptions::new().write(true).open("/dev/full").expect("/dev/full exists on Linux");
    let r = full.write_all(b"the user's document");
    assert_eq!(r.expect_err("a full device refuses the write").raw_os_error(), Some(28)); // ENOSPC

    fn save_ignoring(f: &mut File) -> bool {
        let _ = f.write_all(b"the user's document");
        true // "saved"
    }
    assert!(save_ignoring(&mut full), "the caller is told the save worked");
}

/// `BufWriter` reports nothing if the final flush fails in `drop`; only an explicit `flush`
/// surfaces the error.
#[cfg(target_os = "linux")]
#[test]
fn bufwriter_drop_swallows_the_error() {
    let full = OpenOptions::new().write(true).open("/dev/full").expect("/dev/full");
    let mut w = BufWriter::new(full);
    assert!(w.write_all(b"small").is_ok(), "fits in the buffer, so no error yet");
    drop(w); // error lost here, by design

    let full = OpenOptions::new().write(true).open("/dev/full").expect("/dev/full");
    let mut w = BufWriter::new(full);
    w.write_all(b"small").expect("buffered");
    assert!(w.flush().is_err(), "an explicit flush reports ENOSPC");
}

/// A writer may accept fewer bytes than offered; `write` returns how many. A socket buffer is
/// the everyday case. Dropping the count truncates the stream.
#[cfg(unix)]
#[test]
fn write_can_be_partial_on_a_real_socket() {
    use std::os::unix::net::UnixStream;
    let (mut a, _b) = UnixStream::pair().expect("socket pair");
    a.set_nonblocking(true).expect("nonblocking");
    let big = vec![7u8; 8 << 20];
    let n = a.write(&big).expect("first write succeeds");
    assert!(n < big.len(), "kernel took {n} of {} bytes", big.len());
}

/// The same for a blocking `io::Write` that is allowed to short-write, like `std`'s own docs warn.
#[test]
fn write_contract_allows_short_writes() {
    struct Chunky(Vec<u8>);
    impl Write for Chunky {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = buf.len().min(4);
            self.0.extend_from_slice(&buf[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut w = Chunky(Vec::new());
    w.write(b"hello world").expect("write");
    assert_eq!(w.0, b"hell", "seven bytes silently dropped");
    let mut w = Chunky(Vec::new());
    w.write_all(b"hello world").expect("write_all loops");
    assert_eq!(w.0, b"hello world");
}

/// `process::exit` runs no destructors, so a `BufWriter`'s data never reaches the file.
#[test]
fn process_exit_skips_destructors() {
    let normal = scratch("normal.txt");
    let exited = scratch("exited.txt");
    let bin = env!("CARGO_BIN_EXE_exit_skips_drop");
    assert!(run_helper(bin, &[normal.to_str().expect("utf-8 path")]).status.success());
    assert!(run_helper(bin, &[exited.to_str().expect("utf-8 path"), "exit"]).status.success());
    assert_eq!(fs::read_to_string(&normal).expect("read"), "important record\n");
    assert_eq!(fs::read_to_string(&exited).expect("read"), "", "exit(0) reported success and lost the record");
}
