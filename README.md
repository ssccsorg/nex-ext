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
├── core/            # nex-ext-core: common query contract
│                    #   QueryCapable spec (ColdQuery and friends),
│                    #   moved from nexus interface/query
├── db/              # legacy/tabular database engines
│   ├── duckdb/      # nex-duckdb: DuckDB engine (SQL over parquet views)
│   └── cypher/      # nex-cypher: Cypher-compatible frontend
│                    #   Cypher query -> ColdQuery -> db engine execution
├── kv/              # coord-based key-value engines (planned)
└── graph/           # coord-to-graph execution (planned)
```

## Principles

- One contract, many engines. `QueryCapable` lives in nex-ext-core; the
  contract and its specs stay with the engines, not in the nexus main
  repository.
- Engines are selectable behind the same plug: attach nex-ext and choose
  `duckdb` and DuckDB runs in the nexus network; choose `cypher` and the
  Cypher surface over coord-structured storage is available.
- Query languages are frontends, not infrastructure. Cypher and SQL are
  engine-side concerns.
- Dependency direction is one-way: nex-ext consumes nexus (nex-fih domain
  types); nexus does not depend on nex-ext.
