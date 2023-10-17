use bevy_asset::prelude::*;
use bevy_asset::UntypedAssetId;
use bevy_asset::LoadState;
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
#[derive(Resource, Default)]
pub struct AssetsLoading {
    pending: HashSet<UntypedAssetId>,
    done: HashSet<UntypedAssetId>,
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
    // TODO: avoid this temporary vec (HashSet::drain_filter is in Rust nightly)
    let mut done = vec![];
    for handle in loading.pending.iter() {
        if let Some(load_state) = server.get_load_state(*handle) {
            if load_state == LoadState::Loaded || load_state == LoadState::Failed {
                done.push(*handle);
            }
        }
    }
    for handle in done {
        loading.pending.remove(&handle);
        loading.done.insert(handle);
    }

    Progress {
        done: loading.done.len() as u32,
        total: loading.done.len() as u32 + loading.pending.len() as u32,
    }
}

pub(crate) fn assets_loading_reset(mut loading: ResMut<AssetsLoading>) {
    *loading = AssetsLoading::default();
}
