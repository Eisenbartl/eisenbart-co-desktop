import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import BudgetEditor, { usd } from './BudgetEditor';
import type { Client, ClientBudget } from './types';

const mocks = vi.hoisted(() => ({ getClientBudget: vi.fn(), saveClientBudget: vi.fn() }));
vi.mock('./api', () => ({ api: mocks }));
const client: Client = { id: 'client-a', organizationId: 'org-a', firstName: 'Alex', lastName: 'Fictional', email: '', phone: '', householdSize: 1, status: 'Planning', goals: '', createdAt: '', updatedAt: '' };
const empty: ClientBudget = { clientId: client.id, revision: 0, updatedAt: null, lines: [], summary: {
  monthlyIncome: '0.00', necessaryExpenses: '0.00', discretionaryExpenses: '0.00', totalSpending: '0.00',
  surplusBeforeAllocations: '0.00', proposedSavings: '0.00', availableForDebt: '0.00', plannedDebtPayments: '0.00', unallocated: '0.00',
  monthlyAmounts: [], missingCount: 0, estimatedCount: 0, warnings: ['No income amount entered; this plan is incomplete.'],
} };
const saved: ClientBudget = { ...empty, revision: 1, updatedAt: '2026-10-13T10:00:00Z', lines: [{ label: 'Fictional wages', kind: 'income', amount: '1500.00', frequency: 'biweekly', estimated: false }], summary: { ...empty.summary, monthlyIncome: '3250.00', surplusBeforeAllocations: '3250.00', availableForDebt: '3250.00', unallocated: '3250.00', monthlyAmounts: ['3250.00'], warnings: ['Essential expenses are still missing.'] } };
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(accept => { resolve = accept; }); return { promise, resolve }; }
function open() { return render(<BudgetEditor client={client} onBusy={vi.fn()} onDirty={vi.fn()} otherSaving={false} />); }
beforeEach(() => { vi.clearAllMocks(); mocks.getClientBudget.mockResolvedValue(empty); mocks.saveClientBudget.mockResolvedValue(saved); });

describe('client budget editor', () => {
  it('sends exact strings and revisions, uses native totals, and hides stale totals while editing', async () => {
    open(); await screen.findByText('No saved budget yet');
    fireEvent.click(screen.getByRole('button', { name: 'Add income' }));
    fireEvent.change(screen.getByLabelText('Category or source'), { target: { value: 'Fictional wages' } });
    fireEvent.change(screen.getByLabelText('Amount (USD)'), { target: { value: '1500' } });
    fireEvent.change(screen.getByLabelText('Frequency'), { target: { value: 'biweekly' } });
    expect(screen.queryByRole('heading', { name: 'Monthly plan summary' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Save budget and calculate' }));
    await screen.findByText('Budget saved and calculated.');
    expect(mocks.saveClientBudget).toHaveBeenCalledWith(client.id, { expectedRevision: 0, lines: [{ label: 'Fictional wages', amount: '1500', kind: 'income', frequency: 'biweekly', estimated: false }] });
    expect(screen.getByLabelText('Amount (USD)')).toHaveValue('1500.00');
    expect(screen.getByText('Monthly equivalent:')).toHaveTextContent('$3,250.00');
    expect(screen.getByText('Essential expenses are still missing.')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Amount (USD)'), { target: { value: '1200.123' } });
    mocks.saveClientBudget.mockRejectedValueOnce('At most two decimal places are allowed.');
    fireEvent.click(screen.getByRole('button', { name: 'Save budget and calculate' }));
    await screen.findByRole('alert');
    expect(screen.getByLabelText('Amount (USD)')).toHaveValue('1200.123');
    expect(mocks.saveClientBudget.mock.calls[1][1].expectedRevision).toBe(1);
    expect(screen.queryByRole('heading', { name: 'Monthly plan summary' })).not.toBeInTheDocument();
  });
  it('keeps unknown amounts separate from zero and marks irregular monthly estimates', async () => {
    open(); await screen.findByText('No saved budget yet');
    fireEvent.click(screen.getByRole('button', { name: 'Add income' }));
    fireEvent.change(screen.getByLabelText('Category or source'), { target: { value: 'Irregular wages' } });
    fireEvent.change(screen.getByLabelText('Frequency'), { target: { value: 'irregular' } });
    expect(screen.getByLabelText('Estimated amount')).toBeChecked();
    expect(screen.getByLabelText('Estimated amount')).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Add expense' }));
    fireEvent.change(screen.getAllByLabelText('Category or source')[1], { target: { value: 'Known zero cost' } });
    fireEvent.change(screen.getAllByLabelText('Amount (USD)')[1], { target: { value: '0' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save budget and calculate' }));
    await waitFor(() => expect(mocks.saveClientBudget).toHaveBeenCalled());
    expect(mocks.saveClientBudget.mock.calls[0][1].lines).toEqual([
      { label: 'Irregular wages', kind: 'income', amount: null, frequency: 'irregular', estimated: true },
      { label: 'Known zero cost', kind: 'necessary', amount: '0', frequency: 'monthly', estimated: false },
    ]);
  });
  it('does not replace a failed load with an empty editable budget and permits a retry', async () => {
    mocks.getClientBudget.mockRejectedValueOnce('The workspace could not be read.');
    open(); await screen.findByRole('alert');
    expect(screen.queryByRole('button', { name: 'Save budget and calculate' })).not.toBeInTheDocument();
    expect(mocks.saveClientBudget).not.toHaveBeenCalled();
    mocks.getClientBudget.mockResolvedValueOnce(saved);
    fireEvent.click(screen.getByRole('button', { name: 'Retry loading budget' }));
    await screen.findByDisplayValue('Fictional wages');
    expect(mocks.getClientBudget).toHaveBeenCalledTimes(2);
  });
  it('ignores an old client load and a pending save after the private editor unmounts', async () => {
    const firstLoad = deferred<ClientBudget>();
    mocks.getClientBudget.mockReturnValueOnce(firstLoad.promise);
    const first = open(); first.unmount();
    const secondClient = { ...client, id: 'client-b', firstName: 'Robin' };
    mocks.getClientBudget.mockResolvedValueOnce({ ...empty, clientId: secondClient.id });
    const view = render(<BudgetEditor client={secondClient} onBusy={vi.fn()} onDirty={vi.fn()} otherSaving={false} />);
    await screen.findByText('No saved budget yet');
    await act(async () => firstLoad.resolve(saved));
    expect(screen.queryByDisplayValue('Fictional wages')).not.toBeInTheDocument();
    const pendingSave = deferred<ClientBudget>(); mocks.saveClientBudget.mockReturnValueOnce(pendingSave.promise);
    fireEvent.click(screen.getByRole('button', { name: 'Add income' }));
    fireEvent.change(screen.getByLabelText('Category or source'), { target: { value: 'Robin private estimate' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save budget and calculate' }));
    expect(screen.getByRole('button', { name: 'Calculating…' })).toBeDisabled();
    view.unmount(); await act(async () => pendingSave.resolve({ ...saved, clientId: secondClient.id }));
    expect(screen.queryByText('Budget saved and calculated.')).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent('Robin private estimate');
  });
  it('formats native signed decimal strings without losing precision', () => {
    expect(usd('433333333333.00')).toBe('$433,333,333,333.00');
    expect(usd('-0.01')).toBe('-$0.01');
  });
});
