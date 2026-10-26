import { DepartureSection } from './DepartureSection';
import { MaintenanceSection } from './MaintenanceSection';
import { WeeklySection } from './WeeklySection';
import { VacationSection } from './VacationSection';
import { AlertsSection } from './AlertsSection';
import { TomorrowSection } from './TomorrowSection';
import React, { useEffect, useState } from 'react';
import {
  CalendarCheck2,
  Bell,
  Palmtree,
  FileText,
  Wrench,
  Thermometer,
  Save,
  Eye,
} from 'lucide-react';
import {
  useAuth,
  useMe,
  useResolvedVehicleSelection,
  usePersonalAssistant,
  useSavePersonalAssistant,
  usePreviewPersonalAssistant,
  useAcknowledgeAssistantAlert,
  useUploadAssistantReceipt,
  getAssistantReceipt,
} from '@riviamigo/hooks';
import type {
  AssistantDocument,
  AssistantEvaluation,
  AssistantTripPlan,
  AssistantMaintenanceEntry,
  AssistantPreferences,
} from '@riviamigo/types';
import { Badge, Button, PageLayout, SelectPicker } from '@riviamigo/ui/primitives';
import { getUnitPreferences } from '@riviamigo/ui/lib/utils';
import { AppLayout } from '../../components/layout/AppLayout';
import { NoVehicleState } from '../../components/layout/NoVehicleState';
import { PersonalR2Artwork } from '../../components/PersonalR2Artwork';
import { displayNumber as n, displayTime, localExampleDate } from './format';
import './style.css';
import type { AssistantViewContext } from './viewTypes';

const tabs = [
  { key: 'tomorrow', label: 'Tomorrow', icon: CalendarCheck2 },
  { key: 'alerts', label: 'Alerts', icon: Bell },
  { key: 'vacation', label: 'Vacation', icon: Palmtree },
  { key: 'weekly', label: 'Weekly', icon: FileText },
  { key: 'maintenance', label: 'Maintenance', icon: Wrench },
  { key: 'departure', label: 'Departure', icon: Thermometer },
] as const;
type Tab = (typeof tabs)[number]['key'];
import { errorText, copy } from './format';

export function PersonalAssistantPage() {
  const selection = useResolvedVehicleSelection();
  const me = useMe();
  const setActiveVehicleId = useAuth((state) => state.setActiveVehicleId);
  const vehicle = selection.vehicles.find((v) => v.id === selection.effectiveVehicleId);
  return (
    <AppLayout activeKey="assistant" presentation="r2">
      <PageLayout
        title="Your day, ready"
        subtitle="Plan ahead, notice changes, and keep your R2's story together."
        actions={
          selection.vehicles.length > 1 ? (
            <SelectPicker
              aria-label="Select vehicle"
              value={selection.effectiveVehicleId ?? ''}
              onChange={(id) => setActiveVehicleId(id)}
              options={selection.vehicles.map((v) => ({
                value: v.id,
                label: v.display_name || v.model,
              }))}
            />
          ) : undefined
        }
      >
        {!selection.vehicleSelectionReady ? (
          <p>Loading selected vehicle…</p>
        ) : !vehicle ? (
          <NoVehicleState
            title="Select a vehicle"
            description="Choose your demo R2 or connect your vehicle when it is delivered."
          />
        ) : (
          <AssistantContent
            key={`${me.data?.user_id}:${vehicle.id}`}
            vehicleId={vehicle.id}
            vehicleName={vehicle.display_name || vehicle.model}
          />
        )}
      </PageLayout>
    </AppLayout>
  );
}

function AssistantContent({ vehicleId, vehicleName }: { vehicleId: string; vehicleName: string }) {
  const query = usePersonalAssistant(vehicleId);
  const save = useSavePersonalAssistant(vehicleId);
  const preview = usePreviewPersonalAssistant(vehicleId);
  const acknowledge = useAcknowledgeAssistantAlert(vehicleId);
  const upload = useUploadAssistantReceipt(vehicleId);
  const [draft, setDraft] = useState<AssistantDocument | null>(null);
  const [revision, setRevision] = useState(0);
  const [dirty, setDirty] = useState(false);
  const [tab, setTab] = useState<Tab>('tomorrow');
  const [notice, setNotice] = useState('');
  const [previewResult, setPreviewResult] = useState<{
    evaluation: AssistantEvaluation;
    timezone: string;
  } | null>(null);
  const [planEdit, setPlanEdit] = useState<AssistantTripPlan | null>(null);
  const [serviceEdit, setServiceEdit] = useState<AssistantMaintenanceEntry | null>(null);
  const [removeId, setRemoveId] = useState<string | null>(null);
  useEffect(() => {
    if (query.data && !dirty) {
      setDraft(copy(query.data.document));
      setRevision(query.data.revision);
    }
  }, [query.data, dirty]);
  function change(fn: (doc: AssistantDocument) => void) {
    setDraft((old) => {
      if (!old) return old;
      const next = copy(old);
      fn(next);
      return next;
    });
    setDirty(true);
    setPreviewResult(null);
    setNotice('');
  }
  function preference<K extends keyof AssistantPreferences>(
    key: K,
    value: AssistantPreferences[K]
  ) {
    change((d) => {
      d.preferences[key] = value;
    });
  }
  async function persist(resetVacation = false) {
    if (!draft) return;
    try {
      const next = await save.mutateAsync({
        revision,
        document: draft,
        reset_vacation: resetVacation,
      });
      setDraft(copy(next.document));
      setRevision(next.revision);
      setDirty(false);
      setPreviewResult(null);
      setNotice('Saved on your NAS.');
    } catch (error) {
      setNotice(errorText(error));
    }
  }
  async function previewDraft() {
    if (!draft) return;
    try {
      const evaluation = await preview.mutateAsync(draft);
      setPreviewResult({ evaluation, timezone: draft.preferences.timezone });
      setNotice(
        'Preview only. Save changes to keep these settings and update your saved alert inbox.'
      );
    } catch (error) {
      setNotice(errorText(error));
    }
  }
  if (query.isPending || !draft)
    return (
      <section className="rm-assistant-page">
        <p role="status">Loading your saved plans…</p>
        {query.isError ? (
          <div role="alert">
            The assistant is unavailable. Your existing dashboards are still available.
            <Button
              variant="secondary"
              onClick={() => {
                void query.refetch();
              }}
            >
              Retry
            </Button>
          </div>
        ) : null}
      </section>
    );
  if (!query.data)
    return (
      <p role="alert">
        Could not load the assistant.
        <Button
          onClick={() => {
            void query.refetch();
          }}
        >
          Retry
        </Button>
      </p>
    );
  const data = query.data;
  const evaluation = previewResult?.evaluation ?? data.evaluation;
  const tz = previewResult?.timezone ?? data.document.preferences.timezone;
  const time = (value: string | null | undefined) => displayTime(value, tz);
  const busy = save.isPending || upload.isPending || preview.isPending;
  const unread = data.alerts.filter((a) => !a.acknowledged_at && !a.resolved_at).length;
  const prefs = draft.preferences;
  const metric = getUnitPreferences().distance_unit === 'kilometers';
  const fahrenheit = getUnitPreferences().temperature_unit === 'fahrenheit';
  const distance = (miles: number) =>
    `${n(metric ? miles * 1.609344 : miles, 1)} ${metric ? 'km' : 'mi'}`;
  function addExamples() {
    change((d) => {
      d.preferences.demo_scenario = 'ready';
      d.preferences.alerts_enabled = true;
      d.preferences.estimated_charger_kw = 7.2;
      if (!d.plans.length)
        d.plans.push({
          id: crypto.randomUUID(),
          title: 'Airport · example trip',
          departure_local: localExampleDate(d.preferences.timezone),
          distance_mi: 60,
          round_trip: true,
        });
      if (!d.maintenance.length)
        d.maintenance.push({
          id: crypto.randomUUID(),
          title: 'Tire rotation · example',
          performed_on: localExampleDate(d.preferences.timezone, -30).slice(0, 10),
          odometer_mi: 10_000,
          cost_usd: 75,
          receipt_reference: 'Demo record — no actual service performed',
          notes: 'An example of your own service reminder.',
          next_due_on: null,
          next_due_odometer_mi: 12_500,
        });
    });
  }
  async function addReceipt(entryId: string, file: File | undefined) {
    if (!file) return;
    if (dirty) {
      setNotice('Save the service entry before attaching its receipt.');
      return;
    }
    if (file.size > 5 * 1024 * 1024) {
      setNotice('Choose a PDF, JPEG or PNG no larger than 5 MiB.');
      return;
    }
    try {
      const result = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result).split(',')[1] ?? '');
        reader.onerror = () => reject(new Error('Could not read that receipt.'));
        reader.readAsDataURL(file);
      });
      await upload.mutateAsync({
        entryId,
        filename: file.name,
        mime_type: file.type,
        data_base64: result,
      });
      setNotice('Receipt saved privately on your NAS.');
    } catch (error) {
      setNotice(errorText(error));
    }
  }
  async function downloadReceipt(id: string) {
    try {
      const value = await getAssistantReceipt(vehicleId, id);
      const bytes = Uint8Array.from(atob(value.data_base64), (c) => c.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: value.mime_type }));
      const link = document.createElement('a');
      link.href = url;
      link.download = value.filename;
      link.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 30_000);
    } catch (error) {
      setNotice(errorText(error));
    }
  }
  const context: AssistantViewContext = {
    vehicleId,
    draft,
    data,
    evaluation,
    prefs,
    busy,
    dirty,
    metric,
    fahrenheit,
    time,
    distance,
    planEdit,
    setPlanEdit,
    serviceEdit,
    setServiceEdit,
    removeId,
    setRemoveId,
    change,
    preference,
    persist,
    addExamples,
    addReceipt,
    downloadReceipt,
    acknowledge,
    setNotice,
    previewResult,
    tz,
  };
  return (
    <section className="rm-assistant-page">
      {query.isError ? (
        <div role="alert" className="rm-assistant-notice">
          Could not refresh the assistant. The readings below are from its last successful check.{' '}
          <Button
            variant="secondary"
            onClick={() => {
              void query.refetch();
            }}
          >
            Retry refresh
          </Button>
        </div>
      ) : null}
      <header className="rm-assistant-hero">
        <div>
          <span className="rm-assistant-eyebrow">PERSONAL ASSISTANT · {vehicleName}</span>
          <h2>A little less to think about.</h2>
          <p>Useful plans and a quiet place for the things worth noticing.</p>
          <div className="rm-assistant-badges">
            <Badge variant={evaluation.is_demo ? 'warning' : 'default'}>
              {evaluation.is_demo ? 'DEMO · SAMPLE DATA' : 'Private vehicle data'}
            </Badge>
            <Badge variant="default">Dashboard alerts only</Badge>
            <Badge variant="default">Vehicle controls off</Badge>
          </div>
        </div>
        <PersonalR2Artwork className="rm-assistant-car" />
      </header>
      <div className="rm-assistant-toolbar">
        <p>
          {dirty ? 'Unsaved changes' : 'Saved plans stay on your NAS'} · {prefs.timezone}
        </p>
        <div>
          <Button
            variant="secondary"
            onClick={() => {
              void previewDraft();
            }}
            disabled={busy}
            loading={preview.isPending}
            iconLeft={<Eye size={16} />}
          >
            Preview changes
          </Button>
          <Button
            onClick={() => {
              void persist();
            }}
            disabled={busy || !dirty}
            loading={save.isPending}
            iconLeft={<Save size={16} />}
          >
            Save changes
          </Button>
        </div>
      </div>
      {notice ? (
        <p className="rm-assistant-notice" role="status">
          {notice}
          {save.isError ? (
            <Button
              variant="ghost"
              onClick={() => {
                setDirty(false);
                setPreviewResult(null);
                void query.refetch();
              }}
            >
              Reload saved version
            </Button>
          ) : null}
        </p>
      ) : null}
      {previewResult ? (
        <p className="rm-assistant-preview-note">
          UNSAVED PREVIEW · Your saved alerts and records have not changed.
        </p>
      ) : null}
      <div className="rm-assistant-tabs" role="tablist" aria-label="Personal assistant sections">
        {tabs.map(({ key, label, icon: Icon }) => (
          <Button
            key={key}
            role="tab"
            id={`assistant-tab-${key}`}
            aria-selected={tab === key}
            aria-controls={`assistant-panel-${key}`}
            variant={tab === key ? 'primary' : 'secondary'}
            onClick={() => setTab(key)}
          >
            <Icon size={17} />
            {label}
            {key === 'alerts' && unread ? (
              <span className="rm-assistant-count">{unread}</span>
            ) : null}
          </Button>
        ))}
      </div>
      <fieldset disabled={busy} className="rm-assistant-fieldset">
        <div role="tabpanel" id={`assistant-panel-${tab}`} aria-labelledby={`assistant-tab-${tab}`}>
          {tab === 'tomorrow' ? <TomorrowSection context={context} /> : null}
          {tab === 'alerts' ? <AlertsSection context={context} /> : null}
          {tab === 'vacation' ? <VacationSection context={context} /> : null}
          {tab === 'weekly' ? <WeeklySection context={context} /> : null}
          {tab === 'maintenance' ? <MaintenanceSection context={context} /> : null}
          {tab === 'departure' ? <DepartureSection context={context} /> : null}
        </div>
      </fieldset>
      <footer className="rm-assistant-footer">
        {evaluation.is_demo
          ? 'Demo readings and scenarios'
          : 'Reported readings with explicit planning assumptions'}{' '}
        · Last assessed {time(evaluation.generated_at)} · Assistant v0.3.1
      </footer>
    </section>
  );
}
