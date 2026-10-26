---
title: Personal assistant extension architecture
description: Private per-user vehicle planning, persistent alert monitoring and receipt storage contracts.
---

# Personal assistant extension v0.3.2

The thin `/assistant` route composes the shared app shell, a page controller and
six focused section components. Typed React Query hooks include user and vehicle
in their cache keys and opt out of persistence to browser storage. Reads refresh
once per minute. Drafts are explicit; preview cannot mutate saved documents or
alerts. Updates require the current revision and return409 on a conflicting edit.

Rust services separate typed documents, pure evaluation, database persistence and
an optional background worker. Initialization uses a transaction/advisory lock and
an extension-owned schema version. It does not alter or repair the upstream SQLx
ledger. Initialization failure disables extension routes with503 while core
application startup continues. A60-second worker evaluates documents for current,
enabled vehicle members, independently of browser sessions. No new environment
variables, services, third-party dependencies or external notification providers.

## Session-only HTTP surface

All paths below are prefixed `/v1/vehicles/{vehicle_id}/assistant`. Every endpoint
requires a current session and vehicle membership; API keys are rejected. All
stored access is scoped by both authenticated user and vehicle, including alert
acknowledgments and receipt downloads. Membership allows a user to edit their own
private records; no vehicle command authority is granted.

Both ordinary and receipt-upload routers explicitly send `Cache-Control:
private, no-store` and vary responses by `Authorization` and `Cookie`. This
policy wraps their body/rate limits, including rejected requests; the separate
v2 theme router's cache policy cannot protect these v1 routes. Revision v0.3.2
corrects that omission found by isolated HTTP acceptance. The production router
is regression-tested without live database/cache clients for all unsigned
assistant endpoints and oversized-upload rejection. The lab acceptance runner
also checks cache headers on authenticated responses. Storage version remains1;
no upstream migration or authentication permission changes are required.

| Method and suffix | Contract |
|---|---|
| GET | Saved document/revision, assessments, weekly aggregates, inbox and receipt metadata |
| PUT | `{revision, document, reset_vacation?}`; validate before storage; reject conflicts |
| GET `/weekly` | Lazy detailed seven-day comparisons, matched samples and explicit coverage; no new collection |
| POST `/preview` | `{document}`; pure assessment using current data and saved observations |
| POST `/alerts/{alert_id}/acknowledge` | Idempotent acknowledgment scoped to user/vehicle |
| POST `/maintenance/{entry_id}/receipts` | Filename, exact MIME and base64 bytes; entry must be saved |
| GET `/receipts/{receipt_id}` | JSON filename/MIME/base64/SHA256, never executable inline content |

Ordinary JSON bodies are bounded256KiB; receipt bodies7MiB with matching Axum and
transport limits. Decoded receipts are1byte–5MiB with PDF/JPEG/PNG signature checks,
path-free filenames and a serialized100file/100MiB quota. Downloads use browser
Blob attachments. No uploaded content is executed or rendered by the server.
Normal session identity rate limits still apply. No authentication material or
vehicle-account tokens are returned in extension DTOs.

## Persistence, evaluation and rollback

The four `riviamigo.personal_assistant_*` tables store extension version,
revisioned JSON documents/observations, immutable-identity alert occurrences and
receipt bytes/hashes. Row locks serialize edits, observation updates and uploads.
A partial unique index retains only one active occurrence per rule/user/vehicle;
acknowledgments survive repeated observations. Fresh false conditions resolve
warnings; unknown/stale signals do not establish resolution. Deleted trip/service
rules become inactive. Paused monitoring retains existing history.

Field timestamps and bounded recent telemetry establish freshness. Every result
identifies demo status, estimates and missing inputs. Real vehicles reject demo
scenarios. The evaluator has no Rivian command or message-sending dependency and
always returns `vehicle_commands_enabled=false`. Time inputs use explicit IANA
zones and reject ambiguous/nonexistent local departures. Weekly aggregates use
completed history and current user's charge annotations, with missing-cost and
energy counts; recorded and tariff-estimated cost remain separate.

Full schema dumps include all extension tables and receipts. Old images ignore
these additive tables; leave them intact for rollback. Upstream recovery builds
a fresh staging database before restore/swap; never use production mounts for
an isolated test. Before activation, validate HTTP authorization, revisions,
worker deduplication/resolution, receipt limits/hash/persistence and an isolated
restore containing populated extension records. After replacement/restart check
normal routes, login, private listeners, core ledger and new saved records.

## Weekly analysis v0.3.1

The new GET is session/member protected exactly like the other extension reads.
It runs only when its separate per-user/vehicle React Query hook mounts in the
Weekly section; its five-minute refresh is independent of the minute alert loop.
No tables or upstream migrations are added by this revision. Unknown historical
metrics stay null. Driving uses paired mileage/energy records; charge-price
annotations are joined on the unique user/session key. Invalid/nonfinite costs
and energy are excluded consistently from totals and coverage.

Parked observations reuse `fetch_idle_drain_periods` in the existing idle-drain
route module. The comparison additionally requires fully contained six-hour
intervals and SOC anchors within45minutes; cross-boundary intervals and movement
or charging are excluded. Descriptive temperature/trip-length context is never
reported as an isolated causal effect. Tests cover cost-decomposition identity,
missingness, real zeroes, small samples, unknown distances, weighted parked loss,
window overlap and unreliable boundaries. The lab runner exercises the detailed
HTTP endpoint and confirms another user's charge annotation is not returned.
