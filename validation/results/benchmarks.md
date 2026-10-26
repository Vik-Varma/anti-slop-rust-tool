# Benchmark results

`cargo bench --manifest-path validation/benches/Cargo.toml -- --warm-up-time 1 --measurement-time 3`
(criterion 0.5, rustc 1.97.0, 4-core x86_64 Linux container; median of the
criterion estimate shown with its 95% interval).

| benchmark | flagged pattern | suggested fix | slowdown |
|---|---|---|---:|
| needless-collect, n=100 | `.filter(..).collect::<Vec<_>>().len()` 245 ns [240, 253] | `.filter(..).count()` 57 ns [56, 58] | 4.3x |
| needless-collect, n=10 000 | `collect().len()` 10.9 µs [10.6, 11.1] | `count()` 5.75 µs [5.62, 5.89] | 1.9x |
| needless-collect, n=10 000 | `collect().is_empty()` 7.08 µs [6.85, 7.35] | `next().is_none()` 80 ns [76, 84] | 88x |
| format-push-string, 1000 appends | `push_str(&format!(..))` 53.7 µs [52.5, 55.1] | `write!(s, ..)` 25.7 µs [24.9, 26.7] | 2.1x |
| borrowed-clone, 64-byte String | `f(&s.clone())` 17.6 ns [17.4, 17.9] | `f(&s)` 1.31 ns [1.27, 1.36] | 13x |
| borrowed-clone, 1000-elem Vec | `f(&v.clone())` 107 ns [106, 109] | `f(&v)` 1.45 ns [1.44, 1.47] | 74x |
| redundant-box, create | `Box::new(vec![..; 16])` 26.7 ns [26.1, 27.3] | `vec![..; 16]` 14.0 ns [13.7, 14.5] | 1.9x |
| redundant-box, read 1000 | `Vec<Box<Vec<u64>>>` 1.15 µs [1.12, 1.18] | `Vec<Vec<u64>>` 0.97 µs [0.95, 1.00] | 1.18x |
| eager-fallback-alloc (Some path) | `.unwrap_or(format!(..))` 34.3 ns [33.5, 35.3] | `.unwrap_or_else(\|\| format!(..))` 22.2 ns [21.1, 23.2] | 1.55x (both include a ~20 ns clone of the input) |

## Claims checked and rejected

| claim | measurement | verdict |
|---|---|---|
| "`.iter().count()` on a slice is O(n); use `.len()`" | 406 ps vs 404 ps on 100 000 elements | **false**: slice iterators compute `count` from their length. No rule. |
| "`&Vec<T>` parameters are slower than `&[T]` (double indirection)" | first run 1.36 µs vs 1.24 µs; two longer re-runs 1.28 vs 1.33 and 1.32 vs 1.34 µs | **no measurable difference**. `ptr-arg` is kept as an opt-in API-flexibility rule only. |
| "`rc.clone()` differs from `Rc::clone(&rc)`" | 1.91 ns vs 1.90 ns | **identical**: style preference only. No rule. |
| "`.unwrap_or(String::new())` allocates" | 20.1 ns, the same as the clone alone | **false**: `String::new` does not allocate; `eager-fallback-alloc` ignores it. |
