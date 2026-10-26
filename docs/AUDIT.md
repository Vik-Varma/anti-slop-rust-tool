# Precision audit

Each rule was run over 37 codebases: the 35 study projects at the commits in `study/corpus-heads.txt`, plus Ferrock and Dataminer. Findings were sampled at random per rule (20 when a rule had more than 200 findings, else 10) with `study/audit_sample.py`, then judged by hand from the surrounding code. The question for each finding: is this the failure the rule describes, reachable in shipping code?

## Rounds

| Round | Seed | Build | Used for |
|---|---|---|---|
| 1 | 2026 | before any audit fix | found the false-positive classes: test modules in their own files, `write!` into a `String`, custom `write` methods, deliberate `mem::forget`, `unimplemented!` as "unsupported" |
| 2 | 31337 | after the round-1 fixes | confirmed them; split `panic-on-input` out of `unwrap` |
| 3 | 4242 | after the round-2 fixes | tuned `panic-on-input` (argument counts, `xtask-*` crates), `panic-in-drop` (assertions, `thread::panicking()` guards) and `ignored-io-error` (only in functions returning `Result`) |
| 4 | 777 | final build | **validation**: nothing was tuned on it, it was judged by two raters, and its settled numbers set the severities |

## Two raters

The round-4 sample was judged twice. Rater 1 (the author) judged from the code around each finding. Rater 2, an independent Claude agent, was given only the findings and the neutral criteria in `docs/audit/rater2-instructions.txt`. It was kept from these docs and rater 1's verdicts, and traced callers and crate manifests before deciding. Its verdicts and reasons are in `docs/audit/rater2-*.csv`.

Agreement was 80% over 70 findings, Cohen's κ = 0.54 (moderate). Per rule: ignored-io-error 100% (κ 1.00), unwrap 90% (κ 0.44), panic-on-input 85% (κ 0.67), panic-in-drop 80% (κ 0.41), todo 30% (κ 0.00). Nearly every disagreement went one way: rater 1 said real where rater 2 showed by tracing the code that the path can't be reached. Examples: a `todo!()` on an enum variant no caller ever returns, or on a query type the Metal backend never advertises. Spot checks of rater 2's reachability claims held, so where the two disagreed, rater 2's verdict stands. The one exception is `todo` #5, a public function in a published crate, which any downstream user can call.

## Results by rule

| Rule | Findings in corpus | Rater 1 | Rater 2 | Settled | Severity |
|---|---:|---|---|---|---|
| todo | 15 | 10 / 10 | 3 / 10 | **4 / 10** | note |
| panic-on-input | 222 | 8 / 20 | 5 / 20 | **5 / 20** | note |
| ignored-io-error | 14 | 4 / 10 | 4 / 10 | **4 / 10** | note |
| panic-in-drop | 42 | 3 / 10 | 1 / 10 | **1 / 10** | note |
| unwrap | 5,214 | 2 / 20 | 2 / 20 | **2 / 20** | note |
| lock-held-across-await | 7 | 7 / 7 hold a guard | 7 / 7 hold a guard, 0 / 7 can deadlock | **0 / 7 can deadlock** | note |

Rules judged in earlier rounds by rater 1 only:

| Rule | Findings in corpus | Real / sampled | Round | Severity |
|---|---:|---|---|---|
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

| # | Location | Rater 1 | Rater 2 | Settled | Why (settled) |
|---:|---|---|---|---|---|
| 0 | nushell @ 706259d6f8: `crates/nu-explore/src/explore/pager/mod.rs:458` | yes | no | **no** | No command.react returns Transition::Cmd |
| 1 | tauri @ 1fc6d9c85c: `crates/tauri/src/tray/mod.rs:196` | yes | no | **no** | All variants of pinned tray-icon version covered |
| 2 | nushell @ 706259d6f8: `crates/nu-cmd-lang/src/core_commands/error_make.rs:158` | yes | no | **no** | value is always Record or String |
| 3 | wgpu @ 100c142937: `wgpu/src/backend/webgpu.rs:2312` | yes | yes | **yes** | Public API on web backend reaches todo with acceleration structure binding |
| 4 | wgpu @ 100c142937: `wgpu/src/backend/webgpu.rs:3687` | yes | yes | **yes** | PipelineCache::get_data on web backend panics |
| 5 | tauri @ 1fc6d9c85c: `crates/tauri-utils/src/acl/identifier.rs:75` | yes | no | **yes** | public function in a published crate: any downstream caller panics |
| 6 | wgpu @ 100c142937: `wgpu-hal/src/gles/queue.rs:451` | yes | no | **no** | Validation prevents both buffers being CPU-emulated |
| 7 | cargo @ b57863321f: `src/workspace/source_id.rs:414` | yes | yes | **yes** | Nightly builtin dependency reaches SourceId::load todo |
| 8 | gitoxide @ 9f805e24a0: `gix-ref/src/store/general/handle/find.rs:30` | yes | no | **no** | pub(crate) dead scaffolding, never constructed |
| 9 | wgpu @ 100c142937: `wgpu-hal/src/metal/command.rs:889` | yes | no | **no** | Metal never advertises pipeline statistics queries |

### panic-on-input

| # | Location | Rater 1 | Rater 2 | Settled | Why (settled) |
|---:|---|---|---|---|---|
| 0 | mdBook @ 8c41b823bb: `crates/mdbook-compare/src/main.rs:79` | no | no | **no** | mdbook-compare is an unpublished developer tool |
| 1 | eza @ 471bfbc7b0: `src/options/parser.rs:319` | yes | yes | **yes** | Non-UTF-8 CLI argument panics |
| 2 | tauri @ 1fc6d9c85c: `crates/tauri/src/plugin/mobile.rs:398` | yes | no | **no** | Payload is JSON from in-process Swift side; always UTF-8 |
| 3 | rustls @ 28ee6c688e: `bogo/src/opts.rs:575` | no | no | **no** | bogo is a test shim |
| 4 | meilisearch @ 7b6f9f39bd: `crates/milli/src/update/index_documents/helpers/mod.rs:66` | no | no | **no** | fst built internally from strings |
| 5 | cargo @ b57863321f: `src/compiler/build_context/target_info.rs:581` | yes | yes | **yes** | rustc (or wrapper) output may be non-UTF-8 |
| 6 | meilisearch @ 7b6f9f39bd: `crates/milli/src/update/index_documents/helpers/merge_functions.rs:278` | no | no | **no** | Internally serialized JSON in merge |
| 7 | cargo @ b57863321f: `src/workspace/parser/mod.rs:988` | yes | yes | **yes** | Non-UTF-8 workspace or config path panics |
| 8 | zola @ e93c3de50a: `components/content/src/front_matter/datetime.rs:48` | no | no | **no** | Regex guarantees short digit strings |
| 9 | regex @ 80b9d14e4b: `regex-syntax/src/debug.rs:24` | no | no | **no** | Bytes are internally escaped ASCII |
| 10 | wgpu @ 100c142937: `naga/src/front/wgsl/parse/number.rs:642` | no | no | **no** | Lexer guarantees valid float syntax |
| 11 | cargo @ b57863321f: `crates/semver-check/src/main.rs:228` | no | no | **no** | semver-check is a developer tool |
| 12 | rustls @ 28ee6c688e: `bogo/src/opts.rs:433` | no | no | **no** | bogo is a test shim |
| 13 | meilisearch @ 7b6f9f39bd: `crates/milli/src/vector/embedder/rest.rs:248` | yes | yes | **yes** | Malformed env var value panics server |
| 14 | rustls @ 28ee6c688e: `bogo/src/opts.rs:564` | no | no | **no** | bogo is a test shim |
| 15 | meilisearch @ 7b6f9f39bd: `crates/dump/src/reader/v6/mod.rs:275` | yes | yes | **yes** | Non-UTF-8 file name in imported dump panics |
| 16 | meilisearch @ 7b6f9f39bd: `crates/dump/src/reader/v4/mod.rs:105` | yes | no | **no** | Same file already opened in V4Reader::open |
| 17 | cargo @ b57863321f: `src/workspace/parser/mod.rs:3162` | yes | no | **no** | Path built from String; to_str cannot fail |
| 18 | cargo @ b57863321f: `src/compiler/compile_kind.rs:218` | no | no | **no** | Canonical path already checked UTF-8; stem always exists |
| 19 | cargo @ b57863321f: `src/workspace/global_cache_tracker.rs:1801` | no | no | **no** | Test-only env var hook |

### ignored-io-error

| # | Location | Rater 1 | Rater 2 | Settled | Why (settled) |
|---:|---|---|---|---|---|
| 0 | zellij @ ffb334a949: `zellij-utils/src/ipc.rs:608` | yes | yes | **yes** | BufWriter flush failure leaves message unsent, Ok returned |
| 1 | zellij @ ffb334a949: `zellij-utils/src/ipc.rs:598` | yes | yes | **yes** | BufWriter flush failure leaves message unsent, Ok returned |
| 2 | zellij @ ffb334a949: `zellij-utils/src/ipc.rs:460` | yes | yes | **yes** | BufWriter flush failure leaves message unsent, Ok returned |
| 3 | nushell @ 706259d6f8: `crates/nu-protocol/src/pipeline/byte_stream.rs:1217` | no | no | **no** | Error branch; caller still receives Err |
| 4 | zellij @ ffb334a949: `zellij-window/src/dbus.rs:1335` | no | no | **no** | FakeSessionBus is test-support only |
| 5 | nushell @ 706259d6f8: `crates/nu-command/src/filesystem/save.rs:703` | no | no | **no** | Error branch; function returns Err anyway |
| 6 | gitoxide @ 9f805e24a0: `gix-credentials/src/helper/invoke.rs:13` | yes | yes | **yes** | Store/erase write to helper fails silently, Ok returned |
| 7 | fd @ c44735ec0f: `src/walk.rs:261` | no | no | **no** | Returns Err(KilledBySigint) anyway |
| 8 | nushell @ 706259d6f8: `crates/nu-protocol/src/pipeline/byte_stream.rs:1199` | no | no | **no** | Error branch; caller still receives Err |
| 9 | zellij @ ffb334a949: `zellij-window/src/unattended.rs:62` | no | no | **no** | Log rotation rename only; no data loss reported as success |

### panic-in-drop

| # | Location | Rater 1 | Rater 2 | Settled | Why (settled) |
|---:|---|---|---|---|---|
| 0 | tokio @ 6e3d753fcc: `tokio-util/src/io/simplex.rs:185` | yes | no | **no** | Lock never held across panicking code; poisoning not realistic |
| 1 | zellij @ ffb334a949: `zellij-server/src/plugins/pinned_executor.rs:381` | yes | no | **no** | Critical sections cannot realistically panic; no poisoning |
| 2 | rustls @ 28ee6c688e: `rustls/src/msgs/codec.rs:308` | no | no | **no** | Two-byte slice to [u8;2] is infallible |
| 3 | tokio @ 6e3d753fcc: `tokio/src/sync/broadcast.rs:1625` | no | no | **no** | Internal invariant: next < tail means channel non-empty |
| 4 | diesel @ dde3b22cae: `diesel/src/sqlite/connection/raw.rs:929` | no | no | **no** | Statements finalized first; unwinding case guarded by panicking() |
| 5 | typst @ 64c5e8c02f: `crates/typst-utils/src/defer.rs:15` | no | no | **no** | Option always Some until drop |
| 6 | tokio @ 6e3d753fcc: `tokio/src/runtime/io/driver/uring.rs:161` | yes | yes | **yes** | io_uring_enter syscall can fail (ENOMEM/EAGAIN); expect panics |
| 7 | diesel @ dde3b22cae: `diesel/src/util.rs:139` | no | no | **no** | Deliberate abort helper; panic-in-drop is the intent |
| 8 | actix-web @ 88d7742dae: `actix-web/src/request.rs:583` | no | no | **no** | extensions Rc never cloned; invariant maintained |
| 9 | wgpu @ 100c142937: `wgpu/src/backend/webgpu.rs:4663` | no | no | **no** | modified flag implies mapping initialized |

### unwrap

| # | Location | Rater 1 | Rater 2 | Settled | Why (settled) |
|---:|---|---|---|---|---|
| 0 | rustls @ 28ee6c688e: `rustls/src/conn/split.rs:292` | no | no | **no** | State consumed only on error, which returns early |
| 1 | typst @ 64c5e8c02f: `crates/typst-pdf/src/tags/context/table.rs:600` | no | no | **no** | Position comes from grid's own cells |
| 2 | just @ d6fee03637: `src/token.rs:29` | no | no | **no** | Token line is within its source |
| 3 | zellij @ ffb334a949: `zellij-server/src/lib.rs:4506` | yes | no | **no** | Fails only on poisoned lock; not realistic |
| 4 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:1628` | no | no | **no** | Conversion infallible for NewPane |
| 5 | wgpu @ 100c142937: `naga/src/back/glsl/writer.rs:923` | no | no | **no** | InsertBits always has arg2 |
| 6 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:164` | no | no | **no** | Conversion infallible for GenerateRandomName |
| 7 | meilisearch @ 7b6f9f39bd: `crates/fuzzers/src/bin/fuzz-indexing.rs:162` | no | no | **no** | Fuzzer binary |
| 8 | delta @ 3c2269c6b8: `src/paint.rs:586` | no | no | **no** | Guarded by is_some check |
| 9 | wgpu @ 100c142937: `naga/src/back/spv/block.rs:2252` | no | no | **no** | Access instruction always has result id |
| 10 | cargo @ b57863321f: `src/workspace/parser/mod.rs:290` | no | no | **no** | Manifest file path always has parent |
| 11 | tantivy @ b9c4ed1c62: `src/store/reader.rs:317` | no | no | **no** | Checkpoints cover range up to last_doc_id |
| 12 | gitoxide @ 9f805e24a0: `gix-index/src/lib.rs:255` | no | no | **no** | split_at_checked(4) guarantees four bytes |
| 13 | tokio @ 6e3d753fcc: `tests-integration/src/bin/test-mem.rs:8` | no | no | **no** | Integration test binary |
| 14 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:2515` | yes | yes | **yes** | Host stdin response may be missing or malformed |
| 15 | meilisearch @ 7b6f9f39bd: `crates/milli/src/update/facet/incremental.rs:323` | no | no | **no** | Group size invariant maintained by facet DB |
| 16 | zellij @ ffb334a949: `zellij-utils/src/input/layout.rs:1892` | no | no | **no** | Index from position() on same-length vec |
| 17 | image @ 075e65d618: `src/imageops/colorops.rs:165` | no | yes | **yes** | u32/u64 subpixel max does not fit i32 |
| 18 | helix @ ba40e54742: `helix-lsp/src/client.rs:1675` | no | no | **no** | Uncalled; callers filter to initialized servers |
| 19 | zellij @ ffb334a949: `zellij-tile/src/shim.rs:2661` | no | no | **no** | Conversion infallible for DeleteLayout |

### lock-held-across-await

| # | Location | Guard held across .await | Can deadlock as written | Why |
|---:|---|---|---|---|
| 0 | zellij @ ffb334a949: `zellij-client/src/web_client/unit/web_client_tests.rs:2665` | yes | no | mock_apis guard (l.2650) alive; map only locked when POST /session creates client |
| 1 | zellij @ ffb334a949: `zellij-client/src/web_client/unit/web_client_tests.rs:1409` | yes | no | mock_apis guard (l.1389) alive at sleep; server aborted, nothing else locks map |
| 2 | zellij @ ffb334a949: `zellij-client/src/web_client/unit/web_client_tests.rs:1792` | yes | no | first_messages_sent/sessions_created guards alive; only locked once at session spawn, already done |
| 3 | rustup @ ce2c96b063: `src/cli/self_update.rs:1349` | yes | no | StdoutLock held across download; no progress bar here, nothing else writes stdout |
| 4 | zellij @ ffb334a949: `zellij-client/src/web_client/unit/web_client_tests.rs:560` | yes | no | mock_apis guard (l.526) alive; no new session created during close |
| 5 | zellij @ ffb334a949: `zellij-client/src/web_client/unit/web_client_tests.rs:1792` | yes | no | Duplicate of #2 (same line); guards alive, no contender |
| 6 | zellij @ ffb334a949: `zellij-client/src/web_client/unit/web_client_tests.rs:1152` | yes | no | mock_apis guard (l.1135) alive; both clients created earlier, no contender |

