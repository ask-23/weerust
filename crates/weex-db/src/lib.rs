//! Database access layer for WeeWX MySQL schema
//!
//! Uses existing schema from Python WeeWX - NO migrations.
//! Assumes schema is already created and matches production layout.

pub mod client;
pub mod queries;
pub mod schema;

pub use client::*;
pub use schema::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] sqlx::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Record not found")]
    NotFound,

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

pub type DbResult<T> = Result<T, DbError>;
