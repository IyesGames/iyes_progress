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
//!

use std::fmt::Debug;
use std::hash::Hash;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering as MemOrdering;

use bevy::ecs::component::Component;
use bevy::ecs::schedule::ParallelSystemDescriptor;
use bevy::prelude::*;

pub mod asset;

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

/// Add this plugin to your app, to use this crate for the specified loading state.
///
/// If you have multiple different loading states, you can add the plugin for each one.
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_loading::LoadingPlugin;
/// # let mut app = AppBuilder::default();
/// app.add_plugin(LoadingPlugin {
///     loading_state: MyState::GameLoading,
///     next_state: MyState::InGame,
/// });
/// app.add_plugin(LoadingPlugin {
///     loading_state: MyState::Splash,
///     next_state: MyState::MainMenu,
/// });
/// # #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// # enum MyState {
/// #     Splash,
/// #     MainMenu,
/// #     GameLoading,
/// #     InGame,
/// # }
/// ```
pub struct LoadingPlugin<S: BevyState> {
    /// The loading state during which progress will be tracked
    pub loading_state: S,
    /// The next state to transition to, when all progress completes
    pub next_state: S,
}

impl<S: BevyState> Plugin for LoadingPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<asset::AssetsLoading>();
        app.add_system_set(
            SystemSet::on_enter(self.loading_state.clone()).with_system(loadstate_enter.system()),
        );
        app.add_system_set(
            SystemSet::on_update(self.loading_state.clone())
                .with_system(clear_progress.system().label(ReadyLabel::Pre))
                .with_system(
                    check_progress::<S>
                        .system()
                        .config(|(s, _, _)| {
                            *s = Some(Some(self.next_state.clone()));
                        })
                        .label(ReadyLabel::Post),
                )
                .with_system(track(asset::assets_progress.system())),
        );
        app.add_system_set(
            SystemSet::on_exit(self.loading_state.clone())
                .with_system(loadstate_exit.system())
                .with_system(asset::assets_loading_reset.system()),
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
/// # let mut app = AppBuilder::default();
/// # app.add_system_set(
/// SystemSet::on_update(MyState::GameLoading)
///     .with_system(track(my_loading_system.system()))
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
    s.chain(tracker)
        .before(ReadyLabel::Post)
        .after(ReadyLabel::Pre)
        .into()
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
pub enum ReadyLabel {
    Pre,
    Post,
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
    last_progress: Progress,
}

impl ProgressCounter {
    /// Get the latest overall progress information
    ///
    /// This is the combined total of all systems.
    ///
    /// It is updated during `ReadyLabel::Post`.
    /// If your system runs after that label, you will get the value from the current frame update.
    /// If your system runs before that label, you will get the value from the previous frame update.
    pub fn progress(&self) -> Progress {
        self.last_progress
    }

    /// Add some amount of progress to the running total for the current frame.
    ///
    /// You typically don't need to call this function yourself.
    ///
    /// It may be useful for advanced use cases, like from exclusive systems.
    pub fn manually_tick(&self, progress: Progress) {
        self.total.fetch_add(progress.total, MemOrdering::Release);
        // use `min` to clamp in case a bad user provides `done > total`
        self.done
            .fetch_add(progress.done.min(progress.total), MemOrdering::Release);
    }
}

fn loadstate_enter(mut commands: Commands) {
    commands.insert_resource(ProgressCounter::default());
}

fn loadstate_exit(mut commands: Commands) {
    commands.remove_resource::<ProgressCounter>();
}

fn tracker(In(progress): In<Progress>, counter: Res<ProgressCounter>) {
    counter.manually_tick(progress);
}

fn check_progress<S: BevyState>(
    next_state: Local<Option<S>>,
    mut counter: ResMut<ProgressCounter>,
    mut state: ResMut<State<S>>,
) {
    let total = counter.total.load(MemOrdering::Acquire);
    let done = counter.done.load(MemOrdering::Acquire);

    let progress = Progress { done, total };

    // Update total progress to report to user
    counter.last_progress = progress;

    if progress.is_ready() {
        if let Some(next_state) = &*next_state {
            state.set(next_state.clone()).ok();
        }
    }
}

fn clear_progress(counter: ResMut<ProgressCounter>) {
    counter.done.store(0, MemOrdering::Release);
    counter.total.store(0, MemOrdering::Release);
}

/// Marker trait for all types that are valid for use as Bevy States
pub trait BevyState: Component + Debug + Clone + Eq + Hash {}
impl<T: Component + Debug + Clone + Eq + Hash> BevyState for T {}
