# nex-ext: external engine runners

The `ext/` directory (nex-ext) is a launcher harness for external Blackboard
engines on the nexus stack. Each engine is an adapter implementing the
`AbstractEngine` contract in `_runner/base.py`; the shared `_runner` package
provides the CLI, dependency checks, embedding-dimension probing, tunnel
management, and per-environment defaults.

This is a runner harness, not a Rust crate and not part of the nexus
workspace. It is invoked from the host with Docker and a local
LM Studio-compatible API server.

## Layout

| Path | Purpose |
|------|---------|
| `_runner/` | Shared launcher: CLI (`cli.py`), engine registry (`engines.py`), base contract (`base.py`), checks (`checks.py`), defaults (`defaults.py`), tunnel manager (`tunnel.py`) |
| `lightrag/` | LightRAG adapter; local image built from `Dockerfile` |
| `graphiti/` | Graphiti adapter (FastAPI wrapper over the graphiti-core library); local image from `Dockerfile`, Neo4j 5 sidecar |
| `memgraph/` | Memgraph adapter (HTTP-to-Bolt proxy in `server.py`); local proxy image from `Dockerfile`, Memgraph platform container |
| `edgequake/` | EdgeQuake adapter; prebuilt GHCR images via `docker-compose.yml` profile |
| `tests/` | Standard-library unit tests (`unittest`) for the pure logic in `_runner/` and the engine assets |
| `.env.example` | Environment contract template; copy to `.env` and edit |

Host entry points live in `scripts/`: `run-lightrag.sh`, `run-graphiti.sh`,
`run-memgraph.sh`, `run-edgequake.sh`. Each one execs the generic
`python3 -m _runner --engine <name>` entry with `EXT_DIR` set to `ext/`.

## Usage

```bash
cd ext
cp .env.example .env   # edit for your LLM API and engine settings
./scripts/run-lightrag.sh      # or run-graphiti.sh / run-memgraph.sh / run-edgequake.sh
```

Options and behavior:

- `--refresh` clears engine data before starting.
- `TIMEOUT_SEC` bounds the health-check wait (default 120).
- RAG engines (lightrag, graphiti, edgequake) run a pre-flight that checks the
  LM Studio-compatible API and cloudflared, then start a Cloudflare tunnel.
  Memgraph is a pure graph database proxy: it skips the LLM and tunnel steps.

## Environment contract

Defaults live in `_runner/defaults.py`; every value can be overridden by
environment variables before launch. `DEPLOYMENT_ENV=prod` selects the
production defaults, `dev` (the default) selects local LM Studio defaults.
The authoritative variable list is in `.env.example`.

Notable variables:

| Variable | Meaning |
|----------|---------|
| `LLM_MODEL` | Chat model name |
| `EMBEDDING_MODEL` | Embedding model name |
| `EMBEDDING_DIM` | Embedding dimension; when set, runners use it directly instead of probing the API |
| `API_BASE_URL` / `LMSTUDIO_URL` | OpenAI-compatible API base URL |
| `DEPLOYMENT_ENV` | `dev` or `prod` default set |
| per-engine `*_PORT`, `*_IMAGE`, `*_CONTAINER` | Engine-specific overrides |

## Verification

The Python code is verified with the standard library only:

```bash
python3 -m compileall -q ext
PYTHONPATH=ext python3 -m unittest discover -s ext/tests -t ext -v
```

CI runs both gates in `.github/workflows/ext.yml` on `ext/**` and runner
script changes. Container image builds are available there as a manual
`workflow_dispatch` job (`scripts/run-build-containers.sh` builds the three
local Dockerfiles; EdgeQuake pulls prebuilt GHCR images).

## Constraints

- Live engine smoke tests need a Docker daemon, an LM Studio-compatible API,
  and a cloudflared tunnel; they are intentionally not part of the automated
  gate.
- The runner downloads cloudflared to the current directory when it is missing
  and attempts a `sudo` install; run on a host where that is acceptable.
