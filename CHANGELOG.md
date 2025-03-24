# Changelog

Notable user-facing changes with each release version will be described in this file.

## [0.14.0-rc.1]: 2025-03-24

### Changed
 - Bevy 0.16.0-rc.1 compatibility.

## [0.13.1]: 2024-12-13

### Added
 - You can now represent progress as entities. Add the new `ProgressEntity<S>` component to your entities.
 - You can now report progress from background tasks/threads using `ProgressSender`. Requires the `"async"` cargo feature.

## [0.13.0]: 2024-12-01

### Fixed
 - It was possible for asset tracking to get stuck if asset was added and immediately
   unloaded. Fix: Drop assets if their status is `NotLoaded`.

### Changed
 - Bevy 0.15 compatibility.

### Added
 - Clearing on state enter/exit is now configurable via the `ProgressPlugin`.
 - `ProgressTracker::for_each_entry` method

## [0.13.0-rc.1]: 2024-11-21

### Changed
 - Bevy 0.15.0-rc.3 compatibility.
 - Most things. :) Major overhaul.
   - See [migration guide](./MIGRATION_012_013.md).

## [0.12.0]: 2024-07-05

### Changed
 - Bevy 0.14 compatibility
 - Asset progress tracking now happens in `PostUpdate`

## [0.11.0]: 2024-02-18

### Changed
 - Bevy 0.13 compatibility

## [0.10.0]: 2023-11-7

### Changed
 - Bevy 0.12 compatibility
 - Assets tracking now accounts for dependencies by default

### Added
 - API for configuring when progress is checked (#25, thanks @UkoeHB)
 - Assets tracking can now be configured to not progress failed assets
 - Assets tracking can now be configured to not check asset dependencies

## [0.9.1]: 2023-07-19

### Fixed
 - Assets tracking: prevent duplicate handles from being added

## [0.9.0]: 2023-07-10

### Changed
 - Bevy 0.9 compatibility
 - `bevy_utils` dependency is now mandatory
 - Progress counting is initialised in a new `ProgressPreparationSchedule`, which runs after `StateTransition`, and finalized in `Last`

### Added
 - Enable the `debug` cargo feature to log progress counts to console! Control at runtime with `Res<ProgressDebug>`.

### Fixed
 - `dummy_system_wait_millis` should now work on WASM

## Older

Unfortunately, I was not keeping changelogs for older versions. :(

[0.14.0-rc.1]: https://github.com/IyesGames/iyes_progress/tree/v0.14.0-rc.1
[0.13.1]: https://github.com/IyesGames/iyes_progress/tree/v0.13.1
[0.13.0]: https://github.com/IyesGames/iyes_progress/tree/v0.13.0
[0.13.0-rc.1]: https://github.com/IyesGames/iyes_progress/tree/v0.13.0-rc.1
[0.12.0]: https://github.com/IyesGames/iyes_progress/tree/v0.12.0
[0.11.0]: https://github.com/IyesGames/iyes_progress/tree/v0.11.0
[0.10.0]: https://github.com/IyesGames/iyes_progress/tree/v0.10.0
[0.9.1]: https://github.com/IyesGames/iyes_progress/tree/v0.9.1
[0.9.0]: https://github.com/IyesGames/iyes_progress/tree/v0.9.0
