//! Read-only aggregate views for creator events.
//!
//! This module keeps derived, dashboard-friendly statistics out of mutation
//! paths so callers can inspect an event's participation, prediction volume,
//! and completion state in a single contract view.

use soroban_sdk::{contracttype, Env};

use crate::event::{self, EventError};
use crate::storage;

/// Aggregate statistics for one creator event.
///
/// Returned by `get_event_statistics(event_id)` as a compact summary of the
/// event's current on-chain state:
/// * `event_id` — event being summarized.
/// * `participant_count` — number of joined participants stored on the event.
/// * `match_count` — number of matches stored on the event.
/// * `total_predictions` — total predictions linked to all event matches.
/// * `all_matches_resolved` — `true` only when the event has at least one
///   match and every stored match has a submitted result.
/// * `winners_verified` — `true` when one or more verified winner records are
///   stored for the event.
/// * `winner_count` — number of verified winner records stored for the event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventStatistics {
    pub event_id: u64,
    pub participant_count: u32,
    pub match_count: u32,
    pub total_predictions: u32,
    pub all_matches_resolved: bool,
    pub winners_verified: bool,
    pub winner_count: u32,
}

/// Build aggregate statistics for an existing event.
///
/// The function first retrieves the event to validate that `event_id` exists,
/// then derives prediction totals from the event's match index, completion
/// status from each stored match result, and winner status from the event's
/// verified winners list.
pub fn get_event_statistics(env: &Env, event_id: u64) -> Result<EventStatistics, EventError> {
    let event = event::get_event(env, event_id)?;
    let match_ids = storage::get_event_matches(env, event_id);

    let mut total_predictions: u32 = 0;
    let mut resolved_matches: u32 = 0;

    for match_id in match_ids.iter() {
        total_predictions =
            total_predictions.saturating_add(storage::get_match_predictions(env, match_id).len());

        if let Ok(match_record) = storage::get_match(env, match_id) {
            if match_record.result_submitted {
                resolved_matches = resolved_matches.saturating_add(1);
            }
        }
    }

    let winner_count = storage::get_event_winners(env, event_id).len();
    let all_matches_resolved = event.match_count > 0
        && match_ids.len() == event.match_count
        && resolved_matches == event.match_count;

    Ok(EventStatistics {
        event_id,
        participant_count: event.participant_count,
        match_count: event.match_count,
        total_predictions,
        all_matches_resolved,
        winners_verified: winner_count > 0,
        winner_count,
    })
}
