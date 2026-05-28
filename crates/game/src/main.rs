//! Strategy game — application entry point.
//!
//! Initialises the Bevy application and registers all plugins. Game logic
//! is organised into feature plugins; this file is intentionally thin.
//!
//! The production game entry point intentionally uses the same rendering
//! plugin as the render-gate tools. Tool crates provide editable presets;
//! this binary uses default startup configuration until full game-state
//! driven world creation exists.

use bevy::{asset::AssetPlugin, prelude::*};
use game::rendering::{RenderStartupConfig, RenderingPlugin};

fn main() {
    App::new()
        .insert_resource(RenderStartupConfig::default())
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: workspace_asset_path(),
            ..Default::default()
        }))
        .add_plugins(RenderingPlugin)
        .run();
}

fn workspace_asset_path() -> String {
    format!("{}/../../assets", env!("CARGO_MANIFEST_DIR"))
}
