//! Minimal app state used by startup-time renderer loading.

use bevy::prelude::States;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum RenderAppState {
    #[default]
    LoadingAssets,
    SpawningChunks,
    Running,
}
