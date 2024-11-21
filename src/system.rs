use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemConfigs;
use bevy_state::state::FreelyMutableState;

use crate::prelude::*;

/// Extension trait to add the APIs for handling systems that return progress.
pub trait ProgressReturningSystem<T, Params> {
    /// Call this to add your system returning [`Progress`] to your
    /// [`App`](bevy_app::App)
    ///
    /// It will create an entry in the [`ProgressTracker`] to represent the
    /// system. Every time your system runs, the values it returns will
    /// overwrite the previously stored values in the entry.
    ///
    /// Note: it is OK if your system does not run every frame (for example,
    /// if you have run conditions). The value from when the system last ran
    /// will be retained until your system runs again.
    fn track_progress<S: FreelyMutableState>(self) -> SystemConfigs;

    /// Like [`track_progress`](Self::track_progress), but adds a run condition
    /// to no longer run the system after it has returned a fully ready
    /// progress value.
    fn track_progress_and_stop<S: FreelyMutableState>(self) -> SystemConfigs;
}

impl<S, T, Params> ProgressReturningSystem<T, Params> for S
where
    S: IntoSystem<(), T, Params>,
    T: ApplyProgress + 'static,
{
    fn track_progress<State: FreelyMutableState>(self) -> SystemConfigs {
        let id = ProgressEntryId::new();
        self.pipe(
            move |In(progress): In<T>, tracker: Res<ProgressTracker<State>>| {
                progress.apply_progress(&tracker, id);
            },
        )
        .into_configs()
    }

    fn track_progress_and_stop<State: FreelyMutableState>(
        self,
    ) -> SystemConfigs {
        let id = ProgressEntryId::new();
        self.pipe(
            move |In(progress): In<T>, tracker: Res<ProgressTracker<State>>| {
                progress.apply_progress(&tracker, id);
            },
        )
        .run_if(move |tracker: Res<ProgressTracker<State>>| {
            !tracker.is_id_ready(id)
        })
        .into_configs()
    }
}

/// Adapter for converting a system returning [`Progress`] into
/// [`HiddenProgress`]
///
/// Example:
/// ```rust
/// app.add_systems(Update,
///     my_system
///         .hide_progress()
///         .track_progress()
/// );
/// ```
pub fn hide_progress(In(progress): In<Progress>) -> HiddenProgress {
    HiddenProgress(progress)
}

/// Adapter for converting a system returning [`HiddenProgress`] into
/// [`Progress`]
///
/// Example:
/// ```rust
/// app.add_systems(Update,
///     my_system
///         .unhide_progress()
///         .track_progress()
/// );
/// ```
pub fn unhide_progress(In(progress): In<HiddenProgress>) -> Progress {
    progress.0
}
