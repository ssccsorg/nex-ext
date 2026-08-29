# nex-db

Universal database adapter layer for the Nexus ecosystem.

nex-db is the thin `nex` film applied to database engines: it attaches a
database (DuckDB today, others later) to the Nexus network through the
nex protocol. It is not a Nexus database; it is the standard plug that
makes any database queryable from Nexus.

## Structure

```
nex-db/
├── core/            # common DB abstraction (QueryCapable contract surface)
└── storage/
    ├── duckdb/      # nex-duckdb: DuckDB engine implementation
    └── cypher/      # nex-cypher: Cypher-compatible frontend (planned)
```

## Principles

- One interface, many engines. The `QueryCapable` contract in
  `interface/query` (nexus workspace) is the single connection point.
- The contract stays in nexus; nex-db consumes it as a git dependency.
  Nexus does not depend on nex-db, so the dependency is one-directional.
- Query languages are frontends, not infrastructure. Cypher and SQL are
  engine-side concerns; nexus infrastructure holds the contract only.
