# Bug-fix commit classification check

`validation/mining/fix_commit_sample.py` draws 30 commits classified as bug fixes and 30 classified as non-fixes by `mine.py` (seed 42, 2023-07-01 to 2025-07-01, all 22 repositories: 16,861 first-parent commits, 3,206 classified as fixes). Each subject was judged by hand: y = defect fix, n = not a defect fix, ? = cannot tell from the subject (counted as n).

Result: **18/30 (60%)** of commits classified as fixes are defect fixes; **3/30 (10%)** of commits classified as non-fixes are defect fixes. This is in the range usually reported for keyword-based SZZ heuristics. The misclassification does not depend on the code being analysed, so it biases odds ratios towards 1 (it hides real effects rather than creating false ones).

## Classified as fix

- [y] `rustls_rustl | unbuffered: test and fix TLS1.2 closure on key exhaustion`
- [y] `helix-editor | Use `try_lock` in `diff_handle` for Diff gutter (#11092)`
- [y] `alacritty_al | Fix env variable overrides through CLI`
- [n] `zellij-org_z | feat(ux): tips on startup (#3988)`
- [n] `nushell_nush | Fix --header-row description (#14065)`
- [y] `nushell_nush | better error handling for `view source` (#14624)`
- [y] `nushell_nush | fix(parser): mixed side effects of different choices in `parse_oneof` (#14912)`
- [n] `helix-editor | languages: update rescript grammar (#8962)`
- [y] `zellij-org_z | fix(plugins): styling in the "about" plugin  (#4062)`
- [y] `helix-editor | Fix Teal LSP name. (#12395)`
- [y] `sharkdp_bat | Merge pull request #2835 from FilipRazek/fix-long-file-names`
- [y] `zellij-org_z | fix(tiled-panes): recover from full stack (#4011)`
- [n] `rust-lang_re | automata: fix one outdated regex-cli test command`
- [n] `helix-editor | CI: Set a timeout for the test suite workflow`
- [?] `rust-lang_ru | Merge pull request #18995 from alibektas/12210`
- [n] `nushell_nush | Add is-terminal to determine if stdin/out/err are a terminal (#10970)`
- [y] `alacritty_al | Fix Vi cursor not being dirty when scrolling`
- [y] `alacritty_al | Fallback to underline shader when dotted fails`
- [y] `helix-editor | fix `:yank-joined` disrespecting default-yank-register option (#12890)`
- [y] `nushell_nush | fix(lsp): exit on null root_dir (#15051)`
- [y] `nushell_nush | fix: unknown span for special variables `$in/$it` (#14789)`
- [y] `rust-lang_ca | Make sure search paths inside OUT_DIR precede external paths (#15221)`
- [?] `rust-lang_ru | Merge pull request #18913 from Wilfred/grammar_fix`
- [y] `alacritty_al | Powerline glyphs being cut for narrow fonts`
- [?] `rust-lang_ru | Merge pull request #19875 from ShoyuVanilla/issue-19844`
- [n] `nushell_nush | feat(completion): stdlib virtual path completion & exportable completion (#15270)`
- [y] `tokio-rs_tok | macros: fix trait_method breaking change detection (#6308)`
- [n] `rustls_rustl | Allow optional use of aws-lc-rs`
- [y] `tokio-rs_axu | Fix panic location for overlapping method routes`
- [n] `rust-lang_re | automata: fix more out-dated regex-cli commands`

## Classified as non-fix

- [n] `rust-lang_re | fuzz: add all fuzzers to OSS-fuzz`
- [n] `nushell_nush | bump patch version`
- [n] `rust-lang_ru | Merge pull request #18647 from Veykril/push-nsrrmmnzzoym`
- [n] `rustls_rustl | Add API exposing shape of the performed handshake`
- [n] `nushell_nush | remove str escape-glob command (#12241)`
- [n] `tokio-rs_tra | subscriber: support `NO_COLOR` in `fmt::Layer` (#2647)`
- [n] `BurntSushi_r | grep-pcre2-0.1.7`
- [n] `tokio-rs_axu | Add #[debug_middleware] (#2725)`
- [n] `tokio-rs_tra | chore: prepare tracing-journald 0.3.1 (#3166)`
- [y] `helix-editor | make replace with clipboard commands exit select mode`
- [n] `starship_sta | build(deps): update crate-ci/typos action to v1.29.4`
- [n] `rust-lang_ru | Merge pull request #20087 from Veykril/push-rrtkwpzmmwwz`
- [n] `rust-lang_ca | Auto merge of #14599 - epage:charter, r=weihanglo`
- [n] `nushell_nush | Bump base64 from 0.21.7 to 0.22.0 (#12188)`
- [n] `helix-editor | Support drawing popup frame (#4313)`
- [n] `meilisearch_ | Merge #4080`
- [n] `tokio-rs_axu | Update example low-level-rustls to axum 0.7 (#2375)`
- [n] `rust-lang_ca | Auto merge of #14753 - epage:resoler-prep, r=ehuss`
- [y] `helix-editor | goto_word: Skip keys with modifiers in both on-next-key blocks`
- [y] `helix-editor | Show successfully requested code actions after a failed request (#13156)`
- [n] `rust-lang_ca | Auto merge of #13272 - linyihai:cargo-add-public-manpage, r=epage`
- [n] `helix-editor | feat(lsp): add protobuf language servers (#12936)`
- [n] `rust-lang_ca | Auto merge of #13993 - epage:path-source, r=ehuss`
- [n] `rust-lang_ca | Auto merge of #14496 - tweag:cargo-package-doc, r=epage`
- [n] `rustls_rustl | prepare Codec for non-allocating decoding`
- [n] `nushell_nush | Bump crate-ci/typos from 1.21.0 to 1.22.0`
- [n] `tokio-rs_tra | mock: add `ExpectedId` to link span expectations (#3007)`
- [n] `clap-rs_clap | chore: Release`
- [n] `starship_sta | build(deps): update crate-ci/typos action to v1.17.0`
- [n] `rust-lang_ru | Auto merge of #18128 - ChayimFriedman2:external-macros-lint, r=Veykril`
