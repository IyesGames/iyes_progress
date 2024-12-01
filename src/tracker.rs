//! Storing and tracking progress

use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_state::state::FreelyMutableState;
use bevy_utils::HashMap;
use parking_lot::Mutex;

use crate::prelude::{HiddenProgress, Progress};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// An opaque ID for accessing data stored in the [`ProgressTracker`].
///
/// The ID can be used with the [`ProgressTracker`] resource
/// (for any state type) to record [`Progress`] and [`HiddenProgress`].
///
/// Normally, `iyes_progress` will automatically manage these IDs for you
/// under the hood, if you use the [`ProgressEntry`] system param or
/// write systems that return progress values.
///
/// However, for some advanced use cases, you might want to do it manually.
/// You can create a new unique ID at any time by calling
/// [`ProgressEntryId::new()`]. Store that ID and then use it to update the
/// values in the [`ProgressTracker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgressEntryId(usize);

impl ProgressEntryId {
    /// Create a new unique ID
    pub fn new() -> ProgressEntryId {
        let next_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        ProgressEntryId(next_id)
    }
}

/// The resource where all the progress information is stored.
///
/// You can get information about the overall accumulated progress
/// from here. You can also manage the progress values associated
/// with specific [`ProgressEntryId`]s.
///
/// The internal data is behind a mutex, to allow shared access.
/// Bevy systems only need `Res`, not `ResMut`, allowing systems
/// that use this resource to run in parallel.
///
/// All stored values are cleared automatically when entering a
/// state configured for progress tracking. You can reset everything
/// manually by calling [`clear`](Self::clear).
#[derive(Resource)]
pub struct ProgressTracker<S: FreelyMutableState> {
    inner: Mutex<GlobalProgressTrackerInner>,
    _pd: PhantomData<S>,
}

impl<S: FreelyMutableState> Default for ProgressTracker<S> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            _pd: PhantomData,
        }
    }
}

#[derive(Default)]
struct GlobalProgressTrackerInner {
    entries: HashMap<usize, (Progress, HiddenProgress)>,
    accum: (Progress, HiddenProgress),
}

impl<S: FreelyMutableState> ProgressTracker<S> {
    /// Clear all stored progress values.
    pub fn clear(&mut self) {
        let mut inner = self.inner.lock();
        *inner = Default::default();
    }

    /// Call a closure on each entry stored in the tracker.
    ///
    /// This allows you to inspect or mutate anything stored in the tracker,
    /// which can be useful for debugging or for advanced use cases.
    pub fn foreach_entry(
        &self,
        mut f: impl FnMut(ProgressEntryId, &mut Progress, &mut HiddenProgress),
    ) {
        let mut inner = self.inner.lock();
        for (k, v) in inner.entries.iter_mut() {
            f(ProgressEntryId(*k), &mut v.0, &mut v.1);
        }
    }

    /// Check if there is any progress data stored for a given ID.
    pub fn contains_id(&self, id: ProgressEntryId) -> bool {
        self.inner.lock().entries.contains_key(&id.0)
    }

    /// Check if all progress is complete.
    ///
    /// This accounts for both visible progress and hidden progress.
    pub fn is_ready(&self) -> bool {
        self.get_global_combined_progress().is_ready()
    }

    /// Check if the progress for a specific ID is complete.
    ///
    /// This accounts for both visible progress and hidden progress.
    pub fn is_id_ready(&self, id: ProgressEntryId) -> bool {
        let inner = self.inner.lock();
        inner
            .entries
            .get(&id.0)
            .map(|x| (x.0 + x.1 .0).is_ready())
            .unwrap_or_default()
    }

    /// Get the overall visible progress.
    ///
    /// This is what you should use to display a progress bar or
    /// other user-facing indicator.
    pub fn get_global_progress(&self) -> Progress {
        let inner = self.inner.lock();
        inner.accum.0
    }

    /// Get the overall hidden progress.
    pub fn get_global_hidden_progress(&self) -> HiddenProgress {
        let inner = self.inner.lock();
        inner.accum.1
    }

    /// Get the overall visible+hidden progress.
    ///
    /// This is what you should use to determine if all work is complete.
    pub fn get_global_combined_progress(&self) -> Progress {
        let inner = self.inner.lock();
        inner.accum.0 + inner.accum.1 .0
    }

    /// Get the visible progress stored for a specific ID.
    pub fn get_progress(&self, id: ProgressEntryId) -> Progress {
        let inner = self.inner.lock();
        inner.entries.get(&id.0).copied().unwrap_or_default().0
    }

    /// Get the hidden progress stored for a specific ID.
    pub fn get_hidden_progress(&self, id: ProgressEntryId) -> HiddenProgress {
        let inner = self.inner.lock();
        inner.entries.get(&id.0).copied().unwrap_or_default().1
    }

    /// Get the visible+hidden progress stored for a specific ID.
    pub fn get_combined_progress(&self, id: ProgressEntryId) -> Progress {
        let inner = self.inner.lock();
        inner
            .entries
            .get(&id.0)
            .map(|x| x.0 + x.1 .0)
            .unwrap_or_default()
    }

    /// Get the (visible) expected work item count for a specific ID.
    pub fn get_total(&self, id: ProgressEntryId) -> u32 {
        let inner = self.inner.lock();
        inner
            .entries
            .get(&id.0)
            .copied()
            .unwrap_or_default()
            .0
            .total
    }

    /// Get the (visible) completed work item count for a specific ID.
    pub fn get_done(&self, id: ProgressEntryId) -> u32 {
        let inner = self.inner.lock();
        inner.entries.get(&id.0).copied().unwrap_or_default().0.done
    }

    /// Get the (hidden) expected work item count for a specific ID.
    pub fn get_hidden_total(&self, id: ProgressEntryId) -> u32 {
        let inner = self.inner.lock();
        inner
            .entries
            .get(&id.0)
            .copied()
            .unwrap_or_default()
            .1
            .total
    }

    /// Get the (hidden) completed work item count for a specific ID.
    pub fn get_hidden_done(&self, id: ProgressEntryId) -> u32 {
        let inner = self.inner.lock();
        inner.entries.get(&id.0).copied().unwrap_or_default().1.done
    }

    /// Overwrite the stored visible progress for a specific ID.
    ///
    /// Use this when you want to overwrite both the `total` and `done` at once.
    pub fn set_progress(&self, id: ProgressEntryId, done: u32, total: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            if p.0.total < total {
                let diff = total - p.0.total;
                inner.accum.0.total += diff;
            }
            if p.0.total > total {
                let diff = p.0.total - total;
                inner.accum.0.total -= diff;
            }
            if p.0.done < done {
                let diff = done - p.0.done;
                inner.accum.0.done += diff;
            }
            if p.0.done > done {
                let diff = p.0.done - done;
                inner.accum.0.done -= diff;
            }
            p.0 = Progress { done, total };
        } else {
            inner.entries.insert(
                id.0,
                (Progress { done, total }, HiddenProgress::default()),
            );
            inner.accum.0.total += total;
            inner.accum.0.done += done;
        }
    }

    /// Overwrite the stored hidden progress for a specific ID.
    ///
    /// Use this when you want to overwrite both the `total` and `done` at once.
    pub fn set_hidden_progress(
        &self,
        id: ProgressEntryId,
        done: u32,
        total: u32,
    ) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            if p.1.total < total {
                let diff = total - p.1.total;
                inner.accum.1.total += diff;
            }
            if p.1.total > total {
                let diff = p.1.total - total;
                inner.accum.1.total -= diff;
            }
            if p.1.done < done {
                let diff = done - p.1.done;
                inner.accum.1.done += diff;
            }
            if p.1.done > done {
                let diff = p.1.done - done;
                inner.accum.1.done -= diff;
            }
            p.1 = Progress { done, total }.into();
        } else {
            inner.entries.insert(
                id.0,
                (Progress::default(), Progress { done, total }.into()),
            );
            inner.accum.1.total += total;
            inner.accum.1.done += done;
        }
    }

    /// Overwrite the stored (visible) expected work items for a specific ID.
    pub fn set_total(&self, id: ProgressEntryId, total: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            if p.0.total < total {
                let diff = total - p.0.total;
                inner.accum.0.total += diff;
            }
            if p.0.total > total {
                let diff = p.0.total - total;
                inner.accum.0.total -= diff;
            }
            p.0.total = total;
        } else {
            inner.entries.insert(
                id.0,
                (Progress { done: 0, total }, HiddenProgress::default()),
            );
            inner.accum.0.total += total;
        }
    }

    /// Overwrite the stored (visible) completed work items for a specific ID.
    pub fn set_done(&self, id: ProgressEntryId, done: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            if p.0.done < done {
                let diff = done - p.0.done;
                inner.accum.0.done += diff;
            }
            if p.0.done > done {
                let diff = p.0.done - done;
                inner.accum.0.done -= diff;
            }
            p.0.done = done;
        } else {
            inner.entries.insert(
                id.0,
                (Progress { done, total: 0 }, HiddenProgress::default()),
            );
            inner.accum.0.done += done;
        }
    }

    /// Overwrite the stored (hidden) expected work items for a specific ID.
    pub fn set_hidden_total(&self, id: ProgressEntryId, total: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            if p.1.total < total {
                let diff = total - p.1.total;
                inner.accum.1.total += diff;
            }
            if p.1.total > total {
                let diff = p.1.total - total;
                inner.accum.1.total -= diff;
            }
            p.1.total = total;
        } else {
            inner.entries.insert(
                id.0,
                (Progress::default(), Progress { done: 0, total }.into()),
            );
            inner.accum.1.total += total;
        }
    }

    /// Overwrite the stored (hidden) completed work items for a specific ID.
    pub fn set_hidden_done(&self, id: ProgressEntryId, done: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            if p.1.done < done {
                let diff = done - p.1.done;
                inner.accum.1.done += diff;
            }
            if p.1.done > done {
                let diff = p.1.done - done;
                inner.accum.1.done -= diff;
            }
            p.1.done = done;
        } else {
            inner.entries.insert(
                id.0,
                (Progress::default(), Progress { done, total: 0 }.into()),
            );
            inner.accum.1.done += done;
        }
    }

    /// Add more (visible) work items to the previously stored progress for a
    /// specific ID.
    ///
    /// Use this when you want to add to both the `total` and `done` at once.
    pub fn add_progress(&self, id: ProgressEntryId, done: u32, total: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            p.0.done += done;
            p.0.total += total;
        } else {
            inner.entries.insert(
                id.0,
                (Progress { done, total }, HiddenProgress::default()),
            );
        }
        inner.accum.0.total += total;
        inner.accum.0.done += done;
    }

    /// Add more (visible) expected work items to the previously stored value
    /// for a specific ID.
    pub fn add_total(&self, id: ProgressEntryId, total: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            p.0.total += total;
        } else {
            inner.entries.insert(
                id.0,
                (Progress { done: 0, total }, HiddenProgress::default()),
            );
        }
        inner.accum.0.total += total;
    }

    /// Add more (visible) completed work items to the previously stored value
    /// for a specific ID.
    pub fn add_done(&self, id: ProgressEntryId, done: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            p.0.done += done;
        } else {
            inner.entries.insert(
                id.0,
                (Progress { done, total: 0 }, HiddenProgress::default()),
            );
        }
        inner.accum.0.done += done;
    }

    /// Add more (hidden) work items to the previously stored progress for a
    /// specific ID.
    ///
    /// Use this when you want to add to both the `total` and `done` at once.
    pub fn add_hidden_progress(
        &self,
        id: ProgressEntryId,
        done: u32,
        total: u32,
    ) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            p.1.done += done;
            p.1.total += total;
        } else {
            inner.entries.insert(
                id.0,
                (Progress::default(), Progress { done, total }.into()),
            );
        }
        inner.accum.1.total += total;
        inner.accum.1.done += done;
    }

    /// Add more (hidden) expected work items to the previously stored value for
    /// a specific ID.
    pub fn add_hidden_total(&self, id: ProgressEntryId, total: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            p.1.total += total;
        } else {
            inner.entries.insert(
                id.0,
                (Progress::default(), Progress { done: 0, total }.into()),
            );
        }
        inner.accum.1.total += total;
    }

    /// Add more (hidden) completed work items to the previously stored value
    /// for a specific ID.
    pub fn add_hidden_done(&self, id: ProgressEntryId, done: u32) {
        let inner = &mut *self.inner.lock();
        if let Some(p) = inner.entries.get_mut(&id.0) {
            p.1.done += done;
        } else {
            inner.entries.insert(
                id.0,
                (Progress::default(), Progress { done, total: 0 }.into()),
            );
        }
        inner.accum.1.done += done;
    }
}

/// Because we don't want to impl Default for ProgressEntryId, to prevent user
/// footguns.
struct ProgressEntryIdWrapper(ProgressEntryId);

impl Default for ProgressEntryIdWrapper {
    fn default() -> Self {
        Self(ProgressEntryId::new())
    }
}

/// System param to manage a progress entry in the [`ProgressTracker`].
///
/// You can use this in your systems to report progress to be tracked.
///
/// Each instance of this system param will create an entry in the
/// [`ProgressTracker`] for itself and allow you to access the
/// associated value. The ID is managed internally.
#[derive(SystemParam)]
pub struct ProgressEntry<'w, 's, S: FreelyMutableState> {
    global: Res<'w, ProgressTracker<S>>,
    my_id: Local<'s, ProgressEntryIdWrapper>,
}

impl<S: FreelyMutableState> ProgressEntry<'_, '_, S> {
    /// Get the ID of the [`ProgressTracker`] entry managed by this system param
    pub fn id(&self) -> ProgressEntryId {
        self.my_id.0
    }

    /// Get the overall visible progress.
    ///
    /// This is what you should use to display a progress bar or
    /// other user-facing indicator.
    pub fn get_global_progress(&self) -> Progress {
        self.global.get_global_progress()
    }

    /// Get the overall hidden progress.
    pub fn get_global_hidden_progress(&self) -> HiddenProgress {
        self.global.get_global_hidden_progress()
    }

    /// Get the overall visible+hidden progress.
    ///
    /// This is what you should use to determine if all work is complete.
    pub fn get_global_combined_progress(&self) -> Progress {
        self.global.get_global_combined_progress()
    }

    /// Check if everything is ready.
    pub fn is_global_ready(&self) -> bool {
        self.global.is_ready()
    }

    /// Check if the progress associated with this system param is ready.
    pub fn is_ready(&self) -> bool {
        self.global.is_id_ready(self.my_id.0)
    }

    /// Get the visible+hidden progress associated with this system param.
    pub fn get_combined_progress(&self) -> Progress {
        self.global.get_combined_progress(self.my_id.0)
    }

    /// Get the visible progress associated with this system param.
    pub fn get_progress(&self) -> Progress {
        self.global.get_progress(self.my_id.0)
    }

    /// Get the (visible) expected work items associated with this system param.
    pub fn get_total(&self) -> u32 {
        self.global.get_total(self.my_id.0)
    }

    /// Get the (visible) completed work items associated with this system
    /// param.
    pub fn get_done(&self) -> u32 {
        self.global.get_done(self.my_id.0)
    }

    /// Overwrite the visible progress associated with this system param.
    ///
    /// Use this if you want to set both the `done` and `total` at once.
    pub fn set_progress(&self, done: u32, total: u32) {
        self.global.set_progress(self.my_id.0, done, total)
    }

    /// Overwrite the (visible) expected work items associated with this system
    /// param.
    pub fn set_total(&self, total: u32) {
        self.global.set_total(self.my_id.0, total)
    }

    /// Overwrite the (visible) completed work items associated with this system
    /// param.
    pub fn set_done(&self, done: u32) {
        self.global.set_done(self.my_id.0, done)
    }

    /// Add to the visible progress associated with this system param.
    ///
    /// Use this if you want to add to both the `done` and `total` at once.
    pub fn add_progress(&self, done: u32, total: u32) {
        self.global.add_progress(self.my_id.0, done, total)
    }

    /// Add more (visible) expected work items associated with this system
    /// param.
    pub fn add_total(&self, total: u32) {
        self.global.add_total(self.my_id.0, total)
    }

    /// Add more (visible) completed work items associated with this system
    /// param.
    pub fn add_done(&self, done: u32) {
        self.global.add_done(self.my_id.0, done)
    }

    /// Get the hidden progress associated with this system param.
    pub fn get_hidden_progress(&self) -> HiddenProgress {
        self.global.get_hidden_progress(self.my_id.0)
    }

    /// Get the (hidden) expected work items associated with this system param.
    pub fn get_hidden_total(&self) -> u32 {
        self.global.get_hidden_total(self.my_id.0)
    }

    /// Get the (hidden) completed work items associated with this system param.
    pub fn get_hidden_done(&self) -> u32 {
        self.global.get_hidden_done(self.my_id.0)
    }

    /// Overwrite the hidden progress associated with this system param.
    ///
    /// Use this if you want to set both the `done` and `total` at once.
    pub fn set_hidden_progress(&self, done: u32, total: u32) {
        self.global.set_hidden_progress(self.my_id.0, done, total)
    }

    /// Overwrite the (hidden) expected work items associated with this system
    /// param.
    pub fn set_hidden_total(&self, total: u32) {
        self.global.set_hidden_total(self.my_id.0, total)
    }

    /// Overwrite the (hidden) completed work items associated with this system
    /// param.
    pub fn set_hidden_done(&self, done: u32) {
        self.global.set_hidden_done(self.my_id.0, done)
    }

    /// Add to the hidden progress associated with this system param.
    ///
    /// Use this if you want to add to both the `done` and `total` at once.
    pub fn add_hidden_progress(&self, done: u32, total: u32) {
        self.global.add_hidden_progress(self.my_id.0, done, total)
    }

    /// Add more (hidden) expected work items associated with this system param.
    pub fn add_hidden_total(&self, total: u32) {
        self.global.add_hidden_total(self.my_id.0, total)
    }

    /// Add more (hidden) completed work items associated with this system
    /// param.
    pub fn add_hidden_done(&self, done: u32) {
        self.global.add_hidden_done(self.my_id.0, done)
    }
}

pub(crate) trait ApplyProgress: Sized {
    fn apply_progress<S: FreelyMutableState>(
        self,
        tracker: &ProgressTracker<S>,
        id: ProgressEntryId,
    );
}

impl ApplyProgress for Progress {
    fn apply_progress<S: FreelyMutableState>(
        self,
        tracker: &ProgressTracker<S>,
        id: ProgressEntryId,
    ) {
        tracker.set_progress(id, self.done, self.total);
    }
}

impl ApplyProgress for HiddenProgress {
    fn apply_progress<S: FreelyMutableState>(
        self,
        tracker: &ProgressTracker<S>,
        id: ProgressEntryId,
    ) {
        tracker.set_hidden_progress(id, self.0.done, self.0.total);
    }
}

impl<T1: ApplyProgress, T2: ApplyProgress> ApplyProgress for (T1, T2) {
    fn apply_progress<S: FreelyMutableState>(
        self,
        tracker: &ProgressTracker<S>,
        id: ProgressEntryId,
    ) {
        self.0.apply_progress(tracker, id);
        self.1.apply_progress(tracker, id);
    }
}
