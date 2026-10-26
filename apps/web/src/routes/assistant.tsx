import React from 'react';
import { createRoute } from '@tanstack/react-router';
import { rootRoute } from './__root';
import { ProtectedRoute } from '../components/layout/ProtectedRoute';
import { PersonalAssistantPage } from '../features/assistant/PersonalAssistantPage';
export const assistantRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/assistant',
  component: AssistantRoute,
});
function AssistantRoute() {
  return (
    <ProtectedRoute>
      <PersonalAssistantPage />
    </ProtectedRoute>
  );
}
