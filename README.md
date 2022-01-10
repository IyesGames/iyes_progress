# Bevy Loading State Progress Tracking Helper Crate

[![Crates.io](https://img.shields.io/crates/v/bevy_loading)](https://crates.io/crates/bevy_loading)
[![docs](https://docs.rs/bevy_loading/badge.svg)](https://docs.rs/bevy_loading/)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

This is a plugin for the Bevy game engine, to help you implement loading states.

You might have a whole bunch of different tasks to do during a loading screen, before
transitioning to the in-game state, depending on your game. For example:
  - wait until your assets finish loading
  - generate a world map
  - download something from a server
  - connect to a multiplayer server
  - wait for other players to become ready
  - any number of other things...

This plugin can help you track any such things, generally, and ergonomically.

## Example

See the [example](./examples/full.rs) for an overview of how to use this crate.

## Explanation

To use this plugin, add `LoadingPlugin` to your `App`, configuring it for the relevant app states.

To track assets, load them as normal, and then add their handles to the `AssetsLoading` resource
from this crate.

For other things, implement them as regular Bevy systems that return a `Progress` struct.
The return value indicates the progress of your loading task. You can add such "loading systems"
to your loading state's `on_update`, by wrapping them using the `track` function.

This plugin will check the progress of all tracked systems every frame, and transition to your
next state when all of them report completion.

If you need to access the overall progress information (say, to display a progress bar),
you can get it from the `ProgressCounter` resource.

You can have multiple instances of the plugin for different loading states. For example, you can load your UI
assets for your main menu during a splash screen, and then prepare the game session and assets during
a game loading screen.

## Bevy Compatibility

| Plugin Version | Bevy Version |
|----------------|--------------|
| `0.1`          | `0.5`        |
| `0.2`          | `0.6`        |
| `main`         | `0.6`        |
| `bevy_main`    | `main`       |

