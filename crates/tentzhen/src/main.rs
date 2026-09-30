//! `cargo run -p tentzhen -- log append <what> --by <who> [--param key=value]... --result
//! key=value...`, from the repo root, writes one entry to the lab log, chained to the last:
//! `tentzhen_records::log` says what an entry holds and what the log refuses.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;
use tentzhen_records::log::{self, Datum};

const USAGE: &str = "usage: tentzhen log append <what> --by <person:name | agent:name> \
                     [--param key=value]... --result key=value [--result key=value]...";

/// An action to log, as the command line gives it.
#[derive(Debug, PartialEq)]
struct Action {
    what: String,
    by: String,
    params: BTreeMap<String, Datum>,
    result: BTreeMap<String, Datum>,
}

/// Reads `log append`'s arguments. A value is read as its key says ([`Datum::read`]).
fn action(args: &[String]) -> Result<Action, String> {
    let (what, rest) = args.split_first().ok_or(USAGE)?;
    let mut by = None;
    let (mut params, mut result) = (BTreeMap::new(), BTreeMap::new());
    let mut rest = rest.iter();
    while let Some(option) = rest.next() {
        let value = rest
            .next()
            .ok_or_else(|| format!("{option} needs a value\n{USAGE}"))?;
        let side = match option.as_str() {
            "--by" if by.is_none() => {
                by = Some(value.clone());
                continue;
            }
            "--by" => return Err("--by is given twice".into()),
            "--param" => &mut params,
            "--result" => &mut result,
            _ => return Err(format!("unknown option {option}\n{USAGE}")),
        };
        let (key, v) = value
            .split_once('=')
            .ok_or_else(|| format!("{option} {value}: write it as key=value"))?;
        if side.insert(key.to_string(), Datum::read(key, v)?).is_some() {
            return Err(format!("{option} {key} is given twice"));
        }
    }
    Ok(Action {
        what: what.clone(),
        by: by.ok_or_else(|| format!("say who acted, with --by\n{USAGE}"))?,
        params,
        result,
    })
}

fn append(args: &[String]) -> Result<String, String> {
    let a = action(args)?;
    let e = log::append(
        Path::new("."),
        SystemTime::now(),
        &a.by,
        &a.what,
        a.params,
        a.result,
    )?;
    Ok(format!("{}: entry {} at {}\n", e.record, e.seq, e.at))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let run = match args.as_slice() {
        [log, append_, rest @ ..] if log == "log" && append_ == "append" => append(rest),
        _ => Err(USAGE.to_string()),
    };
    match run {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split(' ').map(String::from).collect()
    }

    #[test]
    fn the_command_line_names_an_action() {
        let a = action(&args(
            "supply.set --by person:jon --param set_volts=3.3 --result done=true --result note=ok",
        ))
        .unwrap();
        assert_eq!(a.what, "supply.set");
        assert_eq!(a.by, "person:jon");
        assert_eq!(a.params["set_volts"], Datum::Number(3.3));
        assert_eq!(a.result["done"], Datum::Flag(true));
        assert_eq!(a.result["note"], Datum::Text("ok".into()));
        for (line, because) in [
            ("supply.set --result done=true", "say who acted"),
            (
                "supply.set --by person:jon --by agent:x",
                "--by is given twice",
            ),
            (
                "supply.set --by person:jon --result done=true --result done=false",
                "--result done is given twice",
            ),
            (
                "supply.set --by person:jon --result done",
                "write it as key=value",
            ),
            (
                "supply.set --by person:jon --result",
                "--result needs a value",
            ),
            (
                "supply.set --by person:jon --force x",
                "unknown option --force",
            ),
            (
                "supply.set --by person:jon --param set_volts=high",
                "is a number",
            ),
        ] {
            let err = action(&args(line)).err().unwrap_or_default();
            assert!(err.contains(because), "{line}: {err}");
        }
        assert!(action(&[]).is_err());
    }
}
