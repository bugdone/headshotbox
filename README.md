# Development setup newui
* `nvm use`
* `npm i`
* `npm run dev`

# Development Setup

* `bower install`
* build https://github.com/bugdone/demoinfogo-linux and place it in the headshotbox directory
* `lein run` (requires leiningen 2)

Check out the [wiki](https://github.com/bugdone/headshotbox/wiki) for more info.

## csdemoparser

### Building

Install [rustup](https://rustup.rs/) and run:

```shell
git submodule init
git submodule update
cargo build --release
```

The output binaries are in `target/release`.

### Profile-guided Optimization

Using [PGO][pgo] has a significant impact on the speed of `csdemoparser`, up to a 40% speedup.

```shell
rustup component add llvm-tools-preview
cargo install cargo-pgo
cargo pgo run -- -p csdemoparser <replay.dem>
cargo pgo optimize
```

[pgo]: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
