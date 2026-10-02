//! The lab's sourcing: where each thing its builds need comes from, and how far it has got. It is
//! `lab/sourcing.toml`, one `[[line]]` per thing to buy, named as a build writes it, and the parts
//! page on tentzhen.com shows it beside the parts list:
//!
//! ```toml
//! [[line]]
//! part = "LFE5U-45F"              # the line as a build writes it, word for word: a part
//! form = "..."                    # and its form, or a commodity (and its form, if any)
//! shop = "aliexpress"             # aliexpress, amazon, lcsc or taobao
//! store = "..."                   # the seller, as the listing names it, or its own name
//! item = "1005007788475037"       # the shop's item number (Amazon's ASIN, LCSC's C number)
//! listing_checked = "2026-10-02"  # the UTC day a person checked the listing against the build
//! in_cart = "2026-10-03"          # the UTC day it went in a cart
//! ordered = true                  # ordered, on a day the line does not give
//! arrived = "2026-10-20"          # the UTC day it arrived
//! passed_qa = "2026-10-21-i9-qa"  # the measurement record of the incoming QA it passed
//! ```
//!
//! Until its line reaches a stage, or with no line, a thing is specced: a build needs it. Its
//! status is the last stage it has reached, never written: the warehouse works it out. A stage is
//! the UTC day it was reached, or `true` when the line gives no day; `false` is refused, since a
//! stage not reached is left out. Every stage is optional, since a part may be ordered without a
//! cart or be on hand already, but the days given run in the stages' order, the incoming QA's
//! being its record's day, and a stage from `listing_checked` to `ordered` names the shop and the
//! item. A status is the whole line's: one of two modules ordered cannot be told from both. Prices
//! stay out, as everything else from a listing does (CLAUDE.md).
//!
//! A thing need not come from a shop. A seller with no listing in the file (a store in town, a
//! thrift store, a person) is a `store` with no `shop`. `used = true` marks a thing bought
//! secondhand, and `salvaged_from` names the device a part was taken out of, with the `store` the
//! device came from, if any, but no shop: an item number would be the device's. A thing already at
//! hand, wherever it came from, is a line with `arrived = true` and no source. `used = false` is
//! refused, as is `used` beside `salvaged_from`: a salvaged part is used already.
//!
//! `authorized` says where the maker lists the seller as one of its authorized distributors, or
//! that the maker sold it itself, for a line that names a seller, new: `authorized =
//! "raspberrypi.com approved resellers, read 2026-10-02"`. A line for a part sold by authorized sellers only (`authorized_only`) needs it
//! once it names a source or reaches a stage. So does a line for a part whose claims rest on
//! `maker-datasheets`, since that entry assumes an authorized distributor, unless the part's
//! claims move to `resold-parts`. A line naming the part alone is not held to it: nothing is
//! chosen.
//!
//! An AliExpress item number is the one aliexpress.com shows. On 2026-10-01 the i9's listing was
//! item 1005007788475037 there and 3256807602160285 on aliexpress.us, 2^51 more, so a number of
//! 2^51 or more is refused rather than taken for the other. Whether every aliexpress.us number is
//! 2^51 more is unknown.
//!
//! A line names a line some build version writes. That it is on the parts list, which counts the
//! newest versions only, is held by the warehouse (`every_sourcing_line_is_on_the_parts_list`).
//! Not held by anything: that a day is the day it happened, or that a part which passed incoming
//! QA passed it; each field is its writer's word. For a commodity, nothing checks what its QA
//! record measured, since a record names its devices as parts or builds. Nor does anything check
//! that a part sold by authorized sellers only is bought from one: the parts page says so beside
//! its source.

use crate::log::real_day;
use crate::{Catalogue, Source, read_source};
use serde::Deserialize;
use std::path::Path;

/// Where the lab's sourcing lives, relative to the repo root.
pub const SOURCING: &str = "lab/sourcing.toml";

/// The trusted-base entry for datasheet values of a part bought from an authorized distributor.
pub const MAKER_DATASHEETS: &str = "maker-datasheets";

/// The trusted-base entry for datasheet values of a part bought from anyone else.
pub const RESOLD_PARTS: &str = "resold-parts";

/// Where a thing is bought.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Shop {
    Aliexpress,
    Amazon,
    Lcsc,
    Taobao,
}

impl Shop {
    pub fn as_str(self) -> &'static str {
        match self {
            Shop::Aliexpress => "aliexpress",
            Shop::Amazon => "amazon",
            Shop::Lcsc => "lcsc",
            Shop::Taobao => "taobao",
        }
    }

    /// Holds `item` to the shop's item numbers, or says what one is.
    fn check_item(self, item: &str) -> Result<(), String> {
        let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        match self {
            Shop::Aliexpress if !digits(item) => Err("digits".into()),
            Shop::Aliexpress if !item.parse::<u64>().is_ok_and(|n| n < 1 << 51) => Err(
                "the number aliexpress.com shows, under 2^51: an aliexpress.us number was 2^51 \
                 more, in the one listing compared"
                    .into(),
            ),
            Shop::Amazon
                if item.len() != 10
                    || !item
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) =>
            {
                Err("an ASIN: ten capital letters and digits".into())
            }
            Shop::Lcsc if !item.strip_prefix('C').is_some_and(digits) => {
                Err("a C number: C, then digits".into())
            }
            Shop::Taobao if !digits(item) => Err("digits".into()),
            _ => Ok(()),
        }
    }
}

/// A stage a line has reached: on the UTC day it gives, or on a day it does not give (`true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    On(String),
    Reached,
}

impl Stage {
    /// The day the line gives for it, if any.
    pub fn day(&self) -> Option<&str> {
        match self {
            Stage::On(day) => Some(day),
            Stage::Reached => None,
        }
    }
}

impl<'de> Deserialize<'de> for Stage {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visit;
        impl serde::de::Visitor<'_> for Visit {
            type Value = Stage;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a UTC day, like \"2026-10-14\", or true for a day not given")
            }
            fn visit_bool<E: serde::de::Error>(self, b: bool) -> Result<Stage, E> {
                if b {
                    Ok(Stage::Reached)
                } else {
                    Err(E::custom("false: leave a stage out until it is reached"))
                }
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Stage, E> {
                Ok(Stage::On(s.into()))
            }
        }
        d.deserialize_any(Visit)
    }
}

/// One thing to buy, and how far it has got.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    /// The line as a build writes it: a part and its form, or a commodity and its form.
    pub part: Option<String>,
    pub form: Option<String>,
    pub commodity: Option<String>,
    pub shop: Option<Shop>,
    /// The seller, as the listing names it, or, with no shop, its own name.
    pub store: Option<String>,
    /// The shop's item number: on Amazon the ASIN, on LCSC the C number.
    pub item: Option<String>,
    /// Bought secondhand. Written as `true`, or left out.
    #[serde(default, deserialize_with = "only_true")]
    pub used: bool,
    /// The device a part was taken out of.
    pub salvaged_from: Option<String>,
    /// Where the maker lists the seller as an authorized distributor, or that the maker sold it
    /// itself; and when, and by whose word.
    pub authorized: Option<String>,
    /// A person checked the listing against the build.
    pub listing_checked: Option<Stage>,
    /// It went in a cart.
    pub in_cart: Option<Stage>,
    /// It was ordered.
    pub ordered: Option<Stage>,
    /// It arrived.
    pub arrived: Option<Stage>,
    /// The id of the measurement record of the incoming QA it passed.
    pub passed_qa: Option<String>,
    /// The file it came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
}

/// A flag written only when it holds: `true`, or left out.
fn only_true<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    if bool::deserialize(d)? {
        Ok(true)
    } else {
        Err(serde::de::Error::custom(
            "false: leave the flag out instead",
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    line: Vec<Line>,
}

impl Line {
    /// How a message names it: `part DPS5005 (running OpenDPS)`, or `commodity "wire"`.
    pub fn named(&self) -> String {
        let what = match (&self.part, &self.commodity) {
            (Some(p), _) => format!("part {p}"),
            (None, Some(c)) => format!("commodity {c:?}"),
            (None, None) => "a line".into(),
        };
        match &self.form {
            Some(f) => format!("{what} ({f})"),
            None => what,
        }
    }

    /// Whether a build's line `l` is the thing this line buys.
    pub fn buys(&self, l: &crate::Line) -> bool {
        self.part == l.part && self.form == l.form && self.commodity == l.commodity
    }

    /// The stages before incoming QA, in their order.
    pub fn stages(&self) -> [(&'static str, Option<&Stage>); 4] {
        [
            ("listing_checked", self.listing_checked.as_ref()),
            ("in_cart", self.in_cart.as_ref()),
            ("ordered", self.ordered.as_ref()),
            ("arrived", self.arrived.as_ref()),
        ]
    }

    /// Holds it to `cat`: some build version writes its line, and its `passed_qa` names a
    /// measurement record from on or after each day its stages give, which lists its part among
    /// the devices it measured.
    pub(crate) fn check_in(&self, cat: &Catalogue) -> Result<(), String> {
        let (at, named) = (&self.record, self.named());
        let lines = || cat.builds.values().flatten().flat_map(|b| &b.line);
        if !lines().any(|l| self.buys(l)) {
            let mut forms: Vec<String> = lines()
                .filter(|l| l.part == self.part && l.commodity == self.commodity)
                .map(|l| format!("{:?}", l.form.as_deref().unwrap_or("")))
                .collect();
            forms.sort();
            forms.dedup();
            let hint = if forms.is_empty() {
                String::new()
            } else {
                format!("; builds write it with the form {}", forms.join(" or "))
            };
            return Err(format!(
                "{at}: {named} is no line a build writes: name it as the build does, word for \
                 word{hint}"
            ));
        }
        // A part sold by authorized sellers only, or whose claims assume one, needs `authorized`
        // once a source is named or a stage reached.
        let chosen = self.shop.is_some()
            || self.store.is_some()
            || self.salvaged_from.is_some()
            || self.stages().iter().any(|(_, s)| s.is_some())
            || self.passed_qa.is_some();
        if let Some(part) = self.part.as_ref().and_then(|p| cat.parts.get(p))
            && chosen
            && self.authorized.is_none()
        {
            if part.authorized_only {
                return Err(format!(
                    "{at}: {named} is sold by authorized sellers only: give `authorized`, where the \
                     maker lists this seller"
                ));
            }
            if let Some(c) = part
                .claim
                .iter()
                .find(|c| c.trusted.as_deref() == Some(MAKER_DATASHEETS))
            {
                return Err(format!(
                    "{at}: {named}'s claim {} rests on {MAKER_DATASHEETS}, which assumes an \
                     authorized distributor: give `authorized`, where the maker lists this seller, \
                     or rest the part's claims on {RESOLD_PARTS}",
                    c.id
                ));
            }
        }
        let Some(id) = &self.passed_qa else {
            return Ok(());
        };
        let Some(m) = cat.measurements.get(id) else {
            return Err(format!(
                "{at}: {named} passed the incoming QA of record {id:?}, which is no file in {}",
                crate::measurement::RECORDS
            ));
        };
        if let Some(p) = &self.part
            && !m.device.iter().any(|d| d.subject().as_deref() == Some(p))
        {
            return Err(format!(
                "{at}: {named} passed the incoming QA of record {id}, which does not list {p} \
                 among the devices it measured"
            ));
        }
        // A record's id starts with its UTC day (`measurement`).
        let qa_day = id.get(..10).unwrap_or_default();
        if let Some((stage, day)) = self.stages().into_iter().find_map(|(s, d)| {
            d.and_then(Stage::day)
                .filter(|d| *d > qa_day)
                .map(|d| (s, d))
        }) {
            return Err(format!(
                "{at}: {named} passed incoming QA on {qa_day}, before its {stage}, {day}"
            ));
        }
        Ok(())
    }
}

/// The file under `root`.
pub fn read(root: &Path) -> Result<Source, String> {
    read_source(root, &root.join(SOURCING))
}

/// Parses the file and holds each line to its shape. What each names is checked by
/// [`Catalogue::with_sourcing`].
pub fn parse(s: &Source) -> Result<Vec<Line>, String> {
    let at = s.path.as_str();
    let File { mut line } = toml::from_str(&s.text).map_err(|e| format!("{at}: {e}"))?;
    for (i, l) in line.iter().enumerate() {
        let named = l.named();
        if l.part.is_some() == l.commodity.is_some() {
            return Err(format!(
                "{at}: a line names exactly one of `part` or `commodity`"
            ));
        }
        let strings = [
            &l.part,
            &l.form,
            &l.commodity,
            &l.store,
            &l.item,
            &l.salvaged_from,
            &l.authorized,
            &l.passed_qa,
        ];
        if strings.iter().any(|s| s.as_deref() == Some("")) {
            return Err(format!("{at}: {named} has an empty string"));
        }
        if line[..i]
            .iter()
            .any(|m| m.part == l.part && m.form == l.form && m.commodity == l.commodity)
        {
            return Err(format!("{at}: {named} is given twice"));
        }
        match (l.shop, &l.item) {
            (Some(shop), Some(item)) => shop.check_item(item).map_err(|want| {
                format!(
                    "{at}: {named}'s item {item:?} is no {} item number: give {want}",
                    shop.as_str()
                )
            })?,
            (None, None) => {}
            _ => {
                return Err(format!(
                    "{at}: {named} names a shop and its item number together"
                ));
            }
        }
        if let Some(donor) = &l.salvaged_from {
            if l.shop.is_some() {
                return Err(format!(
                    "{at}: {named} is salvaged from {donor:?}, so it has no listing: give the \
                     store the device came from, if any, and no shop"
                ));
            }
            if l.used {
                return Err(format!(
                    "{at}: {named} is salvaged from {donor:?}, which is used already: leave \
                     `used` out"
                ));
            }
        }
        if l.authorized.is_some() {
            if l.shop.is_none() && l.store.is_none() {
                return Err(format!(
                    "{at}: {named} says where the maker lists its seller, so it names the seller: \
                     give the shop or the store"
                ));
            }
            if l.used || l.salvaged_from.is_some() {
                return Err(format!(
                    "{at}: {named} is used or salvaged, and an authorized distributor sells a \
                     part new: leave `authorized` out"
                ));
            }
        }
        let mut last: Option<(&str, &str)> = None;
        for (stage, reached) in l.stages() {
            let Some(reached) = reached else { continue };
            if stage != "arrived" && l.shop.is_none() {
                return Err(format!(
                    "{at}: {named}'s {stage} needs the shop and the item number"
                ));
            }
            let Some(day) = reached.day() else { continue };
            if !real_day(day) {
                return Err(format!(
                    "{at}: {named}'s {stage} is a UTC day, like 2026-10-14, or true"
                ));
            }
            if let Some((before, then)) = last
                && day < then
            {
                return Err(format!(
                    "{at}: {named}'s {stage}, {day}, comes before its {before}, {then}"
                ));
            }
            last = Some((stage, day));
        }
    }
    for l in &mut line {
        l.record = at.into();
    }
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::{self, Datum, Entry};
    use std::collections::BTreeMap;

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
    }

    const PICO: &str = "part = \"RP2350\"\nform = \"board\"\n";
    const SHOP: &str = "shop = \"aliexpress\"\nstore = \"Maker\"\nitem = \"1005007788475037\"\n";

    fn refused(text: &str, because: &str) {
        let err = parse(&src(SOURCING, text)).err().unwrap_or_default();
        assert!(err.contains(because), "{because}: {err}");
    }

    #[test]
    fn a_sourcing_line_keeps_its_shape() {
        let good = format!(
            "[[line]]\n{PICO}{SHOP}listing_checked = \"2026-10-02\"\nordered = \"2026-10-02\"\n\
             arrived = \"2026-10-20\"\n\n[[line]]\ncommodity = \"wire\"\narrived = \"2026-09-01\"\n"
        );
        let lines = parse(&src(SOURCING, &good)).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].record, SOURCING);
        assert!(parse(&src(SOURCING, "")).unwrap().is_empty());

        refused(
            &format!("[[line]]\nextra = 1\n{PICO}"),
            "unknown field `extra`",
        );
        refused("extra = 1\n", "unknown field `extra`");
        refused("[[line]]\nform = \"board\"\n", "exactly one of");
        refused(
            "[[line]]\npart = \"RP2350\"\ncommodity = \"wire\"\n",
            "exactly one of",
        );
        refused(
            "[[line]]\npart = \"RP2350\"\nform = \"\"\n",
            "has an empty string",
        );
        refused(
            &format!("[[line]]\n{PICO}\n[[line]]\n{PICO}"),
            "given twice",
        );
        refused(
            &format!("[[line]]\n{PICO}shop = \"ebay\"\nitem = \"1\"\n"),
            "unknown variant `ebay`",
        );
        refused(
            &format!("[[line]]\n{PICO}shop = \"lcsc\"\n"),
            "names a shop and its item number together",
        );
        refused(
            &format!("[[line]]\n{PICO}item = \"C1\"\n"),
            "names a shop and its item number together",
        );
        // A seller with no listing, a thing bought used, a part salvaged, a thing on hand.
        let elsewhere = "[[line]]\npart = \"RP2350\"\nform = \"board\"\nstore = \"Micro Center\"\n\
                         arrived = true\n\n[[line]]\ncommodity = \"wire\"\nstore = \"Goodwill\"\n\
                         used = true\n\n[[line]]\ncommodity = \"tape\"\n\
                         salvaged_from = \"a dead inkjet printer\"\nstore = \"Goodwill\"\n\n\
                         [[line]]\ncommodity = \"glue\"\narrived = true\n";
        let lines = parse(&src(SOURCING, elsewhere)).unwrap();
        assert_eq!(lines[0].store.as_deref(), Some("Micro Center"));
        assert!(lines[1].used && !lines[0].used);
        assert_eq!(
            lines[2].salvaged_from.as_deref(),
            Some("a dead inkjet printer")
        );
        refused(
            &format!("[[line]]\n{PICO}used = false\n"),
            "false: leave the flag out instead",
        );
        refused(
            &format!("[[line]]\n{PICO}salvaged_from = \"a printer\"\nused = true\n"),
            "which is used already: leave `used` out",
        );
        refused(
            &format!("[[line]]\n{PICO}{SHOP}salvaged_from = \"a printer\"\n"),
            "so it has no listing",
        );
        refused(
            &format!("[[line]]\n{PICO}salvaged_from = \"\"\n"),
            "has an empty string",
        );
        refused(
            &format!("[[line]]\n{PICO}store = \"Micro Center\"\nordered = true\n"),
            "ordered needs the shop and the item number",
        );
        // Where the maker lists the seller: with a seller, and new.
        let listed = "authorized = \"raspberrypi.com approved resellers, read 2026-10-02\"\n";
        let ok = format!("[[line]]\n{PICO}store = \"Micro Center\"\n{listed}arrived = true\n");
        let lines = parse(&src(SOURCING, &ok)).unwrap();
        assert!(lines[0].authorized.is_some());
        refused(
            &format!("[[line]]\n{PICO}{listed}"),
            "so it names the seller: give the shop or the store",
        );
        refused(
            &format!("[[line]]\n{PICO}store = \"Goodwill\"\nused = true\n{listed}"),
            "an authorized distributor sells a part new",
        );
        refused(
            &format!(
                "[[line]]\n{PICO}store = \"Goodwill\"\nsalvaged_from = \"a printer\"\n{listed}"
            ),
            "an authorized distributor sells a part new",
        );
        refused(
            &format!("[[line]]\n{PICO}store = \"x\"\nauthorized = \"\"\n"),
            "has an empty string",
        );
        for (shop, item, want) in [
            ("aliexpress", "1005-007", "give digits"),
            ("aliexpress", "3256807602160285", "under 2^51"),
            ("aliexpress", "2251799813685248", "under 2^51"),
            ("amazon", "b0abcdefgh", "an ASIN"),
            ("amazon", "B0ABCDEFG", "an ASIN"),
            ("lcsc", "12345", "a C number"),
            ("lcsc", "C", "a C number"),
            ("taobao", "t123", "give digits"),
        ] {
            refused(
                &format!("[[line]]\n{PICO}shop = \"{shop}\"\nitem = \"{item}\"\n"),
                want,
            );
        }
        for (shop, item) in [
            ("aliexpress", "2251799813685247"),
            ("amazon", "B0ABCDEFGH"),
            ("lcsc", "C2040"),
            ("taobao", "123"),
        ] {
            let text = format!("[[line]]\n{PICO}shop = \"{shop}\"\nitem = \"{item}\"\n");
            parse(&src(SOURCING, &text)).unwrap();
        }
        refused(
            &format!("[[line]]\n{PICO}ordered = \"2026-10-02\"\n"),
            "ordered needs the shop and the item number",
        );
        refused(
            &format!("[[line]]\n{PICO}{SHOP}ordered = \"2026-02-30\"\n"),
            "ordered is a UTC day",
        );
        refused(
            &format!(
                "[[line]]\n{PICO}{SHOP}listing_checked = \"2026-10-03\"\narrived = \"2026-10-02\"\n"
            ),
            "arrived, 2026-10-02, comes before its listing_checked, 2026-10-03",
        );

        // A stage with no day: reached, and out of the days' order.
        let undated = format!(
            "[[line]]\n{PICO}{SHOP}listing_checked = \"2026-10-03\"\nordered = true\n\
             arrived = \"2026-10-04\"\n"
        );
        let lines = parse(&src(SOURCING, &undated)).unwrap();
        assert_eq!(lines[0].ordered, Some(Stage::Reached));
        assert_eq!(lines[0].arrived, Some(Stage::On("2026-10-04".into())));
        refused(
            &format!("[[line]]\n{PICO}ordered = true\n"),
            "ordered needs the shop and the item number",
        );
        refused(
            &format!("[[line]]\n{PICO}{SHOP}ordered = false\n"),
            "false: leave a stage out until it is reached",
        );
        refused(
            &format!("[[line]]\n{PICO}{SHOP}ordered = 5\n"),
            "expected a UTC day, like \"2026-10-14\", or true",
        );
    }

    /// A lab log of one entry, on 2026-10-14, as `tentzhen log append` writes it.
    fn lab_log() -> Vec<Source> {
        let e = Entry {
            seq: 1,
            at: "2026-10-14T10:00:00Z".into(),
            by: "person:jon".into(),
            what: "board.look".into(),
            params: BTreeMap::new(),
            result: BTreeMap::from([("seen".into(), Datum::Flag(true))]),
            limits_sha256: "a".repeat(64),
            prev: None,
            sha256: String::new(),
            record: String::new(),
        };
        let line = serde_json::to_string(&e).unwrap();
        vec![src(
            &format!("{}/2026-10-14.jsonl", log::LOG),
            &format!("{line}\n"),
        )]
    }

    /// A probe build of an RP2350 board, wire and three chips: MON, whose claim rests on
    /// `maker-datasheets`; AMP, whose claim rests on `resold-parts`; and SAFE, sold by authorized
    /// sellers only. With a lab log and a record of the RP2350 seen on 2026-10-14, then `sourcing`.
    fn with(sourcing: &str) -> Result<Catalogue, String> {
        let base = src(
            crate::TRUSTED_BASE,
            &format!(
                "[[entry]]\nid = \"entry\"\nassumes = \"x\"\n\n[[entry]]\nid = \"{MAKER_DATASHEETS}\"\n\
                 assumes = \"x\"\n\n[[entry]]\nid = \"{RESOLD_PARTS}\"\nassumes = \"x\"\n"
            ),
        );
        let rp2350 = "part = \"RP2350\"\nkind = \"chip\"\nis = \"microcontroller\"\n";
        let chip = |name: &str, extra: &str| {
            src(
                &format!("parts/{}.toml", name.to_lowercase()),
                &format!("part = \"{name}\"\nkind = \"chip\"\nis = \"x\"\n{extra}"),
            )
        };
        let claim =
            |on: &str| format!("\n[[claim]]\nid = \"a\"\nsays = \"x\"\ntrusted = \"{on}\"\n");
        let probe = "build = \"probe\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n\n\
                     [[line]]\npart = \"RP2350\"\nform = \"board\"\nclass = \"A\"\nqty = 1\n\n\
                     [[line]]\ncommodity = \"wire\"\nclass = \"W\"\nqty = 6\n\n\
                     [[line]]\npart = \"MON\"\nclass = \"U\"\nqty = 1\n\n\
                     [[line]]\npart = \"AMP\"\nclass = \"U\"\nqty = 1\n\n\
                     [[line]]\npart = \"SAFE\"\nclass = \"U\"\nqty = 1\n";
        let cat = Catalogue::from_sources(
            &base,
            &[
                src("parts/rp2350.toml", rp2350),
                chip("MON", &claim(MAKER_DATASHEETS)),
                chip("AMP", &claim(RESOLD_PARTS)),
                chip("SAFE", "authorized_only = true\n"),
            ],
            &[src("builds/probe/v1.toml", probe)],
        )?
        .with_log(lab_log())?;
        let first = cat.log[0].sha256.clone();
        let record = |device: &str| {
            format!(
                "measures = \"x\"\nsetup = \"x\"\nlog = {{ first = \"{first}\", last = \"{first}\" }}\n\n\
                 [[device]]\n{device}\n\n[results]\nseen = true\n"
            )
        };
        cat.with_measurements(vec![
            src(
                "lab/records/2026-10-14-pico-incoming.toml",
                &record("part = \"RP2350\""),
            ),
            src(
                "lab/records/2026-10-14-probe-seen.toml",
                &record("build = \"probe\"\nversion = 1"),
            ),
        ])?
        .with_sourcing(src(SOURCING, sourcing))
    }

    /// A part sold by authorized sellers only, or whose claims assume one, names where the maker
    /// lists its seller once a source is named or a stage reached.
    #[test]
    fn a_part_that_assumes_an_authorized_seller_names_one() {
        let listed = "authorized = \"maker's distributor list, read 2026-10-02\"\n";
        for ok in [
            "[[line]]\npart = \"MON\"\n".to_string(),
            format!("[[line]]\npart = \"MON\"\n{SHOP}{listed}ordered = true\n"),
            format!(
                "[[line]]\npart = \"SAFE\"\nstore = \"Micro Center\"\n{listed}arrived = true\n"
            ),
            format!("[[line]]\npart = \"AMP\"\n{SHOP}ordered = true\n"),
            "[[line]]\npart = \"AMP\"\nstore = \"Goodwill\"\nused = true\n".to_string(),
        ] {
            with(&ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
        }
        let refused = |text: &str, because: &str| {
            let err = with(text).err().unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        let assumes = "claim a rests on maker-datasheets, which assumes an authorized distributor: \
                       give `authorized`, where the maker lists this seller, or rest the part's \
                       claims on resold-parts";
        refused(&format!("[[line]]\npart = \"MON\"\n{SHOP}"), assumes);
        refused("[[line]]\npart = \"MON\"\narrived = true\n", assumes);
        refused(
            "[[line]]\npart = \"MON\"\nsalvaged_from = \"a dead supply\"\n",
            assumes,
        );
        refused(
            "[[line]]\npart = \"SAFE\"\nstore = \"Goodwill\"\n",
            "is sold by authorized sellers only: give `authorized`",
        );
    }

    #[test]
    fn a_sourcing_line_names_a_line_a_build_writes() {
        let cat = with(&format!(
            "[[line]]\n{PICO}{SHOP}arrived = \"2026-10-14\"\npassed_qa = \"2026-10-14-pico-incoming\"\n\n\
             [[line]]\ncommodity = \"wire\"\npassed_qa = \"2026-10-14-probe-seen\"\n"
        ))
        .unwrap();
        assert_eq!(cat.sourcing.len(), 2);
        assert!(cat.sources.iter().any(|s| s.path == SOURCING));

        let refused = |text: &str, because: &str| {
            let err = with(text).err().unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        refused(
            "[[line]]\npart = \"RP2350\"\nform = \"Pico 2\"\n",
            "is no line a build writes: name it as the build does, word for word; builds write \
             it with the form \"board\"",
        );
        refused(
            "[[line]]\npart = \"RP2350\"\n",
            "builds write it with the form \"board\"",
        );
        refused(
            "[[line]]\ncommodity = \"wires\"\n",
            "is no line a build writes",
        );
        refused(
            "[[line]]\ncommodity = \"wire\"\nform = \"red\"\n",
            "is no line a build writes",
        );
        refused(
            &format!("[[line]]\n{PICO}passed_qa = \"2026-10-14-nope\"\n"),
            "the incoming QA of record \"2026-10-14-nope\", which is no file in lab/records",
        );
        refused(
            &format!("[[line]]\n{PICO}passed_qa = \"2026-10-14-probe-seen\"\n"),
            "which does not list RP2350 among the devices it measured",
        );
        refused(
            &format!(
                "[[line]]\n{PICO}{SHOP}arrived = \"2026-10-15\"\npassed_qa = \"2026-10-14-pico-incoming\"\n"
            ),
            "passed incoming QA on 2026-10-14, before its arrived, 2026-10-15",
        );
        // A stage with no day is not held to the record's.
        with(&format!(
            "[[line]]\n{PICO}{SHOP}arrived = true\npassed_qa = \"2026-10-14-pico-incoming\"\n"
        ))
        .unwrap();
    }
}
