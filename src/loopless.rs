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
        use iyes_loopless::condition::IntoConditionalExclusiveSystem;
        use iyes_loopless::prelude::*;

        app.add_enter_system(self.state.clone(), crate::loadstate_enter);
        app.add_exit_system(self.state.clone(), crate::loadstate_exit);

        #[derive(Debug, Clone)]
        struct StageLabel(String);

        impl bevy_ecs::schedule::StageLabel for StageLabel {
            fn as_str(&self) -> &'static str {
                Box::leak(self.0.clone().into_boxed_str())
            }
        }

        let stagelabel = StageLabel(format!("iyes_progress init: {:?}", &self.state));

        app.add_stage_after(
            iyes_loopless::state::StateTransitionStageLabel::from_type::<S>(),
            stagelabel.clone(),
            SystemStage::single_threaded(),
        );

        app.add_system_to_stage(
            stagelabel,
            crate::next_frame
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
        if self.track_assets {
            app.init_resource::<crate::asset::AssetsLoading>();
            app.add_exit_system(self.state.clone(), crate::asset::assets_loading_reset);
            app.add_system(
                crate::asset::assets_progress
                    .track_progress()
                    .run_in_state(self.state.clone()),
            );
        }

        #[cfg(not(feature = "assets"))]
        if self.track_assets {
            panic!("Enable the \"assets\" cargo feature to use assets tracking!");
        }
    }
}

/// Extension trait for systems with Progress tracking
pub trait ProgressSystem<Params, T: ApplyProgress>: IntoSystem<(), T, Params> {
    /// Call this to add your system returning [`Progress`] to your [`App`]
    ///
    /// This adds the functionality for tracking the returned Progress.
    fn track_progress(self) -> iyes_loopless::condition::ConditionalSystemDescriptor;
}

impl<S, T, Params> ProgressSystem<Params, T> for S
where
    T: ApplyProgress + 'static,
    S: IntoSystem<(), T, Params>,
{
    fn track_progress(self) -> iyes_loopless::condition::ConditionalSystemDescriptor {
        use iyes_loopless::condition::IntoConditionalSystem;
        self.chain(
            |In(progress): In<T>, counter: Res<ProgressCounter>| {
                progress.apply_progress(&*counter);
            },
        )
        .into_conditional()
        .label(ProgressSystemLabel::Tracking)
    }
}

fn check_progress<S: StateData>(next_state: Option<S>) -> impl FnMut(&mut World) {
    move |world| {
        let progress = world.resource::<ProgressCounter>().progress_complete();
        if progress.is_ready() {
            if let Some(next_state) = &next_state {
                world.insert_resource(iyes_loopless::state::NextState(next_state.clone()));
            }
        }
    }
}
