//! The lab's safety records. For now that is `lab/limits.toml`, what the hardware below any agent
//! will allow. [`Limits::parse`] refuses a key it does not know, a value that is not a finite
//! number above zero, a DPS5005 setting outside the supply's ceiling or its input rule, and a fuse
//! that is not the next rating above `max_amps`. The tests also check what the file's comments say
//! about its own values.

use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;

/// Where the limits live, relative to the repo root.
pub const LIMITS: &str = "lab/limits.toml";

/// The DPS5005's input range, and the rule that its input be at least 1.1 x its output (Joy-IT
/// manual and datasheet). `trusted`: docs/verification.md, "Trusted base".
const DPS_MIN_INPUT_VOLTS: f64 = 6.0;
const DPS_MAX_INPUT_VOLTS: f64 = 55.0;
const DPS_INPUT_RATIO: f64 = 1.1;

/// The rated currents, in amps, of a quick-acting 5 x 20 mm fuse to IEC 60127-2, as one maker
/// offers them (Schurter FSF 5x20 datasheet, "Variants", read 2026-09-29).
const FUSE_5X20_AMPS: [f64; 26] = [
    0.032, 0.04, 0.05, 0.063, 0.08, 0.1, 0.125, 0.16, 0.2, 0.25, 0.315, 0.4, 0.5, 0.63, 0.8, 1.0,
    1.25, 1.6, 2.0, 2.5, 3.15, 4.0, 5.0, 6.3, 8.0, 10.0,
];

macro_rules! quantity {
    ($name:ident, $unit:literal, $symbol:literal) => {
        #[doc = concat!("A value in ", $unit, ": always finite and above zero.")]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize)]
        #[serde(try_from = "f64")]
        pub struct $name(f64);

        impl $name {
            pub fn get(self) -> f64 {
                self.0
            }
        }

        impl TryFrom<f64> for $name {
            type Error = String;

            fn try_from(v: f64) -> Result<Self, String> {
                if v.is_finite() && v > 0.0 {
                    Ok(Self(v))
                } else {
                    Err(format!(
                        "{} must be a finite number above 0, not {v}",
                        $unit
                    ))
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{} {}", self.0, $symbol)
            }
        }
    };
}

quantity!(Volts, "volts", "V");
quantity!(Amps, "amps", "A");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub pico_3v3: Ceiling,
    pub supply: Supply,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ceiling {
    pub max_volts: Volts,
    pub max_amps: Amps,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    pub max_volts: Volts,
    pub max_amps: Amps,
    pub reenable: Reenable,
    pub upstream: Upstream,
    pub dps: Dps,
    pub fuse: Fuse,
}

/// After a trip, who may turn the output back on.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Reenable {
    Person,
    Agent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upstream {
    pub volts: Volts,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dps {
    pub max_setpoint_volts: Volts,
    pub current_limit_amps: Amps,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuse {
    pub size: FuseSize,
    pub rating_amps: Amps,
    pub speed: FuseSpeed,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum FuseSize {
    #[serde(rename = "5x20 mm")]
    Mm5x20,
}

/// Quick-acting only: the file asks for it, and the ratings above are that series'.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FuseSpeed {
    Fast,
}

impl Supply {
    /// The least the DPS5005's input may be for its highest setpoint.
    pub fn dps_input_floor(&self) -> f64 {
        DPS_MIN_INPUT_VOLTS.max(DPS_INPUT_RATIO * self.dps.max_setpoint_volts.get())
    }

    /// The fuse rating the file asks for: the next one above `max_amps`.
    pub fn fuse_wanted(&self) -> Option<f64> {
        FUSE_5X20_AMPS
            .into_iter()
            .find(|&r| r > self.max_amps.get())
    }
}

impl Limits {
    /// Parses the limits and checks them. The error names the rule a value breaks.
    pub fn parse(text: &str) -> Result<Self, String> {
        let limits: Limits = toml::from_str(text).map_err(|e| e.to_string())?;
        limits.check()?;
        Ok(limits)
    }

    /// Reads and checks [`LIMITS`] under `root`, returning the limits and the text they came from.
    pub fn load(root: &Path) -> Result<(Self, String), String> {
        let text = fs::read_to_string(root.join(LIMITS)).map_err(|e| format!("{LIMITS}: {e}"))?;
        let limits = Self::parse(&text).map_err(|e| format!("{LIMITS}: {e}"))?;
        Ok((limits, text))
    }

    fn check(&self) -> Result<(), String> {
        let s = &self.supply;
        if s.dps.max_setpoint_volts > s.max_volts {
            return Err(format!(
                "[supply.dps] max_setpoint_volts is {}, above [supply] max_volts {}",
                s.dps.max_setpoint_volts, s.max_volts
            ));
        }
        if s.dps.current_limit_amps > s.max_amps {
            return Err(format!(
                "[supply.dps] current_limit_amps is {}, above [supply] max_amps {}",
                s.dps.current_limit_amps, s.max_amps
            ));
        }
        let (volts, floor) = (s.upstream.volts.get(), s.dps_input_floor());
        if volts < floor {
            return Err(format!(
                "[supply.upstream] volts is {volts} V, below the {floor} V the DPS5005 needs to \
                 put out {} (at least {DPS_MIN_INPUT_VOLTS} V, and {DPS_INPUT_RATIO} x its output)",
                s.dps.max_setpoint_volts
            ));
        }
        if volts > DPS_MAX_INPUT_VOLTS {
            return Err(format!(
                "[supply.upstream] volts is {volts} V, above the DPS5005's maximum input of \
                 {DPS_MAX_INPUT_VOLTS} V"
            ));
        }
        match s.fuse_wanted() {
            Some(want) if s.fuse.rating_amps.get() == want => Ok(()),
            Some(want) => Err(format!(
                "[supply.fuse] rating_amps is {}; the next 5 x 20 mm rating above max_amps {} is \
                 {want} A",
                s.fuse.rating_amps, s.max_amps
            )),
            None => Err(format!(
                "[supply] max_amps {} is above every 5 x 20 mm fuse rating listed in crates/lab",
                s.max_amps
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        fs::read_to_string(root.join(LIMITS)).unwrap()
    }

    /// The repo's file with `from` changed to `to`, where `from` occurs exactly once.
    fn with(from: &str, to: &str) -> String {
        let text = repo();
        assert_eq!(text.matches(from).count(), 1, "{from:?} is not unique");
        text.replace(from, to)
    }

    fn refused(text: &str, because: &str) {
        let err = Limits::parse(text)
            .err()
            .unwrap_or_else(|| panic!("accepted, but {because}"));
        assert!(err.contains(because), "refused for another reason: {err}");
    }

    #[test]
    fn the_repo_limits_load() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        Limits::load(&root).expect("lab/limits.toml loads");
    }

    // What the file's comments say about its own values.

    #[test]
    fn the_supply_takes_the_pico_ceiling() {
        let l = Limits::parse(&repo()).unwrap();
        assert_eq!(l.supply.max_volts, l.pico_3v3.max_volts);
        assert_eq!(l.supply.max_amps, l.pico_3v3.max_amps);
    }

    #[test]
    fn the_dps_needs_6_volts_in_for_3v6_out() {
        let l = Limits::parse(&repo()).unwrap();
        assert_eq!(l.supply.dps_input_floor(), 6.0);
    }

    #[test]
    fn the_dps_input_is_above_the_ceiling_it_would_have_to_hold() {
        let l = Limits::parse(&repo()).unwrap();
        assert!(l.supply.dps_input_floor() > l.supply.max_volts.get());
    }

    // The rules: each one refuses a file that breaks it.

    #[test]
    fn a_misspelt_key_is_refused() {
        refused(
            &with("current_limit_amps = 0.200", "current_limit_amp = 0.200"),
            "unknown field `current_limit_amp`",
        );
    }

    #[test]
    fn every_table_refuses_a_key_it_does_not_know() {
        refused(&format!("extra = 1\n{}", repo()), "unknown field `extra`");
        for table in [
            "[pico_3v3]\n",
            "[supply]\n",
            "[supply.upstream]\n",
            "[supply.dps]\n",
            "[supply.fuse]\n",
        ] {
            refused(
                &with(table, &format!("{table}extra = 1\n")),
                "unknown field `extra`",
            );
        }
    }

    #[test]
    fn a_misspelt_choice_is_refused() {
        refused(
            &with("reenable = \"agent\"", "reenable = \"agnet\""),
            "agnet",
        );
        refused(&with("speed = \"fast\"", "speed = \"slow\""), "slow");
    }

    #[test]
    fn a_value_must_be_finite_and_above_zero() {
        let key = "max_setpoint_volts = 3.6";
        for bad in ["inf", "nan", "0.0", "-3.6"] {
            refused(
                &with(key, &format!("max_setpoint_volts = {bad}")),
                "must be a finite number above 0",
            );
        }
    }

    #[test]
    fn the_dps_stays_within_the_ceiling() {
        refused(
            &with("max_setpoint_volts = 3.6", "max_setpoint_volts = 3.7"),
            "above [supply] max_volts",
        );
        refused(
            &with("current_limit_amps = 0.200", "current_limit_amps = 0.210"),
            "above [supply] max_amps",
        );
    }

    #[test]
    fn the_dps_gets_the_input_it_needs() {
        refused(&with("volts = 6.5", "volts = 5.9"), "the DPS5005 needs");
        refused(&with("volts = 6.5", "volts = 56"), "maximum input");
    }

    #[test]
    fn the_fuse_is_the_next_rating_up() {
        let key = "rating_amps = 0.250";
        for bad in ["0.200", "0.315", "0.24"] {
            refused(
                &with(key, &format!("rating_amps = {bad}")),
                "the next 5 x 20 mm rating",
            );
        }
        refused(
            &with("max_amps = 0.200\n# After", "max_amps = 20\n# After"),
            "above every 5 x 20 mm fuse rating",
        );
    }
}
