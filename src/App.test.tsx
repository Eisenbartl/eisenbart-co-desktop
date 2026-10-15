import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import type { Client, ClientNote, WorkspaceInfo } from './types';

const mocks = vi.hoisted(() => ({
  desktop: true,
  lockHandler: undefined as (() => void) | undefined,
  setupStatus: vi.fn(), setupWorkspace: vi.fn(), unlockWorkspace: vi.fn(),
  lockWorkspace: vi.fn(), workspaceInfo: vi.fn(), listClients: vi.fn(),
  getClient: vi.fn(), saveClient: vi.fn(), addClientNote: vi.fn(), listClientNotes: vi.fn(),
  chooseBackupFolder:vi.fn(), chooseRestoreFile:vi.fn(), createWorkspaceBackup:vi.fn(), prepareWorkspaceRestore:vi.fn(), confirmWorkspaceRestore:vi.fn(), cancelWorkspaceRestore:vi.fn(),
  chooseClientCsv:vi.fn(),inspectClientCsv:vi.fn(),cancelClientCsv:vi.fn(),getClientImports:vi.fn(),reviewRecurringPayment:vi.fn(),reviewStatementBalances:vi.fn(),stageClientCsv:vi.fn(),reviewTransaction:vi.fn(),bulkApproveTransactions:vi.fn(),pairTransferTransactions:vi.fn(),
  resetApiSession: vi.fn(), getClientBudget: vi.fn(), saveClientBudget: vi.fn(), getClientDebtPlan: vi.fn(), saveClientDebtPlan: vi.fn(), getClientCashflowPlan: vi.fn(), previewClientCashflowPlan: vi.fn(), saveClientCashflowPlan: vi.fn(),
  listClientScenarios:vi.fn(),getAssessmentPlanning:vi.fn(),compareFinancialScenarios:vi.fn(),
  getClientSavingsPlan: vi.fn(), saveClientSavingsPlan: vi.fn(),
  getClientCare:vi.fn(),saveClientCare:vi.fn(),
}));

vi.mock('./api', () => ({
  desktopAvailable: () => mocks.desktop,
  resetApiSession: mocks.resetApiSession,
  api: mocks,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_event: string, handler: () => void) => {
    mocks.lockHandler = handler;
    return () => { if (mocks.lockHandler === handler) mocks.lockHandler = undefined; };
  }),
}));

const workspace: WorkspaceInfo = { organizationId: 'org-1', organizationName: 'Fictional Practice', counselorId: 'counselor-1', counselorName: 'Morgan Example' };
const alice: Client = { id: 'client-a', organizationId: 'org-1', firstName: 'Alice', lastName: 'Able', email: 'alice@example.test', phone: '', householdSize: 2, status: 'New Intake', goals: 'Fictional Alice private goal', createdAt: '2026-10-01T10:00:00Z', updatedAt: '2026-10-01T10:00:00Z' };
const bob: Client = { ...alice, id: 'client-b', firstName: 'Bob', lastName: 'Baker', email: 'bob@example.test', goals: 'Fictional Bob goal', status: 'Planning' };
const aliceNote: ClientNote = { id: 'note-a', clientId: alice.id, counselorId: workspace.counselorId, counselorName: workspace.counselorName, body: 'Private Alice session note', createdAt: '2026-10-01T12:00:00Z' };

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(accept => { resolve = accept; });
  return { promise, resolve };
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.getClientCare.mockImplementation(async(clientId:string)=>({clientId,revision:0,updatedAt:null,reviewDate:'2026-10-15',overdue:0,dueToday:0,data:{intake:{address:'',household:'',employment:'',employer:'',occupation:'',incomeNotes:''},followUps:[]}}));
  mocks.listClientScenarios.mockImplementation(async(clientId:string)=>({clientId,records:[]}));
  mocks.getClientImports.mockImplementation(async (clientId:string)=>({clientId,revision:0,accounts:[],documents:[],records:[],history:[],merchantGroups:[],recurringHistory:[],summary:{pending:0,needsClarification:0,approved:0,rejected:0,income:'0.00',spending:'0.00',refunds:'0.00',netSpending:'0.00',netCash:'0.00',accountNet:[],warnings:[]}}));
  mocks.cancelClientCsv.mockResolvedValue(undefined);
  mocks.cancelWorkspaceRestore.mockResolvedValue(undefined);
  mocks.prepareWorkspaceRestore.mockResolvedValue({token:'synthetic-review',organizationName:'Recovered firm',counselorName:'Recovered counselor',clientCount:2,createdAt:'2026-10-14T00:00:00Z',sourceSchema:7,currentWorkspacePresent:true});
  mocks.confirmWorkspaceRestore.mockResolvedValue({previousWorkspacePreserved:true,durabilityWarning:false});
  mocks.desktop = true;
  mocks.lockHandler = undefined;
  mocks.setupStatus.mockResolvedValue({ configured: true });
  mocks.setupWorkspace.mockResolvedValue(workspace);
  mocks.unlockWorkspace.mockResolvedValue(workspace);
  mocks.lockWorkspace.mockResolvedValue(undefined);
  mocks.workspaceInfo.mockResolvedValue(workspace);
  mocks.listClients.mockResolvedValue([alice, bob]);
  mocks.getClient.mockImplementation(async (id: string) => id === alice.id ? alice : bob);
  mocks.listClientNotes.mockResolvedValue([]);
  mocks.getClientCashflowPlan.mockImplementation(async (clientId:string)=>({clientId,revision:0,updatedAt:null,data:null,summary:null,context:{budgetRevision:0,debtRevision:0,savingsRevision:0,monthlySurplus:null,budgetDebt:null,budgetSavings:null,debtMonthlyPayment:null,savingsMonthlyContribution:null}}));
  mocks.getClientSavingsPlan.mockImplementation(async (clientId:string)=>({clientId,revision:0,updatedAt:null,data:null,summary:null,context:{budgetRevision:0,debtRevision:0,necessaryExpenses:null,monthlySurplus:null,budgetSavings:null,budgetDebtPayments:null,debtStartMonth:null,debtMonthlyPayment:null,debtChanges:[],debtExtras:[],warnings:[]}}));
  mocks.getClientDebtPlan.mockImplementation(async (clientId: string) => ({ clientId, revision: 0, updatedAt: null, data: null, comparison: null, budgetContext: {revision:0, monthlyCapacity:'0.00', plannedDebtPayments:'0.00', warnings:[]} }));
  mocks.getClientBudget.mockImplementation(async (clientId: string) => ({ clientId, revision: 0, updatedAt: null, lines: [], summary: { monthlyIncome: '0.00', necessaryExpenses: '0.00', discretionaryExpenses: '0.00', totalSpending: '0.00', surplusBeforeAllocations: '0.00', proposedSavings: '0.00', availableForDebt: '0.00', plannedDebtPayments: '0.00', unallocated: '0.00', monthlyAmounts: [], missingCount: 0, estimatedCount: 0, warnings: ['No income amount entered; this plan is incomplete.'] } }));
  mocks.saveClient.mockImplementation(async input => ({ ...alice, ...input, id: input.id ?? 'client-new' }));
  mocks.addClientNote.mockImplementation(async (clientId: string, body: string) => ({ ...aliceNote, clientId, body }));
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});

async function unlock() {
  await screen.findByRole('heading', { name: 'Welcome back' });
  fireEvent.change(screen.getByLabelText('Workspace password'), { target: { value: 'fictional-password-123' } });
  fireEvent.click(screen.getByRole('button', { name: 'Unlock workspace' }));
  await screen.findByRole('heading', { name: 'A clear view of your practice.' });
  await waitFor(() => expect(mocks.listClients).toHaveBeenCalled());
}

async function selectClient(name: string) {
  fireEvent.click(screen.getByRole('button', { name: /^Clients/ }));
  fireEvent.click(screen.getByText(name).closest('button')!);
  await screen.findByRole('heading', { name });
}

describe('desktop workspace', () => {
  it('guards an intake draft when changing clients and clears private employment details on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');
    await screen.findByText('No saved budget yet');
    fireEvent.click(screen.getByText('Intake and follow-ups'));
    await screen.findByLabelText('Employer');
    await waitFor(()=>expect(screen.getByLabelText('Employer')).toBeEnabled());
    fireEvent.change(screen.getByLabelText('Employer'),{target:{value:'Alice private fictional employer'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    expect(window.confirm).toHaveBeenCalled();
    expect(screen.getByLabelText('Employer')).toHaveValue('Alice private fictional employer');
    await act(async()=>mocks.lockHandler?.());
    expect(document.documentElement.outerHTML).not.toContain('Alice private fictional employer');
    expect(screen.queryByText('Intake and follow-ups')).not.toBeInTheDocument();
    expect(mocks.saveClientCare).not.toHaveBeenCalled();
  });
  it('blocks writes during scenario baseline loading and clears temporary proposals and late replies on lock',async()=>{
    const pending=deferred<unknown>();mocks.getAssessmentPlanning.mockReturnValueOnce(pending.promise);
    render(<App/>);await unlock();await selectClient('Alice Able');await screen.findByText('No saved budget yet');await screen.findByText('No imported transactions yet');
    fireEvent.click(screen.getByText('Compare combined financial scenarios'));
    fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'Add income'})).toBeDisabled());
    expect(screen.getByRole('button',{name:'Save changes'})).toBeDisabled();
    await act(async()=>mocks.lockHandler?.());
    await act(async()=>pending.resolve({clientId:'client-a',privateAssumption:'Alice private scenario'}));
    expect(document.documentElement.outerHTML).not.toContain('Alice private scenario');
    expect(screen.queryByText('Compare combined financial scenarios')).not.toBeInTheDocument();
    expect(mocks.compareFinancialScenarios).not.toHaveBeenCalled();
  });

  it('guards unsaved CSV selection and clears private source paths on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');await screen.findByText('No imported transactions yet');
    fireEvent.change(screen.getByLabelText('Local CSV file'),{target:{value:'D:\\Synthetic\\alice-private.csv'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByText('Bob Baker').closest('button')!);expect(window.confirm).toHaveBeenCalled();expect(screen.getByLabelText('Local CSV file')).toHaveValue('D:\\Synthetic\\alice-private.csv');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('alice-private.csv');expect(mocks.stageClientCsv).not.toHaveBeenCalled();
  });

  it('permits verified recovery when damaged workspace setup fails and returns to a clean locked screen',async()=>{
    mocks.setupStatus.mockRejectedValueOnce('Workspace files are incomplete.');render(<App/>);await screen.findByText('Unable to open the workspace');
    fireEvent.click(screen.getByText('Restore an encrypted backup'));
    fireEvent.change(screen.getByLabelText('Encrypted backup file'),{target:{value:'D:\\Backups\\synthetic.ebcbackup'}});
    fireEvent.change(screen.getByLabelText('Password when backup was created'),{target:{value:'Synthetic-password-42'}});
    fireEvent.click(screen.getByRole('button',{name:'Verify backup for recovery'}));await screen.findByText('Recovered firm');
    fireEvent.change(screen.getByLabelText('Type RESTORE to confirm'),{target:{value:'RESTORE'}});
    fireEvent.click(screen.getByRole('button',{name:'Restore verified workspace'}));await screen.findByRole('heading',{name:'Welcome back'});
    expect(screen.getByRole('status')).toHaveTextContent('Workspace restored.');expect(screen.getByLabelText('Workspace password')).toHaveValue('');
    expect(screen.queryByText('Recovered firm')).not.toBeInTheDocument();expect(mocks.listClients).not.toHaveBeenCalled();
  });

  it('guards cash-flow edits and removes private account data on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');await screen.findByText('No saved cash-flow plan yet');
    fireEvent.click(screen.getByRole('button',{name:'Add cash-flow account'}));
    fireEvent.change(screen.getByLabelText('Account name'),{target:{value:'Alice private checking'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    expect(window.confirm).toHaveBeenCalled();expect(screen.getByLabelText('Account name')).toHaveValue('Alice private checking');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Alice private checking');expect(mocks.saveClientCashflowPlan).not.toHaveBeenCalled();
  });

  it('guards unsaved savings edits and clears private reserves on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');await screen.findByText('No saved savings plan yet');
    fireEvent.change(screen.getByLabelText('Funding priorities and notes'),{target:{value:'Alice private savings priority'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    expect(window.confirm).toHaveBeenCalled();expect(screen.getByLabelText('Funding priorities and notes')).toHaveValue('Alice private savings priority');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Alice private savings priority');expect(mocks.saveClientSavingsPlan).not.toHaveBeenCalled();
  });

  it('guards unsaved debt edits and clears them when the workspace locks', async () => {
    render(<App />); await unlock(); await selectClient('Alice Able');
    await screen.findByText('No saved debt plan yet');
    fireEvent.click(screen.getByRole('button', {name:'Add debt'}));
    fireEvent.change(screen.getByLabelText('Debt name'), {target:{value:'Private Alice loan'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    expect(window.confirm).toHaveBeenCalled();
    expect(screen.getByLabelText('Debt name')).toHaveValue('Private Alice loan');
    await act(async () => mocks.lockHandler?.());
    expect(document.documentElement.outerHTML).not.toContain('Private Alice loan');
    expect(mocks.saveClientDebtPlan).not.toHaveBeenCalled();
  });

  it('guards unsaved budget edits when changing clients and clears them on a lock event', async () => {
    render(<App />); await unlock(); await selectClient('Alice Able');
    await screen.findByText('No saved budget yet');
    fireEvent.click(screen.getByRole('button', { name: 'Add income' }));
    fireEvent.change(screen.getByLabelText('Category or source'), { target: { value: 'Alice private wage estimate' } });
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    expect(window.confirm).toHaveBeenCalled();
    expect(screen.getByRole('heading', { name: 'Alice Able' })).toBeInTheDocument();
    expect(screen.getByLabelText('Category or source')).toHaveValue('Alice private wage estimate');
    await act(async () => mocks.lockHandler?.());
    expect(screen.queryByLabelText('Category or source')).not.toBeInTheDocument();
    expect(document.documentElement.outerHTML).not.toContain('Alice private wage estimate');
    expect(mocks.saveClientBudget).not.toHaveBeenCalled();
  });
  it('does not display a delayed budget response in another client record', async () => {
    const delayed = deferred<Awaited<ReturnType<typeof mocks.getClientBudget>>>();
    mocks.getClientBudget.mockImplementationOnce(() => delayed.promise);
    render(<App />); await unlock(); await selectClient('Alice Able');
    fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    await screen.findByRole('heading', { name: 'Bob Baker' });
    await screen.findByText('No saved budget yet');
    await act(async () => delayed.resolve({ clientId: alice.id, lines: [{ label: 'Alice hidden financial detail' }] }));
    expect(screen.queryByDisplayValue('Alice hidden financial detail')).not.toBeInTheDocument();
    expect(document.documentElement.outerHTML).not.toContain('Alice hidden financial detail');
  });

  it('requires the desktop bridge and does not create fake browser data', () => {
    mocks.desktop = false;
    render(<App />);
    expect(screen.getByRole('heading', { name: 'Open the desktop application' })).toBeInTheDocument();
    expect(mocks.setupStatus).not.toHaveBeenCalled();
  });

  it('validates first-run names, password length and confirmation before creating a workspace', async () => {
    mocks.setupStatus.mockResolvedValue({ configured: false });
    render(<App />);
    await screen.findByRole('heading', { name: 'Set up your workspace' });
    expect(screen.getByLabelText('Organization name')).toHaveValue('Eisenbart & Co.');
    fireEvent.click(screen.getByRole('button', { name: 'Create workspace' }));
    expect(screen.getByRole('alert')).toHaveTextContent('organization and counselor names');
    fireEvent.change(screen.getByLabelText('Counselor name'), { target: { value: 'Morgan Example' } });
    fireEvent.change(screen.getByLabelText('Workspace password'), { target: { value: 'short' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create workspace' }));
    expect(screen.getByRole('alert')).toHaveTextContent('at least 12 characters');
    fireEvent.change(screen.getByLabelText('Workspace password'), { target: { value: 'fictional-password-123' } });
    fireEvent.change(screen.getByLabelText('Confirm password'), { target: { value: 'different-password-123' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create workspace' }));
    expect(screen.getByRole('alert')).toHaveTextContent('passwords do not match');
    expect(mocks.setupWorkspace).not.toHaveBeenCalled();
    fireEvent.change(screen.getByLabelText('Confirm password'), { target: { value: 'fictional-password-123' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create workspace' }));
    await screen.findByRole('heading', { name: 'A clear view of your practice.' });
    expect(mocks.setupWorkspace).toHaveBeenCalledWith({ organizationName: 'Eisenbart & Co.', counselorName: 'Morgan Example', password: 'fictional-password-123' });
  });

  it('shows an incorrect-password error and clears the submitted password', async () => {
    mocks.unlockWorkspace.mockRejectedValue('Incorrect workspace password.');
    render(<App />);
    await screen.findByRole('heading', { name: 'Welcome back' });
    fireEvent.change(screen.getByLabelText('Workspace password'), { target: { value: 'wrong-password-123' } });
    fireEvent.click(screen.getByRole('button', { name: 'Unlock workspace' }));
    await screen.findByRole('alert');
    expect(screen.getByRole('alert')).toHaveTextContent('Incorrect workspace password');
    expect(screen.getByLabelText('Workspace password')).toHaveValue('');
    expect(mocks.listClients).not.toHaveBeenCalled();
    expect(screen.queryByText(alice.goals)).not.toBeInTheDocument();
  });

  it('creates a real client through the API and rejects invalid intake fields first', async () => {
    mocks.listClients.mockResolvedValue([]);
    render(<App />);
    await unlock();
    fireEvent.click(screen.getByRole('button', { name: 'Add client' }));
    fireEvent.click(screen.getByRole('button', { name: 'Create client' }));
    expect(screen.getByRole('alert')).toHaveTextContent('first and last name');
    fireEvent.change(screen.getByLabelText(/First name/), { target: { value: 'Fictional' } });
    fireEvent.change(screen.getByLabelText(/Last name/), { target: { value: 'Client' } });
    fireEvent.change(screen.getByLabelText(/Email address/), { target: { value: 'invalid' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create client' }));
    expect(screen.getByRole('alert')).toHaveTextContent('valid email');
    expect(mocks.saveClient).not.toHaveBeenCalled();
    fireEvent.change(screen.getByLabelText(/Email address/), { target: { value: 'client@example.test' } });
    fireEvent.change(screen.getByLabelText('Counseling goals'), { target: { value: 'Build a fictional savings plan' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create client' }));
    await screen.findByRole('heading', { name: 'Fictional Client' });
    expect(mocks.saveClient).toHaveBeenCalledWith(expect.objectContaining({ firstName: 'Fictional', lastName: 'Client', email: 'client@example.test', status: 'New Intake', householdSize: 1, goals: 'Build a fictional savings plan' }));
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeInTheDocument();
  });

  it('updates the selected client identity and counseling status without changing client ownership', async () => {
    render(<App />);
    await unlock();
    await selectClient('Alice Able');
    fireEvent.change(screen.getByLabelText(/Last name/), { target: { value: 'Updated' } });
    fireEvent.change(screen.getByLabelText('Counseling status'), { target: { value: 'Follow-Up' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await screen.findByRole('heading', { name: 'Alice Updated' });
    expect(mocks.saveClient).toHaveBeenCalledWith(expect.objectContaining({ id: alice.id, lastName: 'Updated', status: 'Follow-Up' }));
    expect(mocks.saveClient.mock.calls[0][0]).not.toHaveProperty('organizationId');
    expect(screen.getByText('Client saved')).toHaveAttribute('role', 'status');
  });

  it('does not show a late note response from the previous client', async () => {
    const previousNotes = deferred<ClientNote[]>();
    mocks.listClientNotes.mockImplementation((id: string) => id === alice.id ? previousNotes.promise : Promise.resolve([]));
    render(<App />);
    await unlock();
    await selectClient('Alice Able');
    fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    await screen.findByRole('heading', { name: 'Bob Baker' });
    await waitFor(() => expect(screen.getByLabelText('New session note')).toBeEnabled());
    await act(async () => { previousNotes.resolve([aliceNote]); });
    expect(screen.queryByText(aliceNote.body)).not.toBeInTheDocument();
    expect(screen.getByLabelText('New session note')).toHaveValue('');
    expect(screen.getByLabelText('Counseling goals')).toHaveValue(bob.goals);
  });

  it('clears an unsaved note when switching clients and saves the new note to the current client only', async () => {
    render(<App />);
    await unlock();
    await selectClient('Alice Able');
    await waitFor(() => expect(screen.getByLabelText('New session note')).toBeEnabled());
    fireEvent.change(screen.getByLabelText('New session note'), { target: { value: 'Unsaved private Alice draft' } });
    await waitFor(() => expect(window.confirm).not.toHaveBeenCalled());
    fireEvent.click(screen.getByText('Bob Baker').closest('button')!);
    await screen.findByRole('heading', { name: 'Bob Baker' });
    await waitFor(() => expect(screen.getByLabelText('New session note')).toBeEnabled());
    expect(window.confirm).toHaveBeenCalled();
    expect(screen.getByLabelText('New session note')).toHaveValue('');
    expect(screen.queryByText('Unsaved private Alice draft')).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('New session note'), { target: { value: 'Fictional Bob note' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save session note' }));
    await screen.findByText('Fictional Bob note');
    expect(mocks.addClientNote).toHaveBeenCalledWith(bob.id, 'Fictional Bob note');
    expect(screen.getByLabelText('New session note')).toHaveValue('');
  });

  it('clears client details, notes and drafts when the native idle lock fires, then starts a clean unlocked session', async () => {
    mocks.listClientNotes.mockResolvedValue([aliceNote]);
    render(<App />);
    await unlock();
    await selectClient('Alice Able');
    await screen.findByText(aliceNote.body);
    fireEvent.change(screen.getByLabelText('New session note'), { target: { value: 'Sensitive unsaved draft' } });
    await act(async () => { mocks.lockHandler?.(); });
    await screen.findByRole('heading', { name: 'Welcome back' });
    expect(screen.queryByText(aliceNote.body)).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Counseling goals')).not.toBeInTheDocument();
    expect(screen.queryByText('Sensitive unsaved draft')).not.toBeInTheDocument();
    expect(mocks.resetApiSession).toHaveBeenCalled();
    await unlock();
    expect(screen.queryByLabelText('New session note')).not.toBeInTheDocument();
    expect(screen.queryByText(aliceNote.body)).not.toBeInTheDocument();
  });

  it('shows an actionable storage error and keeps the failed draft available to retry', async () => {
    mocks.saveClient.mockRejectedValue('Unable to save this client. Check available disk space.');
    render(<App />);
    await unlock();
    await selectClient('Alice Able');
    fireEvent.change(screen.getByLabelText('Counseling goals'), { target: { value: 'A revised fictional goal' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await screen.findByRole('alert');
    expect(screen.getByRole('alert')).toHaveTextContent('disk space');
    expect(screen.getByLabelText('Counseling goals')).toHaveValue('A revised fictional goal');
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeEnabled();
  });
});
