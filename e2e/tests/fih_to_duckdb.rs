// End-to-end verification: FihStorage (the nexus network) to DuckDB.
//
// The previous e2e injected parquet directly. This one drives the real
// write side: facts are submitted to a FihStorage instance over an
// in-memory FileIo, flushed, read back as materialized state, and
// exported into the DuckDbStorage partition layout. The facts are then
// answerable through the Cypher path.
//
//   FihStorage (submit_fact, flush_pending, read_state)
//     -> export to parquet partition                    (this test's bridge)
//     -> DuckDbStorage views
//     -> nex-cypher (parse, cold-route) -> ColdQuery
//     -> QueryCapable::query_plan                       -> result rows
//
// The export bridge is intentionally a small local helper here: it is the
// attachment point between the nex network and the engine, and it proves
// the mapping (record to tabular row) the network adapter will own.

use nex_cypher::Plan;
use nex_duckdb::DuckDbStorage;
use nex_fih::QueryCapable;
use nex_fih::io::file_io::{FileIo, IoFuture};
use nex_fih::{AsyncFactCapable, AsyncStorageRead, Content, Fact, FihStorage, StorageRead};
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::TempDir;

// ── Minimal in-memory FileIo for the FihStorage instance ───────────────

#[derive(Default)]
struct MemIo {
    data: Mutex<HashMap<String, Vec<u8>>>,
}

impl FileIo for MemIo {
    fn read<'a>(&'a self, path: &'a str) -> IoFuture<'a, Option<Vec<u8>>> {
        let data = &self.data;
        Box::pin(async move { Ok(data.lock().unwrap().get(path).cloned()) })
    }
    fn write<'a>(&'a self, path: &'a str, data: &'a [u8]) -> IoFuture<'a, ()> {
        let map = &self.data;
        Box::pin(async move {
            map.lock().unwrap().insert(path.to_string(), data.to_vec());
            Ok(())
        })
    }
    fn list<'a>(&'a self, prefix: &'a str) -> IoFuture<'a, Vec<String>> {
        let map = &self.data;
        Box::pin(async move {
            Ok(map
                .lock()
                .unwrap()
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect())
        })
    }
    fn delete<'a>(&'a self, path: &'a str) -> IoFuture<'a, ()> {
        let map = &self.data;
        Box::pin(async move {
            map.lock().unwrap().remove(path);
            Ok(())
        })
    }
}

fn empty_duckdb() -> (DuckDbStorage, TempDir) {
    let tempdir = TempDir::new().unwrap();
    let base = tempdir.path().to_str().unwrap().to_string();
    std::fs::create_dir_all(tempdir.path().join("facts")).unwrap();
    std::fs::create_dir_all(tempdir.path().join("intents")).unwrap();
    std::fs::create_dir_all(tempdir.path().join("hints")).unwrap();
    let storage = DuckDbStorage::new(&base, "e2e").unwrap();
    (storage, tempdir)
}

/// Export materialized facts into the DuckDbStorage partition layout.
///
/// The content column holds what both DuckDbStorage readers accept: JSON
/// for application/json content, and the raw bytes for text content (the
/// readers' JSON parse falls back to text/plain for non-JSON, and the
/// raw query converter emits text/plain for unquoted values).
fn export_facts_to_duckdb(storage: &DuckDbStorage, facts: &[Fact]) {
    let project_id = storage.project_id();
    let facts_dir = format!("{}/facts/partition={}", storage.base_path(), project_id);
    std::fs::create_dir_all(&facts_dir).unwrap();
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let conn = storage.conn().lock().unwrap();
    for (i, fact) in facts.iter().enumerate() {
        // Store the raw content bytes: JSON content stays JSON, text
        // content stays plain, and both DuckDbStorage readers agree.
        let content_col = String::from_utf8_lossy(&fact.content.data).into_owned();
        let path = format!("{facts_dir}/fih-{i}.parquet");
        let sql = format!(
            "COPY (SELECT '{}' as fact_id, '{}' as origin, '{}' as content, '{}' as creator, '{}' as created_at) TO '{}' (FORMAT PARQUET);",
            fact.id,
            fact.origin.replace('\'', "''"),
            content_col.replace('\'', "''"),
            fact.creator.replace('\'', "''"),
            now_secs,
            path
        );
        conn.execute(&sql, []).unwrap();
    }
}

// ── The full network path ───────────────────────────────────────────────

#[test]
fn fih_storage_facts_are_queryable_through_duckdb() {
    let (duck, _td) = empty_duckdb();

    // 1. Ingest into the nex network (FihStorage over in-memory FileIo).
    let fih = FihStorage::new(MemIo::default(), "e2e");
    let f1 = Fact::new(
        "test".into(),
        Content {
            mime_type: "text/plain".into(),
            data: b"hello mcu".to_vec(),
        },
        "harness".into(),
    );
    let f2 = Fact::new(
        "other".into(),
        Content {
            mime_type: "text/plain".into(),
            data: b"second fact".to_vec(),
        },
        "harness".into(),
    );
    let id1 = futures_executor::block_on(fih.submit_fact(&f1)).expect("submit f1");
    let id2 = futures_executor::block_on(fih.submit_fact(&f2)).expect("submit f2");
    futures_executor::block_on(fih.flush_pending()).expect("flush pending");

    // 2. Read the materialized state and export it into DuckDB.
    let state = futures_executor::block_on(fih.read_state());
    assert_eq!(state.facts.len(), 2, "two facts materialized");
    export_facts_to_duckdb(&duck, &state.facts);

    // 3. Query through Cypher against DuckDbStorage.
    let plan =
        Plan::from_cyrs("MATCH (f:Fact) WHERE f.origin = 'test' RETURN f.fact_id, f.content")
            .unwrap();
    let cold = plan.to_cold_query().expect("cold-eligible");
    let rows = duck.query_plan(&cold).expect("query_plan executes");

    assert_eq!(rows.len(), 1, "only the origin='test' fact");
    let row = &rows[0];
    let fact_id = row.get("fact_id").expect("fact_id column");
    assert_eq!(String::from_utf8_lossy(&fact_id.data), id1.to_string());
    let content = row.get("content").expect("content column");
    assert_eq!(content.mime_type, "text/plain");
    assert_eq!(content.data, b"hello mcu", "content round-trips");

    // 4. The other origin is reachable with its own filter.
    let plan2 =
        Plan::from_cyrs("MATCH (f:Fact) WHERE f.origin = 'other' RETURN f.fact_id").unwrap();
    let cold2 = plan2.to_cold_query().expect("cold-eligible");
    let rows2 = duck.query_plan(&cold2).expect("query_plan executes");
    assert_eq!(rows2.len(), 1);
    let row2 = &rows2[0];
    let fact_id2 = row2.get("fact_id").expect("fact_id column");
    assert_eq!(String::from_utf8_lossy(&fact_id2.data), id2.to_string());
}
