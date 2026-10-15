import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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
  chooseClientCsv:vi.fn(),inspectClientCsv:vi.fn(),cancelClientCsv:vi.fn(),getStatementAccounts:vi.fn(),saveFirmBranding:vi.fn(),getFirmBranding:vi.fn(),getClientImports:vi.fn(),reviewRecurringPayment:vi.fn(),reviewStatementBalances:vi.fn(),stageClientCsv:vi.fn(),reviewTransaction:vi.fn(),bulkApproveTransactions:vi.fn(),pairTransferTransactions:vi.fn(),
  resetApiSession: vi.fn(), getClientBudget: vi.fn(), saveClientBudget: vi.fn(), getClientDebtPlan: vi.fn(), saveClientDebtPlan: vi.fn(), getClientCashflowPlan: vi.fn(), previewClientCashflowPlan: vi.fn(), saveClientCashflowPlan: vi.fn(),
  listAssessmentRecords:vi.fn(),getAssessmentRecord:vi.fn(),listClientScenarios:vi.fn(),getAssessmentPlanning:vi.fn(),compareFinancialScenarios:vi.fn(),
  getClientSavingsPlan: vi.fn(), saveClientSavingsPlan: vi.fn(),
  getCoverageReview:vi.fn(),getFinancialCompleteness:vi.fn(),getDocumentCollection:vi.fn(),saveDocumentCollection:vi.fn(),getClientCare:vi.fn(),saveClientCare:vi.fn(),getFollowUpOverview:vi.fn(),
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

beforeEach(() => {mocks.getFirmBranding.mockResolvedValue({organizationId:workspace.organizationId,revision:0,updatedAt:null,data:{name:workspace.organizationName,address:'',email:'',phone:'',website:'',disclosure:'',logoPng:null},history:[]});
  mocks.getStatementAccounts.mockImplementation(async(clientId:string)=>({clientId,importRevision:0,accounts:[]}));
  vi.clearAllMocks();
  mocks.getFollowUpOverview.mockImplementation(async(input)=>({organizationId:workspace.organizationId,counselorId:workspace.counselorId,reviewDate:'2026-10-15',timeZone:'America/Chicago',input,overdue:0,dueToday:0,upcoming:0,closed:0,total:0,pageSize:25,items:[]}));
  mocks.getDocumentCollection.mockImplementation(async(clientId:string)=>({clientId,revision:0,latestRevision:0,updatedAt:null,reviewDate:'2026-10-14',items:[],history:[],accounts:[],sources:[],missing:0,received:0,archived:0}));
  mocks.getClientCare.mockImplementation(async(clientId:string)=>({clientId,revision:0,updatedAt:null,reviewDate:'2026-10-15',overdue:0,dueToday:0,data:{intake:{address:'',household:'',employment:'',employer:'',occupation:'',incomeNotes:''},followUps:[]}}));
  mocks.listAssessmentRecords.mockImplementation(async(clientId:string)=>({clientId,records:[]}));
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
  it('opens the selected dashboard follow-up, guards its draft and clears it on lock',async()=>{
    const id='e9f2be11-e17b-4ee4-8bb3-39a7062ac9c2';
    const descriptor=Object.getOwnPropertyDescriptor(HTMLElement.prototype,'scrollIntoView');
    Object.defineProperty(HTMLElement.prototype,'scrollIntoView',{configurable:true,value:vi.fn()});
    try {
      mocks.getFollowUpOverview.mockImplementationOnce(async(input)=>({organizationId:workspace.organizationId,counselorId:workspace.counselorId,reviewDate:'2026-10-15',timeZone:'America/Chicago',input,overdue:1,dueToday:0,upcoming:0,closed:0,total:1,pageSize:25,items:[{clientId:alice.id,clientName:'Alice Able',clientStatus:alice.status,followUpId:id,kind:'task',title:'Private Alice follow-up',dueDate:'1900-01-01',time:'',status:'open',careRevision:1}]}));
      mocks.getClientCare.mockResolvedValueOnce({clientId:alice.id,revision:1,updatedAt:'2026-10-15T12:00:00Z',reviewDate:'2026-10-15',overdue:1,dueToday:0,data:{intake:{address:'',household:'',employment:'',employer:'',occupation:'',incomeNotes:''},followUps:[{id,kind:'task',title:'Private Alice follow-up',dueDate:'1900-01-01',time:'',status:'open',notes:''}]}});
      render(<App/>);await unlock();
      fireEvent.click(await screen.findByRole('button',{name:'Open Private Alice follow-up for Alice Able'}));
      await waitFor(()=>expect(screen.getByLabelText('Title')).toHaveFocus());
      expect(screen.getByRole('heading',{name:'Alice Able'})).toBeInTheDocument();
      fireEvent.change(screen.getByLabelText('Follow-up notes / outcome'),{target:{value:'Private unsaved outcome'}});
      vi.mocked(window.confirm).mockReturnValueOnce(false);
      fireEvent.click(screen.getByRole('button',{name:'Dashboard'}));
      expect(window.confirm).toHaveBeenCalled();
      expect(screen.getByLabelText('Follow-up notes / outcome')).toHaveValue('Private unsaved outcome');
      await act(async()=>mocks.lockHandler?.());
      expect(document.documentElement.outerHTML).not.toContain('Private Alice follow-up');
      expect(document.documentElement.outerHTML).not.toContain('Private unsaved outcome');
      expect(mocks.saveClientCare).not.toHaveBeenCalled();
    } finally {
      if(descriptor)Object.defineProperty(HTMLElement.prototype,'scrollIntoView',descriptor);
      else Reflect.deleteProperty(HTMLElement.prototype,'scrollIntoView');
    }
  });
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

  it('guards standalone account drafts across client/page changes and clears private labels on lock',async()=>{
    render(<App/>);await unlock();
    fireEvent.click(screen.getByRole('button',{name:'Documents'}));
    fireEvent.change(screen.getByLabelText('Client for Documents'),{target:{value:alice.id}});
    fireEvent.click(screen.getByRole('button',{name:'Statement accounts'}));
    fireEvent.click(await screen.findByRole('button',{name:'Add statement account'}));
    fireEvent.change(screen.getByLabelText('Current account name'),{target:{value:'Private Alice account'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    fireEvent.change(screen.getByLabelText('Client for Documents'),{target:{value:bob.id}});
    expect(screen.getByLabelText('Client for Documents')).toHaveValue(alice.id);
    expect(screen.getByLabelText('Current account name')).toHaveValue('Private Alice account');
    vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Collection checklist'}));
    expect(screen.getByLabelText('Current account name')).toHaveValue('Private Alice account');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private Alice account');
  });

  it('guards branding drafts across settings/main pages and updates the sidebar after save',async()=>{
    mocks.saveFirmBranding.mockImplementation(async({data})=>({organizationId:workspace.organizationId,revision:1,updatedAt:'2026-10-14T22:00:00Z',data,history:[{revision:1,name:data.name,updatedAt:'2026-10-14T22:00:00Z'}]}));render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Branding'}));await screen.findByLabelText('Firm display name');fireEvent.change(screen.getByLabelText('Firm display name'),{target:{value:'Fictional branded practice'}});vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Security'}));expect(screen.getByLabelText('Firm display name')).toHaveValue('Fictional branded practice');vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Dashboard'}));expect(screen.getByLabelText('Firm display name')).toHaveValue('Fictional branded practice');fireEvent.click(screen.getByRole('button',{name:'Save firm branding'}));await screen.findByText(/Firm branding saved locally/);expect(document.querySelector('.workspace-label')).toHaveTextContent('Fictional branded practice');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Fictional branded practice');
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


it('opens all twelve sections and retains the selected client across focused working pages',async()=>{
 render(<App/>);await unlock();const nav=screen.getByRole('navigation',{name:'Main navigation'});expect(within(nav).getAllByRole('button')).toHaveLength(12);
 fireEvent.click(within(nav).getByRole('button',{name:'Documents'}));expect(screen.getByRole('heading',{name:'Select a client to open Documents'})).toBeInTheDocument();
 fireEvent.change(screen.getByLabelText('Client for Documents'),{target:{value:alice.id}});await screen.findByText('No source documents yet. Inspect a local statement above to begin.');expect(mocks.getClientImports).toHaveBeenLastCalledWith(alice.id);expect(screen.queryByText('Client review queue')).not.toBeInTheDocument();expect(screen.queryByText('Saved counselor assessments')).not.toBeInTheDocument();
 for(const page of ['Financial Review','Transactions','Budget Planner','Debt Planner','Savings Planner','Cash-Flow Calendar','Financial Scenarios','Reports']){
  fireEvent.click(within(nav).getByRole('button',{name:page}));await waitFor(()=>expect(screen.getByLabelText(`Client for ${page}`)).toHaveValue(alice.id));expect(within(nav).getByRole('button',{name:page})).toHaveAttribute('aria-current','page');
  await waitFor(()=>expect(screen.queryByText('Loading imported records…')).not.toBeInTheDocument());
 }
 expect(screen.getByText('Saved counselor assessments')).toBeInTheDocument();expect(screen.queryByLabelText('Local CSV file')).not.toBeInTheDocument();expect(screen.queryByText('Client review queue')).not.toBeInTheDocument();
 fireEvent.click(within(nav).getByRole('button',{name:'Settings'}));expect(screen.getByRole('heading',{name:'Organization & counselor'})).toBeInTheDocument();
 fireEvent.click(within(nav).getByRole('button',{name:'Clients'}));await screen.findByRole('heading',{name:'Alice Able'});
 fireEvent.click(screen.getByRole('button',{name:'Profile'}));expect(screen.getByLabelText(/First name/)).toHaveValue('Alice');expect(screen.queryByText('No saved budget yet')).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('button',{name:'History & records'}));expect(screen.getByRole('heading',{name:'Client history and records'})).toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:'Open report and action-plan history'}));await screen.findByText('Saved counselor assessments');
});
it('guards page, future-area and client changes, then discards explicitly without saving private drafts',async()=>{
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Budget Planner'}));fireEvent.change(screen.getByLabelText('Client for Budget Planner'),{target:{value:alice.id}});await screen.findByText('No saved budget yet');fireEvent.click(screen.getByRole('button',{name:'Add income'}));fireEvent.change(screen.getByLabelText('Category or source'),{target:{value:'Private prototype draft'}});
 vi.mocked(window.confirm).mockReturnValue(false);
 fireEvent.click(screen.getByRole('button',{name:'Documents'}));expect(screen.getByLabelText('Category or source')).toHaveValue('Private prototype draft');
 fireEvent.click(screen.getByRole('button',{name:/Paycheck allocation/}));expect(screen.getByLabelText('Category or source')).toHaveValue('Private prototype draft');
 fireEvent.change(screen.getByLabelText('Client for Budget Planner'),{target:{value:bob.id}});expect(screen.getByLabelText('Client for Budget Planner')).toHaveValue(alice.id);expect(mocks.getClient).toHaveBeenCalledTimes(1);
 vi.mocked(window.confirm).mockReturnValue(true);fireEvent.click(screen.getByRole('button',{name:/Paycheck allocation/}));expect(screen.getByRole('region',{name:'Paycheck allocation planned feature'})).toHaveTextContent('does not create records');expect(screen.queryByDisplayValue('Private prototype draft')).not.toBeInTheDocument();expect(mocks.saveClientBudget).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Working tools'}));await screen.findByText('No saved budget yet');expect(screen.queryByDisplayValue('Private prototype draft')).not.toBeInTheDocument();
 await act(async()=>mocks.lockHandler?.());expect(screen.queryByLabelText('Client for Budget Planner')).not.toBeInTheDocument();expect(document.documentElement.outerHTML).not.toContain('Private prototype draft');
});
it('ignores a delayed client selection after leaving a module page and permits retry after a read error',async()=>{
 const pending=deferred<Client>();mocks.getClient.mockReturnValueOnce(pending.promise);render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Documents'}));fireEvent.change(screen.getByLabelText('Client for Documents'),{target:{value:alice.id}});expect(screen.getByRole('status')).toHaveTextContent('Opening client record');fireEvent.click(screen.getByRole('button',{name:'Reports'}));await act(async()=>pending.resolve(alice));expect(screen.getByLabelText('Client for Reports')).toHaveValue('');expect(screen.queryByText('Saved counselor assessments')).not.toBeInTheDocument();
 mocks.getClient.mockRejectedValueOnce(new Error('Synthetic selection failure'));fireEvent.change(screen.getByLabelText('Client for Reports'),{target:{value:alice.id}});expect(await screen.findByRole('alert')).toHaveTextContent('Synthetic selection failure');fireEvent.change(screen.getByLabelText('Client for Reports'),{target:{value:bob.id}});await screen.findByText('Saved counselor assessments');expect(screen.getByLabelText('Client for Reports')).toHaveValue(bob.id);
});
it('exposes labeled future areas and every administration page without fake save controls',async()=>{
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Documents'}));fireEvent.click(screen.getByRole('button',{name:/Scanned documents \/ OCR/}));expect(screen.getByRole('region',{name:'Scanned documents / OCR planned feature'})).toHaveTextContent('mandatory verification');expect(screen.queryByRole('button',{name:/Save OCR/})).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('button',{name:'Settings'}));
 for(const tab of ['Branding','Counselors & roles','Data management','Licensing','Help & about','Security','Backup & recovery']){
  await waitFor(()=>expect(screen.getByRole('button',{name:tab})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:tab}));expect(screen.getByRole('button',{name:tab})).toHaveAttribute('aria-current','page');if(tab==='Branding')await screen.findByLabelText('Firm display name');
 }
 expect(screen.getByRole('heading',{name:'Encrypted workspace backup'})).toBeInTheDocument();expect(screen.getByRole('button',{name:'Lock workspace to restore a backup'})).toBeInTheDocument();
});

it('creates, reloads and removes a saved budget row through its dedicated page',async()=>{
 let budget=await mocks.getClientBudget(alice.id);mocks.getClientBudget.mockImplementation(async()=>budget);mocks.saveClientBudget.mockImplementation(async(clientId,input)=>{budget={...budget,clientId,revision:budget.revision+1,lines:input.lines,summary:{...budget.summary,monthlyAmounts:input.lines.length?['2500.00']:[],monthlyIncome:input.lines.length?'2500.00':'0.00',surplusBeforeAllocations:input.lines.length?'2500.00':'0.00',availableForDebt:input.lines.length?'2500.00':'0.00',unallocated:input.lines.length?'2500.00':'0.00'}};return budget;});
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Budget Planner'}));fireEvent.change(screen.getByLabelText('Client for Budget Planner'),{target:{value:alice.id}});await screen.findByText('No saved budget yet');fireEvent.click(screen.getByRole('button',{name:'Add income'}));fireEvent.change(screen.getByLabelText('Category or source'),{target:{value:'Fictional prototype wages'}});fireEvent.change(screen.getByLabelText('Amount (USD)'),{target:{value:'2500.00'}});fireEvent.click(screen.getByRole('button',{name:'Save budget and calculate'}));await screen.findByText('Budget saved and calculated.');expect(mocks.saveClientBudget).toHaveBeenLastCalledWith(alice.id,{expectedRevision:0,lines:[{label:'Fictional prototype wages',kind:'income',amount:'2500.00',frequency:'monthly',estimated:false}]});
 fireEvent.click(screen.getByRole('button',{name:'Documents'}));await screen.findByText('No source documents yet. Inspect a local statement above to begin.');fireEvent.click(screen.getByRole('button',{name:'Budget Planner'}));await screen.findByDisplayValue('Fictional prototype wages');fireEvent.click(screen.getByRole('button',{name:'Remove line 1'}));fireEvent.click(screen.getByRole('button',{name:'Save budget and calculate'}));await screen.findByText('Budget saved and calculated.');expect(mocks.saveClientBudget).toHaveBeenLastCalledWith(alice.id,{expectedRevision:1,lines:[]});
});

it('keeps backup operations on their page until completion and clears the password immediately',async()=>{
 const pending=deferred<{path:string;createdAt:string;auditRecorded:boolean;durabilityWarning:boolean}>();mocks.createWorkspaceBackup.mockReturnValueOnce(pending.promise);render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Backup & recovery'}));fireEvent.change(screen.getByLabelText('Backup folder'),{target:{value:'/fictional/backups'}});fireEvent.change(screen.getByLabelText('Current workspace password'),{target:{value:'Fictional-test-password-123'}});fireEvent.click(screen.getByRole('button',{name:'Create encrypted backup'}));expect(screen.getByLabelText('Current workspace password')).toHaveValue('');expect(screen.getByRole('button',{name:'Branding'})).toBeDisabled();fireEvent.click(screen.getByRole('button',{name:'Documents'}));expect(screen.getByRole('heading',{name:'Encrypted workspace backup'})).toBeInTheDocument();await act(async()=>pending.resolve({path:'/fictional/backups/test.ebcbackup',createdAt:'2026-10-15T00:00:00Z',auditRecorded:true,durabilityWarning:false}));expect(await screen.findByText('Encrypted backup saved.')).toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:'Documents'}));expect(screen.getByRole('heading',{name:'Select a client to open Documents'})).toBeInTheDocument();
});

it('keeps the collection area when selecting a client and guards checklist drafts on navigation and lock',async()=>{
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Documents'}));fireEvent.click(screen.getByRole('button',{name:'Collection checklist'}));fireEvent.change(screen.getByLabelText('Client for Documents'),{target:{value:alice.id}});await screen.findByText('No document requests yet. Add a request to begin.');fireEvent.click(screen.getByRole('button',{name:'Add document request'}));fireEvent.change(screen.getByLabelText('Request title'),{target:{value:'Private fictional collection draft'}});vi.mocked(window.confirm).mockReturnValue(false);fireEvent.click(screen.getByRole('button',{name:'Reports'}));expect(screen.getByLabelText('Request title')).toHaveValue('Private fictional collection draft');fireEvent.change(screen.getByLabelText('Client for Documents'),{target:{value:bob.id}});expect(screen.getByLabelText('Client for Documents')).toHaveValue(alice.id);await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional collection draft');expect(mocks.saveDocumentCollection).not.toHaveBeenCalled();
});

it('opens working completeness and report checklist areas for the selected client without planned badges',async()=>{
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Financial Review'}));
 const coverage=screen.getByRole('button',{name:'Completeness & final validation'});expect(coverage).not.toHaveTextContent('Planned');fireEvent.click(coverage);
 expect(screen.getByRole('heading',{name:'Select a client to open Financial Review'})).toBeInTheDocument();
 fireEvent.change(screen.getByLabelText('Client for Financial Review'),{target:{value:alice.id}});await screen.findByRole('region',{name:'Financial completeness review'});
 expect(mocks.getFinancialCompleteness).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Reports'}));fireEvent.click(screen.getByRole('button',{name:'Final report checklist'}));
 await screen.findByLabelText('Saved assessment for validation');expect(mocks.listAssessmentRecords).toHaveBeenCalledWith(alice.id);
 await act(async()=>mocks.lockHandler?.());expect(screen.queryByRole('region',{name:'Saved report checklist'})).not.toBeInTheDocument();
});

it('guards persistent completeness drafts across navigation and clears them on workspace lock',async()=>{
 mocks.getCoverageReview.mockResolvedValue({clientId:alice.id,latestRevision:0,snapshot:null,history:[],choices:{accounts:[],sources:[]},currentSourceRevision:0,currentProfileUpdatedAt:alice.updatedAt,needsReview:false,reviewReasons:[],unresolved:[]});
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Financial Review'}));fireEvent.click(screen.getByRole('button',{name:'Completeness & final validation'}));fireEvent.change(screen.getByLabelText('Client for Financial Review'),{target:{value:alice.id}});await screen.findByText('Saved completeness reviews');fireEvent.click(screen.getByText('Saved completeness reviews'));await screen.findByLabelText('Missing information');fireEvent.change(screen.getByLabelText('Missing information'),{target:{value:'Private fictional completeness draft'}});vi.mocked(window.confirm).mockReturnValue(false);fireEvent.click(screen.getByRole('button',{name:'Documents'}));expect(screen.getByLabelText('Missing information')).toHaveValue('Private fictional completeness draft');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional completeness draft');
});
