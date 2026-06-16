//! #810 — Integration tests for the public `submit_match_result` oracle entry
//! point (exercised through the contract client), covering:
//! - AI agent can submit a result
//! - Non-agent cannot submit
//! - Result before match time is rejected
//! - Duplicate submission is rejected
//! - Invalid outcome is rejected
//! - Predictions are graded correct/incorrect
//! - All outcomes (TEAM_A, TEAM_B, DRAW) work
//! - Full prediction flow (submit -> grade -> score)

use creator_event_manager::storage;
use creator_event_manager::storage_types::Match;
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol};

const FEE: i128 = 1_000_000;

fn setup() -> (
    Env,
    CreatorEventManagerContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(creator_event_manager::CreatorEventManagerContract, ());
    let client = CreatorEventManagerContractClient::new(&env, &contract_id);
    let client: CreatorEventManagerContractClient<'static> =
        unsafe { core::mem::transmute(client) };

    let admin = Address::generate(&env);
    let ai_agent = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let xlm_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.initialize(&admin, &ai_agent, &treasury, &xlm_token, &FEE);
    (env, client, contract_id, admin, ai_agent, xlm_token)
}

fn fund(env: &Env, token: &Address, user: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(user, &amount);
}

fn title(env: &Env) -> String {
    String::from_str(env, "Test Event")
}

fn desc(env: &Env) -> String {
    String::from_str(env, "Test Description")
}

fn get_future_time(env: &Env, offset_seconds: u64) -> u64 {
    env.ledger().timestamp() + offset_seconds
}

/// Create an event with a single match starting `match_time_offset` seconds
/// from now. Returns `(event_id, invite_code, match_id)`.
fn create_event_with_match(
    env: &Env,
    contract_id: &Address,
    client: &CreatorEventManagerContractClient<'static>,
    creator: &Address,
    xlm_token: &Address,
    match_time_offset: u64,
) -> (u64, Symbol, u64) {
    fund(env, xlm_token, creator, FEE);
    let start_time = get_future_time(env, 3600);
    let end_time = get_future_time(env, 7200);
    let (event_id, invite_code) = client.create_event(
        creator,
        &title(env),
        &desc(env),
        &10u32,
        &start_time,
        &end_time,
    );

    let match_id = env.as_contract(contract_id, || {
        let match_id = storage::next_match_id(env);
        let match_record = creator_event_manager::storage_types::Match::new(
            match_id,
            event_id,
            String::from_str(env, "Team A"),
            String::from_str(env, "Team B"),
            env.ledger().timestamp() + match_time_offset,
        );
        storage::set_match(env, match_id, &match_record);
        storage::add_event_match(env, event_id, match_id);

        let mut event = storage::get_event(env, event_id).expect("event exists");
        event.add_match();
        storage::set_event(env, event_id, &event);
        match_id
    });

    (event_id, invite_code, match_id)
}

fn read_match(env: &Env, contract_id: &Address, match_id: u64) -> Match {
    env.as_contract(contract_id, || storage::get_match(env, match_id).unwrap())
}

#[test]
fn test_ai_agent_can_submit_result() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);

    env.ledger().with_mut(|l| l.timestamp += 2_000);
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "TEAM_A"));

    let m = read_match(&env, &contract_id, match_id);
    assert!(m.result_submitted);
    assert_eq!(m.winning_team, Some(0));
    assert_eq!(m.submitted_by, Some(ai_agent));
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_non_agent_cannot_submit() {
    let (env, client, contract_id, _admin, _ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);

    env.ledger().with_mut(|l| l.timestamp += 2_000);
    let imposter = Address::generate(&env);
    client.submit_match_result(&imposter, &match_id, &Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "match_not_started")]
fn test_result_before_match_time_rejected() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    // Do NOT advance time — the match has not started yet.
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "result_already_submitted")]
fn test_duplicate_submission_rejected() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);

    env.ledger().with_mut(|l| l.timestamp += 2_000);
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "TEAM_A"));
    // Second submission must be rejected.
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "TEAM_B"));
}

#[test]
#[should_panic(expected = "invalid_outcome")]
fn test_invalid_outcome_rejected() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);

    env.ledger().with_mut(|l| l.timestamp += 2_000);
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "NOT_A_TEAM"));
}

#[test]
#[should_panic(expected = "match_not_found")]
fn test_unknown_match_rejected() {
    let (env, client, _contract_id, _admin, ai_agent, _xlm_token) = setup();
    env.ledger().with_mut(|l| l.timestamp += 2_000);
    client.submit_match_result(&ai_agent, &404u64, &Symbol::new(&env, "TEAM_A"));
}

#[test]
fn test_predictions_marked_correct_and_incorrect() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let winner = Address::generate(&env);
    let loser = Address::generate(&env);
    client.join_event(&winner, &invite);
    client.join_event(&loser, &invite);
    let winner_pred = client.submit_prediction(&winner, &match_id, &Symbol::new(&env, "TEAM_A"));
    let loser_pred = client.submit_prediction(&loser, &match_id, &Symbol::new(&env, "TEAM_B"));

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "TEAM_A"));

    assert_eq!(client.get_prediction(&winner_pred).is_correct, Some(true));
    assert_eq!(client.get_prediction(&loser_pred).is_correct, Some(false));
}

#[test]
fn test_all_outcomes_work() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);

    let (_e1, _i1, m_a) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);
    let (_e2, _i2, m_b) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);
    let (_e3, _i3, m_d) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1_000);

    env.ledger().with_mut(|l| l.timestamp += 2_000);
    client.submit_match_result(&ai_agent, &m_a, &Symbol::new(&env, "TEAM_A"));
    client.submit_match_result(&ai_agent, &m_b, &Symbol::new(&env, "TEAM_B"));
    client.submit_match_result(&ai_agent, &m_d, &Symbol::new(&env, "DRAW"));

    assert_eq!(read_match(&env, &contract_id, m_a).winning_team, Some(0));
    assert_eq!(read_match(&env, &contract_id, m_b).winning_team, Some(1));
    assert_eq!(read_match(&env, &contract_id, m_d).winning_team, Some(2));
}

#[test]
fn test_full_prediction_flow_with_scoring() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    client.join_event(&alice, &invite);
    client.join_event(&bob, &invite);
    client.submit_prediction(&alice, &match_id, &Symbol::new(&env, "DRAW"));
    client.submit_prediction(&bob, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    client.submit_match_result(&ai_agent, &match_id, &Symbol::new(&env, "DRAW"));

    // Alice predicted the winning outcome; Bob did not.
    assert_eq!(client.get_user_score(&alice, &event_id), (1, 1));
    assert_eq!(client.get_user_score(&bob, &event_id), (0, 1));

    // And the match is fully resolved.
    let m = read_match(&env, &contract_id, match_id);
    assert!(m.result_submitted);
    assert_eq!(m.winning_team, Some(2));
}


// ============================================================================
// Scoreline grading tests (#xxx — exact score predictions)
// Acceptance tests: These tests will pass once the feature is implemented.
// After implementation:
// - submit_prediction: (predictor, match_id, home_score: u32, away_score: u32) -> u64
// - submit_match_result: (caller, match_id, home_score: u32, away_score: u32) -> ()
// - get_user_score: (...) -> (total_points, correct_results, exact_scores, total_matches)
// ============================================================================

#[test]
#[ignore] // Ignore until implementation is complete
fn test_grading_exact_score_awards_4_points() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let predictor = Address::generate(&env);
    client.join_event(&predictor, &invite);
    // New API: submit_prediction(predictor, match_id, home_score, away_score)
    let prediction_id = client.submit_prediction(&predictor, &match_id, &2u32, &1u32);

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    // New API: submit_match_result(caller, match_id, home_score, away_score)
    client.submit_match_result(&ai_agent, &match_id, &2u32, &1u32);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.points_earned, Some(4));
    assert_eq!(prediction.is_correct, Some(true));

    // New API: get_user_score returns (total_points, correct_results, exact_scores, total_matches)
    let (total_points, correct_results, exact_scores, total_matches) =
        client.get_user_score(&predictor, &event_id);
    assert_eq!(total_points, 4);
    assert_eq!(correct_results, 1);
    assert_eq!(exact_scores, 1);
    assert_eq!(total_matches, 1);
}

#[test]
#[ignore] // Ignore until implementation is complete
fn test_grading_correct_result_wrong_score_awards_1_point() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let predictor = Address::generate(&env);
    client.join_event(&predictor, &invite);
    // Predict 2-1 (Team A wins)
    let prediction_id = client.submit_prediction(&predictor, &match_id, &2u32, &1u32);

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    // Actual result is 3-1 (Team A wins)
    client.submit_match_result(&ai_agent, &match_id, &3u32, &1u32);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.points_earned, Some(1));
    assert_eq!(prediction.is_correct, Some(true)); // Result was correct (Team A won)

    let (total_points, correct_results, exact_scores, total_matches) =
        client.get_user_score(&predictor, &event_id);
    assert_eq!(total_points, 1);
    assert_eq!(correct_results, 1);
    assert_eq!(exact_scores, 0);
    assert_eq!(total_matches, 1);
}

#[test]
#[ignore] // Ignore until implementation is complete
fn test_grading_wrong_result_awards_0_points() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let predictor = Address::generate(&env);
    client.join_event(&predictor, &invite);
    // Predict 1-0 (Team A wins)
    let prediction_id = client.submit_prediction(&predictor, &match_id, &1u32, &0u32);

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    // Actual result is 0-1 (Team B wins)
    client.submit_match_result(&ai_agent, &match_id, &0u32, &1u32);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.points_earned, Some(0));
    assert_eq!(prediction.is_correct, Some(false));

    let (total_points, correct_results, exact_scores, total_matches) =
        client.get_user_score(&predictor, &event_id);
    assert_eq!(total_points, 0);
    assert_eq!(correct_results, 0);
    assert_eq!(exact_scores, 0);
    assert_eq!(total_matches, 1);
}

#[test]
#[ignore] // Ignore until implementation is complete
fn test_grading_draw_exact_score() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let predictor = Address::generate(&env);
    client.join_event(&predictor, &invite);
    // Predict 1-1 (Draw)
    let prediction_id = client.submit_prediction(&predictor, &match_id, &1u32, &1u32);

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    // Actual result is 1-1 (Draw)
    client.submit_match_result(&ai_agent, &match_id, &1u32, &1u32);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.points_earned, Some(4));
    assert_eq!(prediction.is_correct, Some(true));

    let (total_points, correct_results, exact_scores, total_matches) =
        client.get_user_score(&predictor, &event_id);
    assert_eq!(total_points, 4);
    assert_eq!(correct_results, 1);
    assert_eq!(exact_scores, 1);
    assert_eq!(total_matches, 1);
}

#[test]
#[ignore] // Ignore until implementation is complete
fn test_grading_draw_wrong_score() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (event_id, invite, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let predictor = Address::generate(&env);
    client.join_event(&predictor, &invite);
    // Predict 1-1 (Draw)
    let prediction_id = client.submit_prediction(&predictor, &match_id, &1u32, &1u32);

    env.ledger().with_mut(|l| l.timestamp += 20_000);
    // Actual result is 2-2 (Draw)
    client.submit_match_result(&ai_agent, &match_id, &2u32, &2u32);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.points_earned, Some(1));
    assert_eq!(prediction.is_correct, Some(true)); // Result was correct (Draw)

    let (total_points, correct_results, exact_scores, total_matches) =
        client.get_user_score(&predictor, &event_id);
    assert_eq!(total_points, 1);
    assert_eq!(correct_results, 1);
    assert_eq!(exact_scores, 0);
    assert_eq!(total_matches, 1);
}

#[test]
#[ignore] // Ignore until implementation is complete
fn test_get_user_score_aggregates_points_across_multiple_matches() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);

    fund(&env, &xlm_token, &creator, FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &10u32,
        &start_time,
        &end_time,
    );

    let (match_id_1, match_id_2, match_id_3) = env.as_contract(&contract_id, || {
        let m1 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m1,
            &Match::new(
                m1,
                event_id,
                String::from_str(&env, "Team A"),
                String::from_str(&env, "Team B"),
                env.ledger().timestamp() + 10_000,
            ),
        );
        storage::add_event_match(&env, event_id, m1);

        let m2 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m2,
            &Match::new(
                m2,
                event_id,
                String::from_str(&env, "Team C"),
                String::from_str(&env, "Team D"),
                env.ledger().timestamp() + 20_000,
            ),
        );
        storage::add_event_match(&env, event_id, m2);

        let m3 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m3,
            &Match::new(
                m3,
                event_id,
                String::from_str(&env, "Team E"),
                String::from_str(&env, "Team F"),
                env.ledger().timestamp() + 30_000,
            ),
        );
        storage::add_event_match(&env, event_id, m3);

        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.add_match();
        event.add_match();
        event.add_match();
        storage::set_event(&env, event_id, &event);

        (m1, m2, m3)
    });

    let predictor = Address::generate(&env);
    client.join_event(&predictor, &invite_code);

    // Match 1: predict exact score (2-1)
    client.submit_prediction(&predictor, &match_id_1, &2u32, &1u32);
    // Match 2: predict correct result but wrong score (predict 1-0, actual 2-0)
    client.submit_prediction(&predictor, &match_id_2, &1u32, &0u32);
    // Match 3: predict wrong result (predict 1-0, actual 0-1)
    client.submit_prediction(&predictor, &match_id_3, &1u32, &0u32);

    env.ledger().with_mut(|l| l.timestamp += 35_000);

    // Grade all three matches
    client.submit_match_result(&ai_agent, &match_id_1, &2u32, &1u32); // Exact match: 4 points
    client.submit_match_result(&ai_agent, &match_id_2, &2u32, &0u32); // Correct result: 1 point
    client.submit_match_result(&ai_agent, &match_id_3, &0u32, &1u32); // Wrong result: 0 points

    let (total_points, correct_results, exact_scores, total_matches) =
        client.get_user_score(&predictor, &event_id);

    assert_eq!(total_points, 5); // 4 + 1 + 0
    assert_eq!(correct_results, 2); // First two were correct result
    assert_eq!(exact_scores, 1); // Only first was exact
    assert_eq!(total_matches, 3);
}
