# Equation Layer — Full Production Implementation Plan

## Context

The architecture (`equation_layer_architecture.md`) defines a 7-layer + 3 cross-cutting reasoning infrastructure. The current codebase has solid data models, storage schema, config, gating, and epistemic ledger. Every computational component is mocked. Four layers are entirely missing. The ingestion pipeline handles CSV only, with bugs. Rust scaffold exists but has no implementation.

Goal: production-grade implementation of the full product per the architecture, except dashboard (last).

## What Stays (Salvageable)

| File | Verdict | Notes |
|------|---------|-------|
| `models.py` | Keep | Frozen dataclasses are correct. May add fields for new layers. |
| `config.py` | Keep | Thresholds match architecture. Add config for LLM provider, Valkey, DB connectors. |
| `storage.py` | Keep schema, rewrite functions | Schema is complete. Remove hardcoded string patterns, add proper error handling, connection pooling. |
| `epistemic_ledger.py` | Keep, minor enhance | Add structured event types for all 7 layers. Add batch append. |
| `gating.py` | Keep | Correct. Add drift and conflict checks. |
| `domain_router.py` | Rewrite | Current 6-keyword match is inadequate. Needs ontology loading + library partition selection. |
| `__init__.py` | Keep | Empty, fine. |

## What Gets Replaced (Full Rewrite)

### Phase 1: Core Math Engine (L3, L4, L5)

**`fitting.py`** — Real SciPy fitting:
- `fit_canonical_families(x, y)` → for each of 6 families (linear, power, exponential, logarithmic, logistic, polynomial≤2), use `scipy.optimize.least_squares` with trust-region reflective (TRF) method
- `compute_fit_stats(model_fn, x, y, params, x_holdout, y_holdout)` → real R², adjusted R², AIC, holdout MAPE, Durbin-Watson residual randomness test
- `structural_validity(params, family, domain)` → domain constraint checks (e.g., price elasticity sign)
- `split_holdout(x, y, ratio, temporal)` → time-aware split for time-series, random split otherwise
- `choose_best(candidates)` → AIC-based selection among those passing all thresholds
- Remove `_mock_fit_stats`, remove `Candidate` dataclass (use `EquationFit` directly)

**`compute.py`** — Real SymPy:
- `evaluate_equation(equation, x_values, point)` → `sympify` the expression, `lambdify` for numeric eval, compute at point
- `symbolic_derivatives(expr, variables)` → `sympy.diff` for ∂f/∂xᵢ, ∂²f/∂xᵢ², ∂²f/∂xᵢ∂xⱼ
- `sensitivity_map(derivatives, point)` → elasticities at evaluation point
- Remove hardcoded arithmetic

**`probabilistic.py`** — Real Laplace + BMA:
- `laplace_approximation(params, covariance, equation, point, n_samples=10000)` → sample from multivariate Gaussian N(θ*, Σ), evaluate equation at each sample, compute E[answer], σ, 95% CI, P(answer > k)
- `bayesian_model_averaging(passing_candidates, x, y, point)` → weight = exp(-AIC/2), normalized; compute weighted mean/variance across all families
- `extract_covariance(fit_result)` → extract covariance from scipy.optimize result (Jacobian-based Hessian approximation)
- `tail_risk(samples, threshold)` → P(answer < threshold)
- Remove hardcoded `mean * 0.1`

**`pysr_discovery.py`** — Real PySR integration:
- `run_pysr(x, y, niterations=40, binary_operators, unary_operators)` → actually call PySR via subprocess or Python API
- Falls back gracefully if PySR not installed (returns empty result)
- `pysr_to_equation_fit(pysr_model, x, y)` → convert PySR output to EquationFit format
- Remove hardcoded mocks

### Phase 2: Missing Layers (L1, L2, L7)

**`epistemic_parser.py`** (NEW — Layer 1):
- `parse_query(query, domain, ontology)` → LLM call with strict JSON schema (Anthropic `tool_use` or OpenAI `response_format`)
- Extracts ONLY: variables [{name, unit, entity, time_window}], assumptions [], domain
- Schema is enforced by the API — no equation forms allowed in output
- `build_extraction_prompt(query, domain_context)` → few-shot examples per domain
- Returns `ExtractionResult` — no equation proposal field in the model

**`correlation.py`** (NEW — Layer 2):
- `correlation_matrix(x_values, y_values)` → Pearson + Spearman for all pairs, `scipy.stats`
- `partial_correlation(x, y, controlling_for)` → strips confounders
- `drift_detector(historical_rho, current_rho, threshold=0.2)` → flag if |Δρ| > threshold
- `validate_relationships(extraction, observations)` → blocks fit if |ρ| < min_correlation for claimed variable pairs
- Returns `CorrelationResult` with ρ map and drift flags

**`formatter.py`** (NEW — Layer 7):
- `format_output(solve_result, audience="analyst")` → LLM converts structured `SolveResult` to prose
- Speaks only — never reasons about math, never proposes equations
- `format_fallback(fallback_result)` → produces user-facing explanation of why the gate failed
- Template-based fallback if LLM unavailable

### Phase 3: Resilience Layer

**`fallback_router.py`** (NEW):
- `route_failure(gate_result, equation, extraction)` → maps gate reason to actionable response:
  - `insufficient_observations` → data collection plan (which variables, how many more)
  - `low_r2_adj` / `high_holdout_mape` → human review queue entry
  - `residuals_not_random` → structural break flag
  - `drift_detected` → before/after comparison, structural change alert
  - `conflict_exists` → resolution workflow with competing equations
  - `correlation_too_weak` → explicit "no detectable relationship"
- `create_review_queue_entry(equation, reason)` → persist to DB for human review
- `generate_data_collection_plan(extraction, shortfall)` → explicit list of missing observations

### Phase 4: Pipeline Rewrite

**`pipeline.py`** — Full rewrite:
- Wire all real components together
- `solve(query, observations, db_path, llm_client)` → full 7-layer flow:
  1. Domain Router → domain + ontology + partition
  2. L1 Parser → extract variables
  3. L2 Correlation → validate relationships + drift
  4. L3 Library lookup → if found + not drifted: use it. If not: fit canonical families + run PySR in parallel. Shadow eval. Promote if improved.
  5. L4 Compute → evaluate + derivatives + sensitivity
  6. L5 Probabilistic → Laplace + BMA → full uncertainty distribution
  7. L6 Gate → hard thresholds
  8. L7 Formatter → prose output
  9. Fallback Router (if gate fails)
- Every step logs to Epistemic Ledger
- `solve_from_csv(query, csv_path, db_path)` → convenience: ingest + solve in one call

**`domain_router.py`** — Rewrite:
- `route_domain(query)` → fast classifier (keyword + regex + light LLM)
- `load_ontology(domain)` → returns variable ontology for domain
- `load_library_partition(domain, db_path)` → returns equations scoped to domain
- Ontology stored as JSON in `src/ontologies/` directory (finance.saas, finance.ecommerce, general)
- Cross-domain inheritance chain

### Phase 5: Ingestion Enhancement

**`ingestion.py`** — Bug fixes + features:
- Fix `build_observations`: don't add observations with `None` values
- `_parse_number`: handle `"$1,234.56"`, `"12.5%"`, `"2.3M"`, `"1.2K"`
- `_parse_date`: handle multiple date formats → ISO 8601
- `detect_delimiter(raw_bytes)` → auto-detect CSV delimiter
- `ingest_excel(file_bytes)` → Excel/Parquet support
- `deduplicate_observations(db_path)` → (variable, entity, timestamp) uniqueness

**`ingestion_api.py`** — Enhance:
- Add `upload_excel()`, `upload_parquet()`
- Add `get_ingestion_status(job_id)`

### Phase 6: Database Connectors

**`connectors/`** (NEW directory):
- `connectors/__init__.py`
- `connectors/base.py` — `BaseConnector` abstract class:
  - `connect(credentials)` → connection
  - `introspect()` → list tables/columns
  - `fetch(table, columns, since)` → incremental pull
  - `disconnect()`
- `connectors/postgres.py` — `PostgresConnector`: psycopg2, read-only, schema introspection
- `connectors/mysql.py` — `MySQLConnector`: mysql-connector-python
- `connectors/schema_interpreter.py` — `SchemaInterpreter`:
  - `map_columns(columns, ontology)` → LLM-powered mapping with confidence scoring
  - `suggest_mappings(table_schema)` → returns `List[SchemaMapping]`
  - Ontology-driven: matches column names to known variables
- `connectors/sync_engine.py` — `SyncEngine`:
  - `full_sync(connector, mappings)` → initial load
  - `incremental_sync(connector, mappings, since)` → delta via `updated_at`
  - `watch(connector, mappings, interval)` → periodic sync
  - `trigger_refit_on_new_data(db_path, variable_set)` → auto-queue equation refit

### Phase 7: SaaS Connectors

**`connectors/saas/`** (NEW):
- `connectors/saas/stripe.py` — charges, subscriptions, invoices → MRR, Churn, Revenue
- `connectors/saas/salesforce.py` — opportunities, accounts → Pipeline, ACV, WinRate
- `connectors/saas/hubspot.py` — contacts, deals → Leads, ConversionRate, CAC
- Each has pre-built variable mappings to standard ontology
- OAuth flow + token refresh
- Cursor-based pagination

### Phase 8: Equation Graph + Conflict Detection

**`equation_graph.py`** (NEW):
- `EquationGraph` class on NetworkX DiGraph
- `add_equation(equation)` → nodes for variables, edges for relationships
- `find_path(source_var, target_var)` → shortest derivation path (Dijkstra)
- `propagate_uncertainty(path, point)` → multiplicative uncertainty along chain
- `multi_hop_solve(query, graph, observations)` → chain equations when single-hop missing
- `import_library_to_graph(db_path)` → load all active equations into graph

**`conflict_detector.py`** (NEW):
- `check_conflict(new_equation, existing_equations)` → compare directional claims on shared variable pairs
- `compare_elasticity_signs(eq1, eq2, var_pair)` → contradictory if signs differ
- `resolution_workflow(conflict)` → human review queue, higher evidence bar for override
- `flag_conflicting(db_path, equation_id, conflict_detail)` → persist to DB

### Phase 9: Temporal Alignment

**`temporal_alignment.py`** (NEW):
- `AlignEngine` class:
  - `detect_frequency(timestamps)` → daily, weekly, monthly, quarterly, annual
  - `align_to_common(observations, target_freq)` → upsample/downsample to common granularity
  - `tag_native_frequency(observation)` → per-observation frequency metadata
  - `block_incompatible_fits(var1_freq, var2_freq)` → reject mixing daily with annual without aggregation
- `resample(observations, freq, method)` → mean/sum/last for downsampling

### Phase 10: Valkey Cache + Queue

**`cache.py`** (NEW):
- `ValkeyCache` class (redis-py compatible):
  - `get_equation(var_hash, domain)` → cache hit returns equation
  - `set_equation(var_hash, domain, equation, ttl=3600)`
  - `invalidate(var_hash, domain)` → on new data or refit
- `ValkeyQueue`:
  - `enqueue_refit(equation_id)` → queue for async refit
  - `enqueue_ingestion(file_id)`
  - `dequeue(worker_id)` → worker picks up job
- Graceful degradation if Valkey unavailable (direct DB fallback)

### Phase 11: FastAPI

**`api.py`** (NEW):
- `POST /v1/solve` — accepts query + observations, returns `SolveResult` + formatted prose
- `POST /v1/ingest` — file upload (CSV/Excel/Parquet), returns `IngestionSummary`
- `GET /v1/equations/{id}` — equation detail with version history
- `GET /v1/equations/{id}/versions` — version list with shadow eval results
- `GET /v1/health` — component health check
- `GET /v1/ledger/{trace_id}` — audit trail for a query
- Request validation via Pydantic models
- Rate limiting (slowapi)
- Idempotency keys on POST endpoints

**`worker.py`** (NEW):
- Background worker that processes Valkey queues
- Refit worker: picks up equation refit jobs, runs shadow eval, promotes if improved
- Ingestion worker: picks up file ingestion jobs, processes asynchronously

### Phase 12: Rust Implementation

**`src/rust/`** — Full implementation (scaffold exists):

**`ingestion_grpc/`** (rewrite existing):
- `proto/ingestion.proto` — gRPC service definition for ingestion
- `src/parser.rs` — CSV/Parquet parsing with proper error handling
- `src/schema.rs` — schema inference + type detection
- `src/dedup.rs` — (variable, entity, timestamp) deduplication
- `src/quality.rs` — range checks, variance checks, outlier detection
- `src/temporal.rs` — temporal alignment + aggregation
- `src/server.rs` — gRPC server implementation
- `src/lib.rs` — PyO3 bindings to expose to Python

**`fit_kernel/`** (NEW):
- Hot-path fitting operations in Rust
- Residual computation, MAPE, AIC, bootstrap sampling
- Exposed via PyO3 as `equation_layer_rs.fit_kernel`

**`ledger/`** (NEW):
- High-throughput append-only ledger writer
- Batched writes with WAL optimization
- Exposed via PyO3 as `equation_layer_rs.ledger`

### Phase 13: Docker + K8s

**Docker**:
- `Dockerfile.api` — FastAPI production image (python:3.11-slim, gunicorn + uvicorn)
- `Dockerfile.worker` — background worker image
- `Dockerfile.rust` — Rust builder image
- `docker-compose.yml` — API + worker + Valkey + SQLite volume

**Kubernetes**:
- `k8s/api-deployment.yaml` — API deployment + HPA
- `k8s/api-service.yaml` — ClusterIP service
- `k8s/worker-deployment.yaml` — Worker deployment
- `k8s/valkey-statefulset.yaml` — Valkey with persistent volume
- `k8s/ingress.yaml` — Ingress with TLS
- `k8s/configmap.yaml` — App config
- `k8s/secrets.yaml` — Secrets template

### Phase 14: Tests

**Keep existing** but add comprehensive tests:
- `tests/test_fitting.py` — fit accuracy against known functions (y=2x+1, y=3x², etc.), holdout split logic, structural validity checks
- `tests/test_compute.py` — SymPy evaluation correctness, derivative accuracy vs numerical
- `tests/test_probabilistic.py` — Laplace CI coverage (should contain true value ~95% of time), BMA weighting
- `tests/test_correlation.py` — correlation matrix, partial correlation, drift detection thresholds
- `tests/test_parser.py` — extraction schema enforcement, domain routing accuracy
- `tests/test_fallback.py` — every gate failure reason produces actionable output
- `tests/test_pipeline.py` — end-to-end flow with known data, library hit, library miss, gate failure paths
- `tests/test_connectors.py` — schema interpreter accuracy, sync engine logic
- `tests/test_equation_graph.py` — path finding, uncertainty propagation, conflict detection
- `tests/test_temporal.py` — frequency detection, alignment, incompatible frequency blocking
- `tests/test_api.py` — FastAPI endpoint tests with TestClient

## Files to Create (Complete List)

```
src/
  epistemic_parser.py        NEW — L1
  correlation.py              NEW — L2
  fitting.py                  REWRITE — L3
  compute.py                  REWRITE — L4
  probabilistic.py            REWRITE — L5
  pysr_discovery.py           REWRITE — L3 parallel
  formatter.py                NEW — L7
  fallback_router.py          NEW — Post-gate
  domain_router.py            REWRITE
  pipeline.py                 REWRITE
  ingestion.py                ENHANCE
  ingestion_api.py            ENHANCE
  temporal_alignment.py       NEW
  equation_graph.py           NEW
  conflict_detector.py        NEW
  cache.py                    NEW
  api.py                      NEW
  worker.py                   NEW
  ontologies/
    __init__.py
    finance_saas.json
    finance_ecommerce.json
    general.json
  connectors/
    __init__.py               NEW
    base.py                   NEW
    postgres.py               NEW
    mysql.py                   NEW
    schema_interpreter.py      NEW
    sync_engine.py             NEW
    saas/
      __init__.py
      stripe.py               NEW
      salesforce.py            NEW
      hubspot.py               NEW
  rust/
    ingestion_grpc/
      proto/ingestion.proto   REWRITE
      src/parser.rs           NEW
      src/schema.rs           NEW
      src/dedup.rs            NEW
      src/quality.rs          NEW
      src/temporal.rs         NEW
      src/server.rs           NEW
      src/lib.rs              NEW
    fit_kernel/
      Cargo.toml              NEW
      src/lib.rs              NEW
    ledger/
      Cargo.toml              NEW
      src/lib.rs              NEW
Dockerfile.api                NEW
Dockerfile.worker             NEW
docker-compose.yml            NEW
k8s/
  api-deployment.yaml
  api-service.yaml
  worker-deployment.yaml
  valkey-statefulset.yaml
  ingress.yaml
  configmap.yaml
  secrets.yaml
tests/
  test_fitting.py             NEW
  test_compute.py             NEW
  test_probabilistic.py       NEW
  test_correlation.py         NEW
  test_parser.py              NEW
  test_fallback.py            NEW
  test_pipeline.py            REWRITE
  test_connectors.py          NEW
  test_equation_graph.py      NEW
  test_temporal.py            NEW
  test_api.py                 NEW
  test_gate.py                KEEP
  test_ingestion.py           ENHANCE
```

## Verification

1. `cd src && python -c "from pipeline import solve; print(solve('test', [1,2,3], [2,4,6], ':memory:'))"` — end-to-end flow works
2. `pytest tests/ -v` — all tests pass
3. `uvicorn src.api:app` — API starts, `/v1/health` returns 200
4. `docker-compose up` — all services start
5. `cargo build --manifest-path src/rust/ingestion_grpc/Cargo.toml` — Rust compiles
6. `python -c "import equation_layer_rs"` — PyO3 bindings load (if compiled)
