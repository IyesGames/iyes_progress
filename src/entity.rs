use std::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_state::state::FreelyMutableState;

use crate::prelude::*;

/// Component to store progress on an entity.
///
/// This is yet another way to report progress. If you insert this component
/// onto entities, `iyes_progress` will copy the values from here into
/// entries in the `ProgressTracker<S>` (using a system that runs in `PostUpdate`).
///
/// Every entity with this component gets its own entry in the `ProgressTracker<S>`.
/// If the entity is despawned or the component removed, any previously reported
/// progress will stay.
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
    q: Query<(Entity, &ProgressEntity<S>)>,
) {
    q.iter().for_each(|(e, pfs)| {
        tracker.set_progress(
            ProgressEntryId::from_entity(e),
            pfs.visible.done,
            pfs.visible.total,
        );
        tracker.set_hidden_progress(
            ProgressEntryId::from_entity(e),
            pfs.hidden.done,
            pfs.hidden.total,
        );
    });
}
