use bevy_asset::prelude::*;
use bevy_asset::UntypedAssetId;
use bevy_asset::LoadState;
use bevy_asset::RecursiveDependencyLoadState;
use bevy_ecs::prelude::*;
use bevy_utils::HashSet;

use crate::Progress;

/// System Set for assets progress tracking.
///
/// You can use this set for system ordering, if any of your systems need to
/// run before/after [`AssetsLoading`] is checked every frame. For example, if
/// you need to add more handles to track, your system should run before this.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetsTrackProgress;

/// Resource for tracking the loading of assets
///
/// You must `.add(&handle)` all of your assets here, if you want to wait
/// for their loading to complete before transitioning to the next app state.
/// You should do this during `on_enter` for the loading state (or earlier).
///
/// Note that failed/errored/unloaded assets are counted as completed.
/// Otherwise, your game could get stuck on the loading screen.
///
/// This resource is not added/removed when entering/exiting the load state.
/// It is initialized with the app, so that it is available for you to add
/// your asset handles before the load state becomes active.
/// On exiting the load state, its value is simply cleared/reset.
#[derive(Resource)]
pub struct AssetsLoading {
    pending: HashSet<UntypedAssetId>,
    done: HashSet<UntypedAssetId>,
    /// Should we count assets that failed to load as progress?
    /// Warning: if this is false, you may freeze in your loading state
    /// if there are any errors. Defaults to true.
    pub allow_failures: bool,
    /// Should we check the status of asset dependencies?
    /// Defaults to true.
    pub track_dependencies: bool,
}

impl Default for AssetsLoading {
    fn default() -> Self {
        AssetsLoading {
            pending: Default::default(),
            done: Default::default(),
            allow_failures: true,
            track_dependencies: true,
        }
    }
}

impl AssetsLoading {
    /// Add an asset to be tracked
    pub fn add<T: Into<UntypedAssetId>>(&mut self, handle: T) {
        let asset_id = handle.into();
        if !self.done.contains(&asset_id) {
            self.pending.insert(asset_id);
        }
    }

    /// Have all assets finished loading?
    pub fn is_ready(&self) -> bool {
        self.pending.is_empty()
    }
}

pub(crate) fn assets_progress(
    mut loading: ResMut<AssetsLoading>,
    server: Res<AssetServer>,
) -> Progress {
    let mut any_changed = false;
    {
        let loading = loading.bypass_change_detection();
        loading.pending.retain(|aid| {
            let loaded = server.load_state(*aid);
            let ready = if loaded == LoadState::Loaded {
                if loading.track_dependencies {
                    let loaded_deps = server.recursive_dependency_load_state(*aid);
                    if loading.allow_failures && loaded_deps == RecursiveDependencyLoadState::Failed {
                        true
                    } else {
                        loaded_deps == RecursiveDependencyLoadState::Loaded
                    }
                } else {
                    true
                }
            } else if loading.allow_failures && loaded == LoadState::Failed {
                true
            } else {
                false
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

pub(crate) fn assets_loading_reset(mut loading: ResMut<AssetsLoading>) {
    *loading = AssetsLoading::default();
}
