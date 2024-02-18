#![allow(dead_code)]

use bevy::prelude::*;
use iyes_progress::{prelude::*, dummy_system_wait_frames, dummy_system_wait_millis};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, States)]
enum AppState {
    #[default]
    Loading,
    MainMenu,
}

fn main() {
    App::new()
        // Init bevy
        .add_plugins(DefaultPlugins)
        // Add our state type
        .init_state::<AppState>()
        .add_plugins(
            ProgressPlugin::new(AppState::Loading)
                .continue_to(AppState::MainMenu)
        )
        // Our game loading screen
        // systems that implement tasks to be tracked for completion:
        .add_systems(
            Update,
            (
                dummy_system_wait_frames::<50>.track_progress(),
                dummy_system_wait_millis::<500>.track_progress(),
            )
                .run_if(in_state(AppState::Loading)),
        )
        .run();
}

