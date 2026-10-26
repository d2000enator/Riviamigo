import React from 'react';
import type { AssistantViewContext } from './viewTypes';
import { Button, Input, SelectPicker, Badge } from '@riviamigo/ui/primitives';
import { Plus } from 'lucide-react';
import { Metric } from './Metric';
import {
  localExampleDate,
  planStatusLabel,
  optionalNumber,
  copy,
  displayNumber as n,
} from './format';
import type { AssistantDemoScenario } from '@riviamigo/types';
export function TomorrowSection({ context }: { context: AssistantViewContext }) {
  const {
    draft,
    dirty,
    previewResult,
    evaluation,
    prefs,
    metric,
    time,
    distance,
    planEdit,
    setPlanEdit,
    removeId,
    setRemoveId,
    change,
    preference,
    addExamples,
  } = context;
  return (
    <>
      <div className="rm-assistant-section-heading">
        <div>
          <h3>Ready for your next departure</h3>
          <p>
            Enter driving distance and an arrival reserve. Estimates use your assumptions and the
            latest available battery reading.
          </p>
        </div>
        <Button
          variant="secondary"
          iconLeft={<Plus size={16} />}
          onClick={() =>
            setPlanEdit({
              id: crypto.randomUUID(),
              title: '',
              departure_local: localExampleDate(prefs.timezone),
              distance_mi: 30,
              round_trip: false,
            })
          }
        >
          Add trip
        </Button>
      </div>
      {evaluation.is_demo ? (
        <div className="rm-assistant-demo">
          <div>
            <strong>Explore before delivery</strong>
            <p>
              Choose a clearly labeled scenario, then preview it. Real vehicles cannot use these
              simulated readings.
            </p>
          </div>
          <Button variant="secondary" onClick={addExamples}>
            Add examples
          </Button>
          <SelectPicker
            aria-label="Demo scenario"
            value={prefs.demo_scenario}
            onChange={(v) => preference('demo_scenario', v as AssistantDemoScenario)}
            options={[
              { value: 'recorded', label: 'Recorded demo history' },
              { value: 'ready', label: 'Ready to go' },
              { value: 'low_charge', label: 'Not enough charge' },
              { value: 'charging_interrupted', label: 'Charging interrupted' },
              { value: 'fluid_warning', label: 'Washer fluid warning' },
              { value: 'vacation_drain', label: 'Vacation battery loss' },
            ]}
          />
        </div>
      ) : null}
      {planEdit ? (
        <form
          className="rm-assistant-panel rm-assistant-form"
          onSubmit={(event) => {
            event.preventDefault();
            const plan = planEdit;
            change((d) => {
              const index = d.plans.findIndex((p) => p.id === plan.id);
              if (index < 0) d.plans.push(plan);
              else d.plans[index] = plan;
            });
            setPlanEdit(null);
          }}
        >
          <h4>Trip plan</h4>
          <Input
            label="Trip name"
            required
            maxLength={120}
            value={planEdit.title}
            onChange={(e) => setPlanEdit({ ...planEdit, title: e.target.value })}
          />
          <Input
            label={`Departure (${prefs.timezone})`}
            type="datetime-local"
            required
            value={planEdit.departure_local}
            onChange={(e) => setPlanEdit({ ...planEdit, departure_local: e.target.value })}
          />
          <Input
            label={`One-way driving distance (${metric ? 'km' : 'mi'})`}
            type="number"
            min="0.1"
            max={metric ? 8046 : 5000}
            step="0.1"
            required
            value={
              metric ? Number((planEdit.distance_mi * 1.609344).toFixed(1)) : planEdit.distance_mi
            }
            onChange={(e) =>
              setPlanEdit({
                ...planEdit,
                distance_mi: Number(e.target.value) / (metric ? 1.609344 : 1),
              })
            }
          />
          <label className="rm-assistant-check">
            <input
              type="checkbox"
              checked={planEdit.round_trip}
              onChange={(e) => setPlanEdit({ ...planEdit, round_trip: e.target.checked })}
            />
            Include the return drive
          </label>
          <div className="rm-assistant-actions">
            <Button type="submit">Keep in draft</Button>
            <Button type="button" variant="ghost" onClick={() => setPlanEdit(null)}>
              Cancel
            </Button>
          </div>
        </form>
      ) : null}
      {!draft.plans.length ? (
        <div className="rm-assistant-empty">
          No trips planned yet. Add a destination and driving distance, or try the demo examples.
        </div>
      ) : (
        <div className="rm-assistant-grid">
          {draft.plans.map((plan) => {
            const a =
              dirty && !previewResult ? undefined : evaluation.plans.find((a) => a.id === plan.id);
            return (
              <article key={plan.id} className="rm-assistant-panel">
                <div className="rm-assistant-section-heading">
                  <h4>{plan.title}</h4>
                  <Badge
                    variant={
                      a?.status === 'ready'
                        ? 'success'
                        : a?.status === 'waiting_for_data' || !a
                          ? 'default'
                          : 'warning'
                    }
                  >
                    {a ? planStatusLabel(a.status) : 'Not assessed'}
                  </Badge>
                </div>
                <p>
                  {a
                    ? time(a.departure_at)
                    : `${plan.departure_local.replace('T', ' ')} · ${prefs.timezone}`}
                </p>
                <div className="rm-assistant-metrics">
                  <Metric
                    label="Planned driving"
                    value={distance(plan.distance_mi * (plan.round_trip ? 2 : 1))}
                  />
                  <Metric
                    label="Departure charge estimate"
                    value={
                      a?.required_soc_pct != null && a.required_soc_pct <= 100
                        ? `${n(a.required_soc_pct)}%`
                        : a?.status === 'charging_stop_needed'
                          ? 'Charging stop'
                          : '—'
                    }
                  />
                  <Metric
                    label="Estimated arrival"
                    value={a?.arrival_soc_pct == null ? '—' : `${n(a.arrival_soc_pct)}%`}
                  />
                  <Metric
                    label="Energy estimate"
                    value={
                      a?.estimated_energy_kwh == null ? '—' : `${n(a.estimated_energy_kwh, 1)} kWh`
                    }
                  />
                </div>
                <p className="rm-assistant-explanation">
                  {a?.explanation ?? 'Preview or save your changes to assess this trip.'}
                </p>
                {a?.estimated_charge_hours != null ? (
                  <p>
                    Charging estimate: {n(a.estimated_charge_hours, 1)} hours at your entered power,
                    including a 10% loss allowance.
                  </p>
                ) : null}
                <div className="rm-assistant-actions">
                  <Button variant="ghost" onClick={() => setPlanEdit(copy(plan))}>
                    Edit
                  </Button>
                  {removeId === plan.id ? (
                    <>
                      <Button
                        variant="danger"
                        onClick={() => {
                          change((d) => {
                            d.plans = d.plans.filter((p) => p.id !== plan.id);
                          });
                          setRemoveId(null);
                        }}
                      >
                        Remove from draft
                      </Button>
                      <Button variant="ghost" onClick={() => setRemoveId(null)}>
                        Cancel
                      </Button>
                    </>
                  ) : (
                    <Button variant="ghost" onClick={() => setRemoveId(plan.id)}>
                      Remove
                    </Button>
                  )}
                </div>
              </article>
            );
          })}
        </div>
      )}
      <details className="rm-assistant-panel">
        <summary>Planning assumptions</summary>
        <div className="rm-assistant-form rm-assistant-grid">
          <Input
            label="Arrival reserve (%)"
            type="number"
            min={5}
            max={80}
            value={prefs.reserve_pct}
            onChange={(e) => preference('reserve_pct', Number(e.target.value))}
          />
          <Input
            label="Assumed consumption (Wh/mi)"
            type="number"
            min={100}
            max={1500}
            value={prefs.assumed_efficiency_wh_mi}
            onChange={(e) => preference('assumed_efficiency_wh_mi', Number(e.target.value))}
          />
          <Input
            label="Driving/weather buffer (%)"
            type="number"
            min={0}
            max={100}
            value={prefs.weather_buffer_pct}
            onChange={(e) => preference('weather_buffer_pct', Number(e.target.value))}
          />
          <Input
            label="Capacity fallback (kWh, optional)"
            type="number"
            min={20}
            max={250}
            value={prefs.battery_capacity_kwh ?? ''}
            onChange={(e) => preference('battery_capacity_kwh', optionalNumber(e.target.value))}
          />
          <Input
            label="Estimated charging power (kW, optional)"
            type="number"
            min={0.1}
            max={400}
            step={0.1}
            value={prefs.estimated_charger_kw ?? ''}
            onChange={(e) => preference('estimated_charger_kw', optionalNumber(e.target.value))}
          />
          <Input
            label="Departure timezone"
            value={prefs.timezone}
            onChange={(e) => preference('timezone', e.target.value)}
          />
        </div>
        <p>
          Driving distance is entered manually. The buffer is your planning allowance; no calendar,
          map route, weather forecast or vehicle control is connected here.
        </p>
      </details>
    </>
  );
}
