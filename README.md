# Befreak interpreter

[![dependency status](https://deps.rs/repo/github/PartyWumpus/befreak-interpreter/status.svg)](https://deps.rs/repo/github/PartyWumpus/befreak-interpreter)
[![Build Status](https://github.com/PartyWumpus/befreak-interpreter/workflows/CI/badge.svg)](https://github.com/PartyWumpus/befreak-interpreter/actions?workflow=CI)

This is a [befreak](http://tunes.org/~iepos/befreak.html) interpreter (an esoteric language from 2003) written in rust 🦀.

You can try it out at <https://partywumpus.github.io/befreak-interpreter>.

## Building from source

Run `nix develop` to get all the relevant dependencies for any of these steps.

### Running natively locally

Run `cargo run --release` to open it as a native egui app.

### Web Locally

0. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
0. Open `http://127.0.0.1:8080/index.html#dev` in a browser.

### Manual Web Deploy
0. Run `trunk build --release`.
0. Upload the `dist` directory to the hosting site.

## Updating egui

As of 2023, egui is in active development with frequent releases with breaking changes. [eframe_template](https://github.com/emilk/eframe_template/) will be updated in lock-step to always use the latest version of egui.

When updating `egui` and `eframe` it is recommended you do so one version at the time, and read about the changes in [the egui changelog](https://github.com/emilk/egui/blob/master/CHANGELOG.md) and [eframe changelog](https://github.com/emilk/egui/blob/master/crates/eframe/CHANGELOG.md).
