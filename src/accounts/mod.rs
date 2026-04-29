//! Accounts domain: list accounts and manage preferences.
//!
//! Entry point is [`AccountsApi`], obtained via [`crate::IgClient::accounts`].

pub mod models;

mod api;

use crate::IgClient;

pub use models::{Account, AccountBalance, AccountPreferences, AccountType, UpdatePreferences};

/// Typed accessor for all accounts-related IG endpoints.
///
/// Obtain via [`crate::IgClient::accounts`].
#[derive(Debug)]
pub struct AccountsApi<'a> {
    pub(crate) client: &'a IgClient,
}
