import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  AssistantDocument,
  AssistantEvaluation,
  AssistantReceiptDownload,
  PersonalAssistantState,
  AssistantWeeklySummary,
} from '@riviamigo/types';
import { api } from './api';
import { useAuthReady } from './useAuthState';
import { useMe } from './useMe';

export const personalAssistantKey = (userId: string | undefined, vehicleId: string | null) =>
  ['personal-assistant', 'v031', userId, vehicleId] as const;
const endpoint = (vehicleId: string) => `/v1/vehicles/${encodeURIComponent(vehicleId)}/assistant`;
export function usePersonalAssistant(vehicleId: string | null) {
  const ready = useAuthReady();
  const me = useMe();
  return useQuery({
    queryKey: personalAssistantKey(me.data?.user_id, vehicleId),
    queryFn: () => api.apiFetch<PersonalAssistantState>('GET', endpoint(vehicleId!)),
    enabled: ready && !!vehicleId && !!me.data?.user_id,
    staleTime: 30_000,
    refetchInterval: 60_000,
    refetchOnWindowFocus: true,
    meta: { persist: false },
  });
}
export function useSavePersonalAssistant(vehicleId: string) {
  const client = useQueryClient();
  const me = useMe();
  return useMutation({
    mutationFn: (input: {
      revision: number;
      document: AssistantDocument;
      reset_vacation?: boolean;
    }) => api.apiFetch<PersonalAssistantState>('PUT', endpoint(vehicleId), input),
    onSuccess: (value) => {
      client.setQueryData(personalAssistantKey(me.data?.user_id, vehicleId), value);
      void client.invalidateQueries({
        queryKey: personalAssistantWeeklyKey(me.data?.user_id, vehicleId),
      });
    },
  });
}
export function usePreviewPersonalAssistant(vehicleId: string) {
  return useMutation({
    mutationFn: (document: AssistantDocument) =>
      api.apiFetch<AssistantEvaluation>('POST', `${endpoint(vehicleId)}/preview`, { document }),
  });
}
export function useAcknowledgeAssistantAlert(vehicleId: string) {
  const client = useQueryClient();
  const me = useMe();
  return useMutation({
    mutationFn: (id: string) =>
      api.apiFetch('POST', `${endpoint(vehicleId)}/alerts/${encodeURIComponent(id)}/acknowledge`),
    onSuccess: () =>
      client.invalidateQueries({ queryKey: personalAssistantKey(me.data?.user_id, vehicleId) }),
  });
}
export function useUploadAssistantReceipt(vehicleId: string) {
  const client = useQueryClient();
  const me = useMe();
  return useMutation({
    mutationFn: (input: {
      entryId: string;
      filename: string;
      mime_type: string;
      data_base64: string;
    }) =>
      api.apiFetch(
        'POST',
        `${endpoint(vehicleId)}/maintenance/${encodeURIComponent(input.entryId)}/receipts`,
        { filename: input.filename, mime_type: input.mime_type, data_base64: input.data_base64 }
      ),
    onSuccess: () =>
      client.invalidateQueries({ queryKey: personalAssistantKey(me.data?.user_id, vehicleId) }),
  });
}
export function getAssistantReceipt(vehicleId: string, receiptId: string) {
  return api.apiFetch<AssistantReceiptDownload>(
    'GET',
    `${endpoint(vehicleId)}/receipts/${encodeURIComponent(receiptId)}`
  );
}

export const personalAssistantWeeklyKey = (userId: string | undefined, vehicleId: string | null) =>
  ['personal-assistant-weekly', 'v031', userId, vehicleId] as const;
export function usePersonalAssistantWeekly(vehicleId: string | null) {
  const ready = useAuthReady();
  const me = useMe();
  return useQuery({
    queryKey: personalAssistantWeeklyKey(me.data?.user_id, vehicleId),
    queryFn: () => api.apiFetch<AssistantWeeklySummary>('GET', `${endpoint(vehicleId!)}/weekly`),
    enabled: ready && !!vehicleId && !!me.data?.user_id,
    staleTime: 300_000,
    refetchInterval: 300_000,
    refetchOnWindowFocus: true,
    meta: { persist: false },
  });
}
