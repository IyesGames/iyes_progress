use bevy_ecs::prelude::*;
use bevy_state::state::FreelyMutableState;

use crate::prelude::*;

pub(crate) type Sender =
    crossbeam_channel::Sender<(ProgressEntryId, ProgressMessage)>;
pub(crate) type Receiver =
    crossbeam_channel::Receiver<(ProgressEntryId, ProgressMessage)>;

/// A "handle" to send progress updates from a background thread or async task.
///
/// To create an instance of this struct, call [`ProgressTracker::new_async_entry`].
///
/// Each instance of this struct represents a [`ProgressEntryId`] in the
/// [`ProgressTracker<S>`] resource. If you clone it, you create another
/// instance using the same [`ProgressEntryId`].
///
/// When you call the various methods on this struct to update your progress
/// entry, a message will be sent via an internal channel. A system running
/// in `PreUpdate` will read these messages and actually update the entry
/// in the [`ProgressTracker`].
#[derive(Clone)]
pub struct ProgressSender {
    pub(crate) id: ProgressEntryId,
    pub(crate) sender: Sender,
}

impl ProgressSender {
    /// Get the [`ProgressEntryId`] associated with this [`ProgressSender`].
    pub fn id(&self) -> ProgressEntryId {
        self.id
    }

    fn msg(&self, msg: ProgressMessage) {
        self.sender.try_send((self.id, msg)).ok();
    }

    /// Set the visible progress.
    pub fn set_progress(&self, done: u32, total: u32) {
        self.msg(ProgressMessage::SetProgress(done, total));
    }

    /// Set the hidden progress.
    pub fn set_hidden_progress(&self, done: u32, total: u32) {
        self.msg(ProgressMessage::SetHiddenProgress(done, total));
    }

    /// Set the visible expected units of work.
    pub fn set_total(&self, total: u32) {
        self.msg(ProgressMessage::SetTotal(total));
    }

    /// Set the visible completed units of work.
    pub fn set_done(&self, done: u32) {
        self.msg(ProgressMessage::SetDone(done));
    }

    /// Set the hidden expected units of work.
    pub fn set_hidden_total(&self, total: u32) {
        self.msg(ProgressMessage::SetHiddenTotal(total));
    }

    /// Set the hidden completed units of work.
    pub fn set_hidden_done(&self, done: u32) {
        self.msg(ProgressMessage::SetHiddenDone(done));
    }

    /// Add to the visible progress.
    pub fn add_progress(&self, done: u32, total: u32) {
        self.msg(ProgressMessage::AddProgress(done, total));
    }

    /// Add to the hidden progress.
    pub fn add_hidden_progress(&self, done: u32, total: u32) {
        self.msg(ProgressMessage::AddHiddenProgress(done, total));
    }

    /// Add to the visible expected units of work.
    pub fn add_total(&self, total: u32) {
        self.msg(ProgressMessage::AddTotal(total));
    }

    /// Add to the visible completed units of work.
    pub fn add_done(&self, done: u32) {
        self.msg(ProgressMessage::AddDone(done));
    }

    /// Add to the hidden expected units of work.
    pub fn add_hidden_total(&self, total: u32) {
        self.msg(ProgressMessage::AddHiddenTotal(total));
    }

    /// Add to the hidden completed units of work.
    pub fn add_hidden_done(&self, done: u32) {
        self.msg(ProgressMessage::AddHiddenDone(done));
    }
}

pub(crate) enum ProgressMessage {
    SetProgress(u32, u32),
    SetHiddenProgress(u32, u32),
    SetTotal(u32),
    SetDone(u32),
    SetHiddenTotal(u32),
    SetHiddenDone(u32),
    AddProgress(u32, u32),
    AddHiddenProgress(u32, u32),
    AddTotal(u32),
    AddDone(u32),
    AddHiddenTotal(u32),
    AddHiddenDone(u32),
}

pub(crate) fn rc_recv_progress_msgs<S: FreelyMutableState>(
    tracker: Res<ProgressTracker<S>>,
) -> bool {
    tracker.chan.is_some()
}

pub(crate) fn recv_progress_msgs<S: FreelyMutableState>(
    tracker: Res<ProgressTracker<S>>,
) {
    let Some((_, rx)) = &tracker.chan else {
        return;
    };
    rx.try_iter().for_each(|msg| match msg.1 {
        ProgressMessage::SetProgress(done, total) => {
            tracker.set_progress(msg.0, done, total);
        }
        ProgressMessage::SetHiddenProgress(done, total) => {
            tracker.set_hidden_progress(msg.0, done, total);
        }
        ProgressMessage::SetTotal(total) => {
            tracker.set_total(msg.0, total);
        }
        ProgressMessage::SetDone(done) => {
            tracker.set_done(msg.0, done);
        }
        ProgressMessage::SetHiddenTotal(total) => {
            tracker.set_hidden_total(msg.0, total);
        }
        ProgressMessage::SetHiddenDone(done) => {
            tracker.set_hidden_done(msg.0, done);
        }
        ProgressMessage::AddProgress(done, total) => {
            tracker.add_progress(msg.0, done, total);
        }
        ProgressMessage::AddHiddenProgress(done, total) => {
            tracker.add_hidden_progress(msg.0, done, total);
        }
        ProgressMessage::AddTotal(total) => {
            tracker.add_total(msg.0, total);
        }
        ProgressMessage::AddDone(done) => {
            tracker.add_done(msg.0, done);
        }
        ProgressMessage::AddHiddenTotal(total) => {
            tracker.add_hidden_total(msg.0, total);
        }
        ProgressMessage::AddHiddenDone(done) => {
            tracker.add_hidden_done(msg.0, done);
        }
    });
}
