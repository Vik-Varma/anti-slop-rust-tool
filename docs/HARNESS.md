# The harness

The rules judge code. The harness judges a *change*, which is what an AI coding agent produces:
it runs the rules on the lines the change touched, watches what the change did to the things that
measure it (tests, benchmarks, profiles, lint levels), and gives a verdict an agent, a hook or a
CI job can act on. It needs `git`; nothing else in the tool does.

```sh
anti-slop check --since main            # findings on lines changed since main, nothing else
anti-slop guard --since main            # the same, plus the measuring-stick report and a verdict
anti-slop guard --staged --format agent # what the next commit contains, written for an agent
anti-slop hook stop < event.json        # the hook protocol (Claude Code native or Symposium)
```

Exit codes: `guard` exits 1 when the policy says something blocks, `check --since` when there is
any finding on a changed line, both exit 2 on a file that can't be read or parsed or a bad
policy file.

## Why a change, not a tree

Two reasons, both measured elsewhere in this project.

The rules' precision was audited on mature, reviewed code, where most findings turned out to be
guarded by something a syntax check can't see ([AUDIT.md](AUDIT.md)). Google's data on its own
analysers says findings on fresh code are real about three times as often as findings on
checked-in code (74% against 21%, Sadowski et al., CACM 2018). An agent's diff is fresh code, so
the same rules are worth more there, and the pre-existing findings in the files it touched would
only drown the signal. `check --since` lists findings on changed lines and counts the rest.

The second reason is that an agent given a check will, often enough, satisfy the check rather than
the goal. The record of this is specific: an agent asked for speed reported a 34,500× speedup by
switching the physics engine off; another cut the number of training epochs the benchmark ran;
others added `target-cpu=native`, ran benchmarks in parallel, wrote their own benchmark harness,
or tweaked a constant and "called it a day" ([Woolf, 2026](https://minimaxir.com/2026/09/agentic-iteration/);
Ferrock's `AGENTS.md` records the same incidents from the other side). None of those is visible to
a linter run on the result. All of them are visible in the diff.

## `guard`: the measuring stick and the handoff

`guard` runs `check --since` and then reads the diff itself, every file, not only Rust. Each item
has a kind; the policy says which kinds block and which are only reported.

| Kind | What it catches | Default |
|---|---|---|
| `findings` | warnings and errors (or the configured severity) on changed lines | block |
| `suppression-without-reason` | `// anti-slop: allow(rule)` added with no reason, or one under 12 characters | block |
| `tests-removed` | more `#[test]` functions removed than added in a file | block |
| `tests-ignored` | `#[ignore]` added | block |
| `flags` | `target-cpu=native`, `RUSTFLAGS` or `rustflags` added anywhere | block |
| `suppressions` | any `anti-slop: allow(..)` added, with its reason | report |
| `allow-attributes` | `#[allow(..)]` or `#[expect(..)]` added | report |
| `assertions-removed` | more assertions removed than added in a test file | report |
| `deferrals` | `todo!()`, `unimplemented!()`, or a TODO/FIXME/HACK comment added | report |
| `measuring-stick` | a benchmark, test, `[profile]`, `[lints]`, `.cargo/config`, toolchain, lint-config or CI file changed | report |
| `size` | a changed file over 1,000 code lines | report |

Two details come straight from the record. Sizes are code lines, not raw lines, because an agent
told to shrink a file will delete the comments; and a `Cargo.toml` edit is a measuring-stick change
only when it touches a profile or lint level, because dependency bumps are ordinary work.

The report's reported-only items are the handoff: the list of what a person must look at before
merging agent work, which is what the agent reaches for when a check is in its way. A `handoff`
note written by the agent should cite the report rather than restate it; the hook saves each
report under the temp directory for that purpose.

## The policy

`anti-slop.toml` at the repository root; built-in defaults when absent. Every key is checked, so a
misspelt key is an error rather than a silently looser policy.

```toml
base = "origin/main"       # what --since compares against when not given (default: HEAD)
style = false              # also run the style rules
skip = []                  # rules never reported
min_severity = "warning"   # what counts as a finding on changed lines
block = ["findings", "suppression-without-reason", "tests-removed", "tests-ignored", "flags"]
min_reason_chars = 12
max_file_lines = 1000
```

The skill tells agents not to edit this file unless asked; `guard` itself reports it as a
measuring-stick change when it appears in the diff.

## The agent format

`--format agent` is summary first: a verdict line with counts, one line per finding (`severity
file:line rule message`), one line per guard item, a cap of 30 with "and N more", and one line
on what to do next. The long material (why a rule exists, its evidence, the exceptions its source
allows) stays behind `anti-slop explain <rule>`, so a report costs little context and loses
nothing that a downstream compressor could drop: a test asserts that every rule id and
`file:line` survives the cap. This is the shape
[Token Optimizer](https://github.com/alexgreensh/token-optimizer)'s archive-and-expand pattern
and the [caveman](https://github.com/JuliusBrussee/caveman) skill's "payload verbatim" rule both
point at; the format works unchanged under either.

## The hook

`anti-slop hook <event>` reads one event as JSON on stdin and answers on stdout, in whichever of
two dialects the input used:

- Claude Code's native events (fields at the top level), registered in `.claude/settings.json`;
- Symposium's canonical events (`{"PreToolUse": {..}}`), which `cargo agents` delivers for
  Claude Code, Codex, Copilot, Antigravity and Kiro.

Three events are handled:

| Event | What it does | Blocks? |
|---|---|---|
| `pre-tool-use` | on `git commit` (staged) or `git push` (against upstream): runs `guard`, denies the command while something blocks | yes |
| `post-tool-use` | after an edit to a `.rs` file: that file's findings, added to the agent's context | no |
| `stop` | runs `guard` against the policy's base; sends the agent back while something blocks, at most 3 times per session, then lets it stop and prints the report for a person | Claude native only |

Two rules of conduct, both learned from tools that got them wrong: the hook **appends, never
substitutes** (it adds context; it never rewrites what a command printed, since agents have been
measured acting on a rewritten tool result 16–50% of the time), and it **fails open on its own
errors** (unreadable input, no git repository, a bad policy file let the action through and say so
on stderr). The one thing it blocks on is the policy.

Every run's report is also saved to `$TMPDIR/anti-slop-report-<session>.txt`. Set
`ANTI_SLOP_LOG=/path/runs.jsonl` to append one line per hook run to the run log.

## Where it plugs in

**Symposium.** `Symposium.toml` at the root of this repository is the plugin: it installs the
binary (`cargo install --git`), ships `skills/anti-slop/SKILL.md`, registers
`cargo agents anti-slop <args>`, and three hooks (`PreToolUse` and `PostToolUse` in the canonical
format for every agent; `Stop` in Claude's native format, since only that one can refuse a stop).
Until it is in Symposium's reviewed recommendations repository, add it as a custom source:

```toml
# ~/.symposium/config.toml
[[registry]]
name = "anti-slop"
git = "https://github.com/Vik-Varma/anti-slop-rust-tool"
```

then `cargo agents use anti-slop`. The plugin matches every workspace (`depends-on = ["*"]`),
since anti-slop is a tool, not a dependency.

**Claude Code without Symposium.** In `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "anti-slop hook pre-tool-use" }] }],
    "PostToolUse": [{ "matcher": "Edit|Write|MultiEdit", "hooks": [{ "type": "command", "command": "anti-slop hook post-tool-use" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "anti-slop hook stop" }] }]
  }
}
```

**GitHub Actions.** `action.yml` runs `guard` over a pull request's diff and writes the report
to the job summary:

```yaml
- uses: actions/checkout@v5
  with: { fetch-depth: 0 }
- uses: Vik-Varma/anti-slop-rust-tool@main
  with: { base: "${{ github.event.pull_request.base.sha }}" }
```

**A `CLAUDE.md` for the project.** `templates/CLAUDE.md` is a 250-word instruction file for a
repository that uses the harness: how to run it, how to read it, and the four rules that keep an
agent from passing the check instead of the task. Copy it in and set the base branch.

**Beside other agent tooling.** [Token Optimizer](https://github.com/alexgreensh/token-optimizer)
compresses recognised commands' output; the harness output is already compact, so leave
`anti-slop` out of its compression (`[filters.exclude] commands = ["anti-slop"]`) rather than let
its lint handler group findings by rule and drop locations. The
[caveman](https://github.com/JuliusBrussee/caveman) skill compresses prose and leaves code,
paths and errors alone; it can stay on. The
[intent-layer](https://github.com/crafter-station/skills/tree/main/skills/intent-layer) method
puts a project's invariants in per-directory `AGENTS.md` files; reading those for per-directory
policy and checking the invariants syntax can check is the next step for this harness, not part
of it yet.

## The run log

`--log FILE` (or `ANTI_SLOP_LOG` for the hook) appends one JSON line per run: time, base, files,
findings by rule, guard items by kind, what blocked, code lines added and removed, and the byte
size of what was emitted. Counts only. There is no published evidence yet that feeding
static-analysis findings back to an agent improves its output rather than teaching it to
suppress them, and the log exists to find out: if findings fall while suppressions rise, the
harness is being gamed, and the log will show it.

## What it doesn't do

- It doesn't run tests or benchmarks, and can't tell whether a benchmark's inputs shrank; it can
  only say that the benchmark file changed and leave the diff to a person.
- `--staged` reads the diff from the index but analyses the working tree, so an unstaged edit to a
  staged file is judged as it is on disk.
- A local function named `catch_unwind`, or a `use` of a module whose last segment looks like a
  function, is taken at face value, as the rules' own entries say.
- Nothing here measures whether the loop helps. The log is the instrument; the measurement is
  still to be done.
