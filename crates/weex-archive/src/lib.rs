//! Archive interval aggregator
//!
//! Accumulates weather packets over configured intervals and
//! generates archive records for database storage.

pub mod aggregator;
pub mod buffer;

pub use aggregator::*;
pub use buffer::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] weex_db::DbError),

    #[error("Aggregation error: {0}")]
    AggregationError(String),

    #[error("Invalid interval: {0}")]
    InvalidInterval(String),

    #[error("Buffer overflow")]
    BufferOverflow,
}

pub type ArchiveResult<T> = Result<T, ArchiveError>;
