use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_state::state::{FreelyMutableState, State};

use crate::state::*;
use crate::tracker::ProgressTracker;

/// Use this resource to control the logging of progress values every frame.
///
/// The log messages are at TRACE level.
///
/// Enabled by default. Only available if the `debug` cargo feature is enabled.
#[derive(Resource)]
pub struct ProgressDebug {
    /// If true, print trace messages.
    pub enabled: bool,
}

impl Default for ProgressDebug {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub(crate) fn rc_debug_progress<S: FreelyMutableState>(
    cfg_debug: Option<Res<ProgressDebug>>,
    cfg_state: Res<StateTransitionConfig<S>>,
    state: Res<State<S>>,
) -> bool {
    cfg_debug.map(|cfg| cfg.enabled).unwrap_or(false)
        && cfg_state.map_from_to.contains_key(state.get())
}

pub(crate) fn debug_progress<S: FreelyMutableState>(
    pt: Res<ProgressTracker<S>>,
) {
    let visible = pt.get_global_progress();
    let hidden = pt.get_global_hidden_progress().0;
    let full = pt.get_global_combined_progress();
    trace!(
        "Progress: Visible: {}/{}, Hidden: {}/{}, Full: {}/{}",
        visible.done,
        visible.total,
        hidden.done,
        hidden.total,
        full.done,
        full.total,
    );
}
