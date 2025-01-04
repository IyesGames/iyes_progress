use bevy_ecs::prelude::*;
#[cfg(feature = "debug")]
use bevy_log::prelude::*;
use bevy_state::state::{FreelyMutableState, NextState, State};
use bevy_utils::HashMap;

use crate::prelude::*;

#[derive(Resource, Clone)]
pub(crate) struct StateTransitionConfig<S: FreelyMutableState> {
    pub(crate) map_from_to: HashMap<S, S>,
}

impl<S: FreelyMutableState> Default for StateTransitionConfig<S> {
    fn default() -> Self {
        Self {
            map_from_to: Default::default(),
        }
    }
}

/// System that calls [`ProgressTracker::clear`].
///
/// This will be automatically added to the `OnEnter`/`OnExit`
/// schedules of progress-tracked states, if so configured
/// by the [`ProgressPlugin`].
///
/// This `fn` is `pub` so you can order your systems around it.
/// Or add other "clearing points" to your app.
pub fn clear_global_progress<S: FreelyMutableState>(
    mut gpt: ResMut<ProgressTracker<S>>,
) {
    gpt.clear();
    #[cfg(feature = "debug")]
    debug!("Clearing progress data.");
}

pub(crate) fn rc_configured_state<S: FreelyMutableState>(
    config: Res<StateTransitionConfig<S>>,
    state: Option<Res<State<S>>>,
) -> bool {
    let Some(state) = state else { return false };
    config.map_from_to.contains_key(state.get())
}

pub(crate) fn transition_if_ready<S: FreelyMutableState>(
    gpt: Res<ProgressTracker<S>>,
    config: Res<StateTransitionConfig<S>>,
    state: Res<State<S>>,
    mut next_state: ResMut<NextState<S>>,
) {
    if let Some(to) = config.map_from_to.get(state.get()) {
        if gpt.is_ready() {
            next_state.set(to.clone());
            #[cfg(feature = "debug")]
            debug!("Progress complete! Transitioning to state {:?}", to);
        }
    }
}
