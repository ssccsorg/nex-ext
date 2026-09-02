// End-to-end verification: Cypher query to DuckDB execution.
//
// This is the verification framework for the original restoration goal:
// choose `duckdb` in the nex-ext adapter and DuckDB runs in the nexus
// network. The full path is exercised here:
//
//   Cypher string
//     -> nex-cypher (parse, cold-route)      -> ColdQuery
//     -> nex-duckdb (cypher_sql translate)   -> SQL
//     -> DuckDbStorage (QueryCapable)        -> result rows
//
// Data is injected as parquet files into the storage's partition layout
// (facts/partition={project_id}/), the same layout the flush machinery
// writes, then queried through the public QueryCapable surface.

use nex_cypher::Plan;
use nex_duckdb::DuckDbStorage;
use nex_fih::QueryCapable;
use nex_fih::{Content, StorageRead};
use std::collections::BTreeMap;
use tempfile::TempDir;

/// Create a DuckDbStorage over an empty temp directory.
fn empty_storage() -> (DuckDbStorage, TempDir) {
    let tempdir = TempDir::new().unwrap();
    let base = tempdir.path().to_str().unwrap().to_string();
    std::fs::create_dir_all(tempdir.path().join("facts")).unwrap();
    std::fs::create_dir_all(tempdir.path().join("intents")).unwrap();
    std::fs::create_dir_all(tempdir.path().join("hints")).unwrap();
    let storage = DuckDbStorage::new(&base, "e2e").unwrap();
    (storage, tempdir)
}

/// Write one fact parquet into the storage's project partition.
fn inject_fact(storage: &DuckDbStorage, fact_id: &str, origin: &str, content: &str) {
    let project_id = storage.project_id();
    let facts_dir = format!("{}/facts/partition={}", storage.base_path(), project_id);
    std::fs::create_dir_all(&facts_dir).unwrap();
    let path = format!("{}/{}.parquet", facts_dir, fact_id);
    let sql = format!(
        "COPY (SELECT '{}' as fact_id, '{}' as origin, '{}' as content, 'tester' as creator, '1000' as created_at) TO '{}' (FORMAT PARQUET);",
        fact_id, origin, content, path
    );
    storage.conn().lock().unwrap().execute(&sql, []).unwrap();
}

fn text_content_value(row: &BTreeMap<String, Content>, col: &str) -> String {
    let c = row.get(col).expect("column present");
    assert_eq!(c.mime_type, "text/plain", "column {col} should be text");
    String::from_utf8(c.data.clone()).unwrap()
}

// ── External plan path: cyrs lowering -> ColdQuery -> DuckDB ─────────────

#[test]
fn external_cypher_query_runs_against_duckdb() {
    let (storage, _td) = empty_storage();
    inject_fact(&storage, "f1", "test", "\"alpha\"");
    inject_fact(&storage, "f2", "test", "\"beta\"");
    inject_fact(&storage, "f3", "other", "\"gamma\"");

    let plan = Plan::from_cyrs("MATCH (f:Fact) WHERE f.origin = 'test' RETURN f.fact_id").unwrap();
    let cold = plan.to_cold_query().expect("cold-eligible");
    let rows = match storage.query_plan(&cold) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("cold query: {cold:?}");
            panic!("query_plan failed: {e}");
        }
    };

    let ids: Vec<String> = rows
        .iter()
        .map(|r| text_content_value(r, "fact_id"))
        .collect();
    assert!(ids.contains(&"f1".into()), "rows: {ids:?}");
    assert!(ids.contains(&"f2".into()), "rows: {ids:?}");
    assert!(!ids.contains(&"f3".into()), "filter leaked: {ids:?}");
    assert_eq!(ids.len(), 2);
}

#[test]
fn external_cypher_scan_returns_all_facts() {
    let (storage, _td) = empty_storage();
    inject_fact(&storage, "f1", "a", "\"1\"");
    inject_fact(&storage, "f2", "b", "\"2\"");

    let plan = Plan::from_cyrs("MATCH (f:Fact) RETURN f.fact_id").unwrap();
    let cold = plan.to_cold_query().expect("cold-eligible");
    let rows = storage.query_plan(&cold).expect("query_plan executes");
    assert_eq!(rows.len(), 2, "scan should see both facts");
}

// ── Internal plan path: parse_query (PlanIR) -> ColdQuery -> DuckDB ─────

#[test]
fn internal_cypher_query_runs_against_duckdb() {
    let (storage, _td) = empty_storage();
    inject_fact(&storage, "f1", "test", "\"alpha\"");
    inject_fact(&storage, "f2", "other", "\"beta\"");

    // from_internal runs parse_query, so this exercises the parser too.
    let plan =
        Plan::from_internal("MATCH (f:Fact) WHERE f.origin = 'test' RETURN f.fact_id").unwrap();
    let cold = plan.to_cold_query().expect("cold-eligible");
    let rows = storage.query_plan(&cold).expect("query_plan executes");
    let ids: Vec<String> = rows
        .iter()
        .map(|r| text_content_value(r, "fact_id"))
        .collect();
    assert_eq!(ids, vec!["f1".to_string()]);
}

// ── Relationship patterns are NOT cold-eligible: no SQL is attempted ────

#[test]
fn graph_pattern_is_not_routed_to_duckdb() {
    let (storage, _td) = empty_storage();
    let plan = Plan::from_cyrs("MATCH (f:Fact)-[:drives]->(i:Intent) RETURN f.fact_id").unwrap();
    assert!(
        plan.to_cold_query().is_none(),
        "relationship patterns must not reach the tabular engine"
    );
}
