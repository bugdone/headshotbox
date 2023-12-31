# Logging

To activate logging, you must enable the "tracing" cargo feature and set `RUST_LOG`:

```
RUST_LOG=trace cargo run --release --features=tracing -p csdemoparser <demo.dem>
```

You can focus on only one span:

```
RUST_LOG=[handle_game_event]=trace cargo run --release --features=tracing -p csdemoparser <demo.dem>
```

The reference for the RUST_LOG format is available in the
[tracing docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives)
