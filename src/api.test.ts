import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, resetApiSession } from './api';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

afterEach(() => { vi.clearAllMocks(); resetApiSession(); });

describe('workspace access error boundary', () => {
  it('announces a current locked workspace error so the UI clears sensitive local state', async () => {
    const expired = vi.fn();
    window.addEventListener('workspace-access-expired', expired);
    invokeMock.mockRejectedValueOnce('Workspace is locked. Unlock it to continue.');
    await expect(api.listClients()).rejects.toContain('Workspace is locked');
    expect(expired).toHaveBeenCalledOnce();
    window.removeEventListener('workspace-access-expired', expired);
  });

  it('does not let a late rejected command from an old session clear a newly authenticated session', async () => {
    const expired = vi.fn();
    window.addEventListener('workspace-access-expired', expired);
    let rejectRequest!: (reason: unknown) => void;
    invokeMock.mockImplementationOnce(() => new Promise((_resolve, reject) => { rejectRequest = reject; }));
    const oldRequest = api.listClientNotes('old-client');
    const expectedRejection = expect(oldRequest).rejects.toContain('Workspace is locked');
    resetApiSession();
    invokeMock.mockResolvedValueOnce({ organizationId: 'new-workspace' });
    await api.unlockWorkspace('new-session-password');
    resetApiSession();
    rejectRequest('Workspace is locked. Unlock it to continue.');
    await expectedRejection;
    expect(expired).not.toHaveBeenCalled();
    window.removeEventListener('workspace-access-expired', expired);
  });
});
