# tentzhen

Host tools for the bench of open, verifiable, low-cost electronics instruments built from generic
parts that is project #1 of [Tentzhen](https://github.com/via-balaena/tentzhen).

From the repo root, one command so far writes an entry to the lab log, `lab/log/`: one hardware
action, chained to the entry before it (`crates/records/src/log.rs` describes the format).

```fish
cargo run -p tentzhen -- log append supply.set --by person:jon --param set_volts=3.3 --result done=true
```

The release on crates.io, 0.0.1, is a placeholder with no functionality.
