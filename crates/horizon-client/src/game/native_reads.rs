//! What a native mod reads of the player and the world's clock (`native_player`, `native_clock`).

/// The player as a native mod reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct NativePlayer {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Degrees; 0 faces south, 90 west (`Direction.fromYRot`).
    pub yaw: f32,
    pub pitch: f32,
    /// `minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`.
    pub dimension: &'static str,
}

/// The world's clock as a client mod reads it (`native_clock`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeClock {
    /// Ticks since the world was created (`Level.getGameTime`).
    pub game_time: i64,
    /// The time of day in ticks (`Level.getOverworldClockTime`): 0 at sunrise, 24000 a
    /// day. It keeps counting past 24000 (`/time add`, days going by), so take
    /// it modulo 24000 for the hour; `/time set` can make it negative.
    pub day_time: i64,
}
