//! Example to showcase `iyes_progress`
//!
//! The whole point of `iyes_progress` is that you can do arbitrarily
//! complex things during your loading screens, beyond just loading
//! assets. It will help you track everything, report the progress
//! to the user, and only transition the state when all the work is
//! complete.
//!
//! In this example, we will make a silly contrived "loading screen"
//! where the user has to hold the spacebar and click the mouse a
//! bunch of times to advance.
//!
//! This will teach you the various ways of using `iyes_progress` to
//! track custom Bevy systems doing different things.
//!
//! In a real game, you might instead have systems to prepare the map,
//! connect to a multiplayer server, etc. The sky is the limit!

use bevy::prelude::*;
use bevy::utils::Duration;
use iyes_progress::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyStates {
    #[default]
    Loading,
    Done,
}

// The simplest way to use this crate is to write Bevy systems that return
// `Progress`
//
// `Progress` is a struct to indicate how many "units of work" you have
// completed, out of how many total expected.
fn mouse_clicks(
    input: Res<ButtonInput<MouseButton>>,
    mut n_clicks: Local<u32>,
) -> Progress {
    const MAX_CLICKS: u32 = 5;

    if input.just_pressed(MouseButton::Left) {
        *n_clicks += 1;
    }

    Progress {
        done: *n_clicks,
        total: MAX_CLICKS,
    }
}

fn hold_spacebar(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut total_held_time: Local<Duration>,
) -> Progress {
    const TARGET_DURATION: Duration = Duration::from_secs(5);

    if input.pressed(KeyCode::Space) {
        *total_held_time += time.delta();
    }

    // we can create `Progress` from a `bool`
    (*total_held_time > TARGET_DURATION).into()
}

// You can also return `HiddenProgress`, which means the value will
// be tracked and the work will have to be completed in order to
// transition state, but it will not be shown to the user
// (in UI progress bars, etc).
fn hidden_timer(
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
) -> HiddenProgress {
    if let Some(timer) = &mut *timer {
        timer.tick(time.delta());
        timer.finished().into()
    } else {
        *timer = Some(Timer::new(Duration::from_secs(10), TimerMode::Once));
        false.into()
    }
}

// Another way to track progress is using the special `ProgressEntry`
// system param, instead of returning a value.
//
// Each such param will track its own progress values.
fn count_abc_keypresses(
    progress_a: ProgressEntry<MyStates>,
    progress_b: ProgressEntry<MyStates>,
    progress_c: ProgressEntry<MyStates>,
    input: Res<ButtonInput<KeyCode>>,
    // to check for first run and initialize
    mut initted: Local<bool>,
) {
    if !*initted {
        // set the total expected progress
        progress_a.set_total(3);
        progress_b.set_total(2);
        progress_c.set_total(1);
        *initted = true;
    }

    if input.just_pressed(KeyCode::KeyA) && !progress_a.is_ready() {
        progress_a.add_done(1);
    }
    if input.just_pressed(KeyCode::KeyB) && !progress_b.is_ready() {
        progress_b.add_done(1);
    }
    if input.just_pressed(KeyCode::KeyC) && !progress_c.is_ready() {
        progress_c.add_done(1);
    }
}

#[derive(Component)]
struct ProgressBarInner;

#[derive(Component)]
struct ProgressBarText;

// And this is how you might report the total progress info to the user
fn update_progress_bar(
    // Here is where the progress is stored
    pt: Res<ProgressTracker<MyStates>>,
    // our UI elements
    mut q_bar_inner: Query<&mut Node, With<ProgressBarInner>>,
    mut q_bar_text: Query<&mut Text, With<ProgressBarText>>,
) {
    // get the progress info (visible progress)
    let progress = pt.get_global_progress();
    let ratio: f32 = progress.into();

    for mut text in q_bar_text.iter_mut() {
        *text = Text::new(format!("{}/{}", progress.done, progress.total));
    }
    for mut node in q_bar_inner.iter_mut() {
        node.width = Val::Percent(ratio * 100.0);
    }
}

fn setup_loading_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            ..Default::default()
        },
        Text::new(
            "Surprise! In this \"loading\" screen,
YOU have to do things and the game waits for YOU!

To progress:
 - Hold the spacebar for 5 seconds
 - Press the A/B/C keys a few times
 - Click the mouse a few times",
        ),
        StateScoped(MyStates::Loading),
    ));
    let bar_outer = commands
        .spawn((
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            BorderColor(Color::srgb(1.0, 1.0, 1.0)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(69.0),
                bottom: Val::Percent(24.0),
                left: Val::Percent(12.0),
                right: Val::Percent(12.0),
                ..Default::default()
            },
            StateScoped(MyStates::Loading),
        ))
        .id();

    let bar_inner = commands
        .spawn((
            BackgroundColor(Color::srgb(0.75, 0.75, 0.75)),
            BorderColor(Color::srgb(0.5, 0.5, 0.5)),
            Node {
                height: Val::Percent(100.0),
                width: Val::Percent(0.0),
                padding: UiRect::left(Val::Px(16.0)),
                align_items: AlignItems::Center,
                ..Default::default()
            },
            ProgressBarInner,
        ))
        .id();

    let bar_text = commands
        .spawn((
            Text::new("0/0".to_owned()),
            TextColor(Color::WHITE),
            ProgressBarText,
        ))
        .id();

    commands.entity(bar_outer).add_child(bar_inner);
    commands.entity(bar_inner).add_child(bar_text);
}

fn setup_done_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            ..Default::default()
        },
        Text::new("All done! You are the BEST!"),
    ));
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<MyStates>()
        .enable_state_scoped_entities::<MyStates>()
        .add_plugins(
            ProgressPlugin::<MyStates>::new()
                .with_state_transition(MyStates::Loading, MyStates::Done),
        )
        .add_systems(
            Update,
            (
                // `track_progress` adds the needed machinery to apply the
                // value returned by the system to the global `ProgressTracker`
                // (for the `MyStates` states type)
                hold_spacebar.track_progress::<MyStates>(),
                hidden_timer.track_progress::<MyStates>(),
                mouse_clicks
                    // `track_progress_and_stop` also adds a run condition to
                    // make it so that if the system returns fully completed
                    // progress, it will not run anymore.
                    .track_progress_and_stop::<MyStates>(),
                // systems with `ProgressEntry` don't need anything special
                count_abc_keypresses,
            )
                .run_if(in_state(MyStates::Loading)),
        )
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(MyStates::Loading), setup_loading_ui)
        .add_systems(OnEnter(MyStates::Done), setup_done_ui)
        .add_systems(Update, update_progress_bar)
        .run();
}
