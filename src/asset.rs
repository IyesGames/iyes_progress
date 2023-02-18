use bevy::asset::{HandleId, LoadState};
use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::Progress;

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
    handles: HashSet<HandleId>,
    total: u32,
}

impl AssetsLoading {
    /// Add an asset to be tracked
    pub fn add<T: Into<HandleId>>(&mut self, handle: T) {
        self.handles.insert(handle.into());
        self.total += 1;
    }

    /// Have all assets finished loading?
    pub fn is_ready(&self) -> bool {
        self.handles.is_empty()
    }
}

pub(crate) fn assets_progress(
    mut loading: ResMut<AssetsLoading>,
    server: Res<AssetServer>,
) -> Progress {
    // TODO: avoid this temporary vec (HashSet::drain_filter is in Rust nightly)
    let mut done = vec![];
    for handle in loading.handles.iter() {
        let loadstate = server.get_load_state(*handle);
        if loadstate == LoadState::Loaded || loadstate == LoadState::Failed {
            done.push(*handle);
        }
    }
    for handle in done {
        loading.handles.remove(&handle);
    }

    Progress {
        done: loading.total - loading.handles.len() as u32,
        total: loading.total,
    }
}

pub(crate) fn assets_loading_reset(mut loading: ResMut<AssetsLoading>) {
    *loading = AssetsLoading::default();
}
