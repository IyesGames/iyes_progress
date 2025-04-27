//! This example shows how to update progress from a background thread,
//! instead of a bevy system. The same API works for both OS threads and
//! async tasks running on an async executor.

use std::time::Duration;

use bevy::prelude::*;
use iyes_progress::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<MyStates>()
        .add_plugins(
            ProgressPlugin::<MyStates>::new()
                // Do not clear on enter.
                // This is so that our `spawn_background_work` system can
                // set the total without it being immediately lost.
                .auto_clear(false, true)
                .with_state_transition(MyStates::Loading, MyStates::Done),
        )
        .add_systems(OnEnter(MyStates::Loading), spawn_background_work)
        .add_systems(OnEnter(MyStates::Done), move || {
            info!("Loading complete!");
        })
        .run();
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyStates {
    #[default]
    Loading,
    Done,
}

fn spawn_background_work(mut pt: ResMut<ProgressTracker<MyStates>>) {
    // Create an entry in the progress tracker representing our
    // background work and a "sender handle" that we can give to
    // our thread to update the progress values.
    let sender = pt.new_async_entry();

    // While we are still here and we have direct access to the
    // progress tracker, we can directly update the values for the entry.
    pt.set_total(sender.id(), 1);

    // Create our background thread
    std::thread::spawn(move || {
        // woo! imagine we are doing some really hard and long work here...
        std::thread::sleep(Duration::from_secs(5));

        // From our thread, we can use the sender to report our progress.
        // `iyes_progress` runs a system every bevy frame, which will actually
        // apply the values we send to the entry in the progress tracker.
        sender.set_done(1);
    });
}
