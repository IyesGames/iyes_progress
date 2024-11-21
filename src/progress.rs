use derive_more::derive::{Add, AddAssign, Deref, DerefMut, Sub, SubAssign};

/// Represents the progress that is being tracked.
///
/// It indicates how much work has been completed and how much is left to do.
///
/// When the value of `done` reaches the value of `total`, it is considered
/// "ready".
///
/// For your convenience, you can easily convert `bool`s into this type.
/// You can also convert `Progress` values into floats in the `0.0..=1.0` range.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[derive(Add, AddAssign, Sub, SubAssign)]
pub struct Progress {
    /// The units of work that have been completed.
    pub done: u32,
    /// The total units of work expected.
    pub total: u32,
}

impl From<bool> for Progress {
    fn from(b: bool) -> Progress {
        Progress {
            total: 1,
            done: b as u32,
        }
    }
}

impl From<Progress> for f32 {
    fn from(p: Progress) -> f32 {
        p.done as f32 / p.total as f32
    }
}

impl From<Progress> for f64 {
    fn from(p: Progress) -> f64 {
        p.done as f64 / p.total as f64
    }
}

impl Progress {
    /// Returns true if `done` has reached `total`
    pub fn is_ready(self) -> bool {
        self.done >= self.total
    }
}

/// Represents progress that is intended to be "hidden" from the user.
///
/// Such progress must be completed in order to advance state (or generally
/// consider everything to be ready), but is not intended to be shown in UI
/// progress bars or other user-facing progress indicators.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[derive(Add, AddAssign, Sub, SubAssign)]
#[derive(Deref, DerefMut)]
pub struct HiddenProgress(pub Progress);

impl From<Progress> for HiddenProgress {
    fn from(value: Progress) -> Self {
        Self(value)
    }
}

impl From<HiddenProgress> for Progress {
    fn from(value: HiddenProgress) -> Self {
        value.0
    }
}

impl From<bool> for HiddenProgress {
    fn from(b: bool) -> HiddenProgress {
        Progress::from(b).into()
    }
}

impl From<HiddenProgress> for f32 {
    fn from(p: HiddenProgress) -> f32 {
        f32::from(p.0)
    }
}

impl From<HiddenProgress> for f64 {
    fn from(p: HiddenProgress) -> f64 {
        f64::from(p.0)
    }
}
