//! Bevy Loading State Progress Tracking Helper Crate
//!
//! This is a plugin for the Bevy game engine, to help you implement loading states.
//!
//! You might have a whole bunch of different tasks to do during a loading screen, before
//! transitioning to the in-game state, depending on your game. For example:
//!   - wait until your assets finish loading
//!   - generate a world map
//!   - download something from a server
//!   - connect to a multiplayer server
//!   - wait for other players to become ready
//!   - any number of other things...
//!
//! This plugin can help you track any such things, generally, and ergonomically.
//!
//! To use this plugin, add `LoadingPlugin` to your `App`, configuring it for the relevant app states.
//!
//! For assets, load them as normal, and then add their handles to the `AssetsLoading` resource
//! from this crate.
//!
//! For other things, implement them as regular Bevy systems that return a `Progress` struct.
//! The return value indicates the progress of your loading task. You can add such "loading systems"
//! to your loading state's `on_update`, by wrapping them using the `track` function.
//!
//! This plugin will check the progress of all tracked systems every frame, and transition to your
//! next state when all of them report completion.
//!
//! If you need to access the overall progress information (say, to display a progress bar),
//! you can get it from the `ProgressCounter` resource.
//!
//! You can have multiple instances of the plugin for different loading states. For example, you can load your UI
//! assets for your main menu during a splash screen, and then prepare the game session and assets during
//! a game loading screen.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Add, AddAssign};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering as MemOrdering;

use bevy::ecs::schedule::{ParallelSystemDescriptor, StateData};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;

mod asset;
pub use crate::asset::AssetsLoading;

/// Most used imports from `bevy_loading`
pub mod prelude {
    pub use crate::asset::AssetsLoading;
    pub use crate::track;
    pub use crate::LoadingPlugin;
    pub use crate::Progress;
}

/// Progress reported by a loading system
///
/// Your loading systems must return a value of this type.
/// It indicates how much work that system has still left to do.
///
/// When the value of `done` reaches the value of `total`, the system is considered "ready".
/// When all systems in your loading state are "ready", we will transition to the next application state.
///
/// For your convenience, you can easily convert `bool`s into this type.
/// You can also convert `Progress` values into floats in the 0.0..1.0 range.
#[derive(Debug, Clone, Copy, Default)]
pub struct Progress {
    /// Units of work completed
    pub done: u32,
    /// Total units of work expected
    pub total: u32,
}

impl From<bool> for Progress {
    fn from(b: bool) -> Progress {
        Progress {
            total: 1,
            done: if b { 1 } else { 0 },
        }
    }
}

impl From<Progress> for f32 {
    fn from(p: Progress) -> f32 {
        p.done as f32 / p.total as f32
    }
}

impl From<Progress> for f64 {
    fn from(p: Progress) -> f64 {
        p.done as f64 / p.total as f64
    }
}

impl Progress {
    fn is_ready(self) -> bool {
        self.done >= self.total
    }
}

impl Add for Progress {
    type Output = Progress;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.done += rhs.done;
        self.total += rhs.total;

        self
    }
}

impl AddAssign for Progress {
    fn add_assign(&mut self, rhs: Self) {
        self.done += rhs.done;
        self.total += rhs.total;
    }
}

/// Add this plugin to your app, to use this crate for the specified loading state.
///
/// If you have multiple different loading states, you can add the plugin for each one.
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_loading::LoadingPlugin;
/// # let mut app = App::default();
/// app.add_plugin(LoadingPlugin::new(MyState::GameLoading).continue_to(MyState::InGame));
/// app.add_plugin(LoadingPlugin::new(MyState::Splash).continue_to(MyState::MainMenu));
/// # #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// # enum MyState {
/// #     Splash,
/// #     MainMenu,
/// #     GameLoading,
/// #     InGame,
/// # }
/// ```
pub struct LoadingPlugin<S: StateData> {
    /// The loading state during which progress will be tracked
    pub loading_state: S,
    /// The next state to transition to, when all progress completes
    pub next_state: Option<S>,
}

impl<S: StateData> LoadingPlugin<S> {
    /// Create a [`LoadingPlugin`] running during the given State
    pub fn new(loading_state: S) -> Self {
        LoadingPlugin {
            loading_state,
            next_state: None,
        }
    }

    /// Configure the [`LoadingPlugin`] to move on to the given state as soon as all Progress
    /// in the loading state is completed.
    pub fn continue_to(mut self, next_state: S) -> Self {
        self.next_state = Some(next_state);

        self
    }
}

impl<S: StateData> Plugin for LoadingPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<asset::AssetsLoading>();
        app.add_system_set(
            SystemSet::on_enter(self.loading_state.clone()).with_system(loadstate_enter),
        );
        app.add_system_set(
            SystemSet::on_update(self.loading_state.clone())
                .with_system(
                    next_frame
                        .exclusive_system()
                        .at_start()
                        .label(ProgressTracking::Preparation),
                )
                .with_system(
                    check_progress::<S>(self.next_state.clone())
                        .exclusive_system()
                        .at_end()
                        .label(ProgressTracking::CheckProgress),
                )
                .with_system(track(asset::assets_progress)),
        );
        app.add_system_set(
            SystemSet::on_exit(self.loading_state.clone())
                .with_system(loadstate_exit)
                .with_system(asset::assets_loading_reset),
        );
    }
}

/// Wrap a loading system, to add to your App.
///
/// Add your systems like this:
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_loading::{LoadingPlugin, track, Progress};
/// # let mut app = App::default();
/// # app.add_system_set(
/// SystemSet::on_update(MyState::GameLoading)
///     .with_system(track(my_loading_system))
/// # );
/// # #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// # enum MyState {
/// #     GameLoading,
/// # }
/// # fn my_loading_system()-> Progress {
/// #     Progress::default()
/// # }
/// ```
pub fn track<Params, S: IntoSystem<(), Progress, Params>>(s: S) -> ParallelSystemDescriptor {
    s.chain(
        |In(progress): In<Progress>, counter: Res<ProgressCounter>| {
            counter.manually_track(progress)
        },
    )
    .label(ProgressTracking::Tracking)
}

/// Label to control system execution order
///
/// Use this if you want to schedule systems to run before or after any tracked systems.
///
/// All tracked systems run after `ReadyLabel::Pre` and before `ReadyLabel::Post`.
///
/// If you need the latest progress information (by calling `ProgressCounter::progress`)
/// from the current frame, your system should run *after* `ReadyLabel::Post`. Otherwise,
/// you will get the value from the previous frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
pub enum ProgressTracking {
    /// All systems tracking progress should run after this label
    ///
    /// The label is only needed for `at_start` exclusive systems.
    /// Any parallel system or exclusive system at a different position
    /// will run after it automatically.
    Preparation,
    /// Any system reading progress should ran after this label.
    /// All systems wrapped in [`track`] automatically get this label.
    Tracking,
    /// All systems tracking progress should run before this label
    ///
    /// The label is only needed for `at_end` exclusive systems.
    /// Any parallel system or exclusive system at a different position
    /// will run before it automatically.
    CheckProgress,
}

/// Resource for tracking overall progress
///
/// This resource is automatically created when entering the load state and removed when exiting it.
#[derive(Default)]
pub struct ProgressCounter {
    // use atomics to track overall progress,
    // so that we can avoid mut access in tracked systems,
    // allowing them to run in parallel
    done: AtomicU32,
    total: AtomicU32,
    persisted: Progress,
}

impl ProgressCounter {
    /// Get the latest overall progress information
    ///
    /// This is the combined total of all systems.
    ///
    /// To get correct information, make sure that you call this function only after
    /// all your systems that track progress finished
    pub fn progress(&self) -> Progress {
        let total = self.total.load(MemOrdering::Acquire);
        let done = self.done.load(MemOrdering::Acquire);

        Progress { done, total }
    }

    /// Add some amount of progress to the running total for the current frame.
    ///
    /// In most cases you do not want to call this function yourself.
    /// Let your systems return a [`Progress`] and wrap them in [`track`] instead.
    pub fn manually_track(&self, progress: Progress) {
        self.total.fetch_add(progress.total, MemOrdering::Release);
        // use `min` to clamp in case a bad user provides `done > total`
        self.done
            .fetch_add(progress.done.min(progress.total), MemOrdering::Release);
    }

    /// Persist progress for the rest of the current state
    pub fn persist_progress(&mut self, progress: Progress) {
        self.manually_track(progress);
        self.persisted += progress;
    }
}

fn loadstate_enter(mut commands: Commands) {
    commands.insert_resource(ProgressCounter::default());
}

fn loadstate_exit(mut commands: Commands) {
    commands.remove_resource::<ProgressCounter>();
}

fn check_progress<S: StateData>(next_state: Option<S>) -> impl FnMut(&mut World) {
    move |world| {
        let mut system_state: SystemState<(Res<ProgressCounter>, ResMut<State<S>>)> =
            SystemState::new(world);
        let (counter, mut state) = system_state.get_mut(world);
        let progress = counter.progress();
        if progress.is_ready() {
            if let Some(next_state) = &next_state {
                state.set(next_state.clone()).ok();
            }
        }
    }
}

fn next_frame(world: &mut World) {
    let counter = world.resource::<ProgressCounter>();

    counter
        .done
        .store(counter.persisted.done, MemOrdering::Release);
    counter
        .total
        .store(counter.persisted.total, MemOrdering::Release);
}
