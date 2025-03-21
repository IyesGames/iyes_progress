use std::marker::PhantomData;

use bevy_asset::prelude::*;
use bevy_asset::{LoadState, UntypedAssetId};
use bevy_ecs::prelude::*;
use bevy_state::state::FreelyMutableState;
use bevy_platform_support::collections::HashSet;

use crate::prelude::*;

/// System Set for assets progress tracking.
///
/// You can use this set for system ordering, if any of your systems need to
/// run before/after [`AssetsLoading`] is checked every frame. For example, if
/// you need to add more handles to track, your system should run before this.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetsTrackProgress;

/// Resource for tracking the loading of assets
///
/// Note: to use this, you have to call
/// [`.with_asset_tracking`](ProgressPlugin::with_asset_tracking)
/// when creating your [`ProgressPlugin`].
///
/// You can add asset handles here, and we will check their status for you
/// and record loading progress.
///
/// Note that failed/errored/unloaded assets are counted as completed by
/// default. Otherwise, your game could get stuck on the loading screen.
///
/// This resource should not be removed.
#[derive(Resource)]
pub struct AssetsLoading<S: FreelyMutableState> {
    pending: HashSet<UntypedAssetId>,
    done: HashSet<UntypedAssetId>,
    /// Should we count assets that failed to load as progress?
    /// Warning: if this is false, you may freeze in your loading state
    /// if there are any errors. Defaults to true.
    pub allow_failures: bool,
    /// Should we check the status of asset dependencies?
    /// Defaults to true.
    pub track_dependencies: bool,
    _pd: PhantomData<S>,
}

impl<S: FreelyMutableState> Default for AssetsLoading<S> {
    fn default() -> Self {
        AssetsLoading {
            pending: Default::default(),
            done: Default::default(),
            allow_failures: true,
            track_dependencies: true,
            _pd: PhantomData,
        }
    }
}

impl<S: FreelyMutableState> AssetsLoading<S> {
    /// Add an asset to be tracked
    pub fn add<T: Into<UntypedAssetId>>(&mut self, handle: T) {
        let asset_id = handle.into();
        if !self.done.contains(&asset_id) {
            self.pending.insert(asset_id);
        }
    }

    /// Have all tracked assets finished loading?
    pub fn is_ready(&self) -> bool {
        self.pending.is_empty()
    }
}

pub(crate) fn assets_progress<S: FreelyMutableState>(
    mut loading: ResMut<AssetsLoading<S>>,
    server: Res<AssetServer>,
) -> Progress {
    let mut any_changed = false;
    {
        let loading = loading.bypass_change_detection();
        loading.pending.retain(|aid| {
            let loaded = server.load_state(*aid);
            let ready = match loaded {
                LoadState::NotLoaded => true,
                LoadState::Loading => false,
                LoadState::Loaded => {
                    if loading.track_dependencies {
                        let loaded_deps =
                            server.recursive_dependency_load_state(*aid);
                        if loading.allow_failures && loaded_deps.is_failed() {
                            true
                        } else {
                            loaded_deps.is_loaded()
                        }
                    } else {
                        true
                    }
                }
                LoadState::Failed(_) => loading.allow_failures,
            };
            if ready {
                loading.done.insert(*aid);
                any_changed = true;
            }
            !ready
        });
    }
    if any_changed {
        loading.set_changed();
    }

    Progress {
        done: loading.done.len() as u32,
        total: loading.done.len() as u32 + loading.pending.len() as u32,
    }
}

/// This system clears the [`AssetsLoading<S>`] resource.
///
/// This will be automatically added to the `OnEnter`/`OnExit`
/// schedules of progress-tracked states, if so configured
/// by the [`ProgressPlugin`].
///
/// This `fn` is `pub` so you can order your systems around it.
/// Or add other "clearing points" to your app.
pub fn assets_loading_reset<S: FreelyMutableState>(
    mut loading: ResMut<AssetsLoading<S>>,
) {
    *loading = AssetsLoading::default();
}
