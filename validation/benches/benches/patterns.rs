//! Measures the cost of each performance pattern anti-slop flags against the
//! suggested fix, plus "debunk" benchmarks for popular claims that turned out
//! not to hold (see docs/EVIDENCE.md).
//!
//! Run: `cargo bench --manifest-path validation/benches/Cargo.toml`

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::fmt::Write as _;

fn data(n: usize) -> Vec<u64> {
    (0..n as u64).map(|i| i.wrapping_mul(2654435761) % 1000).collect()
}

// needless-collect: `.collect::<Vec<_>>().len()` vs `.count()`.
fn needless_collect(c: &mut Criterion) {
    let mut g = c.benchmark_group("needless_collect");
    for n in [100usize, 10_000] {
        let v = data(n);
        g.bench_with_input(BenchmarkId::new("collect_len", n), &v, |b, v| {
            b.iter(|| black_box(v).iter().filter(|x| **x > 500).collect::<Vec<_>>().len())
        });
        g.bench_with_input(BenchmarkId::new("count", n), &v, |b, v| {
            b.iter(|| black_box(v).iter().filter(|x| **x > 500).count())
        });
        g.bench_with_input(BenchmarkId::new("collect_is_empty", n), &v, |b, v| {
            b.iter(|| black_box(v).iter().filter(|x| **x > 998).collect::<Vec<_>>().is_empty())
        });
        g.bench_with_input(BenchmarkId::new("next_is_none", n), &v, |b, v| {
            b.iter(|| black_box(v).iter().filter(|x| **x > 998).next().is_none())
        });
    }
    g.finish();
}

// format-push-string: `s.push_str(&format!(..))` vs `write!(s, ..)`.
fn format_push_string(c: &mut Criterion) {
    let mut g = c.benchmark_group("format_push_string");
    let v = data(1000);
    g.bench_function("push_str_format", |b| {
        b.iter(|| {
            let mut s = String::new();
            for x in black_box(&v) {
                s.push_str(&format!("{x},"));
            }
            s
        })
    });
    g.bench_function("write_macro", |b| {
        b.iter(|| {
            let mut s = String::new();
            for x in black_box(&v) {
                let _ = write!(s, "{x},");
            }
            s
        })
    });
    g.finish();
}

// borrowed-clone: `f(&s.clone())` vs `f(&s)`.
#[inline(never)]
fn takes_str(s: &str) -> usize {
    s.len()
}
#[inline(never)]
fn takes_slice(s: &[u64]) -> u64 {
    s.first().copied().unwrap_or(0)
}
fn borrowed_clone(c: &mut Criterion) {
    let mut g = c.benchmark_group("borrowed_clone");
    let s = "x".repeat(64);
    let v = data(1000);
    g.bench_function("string_clone_then_borrow", |b| b.iter(|| takes_str(&black_box(&s).clone())));
    g.bench_function("string_borrow", |b| b.iter(|| takes_str(black_box(&s))));
    g.bench_function("vec_clone_then_borrow", |b| b.iter(|| takes_slice(&black_box(&v).clone())));
    g.bench_function("vec_borrow", |b| b.iter(|| takes_slice(black_box(&v))));
    g.finish();
}

// redundant-box: building and reading `Box<Vec<u64>>` vs `Vec<u64>`.
fn redundant_box(c: &mut Criterion) {
    let mut g = c.benchmark_group("redundant_box");
    g.bench_function("box_vec_create", |b| b.iter(|| Box::new(vec![black_box(1u64); 16])));
    g.bench_function("vec_create", |b| b.iter(|| vec![black_box(1u64); 16]));
    let boxed: Vec<Box<Vec<u64>>> = (0..1000).map(|i| Box::new(vec![i; 4])).collect();
    let plain: Vec<Vec<u64>> = (0..1000).map(|i| vec![i; 4]).collect();
    g.bench_function("box_vec_read", |b| b.iter(|| black_box(&boxed).iter().map(|v| v[0]).sum::<u64>()));
    g.bench_function("vec_read", |b| b.iter(|| black_box(&plain).iter().map(|v| v[0]).sum::<u64>()));
    g.finish();
}

// eager-fallback-alloc: `opt.unwrap_or(format!(..))` vs `unwrap_or_else`, on the success path.
fn eager_fallback(c: &mut Criterion) {
    let mut g = c.benchmark_group("eager_fallback");
    let present: Option<String> = Some("value".to_string());
    g.bench_function("unwrap_or_format", |b| {
        b.iter(|| black_box(present.clone()).unwrap_or(format!("default-{}", 42)))
    });
    g.bench_function("unwrap_or_else_format", |b| {
        b.iter(|| black_box(present.clone()).unwrap_or_else(|| format!("default-{}", 42)))
    });
    g.bench_function("unwrap_or_string_new", |b| b.iter(|| black_box(present.clone()).unwrap_or(String::new())));
    g.finish();
}

// ---- Debunk benchmarks: claims we checked and did NOT turn into rules. ----

// "`.iter().count()` is O(n), use `.len()`": slice iterators implement count via len.
fn debunk_iter_count(c: &mut Criterion) {
    let mut g = c.benchmark_group("debunk_iter_count");
    let v = data(100_000);
    g.bench_function("iter_count", |b| b.iter(|| black_box(&v).iter().count()));
    g.bench_function("len", |b| b.iter(|| black_box(&v).len()));
    g.finish();
}

// "`&Vec<T>` params are slower than `&[T]`" (double indirection).
#[inline(never)]
fn sum_vec_ref(v: &Vec<u64>) -> u64 {
    v.iter().sum()
}
#[inline(never)]
fn sum_slice(v: &[u64]) -> u64 {
    v.iter().sum()
}
fn debunk_ptr_arg(c: &mut Criterion) {
    let mut g = c.benchmark_group("debunk_ptr_arg");
    let v = data(10_000);
    g.bench_function("vec_ref_param", |b| b.iter(|| sum_vec_ref(black_box(&v))));
    g.bench_function("slice_param", |b| b.iter(|| sum_slice(black_box(&v))));
    g.finish();
}

// "`rc.clone()` is slower/different from `Rc::clone(&rc)`": same call.
fn debunk_rc_clone(c: &mut Criterion) {
    let mut g = c.benchmark_group("debunk_rc_clone");
    let rc = std::rc::Rc::new(vec![1u8; 100]);
    g.bench_function("method_clone", |b| b.iter(|| black_box(&rc).clone()));
    g.bench_function("rc_clone_fn", |b| b.iter(|| std::rc::Rc::clone(black_box(&rc))));
    g.finish();
}

// "`unwrap_or(String::new())` allocates": `String::new` is a const fn, no allocation.
// (Covered by eager_fallback/unwrap_or_string_new.)

criterion_group!(
    benches,
    needless_collect,
    format_push_string,
    borrowed_clone,
    redundant_box,
    eager_fallback,
    debunk_iter_count,
    debunk_ptr_arg,
    debunk_rc_clone
);
criterion_main!(benches);
