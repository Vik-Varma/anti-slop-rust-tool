# Precision audit

Each rule was run over 37 codebases: the 35 study projects at the commits in `study/corpus-heads.txt`, plus Ferrock and Dataminer. Findings were sampled at random per rule (20 when a rule had more than 200 findings, else 10) with `study/audit_sample.py`, then judged by hand from the surrounding code. The question for each finding: is this the failure the rule describes, reachable in shipping code?

## Rounds

| Round | Seed | Build | Used for |
|---|---|---|---|
| 1 | 2026 | before any audit fix | found the false-positive classes: test modules in their own files, `write!` into a `String`, custom `write` methods, deliberate `mem::forget`, `unimplemented!` as "unsupported" |
| 2 | 31337 | after the round-1 fixes | confirmed them; split `panic-on-input` out of `unwrap` |
| 3 | 4242 | after the round-2 fixes | tuned `panic-on-input` (argument counts, `xtask-*` crates), `panic-in-drop` (assertions, `thread::panicking()` guards) and `ignored-io-error` (only in functions returning `Result`) |
| 4 | 777 | final build | **validation**: nothing was tuned on it, and its numbers set the severities |

## Results by rule

| Rule | Findings in corpus | Real / sampled | Round | Severity |
|---|---:|---|---|---|
| todo | 15 | 10 / 10 | 4 | warning |
| lock-held-across-await | 7 | 7 / 7 (all) | 1, 3 | error |
| panic-on-input | 222 | 8 / 20 | 4 | note |
| ignored-io-error | 14 | 4 / 10 | 4 | note |
| panic-in-drop | 42 | 3 / 10 | 4 | note |
| unwrap | 5,214 | 2 / 20 on fallible operations; 15 asserted invariants | 4 | note |
| static-mut | 12 | 0 / 10 races (all deliberate) | 3 | note |
| process-exit-in-library | 152 | 0 / 10 lost data (10 / 10 were library exits) | 3 | note |
| unsafe-without-comment | 4,354 | 20 / 20 lacked a justification; none shown unsound | 3 | note |
| len-truncating-cast | 442 | 0 / 20 shown reachable | 1 | note |
| dbg-macro | 10 | 1 / 10 accidental | 1 | note |
| unused-io-amount (dropped) | 136 | 0 / 10 | 1 | dropped |
| mem-forget (dropped) | 40 | 0 / 10 | 1 | dropped |
| uninit-memory, float-eq-inexact | 0 | no instances | n/a | error, warning |
| blocking-sleep-in-async, deny-warnings | 0 in shipping code | all were in tests or examples, which are now exempt | 1 | warning |

## Round 4 (validation) judgments

Each row: repository @ commit, file:line, verdict, reason. The code isn't reproduced here; check out the commit to re-judge it. Rounds 1 to 3 were judged the same way; their samples regenerate from the seeds above, but the build has changed since, so they come out different.

### todo

| # | Location | Real? | Why |
|---:|---|---|---|
| 0 | nushell @ 706259d6f8: `crates/nu-explore/src/explore/pager/mod.rs:458` | yes | unfinished branch reachable in shipping code |
| 1 | tauri @ 1fc6d9c85c: `crates/tauri/src/tray/mod.rs:196` | yes | unfinished branch reachable in shipping code |
| 2 | nushell @ 706259d6f8: `crates/nu-cmd-lang/src/core_commands/error_make.rs:158` | yes | unfinished branch reachable in shipping code |
| 3 | wgpu @ 100c142937: `wgpu/src/backend/webgpu.rs:2312` | yes | unfinished branch reachable in shipping code |
| 4 | wgpu @ 100c142937: `wgpu/src/backend/webgpu.rs:3687` | yes | unfinished branch reachable in shipping code |
| 5 | tauri @ 1fc6d9c85c: `crates/tauri-utils/src/acl/identifier.rs:75` | yes | unfinished branch reachable in shipping code |
| 6 | wgpu @ 100c142937: `wgpu-hal/src/gles/queue.rs:451` | yes | unfinished branch reachable in shipping code |
| 7 | cargo @ b57863321f: `src/workspace/source_id.rs:414` | yes | unfinished branch reachable in shipping code |
| 8 | gitoxide @ 9f805e24a0: `gix-ref/src/store/general/handle/find.rs:30` | yes | unfinished branch reachable in shipping code |
| 9 | wgpu @ 100c142937: `wgpu-hal/src/metal/command.rs:889` | yes | unfinished branch reachable in shipping code |

### panic-on-input

| # | Location | Real? | Why |
|---:|---|---|---|
| 0 | mdBook @ 8c41b823bb: `crates/mdbook-compare/src/main.rs:79` | no | dev tool binary |
| 1 | eza @ 471bfbc7b0: `src/options/parser.rs:319` | yes | non-UTF-8 command-line argument |
| 2 | tauri @ 1fc6d9c85c: `crates/tauri/src/plugin/mobile.rs:398` | yes | JNI payload from the mobile side |
| 3 | rustls @ 28ee6c688e: `bogo/src/opts.rs:575` | no | test shim (bogo) |
| 4 | meilisearch @ 7b6f9f39bd: `crates/milli/src/update/index_documents/helpers/mod.rs:66` | no | internal data the index wrote |
| 5 | cargo @ b57863321f: `src/compiler/build_context/target_info.rs:581` | yes | output of a spawned rustc |
| 6 | meilisearch @ 7b6f9f39bd: `crates/milli/src/update/index_documents/helpers/merge_functions.rs:278` | no | internal database value |
| 7 | cargo @ b57863321f: `src/workspace/parser/mod.rs:988` | yes | non-UTF-8 path |
| 8 | zola @ e93c3de50a: `components/content/src/front_matter/datetime.rs:48` | no | validated by a regex just before (commented) |
| 9 | regex @ 80b9d14e4b: `regex-syntax/src/debug.rs:24` | no | ASCII escapes built locally |
| 10 | wgpu @ 100c142937: `naga/src/front/wgsl/parse/number.rs:642` | no | "will never fail" (commented) |
| 11 | cargo @ b57863321f: `crates/semver-check/src/main.rs:228` | no | dev tool crate |
| 12 | rustls @ 28ee6c688e: `bogo/src/opts.rs:433` | no | test shim (bogo) |
| 13 | meilisearch @ 7b6f9f39bd: `crates/milli/src/vector/embedder/rest.rs:248` | yes | malformed environment variable |
| 14 | rustls @ 28ee6c688e: `bogo/src/opts.rs:564` | no | test shim (bogo) |
| 15 | meilisearch @ 7b6f9f39bd: `crates/dump/src/reader/v6/mod.rs:275` | yes | file names inside an external dump |
| 16 | meilisearch @ 7b6f9f39bd: `crates/dump/src/reader/v4/mod.rs:105` | yes | missing file inside an external dump |
| 17 | cargo @ b57863321f: `src/workspace/parser/mod.rs:3162` | yes | non-UTF-8 licence file name |
| 18 | cargo @ b57863321f: `src/compiler/compile_kind.rs:218` | no | path validated three lines up |
| 19 | cargo @ b57863321f: `src/workspace/global_cache_tracker.rs:1801` | no | testing-only environment variable |

### ignored-io-error

| # | Location | Real? | Why |
|---:|---|---|---|
| 0 | zellij @ ffb334a949: `zellij-utils/src/ipc.rs:608` | yes | IPC flush failure reported as success |
| 1 | zellij @ ffb334a949: `zellij-utils/src/ipc.rs:598` | yes | IPC flush failure reported as success |
| 2 | zellij @ ffb334a949: `zellij-utils/src/ipc.rs:460` | yes | IPC flush failure reported as success |
| 3 | nushell @ 706259d6f8: `crates/nu-protocol/src/pipeline/byte_stream.rs:1217` | no | already returning the original error |
| 4 | zellij @ ffb334a949: `zellij-window/src/dbus.rs:1335` | no | test double inside src/ |
| 5 | nushell @ 706259d6f8: `crates/nu-command/src/filesystem/save.rs:703` | no | already returning the original error |
| 6 | gitoxide @ 9f805e24a0: `gix-credentials/src/helper/invoke.rs:13` | yes | credential write failure reported as Ok |
| 7 | fd @ c44735ec0f: `src/walk.rs:261` | no | commented: about to exit |
| 8 | nushell @ 706259d6f8: `crates/nu-protocol/src/pipeline/byte_stream.rs:1199` | no | already returning the original error |
| 9 | zellij @ ffb334a949: `zellij-window/src/unattended.rs:62` | no | best-effort log rotation |

### panic-in-drop

| # | Location | Real? | Why |
|---:|---|---|---|
| 0 | tokio @ 6e3d753fcc: `tokio-util/src/io/simplex.rs:185` | yes | lock().unwrap() can double-panic on a poisoned lock |
| 1 | zellij @ ffb334a949: `zellij-server/src/plugins/pinned_executor.rs:381` | yes | lock().unwrap() can double-panic on a poisoned lock |
| 2 | rustls @ 28ee6c688e: `rustls/src/msgs/codec.rs:308` | no | infallible slice conversion |
| 3 | tokio @ 6e3d753fcc: `tokio/src/sync/broadcast.rs:1625` | no | asserted channel invariant |
| 4 | diesel @ dde3b22cae: `diesel/src/sqlite/connection/raw.rs:929` | no | guarded by thread::panicking() (now exempt) |
| 5 | typst @ 64c5e8c02f: `crates/typst-utils/src/defer.rs:15` | no | Option invariant |
| 6 | tokio @ 6e3d753fcc: `tokio/src/runtime/io/driver/uring.rs:161` | yes | io_uring submit can fail |
| 7 | diesel @ dde3b22cae: `diesel/src/util.rs:139` | no | deliberate abort bomb |
| 8 | actix-web @ 88d7742dae: `actix-web/src/request.rs:583` | no | Rc invariant (commented) |
| 9 | wgpu @ 100c142937: `wgpu/src/backend/webgpu.rs:4663` | no | OnceCell invariant |

### unwrap

| # | Location | Real? | Why |
|---:|---|---|---|
| 0 | rustls @ 28ee6c688e: `rustls/src/conn/split.rs:292` | no | state invariant |
| 1 | typst @ 64c5e8c02f: `crates/typst-pdf/src/tags/context/table.rs:600` | no | index invariant |
| 2 | just @ d6fee03637: `src/token.rs:29` | no | line exists by construction |
| 3 | zellij @ ffb334a949: `zellij-server/src/lib.rs:4506` | yes | client removal is I/O |
| 4 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:1628` | no | internal conversion |
| 5 | wgpu @ 100c142937: `naga/src/back/glsl/writer.rs:923` | no | invariant |
| 6 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:164` | no | internal conversion |
| 7 | meilisearch @ 7b6f9f39bd: `crates/fuzzers/src/bin/fuzz-indexing.rs:162` | no | fuzz binary |
| 8 | delta @ 3c2269c6b8: `src/paint.rs:586` | no | invariant |
| 9 | wgpu @ 100c142937: `naga/src/back/spv/block.rs:2252` | no | invariant |
| 10 | cargo @ b57863321f: `src/workspace/parser/mod.rs:290` | no | manifest path has a parent |
| 11 | tantivy @ b9c4ed1c62: `src/store/reader.rs:317` | no | checkpoint invariant |
| 12 | gitoxide @ 9f805e24a0: `gix-index/src/lib.rs:255` | no | fixed-size slice |
| 13 | tokio @ 6e3d753fcc: `tests-integration/src/bin/test-mem.rs:8` | no | integration-test binary |
| 14 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:2515` | yes | reply decoded from the host process |
| 15 | meilisearch @ 7b6f9f39bd: `crates/milli/src/update/facet/incremental.rs:323` | no | iterator invariant |
| 16 | zellij @ ffb334a949: `zellij-utils/src/input/layout.rs:1892` | no | index invariant |
| 17 | image @ 075e65d618: `src/imageops/colorops.rs:165` | no | numeric cast of a bounded value |
| 18 | helix @ ba40e54742: `helix-lsp/src/client.rs:1675` | no | capabilities set at initialisation |
| 19 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:2661` | no | internal conversion |

