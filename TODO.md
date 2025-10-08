# `TODO.md`: Fitbit Integration Revamp (Production Blueprint)

This document outlines the phased implementation plan for refactoring the Fitbit integration into our modern, clean architecture.  
Follow these steps sequentially to ensure a **secure, reliable, and correct** implementation.

---

## Guiding Principles

- **Domain Stability:** `lifestats-core` remains unchanged. The new repository port will be defined at the application layer.
- **Ports & Adapters:** The `application` layer defines traits (Ports); `infrastructure` crates provide concrete implementations (Adapters).
- **Security First:** All sensitive data (tokens, secrets) must be handled using the `secrecy` crate and wiped via `zeroize`.
- **Idempotency & Reliability:** The sync process must be idempotent and resilient to transient network failures.
- **Single Source of Truth:** Token refresh, retry, and mapping logic live exclusively in the `SyncFitbitDataCommandHandler`.

---

## Phase 1: Foundational Types & Application Contracts (COMPLETED)

### `lifestats-application`

#### 1. Secure Token Types

Create `application/src/integrations/tokens.rs`.

- Use `secrecy` + `zeroize` to wrap tokens safely.
- **DO NOT** implement `Debug`, `Display`, or `Serialize`.  
  Provide a manual redacting `Debug` if absolutely necessary.
- Secrets may only be exposed via `as_str()` and **only inside driven adapters**.

```rust
use secrecy::{Secret, SecretString, ExposeSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct AccessToken(Secret<SecretString>);
impl AccessToken {
    pub fn new(s: String) -> Self { Self(Secret::new(SecretString::new(s))) }
    pub fn as_str(&self) -> &str { self.0.expose_secret().as_ref() }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct RefreshToken(Secret<SecretString>);
impl RefreshToken {
    pub fn new(s: String) -> Self { Self(Secret::new(SecretString::new(s))) }
    pub fn as_str(&self) -> &str { self.0.expose_secret().as_ref() }
}

pub struct FitbitTokenData {
    pub access_token: AccessToken,
    pub refresh_token: RefreshToken,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}
```

#### 2. Canonical TagSet Value Object

Create `application/src/shared/tags.rs`.

```rust
pub struct TagSet(BTreeMap<String, String>);

impl TagSet {
    pub fn new<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(pairs: I) -> Self {
        let mut map = BTreeMap::new();
        for (k, v) in pairs {
            let k = k.into().trim().to_ascii_lowercase();
            let v = v.into().trim().to_string();
            map.insert(k, v);
        }
        Self(map)
    }

    pub fn canonical_fingerprint(&self) -> String {
        self.0.iter()
            .map(|(k,v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(";")
    }
}
```

All keys must be lowercase, values trimmed, and pairs sorted by key.

#### 3. Application-Level Repository Port

Extend `application/src/statistic/ports.rs`:

```rust
#[async_trait]
pub trait TagQueryableStatisticRepository {
    async fn find_statistic_id_by_exact_tags(
        &self,
        tags: &TagSet
    ) -> ApplicationResult<Option<StatisticId>>;
}
```

> **Note:** This lives in the application layer (not domain) to preserve domain purity.

#### 4. Fitbit Ports (API Contracts)

Define in `application/src/integrations/fitbit/ports.rs`.

```rust
#[async_trait]
pub trait FitbitOAuthApi {
    async fn exchange_code_for_token(
        &self,
        code: &str,
        pkce_verifier: Option<String>
    ) -> ApplicationResult<FitbitTokenData>;

    async fn refresh(
        &self,
        refresh_token: &RefreshToken
    ) -> ApplicationResult<FitbitTokenData>;
}

#[async_trait]
pub trait FitbitDataApi {
    async fn get_activity_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate
    ) -> ApplicationResult<FitbitActivityData>;

    async fn get_sleep_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate
    ) -> ApplicationResult<FitbitSleepData>;
}

#[async_trait]
pub trait TokenStorePort {
    async fn load(&self, source: &str) -> ApplicationResult<Option<FitbitTokenData>>;
    async fn save(&self, source: &str, token: &FitbitTokenData) -> ApplicationResult<()>;
    async fn set_last_sync_at(&self, source: &str, t: DateTime<Utc>) -> ApplicationResult<()>;
    async fn last_sync_at(&self, source: &str) -> ApplicationResult<Option<DateTime<Utc>>>;
}

pub trait Clock { fn now_utc(&self) -> DateTime<Utc>; }
```

#### 5. Standardized Upstream Error Type

`application/src/integrations/fitbit/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FitbitUpstreamError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited")]
    RateLimited { retry_after: Option<Duration> },
    #[error("invalid grant")]
    InvalidGrant,
    #[error("upstream service error: {0}")]
    UpstreamServiceError(anyhow::Error),
}

impl From<FitbitUpstreamError> for ApplicationError {
    fn from(e: FitbitUpstreamError) -> Self {
        ApplicationError::FitbitSyncFailed(e)
    }
}
```

---

## Phase 2: Application Logic & Reliability Policies

### `lifestats-application`

#### 1. Define `SyncFitbitDataCommand`

`application/src/use_cases/commands/sync_fitbit_data.rs`

```rust
pub struct SyncFitbitDataCommand {
    pub since: NaiveDate,
    pub until: NaiveDate,
}
```

Validate `since <= until` and `(until - since) <= MAX_SYNC_RANGE` (e.g., 31 days).

#### 2. Implement `SyncFitbitDataCommandHandler`

**Configuration**

- Inject all required ports (`FitbitOAuthApi`, `FitbitDataApi`, `TokenStorePort`, `TagQueryableStatisticRepository`, `StatisticRepository`, `Clock`).
- Include:
  - `timezone: Tz`
  - `token_expiry_skew: Duration` (default 5m)
  - `max_concurrency: usize` (default 3)
  - `max_retries: usize` (default 3)
  - `base_backoff_ms: u64` (default 500)
  - `backoff_multiplier: f64` (default 2.0)
  - `http_timeout: Duration` (default 15s)
  - `enabled_metrics: HashSet<FitbitMetric>`

**Observability**

```rust
#[tracing::instrument(skip_all, fields(run_id = %sync_run_id))]
```

Generate a `sync_run_id = Uuid::new_v4()` and propagate through all spans.

**Logic**

1. **Validate Date Range** — enforce max range.
2. **Load & Verify Token**
   - Load via `TokenStorePort`.
   - Verify token scopes ⊇ required scopes for `enabled_metrics`.
     If missing, return actionable error listing scopes.

3. **Proactive Token Refresh**
   - If `clock.now_utc() >= token.expires_at - token_expiry_skew`, refresh via `FitbitOAuthApi::refresh()` and save immediately.

4. **Metric-to-ID Mapping**
   - For each enabled metric, build tags `{source="fitbit", metric=<metric>}` and call `find_statistic_id_by_exact_tags`.
   - Fail fast if any metric is unmapped or has duplicates.

5. **Per-Run Cache**
   - Use `HashMap<(FitbitMetric, NaiveDate), FitbitData>` to deduplicate calls within a single run.

6. **Concurrent Date Processing**
   - Use `futures::stream::iter(...).for_each_concurrent(max_concurrency, ...)`.

7. **Per-Date Logic**
   - **Retry Loop:**
     - On `Unauthorized`: refresh once, retry once.
     - On `RateLimited`: respect `retry_after` or use exponential backoff with full jitter.
     - Cap at `max_retries`.

   - **Data Transformation:** Attribute cross-midnight sleep correctly (use tested `canonical_sleep_date` helper).
   - **UPSERT Logging:** Call repository `save_log_value` using database UPSERT (replace existing value).

8. **Post-Run:** Update `last_sync_at`.

---

## Phase 3: Infrastructure Adapters (Driven)

### `core/infrastructure/fitbit_client`

- Implement `HttpFitbitOAuthClient` and `HttpFitbitDataClient`.
- Use `reqwest` with appropriate headers.
- Map HTTP errors to `FitbitUpstreamError`:
  - 401 → `Unauthorized`
  - 429 → `RateLimited { retry_after }`
  - 400 with `"invalid_grant"` → `InvalidGrant`

- Call `.as_str()` on tokens **only** immediately before building the `Authorization` header.

### `lifestats-persistence`

1. **Migrations**
   - Add columns to `integrations`:
     - `access_token TEXT`
     - `refresh_token TEXT`
     - `expires_at_epoch_seconds INTEGER`
     - `scopes TEXT` (sorted, space-separated)
     - `last_sync_at TEXT`
     - `canonical_tags TEXT`

   - Constraints:
     - `UNIQUE(source_name)`
     - `UNIQUE(statistic_id, date)` on `logging_data`
     - `UNIQUE(canonical_tags)` on `statistics`

   - Index: `CREATE INDEX idx_integrations_source_name ON integrations(source_name);`

2. **Implement `SqliteTokenStore`**
   - Read/write `FitbitTokenData`.
   - Always sort scopes when reading.

3. **Implement `TagQueryableStatisticRepository`**
   - Extend `SqliteStatisticRepository` to support tag lookups by `canonical_fingerprint`.

4. **UPSERT Guarantee**

   ```sql
   INSERT INTO logging_data (statistic_id, date, value)
   VALUES (?1, ?2, ?3)
   ON CONFLICT(statistic_id, date)
   DO UPDATE SET value = excluded.value;
   ```

### `core/infrastructure/system_clock`

Implement `SystemClock`:

```rust
pub struct SystemClock;
impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> { chrono::Utc::now() }
}
```

---

## Phase 4: Driving Adapter (Web API)

### `lifestats-web-api`

1. **OAuth Flow**
   - `GET /integrations/fitbit/auth/begin`
     - Generate `state` + `pkce_verifier`.
     - Store `{state, pkce_verifier, user_id, expires_at}` in a `moka` cache with 5-min TTL.
     - **Consume-on-read**: remove the entry once validated.

   - `GET /integrations/fitbit/auth/callback`
     - Validate `state` in cache (must belong to current user).
     - Atomically consume entry (get-and-delete). If missing → `400 Bad Request`.
     - Exchange code for token via `FitbitOAuthApi`.
     - Save token via `TokenStorePort`.

2. **Sync Endpoint**
   - `POST /integrations/fitbit/sync`
     - Validate `since/until`.
     - Execute `SyncFitbitDataCommand`.
     - Return `204 No Content` on success.

3. **Error Mapping**
   - `InvalidGrant` → `401`
   - `RateLimited` → `429` + `Retry-After` header
   - `PreconditionFailed` → `412`

---

## Phase 5: Composition & Provisioning

### `lifestats-bootstrap`

1. **FitbitBundleFactory**
   - Create statistics tagged:
     - `{source="fitbit", metric="steps"}`
     - `{source="fitbit", metric="sleep_duration"}`
     - `{source="fitbit", metric="fell_asleep_time"}`
     - `{source="fitbit", metric="wake_up_time"}`

   - Fail provisioning if a statistic with identical canonical tags already exists.

2. **Build Services**
   - Read:
     - `FITBIT_CLIENT_ID`
     - `FITBIT_CLIENT_SECRET`
     - `FITBIT_REDIRECT_URI`
     - `TIMEZONE`
     - `TOKEN_EXPIRY_SKEW`
     - `MAX_CONCURRENCY`

   - Instantiate:
     - `HttpFitbitOAuthClient`
     - `HttpFitbitDataClient`
     - `SqliteTokenStore`
     - `SystemClock`

   - Register `FitbitBundleFactory`.

---

## Phase 6: Testing & Validation

- **Unit Tests (`fitbit_client`):**
  - Verify HTTP → `FitbitUpstreamError` mapping.

- **Integration Tests (`lifestats-application`):**
  - `401 Unauthorized` → refresh → retry once.
  - `429 RateLimited` → respect `Retry-After`.
  - Missing scopes → actionable error.
  - Sync same day twice → identical DB state (idempotency).
  - Token expires mid-range → only one refresh total.
  - **Property Tests:** sleep attribution around DST transitions.
  - **Property Tests:** `TagSet` canonicalization merges `"A"="x"` and `"a"="x "` identically.

- **E2E Tests (`lifestats-web-api`):**
  - Full OAuth2 flow via mock Fitbit endpoints.
  - `/sync` endpoint executes command end-to-end and updates DB.

---

## Phase 7: Deprecation of Legacy Logger

### `logger_applications/fitbit_logger`

1. Replace `main.rs` with a thin client:

   ```rust
   reqwest::Client::new()
       .post("http://localhost:PORT/integrations/fitbit/sync")
       .json(&SyncFitbitDataCommand { since, until })
       .send()
       .await?;
   ```

2. Delete all legacy OAuth, caching, and persistence logic.
3. Add a comment noting deprecation; remove the crate entirely after one stable release once telemetry confirms full adoption.

---

## Optional Enhancements

- **Feature Flag:** `INTEGRATION_FITBIT=on/off` for safe rollout.
- **Operational Metrics:** Counters (`sync.success`, `sync.failure`), histograms (`fitbit.latency`), gauge (`token.expires_in_seconds`).
- **Secure Storage:** Consider OS keyring integration for refresh tokens behind `TokenStorePort`.
