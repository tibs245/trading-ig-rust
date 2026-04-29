//! Operations domain — manage API application keys.
//!
//! Entry point: [`crate::IgClient::operations`].
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! use trading_ig::operations::UpdateApplicationRequest;
//! use trading_ig::operations::ApplicationStatus;
//!
//! // List all application keys
//! let apps = client.operations().applications().await?;
//!
//! // Update allowances
//! let updated = client.operations().update_application(UpdateApplicationRequest {
//!     api_key: "my-api-key".into(),
//!     status: ApplicationStatus::Enabled,
//!     allowance_account_overall: 60,
//!     allowance_account_trading: 10,
//! }).await?;
//!
//! // Disable the current key (irreversible without the web UI)
//! let disabled = client.operations().disable_current_key().await?;
//! # Ok(()) }
//! ```

pub mod api;
pub mod models;

pub use api::OperationsApi;
pub use models::{Application, ApplicationStatus, UpdateApplicationRequest};
