import React from 'react';
import type { AssistantViewContext } from './viewTypes';
import { Button, Input } from '@riviamigo/ui/primitives';
import { Plus, Download } from 'lucide-react';
import { localExampleDate, downloadText, optionalNumber, copy, displayNumber as n } from './format';
export function MaintenanceSection({ context }: { context: AssistantViewContext }) {
  const {
    draft,
    data,
    prefs,
    busy,
    dirty,
    metric,
    distance,
    serviceEdit,
    setServiceEdit,
    removeId,
    setRemoveId,
    change,
    addReceipt,
    downloadReceipt,
  } = context;
  return (
    <>
      <div className="rm-assistant-section-heading">
        <div>
          <h3>Your service notebook</h3>
          <p>Keep dates, mileage, costs, receipts and reminders together.</p>
        </div>
        <Button
          variant="secondary"
          iconLeft={<Plus size={16} />}
          onClick={() =>
            setServiceEdit({
              id: crypto.randomUUID(),
              title: '',
              performed_on: localExampleDate(prefs.timezone, 0).slice(0, 10),
              odometer_mi: null,
              cost_usd: null,
              receipt_reference: '',
              notes: '',
              next_due_on: null,
              next_due_odometer_mi: null,
            })
          }
        >
          Add service
        </Button>
      </div>
      {serviceEdit ? (
        <form
          className="rm-assistant-panel rm-assistant-form"
          onSubmit={(event) => {
            event.preventDefault();
            const entry = serviceEdit;
            change((d) => {
              const index = d.maintenance.findIndex((e) => e.id === entry.id);
              if (index < 0) d.maintenance.push(entry);
              else d.maintenance[index] = entry;
            });
            setServiceEdit(null);
          }}
        >
          <h4>Service entry</h4>
          <Input
            label="Service name"
            required
            maxLength={120}
            value={serviceEdit.title}
            onChange={(e) => setServiceEdit({ ...serviceEdit, title: e.target.value })}
          />
          <Input
            label="Service date"
            type="date"
            required
            value={serviceEdit.performed_on}
            onChange={(e) => setServiceEdit({ ...serviceEdit, performed_on: e.target.value })}
          />
          <Input
            label={`Odometer (${metric ? 'km' : 'mi'}, optional)`}
            type="number"
            min={0}
            value={
              serviceEdit.odometer_mi == null
                ? ''
                : metric
                  ? Number((serviceEdit.odometer_mi * 1.609344).toFixed(1))
                  : serviceEdit.odometer_mi
            }
            onChange={(e) =>
              setServiceEdit({
                ...serviceEdit,
                odometer_mi: e.target.value
                  ? Number(e.target.value) / (metric ? 1.609344 : 1)
                  : null,
              })
            }
          />
          <Input
            label="Amount paid ($ USD, optional)"
            type="number"
            min={0}
            step={0.01}
            value={serviceEdit.cost_usd ?? ''}
            onChange={(e) =>
              setServiceEdit({ ...serviceEdit, cost_usd: optionalNumber(e.target.value) })
            }
          />
          <Input
            label="Receipt reference"
            maxLength={500}
            value={serviceEdit.receipt_reference}
            onChange={(e) => setServiceEdit({ ...serviceEdit, receipt_reference: e.target.value })}
          />
          <Input
            label="Notes"
            maxLength={1000}
            value={serviceEdit.notes}
            onChange={(e) => setServiceEdit({ ...serviceEdit, notes: e.target.value })}
          />
          <Input
            label="Next reminder date (optional)"
            type="date"
            value={serviceEdit.next_due_on ?? ''}
            onChange={(e) =>
              setServiceEdit({ ...serviceEdit, next_due_on: e.target.value || null })
            }
          />
          <Input
            label={`Next reminder odometer (${metric ? 'km' : 'mi'}, optional)`}
            type="number"
            min={0}
            value={
              serviceEdit.next_due_odometer_mi == null
                ? ''
                : metric
                  ? Number((serviceEdit.next_due_odometer_mi * 1.609344).toFixed(1))
                  : serviceEdit.next_due_odometer_mi
            }
            onChange={(e) =>
              setServiceEdit({
                ...serviceEdit,
                next_due_odometer_mi: e.target.value
                  ? Number(e.target.value) / (metric ? 1.609344 : 1)
                  : null,
              })
            }
          />
          <div className="rm-assistant-actions">
            <Button type="submit">Keep in draft</Button>
            <Button type="button" variant="ghost" onClick={() => setServiceEdit(null)}>
              Cancel
            </Button>
          </div>
        </form>
      ) : null}
      {!draft.maintenance.length ? (
        <div className="rm-assistant-empty">
          No service records yet. Reminders use the dates and mileage you enter; no official service
          schedule is assumed.
        </div>
      ) : (
        <div className="rm-assistant-grid">
          {draft.maintenance.map((entry) => (
            <article key={entry.id} className="rm-assistant-panel">
              <h4>{entry.title}</h4>
              <p>
                {entry.performed_on} ·{' '}
                {entry.odometer_mi == null ? 'Mileage not entered' : distance(entry.odometer_mi)} ·{' '}
                {entry.cost_usd == null ? 'Amount not entered' : `$${n(entry.cost_usd, 2)} USD`}
              </p>
              <p>{entry.notes}</p>
              {entry.receipt_reference ? <p>Reference: {entry.receipt_reference}</p> : null}
              <p>
                Next reminder: {entry.next_due_on ?? 'No date'}
                {entry.next_due_odometer_mi == null
                  ? ''
                  : ` / ${distance(entry.next_due_odometer_mi)}`}
              </p>
              <div className="rm-assistant-receipts">
                {data.receipts
                  .filter((r) => r.entry_id === entry.id)
                  .map((r) => (
                    <Button
                      variant="secondary"
                      key={r.id}
                      onClick={() => {
                        void downloadReceipt(r.id);
                      }}
                      iconLeft={<Download size={15} />}
                    >
                      {r.filename}
                    </Button>
                  ))}
              </div>
              <label className="rm-assistant-file-label">
                Attach a receipt <small>PDF, JPEG or PNG · up to 5 MiB · save entry first</small>
                <input
                  aria-label={`Attach receipt to ${entry.title}`}
                  type="file"
                  accept="application/pdf,image/jpeg,image/png"
                  disabled={dirty || busy}
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    e.target.value = '';
                    void addReceipt(entry.id, file);
                  }}
                />
              </label>
              <div className="rm-assistant-actions">
                <Button variant="ghost" onClick={() => setServiceEdit(copy(entry))}>
                  Edit
                </Button>
                {removeId === entry.id ? (
                  <>
                    <p>Saving this removal also deletes its attached receipts.</p>
                    <Button
                      variant="danger"
                      onClick={() => {
                        change((d) => {
                          d.maintenance = d.maintenance.filter((e) => e.id !== entry.id);
                        });
                        setRemoveId(null);
                      }}
                    >
                      Remove entry and receipts on save
                    </Button>
                    <Button variant="ghost" onClick={() => setRemoveId(null)}>
                      Cancel
                    </Button>
                  </>
                ) : (
                  <Button variant="ghost" onClick={() => setRemoveId(entry.id)}>
                    Remove
                  </Button>
                )}
              </div>
            </article>
          ))}
        </div>
      )}
      <Button
        variant="secondary"
        onClick={() =>
          downloadText(
            'riviamigo-service-notebook.json',
            JSON.stringify(draft.maintenance, null, 2),
            'application/json'
          )
        }
      >
        Export service notebook
      </Button>
    </>
  );
}
