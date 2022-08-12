use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::StateData;

use crate::{ProgressPlugin, ProgressSystemLabel};
use crate::ProgressCounter;
use crate::{Progress, HiddenProgress};

pub mod prelude {
    pub use super::{ProgressSystem, HiddenProgressSystem, MixedProgressSystem};
}

impl<S: StateData> Plugin for ProgressPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(self.state.clone()).with_system(crate::loadstate_enter));
        app.add_system_set(
            SystemSet::on_update(self.state.clone())
                .with_system(
                    crate::next_frame
                        .exclusive_system()
                        .at_start()
                        .label(ProgressSystemLabel::Preparation),
                )
                .with_system(
                    check_progress::<S>(self.next_state.clone())
                        .exclusive_system()
                        .at_end()
                        .label(ProgressSystemLabel::CheckProgress),
                ),
        );
        app.add_system_set(SystemSet::on_exit(self.state.clone()).with_system(crate::loadstate_exit));

        #[cfg(feature = "assets")]
        if self.track_assets {
            app.init_resource::<crate::asset::AssetsLoading>();
            app.add_system_set(
                SystemSet::on_update(self.state.clone())
                    .with_system(crate::asset::assets_progress.track_progress()),
            );
            app.add_system_set(
                SystemSet::on_exit(self.state.clone()).with_system(crate::asset::assets_loading_reset),
            );
        }

        #[cfg(not(feature = "assets"))]
        if self.track_assets {
            panic!("Enable the \"assets\" cargo feature to use assets tracking!");
        }
    }
}

/// Extension trait for systems with Progress tracking
pub trait ProgressSystem<Params>: IntoSystem<(), Progress, Params> {
    /// Call this to add your system returning [`Progress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor;
}

impl<S, Params> ProgressSystem<Params> for S
where
    S: IntoSystem<(), Progress, Params>,
{
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor {
        self.chain(
            |In(progress): In<Progress>, counter: Res<ProgressCounter>| {
                counter.manually_track(progress);
            },
        )
        .label(ProgressSystemLabel::Tracking)
    }
}

/// Extension trait for systems with Progress tracking
pub trait HiddenProgressSystem<Params>: IntoSystem<(), HiddenProgress, Params> {
    /// Call this to add your system returning [`HiddenProgress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor;
}

impl<S, Params> HiddenProgressSystem<Params> for S
where
    S: IntoSystem<(), HiddenProgress, Params>,
{
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor {
        self.chain(
            |In(progress): In<HiddenProgress>, counter: Res<ProgressCounter>| {
                counter.manually_track_hidden(progress);
            },
        )
        .label(ProgressSystemLabel::Tracking)
    }
}

/// Extension trait for systems with Progress tracking
pub trait MixedProgressSystem<Params>: IntoSystem<(), (Progress, HiddenProgress), Params> {
    /// Call this to add your system returning both `Progress` and `HiddenProgress` to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor;
}

impl<S, Params> MixedProgressSystem<Params> for S
where
    S: IntoSystem<(), (Progress, HiddenProgress), Params>,
{
    fn track_progress(self) -> bevy_ecs::schedule::ParallelSystemDescriptor {
        self.chain(
            |In((progress, hidden)): In<(Progress, HiddenProgress)>, counter: Res<ProgressCounter>| {
                counter.manually_track(progress);
                counter.manually_track_hidden(hidden);
            },
        )
        .label(ProgressSystemLabel::Tracking)
    }
}

fn check_progress<S: StateData>(next_state: Option<S>) -> impl FnMut(&mut World) {
    move |world| {
        let progress = world.resource::<ProgressCounter>().progress_complete();
        if progress.is_ready() {
            if let Some(next_state) = &next_state {
                let mut state = world.resource_mut::<State<S>>();
                state.set(next_state.clone()).ok();
            }
        }
    }
}
