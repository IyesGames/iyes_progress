//! Progress Tracking Helper Crate
//!
//! This crate helps you in cases where you need to track when a bunch of
//! work has been completed, and perform a state transition.
//!
//! The most typical use case are loading screens, where you might need to
//! load assets, prepare the game world, etc… and then transition to the
//! in-game state when everything is done.
//!
//! However, this crate is general, and could also be used for any number of
//! other things, even things like cooldowns and animations (especially when
//! used with `iyes_loopless` to easily have many state types).
//!
//! Works with either legacy Bevy states (default) or `iyes_loopless` (via
//! optional cargo feature).
//!
//! To use this plugin, add one or more instances `ProgressPlugin` to your
//! `App`, configuring for the relevant states.
//!
//! Implement your tasks as regular Bevy systems that return a `Progress`
//! and add them to your respective state(s) using `.track_progress()`.
//!
//! The return value indicates how much progress a system has completed so
//! far. It specifies the currently completed "units of work" as well as
//! the expected total.
//!
//! When all registered systems return a progress value where `done >= total`,
//! your desired state transition will be performed automatically.
//!
//! If you need to access the overall progress information (say, to display a
//! progress bar), you can get it from the `ProgressCounter` resource.
//!
//! ---
//!
//! There is also an optional feature (`assets`) implementing basic asset
//! loading tracking. Just add your handles to the `AssetsLoading` resource.
//!
//! If you need something more advanced, I recommend the `bevy_asset_loader`
//! crate, which now has support for integrating with this crate. :)

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Add, AddAssign};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering as MemOrdering;

use bevy_ecs::schedule::StateData;
use bevy_ecs::prelude::*;
use bevy_app::prelude::*;

#[cfg(feature = "assets")]
mod asset;

/// Most used imports
pub mod prelude {
    pub use crate::ProgressPlugin;
    pub use crate::Progress;
    pub use crate::ProgressSystem;
    pub use crate::ProgressCounter;
    #[cfg(feature = "assets")]
    pub use crate::asset::AssetsLoading;
}

/// Progress reported by a system
///
/// It indicates how much work that system has still left to do.
///
/// When the value of `done` reaches the value of `total`, the system is considered "ready".
/// When all systems in your state are "ready", we will transition to the next state.
///
/// For your convenience, you can easily convert `bool`s into this type.
/// You can also convert `Progress` values into floats in the 0.0..1.0 range.
#[derive(Debug, Clone, Copy, Default)]
pub struct Progress {
    /// Units of work completed during this execution of the system
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

/// Add this plugin to your app, to use this crate for the specified state.
///
/// If you have multiple different states that need progress tracking,
/// you can add the plugin for each one.
///
/// **Warning**: Progress tracking will only work in some stages!
///
/// If not using `iyes_loopless`, it is only allowed in `CoreStage::Update`.
///
/// If using `iyes_loopless`, it is allowed in all stages after the
/// `StateTransitionStage` responsible for your state type, up to and including
/// `CoreStage::Last`.
///
/// You must ensure to not add any progress-tracked systems to any other stages!
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_loading::ProgressPlugin;
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
pub struct ProgressPlugin<S: StateData> {
    /// The loading state during which progress will be tracked
    pub state: S,
    /// The next state to transition to, when all progress completes
    pub next_state: Option<S>,
}

impl<S: StateData> ProgressPlugin<S> {
    /// Create a [`ProgressPlugin`] running during the given State
    pub fn new(state: S) -> Self {
        ProgressPlugin {
            state,
            next_state: None,
        }
    }

    /// Configure the [`ProgressPlugin`] to move on to the given state as soon as all Progress
    /// in the loading state is completed.
    pub fn continue_to(mut self, next_state: S) -> Self {
        self.next_state = Some(next_state);

        self
    }
}

#[cfg(not(feature = "iyes_loopless"))]
impl<S: StateData> Plugin for ProgressPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(self.state.clone())
                .with_system(loadstate_enter),
        );
        app.add_system_set(
            SystemSet::on_update(self.state.clone())
                .with_system(
                    next_frame
                        .exclusive_system()
                        .at_start()
                        .label(ProgressSystemLabel::Preparation),
                )
                .with_system(
                    check_progress::<S>(self.next_state.clone())
                        .exclusive_system()
                        .at_end()
                        .label(ProgressSystemLabel::CheckProgress),
                )
        );
        app.add_system_set(
            SystemSet::on_exit(self.state.clone())
                .with_system(loadstate_exit)
        );

        #[cfg(feature = "assets")]
        {
            app.init_resource::<asset::AssetsLoading>();
            app.add_system_set(
                SystemSet::on_update(self.state.clone())
                    .with_system(asset::assets_progress.track_progress()),
            );
            app.add_system_set(
                SystemSet::on_exit(self.state.clone())
                    .with_system(asset::assets_loading_reset),
            );
        }
    }
}

#[cfg(feature = "iyes_loopless")]
impl<S: StateData> Plugin for ProgressPlugin<S> {
    fn build(&self, app: &mut App) {
        use iyes_loopless::prelude::*;
        use iyes_loopless::condition::IntoConditionalExclusiveSystem;

        app.add_enter_system(self.state.clone(), loadstate_enter);
        app.add_exit_system(self.state.clone(), loadstate_exit);

        #[derive(Debug, Clone, PartialEq, Eq, Hash, StageLabel)]
        struct StageLabel(String);
        let stagelabel = StageLabel(format!("iyes_progress init: {:?}", &self.state));

        app.add_stage_after(
            iyes_loopless::state::app::StateTransitionStageLabel::from_type::<S>(),
            stagelabel.clone(),
            SystemStage::single_threaded()
        );

        app.add_system_to_stage(
            stagelabel,
            next_frame
                .run_in_state(self.state.clone())
                .at_start()
                .label(ProgressSystemLabel::Preparation),
        );

        app.add_system_to_stage(
            CoreStage::Last,
            check_progress::<S>(self.next_state.clone())
                .run_in_state(self.state.clone())
                .at_end()
                .label(ProgressSystemLabel::CheckProgress),
        );

        #[cfg(feature = "assets")]
        {
            app.init_resource::<asset::AssetsLoading>();
            app.add_exit_system(self.state.clone(), asset::assets_loading_reset);
            app.add_system(
                asset::assets_progress
                    .track_progress()
                    .run_in_state(self.state.clone())
            );
        }
    }
}

#[cfg(not(feature = "iyes_loopless"))]
/// Extension trait for systems with Progress tracking
pub trait ProgressSystem<Params>: IntoSystem<(), Progress, Params> {
    /// Call this to add your system returning [`Progress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor;
}

#[cfg(not(feature = "iyes_loopless"))]
impl<S, Params> ProgressSystem<Params> for S
where S: IntoSystem<(), Progress, Params>
{
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor {
        self.chain(
            |In(progress): In<Progress>, counter: Res<ProgressCounter>| {
                counter.manually_track(progress)
            },
        )
        .label(ProgressSystemLabel::Tracking)
    }
}

#[cfg(feature = "iyes_loopless")]
/// Extension trait for systems with Progress tracking
pub trait ProgressSystem<Params>: IntoSystem<(), Progress, Params> {
    /// Call this to add your system returning [`Progress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> iyes_loopless::condition::ConditionalSystemDescriptor;
}

#[cfg(feature = "iyes_loopless")]
impl<S, Params> ProgressSystem<Params> for S
where S: IntoSystem<(), Progress, Params>
{
    fn track_progress(self) -> iyes_loopless::condition::ConditionalSystemDescriptor {
        use iyes_loopless::condition::IntoConditionalSystem;
        self.chain(
            |In(progress): In<Progress>, counter: Res<ProgressCounter>| {
                counter.manually_track(progress)
            },
        )
        .into_conditional()
        .label(ProgressSystemLabel::Tracking)
    }
}

/// Label to control system execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
pub enum ProgressSystemLabel {
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
/// This resource is automatically created when entering a state that was
/// configured using [`ProgressPlugin`], and removed when exiting it.
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

#[cfg(not(feature = "iyes_loopless"))]
fn check_progress<S: StateData>(next_state: Option<S>) -> impl FnMut(&mut World) {
    move |world| {
        let progress = world.resource::<ProgressCounter>().progress();
        if progress.is_ready() {
            if let Some(next_state) = &next_state {
                let mut state = world.resource_mut::<State<S>>();
                state.set(next_state.clone()).ok();
            }
        }
    }
}
#[cfg(feature = "iyes_loopless")]
fn check_progress<S: StateData>(next_state: Option<S>) -> impl FnMut(&mut World) {
    move |world| {
        let progress = world.resource::<ProgressCounter>().progress();
        if progress.is_ready() {
            if let Some(next_state) = &next_state {
                world.insert_resource(iyes_loopless::state::NextState(next_state.clone()));
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
