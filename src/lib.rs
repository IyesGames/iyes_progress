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
//! other things, even things like cooldowns and animations.
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

use bevy_app::{prelude::*, MainScheduleOrder};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ExecutorKind, SystemConfigs, ScheduleLabel};
use bevy_utils::{Duration, Instant};

#[cfg(feature = "debug")]
use bevy_log::prelude::*;

#[cfg(feature = "assets")]
mod asset;

/// Most used imports
pub mod prelude {
    #[cfg(feature = "assets")]
    pub use crate::asset::AssetsLoading;
    pub use crate::HiddenProgress;
    pub use crate::Progress;
    pub use crate::ProgressCounter;
    pub use crate::ProgressPlugin;
    pub use crate::ProgressSystem;
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
///
/// If you want your system to report some progress in a way that is counted separately
/// and should not affect progress bars or other user-facing indicators, you can
/// use [`HiddenProgress`] instead.
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

/// "Hidden" progress reported by a system.
///
/// Works just like the regular [`Progress`], but will be accounted differently
/// in [`ProgressCounter`].
///
/// Hidden progress counts towards the true total (like for triggering the
/// state transition) as reported by the `progress_complete` method, but is not
/// counted by the `progress` method. The intention is that it should not
/// affect things like progress bars and other user-facing indicators.
#[derive(Debug, Clone, Copy, Default)]
pub struct HiddenProgress(pub Progress);

/// Add this plugin to your app, to use this crate for the specified state.
///
/// If you have multiple different states that need progress tracking,
/// you can add the plugin for each one. Tracking multiple state types
/// simultaneously is *not* currently supported (see issue #20).
///
/// If you want the optional assets tracking ("assets" cargo feature), enable
/// it with `.track_assets()`.
///
/// **Warning**: Progress tracking will only work after the [`StateTransition`]!
///
/// [`TrackedProgressSet`] represents all systems with progress tracking enabled.
/// Calling [`track_progress`] will add your systems to the set automatically.
///
/// [`StateTransition`]: bevy_app::StateTransition
/// [`track_progress`]: crate::ProgressSystem::track_progress
///
/// ```rust
/// # use bevy::prelude::*;
/// # use iyes_progress::ProgressPlugin;
/// # let mut app = App::default();
/// # app.add_state::<MyState>();
/// app.add_plugin(ProgressPlugin::new(MyState::GameLoading).continue_to(MyState::InGame));
/// app.add_plugin(ProgressPlugin::new(MyState::Splash).continue_to(MyState::MainMenu));
/// # #[derive(Debug, Clone, PartialEq, Eq, Hash, Default, States)]
/// # enum MyState {
/// #     #[default]
/// #     Splash,
/// #     MainMenu,
/// #     GameLoading,
/// #     InGame,
/// # }
/// ```
pub struct ProgressPlugin<S: States> {
    /// The loading state during which progress will be tracked
    pub state: S,
    /// The next state to transition to, when all progress completes
    pub next_state: Option<S>,
    /// Unique name, made using the loading state
    pub(crate) plugin_name: String,
    /// Whether to enable the optional assets tracking feature
    pub track_assets: bool,
}

impl<S: States> ProgressPlugin<S> {
    /// Create a [`ProgressPlugin`] running during the given State
    pub fn new(state: S) -> Self {
        ProgressPlugin {
            plugin_name: format!("{}({:?})", std::any::type_name::<Self>(), state),
            state,
            next_state: None,
            track_assets: false,
        }
    }

    /// Configure the [`ProgressPlugin`] to move on to the given state as soon as all Progress
    /// in the loading state is completed.
    pub fn continue_to(mut self, next_state: S) -> Self {
        self.next_state = Some(next_state);
        self
    }

    #[cfg(feature = "assets")]
    /// Enable the optional assets tracking feature
    pub fn track_assets(mut self) -> Self {
        self.track_assets = true;
        self
    }
}

impl<S: States> Plugin for ProgressPlugin<S> {
    fn build(&self, app: &mut App) {
        // set up a schedule after `StateTransition`, where we init our stuff
        if app.get_schedule(ProgressPreparationSchedule).is_none() {
            app.init_schedule(ProgressPreparationSchedule);
            app.edit_schedule(ProgressPreparationSchedule, |sched| {
                sched.set_executor_kind(ExecutorKind::SingleThreaded);
            });
            app.init_resource::<MainScheduleOrder>();
            app.world.resource_mut::<MainScheduleOrder>()
                .insert_after(StateTransition, ProgressPreparationSchedule);
        }

        // clear/init progress count every frame
        app.add_systems(
            ProgressPreparationSchedule,
            next_frame
                .run_if(in_state(self.state.clone()))
                // just in case some progress-tracked systems exist in `ProgressPreparationSchedule`
                .before(TrackedProgressSet)
        );

        // setup and cleanup on state transition
        app.add_systems(OnEnter(self.state.clone()), loadstate_enter);
        app.add_systems(OnExit(self.state.clone()), loadstate_exit);

        // check progress and queue any state transition as late as possible, in `Last`
        if let Some(next_state) = &self.next_state {
            app.add_systems(
                Last,
                check_progress::<S>(next_state.clone())
                    .run_if(in_state(self.state.clone()))
                    // just in case some progress-tracked systems exist in `Last`
                    .after(TrackedProgressSet)
            );
        }

        #[cfg(feature = "debug")]
        // ensure we only add this stuff once, even if the plugin is added multiple times
        // (it is state-agnostic)
        if !app.world.contains_resource::<ProgressDebug>() {
            app.init_resource::<ProgressDebug>();
            app.add_systems(
                Last,
                debug_progress
                    .after(TrackedProgressSet)
                    .run_if(resource_exists::<ProgressCounter>())
                    .run_if(progress_debug_enabled)
            );
        }

        #[cfg(feature = "assets")]
        if self.track_assets {
            app.init_resource::<asset::AssetsLoading>();
            app.add_systems(
                Update,
                asset::assets_progress
                    .track_progress()
                    .run_if(in_state(self.state.clone())),
            );
            app.add_systems(OnExit(self.state.clone()), asset::assets_loading_reset);
        }

        #[cfg(not(feature = "assets"))]
        if self.track_assets {
            panic!("Enable the \"assets\" cargo feature to use assets tracking!");
        }
    }

    fn name(&self) -> &str {
        &self.plugin_name
    }
}

/// Extension trait for systems with progress tracking
pub trait ProgressSystem<Params, T: ApplyProgress>: IntoSystem<(), T, Params> {
    /// Call this to add your system returning [`Progress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> SystemConfigs;
}

impl<S, T, Params> ProgressSystem<Params, T> for S
where
    T: ApplyProgress + 'static,
    S: IntoSystem<(), T, Params>,
{
    fn track_progress(self) -> SystemConfigs {
        self.pipe(|In(progress): In<T>, counter: Res<ProgressCounter>| {
            progress.apply_progress(&*counter);
        })
        .in_set(TrackedProgressSet)
    }
}

fn check_progress<S: States>(next_state: S) -> impl FnMut(Res<ProgressCounter>, ResMut<NextState<S>>) {
    move |progress, mut state| {
        if progress.progress_complete().is_ready() {
            state.set(next_state.clone());
            #[cfg(feature = "debug")]
            debug!("Progress complete! Queueing state transition!");
        }
    }
}

/// Schedule where progress tracking is initialized every frame.
///
/// This will run after `StateTransition`, in the `Main` Bevy schedule.
///
/// Progress-tracked systems must not be added to schedules that run before this!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ProgressPreparationSchedule;

/// Any system tracking progress should be part of this set.
/// All systems wrapped in [`track_progress`] are automatically part of this set.
///
/// [`track_progress`]: crate::ProgressSystem::track_progress
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct TrackedProgressSet;

/// Resource for tracking overall progress
///
/// This resource is automatically created when entering a state that was
/// configured using [`ProgressPlugin`], and removed when exiting it.
#[derive(Resource, Default)]
pub struct ProgressCounter {
    // use atomics to track overall progress,
    // so that we can avoid mut access in tracked systems,
    // allowing them to run in parallel
    done: AtomicU32,
    total: AtomicU32,
    done_hidden: AtomicU32,
    total_hidden: AtomicU32,
    persisted: Progress,
    persisted_hidden: Progress,
}

impl ProgressCounter {
    /// Get the latest overall progress information
    ///
    /// This is the combined total of all systems.
    ///
    /// To get correct information, make sure that you call this function only after
    /// all your systems that track progress finished.
    ///
    /// This does not include "hidden" progress. To get the full "real" total, use
    /// `progress_complete`.
    ///
    /// Use this method for progress bars and other things that indicate/report
    /// progress information to the user.
    pub fn progress(&self) -> Progress {
        let total = self.total.load(MemOrdering::Acquire);
        let done = self.done.load(MemOrdering::Acquire);

        Progress { done, total }
    }

    /// Get the latest overall progress information
    ///
    /// This is the combined total of all systems.
    ///
    /// To get correct information, make sure that you call this function only after
    /// all your systems that track progress finished
    ///
    /// This includes "hidden" progress. To get only the "visible" progress, use
    /// `progress`.
    ///
    /// This is the method to be used for things like state transitions, and other
    /// use cases that must account for the "true" actual progress of the
    /// registered systems.
    pub fn progress_complete(&self) -> Progress {
        let total =
            self.total.load(MemOrdering::Acquire) +
            self.total_hidden.load(MemOrdering::Acquire);
        let done =
            self.done.load(MemOrdering::Acquire) +
            self.done_hidden.load(MemOrdering::Acquire);

        Progress { done, total }
    }

    /// Add some amount of progress to the running total for the current frame.
    ///
    /// In most cases you do not want to call this function yourself.
    /// Let your systems return a [`Progress`] and wrap them in [`track_progress`] instead.
    ///
    /// [`track_progress`]: crate::ProgressSystem::track_progress
    pub fn manually_track(&self, progress: Progress) {
        self.total.fetch_add(progress.total, MemOrdering::Release);
        // use `min` to clamp in case a bad user provides `done > total`
        self.done
            .fetch_add(progress.done.min(progress.total), MemOrdering::Release);
    }

    /// Add some amount of "hidden" progress to the running total for the current frame.
    ///
    /// Hidden progress counts towards the true total (like for triggering the
    /// state transition) as reported by the `progress_complete` method, but is not
    /// counted by the `progress` method. The intention is that it should not
    /// affect things like progress bars and other user-facing indicators.
    ///
    /// In most cases you do not want to call this function yourself.
    /// Let your systems return a [`Progress`] and wrap them in [`track_progress`] instead.
    ///
    /// [`track_progress`]: crate::ProgressSystem::track_progress
    pub fn manually_track_hidden(&self, progress: HiddenProgress) {
        self.total_hidden
            .fetch_add(progress.0.total, MemOrdering::Release);
        // use `min` to clamp in case a bad user provides `done > total`
        self.done_hidden
            .fetch_add(progress.0.done.min(progress.0.total), MemOrdering::Release);
    }

    /// Persist progress for the rest of the current state
    pub fn persist_progress(&mut self, progress: Progress) {
        self.manually_track(progress);
        self.persisted += progress;
    }

    /// Persist hidden progress for the rest of the current state
    pub fn persist_progress_hidden(&mut self, progress: HiddenProgress) {
        self.manually_track_hidden(progress);
        self.persisted_hidden += progress.0;
    }
}

/// Trait for all types that can be returned by systems to report progress
pub trait ApplyProgress {
    /// Account the value into the total progress for this frame
    fn apply_progress(self, total: &ProgressCounter);
}

impl ApplyProgress for Progress {
    fn apply_progress(self, total: &ProgressCounter) {
        total.manually_track(self);
    }
}

impl ApplyProgress for HiddenProgress {
    fn apply_progress(self, total: &ProgressCounter) {
        total.manually_track_hidden(self);
    }
}

impl<T: ApplyProgress> ApplyProgress for (T, T) {
    fn apply_progress(self, total: &ProgressCounter) {
        self.0.apply_progress(total);
        self.1.apply_progress(total);
    }
}

fn loadstate_enter(mut commands: Commands) {
    commands.insert_resource(ProgressCounter::default());
    #[cfg(feature = "debug")]
    debug!("Progress counting enabled on state enter.");
}

fn loadstate_exit(mut commands: Commands) {
    commands.remove_resource::<ProgressCounter>();
    #[cfg(feature = "debug")]
    debug!("Progress counting disabled on state exit.");
}

fn next_frame(counter: Res<ProgressCounter>) {
    counter
        .done
        .store(counter.persisted.done, MemOrdering::Release);
    counter
        .total
        .store(counter.persisted.total, MemOrdering::Release);

    counter
        .done_hidden
        .store(counter.persisted_hidden.done, MemOrdering::Release);
    counter
        .total_hidden
        .store(counter.persisted_hidden.total, MemOrdering::Release);
}

/// Dummy system to count for a number of frames
///
/// May be useful for testing/debug/workaround purposes.
pub fn dummy_system_wait_frames<const N: u32>(mut count: Local<u32>) -> HiddenProgress {
    if *count <= N {
        *count += 1;
    }
    HiddenProgress(Progress {
        done: *count - 1,
        total: N,
    })
}

/// Dummy system to wait for a time duration
///
/// May be useful for testing/debug/workaround purposes.
pub fn dummy_system_wait_millis<const MILLIS: u64>(
    mut state: Local<Option<Instant>>,
) -> HiddenProgress {
    let end = state.unwrap_or_else(
        || Instant::now() + Duration::from_millis(MILLIS)
    );
    *state = Some(end);
    HiddenProgress((Instant::now() > end).into())
}

#[cfg(feature = "debug")]
/// Use this resource to control the logging of debug info
///
/// Enabled by default. Only available if the `debug` cargo feature is enabled.
#[derive(Resource)]
pub struct ProgressDebug {
    enabled: bool,
}

#[cfg(feature = "debug")]
impl Default for ProgressDebug {
    fn default() -> Self {
        Self {
            enabled: true,
        }
    }
}

#[cfg(feature = "debug")]
fn debug_progress(counter: Res<ProgressCounter>) {
    let progress = counter.progress();
    let progress_full = counter.progress_complete();
    trace!(
        "Progress: {}/{}; Full Progress: {}/{}",
        progress.done,
        progress.total,
        progress_full.done,
        progress_full.total,
    );
}

#[cfg(feature = "debug")]
fn progress_debug_enabled(cfg: Option<Res<ProgressDebug>>) -> bool {
    cfg.map(|cfg| cfg.enabled).unwrap_or(false)
}
