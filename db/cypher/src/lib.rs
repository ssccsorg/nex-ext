// nex-cypher: Cypher-compatible frontend of the nex-db adapter.
//
// The frontend parses Cypher queries through the cyrs pipeline (syntax,
// AST, HIR) and lowers them into the lightweight PlanIR. Cold-eligible
// plans (tabular scans without graph patterns) lower further to
// `ColdQuery`, which nex-duckdb executes as SQL. Graph-shaped plans are
// the coord-to-graph execution target (tagma substrate), a later phase.
//
// nex-cypher sits at the same hierarchy level as nex-duckdb: both are
// selectable engines behind the nex-db adapter.

pub mod parser;
pub mod plan;

// Re-export common query types from interface-query for convenience.
pub use nex_ext_core::{AggregateDef, ColdFilter, ColdOrder, ColdQuery, QueryCapable};
pub use parser::parse_query;
pub use plan::*;
