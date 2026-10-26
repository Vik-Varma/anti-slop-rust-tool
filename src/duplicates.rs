//! Cross-file duplicated-code detection.
//!
//! Token-based: the source is tokenised, literals are normalised (so code
//! differing only in constants still matches) while identifiers are kept,
//! and runs of at least `min_duplicate_tokens` identical tokens are reported.

use crate::analyze::FileReport;
use crate::config::Config;
use crate::context::FileContext;
use crate::finding::{Category, Evidence, Finding, Severity};
use crate::rules::{Input, Rule, RuleMeta, Sink};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::str::FromStr;

pub static META: RuleMeta = RuleMeta {
    id: "duplicate-code",
    category: Category::Maintainability,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "Long block of code duplicated elsewhere in the project",
    rationale: "Copy-pasted code must be changed consistently; when a fix is applied to one copy \
and not the others, the bug survives. Juergens et al. (ICSE 2009) found that inconsistently \
changed clones contained faults in practice. Not all duplication is harmful (Kapser & Godfrey, \
2008), so only long runs of identical tokens are reported.",
    sources: &[
        "Juergens, Deissenboeck, Hummel, Wagner. Do code clones matter? ICSE 2009, pp. 485-495. arXiv:1701.05472",
        "Rahman, Bird, Devanbu. Clones: what is that smell? EMSE 17, 2012 (found clones not more bug-prone).",
        "Kapser, Godfrey. 'Cloning considered harmful' considered harmful. EMSE 13(6), 2008. doi:10.1007/s10664-008-9076-6",
    ],
};

/// Catalogue entry; the work happens in [`check`] because it spans files.
pub struct DuplicateCode;

impl Rule for DuplicateCode {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
    fn check(&self, _: &Input, _: &mut Sink) {}
}

#[derive(Debug, Clone, Copy)]
pub struct Tok {
    pub hash: u64,
    pub line: usize,
}

/// Tokenises production code of a file for duplicate detection.
pub fn tokens(ctx: &FileContext) -> Vec<Tok> {
    let Ok(ts) = TokenStream::from_str(ctx.source) else { return Vec::new() };
    let mut out = Vec::new();
    flatten(ts, &mut out);
    out.retain(|t| ctx.is_production_line(t.line) && !ctx.is_suppressed(META.id, t.line));
    out
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn flatten(ts: TokenStream, out: &mut Vec<Tok>) {
    let mut iter = ts.into_iter().peekable();
    while let Some(tt) = iter.next() {
        let line = tt.span().start().line;
        match tt {
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                if !open.is_empty() {
                    out.push(Tok { hash: hash_str(open), line });
                }
                flatten(g.stream(), out);
                if !close.is_empty() {
                    out.push(Tok { hash: hash_str(close), line: g.span_close().start().line });
                }
            }
            TokenTree::Punct(p) => {
                // Skip `#[doc = "..."]` attributes produced by doc comments.
                if p.as_char() == '#' {
                    if let Some(TokenTree::Group(g)) = iter.peek() {
                        if g.delimiter() == Delimiter::Bracket
                            && g.stream().into_iter().next().is_some_and(|t| t.to_string() == "doc")
                        {
                            iter.next();
                            continue;
                        }
                    }
                }
                out.push(Tok { hash: hash_str(&p.as_char().to_string()), line });
            }
            TokenTree::Ident(i) => out.push(Tok { hash: hash_str(&i.to_string()), line }),
            TokenTree::Literal(_) => out.push(Tok { hash: hash_str("<lit>"), line }),
        }
    }
}

fn window_hash(toks: &[Tok]) -> u64 {
    let mut h = DefaultHasher::new();
    for t in toks {
        t.hash.hash(&mut h);
    }
    h.finish()
}

/// Windows with very few distinct tokens (tables of numbers, repeated
/// `x, x, x`) are not meaningful duplication.
fn is_varied(toks: &[Tok]) -> bool {
    let distinct: HashSet<u64> = toks.iter().map(|t| t.hash).collect();
    distinct.len() * 5 >= toks.len()
}

pub fn check(reports: &mut [FileReport], _files: &[PathBuf], config: &Config) {
    if !config.rule_enabled(&META) {
        return;
    }
    let w = config.min_duplicate_tokens.max(10);
    // window hash -> first (file index, token index) where it occurs
    let mut first_seen: HashMap<u64, (usize, usize)> = HashMap::new();
    let mut new_findings: Vec<(usize, Finding)> = Vec::new();

    for fi in 0..reports.len() {
        let toks = std::mem::take(&mut reports[fi].tokens);
        let mut i = 0;
        while i + w <= toks.len() {
            let h = window_hash(&toks[i..i + w]);
            match first_seen.get(&h).copied() {
                Some((ofi, oi)) if (ofi != fi || oi + w <= i) && is_varied(&toks[i..i + w]) => {
                    let other: &[Tok] = if ofi == fi { &toks } else { &reports[ofi].tokens };
                    if other[oi..oi + w].iter().zip(&toks[i..i + w]).all(|(a, b)| a.hash == b.hash) {
                        // Extend the match as far as it goes (without overlapping itself).
                        let mut len = w;
                        while i + len < toks.len()
                            && oi + len < other.len()
                            && (ofi != fi || oi + len < i)
                            && other[oi + len].hash == toks[i + len].hash
                        {
                            len += 1;
                        }
                        let start_line = toks[i].line;
                        let end_line = toks[i + len - 1].line;
                        let orig_line = other[oi].line;
                        let orig_file = reports[ofi].file.display().to_string();
                        new_findings.push((
                            fi,
                            Finding {
                                rule: META.id,
                                category: META.category,
                                severity: META.severity,
                                file: reports[fi].file.clone(),
                                line: start_line,
                                column: 1,
                                message: format!(
                                    "lines {start_line}-{end_line} ({len} tokens) duplicate {orig_file}:{orig_line}"
                                ),
                            },
                        ));
                        i += len;
                        continue;
                    }
                }
                Some(_) => {}
                None => {
                    first_seen.insert(h, (fi, i));
                }
            }
            i += 1;
        }
        reports[fi].tokens = toks;
    }

    for (fi, f) in new_findings {
        reports[fi].findings.push(f);
    }
    for r in reports.iter_mut() {
        r.findings.sort_by_key(|f| (f.line, f.column, f.rule));
        r.tokens.clear();
    }
}
