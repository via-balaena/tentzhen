//! Hacker Fab's stack beside the industry's, row by row, with the gap between them and Tentzhen's
//! route across it. It is `fab/stack.toml`, and tentzhen.com's fab page (`site/fab/`) shows it:
//!
//! ```toml
//! [[row]]
//! layer = "Lithography"            # the part of the stack; a layer's rows sit together
//! measure = "resolution"           # what the two sides are compared on
//! unit = "m"                       # the SI unit of both values; left out when neither has one
//! better = "lower"                 # lower or higher: which way the measure improves
//! route = "..."                    # Tentzhen's route across the gap: a proposal, `unknown`
//! with = ["Rust", "CortenForge"]   # what the route is built with
//! why_not = "..."                  # only when `with` has neither Rust nor CortenForge
//! gap = "..."                      # in words, only when a side has no value
//!
//! [row.hacker_fab]                 # then [row.industry], the same fields
//! value = 1e-6                     # in `unit`, or
//! is = "..."                       # the side in words, when it has no value
//! tool = "stepper V2.1"            # what has it
//! note = "..."                     # the conditions the source attaches, if any
//! quote = "..."                    # the source's own words, which hold the value
//! source = "hacker-fab/gitbook@bd1eef57:fab-toolkit/..."  # a URL, or owner/repo@sha:path
//! read = "2026-11-25"              # the UTC day it was read
//! ```
//!
//! A row's keys come before its two tables: in TOML a key after `[row.industry]` would be the
//! industry's.
//!
//! When both sides have a value the warehouse works out the gap, as a factor (`gold.page_fab`), so
//! a row gives `better` and no `gap`. Otherwise it gives `gap` in words and no `better`, and the
//! words carry no factor (`×`): a factor is only ever worked out. A row's
//! route is built in Rust or CortenForge where it can be (Jon, 2026-11-25: "I'd like to always use
//! CortenForge/Rust where possible"), so a route with neither says why not.
//!
//! Not held by anything: that a quote is its source's words, or that a value is what the quote
//! says. They are checked by hand when a row is written, against the source as read that day.

use crate::log::real_day;
use crate::{Source, read_source};
use serde::Deserialize;
use std::path::Path;

/// Where the stack lives, relative to the repo root.
pub const STACK: &str = "fab/stack.toml";

/// The units a value may be in: SI units, `1` for a count or ratio, and `%`.
pub const UNITS: [&str; 10] = ["m", "s", "W", "A", "V", "Hz", "K", "Pa", "1", "%"];

/// What a route may be built with.
pub const WITH: [&str; 5] = ["Rust", "CortenForge", "Amaranth", "KiCad", "bought"];

/// Which way a measure improves.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Better {
    Lower,
    Higher,
}

impl Better {
    pub fn as_str(self) -> &'static str {
        match self {
            Better::Lower => "lower",
            Better::Higher => "higher",
        }
    }
}

/// One side of a row: Hacker Fab's, or the industry's.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Side {
    pub value: Option<f64>,
    pub is: Option<String>,
    pub tool: String,
    pub note: Option<String>,
    pub quote: String,
    pub source: String,
    pub read: String,
}

/// One thing compared.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub layer: String,
    pub measure: String,
    pub unit: Option<String>,
    pub better: Option<Better>,
    pub route: String,
    pub with: Vec<String>,
    pub why_not: Option<String>,
    pub gap: Option<String>,
    pub hacker_fab: Side,
    pub industry: Side,
    /// The file the row came from.
    #[serde(skip)]
    pub record: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    row: Vec<Row>,
}

pub fn read(root: &Path) -> Result<Source, String> {
    read_source(root, &root.join(STACK))
}

/// A URL, or `owner/repo@sha:path` for a file in a repository at a commit.
fn is_source(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix("https://") {
        return !rest.is_empty() && !rest.contains(char::is_whitespace);
    }
    let Some((repo, rest)) = s.split_once('@') else {
        return false;
    };
    let Some((sha, path)) = rest.split_once(':') else {
        return false;
    };
    let mut parts = repo.split('/');
    let named = |p: Option<&str>| p.is_some_and(|p| !p.is_empty());
    named(parts.next())
        && named(parts.next())
        && parts.next().is_none()
        && sha.len() >= 7
        && sha
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        && !path.is_empty()
        && !path.contains(char::is_whitespace)
}

impl Side {
    fn check(&self, at: &str, named: &str) -> Result<(), String> {
        match (self.value, &self.is) {
            (Some(v), None) if v.is_finite() && v > 0.0 => {}
            (Some(_), None) => return Err(format!("{at}: {named}'s value is not above zero")),
            (None, Some(_)) => {}
            _ => {
                return Err(format!(
                    "{at}: {named} gives exactly one of `value` or `is`"
                ));
            }
        }
        let strings = [
            Some(&self.tool),
            self.is.as_ref(),
            self.note.as_ref(),
            Some(&self.quote),
            Some(&self.source),
        ];
        if strings.iter().flatten().any(|s| s.trim().is_empty()) {
            return Err(format!("{at}: {named} has an empty string"));
        }
        if !is_source(&self.source) {
            return Err(format!(
                "{at}: {named}'s source {:?} is no URL (https://) or owner/repo@sha:path",
                self.source
            ));
        }
        if !real_day(&self.read) {
            return Err(format!(
                "{at}: {named} was read on {:?}, which is no UTC day like \"2026-11-25\"",
                self.read
            ));
        }
        Ok(())
    }
}

/// Parses the file and holds each row to its shape.
pub fn parse(s: &Source) -> Result<Vec<Row>, String> {
    let at = s.path.as_str();
    let File { mut row } = toml::from_str(&s.text).map_err(|e| format!("{at}: {e}"))?;
    for r in &mut row {
        r.record = s.path.clone();
    }
    if row.is_empty() {
        return Err(format!("{at}: no rows"));
    }
    for (i, r) in row.iter().enumerate() {
        let named = format!("{} / {}", r.layer, r.measure);
        let strings = [
            Some(&r.layer),
            Some(&r.measure),
            Some(&r.route),
            r.why_not.as_ref(),
            r.gap.as_ref(),
        ];
        if strings.iter().flatten().any(|s| s.trim().is_empty()) {
            return Err(format!("{at}: {named} has an empty string"));
        }
        if row[..i]
            .iter()
            .any(|q| q.layer == r.layer && q.measure == r.measure)
        {
            return Err(format!("{at}: {named} is given twice"));
        }
        // A layer's rows sit together, so the page can head each layer once.
        if i > 0 && row[i - 1].layer != r.layer && row[..i].iter().any(|q| q.layer == r.layer) {
            return Err(format!(
                "{at}: {named} is apart from the rest of its layer: keep a layer's rows together"
            ));
        }
        r.hacker_fab
            .check(at, &format!("{named}, Hacker Fab's side"))?;
        r.industry
            .check(at, &format!("{named}, the industry's side"))?;
        let valued = [&r.hacker_fab, &r.industry]
            .iter()
            .filter(|s| s.value.is_some())
            .count();
        match (&r.unit, valued) {
            (None, 0) => {}
            (Some(u), 1 | 2) if UNITS.contains(&u.as_str()) => {}
            (Some(u), 1 | 2) => {
                return Err(format!(
                    "{at}: {named}'s unit {u:?} is not one of {UNITS:?}"
                ));
            }
            (Some(_), _) => {
                return Err(format!("{at}: {named} has a unit and no value"));
            }
            (None, _) => return Err(format!("{at}: {named} has a value and no unit")),
        }
        match (valued, r.better, &r.gap) {
            (2, Some(_), None) => {}
            (2, None, _) => {
                return Err(format!(
                    "{at}: {named} has two values, so it says which way is `better`"
                ));
            }
            (2, Some(_), Some(_)) => {
                return Err(format!(
                    "{at}: {named} has two values, so its gap is worked out, not written"
                ));
            }
            (_, None, Some(_)) => {}
            (_, Some(_), _) => {
                return Err(format!(
                    "{at}: {named} has no gap worked out, so `better` is not used"
                ));
            }
            (_, None, None) => {
                return Err(format!(
                    "{at}: {named} has a side without a value, so it says its `gap` in words"
                ));
            }
        }
        if r.gap.as_ref().is_some_and(|g| g.contains('×')) {
            return Err(format!(
                "{at}: {named}'s gap in words carries a factor: give both values and `better`, \
                 so that it is worked out"
            ));
        }
        if r.with.is_empty() {
            return Err(format!(
                "{at}: {named}'s route names what it is built `with`"
            ));
        }
        for (j, w) in r.with.iter().enumerate() {
            if !WITH.contains(&w.as_str()) {
                return Err(format!(
                    "{at}: {named} is built with {w:?}, not one of {WITH:?}"
                ));
            }
            if r.with[..j].contains(w) {
                return Err(format!("{at}: {named} names {w:?} twice"));
            }
        }
        let ours = r.with.iter().any(|w| w == "Rust" || w == "CortenForge");
        match (ours, &r.why_not) {
            (true, Some(_)) => {
                return Err(format!(
                    "{at}: {named} is built in Rust or CortenForge, so it has no `why_not`"
                ));
            }
            (false, None) => {
                return Err(format!(
                    "{at}: {named} is built with neither Rust nor CortenForge, so it says \
                     `why_not`"
                ));
            }
            _ => {}
        }
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIDE: &str =
        "tool = \"t\"\nquote = \"q\"\nsource = \"https://example.org/a\"\nread = \"2026-11-25\"\n";

    /// A file of one row: `keys` are the row's own, then each side's value line.
    fn file(keys: &str, hf: &str, ind: &str) -> Source {
        Source {
            path: STACK.into(),
            text: format!(
                "[[row]]\nlayer = \"L\"\nmeasure = \"M\"\nroute = \"r\"\n{keys}\n\
                 [row.hacker_fab]\n{hf}\n{SIDE}\n[row.industry]\n{ind}\n{SIDE}"
            ),
        }
    }

    const BOTH: &str = "unit = \"m\"\nbetter = \"lower\"\nwith = [\"Rust\"]";

    fn refused(keys: &str, hf: &str, ind: &str, says: &str) {
        let e = parse(&file(keys, hf, ind)).unwrap_err();
        assert!(e.contains(says), "{e:?} lacks {says:?}");
    }

    #[test]
    fn a_row_with_two_values_has_its_gap_worked_out() {
        let rows = parse(&file(BOTH, "value = 1e-6", "value = 13e-9")).unwrap();
        assert_eq!(rows[0].better, Some(Better::Lower));
        assert_eq!(rows[0].gap, None);
        refused(
            "unit = \"m\"\nwith = [\"Rust\"]",
            "value = 1",
            "value = 2",
            "`better`",
        );
        refused(
            &format!("{BOTH}\ngap = \"g\""),
            "value = 1",
            "value = 2",
            "worked out, not written",
        );
    }

    #[test]
    fn a_row_with_words_says_its_gap() {
        parse(&file(
            "with = [\"Rust\"]\ngap = \"g\"",
            "is = \"a\"",
            "is = \"b\"",
        ))
        .unwrap();
        parse(&file(
            "unit = \"m\"\nwith = [\"Rust\"]\ngap = \"g\"",
            "value = 1",
            "is = \"b\"",
        ))
        .unwrap();
        refused(
            "with = [\"Rust\"]",
            "is = \"a\"",
            "is = \"b\"",
            "`gap` in words",
        );
        refused(
            "with = [\"Rust\"]\nbetter = \"lower\"\ngap = \"g\"",
            "is = \"a\"",
            "is = \"b\"",
            "`better` is not used",
        );
        refused(
            "with = [\"Rust\"]\ngap = \"over 1,000×\"",
            "is = \"a\"",
            "is = \"b\"",
            "carries a factor",
        );
    }

    #[test]
    fn a_side_gives_a_value_or_words() {
        refused(BOTH, "value = 1\nis = \"a\"", "value = 2", "exactly one of");
        refused(BOTH, "", "value = 2", "exactly one of");
        refused(BOTH, "value = 0", "value = 2", "not above zero");
        refused(BOTH, "value = -1", "value = 2", "not above zero");
    }

    #[test]
    fn a_value_has_a_unit_from_the_list() {
        refused(
            "better = \"lower\"\nwith = [\"Rust\"]",
            "value = 1",
            "value = 2",
            "no unit",
        );
        refused(
            "unit = \"nm\"\nbetter = \"lower\"\nwith = [\"Rust\"]",
            "value = 1",
            "value = 2",
            "is not one of",
        );
        refused(
            "unit = \"m\"\nwith = [\"Rust\"]\ngap = \"g\"",
            "is = \"a\"",
            "is = \"b\"",
            "a unit and no value",
        );
    }

    #[test]
    fn a_route_is_built_in_rust_or_cortenforge_or_says_why_not() {
        parse(&file(
            "unit = \"m\"\nbetter = \"lower\"\nwith = [\"bought\"]\nwhy_not = \"rule 3\"",
            "value = 1",
            "value = 2",
        ))
        .unwrap();
        parse(&file(
            "unit = \"m\"\nbetter = \"lower\"\nwith = [\"CortenForge\"]",
            "value = 1",
            "value = 2",
        ))
        .unwrap();
        refused(
            "unit = \"m\"\nbetter = \"lower\"\nwith = [\"bought\"]",
            "value = 1",
            "value = 2",
            "says `why_not`",
        );
        refused(
            "unit = \"m\"\nbetter = \"lower\"\nwith = [\"Amaranth\"]",
            "value = 1",
            "value = 2",
            "says `why_not`",
        );
        refused(
            &format!("{BOTH}\nwhy_not = \"w\""),
            "value = 1",
            "value = 2",
            "has no `why_not`",
        );
        refused(
            "unit = \"m\"\nbetter = \"lower\"\nwith = []",
            "value = 1",
            "value = 2",
            "built `with`",
        );
        refused(
            "unit = \"m\"\nbetter = \"lower\"\nwith = [\"Python\"]",
            "value = 1",
            "value = 2",
            "not one of",
        );
        refused(
            "unit = \"m\"\nbetter = \"lower\"\nwith = [\"Rust\", \"Rust\"]",
            "value = 1",
            "value = 2",
            "twice",
        );
    }

    #[test]
    fn a_side_names_its_source_and_the_day_it_was_read() {
        let swap = |from: &str, to: &str| {
            let mut f = file(BOTH, "value = 1", "value = 2");
            f.text = f.text.replacen(from, to, 1);
            parse(&f).unwrap_err()
        };
        assert!(swap("https://example.org/a", "example.org/a").contains("no URL"));
        assert!(swap("https://example.org/a", "https://").contains("no URL"));
        assert!(swap("2026-11-25", "2026-02-30").contains("no UTC day"));
        assert!(swap("quote = \"q\"", "quote = \" \"").contains("empty string"));
        for good in [
            "hacker-fab/gitbook@bd1eef57:fab-toolkit/a.md",
            "https://www.asml.com/en/products",
        ] {
            let mut f = file(BOTH, "value = 1", "value = 2");
            f.text = f.text.replacen("https://example.org/a", good, 1);
            parse(&f).unwrap();
        }
        for bad in [
            "hacker-fab/gitbook@bd1e:a.md",
            "hacker-fab/gitbook@BD1EEF57:a.md",
            "hacker-fab@bd1eef57:a.md",
            "hacker-fab/gitbook@bd1eef57:",
            "hacker-fab/gitbook/x@bd1eef57:a.md",
            "/gitbook@bd1eef57:a.md",
            "hacker-fab/gitbook@bd1eef57:a b.md",
            "https://example.org/a b",
        ] {
            assert!(
                swap("https://example.org/a", bad).contains("no URL"),
                "{bad}"
            );
        }
    }

    #[test]
    fn no_field_is_an_empty_string() {
        let blank = |keys: &str, hf: &str| {
            let e = parse(&file(keys, hf, "is = \"b\"")).unwrap_err();
            assert!(e.contains("empty string"), "{keys} {hf}: {e}");
        };
        let words = "with = [\"Rust\"]\ngap = \"g\"";
        parse(&file(words, "is = \"a\"", "is = \"b\"")).unwrap();
        blank("with = [\"Rust\"]\ngap = \" \"", "is = \"a\"");
        blank(
            "with = [\"bought\"]\nwhy_not = \"\"\ngap = \"g\"",
            "is = \"a\"",
        );
        blank(words, "is = \" \"");
        let swap = |from: &str, to: &str| {
            let mut f = file(words, "is = \"a\"", "is = \"b\"");
            f.text = f.text.replacen(from, to, 1);
            parse(&f).unwrap_err()
        };
        for (from, to) in [
            ("layer = \"L\"", "layer = \"\""),
            ("measure = \"M\"", "measure = \"\""),
            ("route = \"r\"", "route = \"\""),
            ("tool = \"t\"", "tool = \"\""),
            ("is = \"a\"", "is = \"a\"\nnote = \" \""),
        ] {
            assert!(swap(from, to).contains("empty string"), "{to}");
        }
    }

    #[test]
    fn a_layer_s_rows_sit_together_once_each() {
        let row = |layer: &str, measure: &str| {
            format!(
                "[[row]]\nlayer = \"{layer}\"\nmeasure = \"{measure}\"\nroute = \"r\"\n{BOTH}\n\
                 [row.hacker_fab]\nvalue = 1\n{SIDE}\n[row.industry]\nvalue = 2\n{SIDE}\n"
            )
        };
        let src = |rows: &[(&str, &str)]| Source {
            path: STACK.into(),
            text: rows.iter().map(|(l, m)| row(l, m)).collect(),
        };
        parse(&src(&[("A", "x"), ("A", "y"), ("B", "x")])).unwrap();
        let e = parse(&src(&[("A", "x"), ("B", "x"), ("A", "y")])).unwrap_err();
        assert!(e.contains("apart from the rest"), "{e}");
        let e = parse(&src(&[("A", "x"), ("A", "x")])).unwrap_err();
        assert!(e.contains("given twice"), "{e}");
        let e = parse(&Source {
            path: STACK.into(),
            text: "row = []\n".into(),
        })
        .unwrap_err();
        assert!(e.contains("no rows"), "{e}");
    }

    #[test]
    fn a_fab_row_keeps_its_shape() {
        let extra = |at: &str| {
            let mut f = file(BOTH, "value = 1", "value = 2");
            f.text = f.text.replacen(at, &format!("{at}extra = 1\n"), 1);
            let e = parse(&f).unwrap_err();
            assert!(e.contains("unknown field `extra`"), "{at}: {e}");
        };
        extra("[[row]]\n");
        extra("[row.hacker_fab]\n");
        extra("[row.industry]\n");
        let mut f = file(BOTH, "value = 1", "value = 2");
        f.text = format!("extra = 1\n{}", f.text);
        assert!(parse(&f).unwrap_err().contains("unknown field `extra`"));
    }

    #[test]
    fn the_stack_in_the_repo_parses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        parse(&read(&root).unwrap()).unwrap();
    }
}
