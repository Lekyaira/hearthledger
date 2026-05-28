# Hearthledger — Family Inventory & Fulfillment System

*Spec & Design Document — v2.0*

---

## 1. Overview

Hearthledger is a self-hosted web application for shared inventory management across multiple households. It solves a specific problem: a central inventory of goods (maintained at one location) that family members across different households can request items from. Requests are bundled into pickup-ready packages, fulfilled, and inventory is updated automatically.

Think of it as a lightweight internal e-commerce system — browse a catalog, place a request, someone packs it, you pick it up.

### 1.1 Core Workflow

1. An inventory manager maintains **lists** of items (e.g., "Pantry Bulk," "Garden Supplies," "Canning Stock") with target and on-hand quantities.
2. A family member browses a list and creates a **request** — "I need 6 jars of salsa and 2 lbs of dried beans."
3. A fulfiller reviews open requests and creates a **bundle** — grouping one or more requests into a pickup-ready package. This could be all of one person's requests, or a mix across people for a particular day.
4. The fulfiller packs items and marks individual line items as packed. When everything in the bundle is packed, they mark the bundle as **ready**.
5. The requestor picks up their items. The bundle is marked **done**, and on-hand quantities are decremented automatically.
6. Separately, **restocks** add inventory — someone brings in a Costco haul and logs what they added.

### 1.2 Design Goals

- **Self-contained**: single Docker image, SQLite database, no external services. Deployed on a Synology NAS.
- **Multi-household, unified**: no tenant isolation — everyone sees everything. Trust model is "extended family."
- **Simple auth model**: HTTP basic auth at the perimeter (single shared password), user selection inside the app.
- **Type-safe end-to-end**: OpenAPI spec auto-generated from Rust types → TypeScript types for the frontend.
- **Mobile-friendly**: responsive Tailwind UI, touch-optimized for phone use while standing in a pantry.
- **Backupable**: a single API endpoint serves a consistent database snapshot for cron-based backups.

---

## 2. Tech Stack

### 2.1 Backend

| Component | Technology | Purpose |
|-----------|-----------|---------|
| HTTP framework | **Axum 0.8+** | API routes, middleware, static file serving |
| Database | **SQLite** (single file) | All persistent storage |
| Query layer | **sqlx** (compile-time checked) | Type-safe SQL, migrations |
| OpenAPI | **utoipa** + **utoipa-axum** | Auto-generate OpenAPI spec from handler types |
| Auth | **tower-http** `ValidateRequestHeaderLayer` | HTTP basic auth from env var |
| Static files | **tower-http** `ServeDir` | Serve SvelteKit build output |
| Compression | **tower-http** `CompressionLayer` | gzip/brotli for static assets |
| Serialization | **serde** + **serde_json** | JSON request/response bodies |
| IDs | **nanoid** | URL-friendly, non-sequential identifiers |

### 2.2 Frontend

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Framework | **SvelteKit** (TypeScript, strict mode) | File-based routing, component model |
| Adapter | **@sveltejs/adapter-static** | Compiles to static SPA (no Node runtime in prod) |
| Styling | **Tailwind CSS 4** | Utility-first, responsive design |
| API client | **openapi-fetch** | Typed HTTP client generated from OpenAPI spec |
| Type generation | **openapi-typescript** | TS types from OpenAPI spec |
| Icons | **lucide-svelte** | Consistent icon set |

### 2.3 Build & Deploy

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Container | **Docker** (multi-stage) | Single image: build frontend + backend, slim runtime |
| Base image | **debian:bookworm-slim** | Minimal runtime with glibc for SQLite |
| Runtime | **Synology Container Manager** | Docker Compose via NAS GUI |

### 2.4 Data Flow (Type Safety Pipeline)

```
Rust structs (serde + utoipa derives)
    ↓  cargo build generates openapi.json
openapi.json
    ↓  openapi-typescript generates types
frontend/src/lib/api/schema.d.ts
    ↓  openapi-fetch consumes types
Typed API calls in Svelte components
```

This pipeline runs at build time. A build script exports the OpenAPI spec from the Rust binary, then the frontend build consumes it.

---

## 3. Data Model

### 3.1 Entity Relationship Diagram

```
users
  id            TEXT PK (nanoid)
  name          TEXT NOT NULL UNIQUE
  color         TEXT NOT NULL           -- hex color for avatar/attribution
  created_at    TEXT NOT NULL            -- ISO 8601

lists
  id            TEXT PK (nanoid)
  name          TEXT NOT NULL
  description   TEXT
  created_at    TEXT NOT NULL
  updated_at    TEXT NOT NULL

items
  id            TEXT PK (nanoid)
  list_id       TEXT NOT NULL FK → lists.id (CASCADE DELETE)
  name          TEXT NOT NULL
  description   TEXT
  unit          TEXT                     -- "lbs", "cans", "ea", etc.
  target_qty    REAL NOT NULL DEFAULT 0  -- desired stock level
  on_hand_qty   REAL NOT NULL DEFAULT 0  -- current stock level
  created_at    TEXT NOT NULL
  updated_at    TEXT NOT NULL
  UNIQUE(list_id, name)

requests
  id            TEXT PK (nanoid)
  list_id       TEXT NOT NULL FK → lists.id (CASCADE DELETE)
  requested_by  TEXT NOT NULL FK → users.id
  bundle_id     TEXT FK → bundles.id (SET NULL)  -- null = unbundled
  status        TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'bundled', 'done', 'cancelled'))
  note          TEXT
  created_at    TEXT NOT NULL
  completed_at  TEXT

request_items
  id            TEXT PK (nanoid)
  request_id    TEXT NOT NULL FK → requests.id (CASCADE DELETE)
  item_id       TEXT NOT NULL FK → items.id (CASCADE DELETE)
  quantity      REAL NOT NULL            -- how many they want

bundles
  id            TEXT PK (nanoid)
  name          TEXT                     -- optional label ("Mom's Tuesday pickup")
  created_by    TEXT NOT NULL FK → users.id  -- the fulfiller
  status        TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'ready', 'done'))
  note          TEXT
  created_at    TEXT NOT NULL
  completed_at  TEXT

bundle_items
  id            TEXT PK (nanoid)
  bundle_id     TEXT NOT NULL FK → bundles.id (CASCADE DELETE)
  request_item_id TEXT NOT NULL FK → request_items.id (CASCADE DELETE)
  packed        INTEGER NOT NULL DEFAULT 0  -- boolean: 0/1

restocks
  id            TEXT PK (nanoid)
  list_id       TEXT NOT NULL FK → lists.id (CASCADE DELETE)
  created_by    TEXT NOT NULL FK → users.id
  note          TEXT
  created_at    TEXT NOT NULL

restock_items
  id            TEXT PK (nanoid)
  restock_id    TEXT NOT NULL FK → restocks.id (CASCADE DELETE)
  item_id       TEXT NOT NULL FK → items.id (CASCADE DELETE)
  quantity      REAL NOT NULL            -- how many were added
```

### 3.2 Entity Relationships (Visual)

```
  users ──────────────┬───────────────┬──────────────────┐
                      │               │                  │
                 requested_by     created_by         created_by
                      │               │                  │
  lists ──┬──── items │               │                  │
          │      │    │               │                  │
          │      │    ▼               │                  │
          │      │  requests          │                  ▼
          │      │    │               │              restocks
          │      │    │ request_items  │                  │
          │      │    │    │          │            restock_items
          │      │    │    │          ▼
          │      │    │    │       bundles
          │      │    │    │          │
          │      │    │    └───► bundle_items
          │      │    │         (joins request_items
          │      │    │          into bundles)
```

### 3.3 Key Design Decisions

**Requests vs. Restocks as separate entities**: The v1 spec used a single "orders" table with an `order_type` discriminator. This version separates them because they have fundamentally different lifecycles — requests go through a multi-step fulfillment flow (open → bundled → done) while restocks are immediate (create = inventory updated). Different entities, different UI flows, different API surfaces.

**Bundles as the fulfillment unit**: A bundle is the atomic unit of fulfillment. It groups request line items (not whole requests) so a fulfiller can partially fulfill a request across multiple bundles if needed. The `bundle_items` join table maps individual `request_items` into a bundle with a `packed` flag for checklist-style packing.

**Request status flow**:
- `open` — just submitted, not yet assigned to a bundle.
- `bundled` — all of its request_items have been added to a bundle (or bundles). Automatically computed: a request moves to `bundled` when every one of its request_items appears in a bundle_item.
- `done` — the bundle(s) containing its items are complete. Inventory has been decremented.
- `cancelled` — requestor withdrew the request before fulfillment.

**Bundle status flow**:
- `open` — being assembled. Fulfiller is adding request_items to it.
- `ready` — all bundle_items are packed. Awaiting pickup.
- `done` — picked up. Inventory decremented.

**Why `bundle_items` references `request_items` not `items` directly**: This maintains full traceability. You can always trace a packed item back to who requested it, how much they asked for, and from which list. It also means the same inventory item can appear in multiple bundles (from different requests) without ambiguity.

**Restocks are immediate**: No lifecycle beyond creation. Creating a restock immediately increments `on_hand_qty` for all its items. Deleting a restock reverses the increment. This deliberate asymmetry keeps the "I just bought stuff" action fast and frictionless.

**Text IDs (nanoid)**: 12-character nanoids — URL-safe, non-sequential, ~4.6 × 10^21 combinations.

**REAL for quantities**: Supports fractional amounts (2.5 lbs, 0.5 gallons). The UI displays whole numbers cleanly when the value has no fractional part.

**`unit` as freeform text**: No enum. Families have idiosyncratic units. The UI suggests common units via autocomplete from existing items in the same list.

**CASCADE DELETE on list_id**: Deleting a list removes its items, requests, and restocks. Expected behavior — no orphaned data.

### 3.4 Inventory Mutation Rules

**On restock creation**: For each restock_item, `item.on_hand_qty += restock_item.quantity`. Applied immediately in a transaction.

**On restock deletion**: Reverse — `item.on_hand_qty -= restock_item.quantity`, clamped to ≥ 0.

**On bundle completion** (status → `done`): For each bundle_item where `packed = 1`, `item.on_hand_qty -= request_item.quantity`. The item is looked up via `bundle_item.request_item_id → request_item.item_id`. Applied in a transaction. All requests fully covered by the bundle move to `done`.

**On bundle reopen** (status `done` → `open`): Reverse the decrements for all packed bundle_items. Associated requests revert to `bundled`.

**Quick adjustments**: The `PATCH /items/{id}/quantity` endpoint allows direct `+N / -N` tweaks without creating a request or restock. For small corrections ("oops, we actually have 4, not 5").

**Clamping**: `on_hand_qty` is always clamped to ≥ 0 after any mutation.

### 3.5 SQLite Configuration

```sql
PRAGMA journal_mode = WAL;           -- concurrent reads during writes
PRAGMA foreign_keys = ON;            -- enforce FK constraints
PRAGMA busy_timeout = 5000;          -- wait up to 5s on lock contention
PRAGMA synchronous = NORMAL;         -- safe with WAL, better perf than FULL
```

Set via sqlx `connect_options` at startup.

---

## 4. API Design

Base path: `/api/v1`

All endpoints return JSON. Request bodies are JSON. Errors use a consistent envelope:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Item not found"
  }
}
```

### 4.1 Users

| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List all users |
| POST | `/users` | Create user `{name, color}` |
| PUT | `/users/{id}` | Update user `{name?, color?}` |
| DELETE | `/users/{id}` | Delete user (fails if user has open requests/bundles) |

### 4.2 Lists

| Method | Path | Description |
|--------|------|-------------|
| GET | `/lists` | All lists (with item count, low-stock count, open request count) |
| POST | `/lists` | Create list `{name, description?}` |
| GET | `/lists/{id}` | Get list with all items |
| PUT | `/lists/{id}` | Update list `{name?, description?}` |
| DELETE | `/lists/{id}` | Delete list (cascades items, requests, restocks) |

### 4.3 Items

| Method | Path | Description |
|--------|------|-------------|
| GET | `/lists/{list_id}/items` | List items (sortable, filterable by low-stock) |
| POST | `/lists/{list_id}/items` | Create item `{name, unit?, target_qty, on_hand_qty?}` |
| PUT | `/items/{id}` | Update item fields |
| DELETE | `/items/{id}` | Delete item |
| PATCH | `/items/{id}/quantity` | Quick adjust `{delta}` — adds/subtracts from on_hand |

### 4.4 Requests

| Method | Path | Description |
|--------|------|-------------|
| GET | `/requests` | List requests (filter: `?status=open&list_id=X&requested_by=Y`) |
| POST | `/requests` | Create request `{list_id, note?, items: [{item_id, quantity}]}` |
| GET | `/requests/{id}` | Get request with line items (joined with item names/units) |
| PUT | `/requests/{id}` | Update note |
| POST | `/requests/{id}/cancel` | Cancel request (only if `open`) |
| DELETE | `/requests/{id}` | Delete request (only if `open` or `cancelled`) |

The `requested_by` field is extracted from the `X-User-Id` header.

### 4.5 Bundles

| Method | Path | Description |
|--------|------|-------------|
| GET | `/bundles` | List bundles (filter: `?status=open`) |
| POST | `/bundles` | Create bundle `{name?, note?}` |
| GET | `/bundles/{id}` | Get bundle with all bundle_items (joined to request/item data) |
| PUT | `/bundles/{id}` | Update name/note |
| DELETE | `/bundles/{id}` | Delete bundle (only if `open` — unlinks request_items) |

The `created_by` field is extracted from the `X-User-Id` header.

### 4.6 Bundle Items

| Method | Path | Description |
|--------|------|-------------|
| POST | `/bundles/{bundle_id}/items` | Add request_items to bundle `{request_item_ids: [...]}` |
| PATCH | `/bundle-items/{id}/pack` | Toggle packed `{packed: true/false}` |
| DELETE | `/bundle-items/{id}` | Remove item from bundle |

### 4.7 Bundle Lifecycle

| Method | Path | Description |
|--------|------|-------------|
| POST | `/bundles/{id}/ready` | Mark bundle as ready for pickup (all items must be packed) |
| POST | `/bundles/{id}/complete` | Mark bundle done, decrement inventory |
| POST | `/bundles/{id}/reopen` | Reopen bundle, reverse inventory changes |

These are explicit POST actions (not PUT) because they have side effects.

### 4.8 Restocks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/restocks` | List restocks (filter: `?list_id=X`) |
| POST | `/restocks` | Create restock `{list_id, note?, items: [{item_id, quantity}]}` — immediately updates inventory |
| GET | `/restocks/{id}` | Get restock with line items |
| DELETE | `/restocks/{id}` | Delete restock (reverses inventory changes) |

Restocks have no status lifecycle. Creation = inventory updated. Deletion = undo.

### 4.9 Utility & Backup

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check `{status: "ok", version: "..."}` — no auth required |
| GET | `/openapi.json` | OpenAPI 3.1 spec — no auth required |
| GET | `/api/v1/backup` | Download SQLite database snapshot — auth required |

---

## 5. Database Backup

### 5.1 Endpoint

`GET /api/v1/backup`

Returns a consistent snapshot of the SQLite database as a file download. Protected by the same HTTP basic auth as all other API endpoints.

### 5.2 Implementation

The handler uses SQLite's `VACUUM INTO` command to create a point-in-time consistent copy:

```rust
async fn backup(State(pool): State<SqlitePool>) -> Result<impl IntoResponse, AppError> {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let tmp_path = format!("/tmp/hearthledger-backup-{timestamp}.db");

    sqlx::query(&format!("VACUUM INTO '{tmp_path}'"))
        .execute(&pool)
        .await?;

    let body = tokio::fs::read(&tmp_path).await?;
    tokio::fs::remove_file(&tmp_path).await.ok();

    Ok((
        [
            (header::CONTENT_TYPE, "application/x-sqlite3"),
            (header::CONTENT_DISPOSITION,
             format!("attachment; filename=\"hearthledger-{timestamp}.db\"")),
        ],
        body,
    ))
}
```

`VACUUM INTO` creates a complete, self-contained copy with no WAL dependency. Safe to call under load.

### 5.3 Cron Job (External)

Set up on the Synology NAS via Task Scheduler:

```bash
#!/bin/bash
# /usr/local/bin/hearthledger-backup.sh
set -euo pipefail

BACKUP_DIR="/volume1/backups/hearthledger"
RETENTION_DAYS=30
URL="http://localhost:8080/api/v1/backup"
AUTH="family:${HEARTHLEDGER_PASSWORD}"

mkdir -p "$BACKUP_DIR"

curl -sf -u "$AUTH" "$URL" \
    -o "$BACKUP_DIR/hearthledger-$(date +%Y%m%d-%H%M%S).db"

# Prune old backups
find "$BACKUP_DIR" -name "hearthledger-*.db" -mtime +$RETENTION_DAYS -delete
```

Note the `localhost` URL — the cron job runs on the NAS itself, so no TLS needed.

---

## 6. Authentication

### 6.1 Perimeter: HTTP Basic Auth

A `tower-http` middleware layer applies HTTP basic auth to all API routes and static file serving.

```
HEARTHLEDGER_USERNAME=family          # env var, defaults to "family"
HEARTHLEDGER_PASSWORD=<required>      # env var, no default — app refuses to start without it
```

The browser caches basic auth credentials for the session. Users authenticate once per browser session. The health check and OpenAPI spec endpoints are exempt.

### 6.2 In-App: User Selection

Once past basic auth, the app shows a user picker on first visit. The selected user ID is stored in `localStorage` and sent as an `X-User-Id` header on API requests that need attribution (creating requests, bundles, restocks). The backend validates that the user ID exists but enforces no permissions — any user can do anything.

If no `X-User-Id` header is present on endpoints that require it, the API returns `400 Bad Request`.

---

## 7. Screens & UI

The UI is a responsive SPA with a persistent sidebar (desktop) / bottom nav (mobile). The active user's name and color dot appear in the nav. Primary navigation items: **Dashboard**, **Lists**, **Requests**, **Bundles**, **Settings**.

### 7.1 User Picker (First Visit / Switch User)

- Grid of user cards showing name and color dot.
- "Add new member" inline form (name + color picker).
- Selecting a user stores their ID in localStorage and navigates to the dashboard.
- Accessible later via nav avatar to switch users.

### 7.2 Dashboard (`/`)

- **Low Stock** section: items where `on_hand_qty < target_qty`, grouped by list. Quick "Request" button on each.
- **My Open Requests**: the current user's pending/bundled requests.
- **Open Bundles**: bundles in `open` or `ready` status, showing fulfiller name, item count, how many packed.
- **Recent Activity**: last N restocks and completed bundles for a quick audit trail.

### 7.3 List Detail (`/lists/{id}`)

- Header: list name (editable inline), description.
- Item table (responsive — card layout on mobile):
  - Columns: Name, On Hand, Target, Unit, Status (green/amber/red dot).
  - Inline editable quantities (tap to edit, blur to save).
  - Quick "+1 / -1" stepper buttons on On Hand for fast adjustments.
  - "Add Item" row/button at the bottom.
- Actions:
  - "Request Items" — opens request creation pre-filtered to this list, pre-selects items below target.
  - "Restock" — opens restock creation pre-filtered to this list.
  - "Delete List" (with confirmation).
- Filter/sort: by name, by status (low stock first), search.

### 7.4 Request Creation (`/requests/new?list_id=X`)

- Select list (pre-filled if coming from list detail).
- Item picker: checkboxes for items from the selected list, quantity input for each.
  - Default quantity for each item: `max(target_qty - on_hand_qty, 0)` (the gap).
  - Shows current on-hand and target as context.
- Optional note field ("Need this by Friday," etc.).
- Submit creates the request and navigates to request detail.

### 7.5 Request Detail (`/requests/{id}`)

- Header: list name, requested by (name + color), date, status badge.
- Line items: item name, quantity, unit.
- If `open`: "Cancel Request" action.
- If `bundled`: shows which bundle(s) the items are in, linked.
- If `done`: shows completion date.

### 7.6 Requests List (`/requests`)

- Filter toggles: All / Open / Bundled / Done / Cancelled.
- Filter by user ("My Requests" vs. all).
- Request cards: list name, item summary, requested by, date, status badge.
- Tapping navigates to detail.

### 7.7 Bundle Creation & Detail (`/bundles/{id}`)

Bundle creation is integrated into the bundle detail view — create an empty bundle, then add items to it.

- Header: bundle name (editable), created by, date, status badge.
- **Add items panel**: shows open (unbundled) request_items, grouped by request/user. Fulfiller checks items to add to this bundle. Supports "Add all from [user]" and "Add all from [request]" bulk actions.
- **Packed checklist**: the bundle's items as a packing checklist. Each row: item name, quantity, unit, who requested it, packed checkbox.
- Actions:
  - "Mark Ready" — enabled when all items are packed. Signals the bundle is ready for pickup.
  - "Mark Complete" — on ready bundles. Applies inventory decrements.
  - "Reopen" — on completed bundles. Reverses inventory changes.
  - "Delete" — on open bundles. Unlinks items back to their requests.

### 7.8 Bundles List (`/bundles`)

- Filter toggles: All / Open / Ready / Done.
- Bundle cards: name (or auto-generated from requestors), item count, packed count, fulfiller, date.

### 7.9 Restock Creation (`/restocks/new?list_id=X`)

- Select list.
- Item picker with quantity inputs.
- Optional note ("Costco run 3/15").
- Submit immediately updates inventory and navigates to confirmation view.

### 7.10 Settings (`/settings`)

- **User Management**: list of users, edit name/color, add/remove.
- **About**: version, database size, link to backup download.

---

## 8. Project Structure

```
hearthledger/
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── compose.yaml
├── .env.example
├── migrations/
│   └── 001_initial.sql
├── src/
│   ├── main.rs                  -- startup, config, router assembly, CLI flags
│   ├── config.rs                -- env var parsing
│   ├── db.rs                    -- SQLite pool setup, pragmas
│   ├── error.rs                 -- AppError type, IntoResponse impl
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── list.rs
│   │   ├── item.rs
│   │   ├── request.rs           -- Request, RequestItem, CreateRequest
│   │   ├── bundle.rs            -- Bundle, BundleItem, BundleDetail
│   │   └── restock.rs           -- Restock, RestockItem, CreateRestock
│   ├── routes/
│   │   ├── mod.rs               -- nest all route groups
│   │   ├── users.rs
│   │   ├── lists.rs
│   │   ├── items.rs
│   │   ├── requests.rs
│   │   ├── bundles.rs
│   │   ├── bundle_items.rs
│   │   ├── restocks.rs
│   │   └── backup.rs
│   └── openapi.rs               -- utoipa ApiDoc assembly
├── frontend/
│   ├── package.json
│   ├── svelte.config.js         -- adapter-static
│   ├── vite.config.ts           -- dev proxy for /api
│   ├── src/
│   │   ├── app.html
│   │   ├── app.css              -- Tailwind directives
│   │   ├── lib/
│   │   │   ├── api/
│   │   │   │   ├── client.ts    -- openapi-fetch instance, X-User-Id injection
│   │   │   │   └── schema.d.ts  -- generated types (gitignored)
│   │   │   ├── stores/
│   │   │   │   └── user.ts      -- current user store (localStorage-backed)
│   │   │   └── components/
│   │   │       ├── Nav.svelte
│   │   │       ├── UserPicker.svelte
│   │   │       ├── ItemRow.svelte
│   │   │       ├── RequestCard.svelte
│   │   │       ├── BundleCard.svelte
│   │   │       ├── PackingChecklist.svelte
│   │   │       └── QuantityControl.svelte
│   │   ├── routes/
│   │   │   ├── +layout.svelte
│   │   │   ├── +page.svelte           -- dashboard
│   │   │   ├── lists/
│   │   │   │   ├── +page.svelte       -- all lists
│   │   │   │   └── [id]/
│   │   │   │       └── +page.svelte   -- list detail
│   │   │   ├── requests/
│   │   │   │   ├── +page.svelte       -- all requests
│   │   │   │   ├── new/
│   │   │   │   │   └── +page.svelte   -- create request
│   │   │   │   └── [id]/
│   │   │   │       └── +page.svelte   -- request detail
│   │   │   ├── bundles/
│   │   │   │   ├── +page.svelte       -- all bundles
│   │   │   │   └── [id]/
│   │   │   │       └── +page.svelte   -- bundle detail (incl. creation flow)
│   │   │   ├── restocks/
│   │   │   │   └── new/
│   │   │   │       └── +page.svelte   -- create restock
│   │   │   └── settings/
│   │   │       └── +page.svelte
│   │   └── static/
│   │       └── favicon.svg
└── scripts/
    ├── generate-openapi.sh
    └── backup.sh                -- example cron backup script
```

---

## 9. Build Pipeline

### 9.1 Development

```bash
# Terminal 1: backend with auto-reload
cargo watch -x run

# Terminal 2: frontend dev server (proxies /api to Axum)
cd frontend && npm run dev

# After changing Rust API types:
./scripts/generate-openapi.sh
```

The SvelteKit dev server proxies `/api` to the Axum backend via `vite.config.ts`:

```typescript
export default defineConfig({
  server: {
    proxy: {
      '/api': 'http://localhost:3001'  // Axum dev port
    }
  }
});
```

### 9.2 OpenAPI Generation

```bash
#!/bin/bash
# scripts/generate-openapi.sh
set -euo pipefail

cargo build --release
./target/release/hearthledger --export-openapi > frontend/src/lib/api/openapi.json
cd frontend
npx openapi-typescript src/lib/api/openapi.json -o src/lib/api/schema.d.ts
```

The Rust binary accepts `--export-openapi` to print the spec to stdout and exit.

### 9.3 Docker Build (Multi-Stage)

```dockerfile
# Stage 1: Build frontend
FROM node:22-slim AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
COPY openapi.json src/lib/api/openapi.json
RUN npx openapi-typescript src/lib/api/openapi.json -o src/lib/api/schema.d.ts
RUN npm run build

# Stage 2: Build backend
FROM rust:1.82-bookworm AS backend-build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-build /app/target/release/hearthledger .
COPY --from=frontend-build /app/frontend/build ./static
COPY migrations/ migrations/

ENV HEARTHLEDGER_PORT=8080
ENV HEARTHLEDGER_DB_PATH=/data/hearthledger.db
EXPOSE 8080
VOLUME ["/data"]

CMD ["./hearthledger"]
```

**sqlx note**: The Docker build uses `SQLX_OFFLINE=true` with a checked-in `.sqlx/` directory (generated by `cargo sqlx prepare`).

### 9.4 Docker Compose

```yaml
# compose.yaml — compatible with Synology Container Manager
services:
  hearthledger:
    image: hearthledger:latest
    build: .
    ports:
      - "8080:8080"
    environment:
      HEARTHLEDGER_PASSWORD: "${HEARTHLEDGER_PASSWORD}"   # REQUIRED
      HEARTHLEDGER_USERNAME: "family"
    volumes:
      - /volume1/docker/hearthledger/data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
```

The volume path `/volume1/docker/hearthledger/data` follows Synology convention. The healthcheck uses `wget` instead of `curl` since the slim runtime image may not have curl installed (wget is lighter to include, or the healthcheck can be configured in Synology's GUI instead).

---

## 10. Configuration

All configuration via environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HEARTHLEDGER_PASSWORD` | **Yes** | — | HTTP basic auth password. App won't start without it. |
| `HEARTHLEDGER_USERNAME` | No | `family` | HTTP basic auth username. |
| `HEARTHLEDGER_PORT` | No | `8080` | Listen port. |
| `HEARTHLEDGER_DB_PATH` | No | `/data/hearthledger.db` | SQLite database file path. |
| `HEARTHLEDGER_LOG_LEVEL` | No | `info` | tracing log level. |

---

## 11. Axum Router Assembly

```rust
// Unauthenticated routes
let public_routes = Router::new()
    .route("/health", get(health))
    .route("/api/openapi.json", get(openapi_spec));

// Authenticated API routes
let api_router = Router::new()
    .nest("/users", user_routes())
    .nest("/lists", list_routes())
    .nest("/items", item_routes())
    .nest("/requests", request_routes())
    .nest("/bundles", bundle_routes())
    .nest("/bundle-items", bundle_item_routes())
    .nest("/restocks", restock_routes())
    .route("/backup", get(backup));

let authenticated = Router::new()
    .nest("/api/v1", api_router)
    .fallback_service(
        ServeDir::new("static")
            .fallback(ServeFile::new("static/index.html"))
    )
    .layer(CompressionLayer::new())
    .layer(ValidateRequestHeaderLayer::basic(&config.username, &config.password));

let app = public_routes
    .merge(authenticated)
    .layer(TraceLayer::new_for_http())
    .with_state(app_state);
```

The `fallback` to `index.html` is critical — SvelteKit's client-side router handles `/lists/abc123` etc., so all non-API, non-file requests must serve the SPA shell.

---

## 12. Error Handling

### 12.1 Backend

A single `AppError` enum implementing `IntoResponse`:

```rust
enum AppError {
    NotFound(String),
    BadRequest(String),
    Conflict(String),          // duplicate item name, etc.
    Internal(anyhow::Error),
}
```

Each variant maps to an HTTP status code and the consistent error envelope. `Internal` logs the full error server-side, returns a generic message to the client.

### 12.2 Frontend

The `openapi-fetch` client is wrapped in a helper that:
- Checks for error responses and throws typed errors.
- On 401, reloads the page to trigger the browser's basic auth prompt.
- On 400, surfaces the error message in a toast.
- On 500, shows a generic "something went wrong" toast.

---

## 13. Deployment Notes (Synology NAS)

### 13.1 Container Manager Setup

Synology Container Manager is a Docker Compose GUI. To deploy:

1. Create a shared folder for data persistence: e.g., `/volume1/docker/hearthledger/data`.
2. Import the `compose.yaml` or build from a Git repository.
3. Set `HEARTHLEDGER_PASSWORD` in the Container Manager environment variables UI.
4. Map port 8080 (or whatever external port you prefer).
5. The SQLite database file will be created at first startup in the mapped `/data` volume.

### 13.2 Networking

Since the NAS has a static IP on a fiber line, expose the app through one of:

- **Synology's built-in reverse proxy** (Control Panel → Login Portal → Advanced → Reverse Proxy): terminate TLS here with a Let's Encrypt cert (Synology has built-in ACME support), proxy to `localhost:8080`. This is the recommended approach — handles TLS with zero extra containers, and the app's basic auth provides the access control layer.
- **Cloudflare Tunnel**: if you'd rather not expose the NAS IP directly. Runs as a sidecar container. Has the bonus of hiding the origin IP and providing DDoS protection.
- **Direct port forward**: simplest, but no TLS unless handled at the app level (not recommended for internet-facing).

### 13.3 Backup via Task Scheduler

In Synology DSM → Control Panel → Task Scheduler:

1. Create a "User-defined script" task.
2. Schedule: daily at a quiet hour.
3. Script: the backup script from Section 5.3, pointed at `http://localhost:8080/api/v1/backup` (localhost — no TLS needed).
4. Notification: enable email on abnormal termination.

### 13.4 Resource Considerations

Hearthledger is extremely lightweight:
- **Memory**: ~20-30 MB RSS (Rust + SQLite, no runtime/GC).
- **Disk**: <50 MB for the container image. Database grows with usage but will be tiny at family scale.
- **CPU**: negligible — SQLite queries on this data volume are sub-millisecond.

This will run comfortably alongside other containers on the NAS without competing for resources.

---

## 14. Future Considerations

Out of scope for v1, but the design doesn't preclude:

- **Categories/tags on items**: `items` can gain a `tags` JSON column or join table.
- **Low-stock notifications**: data model supports computing `target_qty - on_hand_qty`; push/email alerts could be layered on.
- **Request history / audit log**: timestamps provide basic history. A dedicated `activity_log` table could be added.
- **Per-household views**: add a `household` field to users and filter UIs by household. The unified data model stays the same.
- **Barcode scanning**: frontend feature; API already accepts item lookups by name.
- **PWA / offline support**: static SPA is PWA-friendly; a service worker could enable offline list browsing.
- **Delivery tracking**: bundles could gain a `shipped`/`in_transit` status for scenarios where items are delivered rather than picked up.

---

## Appendix A: SQLite Migration (001_initial.sql)

```sql
-- Bundles (created before requests due to FK dependency)
CREATE TABLE bundles (
    id           TEXT PRIMARY KEY,
    name         TEXT,
    created_by   TEXT NOT NULL,              -- FK added after users table exists
    status       TEXT NOT NULL DEFAULT 'open'
                 CHECK (status IN ('open', 'ready', 'done')),
    note         TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    completed_at TEXT
);

-- Users
CREATE TABLE users (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    color       TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Inventory lists
CREATE TABLE lists (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Items within lists
CREATE TABLE items (
    id          TEXT PRIMARY KEY,
    list_id     TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    unit        TEXT,
    target_qty  REAL NOT NULL DEFAULT 0,
    on_hand_qty REAL NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(list_id, name)
);

-- Requests (users asking for items from inventory)
CREATE TABLE requests (
    id            TEXT PRIMARY KEY,
    list_id       TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    requested_by  TEXT NOT NULL REFERENCES users(id),
    bundle_id     TEXT REFERENCES bundles(id) ON DELETE SET NULL,
    status        TEXT NOT NULL DEFAULT 'open'
                  CHECK (status IN ('open', 'bundled', 'done', 'cancelled')),
    note          TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    completed_at  TEXT
);

-- Line items within requests
CREATE TABLE request_items (
    id          TEXT PRIMARY KEY,
    request_id  TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    item_id     TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    quantity    REAL NOT NULL
);

-- Items packed into a bundle (references request_items for traceability)
CREATE TABLE bundle_items (
    id               TEXT PRIMARY KEY,
    bundle_id        TEXT NOT NULL REFERENCES bundles(id) ON DELETE CASCADE,
    request_item_id  TEXT NOT NULL REFERENCES request_items(id) ON DELETE CASCADE,
    packed           INTEGER NOT NULL DEFAULT 0
);

-- Restocks (adding inventory)
CREATE TABLE restocks (
    id          TEXT PRIMARY KEY,
    list_id     TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    created_by  TEXT NOT NULL REFERENCES users(id),
    note        TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Line items within restocks
CREATE TABLE restock_items (
    id          TEXT PRIMARY KEY,
    restock_id  TEXT NOT NULL REFERENCES restocks(id) ON DELETE CASCADE,
    item_id     TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    quantity    REAL NOT NULL
);

-- Indexes
CREATE INDEX idx_items_list_id ON items(list_id);
CREATE INDEX idx_requests_list_id ON requests(list_id);
CREATE INDEX idx_requests_status ON requests(status);
CREATE INDEX idx_requests_requested_by ON requests(requested_by);
CREATE INDEX idx_requests_bundle_id ON requests(bundle_id);
CREATE INDEX idx_request_items_request_id ON request_items(request_id);
CREATE INDEX idx_request_items_item_id ON request_items(item_id);
CREATE INDEX idx_bundles_status ON bundles(status);
CREATE INDEX idx_bundles_created_by ON bundles(created_by);
CREATE INDEX idx_bundle_items_bundle_id ON bundle_items(bundle_id);
CREATE INDEX idx_bundle_items_request_item_id ON bundle_items(request_item_id);
CREATE INDEX idx_restocks_list_id ON restocks(list_id);
CREATE INDEX idx_restock_items_restock_id ON restock_items(restock_id);
```

---

## Appendix B: OpenAPI Type Examples

```rust
// --- Requests ---

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Request {
    pub id: String,
    pub list_id: String,
    pub requested_by: String,
    pub bundle_id: Option<String>,
    pub status: RequestStatus,
    pub note: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub items: Vec<RequestItemDetail>,  // joined with item name/unit
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum RequestStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "bundled")]
    Bundled,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRequest {
    pub list_id: String,
    pub note: Option<String>,
    pub items: Vec<CreateRequestItem>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRequestItem {
    pub item_id: String,
    pub quantity: f64,
}

// --- Bundles ---

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Bundle {
    pub id: String,
    pub name: Option<String>,
    pub created_by: String,
    pub status: BundleStatus,
    pub note: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum BundleStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "done")]
    Done,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBundle {
    pub name: Option<String>,
    pub note: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AddBundleItems {
    pub request_item_ids: Vec<String>,
}

// --- Restocks ---

#[derive(Deserialize, ToSchema)]
pub struct CreateRestock {
    pub list_id: String,
    pub note: Option<String>,
    pub items: Vec<CreateRestockItem>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRestockItem {
    pub item_id: String,
    pub quantity: f64,
}
```
