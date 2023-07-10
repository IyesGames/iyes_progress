# Changelog

Notable user-facing changes with each release version will be described in this file.

## [0.9.0]: 2023-07-10

## Changed
 - Bevy 0.9 compatibility
 - `bevy_utils` dependency is now mandatory
 - Progress counting is initialised in a new `ProgressPreparationSchedule`, which runs after `StateTransition`, and finalized in `Last`

## Added
 - Enable the `debug` cargo feature to log progress counts to console! Control at runtime with `Res<ProgressDebug>`.

## Fixed
 - `dummy_system_wait_millis` should now work on WASM

## Older

Unfortunately, I was not keeping changelogs for older versions. :(

[0.9.0]: https://github.com/IyesGames/iyes_progress/tree/v0.9.0
