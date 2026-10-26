---
title: Personal R2 dashboard promotion
description: Validate and reversibly promote the local R2 frontend on Synology.
---

# Personal R2 dashboard promotion — v0.2.0

Documentation impact: both internal and user-facing updates, included with
customization, branding and architecture. This frontend extends the installed
local trip/capture repair build; it is not an upstream release.

Build the locked workspace and run focused measurement/composition/Overview
regressions, color/theme checks and docs checks/build. Package only compiled
frontend assets. The derivative image must use the exact installed repair image
as FROM, copying only /app/dist. Verify inherited binaries, migrations and runtime
configuration are unchanged. Do not rebuild or downgrade PostgreSQL.

Inventory production and retained lab mounts before starting. Back up lab state
before reuse; never overwrite credentials. Keep build/transfers/records in one
versioned workflow folder. Temporary Serve8443 remains owner-only, with its gateway
on loopback; keep production443 and policy unchanged. Validate the candidate
against isolated demo state and browser login, charts, filters, nulls, Classic
escape, units and mobile layout. Remove temporary route/gateway on completion.

Before promotion, take a fresh stopped six-volume/configuration backup, encrypt
with the existing public certificate, verify a Mac recovery copy and isolated
restore. Pin the candidate image ID in the existing production override. Preserve
project identity, .env, mounts, DB/Redis identities, policies and private access.
Check owner login, WebSockets, Classic/R2 switch, preferences and app restart.

Use the saved override/prior image for rollback only when the migration ledger
is unchanged. Otherwise preserve state and restore in isolation. Retire only
identified temporary resources after success, preserve encrypted recovery and the
original lab until actual R2 acceptance, and update the fixed NAS inventory plus
a versioned Mac copy. No global prune or down -v.

## Frontend correction v0.2.1

Populated isolated demo validation exposed that the existing trip and charging
list DTOs omit row `vehicle_id`; the shared transport normalizes it to an empty
string. The R2 overview accepts this known sentinel only within the existing
authenticated, vehicle-keyed hooks. Explicit conflicting IDs are still rejected,
range placeholders are still hidden, and the content remains keyed by selected
vehicle. This corrects hidden trip/charge cards without a backend or schema change.
Two regression cases cover actual list DTO behavior and a conflicting charge row.
The stable per-account Classic preference key is preserved across this patch.

## Missing charging power v0.2.2

The built-in latest demo session contains SOC samples without recorded power.
The existing curve transport turns null/missing power into zero. The R2 view
therefore leaves an unmarked zero unavailable instead of displaying a false
measurement. Explicitly recorded zero remains visible, positive legacy values
remain usable, and SOC-derived power gets an estimate caption. No API, database
or demo history is changed. Populated runtime validation exposed this case.
