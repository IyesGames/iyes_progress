#![allow(dead_code)]

use bevy::prelude::*;
use iyes_loopless::prelude::*;
use iyes_progress::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Splash,
    MainMenu,
    GameLoading,
    InGame,
}

fn main() {
    App::new()
        // Init bevy
        .add_plugins(DefaultPlugins)
        // Add our state type
        .add_loopless_state(AppState::Splash)
        // Add plugin for the splash screen
        .add_plugin(
            ProgressPlugin::new(AppState::Splash)
                .continue_to(AppState::MainMenu)
                .track_assets(),
        )
        // Add plugin for our game loading screen
        .add_plugin(ProgressPlugin::new(AppState::GameLoading).continue_to(AppState::InGame))
        // Load our UI assets during our splash screen
        .add_enter_system(AppState::Splash, load_ui_assets)
        // Our game loading screen
        .add_system_set(
            ConditionSet::new()
                .run_in_state(AppState::GameLoading)
                // systems that implement tasks to be tracked for completion:
                .with_system(net_init_session.track_progress())
                .with_system(world_generation.track_progress())
                .with_system(internal_thing.track_progress())
                // we can also add regular untracked systems to our loading screen,
                // like to draw our progress bar:
                .with_system(ui_progress_bar)
                .into(),
        )
        .run();
}

struct MyUiAssets {
    ui_font: Handle<Font>,
    btn_img: Handle<Image>,
}

fn load_ui_assets(
    mut commands: Commands,
    ass: Res<AssetServer>,
    // we need to add our handles here, to track their loading progress:
    mut loading: ResMut<AssetsLoading>,
) {
    let ui_font = ass.load("font.ttf");
    let btn_img = ass.load("btn.png");
    // etc ...

    // don't forget to add them so they can be tracked:
    loading.add(&ui_font);
    loading.add(&btn_img);

    commands.insert_resource(MyUiAssets { ui_font, btn_img });
}

fn net_init_session(// ...
) -> Progress {
    if my_session_is_ready() {
        // we can convert a `bool` into a `Progress`
        return true.into();
    }

    my_session_try_init();

    false.into()
}

fn world_generation(
    // ...
    mut next_chunk_id: Local<u32>,
) -> Progress {
    const N_CHUNKS: u32 = 16;

    if *next_chunk_id < N_CHUNKS {
        // every frame, do some work
        gen_chunk(*next_chunk_id);
        *next_chunk_id += 1;
    }

    // here we can return a `Progress` value that more accurately represents
    // how much of our world map has been generated so far
    Progress {
        done: *next_chunk_id,
        total: N_CHUNKS,
    }
}

fn internal_thing(
    // ...
) -> HiddenProgress {
    // "hidden progress" allows us to report progress
    // that is tracked separately, so it is counted for
    // the state transition, but not for our user-facing
    // progress bar

    // Just wrap the usual `Progress` value in a `HiddenProgress`
    HiddenProgress(internal_ready().into())
}

fn ui_progress_bar(
    counter: Res<ProgressCounter>,
    // ...
) {
    // Get the overall loading progress
    let progress = counter.progress();

    // we can use `progress.done` and `progress.total`,
    // or convert it to a float:
    let _float_progress: f32 = progress.into();

    // TODO: implement our progress bar
    unimplemented!()
}

fn my_session_is_ready() -> bool {
    unimplemented!()
}

fn my_session_try_init() {
    unimplemented!()
}

fn gen_chunk(_id: u32) {
    unimplemented!()
}

fn internal_ready() -> bool {
    unimplemented!()
}
