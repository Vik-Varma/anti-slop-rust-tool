//! Writes through a `BufWriter`, then calls `process::exit` from "library" code.
use std::fs::File;
use std::io::{BufWriter, Write};

fn library_bails_out() -> ! {
    std::process::exit(0)
}

fn main() {
    let path = std::env::args().nth(1).expect("output path");
    let mut out = BufWriter::new(File::create(path).expect("create output"));
    writeln!(out, "important record").expect("buffered write");
    if std::env::args().nth(2).as_deref() == Some("exit") {
        library_bails_out();
    }
    // Returning normally drops `out`, which flushes it.
}
