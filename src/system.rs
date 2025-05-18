use bevy_ecs::{prelude::*, schedule::ScheduleConfigs};
use bevy_state::state::FreelyMutableState;

use crate::prelude::*;

type SystemConfigs = ScheduleConfigs<Box<dyn System<In = (), Out = Result<(), BevyError>>>>;

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
/// # use bevy::prelude::*;
/// # use iyes_progress::prelude::*;
/// #
/// # #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
/// # enum MyStates {}
/// #
/// # fn my_system() -> Progress {
/// #     default()
/// # }
/// #
/// # fn plugin(app: &mut App) {
/// app.add_systems(Update,
///     my_system
///         .pipe(hide_progress)
///         .track_progress::<MyStates>()
/// );
/// # }
/// ```
pub fn hide_progress(In(progress): In<Progress>) -> HiddenProgress {
    HiddenProgress(progress)
}

/// Adapter for converting a system returning [`HiddenProgress`] into
/// [`Progress`]
///
/// Example:
/// ```rust
/// # use bevy::prelude::*;
/// # use iyes_progress::prelude::*;
/// #
/// # #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
/// # enum MyStates {}
/// #
/// # fn my_system() -> HiddenProgress {
/// #     default()
/// # }
/// #
/// # fn plugin(app: &mut App) {
/// app.add_systems(Update,
///     my_system
///         .pipe(unhide_progress)
///         .track_progress::<MyStates>()
/// );
/// # }
/// ```
pub fn unhide_progress(In(progress): In<HiddenProgress>) -> Progress {
    progress.0
}
