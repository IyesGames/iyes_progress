# Migration Guide for `iyes_progress` 0.12 (Bevy 0.14) -> 0.13 (Bevy 0.15)

`iyes_progress` got a rewrite/redesign with version 0.13 (for Bevy 0.15)
and there are significant changes to the API and semantics/behavior.

## Overview

The main aspects of the redesign are as follows:

 - Global progress does not clear every frame. It is now persistent by default.
   - This plays nice with systems that don't run every frame (such as observers and systems with run conditions).
 - Everything is now tracked per-state-type.
   - This plays nice with complex state hierarchies where you might want progress tracking in multiple states at the same time.
   - Unfortunately, it means you have to specify the state type as a generic everywhere (`::<MyState>`). It's not as bad as it sounds, I promise. ;)
 - `iyes_progress` now takes responsibility for tracking the individual progress of each system (as well as custom entries), not just the overall value.
   - Every system's last reported values are stored and remembered, and the system can modify them whenever it likes.
 - New syntax for managing progress:
   - Systems can now take a special `ProgressEntry<State>` system param, which gives you access to a stored progress value you can update at any time.
   - The old way of returning a `Progress`/`HiddenProgress` value from your system `fn` is still supported.

## Plugin

In 0.12, the `ProgressPlugin` is per-state. You add an instance of the plugin
for every individual state you want to track progress in.

In 0.13, the `ProgressPlugin` is per-state-type. You add one instance of
the plugin for your state type, and then you can configure progress tracking
and state transitions for individual states within it.

```rust
#[derive(States, ...)]
enum MyAppState {
    #[default]
    InitialLoading,
    MainMenu,
    GameLoading,
    InGame,
}

#[derive(States, ...)]
enum MyNetworkState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

// 0.12
app.init_state::<MyState>();
app.add_plugins(
    ProgressPlugin::new(MyAppState::InitialLoading)
        .continue_to(MyAppState::MainMenu)
);
app.add_plugins(
    ProgressPlugin::new(MyAppState::GameLoading)
        .continue_to(MyAppState::InGame)
);
app.add_plugins(
    ProgressPlugin::new(MyNetworkState::Connecting)
        .continue_to(MyNetworkState::Connected)
);

// 0.13
app.init_state::<MyState>();
app.add_plugins(
    ProgressPlugin::<MyState>::new()
        .with_state_transition(MyAppState::InitialLoading, MyAppState::MainMenu)
        .with_state_transition(MyAppState::GameLoading, MyAppState::InGame)
);
app.add_plugins(
    ProgressPlugin::<MyNetworkState>::new()
        .with_state_transition(MyNetworkState::Connecting, MyNetworkState::Connected)
);
```

## Reading the tracked progress

In 0.12, the overall progress was stored in a `ProgressCounter` resource. You
could read the accumulated total progress from there. That resource was
inserted/removed when entering/exiting the state for which the `ProgressPlugin`
was configured.

In 0.13, the `ProgressTracker<State>` resource stores all progress information
for states of the given type. You can read the accumulated total progress
from there. This resource is always available. It should not be removed. Upon
entering a state that was configured in the `ProgressPlugin`, the values
stored inside are cleared.

```rust
// 0.12
fn get_progress(v: Res<ProgressCounter>) {
    let visible_progress = v.progress();
    let all_progress = v.progress_complete();
}

// 0.13
fn get_progress_my_state(v: Res<ProgressTracker<MyState>>) {
    let visible_progress = v.get_global_progress();
    let hidden_progress = v.get_global_hidden_progress();
    let all_progress = v.get_global_combined_progress();
}
fn get_progress_any_state<S: FreelyMutableState>(v: Res<ProgressTracker<S>>) {
    let visible_progress = v.get_global_progress();
    let hidden_progress = v.get_global_hidden_progress();
    let all_progress = v.get_global_combined_progress();
}
```

## Reporting progress

### By returning a value from your system `fn`

In 0.12, the usual way was to return a `Progress` or `HiddenProgress` value
from your systems.

```rust
// 0.12
fn my_system(/* ... */) -> Progress {
    // do stuff ...

    Progress {
        done, total
    }
}
```

And add it as follows:

```rust
// 0.12
app.add_systems(Update, my_system.track_progress());
```

Doing it this way is still supported in 0.13. The syntax for the system `fn`
is the same, but when adding it to the app, you must specify the state type
the progress should count towards:

```rust
// 0.13
app.add_systems(Update, my_system.track_progress::<MyState>());
```

The stored progress is no longer cleared every frame. The previous value you
return is remembered and overwritten the next time your system runs. Thus,
it is now OK to add run conditions to your progress-tracked systems.

We also provide a new method:

```rust
// 0.13
app.add_systems(Update, my_system.track_progress_and_stop::<MyState>());
```

This will add an internal run condition to stop your system from running
any more after it has returned a progress value that indicates completion.

### Via a system param

This is new in 0.13.

You can take a `ProgressEntry<State>` system param, which gives you direct
access to a value stored in the `ProgressTracker<State>`. This can give you
more flexibility, compared to returning progress values.

Each instance of this system param will create its own separate entry
in the [`ProgressTracker`] resource. You can have multiple if you want
to manage multiple progress values from one system.

Note: this is a special system param type. It is not a resource (no need for
`Res`/`ResMut`).

```rust
// 0.13
fn my_system(
    mut entry: ProgressEntry<MyState>,
    // ...
) {
    // Overwrite any previously stored values
    // (this is what the old way of returning `Progress` would do)
    entry.set_progress(7, 20);
    entry.set_hidden_progress(1, 2);

    // You can also set only the done/total individually
    entry.set_done(7);
    entry.set_total(20);
    entry.set_hidden_done(1);
    entry.set_hidden_total(2);

    // For your convenience, there are also methods to add to the
    // existing value.
    entry.add_done(1); // we completed 1 more item, yay!
    entry.add_total(1); // one more item pending to do...
    entry.add_hidden_done(1);
    entry.add_hidden_total(2);
}
```

You can just add these systems normally, no need for any special syntax:

```rust
// 0.13
app.add_systems(Update, my_system);
```

### Not tied to a system

In 0.12, you could add some progress to the global `ProgressCounter` via
the `manually_track` method.

```rust
// 0.12
progress_counter.manually_track(Progress { done, total });
```

In 0.13, you can get an entry in the `ProgressTracker`, where you can record
your progress values (using a `ProgressEntryId`).

This is exactly what the previously-shown APIs do internally, but you can
do it manually, if you want to store progress that is not tied to a specific
system or system param.

Note: `ProgressTracker` does not need `mut` access.

```rust
// 0.13
fn thing(tracker: Res<ProgressTracker<MyState>>) {
    // create a new unique ID
    let entry_id = ProgressEntryId::new();

    // we can now use our ID to manage an entry in the tracker
    tracker.set_progress(entry_id, 7, 11);
    tracker.set_hidden_progress(entry_id, 9, 11);

    // Make sure you store your ID somewhere!
    // If you lose it, you will not be able to update the
    // associated progress values anymore!
}
```

## System Sets

In 0.12, there was the `TrackedProgressSet`, which was automatically assigned
to all systems you add to your app with `.track_progress()`. This made sense,
because returning `Progress` from your `fn` was the expected way of reporting
progress.

In 0.13, there is no such set. There is no way to automatically assign
a system set to systems using the `ProgressEntry` system param or other
methods, so it would not make sense. If you want to have a system set to
identify all your systems that use progress tracking, you will have to make
one yourself manually.

The `CheckProgressSet` (for the system where we check the overall progress and
trigger the state transition) is still available and works the same as before.

## Asset tracking

The general idea is the same. It works the same as before.

But you have to specify the states type as a generic type parameter.

```rust
// 0.12
fn add_assets(mut loading: ResMut<AssetsLoading>) {
    // ...
}

// 0.13
fn add_assets(mut loading: ResMut<AssetsLoading<MyState>>) {
    // ...
}
```
