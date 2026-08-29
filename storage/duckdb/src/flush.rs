// DuckDB flush paradigm types.
//
// These types were removed from the nexus workspace (the flush surface
// moved to `flush_pending` on FihStorage) but are the concrete flush
// paradigm for the DuckDB cold-storage engine: incremental export of
// hot data to Parquet partitions tracked by a persisted cursor. They
// belong to the engine, not to nexus infrastructure.

use serde::{Deserialize, Serialize};

/// Cursor for incremental flush. Tracks the last flushed position.
///
/// Persisted across scheduler invocations so that `flush_since` exports
/// only data ingested after the last completed flush.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FlushCursor {
    pub last_flushed_at: u64,
    pub partition: String,
}

/// Result of a flush operation.
#[derive(Debug, Clone)]
pub struct FlushResult {
    pub records_flushed: u64,
    pub new_cursor: FlushCursor,
}

impl From<FlushResult> for (u64, FlushCursor) {
    fn from(r: FlushResult) -> Self {
        (r.records_flushed, r.new_cursor)
    }
}
