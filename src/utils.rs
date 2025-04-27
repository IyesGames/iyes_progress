use std::time::Duration;

use bevy_ecs::prelude::*;
use bevy_platform::time::Instant;

use crate::prelude::Progress;

/// Dummy system to wait for a number of frames.
///
/// Returns hidden progress with 0/1 when not ready and 1/1 when ready.
///
/// May be useful for testing/debug/workaround purposes.
pub fn dummy_system_wait_frames<const N: u32>(
    mut count: Local<u32>,
) -> Progress {
    *count += 1;
    (*count - 1 <= N).into()
}

/// Dummy system to count a number of frames.
///
/// Returns hidden progress with the number of frames (frames as progress
/// units).
///
/// May be useful for testing/debug/workaround purposes.
pub fn dummy_system_count_frames<const N: u32>(
    mut count: Local<u32>,
) -> Progress {
    if *count <= N {
        *count += 1;
    }
    Progress {
        done: *count - 1,
        total: N,
    }
}

/// Dummy system to wait for a time duration
///
/// May be useful for testing/debug/workaround purposes.
pub fn dummy_system_wait_millis<const MILLIS: u64>(
    mut state: Local<Option<Instant>>,
) -> Progress {
    let end =
        state.unwrap_or_else(|| Instant::now() + Duration::from_millis(MILLIS));
    *state = Some(end);
    (Instant::now() > end).into()
}
