// Integration-style tests for storage_types structs.
// These live in tests/ so they import via the crate name.

use creator_event_manager::storage_types::{
    Event, EventMetadata, Match, MatchResult, Prediction, Winner,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol};

// ---------------------------------------------------------------------------
// MatchResult
// ---------------------------------------------------------------------------

#[test]
fn test_match_result_encoding() {
    assert_eq!(MatchResult::TeamA.to_u32(), 0);
    assert_eq!(MatchResult::TeamB.to_u32(), 1);
    assert_eq!(MatchResult::Draw.to_u32(), 2);

    assert_eq!(MatchResult::from_u32(0), Some(MatchResult::TeamA));
    assert_eq!(MatchResult::from_u32(1), Some(MatchResult::TeamB));
    assert_eq!(MatchResult::from_u32(2), Some(MatchResult::Draw));
    assert_eq!(MatchResult::from_u32(3), None);
    assert_eq!(MatchResult::from_u32(999), None);
}

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

fn make_event(env: &Env, event_id: u64) -> Event {
    Event::new(
        event_id,
        Address::generate(env),
        String::from_str(env, "Test Event"),
        String::from_str(env, "A test prediction event"),
        1_000_000i128,
        1_640_995_200u64,
        Symbol::new(env, "ABCD1234"),
        100u32,
    )
}

// ---------------------------------------------------------------------------
// Event tests
// ---------------------------------------------------------------------------

#[test]
fn test_event_creation() {
    let env = Env::default();
    let creator = Address::generate(&env);
    let title = String::from_str(&env, "My Event");
    let description = String::from_str(&env, "Description");
    let invite_code = Symbol::new(&env, "CODE1234");

    let event = Event::new(
        1,
        creator.clone(),
        title.clone(),
        description.clone(),
        500_000i128,
        1_640_995_200u64,
        invite_code.clone(),
        50u32,
    );

    assert_eq!(event.event_id, 1);
    assert_eq!(event.creator, creator);
    assert_eq!(event.title, title);
    assert_eq!(event.description, description);
    assert_eq!(event.creation_fee_paid, 500_000);
    assert_eq!(event.created_at, 1_640_995_200);
    assert!(event.is_active);
    assert!(!event.is_cancelled);
    assert_eq!(event.invite_code, invite_code);
    assert_eq!(event.max_participants, 50);
    assert_eq!(event.participant_count, 0);
    assert_eq!(event.match_count, 0);
}

#[test]
fn test_event_validate() {
    let env = Env::default();
    assert!(make_event(&env, 1).validate().is_ok());
}

#[test]
fn test_event_participant_management() {
    let env = Env::default();
    let mut event = make_event(&env, 1);

    assert_eq!(event.participant_count, 0);
    assert!(event.can_accept_participants());

    assert!(event.add_participant().is_ok());
    assert_eq!(event.participant_count, 1);
    assert!(event.add_participant().is_ok());
    assert_eq!(event.participant_count, 2);

    event.deactivate();
    assert!(!event.is_active);
    assert!(!event.can_accept_participants());
    assert!(event.add_participant().is_err());
}

#[test]
fn test_event_cancel() {
    let env = Env::default();
    let mut event = make_event(&env, 1);

    event.cancel();
    assert!(!event.is_active);
    assert!(event.is_cancelled);
    assert!(!event.can_accept_participants());
    assert!(event.add_participant().is_err());
}

#[test]
fn test_event_max_participants() {
    let env = Env::default();
    let mut event = Event::new(
        1,
        Address::generate(&env),
        String::from_str(&env, "Capped"),
        String::from_str(&env, "2 spots"),
        0i128,
        0u64,
        Symbol::new(&env, "CAPCODE1"),
        2u32,
    );

    assert!(event.add_participant().is_ok());
    assert!(event.add_participant().is_ok());
    assert!(event.add_participant().is_err()); // full
}

#[test]
fn test_event_unlimited_participants() {
    let env = Env::default();
    let mut event = Event::new(
        1,
        Address::generate(&env),
        String::from_str(&env, "Open"),
        String::from_str(&env, "No cap"),
        0i128,
        0u64,
        Symbol::new(&env, "OPENCODE"),
        0u32, // 0 = unlimited
    );

    for _ in 0..10 {
        assert!(event.add_participant().is_ok());
    }
    assert_eq!(event.participant_count, 10);
}

#[test]
fn test_event_add_match() {
    let env = Env::default();
    let mut event = make_event(&env, 1);
    assert_eq!(event.match_count, 0);
    event.add_match();
    event.add_match();
    assert_eq!(event.match_count, 2);
}

#[test]
fn test_event_age_calculation() {
    let env = Env::default();
    let event = make_event(&env, 1); // created_at = 1_640_995_200

    assert_eq!(event.get_age_seconds(1_640_995_200 + 3600), 3600);
    assert_eq!(event.get_age_seconds(1_640_995_200 - 1000), 0); // saturating_sub
}

// ---------------------------------------------------------------------------
// Match helpers
// ---------------------------------------------------------------------------

fn make_match(env: &Env, match_id: u64, event_id: u64, match_time: u64) -> Match {
    Match::new(
        match_id,
        event_id,
        String::from_str(env, "Team Alpha"),
        String::from_str(env, "Team Beta"),
        match_time,
    )
}

// ---------------------------------------------------------------------------
// Match tests
// ---------------------------------------------------------------------------

#[test]
fn test_match_creation() {
    let env = Env::default();
    let m = make_match(&env, 1, 100, 1_640_995_200);

    assert_eq!(m.match_id, 1);
    assert_eq!(m.event_id, 100);
    assert_eq!(m.match_time, 1_640_995_200);
    assert!(!m.result_submitted);
    assert!(m.winning_team.is_none());
    assert!(m.submitted_by.is_none());
    assert!(m.submitted_at.is_none());
}

#[test]
fn test_match_result_submission() {
    let env = Env::default();
    let oracle = Address::generate(&env);
    let match_time = 1_640_995_200u64;
    let result_time = match_time + 7200;

    let mut m = make_match(&env, 1, 100, match_time);

    assert!(m.submit_result(MatchResult::TeamA, oracle.clone(), result_time).is_ok());

    assert!(m.result_submitted);
    // winning_team is stored as Option<u32>
    assert_eq!(m.winning_team, Some(0u32));
    assert_eq!(m.submitted_by, Some(oracle.clone()));
    assert_eq!(m.submitted_at, Some(result_time));
    assert_eq!(m.get_winner(), Some(MatchResult::TeamA));
    assert!(m.is_completed());

    // Cannot submit twice
    assert!(m.submit_result(MatchResult::TeamB, oracle, result_time + 100).is_err());
}

#[test]
fn test_match_timing() {
    let env = Env::default();
    let match_time = 1_640_995_200u64;
    let m = make_match(&env, 1, 100, match_time);

    let before = match_time - 3600;
    assert!(!m.has_started(before));
    assert!(!m.is_ready_for_result(before));
    assert_eq!(m.time_until_start(before), 3600);
    assert_eq!(m.time_since_result(before), 0);

    let after = match_time + 1800;
    assert!(m.has_started(after));
    assert!(m.is_ready_for_result(after));
    assert_eq!(m.time_until_start(after), 0);
    assert_eq!(m.time_since_result(after), 0);
}

#[test]
fn test_match_predictions_allowed() {
    let env = Env::default();
    let match_time = 1_640_995_200u64;
    let m = make_match(&env, 1, 100, match_time);

    assert!(m.allows_predictions(match_time - 7200, 30)); // 2 h before, 30-min cutoff → ok
    assert!(!m.allows_predictions(match_time - 900, 30)); // 15 min before → blocked
    assert!(!m.allows_predictions(match_time + 1, 30));   // after start → blocked
}

#[test]
fn test_match_validation_valid() {
    let env = Env::default();
    assert!(make_match(&env, 1, 100, 1_640_995_200).validate().is_ok());
}

#[test]
fn test_match_validation_empty_team_a() {
    let env = Env::default();
    let m = Match::new(1, 100, String::from_str(&env, ""), String::from_str(&env, "Beta"), 0);
    assert!(m.validate().is_err());
}

#[test]
fn test_match_validation_empty_team_b() {
    let env = Env::default();
    let m = Match::new(1, 100, String::from_str(&env, "Alpha"), String::from_str(&env, ""), 0);
    assert!(m.validate().is_err());
}

#[test]
fn test_match_validation_same_teams() {
    let env = Env::default();
    let name = String::from_str(&env, "Same");
    let m = Match::new(1, 100, name.clone(), name, 0);
    assert!(m.validate().is_err());
}

#[test]
fn test_match_validation_inconsistent_result() {
    let env = Env::default();

    // result_submitted = true but winning_team = None
    let mut m = make_match(&env, 1, 100, 0);
    m.result_submitted = true;
    assert!(m.validate().is_err());

    // winning_team set but result_submitted = false
    let mut m2 = make_match(&env, 1, 100, 0);
    m2.winning_team = Some(0u32);
    assert!(m2.validate().is_err());
}

// ---------------------------------------------------------------------------
// Prediction tests
// ---------------------------------------------------------------------------

#[test]
fn test_prediction_creation_and_validation() {
    let env = Env::default();
    let predictor = Address::generate(&env);

    let pred = Prediction::new(predictor.clone(), 1, 1, 0u32, 1_640_995_200);
    assert_eq!(pred.predictor, predictor);
    assert_eq!(pred.predicted_winner, 0u32);
    assert!(pred.is_correct.is_none());
    assert!(pred.validate().is_ok());

    // Invalid outcome value
    let bad = Prediction::new(predictor, 1, 1, 3u32, 1_640_995_200);
    assert!(bad.validate().is_err());
}

#[test]
fn test_prediction_grading() {
    let env = Env::default();
    let predictor = Address::generate(&env);

    // Predicted TeamA (0), actual TeamA → correct
    let mut pred = Prediction::new(predictor.clone(), 1, 1, 0u32, 1_640_995_200);
    pred.grade(&MatchResult::TeamA);
    assert_eq!(pred.is_correct, Some(true));
    assert!(pred.is_winner());

    // Predicted TeamB (1), actual TeamA → wrong
    let mut pred2 = Prediction::new(predictor, 1, 1, 1u32, 1_640_995_200);
    pred2.grade(&MatchResult::TeamA);
    assert_eq!(pred2.is_correct, Some(false));
    assert!(!pred2.is_winner());
}

// ---------------------------------------------------------------------------
// Winner tests
// ---------------------------------------------------------------------------

#[test]
fn test_winner_creation() {
    let env = Env::default();
    let user = Address::generate(&env);

    let winner = Winner::new(user.clone(), 42, 5, 1_640_995_200);
    assert_eq!(winner.user, user);
    assert_eq!(winner.event_id, 42);
    assert_eq!(winner.total_correct_predictions, 5);
    assert_eq!(winner.verified_at, 1_640_995_200);
}

// ---------------------------------------------------------------------------
// EventMetadata tests
// ---------------------------------------------------------------------------

#[test]
fn test_event_metadata_phases() {
    let env = Env::default();
    let base = 1_640_995_200u64;

    let metadata = EventMetadata::new(
        String::from_str(&env, "Sports"),
        String::from_str(&env, "football,nfl"),
        10,
        1000,
        base + 86_400,  // end_time   +24 h
        base + 172_800, // resolution +48 h
        false,
        100,
    );

    // 12 h in — prediction phase
    assert!(metadata.is_prediction_phase(base + 43_200));
    assert!(!metadata.is_resolution_phase(base + 43_200));
    assert!(!metadata.should_auto_resolve(base + 43_200));

    // 36 h in — resolution phase
    assert!(!metadata.is_prediction_phase(base + 129_600));
    assert!(metadata.is_resolution_phase(base + 129_600));
    assert!(!metadata.should_auto_resolve(base + 129_600));

    // 72 h in — auto-resolve
    assert!(!metadata.is_prediction_phase(base + 259_200));
    assert!(!metadata.is_resolution_phase(base + 259_200));
    assert!(metadata.should_auto_resolve(base + 259_200));
}
