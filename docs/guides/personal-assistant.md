---
title: Personal R2 assistant
description: Private trip planning, dashboard alerts, weekly history and service records in this local R2 extension.
---

# Personal R2 assistant

This local extension adds **Assistant** to the sidebar and a shortcut on the R2
Overview. It uses your existing Riviamigo login. Select the demo vehicle to try
examples before delivery; choose the real vehicle after normal enrollment.
Each vehicle and each user has separate saved plans and service records.

## Six sections

- **Tomorrow:** enter a trip name, local departure time, one-way driving distance
  and whether to count the return. Choose an arrival reserve, an assumed
  efficiency and a driving/weather buffer. Recorded battery capacity is used
  where available; otherwise enter a capacity or use the reported-range estimate.
  All figures are estimates. A requirement above100% means plan a charging stop.
- **Alerts:** enable dashboard monitoring and save. The server checks about once
  a minute while the browser is closed. It retains, deduplicates and lets you
  acknowledge warnings for insufficient charge, short estimated charging time,
  charging stopping below target, a fresh charging limit below the departure estimate, and fresh vehicle-reported washer/brake-fluid
  warnings, your service reminders and preparation time. No email or phone push
  service is connected. Open the dashboard to read the inbox.
- **Vacation:** enable watch after parking and save a fresh baseline. At least
  six hours and a current battery reading are needed for a parked-loss rate.
  Driving, charging or an increase in charge invalidates that interval. The
  return-charge projection is linear, not a battery-health diagnosis.
- **Weekly:** compare the latest seven days of completed recorded trips and
  charges with the prior seven. Detailed cards explain changes in matched energy
  per distance, charging cost and observed parked loss; they show their sample
  coverage and withhold comparisons when history is insufficient. Demo history anchors to its last recorded event.
  Missing distance, energy or prices remain identified. Recorded cost and
  estimates using your optional home tariff are separate. Download a text report.
- **Maintenance:** record service, date, mileage, amount, notes and your own next
  reminder. Save an entry before attaching PDF/JPEG/PNG receipts (5MiB/file,
  100files/100MiB per user and vehicle). Download receipts or export the notebook.
  Removing an entry and saving also removes its receipts; make a backup first.
- **Departure:** choose a cabin-temperature preference and a preparation lead
  time. This creates dashboard reminders for manually entered trips. It sends
  no climate, lock, drive, camera or other vehicle commands. Use the Rivian app
  for cabin preparation until a separately authorized integration is verified.

**Preview changes** evaluates your draft without saving it or changing the inbox.
**Save changes** stores it on the NAS. If another device saves first, an outdated
edit is rejected; reload and reapply the intended changes. Switching vehicles or
leaving the page discards unsaved edits. Finish a trip/service form with **Keep in
draft**, then use **Save changes**.

## Demo and data limits

**Add examples** creates clearly labeled example trip/service drafts only when
none exist. Demo scenarios simulate readiness, low charge, interrupted charging,
washer fluid and parked loss. Save a scenario to exercise the saved alert inbox;
preview alone leaves the saved scenario unchanged. Real vehicles cannot enable
simulated readings. Example service records and receipts do not transfer to a
newly enrolled real vehicle.

Battery, range, charging, fluid and mileage fields have freshness checks. Stale
battery data pauses readiness judgments; a sleeping car may report less often.
Missing fresh fluid data does not clear an existing warning. These features do
not wake the car. Actual R2 field availability must be verified after delivery.
Manual distances are not navigation routes. Buffers are assumptions, not weather
forecasts. Charging power is an entered assumption with a90% allowance; schedules
and taper can change actual charging time. No calendar account is connected.
Dates use the saved IANA timezone (initially America/Chicago); ambiguous or skipped
clock-change departure times are rejected. Distance and cabin inputs follow
existing display units; costs are explicitly USD.

## Storage and recovery

Documents, observations, alerts and receipt bytes live in separately versioned,
additive `riviamigo.personal_assistant_*` tables. They are included in the normal
full database dump and consistent volume snapshots. They are not in the
backup-excluded `system_config` store. Keep the complete recovery bundle,
configuration/encryption keys and exact application image off NAS. A notebook
JSON export does not include receipt files or replace a full backup.

The extension leaves the27 upstream SQLx migration entries intact. Reverting to
the preceding image leaves extension tables and records in place; it hides the
new screens. Restore only into a fresh isolated database for testing. A rollback
of a failed upgrade must preserve any records saved after activation.

## Detailed weekly comparisons in v0.3.1

The detailed report loads only when Weekly is open and refreshes about every five
minutes. Normal Overview and alert monitoring do not run its parked-history
analysis. Saved tariff changes invalidate the weekly view immediately.

Energy-per-distance comparisons use matched trip energy and mileage, at least
three matched trips and80% known-distance coverage in each window, with no unknown
trip distances. Weather or trip-length differences are descriptive context, not
proof of cause. Charging-cost attribution needs complete prices and energy; it
separates changed charging volume at the previous average unit price from the
remaining average-price effect. Recorded and estimated prices remain separate.
The same currently entered home tariff is used for estimates in both windows.

Parked loss reuses Riviamigo's validated parked-session detector, without Parallax.
Only fully contained intervals of at least six hours with battery anchors within
45minutes count. A rate comparison needs at least12observed hours in each window.
Intervals spanning a week boundary are excluded rather than counted twice.
Missing observations stay unknown. Reported rates describe observed intervals,
not every parked hour, an identified vehicle system, or battery degradation.
