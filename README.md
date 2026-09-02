# nex-ext

Storage extensions for the Nexus network.

nex-ext is the attachment tier of the ecosystem: queryable storage engines
that plug into the Nexus network. It is distinct from chton, which owns the
coordinate materialization substrate. nex-ext engines sit above chton and
nexus and expose stored data through query languages (SQL, Cypher, and
others).

```
tagma     coordinate spec (Coord, CoordPath)
chton     coordinate materialization (physical storage substrate)
nexus     semantics (FIH), the nexus network
nex-ext   engines attached to the network (this repository)
```

## Structure

```
nex-ext/
├── db/
│   ├── duckdb/      # nex-duckdb: DuckDB engine (SQL over parquet views)
│   └── cypher/      # nex-cypher: Cypher-compatible frontend
│                    #   Cypher query -> ColdQuery -> db engine execution
├── e2e/             # end-to-end verification framework
└── kv/              # coord-based key-value engines (planned)
```

The FIH query contract (ColdQuery, QueryCapable) lives in the nexus
repository as part of fih-model, the stable FIH storage-trait family
(StorageRead, FilterCapable, ScanCapable, ...). Engine crates here
consume it from nex-fih; the nexus runtime (nexd launching nex) speaks it
as a runtime boundary, so the contract stays with the stable core.

## Principles

- One contract, many engines. `QueryCapable` is part of fih-model in the
  nexus repository; the contract and its specs stay with the stable core
  because it is a runtime boundary (nexd launches nex).
- Engines are selectable behind the same plug: attach nex-ext and choose
  `duckdb` and DuckDB runs in the nexus network; choose `cypher` and the
  Cypher surface over coord-structured storage is available.
- Query languages are frontends, not infrastructure. Cypher and SQL are
  engine-side concerns.
- Dependency direction is one-way: nex-ext consumes nexus (fih-model
  contract and domain types); nexus does not depend on nex-ext.
