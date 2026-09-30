//! Compiles lab/limits.toml's supply limits into the enforcer, read through crates/lab alone, with
//! the sha256 of the file they came from.

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::{env, fs};
use tentzhen_lab::{LIMITS, Limits, Reenable};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    println!("cargo::rerun-if-changed={}", root.join(LIMITS).display());
    let (limits, text) = Limits::load(&root).unwrap_or_else(|e| panic!("{e}"));
    let m = limits
        .supply
        .millis()
        .unwrap_or_else(|e| panic!("{LIMITS}: {e}"));
    let reenable = match m.reenable {
        Reenable::Person => "Person",
        Reenable::Agent => "Agent",
    };
    let sha256: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let out = format!(
        "/// `{LIMITS}`'s supply limits, as the build script read them.
pub const LIMITS: Limits = Limits {{
    max_millivolts: {},
    max_milliamps: {},
    max_setpoint_millivolts: {},
    max_current_limit_milliamps: {},
    reenable: Reenable::{reenable},
}};

/// The sha256 of that file, as a lab-log entry's `limits_sha256` gives it.
pub const LIMITS_SHA256: &str = \"{sha256}\";
",
        m.max_millivolts, m.max_milliamps, m.max_setpoint_millivolts, m.max_current_limit_milliamps,
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("limits.rs"), out).unwrap();
}
