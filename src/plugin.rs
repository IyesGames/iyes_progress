use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy_state::prelude::*;
use bevy_state::state::FreelyMutableState;

use crate::prelude::ProgressTracker;
use crate::state::*;
use crate::ProgressReturningSystem;

/// Add this plugin to enable progress tracking for your states type.
///
/// This plugin will set up everything necessary to track progress in
/// states of the given type.
///
/// ```rust
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .init_state::<MyStates>()
///         .add_plugins(
///             ProgressPlugin::<MyStates>::new()
///                 .with_state_transition(MyStates::Loading, MyStates::Done),
///         )
///         // ...
///         .run();
/// ```
pub struct ProgressPlugin<S: FreelyMutableState> {
    transitions: StateTransitionConfig<S>,
    check_progress_schedule: InternedScheduleLabel,
    #[cfg(feature = "assets")]
    track_assets: bool,
}

/// This set represents the "check progress and transition state if ready" step.
/// It is only useful in the schedule where progress checking occurs (`Last` by
/// default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct CheckProgressSet;

impl<S: FreelyMutableState> Default for ProgressPlugin<S> {
    fn default() -> Self {
        Self {
            check_progress_schedule: Last.intern(),
            transitions: Default::default(),
            #[cfg(feature = "assets")]
            track_assets: false,
        }
    }
}

impl<S: FreelyMutableState> ProgressPlugin<S> {
    /// Create a new instance of this plugin.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure progress tracking in a specific state.
    ///
    /// (Mutable method variant)
    ///
    /// When the `from` state is entered, all values stored in the
    /// [`ProgressTracker<S>`] resource will be cleared.
    ///
    /// When all the progress is complete, a state transition to the
    /// `to` state will be queued automatically.
    pub fn add_state_transition(&mut self, from: S, to: S) {
        self.transitions.map_from_to.insert(from, to);
    }

    /// Configure progress tracking in a specific state.
    ///
    /// (Builder variant)
    ///
    /// When the `from` state is entered, all values stored in the
    /// [`ProgressTracker<S>`] resource will be cleared.
    ///
    /// When all the progress is complete, a state transition to the
    /// `to` state will be queued automatically.
    pub fn with_state_transition(mut self, from: S, to: S) -> Self {
        self.add_state_transition(from, to);
        self
    }

    /// Configure in which schedule to check the global progress and queue state
    /// transitions.
    ///
    /// Default: `Last`
    pub fn check_progress_in<L: ScheduleLabel>(mut self, schedule: L) -> Self {
        self.check_progress_schedule = schedule.intern();
        self
    }

    /// Set whether the built-in asset tracking should be enabled.
    #[cfg(feature = "assets")]
    pub fn set_asset_tracking(&mut self, asset_tracking: bool) {
        self.track_assets = asset_tracking;
    }

    /// Enable the built-in asset tracking feature.
    #[cfg(feature = "assets")]
    pub fn with_asset_tracking(mut self) -> Self {
        self.track_assets = true;
        self
    }
}

impl<S: FreelyMutableState> Plugin for ProgressPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProgressTracker<S>>();
        app.insert_resource(self.transitions.clone());
        app.add_systems(
            self.check_progress_schedule,
            transition_if_ready::<S>
                .run_if(rc_configured_state::<S>)
                .in_set(CheckProgressSet),
        );
        for s in self.transitions.map_from_to.keys() {
            app.add_systems(OnEnter(s.clone()), clear_global_progress::<S>);
        }
        #[cfg(feature = "debug")]
        {
            use crate::debug::*;
            app.add_systems(
                self.check_progress_schedule,
                debug_progress::<S>
                    .run_if(rc_debug_progress::<S>)
                    .in_set(CheckProgressSet)
                    .before(transition_if_ready::<S>),
            );
        }
        #[cfg(feature = "assets")]
        if self.track_assets {
            use crate::assets::*;
            app.init_resource::<AssetsLoading<S>>();
            app.add_systems(
                PostUpdate,
                assets_progress::<S>
                    .track_progress::<S>()
                    .in_set(AssetsTrackProgress)
                    .run_if(rc_configured_state::<S>),
            );
            for s in self.transitions.map_from_to.keys() {
                app.add_systems(OnExit(s.clone()), assets_loading_reset::<S>);
            }
        }
    }
}
