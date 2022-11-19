use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::StateData;

use crate::{ProgressPlugin, ProgressSystemLabel};
use crate::ProgressCounter;
use crate::ApplyProgress;

pub mod prelude {
    pub use super::ProgressSystem;
}

impl<S: StateData> Plugin for ProgressPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(self.state.clone()).with_system(crate::loadstate_enter));
        app.add_system_set(
            SystemSet::on_update(self.state.clone())
                .with_system(
                    crate::next_frame
                        .at_start()
                        .label(ProgressSystemLabel::Preparation),
                )
                .with_system(
                    check_progress::<S>(self.next_state.clone())
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

    fn is_unique(&self) -> bool {
        false
    }
}

/// Extension trait for systems with progress tracking
pub trait ProgressSystem<Params, T: ApplyProgress>: IntoSystem<(), T, Params> {
    /// Call this to add your system returning [`Progress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> bevy_ecs::schedule::SystemDescriptor;
}

impl<S, T, Params> ProgressSystem<Params, T> for S
where
    T: ApplyProgress + 'static,
    S: IntoSystem<(), T, Params>,
{
    fn track_progress(self) -> bevy_ecs::schedule::SystemDescriptor {
        self.pipe(
            |In(progress): In<T>, counter: Res<ProgressCounter>| {
                progress.apply_progress(&*counter);
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
