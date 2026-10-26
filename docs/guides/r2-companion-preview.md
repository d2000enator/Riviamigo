---
title: R2 Companion preview
description: An unofficial, source-available R2 dashboard and personal assistant preview.
---

# R2 Companion preview — v0.3.2

This is an unofficial fork of [Riviamigo](https://github.com/bballdavis/Riviamigo),
based on the upstream **2026.10.1** release. Credit for Riviamigo and its existing
features belongs to its upstream contributors. This fork retains GPL-3.0-only.
It is not an official release, an endorsed Rivian product, or a maintained
replacement for upstream security updates.

## What is different

- An optional R2 Overview with shared Launch Green concept artwork, recent trips,
  charging and battery history. Classic dashboards remain available.
- A private, saved Assistant with manually entered trip planning, dashboard-only
  alerts, vacation parked-loss estimates, evidence-based weekly comparisons,
  service records/receipt attachments and departure reminders.
- A conservative trip-detection fallback for missing power/gear signals,
  proposed for upstream [issue #75](https://github.com/bballdavis/Riviamigo/issues/75).
- An end-of-stream capture-download repair for upstream
  [issue #76](https://github.com/bballdavis/Riviamigo/issues/76).
- Missing charging power is displayed as unavailable instead of a false zero.

The [Assistant guide](./personal-assistant.md) describes what these features
actually do, their data limits, storage, and recovery. The
[architecture notes](../architecture/personal-assistant.md) explain their owning
seams and failure behavior. Generated artwork is illustrative; it does not prove
vehicle compatibility.

![Illustrative Launch Green R2 artwork](https://raw.githubusercontent.com/d2000enator/Riviamigo/r2-companion-preview-v0.3.2/apps/web/public/r2-dashboard/r2-launch-green-v0.1.0.png)

## Validation and release status

The custom runtime has been exercised with demo and synthetic data on a Synology
x86-64 NAS, including isolated restore, authorization, private HTTPS access,
WebSockets, saved-content restarts, and desktop/mobile browser layouts. These
are local results, not independent certification.

Focused checks of the exact source include 26 trip-signal tests, two actual HTTP
capture-body regressions, 40 Assistant backend tests, two real-router private
cache regressions and 19 frontend tests. Frontend and documentation builds passed.
Eight existing architecture-guard failures are known; a complete green CI run
is not claimed. The public contribution branches describe checks against their
own upstream development base separately.

**Live R2 telemetry has not been validated by this fork's contributor.** Demo
success does not establish real R2 field availability, trip boundaries or charge
behavior. The fork adds no vehicle commands, phone push, calendar connection,
Roamer or Parallax integration. Existing upstream capabilities are distinct.

This is a source preview for review and development. No stable binary release
or safe automatic upgrade is promised. A companion installer prototype is still
awaiting a fresh-machine rehearsal; it is not a recommended installer yet.

## Safe evaluation

Use the upstream [contributor setup](../../README.md) in a separate disposable
development environment and add a demo vehicle through the application's normal
flow. Keep production databases, volumes, credentials and backup keys separate.
The source contains no deployed instance configuration, real vehicle data or
recovery archives. Example configuration values are placeholders and must never
be used as production secrets.

The Assistant creates its own four additive `personal_assistant_*` tables with
an extension schema version, leaving the 27 SQLx entries at this release baseline
unchanged. Review this fork's schema and recovery contract before any upgrade;
this branch is not rebased onto the newer upstream development schema.

## Contributions and support

The two bug fixes are offered separately to upstream so maintainers can review
them without adopting this dashboard. Larger feature changes remain here for
discussion rather than a single unsolicited upstream patch. Follow upstream's
[contribution policy](../contributing.md), retain attribution, disclose AI
assistance and request human review.

AI assistance was used for implementation, tests, documentation and publication
preparation. The contributor evaluated the demo UI and private access. Maintainer
code review and live R2-owner confirmation remain outstanding; neither is
represented as already complete. Report bugs using synthetic/redacted examples,
never credentials, VINs, raw telemetry or precise locations. Security disclosures
should follow [SECURITY.md](../../SECURITY.md).
