#[derive(Debug, Clone)]
pub struct ValidatorSlot {
    pub validator_id: String,
    pub slot_number: u64,
    pub start_time_ms: u128,
    pub end_time_ms: u128,
    pub is_active: bool,
}
