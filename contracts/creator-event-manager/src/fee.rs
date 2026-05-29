use soroban_sdk::{Address, Env, token::Client as TokenClient};

use crate::admin;
use crate::storage_types::DataKey;
use crate::token::TokenHelper;

/// Errors for fee module operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FeeError {
    Paused = 1,
    Unauthorized = 2,
    InvalidAddress = 3,
    InvalidAmount = 4,
    InsufficientBalance = 5,
    TransferFailed = 6,
}

/// Return the XLM balance of the configured treasury address.
pub fn get_treasury_balance(env: &Env) -> i128 {
    let treasury = admin::get_treasury(env).unwrap_or_else(|| panic!("not_initialized"));
    let xlm_token = admin::get_xlm_token(env).unwrap_or_else(|| panic!("not_initialized"));
    TokenHelper::get_balance(env, &xlm_token, &treasury)
}

/// Withdraw XLM from the treasury to `to` address. Only admin may call.
pub fn withdraw_fees(env: &Env, caller: Address, to: Address, amount: i128) -> Result<(), FeeError> {
    // Verify not paused
    if admin::is_paused(env) {
        return Err(FeeError::Paused);
    }

    // Verify caller is admin
    caller.require_auth();
    let is_admin = env
        .storage()
        .persistent()
        .get::<DataKey, Address>(&DataKey::Admin(caller.clone()))
        .is_some();
    if !is_admin {
        return Err(FeeError::Unauthorized);
    }

    // Validate `to` address
    if to == env.current_contract_address() {
        return Err(FeeError::InvalidAddress);
    }

    if amount <= 0 {
        return Err(FeeError::InvalidAmount);
    }

    let treasury = admin::get_treasury(env).unwrap_or_else(|| panic!("not_initialized"));
    let xlm_token = admin::get_xlm_token(env).unwrap_or_else(|| panic!("not_initialized"));

    let balance = TokenHelper::get_balance(env, &xlm_token, &treasury);
    if balance < amount {
        return Err(FeeError::InsufficientBalance);
    }

    // If the treasury equals the contract, perform a contract-send transfer.
    let contract = env.current_contract_address();
    if treasury == contract {
        TokenHelper::transfer(env, &xlm_token, &to, amount)
            .map_err(|_| FeeError::TransferFailed)?;
        return Ok(());
    }

    // Otherwise attempt a direct token transfer from the treasury to `to`.
    let client = TokenClient::new(env, &xlm_token);
    client.transfer(&treasury, &to, &amount);

    Ok(())
}
