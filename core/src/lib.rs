// nex-db core: common database abstraction for the Nexus ecosystem.
//
// The contract surface lives in `interface-query` in the nexus workspace
// (`QueryCapable`, `ColdQuery`, `ColdFilter`, `ColdOrder`, `AggregateDef`).
// nex-db consumes that contract as a git dependency and re-exports it so
// engine crates (storage/duckdb, storage/cypher) share one path.

pub use interface_query::{AggregateDef, ColdFilter, ColdOrder, ColdQuery, QueryCapable};
