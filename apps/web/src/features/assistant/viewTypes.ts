import type React from 'react';
import type {
  AssistantDocument,
  AssistantEvaluation,
  AssistantPreferences,
  AssistantTripPlan,
  AssistantMaintenanceEntry,
} from '@riviamigo/types';
import type { useAcknowledgeAssistantAlert } from '@riviamigo/hooks';
export interface AssistantViewContext {
  vehicleId: string;
  draft: AssistantDocument;
  data: import('@riviamigo/types').PersonalAssistantState;
  evaluation: AssistantEvaluation;
  prefs: AssistantPreferences;
  busy: boolean;
  dirty: boolean;
  metric: boolean;
  fahrenheit: boolean;
  time: (value: string | null | undefined) => string;
  distance: (miles: number) => string;
  planEdit: AssistantTripPlan | null;
  setPlanEdit: React.Dispatch<React.SetStateAction<AssistantTripPlan | null>>;
  serviceEdit: AssistantMaintenanceEntry | null;
  setServiceEdit: React.Dispatch<React.SetStateAction<AssistantMaintenanceEntry | null>>;
  removeId: string | null;
  setRemoveId: React.Dispatch<React.SetStateAction<string | null>>;
  change: (fn: (doc: AssistantDocument) => void) => void;
  preference: <K extends keyof AssistantPreferences>(
    key: K,
    value: AssistantPreferences[K]
  ) => void;
  persist: (resetVacation?: boolean) => Promise<void>;
  addExamples: () => void;
  addReceipt: (entryId: string, file: File | undefined) => Promise<void>;
  downloadReceipt: (id: string) => Promise<void>;
  acknowledge: ReturnType<typeof useAcknowledgeAssistantAlert>;
  setNotice: React.Dispatch<React.SetStateAction<string>>;
  previewResult: { evaluation: AssistantEvaluation; timezone: string } | null;
  tz: string;
}
