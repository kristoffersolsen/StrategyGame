//! Strategy game — application entry point.
//!
//! Initialises the Bevy application and registers all plugins. Game logic
//! is organised into feature plugins; this file is intentionally thin.
//!
//! World generation is handled by the `worldgen` crate before Bevy
//! initialises rendering. The generated [`worldgen::WorldData`] is inserted
//! as a Bevy resource and consumed by the map plugin at startup.

fn main() {
    println!("Hello, world!");
}
