#![allow(dead_code)]

use bevy::prelude::*;
use bevy_loading::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Component)]
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
        .add_state(AppState::Splash)

        // Add loading plugin for the splash screen
        .add_plugin(LoadingPlugin {
            loading_state: AppState::Splash,
            next_state: AppState::MainMenu,
        })
        // Add loading plugin for our game loading screen
        .add_plugin(LoadingPlugin {
            loading_state: AppState::GameLoading,
            next_state: AppState::InGame,
        })

        // Load our UI assets during our splash screen
        .add_system_set(
            SystemSet::on_enter(AppState::Splash)
                .with_system(load_ui_assets.system())
        )

        // Our game loading screen
        .add_system_set(
            SystemSet::on_update(AppState::GameLoading)
                // systems that implement tasks to be tracked for completion:
                // (wrap systems that return `Progress` with `track`)
                .with_system(track(net_init_session))
                .with_system(track(worldgen))

                // we can also add regular untracked systems to our loading screen,
                // like to draw our progress bar:
                .with_system(ui_progress_bar)
        )
        .run();
}

struct MyUiAssets {
    ui_font: Handle<Font>,
    btn_tex: Handle<Texture>,
}

fn load_ui_assets(
    mut commands: Commands,
    ass: Res<AssetServer>,
    // we need to add our handles here, to track their loading progress:
    mut loading: ResMut<AssetsLoading>,
) {
    let ui_font = ass.load("font.ttf");
    let btn_tex = ass.load("btn.png");
    // etc ...

    // don't forget to add them so they can be tracked:
    loading.add(&ui_font);
    loading.add(&btn_tex);

    commands.insert_resource(MyUiAssets {
        ui_font,
        btn_tex,
    });
}

fn net_init_session(
    // ...
) -> Progress {
    if my_session_is_ready() {
        // we can convert a `bool` into a `Progress`
        return true.into();
    }

    my_session_try_init();

    false.into()
}

fn worldgen(
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

fn ui_progress_bar(
    counter: Res<bevy_loading::ProgressCounter>,
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

