//! Error messages for use with `.expect()` and explanations
//! for why the `Err` or `None` variant is not handled.

/// # Reason
/// This unwrap is guaranteed to succeed
pub const EXP_SAFE: &str = "Unexpected error";
/// # Reason
/// This unwrap is guaranteed to succeed because the item is properly initialized
pub const EXP_INIT: &str = "Not properly initialized";
/// # Reason
/// This channel receiver is expected to be open
pub const EXP_RX: &str = "Could not send player message";

/// # Reason
/// Panic if initialization fails because the failure case cannot be handled
pub const INIT_ERR: &str = "Initialization failed";
/// # Reason
/// Panic when activating a non-existent `gio` action
pub const ACTION_ERR: &str = "Could not activate action";
