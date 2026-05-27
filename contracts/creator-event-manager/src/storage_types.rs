use soroban_sdk::{contracttype, Address, String, Symbol};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum length for event title (characters)
pub const MAX_TITLE_LEN: u32 = 200;
/// Maximum length for event description (characters)
pub const MAX_DESCRIPTION_LEN: u32 = 1000;
/// Maximum length for team names (characters)
pub const MAX_TEAM_NAME_LEN: u32 = 100;
/// Required length for invite codes (characters)
pub const INVITE_CODE_LEN: u32 = 8;

// ---------------------------------------------------------------------------
// MatchResult
// ---------------------------------------------------------------------------

/// Possible outcomes of a prediction match.
///
/// Encoded as u8 on the wire: 0 = TeamA, 1 = TeamB, 2 = Draw.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchResult {
    /// First team / option A wins
    TeamA,
    /// Second team / option B wins
    TeamB,
    /// Match ends in a draw / tie
    Draw,
}

impl MatchResult {
    /// Encode to u8 for compact storage and prediction fields.
    pub fn to_u8(&self) -> u8 {
        match self {
            MatchResult::TeamA => 0,
            MatchResult::TeamB => 1,
            MatchResult::Draw => 2,
        }
    }

    /// Decode from u8.  Returns `None` for any value outside 0–2.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MatchResult::TeamA),
            1 => Some(MatchResult::TeamB),
            2 => Some(MatchResult::Draw),
            _ => None,
        }
    }

    /// Convenience alias kept for callers that still use u32.
    pub fn to_u32(&self) -> u32 {
        self.to_u8() as u32
    }

    /// Convenience alias kept for callers that still use u32.
    pub fn from_u32(value: u32) -> Option<Self> {
        if value > u8::MAX as u32 {
            return None;
        }
        Self::from_u8(value as u8)
    }
}

// ---------------------------------------------------------------------------
// EventStatus
// ---------------------------------------------------------------------------

/// Granular status for an event beyond the simple `is_active` boolean.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventStatus {
    /// Open and accepting predictions
    Active,
    /// Closed for new predictions, not yet resolved
    Closed,
    /// All match results submitted and winners verified
    Resolved,
    /// Cancelled by creator or admin
    Cancelled,
    /// Temporarily paused
    Paused,
}

// ---------------------------------------------------------------------------
// DataKey
// ---------------------------------------------------------------------------

/// Unified storage key enum for every piece of contract state.
///
/// Using a single enum keeps key namespacing explicit and avoids collisions
/// between different storage domains.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Global contract configuration (token address, fee %, admin, …)
    Config,

    /// Global monotonic event counter → u64
    EventCounter,

    /// Core event data keyed by event_id
    Event(u64),

    /// Per-event match counter → u64  (event_id)
    MatchCounter(u64),

    /// Individual match keyed by (event_id, match_id)
    Match(u64, u64),

    /// A user's prediction for a specific match  (event_id, match_id, predictor)
    Prediction(u64, u64, Address),

    /// All event_ids a user has joined  (user) → Vec<u64>
    UserEvents(Address),

    /// All participant addresses for an event  (event_id) → Vec<Address>
    EventParticipants(u64),

    /// Invite code → event_id mapping  (8-char Symbol)
    InviteCode(Symbol),

    /// Whether an address has passed KYC / verification
    VerifiedAddress(Address),

    /// Verified winners for an event  (event_id) → Vec<Winner>
    Winners(u64),

    /// Running XLM balance held by the contract treasury
    TreasuryBalance,
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// Core event struct — all information about a creator's prediction event.
///
/// Stored under `DataKey::Event(event_id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Auto-incremented unique identifier
    pub event_id: u64,

    /// Address of the creator; only they can manage the event
    pub creator: Address,

    /// Human-readable title (max `MAX_TITLE_LEN` chars)
    pub title: String,

    /// Full description / rules (max `MAX_DESCRIPTION_LEN` chars)
    pub description: String,

    /// XLM fee (in stroops) the creator paid to create the event
    pub creation_fee_paid: i128,

    /// Unix timestamp when the event was created
    pub created_at: u64,

    /// Whether the event is open for new predictions
    pub is_active: bool,

    /// Whether the event has been cancelled
    pub is_cancelled: bool,

    /// 8-character invite code used for private events
    pub invite_code: Symbol,

    /// Hard cap on participants (0 = unlimited)
    pub max_participants: u32,

    /// Current number of confirmed participants
    pub participant_count: u32,

    /// Number of matches that belong to this event
    pub match_count: u32,
}

impl Event {
    /// Construct a new active, uncancelled event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: u64,
        creator: Address,
        title: String,
        description: String,
        creation_fee_paid: i128,
        created_at: u64,
        invite_code: Symbol,
        max_participants: u32,
    ) -> Self {
        Self {
            event_id,
            creator,
            title,
            description,
            creation_fee_paid,
            created_at,
            is_active: true,
            is_cancelled: false,
            invite_code,
            max_participants,
            participant_count: 0,
            match_count: 0,
        }
    }

    /// Returns `true` when the event can still accept new participants.
    pub fn can_accept_participants(&self) -> bool {
        if !self.is_active || self.is_cancelled {
            return false;
        }
        // max_participants == 0 means unlimited
        self.max_participants == 0 || self.participant_count < self.max_participants
    }

    /// Close the event for new predictions without cancelling it.
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Cancel the event entirely.
    pub fn cancel(&mut self) {
        self.is_active = false;
        self.is_cancelled = true;
    }

    /// Register a new participant.  Returns `Err` if the event is full or inactive.
    pub fn add_participant(&mut self) -> Result<(), &'static str> {
        if self.is_cancelled {
            return Err("Event is cancelled");
        }
        if !self.is_active {
            return Err("Event is not active");
        }
        if self.max_participants > 0 && self.participant_count >= self.max_participants {
            return Err("Event has reached maximum participants");
        }
        self.participant_count += 1;
        Ok(())
    }

    /// Increment the match counter when a new match is added.
    pub fn add_match(&mut self) {
        self.match_count += 1;
    }

    /// Age of the event in seconds relative to `current_timestamp`.
    pub fn get_age_seconds(&self, current_timestamp: u64) -> u64 {
        current_timestamp.saturating_sub(self.created_at)
    }

    /// Validate title and description lengths.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.title.len() == 0 {
            return Err("Title cannot be empty");
        }
        if self.title.len() > MAX_TITLE_LEN {
            return Err("Title exceeds maximum length");
        }
        if self.description.len() > MAX_DESCRIPTION_LEN {
            return Err("Description exceeds maximum length");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Match
// ---------------------------------------------------------------------------

/// A single prediction match within an event.
///
/// Stored under `DataKey::Match(event_id, match_id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    /// Unique identifier scoped to the parent event
    pub match_id: u64,

    /// ID of the parent event
    pub event_id: u64,

    /// Name of the first team / option (max `MAX_TEAM_NAME_LEN` chars)
    pub team_a: String,

    /// Name of the second team / option (max `MAX_TEAM_NAME_LEN` chars)
    pub team_b: String,

    /// Scheduled start time (Unix timestamp in seconds)
    pub match_time: u64,

    /// Whether a result has been submitted
    pub result_submitted: bool,

    /// The winning outcome; `None` until a result is submitted.
    /// Stored as `Option<u32>` (0=TeamA, 1=TeamB, 2=Draw) because Soroban's
    /// `#[contracttype]` does not support `Option<EnumType>` directly.
    pub winning_team: Option<u32>,

    /// Address of the oracle / admin that submitted the result
    pub submitted_by: Option<Address>,

    /// Unix timestamp when the result was submitted
    pub submitted_at: Option<u64>,
}

impl Match {
    /// Create a new pending match.
    pub fn new(
        match_id: u64,
        event_id: u64,
        team_a: String,
        team_b: String,
        match_time: u64,
    ) -> Self {
        Self {
            match_id,
            event_id,
            team_a,
            team_b,
            match_time,
            result_submitted: false,
            winning_team: None,
            submitted_by: None,
            submitted_at: None,
        }
    }

    // -----------------------------------------------------------------------
    // Result management
    // -----------------------------------------------------------------------

    /// Submit a result for this match.
    ///
    /// # Errors
    /// Returns `Err` if a result has already been submitted.
    pub fn submit_result(
        &mut self,
        result: MatchResult,
        submitted_by: Address,
        timestamp: u64,
    ) -> Result<(), &'static str> {
        if self.result_submitted {
            return Err("Result already submitted for this match");
        }
        self.winning_team = Some(result.to_u32());
        self.submitted_by = Some(submitted_by);
        self.submitted_at = Some(timestamp);
        self.result_submitted = true;
        Ok(())
    }

    /// Return the winning `MatchResult`, or `None` if not yet submitted.
    pub fn get_winner(&self) -> Option<MatchResult> {
        self.winning_team.and_then(MatchResult::from_u32)
    }

    /// `true` once a result has been recorded.
    pub fn is_completed(&self) -> bool {
        self.result_submitted
    }

    // -----------------------------------------------------------------------
    // Timing helpers
    // -----------------------------------------------------------------------

    /// `true` if `current_time >= match_time`.
    pub fn has_started(&self, current_time: u64) -> bool {
        current_time >= self.match_time
    }

    /// `true` if the match has started but no result has been submitted yet.
    pub fn is_ready_for_result(&self, current_time: u64) -> bool {
        self.has_started(current_time) && !self.result_submitted
    }

    /// Seconds until the match starts; 0 if already started.
    pub fn time_until_start(&self, current_time: u64) -> u64 {
        if current_time >= self.match_time {
            0
        } else {
            self.match_time - current_time
        }
    }

    /// Seconds since the result was submitted; 0 if no result yet.
    pub fn time_since_result(&self, current_time: u64) -> u64 {
        match self.submitted_at {
            Some(t) => current_time.saturating_sub(t),
            None => 0,
        }
    }

    // -----------------------------------------------------------------------
    // Prediction window
    // -----------------------------------------------------------------------

    /// `true` if predictions are still open.
    ///
    /// Predictions close `prediction_cutoff_minutes` before `match_time` and
    /// are always closed once a result has been submitted.
    pub fn allows_predictions(&self, current_time: u64, prediction_cutoff_minutes: u64) -> bool {
        let cutoff = self.match_time.saturating_sub(prediction_cutoff_minutes * 60);
        current_time < cutoff && !self.result_submitted
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate team names and internal state consistency.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.team_a.len() == 0 {
            return Err("Team A name cannot be empty");
        }
        if self.team_a.len() > MAX_TEAM_NAME_LEN {
            return Err("Team A name exceeds maximum length");
        }
        if self.team_b.len() == 0 {
            return Err("Team B name cannot be empty");
        }
        if self.team_b.len() > MAX_TEAM_NAME_LEN {
            return Err("Team B name exceeds maximum length");
        }
        if self.team_a == self.team_b {
            return Err("Team names must be different");
        }

        // Result consistency
        if self.result_submitted {
            if self.winning_team.is_none() {
                return Err("Result submitted but winning_team is None");
            }
            if self.submitted_by.is_none() {
                return Err("Result submitted but submitted_by is None");
            }
            if self.submitted_at.is_none() {
                return Err("Result submitted but submitted_at is None");
            }
            // Validate the encoded value is a legal outcome
            if let Some(v) = self.winning_team {
                if v > 2 {
                    return Err("winning_team value must be 0 (TeamA), 1 (TeamB), or 2 (Draw)");
                }
            }
        } else {
            if self.winning_team.is_some() {
                return Err("winning_team set but result_submitted is false");
            }
            if self.submitted_at.is_some() {
                return Err("submitted_at set but result_submitted is false");
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Prediction
// ---------------------------------------------------------------------------

/// A user's prediction for a single match inside an event.
///
/// Stored under `DataKey::Prediction(event_id, match_id, predictor)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prediction {
    /// Address of the user who placed this prediction
    pub predictor: Address,

    /// Parent event identifier
    pub event_id: u64,

    /// Match this prediction is for
    pub match_id: u64,

    /// Predicted outcome: 0 = Team A, 1 = Team B, 2 = Draw.
    /// Stored as `u32` because Soroban's `#[contracttype]` does not support `u8`.
    pub predicted_winner: u32,

    /// Unix timestamp when the prediction was submitted
    pub predicted_at: u64,

    /// `Some(true)` = correct, `Some(false)` = wrong, `None` = not yet graded
    pub is_correct: Option<bool>,
}

impl Prediction {
    /// Create a new ungraded prediction.
    pub fn new(
        predictor: Address,
        event_id: u64,
        match_id: u64,
        predicted_winner: u32,
        predicted_at: u64,
    ) -> Self {
        Self {
            predictor,
            event_id,
            match_id,
            predicted_winner,
            predicted_at,
            is_correct: None,
        }
    }

    /// Grade this prediction against the actual match result.
    pub fn grade(&mut self, actual_result: &MatchResult) {
        self.is_correct = Some(self.predicted_winner == actual_result.to_u32());
    }

    /// `true` if the prediction has been graded and was correct.
    pub fn is_winner(&self) -> bool {
        self.is_correct == Some(true)
    }

    /// Validate that `predicted_winner` encodes a legal outcome (0, 1, or 2).
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.predicted_winner > 2 {
            return Err("predicted_winner must be 0 (Team A), 1 (Team B), or 2 (Draw)");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Winner
// ---------------------------------------------------------------------------

/// Records a user who correctly predicted every match in an event.
///
/// Stored inside the `Vec<Winner>` at `DataKey::Winners(event_id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Winner {
    /// Address of the winning predictor
    pub user: Address,

    /// Event they won
    pub event_id: u64,

    /// How many matches they predicted correctly (should equal event.match_count)
    pub total_correct_predictions: u32,

    /// Unix timestamp when the win was verified on-chain
    pub verified_at: u64,
}

impl Winner {
    /// Construct a new verified winner record.
    pub fn new(
        user: Address,
        event_id: u64,
        total_correct_predictions: u32,
        verified_at: u64,
    ) -> Self {
        Self {
            user,
            event_id,
            total_correct_predictions,
            verified_at,
        }
    }
}

// ---------------------------------------------------------------------------
// EventMetadata  (unchanged — kept for backward compatibility)
// ---------------------------------------------------------------------------

/// Extended metadata stored separately from the core `Event` to keep the
/// hot-path struct lean.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventMetadata {
    /// Category label, e.g. "Sports", "Crypto", "Politics"
    pub category: String,

    /// Comma-separated discovery tags
    pub tags: String,

    /// Minimum participants required for the event to be valid
    pub min_participants: u32,

    /// Maximum participants allowed (mirrors `Event::max_participants`)
    pub max_participants: u32,

    /// Unix timestamp when predictions close
    pub end_time: u64,

    /// Unix timestamp when results must be submitted
    pub resolution_time: u64,

    /// Whether the event requires an invite code to join
    pub is_invite_only: bool,

    /// Creator's reputation score at the time of event creation
    pub creator_reputation: u32,
}

impl EventMetadata {
    pub fn new(
        category: String,
        tags: String,
        min_participants: u32,
        max_participants: u32,
        end_time: u64,
        resolution_time: u64,
        is_invite_only: bool,
        creator_reputation: u32,
    ) -> Self {
        Self {
            category,
            tags,
            min_participants,
            max_participants,
            end_time,
            resolution_time,
            is_invite_only,
            creator_reputation,
        }
    }

    /// `true` while predictions are still open.
    pub fn is_prediction_phase(&self, current_time: u64) -> bool {
        current_time < self.end_time
    }

    /// `true` after predictions close but before the resolution deadline.
    pub fn is_resolution_phase(&self, current_time: u64) -> bool {
        current_time >= self.end_time && current_time < self.resolution_time
    }

    /// `true` once the resolution deadline has passed.
    pub fn should_auto_resolve(&self, current_time: u64) -> bool {
        current_time >= self.resolution_time
    }
}
