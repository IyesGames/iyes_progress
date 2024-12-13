use std::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_state::state::FreelyMutableState;

use crate::prelude::*;

/// Component to store progress on an entity.
///
/// This is yet another way to report/store progress. You can insert
/// this component on your entities. A system (in [`PostUpdate`]) will
/// sum up all the values and track that sum in the [`ProgressTracker<S>`].
///
/// Note: the values from individual instances of this component are not
/// copied/replicated in the [`ProgressTracker`]. Only the total sum is
/// tracked. If you despawn your entity, any progress that was stored on it
/// will be lost.
///
/// ```rust
/// commands.spawn((
///     ProgressEntity::<MyStates>::new()
///         .with_progress(0, 1)
///         .with_hidden_progress(0, 1),
///
///     // ... other components
/// ));
/// ```
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ProgressEntity<S: FreelyMutableState> {
    /// The visible progress associated with the entity.
    pub visible: Progress,
    /// The hidden progress associated with the entity.
    pub hidden: HiddenProgress,
    _pd: PhantomData<S>,
}

impl<S: FreelyMutableState> Default for ProgressEntity<S> {
    fn default() -> Self {
        Self {
            visible: Progress::default(),
            hidden: HiddenProgress::default(),
            _pd: PhantomData,
        }
    }
}

impl<S: FreelyMutableState> ProgressEntity<S> {
    /// The same as `Default::default()`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Builder-style method to set the visible progress.
    pub fn with_progress(mut self, done: u32, total: u32) -> Self {
        self.visible.done = done;
        self.visible.total = total;
        self
    }

    /// Builder-style method to set the hidden progress.
    pub fn with_hidden_progress(mut self, done: u32, total: u32) -> Self {
        self.hidden.done = done;
        self.hidden.total = total;
        self
    }
}

pub(crate) fn apply_progress_from_entities<S: FreelyMutableState>(
    tracker: Res<ProgressTracker<S>>,
    q: Query<&ProgressEntity<S>>,
) {
    let sum = q.iter().fold(
        (Progress::default(), HiddenProgress::default()),
        |sum, pfs| {
            (sum.0 + pfs.visible, sum.1 + pfs.hidden)
        },
    );
    tracker.set_sum_entities(sum.0, sum.1);
}
