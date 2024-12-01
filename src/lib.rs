//! Progress Tracking Helper Crate
//!
//! This crate helps you in cases where you need to track when a bunch of
//! work has been completed and perform a state transition.
//!
//! The most typical use case are loading screens, where you might need to
//! load assets, prepare the game world, etc… and then transition to the
//! in-game state when everything is done.
//!
//! To use this plugin, add one or more instances
//! [`ProgressPlugin<S>`] to your
//! `App`, configuring for the relevant states.
//!
//! ```rust
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .init_state::<MyStates>()
//!         .add_plugins(
//!             ProgressPlugin::<MyStates>::new()
//!                 .with_state_transition(MyStates::Loading, MyStates::Done),
//!         )
//!         // ...
//!         .run();
//! ```
//!
//! You can have any number of systems doing different things during
//! your loading state, and they can report their progress to this crate.
//!
//! This can be done in several different ways. Use whichever is convenient.
//!
//!  - Using the special [`ProgressEntry`] system param
//!  - By returning [`Progress`], [`HiddenProgress`], or a tuple of the two
//!    - Add such systems to your app by calling `.track_progress::<S>()` or
//!      `.track_progress_and_stop::<S>()` to add a run condition so they stop
//!      running after they return full progress.
//!  - Manually, by creating a [`ProgressEntryId`] and updating the values
//!    stored in the [`ProgressTracker<S>`] resource.

#![warn(missing_docs)]

/// All the public API offered by this crate
pub mod prelude {
    #[cfg(feature = "assets")]
    pub use crate::assets::*;
    #[cfg(feature = "debug")]
    pub use crate::debug::*;
    pub use crate::plugin::*;
    pub use crate::progress::*;
    pub use crate::state::*;
    pub use crate::system::*;
    pub use crate::tracker::*;
    pub use crate::utils::*;
}

pub use crate::prelude::*;

#[cfg(feature = "assets")]
mod assets;
#[cfg(feature = "debug")]
mod debug;
mod plugin;
mod progress;
mod state;
mod system;
mod tracker;
mod utils;
