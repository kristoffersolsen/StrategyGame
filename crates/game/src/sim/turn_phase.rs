//! Turn state machine: phase enum, turn counter, and end-turn transition.
//!
//! [`TurnPhase`] tracks the authoritative simulation phase. [`TurnState`]
//! tracks the current turn number. [`EndTurnRequested`] is the buffered
//! message command sources write to request a resolution pass;
//! [`handle_end_turn`] consumes it and transitions to
//! [`TurnPhase::Resolution`].

use bevy::prelude::{MessageReader, NextState, ResMut, Resource, States};

/// Active phase within a single game turn.
///
/// - `PlayerInput`: command sources may push commands onto the queue.
/// - `Resolution`: the command processor drains and applies the queue.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum TurnPhase {
    /// Player (or AI) input is accepted; command sources issue commands.
    #[default]
    PlayerInput,
    /// The command processor drains and applies all queued commands.
    Resolution,
}

/// Monotonically increasing turn counter.
///
/// Incremented by the command processor after each resolution pass.
/// Starts at `0` on world load; the first player turn is turn `1`.
#[derive(Resource, Default)]
pub(crate) struct TurnState {
    /// Current turn number. `0` = pre-game; incremented after each resolution.
    pub(crate) turn: u32,
}

/// Written by command sources to request a turn advance.
///
/// Consumed in [`TurnPhase::PlayerInput`] by [`handle_end_turn`], which
/// transitions the phase to [`TurnPhase::Resolution`].
#[derive(bevy::prelude::Message, Debug, Clone)]
pub(crate) struct EndTurnRequested;

/// Transitions from [`TurnPhase::PlayerInput`] to [`TurnPhase::Resolution`]
/// when [`EndTurnRequested`] is received.
///
/// Runs in `Update`, gated on [`TurnPhase::PlayerInput`].
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn handle_end_turn(
    mut phase: ResMut<NextState<TurnPhase>>,
    mut messages: MessageReader<EndTurnRequested>,
) {
    if messages.read().next().is_some() {
        phase.set(TurnPhase::Resolution);
    }
}

/// Increments the turn counter and returns to [`TurnPhase::PlayerInput`].
///
/// The entire job of the `Resolution` phase: advance the counter so that
/// `OnEnter(PlayerInput)` fires and resets unit movement points, then hand
/// control back to the player.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn resolve_turn(
    mut turn_state: ResMut<TurnState>,
    mut phase: ResMut<NextState<TurnPhase>>,
) {
    turn_state.turn += 1;
    phase.set(TurnPhase::PlayerInput);
}
