//! The Pico supply enforcer's limit logic: what it lets through to the DPS5005, and when it opens
//! the output (`docs/vision.md`, "The bench"). There is no hardware and no OS here. The firmware on
//! Tock is to give it every input, send the DPS5005 what it returns, and set the output switch
//! from [`Enforcer::switch_closed`] after each input.
//!
//! The limits are `lab/limits.toml`'s, compiled in by the build script through `crates/lab` alone
//! ([`LIMITS`]), so nothing at run time can raise them. The rules, for any limits:
//!
//! - A setpoint or current limit above `[supply.dps]`'s is refused, not clamped. A refused input
//!   changes nothing.
//! - A reading above `[supply]`'s ceilings, a failed read, or a tick with no reading since the tick
//!   before trips the output, whatever state it is in.
//! - The output turns on only by `On`, from off, once a setpoint has been let through.
//! - A trip ends only by the button or, under `reenable = "agent"`, the host's `Clear`, and either
//!   leaves the output off.
//!
//! Kani checks each rule for any limits, one input at a time from any state, reachable or not, so
//! the rule holds after every sequence of inputs (the `proofs` module). The tests hold the rules
//! on examples.

#![cfg_attr(not(test), no_std)]

include!(concat!(env!("OUT_DIR"), "/limits.rs"));

/// After a trip, what may end it: `reenable` in `lab/limits.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Reenable {
    /// Only the button on the enforcer.
    Person,
    /// The host's [`Input::Clear`] too.
    Agent,
}

/// The limits the enforcer holds, in whole millivolts and milliamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Limits {
    /// `supply.max_volts`: a reading above it trips the output.
    pub max_millivolts: u32,
    /// `supply.max_amps`: a reading above it trips the output.
    pub max_milliamps: u32,
    /// `supply.dps.max_setpoint_volts`: a setpoint above it is refused.
    pub max_setpoint_millivolts: u32,
    /// `supply.dps.current_limit_amps`: a current limit above it is refused.
    pub max_current_limit_milliamps: u32,
    pub reenable: Reenable,
}

/// Everything that reaches the enforcer, one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Input {
    /// From the host: a voltage setpoint and a current limit for the DPS5005.
    Set { millivolts: u32, milliamps: u32 },
    /// From the host: close the output switch.
    On,
    /// From the host: open it.
    Off,
    /// From the host: end a trip, which only [`Reenable::Agent`] allows.
    Clear,
    /// The enforcer's own reading of the output.
    Reading { millivolts: u32, milliamps: u32 },
    /// A reading that failed.
    ReadFailed,
    /// The firmware's timer. A reading must have arrived since the tick before.
    Tick,
    /// The button on the enforcer.
    Button,
}

/// Why the output tripped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Trip {
    OverVoltage,
    OverCurrent,
    ReadFailed,
    NoReading,
}

/// The output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Output {
    Off,
    On,
    /// Off until the trip ends. It keeps the trip's first cause.
    Tripped(Trip),
}

/// What the firmware is to send the DPS5005 after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Send {
    Nothing,
    Setpoint { millivolts: u32, milliamps: u32 },
}

/// Why an input was refused. A refused input changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// A setpoint or current limit above the limits.
    AboveLimit,
    /// `On` before any setpoint was let through.
    NoSetpoint,
    /// `On` while tripped.
    Tripped,
    /// `Clear` under [`Reenable::Person`].
    OnlyThePersonClears,
}

/// The output, the setpoint last let through, and whether a reading has arrived since the last
/// tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Enforcer {
    limits: Limits,
    output: Output,
    setpoint: Option<(u32, u32)>,
    read_since_tick: bool,
}

impl Enforcer {
    /// Off, with no setpoint and no reading yet, so a tick before the first reading trips it.
    pub const fn new(limits: Limits) -> Self {
        Enforcer {
            limits,
            output: Output::Off,
            setpoint: None,
            read_since_tick: false,
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    pub fn output(&self) -> Output {
        self.output
    }

    /// Whether the output switch is to be closed.
    pub fn switch_closed(&self) -> bool {
        self.output == Output::On
    }

    /// Takes one input, and returns what to send the DPS5005 or why the input was refused.
    pub fn step(&mut self, input: Input) -> Result<Send, Refused> {
        let l = self.limits;
        match input {
            Input::Set {
                millivolts,
                milliamps,
            } => {
                if millivolts > l.max_setpoint_millivolts
                    || milliamps > l.max_current_limit_milliamps
                {
                    return Err(Refused::AboveLimit);
                }
                self.setpoint = Some((millivolts, milliamps));
                return Ok(Send::Setpoint {
                    millivolts,
                    milliamps,
                });
            }
            Input::On => match self.output {
                Output::Tripped(_) => return Err(Refused::Tripped),
                _ if self.setpoint.is_none() => return Err(Refused::NoSetpoint),
                _ => self.output = Output::On,
            },
            Input::Off => {
                if self.output == Output::On {
                    self.output = Output::Off;
                }
            }
            Input::Clear => {
                if let Output::Tripped(_) = self.output {
                    if l.reenable == Reenable::Person {
                        return Err(Refused::OnlyThePersonClears);
                    }
                    self.output = Output::Off;
                }
            }
            Input::Button => {
                if let Output::Tripped(_) = self.output {
                    self.output = Output::Off;
                }
            }
            Input::Reading {
                millivolts,
                milliamps,
            } => {
                self.read_since_tick = true;
                if millivolts > l.max_millivolts {
                    self.trip(Trip::OverVoltage);
                } else if milliamps > l.max_milliamps {
                    self.trip(Trip::OverCurrent);
                }
            }
            Input::ReadFailed => self.trip(Trip::ReadFailed),
            Input::Tick => {
                if !self.read_since_tick {
                    self.trip(Trip::NoReading);
                }
                self.read_since_tick = false;
            }
        }
        Ok(Send::Nothing)
    }

    fn trip(&mut self, why: Trip) {
        if !matches!(self.output, Output::Tripped(_)) {
            self.output = Output::Tripped(why);
        }
    }
}

/// Kani's harnesses: each rule, for any limits and any input, from any state.
#[cfg(kani)]
mod proofs {
    use super::*;

    /// The enforcer in any state, reachable or not, given any input, with `holds` checked against
    /// the state before, the input, the result and the state after. A rule that holds for one
    /// input from any state holds after every sequence of inputs.
    fn one_step(holds: impl Fn(&Enforcer, Input, Result<Send, Refused>, &Enforcer)) {
        let before: Enforcer = kani::any();
        let input: Input = kani::any();
        let mut after = before;
        let result = after.step(input);
        holds(&before, input, result, &after);
    }

    #[kani::proof]
    fn nothing_above_the_limits_reaches_the_dps() {
        one_step(|before, input, result, _| {
            let l = before.limits;
            if let Ok(Send::Setpoint {
                millivolts,
                milliamps,
            }) = result
            {
                assert!(millivolts <= l.max_setpoint_millivolts);
                assert!(milliamps <= l.max_current_limit_milliamps);
            }
            // Refused, not clamped: a setpoint above the limits sends nothing, and one within
            // them is sent as asked.
            if let Input::Set {
                millivolts,
                milliamps,
            } = input
            {
                if millivolts > l.max_setpoint_millivolts
                    || milliamps > l.max_current_limit_milliamps
                {
                    assert!(result == Err(Refused::AboveLimit));
                } else {
                    assert!(
                        result
                            == Ok(Send::Setpoint {
                                millivolts,
                                milliamps
                            })
                    );
                }
            }
        });
    }

    #[kani::proof]
    fn a_refused_input_changes_nothing() {
        one_step(|before, _, result, after| {
            if result.is_err() {
                assert!(before == after);
            }
        });
    }

    #[kani::proof]
    fn a_bad_reading_or_a_silent_tick_trips_the_output_at_once() {
        one_step(|before, input, _, after| {
            let l = before.limits;
            let trips = match input {
                Input::Reading {
                    millivolts,
                    milliamps,
                } => millivolts > l.max_millivolts || milliamps > l.max_milliamps,
                Input::ReadFailed => true,
                Input::Tick => !before.read_since_tick,
                _ => false,
            };
            if trips {
                assert!(matches!(after.output, Output::Tripped(_)));
                assert!(!after.switch_closed());
            }
        });
    }

    #[kani::proof]
    fn the_output_turns_on_only_by_on_from_off_with_a_setpoint() {
        one_step(|before, input, _, after| {
            if after.switch_closed() && !before.switch_closed() {
                assert!(input == Input::On);
                assert!(before.output == Output::Off);
                assert!(before.setpoint.is_some());
            }
        });
    }

    #[kani::proof]
    fn only_the_button_or_an_agent_s_clear_ends_a_trip() {
        one_step(|before, input, _, after| {
            let was = matches!(before.output, Output::Tripped(_));
            let is = matches!(after.output, Output::Tripped(_));
            if was && !is {
                let agent = before.limits.reenable == Reenable::Agent;
                assert!(input == Input::Button || (input == Input::Clear && agent));
                assert!(after.output == Output::Off);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::path::Path;

    /// Limits for the examples, with the setpoint's limits below the ceilings so that each shows.
    const L: Limits = Limits {
        max_millivolts: 3400,
        max_milliamps: 200,
        max_setpoint_millivolts: 3300,
        max_current_limit_milliamps: 150,
        reenable: Reenable::Agent,
    };

    const PERSON: Limits = Limits {
        reenable: Reenable::Person,
        ..L
    };

    fn reading(millivolts: u32, milliamps: u32) -> Input {
        Input::Reading {
            millivolts,
            milliamps,
        }
    }

    /// An enforcer with a setpoint, a reading, and its output on.
    fn on(limits: Limits) -> Enforcer {
        let mut e = Enforcer::new(limits);
        let set = Input::Set {
            millivolts: 3300,
            milliamps: 150,
        };
        e.step(set).unwrap();
        e.step(reading(3300, 10)).unwrap();
        e.step(Input::On).unwrap();
        assert!(e.switch_closed());
        e
    }

    /// `step`, which must refuse `input` for `why` and change nothing.
    fn refused(e: &mut Enforcer, input: Input, why: Refused) {
        let before = *e;
        assert_eq!(e.step(input), Err(why), "{input:?}");
        assert_eq!(*e, before);
    }

    #[test]
    fn the_enforcer_compiles_in_the_limits_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (limits, text) = tentzhen_lab::Limits::load(&root).unwrap();
        let m = limits.supply.millis().unwrap();
        let reenable = match m.reenable {
            tentzhen_lab::Reenable::Person => Reenable::Person,
            tentzhen_lab::Reenable::Agent => Reenable::Agent,
        };
        assert_eq!(
            LIMITS,
            Limits {
                max_millivolts: m.max_millivolts,
                max_milliamps: m.max_milliamps,
                max_setpoint_millivolts: m.max_setpoint_millivolts,
                max_current_limit_milliamps: m.max_current_limit_milliamps,
                reenable,
            }
        );
        let sha256: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(LIMITS_SHA256, sha256);
    }

    /// Kani runs only a function marked `#[kani::proof]`, so a harness that lost its mark would
    /// stop running and nothing would fail. Every function in `proofs` but `one_step` has it.
    #[test]
    fn every_function_in_proofs_is_a_kani_harness() {
        let (_, proofs) = include_str!("lib.rs").split_once("\nmod proofs {").unwrap();
        let (proofs, _) = proofs.split_once("\n}\n").unwrap();
        let lines: Vec<&str> = proofs.lines().map(str::trim).collect();
        let mut harnesses = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("fn ") && !line.starts_with("fn one_step(") {
                assert_eq!(lines[i - 1], "#[kani::proof]", "{line}");
                harnesses += 1;
            }
        }
        assert_eq!(harnesses, 5);
    }

    #[test]
    fn a_setpoint_above_the_limits_is_refused_not_clamped() {
        let mut e = Enforcer::new(L);
        let at = Input::Set {
            millivolts: 3300,
            milliamps: 150,
        };
        let sent = Send::Setpoint {
            millivolts: 3300,
            milliamps: 150,
        };
        assert_eq!(e.step(at), Ok(sent));
        for over in [(3301, 150), (3300, 151), (u32::MAX, 0)] {
            let set = Input::Set {
                millivolts: over.0,
                milliamps: over.1,
            };
            refused(&mut e, set, Refused::AboveLimit);
        }
        assert_eq!(e.setpoint, Some((3300, 150)));
    }

    #[test]
    fn a_bad_reading_or_a_silent_tick_trips_the_output() {
        let cases = [
            (reading(3401, 0), Trip::OverVoltage),
            (reading(0, 201), Trip::OverCurrent),
            (reading(3401, 201), Trip::OverVoltage),
            (Input::ReadFailed, Trip::ReadFailed),
        ];
        for (input, why) in cases {
            let mut e = on(L);
            e.step(input).unwrap();
            assert_eq!(e.output(), Output::Tripped(why), "{input:?}");
            assert!(!e.switch_closed());
            // From off too.
            let mut e = Enforcer::new(L);
            e.step(input).unwrap();
            assert_eq!(e.output(), Output::Tripped(why), "{input:?}");
        }
        // At the ceilings is not over them.
        let mut e = on(L);
        e.step(reading(3400, 200)).unwrap();
        assert!(e.switch_closed());
        // A tick needs a reading since the tick before.
        e.step(Input::Tick).unwrap();
        assert!(e.switch_closed());
        e.step(Input::Tick).unwrap();
        assert_eq!(e.output(), Output::Tripped(Trip::NoReading));
        // And the first tick needs a reading since power-up.
        let mut e = Enforcer::new(L);
        e.step(Input::Tick).unwrap();
        assert_eq!(e.output(), Output::Tripped(Trip::NoReading));
    }

    #[test]
    fn a_trip_keeps_its_first_cause() {
        let mut e = on(L);
        e.step(Input::ReadFailed).unwrap();
        e.step(reading(5000, 0)).unwrap();
        e.step(Input::Tick).unwrap();
        e.step(Input::Tick).unwrap();
        assert_eq!(e.output(), Output::Tripped(Trip::ReadFailed));
    }

    #[test]
    fn the_output_turns_on_only_from_off_with_a_setpoint() {
        let mut e = Enforcer::new(L);
        refused(&mut e, Input::On, Refused::NoSetpoint);
        let mut e = on(L);
        e.step(Input::Off).unwrap();
        assert_eq!(e.output(), Output::Off);
        e.step(Input::On).unwrap();
        assert!(e.switch_closed());
        e.step(Input::ReadFailed).unwrap();
        refused(&mut e, Input::On, Refused::Tripped);
    }

    #[test]
    fn under_person_only_the_button_ends_a_trip() {
        let mut e = on(PERSON);
        e.step(reading(3401, 0)).unwrap();
        refused(&mut e, Input::Clear, Refused::OnlyThePersonClears);
        refused(&mut e, Input::On, Refused::Tripped);
        e.step(Input::Off).unwrap();
        e.step(reading(0, 0)).unwrap();
        e.step(Input::Tick).unwrap();
        assert_eq!(e.output(), Output::Tripped(Trip::OverVoltage));
        e.step(Input::Button).unwrap();
        assert_eq!(e.output(), Output::Off);
        e.step(Input::On).unwrap();
        assert!(e.switch_closed());
    }

    #[test]
    fn under_agent_a_clear_ends_a_trip_and_leaves_the_output_off() {
        for end in [Input::Clear, Input::Button] {
            let mut e = on(L);
            e.step(reading(0, 201)).unwrap();
            e.step(end).unwrap();
            assert_eq!(e.output(), Output::Off, "{end:?}");
            e.step(Input::On).unwrap();
            assert!(e.switch_closed());
        }
    }
}
