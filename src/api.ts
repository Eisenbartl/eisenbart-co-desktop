import { invoke } from '@tauri-apps/api/core';
import type { BudgetInput, ClientBudget, Client, ClientInput, ClientNote, SetupInput, WorkspaceInfo } from './types';

export function desktopAvailable(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

let sessionEpoch = 0;
export function resetApiSession(): void {
  sessionEpoch += 1;
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const requestEpoch = sessionEpoch;
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    const text = error instanceof Error ? error.message : String(error);
    if (requestEpoch === sessionEpoch && /workspace is locked/i.test(text)) window.dispatchEvent(new Event('workspace-access-expired'));
    throw error;
  }
}

export const api = {
  getClientBudget: (clientId: string) => command<ClientBudget>('get_client_budget', { clientId }),
  saveClientBudget: (clientId: string, input: BudgetInput) => command<ClientBudget>('save_client_budget', { clientId, input }),
  setupStatus: () => command<{ configured: boolean }>('setup_status'),
  setupWorkspace: (input: SetupInput) => command<WorkspaceInfo>('setup_workspace', { input }),
  unlockWorkspace: (password: string) => command<WorkspaceInfo>('unlock_workspace', { password }),
  lockWorkspace: () => command<void>('lock_workspace'),
  workspaceInfo: () => command<WorkspaceInfo>('workspace_info'),
  listClients: () => command<Client[]>('list_clients'),
  getClient: (id: string) => command<Client>('get_client', { id }),
  saveClient: (input: ClientInput) => command<Client>('save_client', { input }),
  addClientNote: (clientId: string, body: string) => command<ClientNote>('add_client_note', { clientId, body }),
  listClientNotes: (clientId: string) => command<ClientNote[]>('list_client_notes', { clientId }),
};
