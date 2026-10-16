import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import type { Client, ClientNote, WorkspaceInfo } from './types';

const mocks = vi.hoisted(() => ({
  getClientWorkspaceOverview:vi.fn(),getSecurityPolicy:vi.fn(),saveSecurityPolicy:vi.fn(),changeWorkspacePassword:vi.fn(),getClientHistory:vi.fn(),getNoteHistory:vi.fn(),correctClientNote:vi.fn(),
  getReportPresentation:vi.fn(),chooseReportPresentationPdf:vi.fn(),exportReportPresentation:vi.fn(),
  getCategoryRules:vi.fn(),saveCategoryRule:vi.fn(),reviewRuleCategory:vi.fn(),queryCategorySuggestions:vi.fn(),prepareCategoryDraft:vi.fn(),queryTransactionActivity:vi.fn(),
  desktop: true,
  lockHandler: undefined as (() => void) | undefined,
  setupStatus: vi.fn(), setupWorkspace: vi.fn(), unlockWorkspace: vi.fn(),
  lockWorkspace: vi.fn(), workspaceInfo: vi.fn(), listClients: vi.fn(),
  getClient: vi.fn(), saveClient: vi.fn(), addClientNote: vi.fn(), listClientNotes: vi.fn(),
  chooseBackupFolder:vi.fn(), chooseRestoreFile:vi.fn(), createWorkspaceBackup:vi.fn(), prepareWorkspaceRestore:vi.fn(), confirmWorkspaceRestore:vi.fn(), cancelWorkspaceRestore:vi.fn(),
  chooseClientCsv:vi.fn(),inspectClientCsv:vi.fn(),cancelClientCsv:vi.fn(),getStatementAccounts:vi.fn(),saveFirmBranding:vi.fn(),getFirmBranding:vi.fn(),getClientImports:vi.fn(),reviewRecurringPayment:vi.fn(),reviewStatementBalances:vi.fn(),stageClientCsv:vi.fn(),reviewTransaction:vi.fn(),bulkApproveTransactions:vi.fn(),pairTransferTransactions:vi.fn(),
  resetApiSession: vi.fn(), getClientBudget: vi.fn(), saveClientBudget: vi.fn(), getClientDebtPlan: vi.fn(), saveClientDebtPlan: vi.fn(), getClientCashflowPlan: vi.fn(), previewClientCashflowPlan: vi.fn(), saveClientCashflowPlan: vi.fn(),
  listAssessmentRecords:vi.fn(),getAssessmentRecord:vi.fn(),reviewFinalReport:vi.fn(),approveFinalReport:vi.fn(),listClientScenarios:vi.fn(),getAssessmentPlanning:vi.fn(),compareFinancialScenarios:vi.fn(),
  getClientSavingsPlan: vi.fn(), saveClientSavingsPlan: vi.fn(),
  getCardFacts:vi.fn(),getCardReview:vi.fn(),getCardStatements:vi.fn(),getCoverageReview:vi.fn(),getFinancialCompleteness:vi.fn(),getDocumentCollection:vi.fn(),saveDocumentCollection:vi.fn(),getClientCare:vi.fn(),saveClientCare:vi.fn(),getFollowUpOverview:vi.fn(),
}));

vi.mock('./ConfirmationDialog',async importOriginal=>{const original=await importOriginal<typeof import('./ConfirmationDialog')>();return {...original,confirmDiscard:(message:string,action:()=>void)=>{if(window.confirm(message))action();}};});

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

beforeEach(() => {mocks.getClientWorkspaceOverview.mockImplementation(async(clientId:string)=>({clientId,monthlyIncome:null,necessaryExpenses:null,unallocated:null,enteredDebt:null,debtMonthlyPayment:null,budgetDebtPayment:null,emergencyTarget:null,accessibleSavings:null,restrictedSavings:null,payoffMonth:null,warnings:[]}));mocks.getSecurityPolicy.mockResolvedValue({revision:0,idleMinutes:15,totalChanges:0,history:[]});mocks.getClientHistory.mockImplementation(async(clientId:string)=>({clientId,snapshot:'history-snapshot',offset:0,total:0,events:[]}));mocks.getCategoryRules.mockImplementation(async(clientId:string)=>({clientId,fingerprint:'rule-fingerprint',accounts:[],rules:[],applications:[],applicationCount:0}));mocks.getFirmBranding.mockResolvedValue({organizationId:workspace.organizationId,revision:0,updatedAt:null,data:{name:workspace.organizationName,address:'',email:'',phone:'',website:'',disclosure:'',logoPng:null},history:[]});
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

function go(label:string){fireEvent.click(within(screen.getByRole('navigation',{name:'Client workspace sections'})).getByRole('button',{name:label}));}
async function selectClient(name:string,section='Profile & intake'){
 fireEvent.click(screen.getByRole('button',{name:'Clients'}));
 fireEvent.click(screen.getByRole('button',{name:`Open workspace for ${name}`}));
 await screen.findByRole('heading',{name});
 if(section)go(section);
}
async function switchClient(name:string){
 const active=screen.queryByRole('navigation',{name:'Client workspace sections'})?.querySelector('[aria-current=page]')?.getAttribute('aria-label')??'Profile & intake';
 fireEvent.click(screen.getByRole('tab',{name:'All clients'}));
 const open=screen.queryByRole('button',{name:`Open workspace for ${name}`});
 if(!open)return;
 fireEvent.click(open);await screen.findByRole('heading',{name});go(active);
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
    go('Profile & intake');fireEvent.click(screen.getByRole('button',{name:'Intake & follow-ups'}));fireEvent.click(screen.getByText('Intake and follow-ups'));
    await screen.findByLabelText('Employer');
    await waitFor(()=>expect(screen.getByLabelText('Employer')).toBeEnabled());
    fireEvent.change(screen.getByLabelText('Employer'),{target:{value:'Alice private fictional employer'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    await switchClient('Bob Baker');
    expect(window.confirm).toHaveBeenCalled();
    expect(screen.getByLabelText('Employer')).toHaveValue('Alice private fictional employer');
    await act(async()=>mocks.lockHandler?.());
    expect(document.documentElement.outerHTML).not.toContain('Alice private fictional employer');
    expect(screen.queryByText('Intake and follow-ups')).not.toBeInTheDocument();
    expect(mocks.saveClientCare).not.toHaveBeenCalled();
  });
  it('blocks navigation during scenario baseline loading and clears late private proposals on lock',async()=>{
    const pending=deferred<unknown>();mocks.getAssessmentPlanning.mockReturnValueOnce(pending.promise);
    render(<App/>);await unlock();await selectClient('Alice Able');go('Scenario comparisons');
    fireEvent.click(screen.getByText('Compare combined financial scenarios'));fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));
    fireEvent.click(screen.getByRole('button',{name:'Dashboard'}));expect(screen.getByRole('heading',{name:'Scenario comparisons'})).toBeInTheDocument();
    expect(screen.queryByRole('button',{name:'Add income'})).not.toBeInTheDocument();expect(mocks.saveClientBudget).not.toHaveBeenCalled();
    await act(async()=>mocks.lockHandler?.());await act(async()=>pending.resolve({clientId:'client-a',privateAssumption:'Alice private scenario'}));
    expect(document.documentElement.outerHTML).not.toContain('Alice private scenario');expect(screen.queryByText('Compare combined financial scenarios')).not.toBeInTheDocument();expect(mocks.compareFinancialScenarios).not.toHaveBeenCalled();

  });

  it('guards standalone account drafts across client/page changes and clears private labels on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');
    go('Import statements');

    fireEvent.click(screen.getByRole('button',{name:'Statement accounts'}));
    fireEvent.click(await screen.findByRole('button',{name:'Add statement account'}));
    fireEvent.change(screen.getByLabelText('Current account name'),{target:{value:'Private Alice account'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    await switchClient('Bob Baker');
    expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');
    expect(screen.getByLabelText('Current account name')).toHaveValue('Private Alice account');
    vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Collection checklist'}));
    expect(screen.getByLabelText('Current account name')).toHaveValue('Private Alice account');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private Alice account');
  });

  it('guards branding drafts across settings/main pages and updates the sidebar after save',async()=>{
    mocks.saveFirmBranding.mockImplementation(async({data})=>({organizationId:workspace.organizationId,revision:1,updatedAt:'2026-10-14T22:00:00Z',data,history:[{revision:1,name:data.name,updatedAt:'2026-10-14T22:00:00Z'}]}));render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Branding'}));await screen.findByLabelText('Firm display name');fireEvent.change(screen.getByLabelText('Firm display name'),{target:{value:'Fictional branded practice'}});vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Security'}));expect(screen.getByLabelText('Firm display name')).toHaveValue('Fictional branded practice');vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Dashboard'}));expect(screen.getByLabelText('Firm display name')).toHaveValue('Fictional branded practice');fireEvent.click(screen.getByRole('button',{name:'Save firm branding'}));await screen.findByText(/Firm branding saved locally/);expect(document.querySelector('.workspace-label')).toHaveTextContent('Fictional branded practice');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Fictional branded practice');
  });

  it('guards unsaved CSV selection and clears private source paths on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');go('Review & reconciliation');await screen.findByText('No imported transactions yet');
    go('Import statements');await screen.findByLabelText('Local CSV file');fireEvent.change(screen.getByLabelText('Local CSV file'),{target:{value:'D:\\Synthetic\\alice-private.csv'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);await switchClient('Bob Baker');expect(window.confirm).toHaveBeenCalled();expect(screen.getByLabelText('Local CSV file')).toHaveValue('D:\\Synthetic\\alice-private.csv');
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
    render(<App/>);await unlock();await selectClient('Alice Able');go('Cash-flow calendar');await screen.findByText('No saved cash-flow plan yet');
    fireEvent.click(screen.getByRole('button',{name:'Add cash-flow account'}));
    fireEvent.change(screen.getByLabelText('Account name'),{target:{value:'Alice private checking'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);await switchClient('Bob Baker');
    expect(window.confirm).toHaveBeenCalled();expect(screen.getByLabelText('Account name')).toHaveValue('Alice private checking');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Alice private checking');expect(mocks.saveClientCashflowPlan).not.toHaveBeenCalled();
  });

  it('guards unsaved savings edits and clears private reserves on lock',async()=>{
    render(<App/>);await unlock();await selectClient('Alice Able');go('Savings & sinking funds');await screen.findByText('No saved savings plan yet');
    fireEvent.change(screen.getByLabelText('Funding priorities and notes'),{target:{value:'Alice private savings priority'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);await switchClient('Bob Baker');
    expect(window.confirm).toHaveBeenCalled();expect(screen.getByLabelText('Funding priorities and notes')).toHaveValue('Alice private savings priority');
    await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Alice private savings priority');expect(mocks.saveClientSavingsPlan).not.toHaveBeenCalled();
  });

  it('guards unsaved debt edits and clears them when the workspace locks', async () => {
    render(<App />); await unlock(); await selectClient('Alice Able');
    go('Debt payoff');await screen.findByText('No saved debt plan yet');
    fireEvent.click(screen.getByRole('button', {name:'Add debt'}));
    fireEvent.change(screen.getByLabelText('Debt name'), {target:{value:'Private Alice loan'}});
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    await switchClient('Bob Baker');
    expect(window.confirm).toHaveBeenCalled();
    expect(screen.getByLabelText('Debt name')).toHaveValue('Private Alice loan');
    await act(async () => mocks.lockHandler?.());
    expect(document.documentElement.outerHTML).not.toContain('Private Alice loan');
    expect(mocks.saveClientDebtPlan).not.toHaveBeenCalled();
  });

  it('guards unsaved budget edits when changing clients and clears them on a lock event', async () => {
    render(<App />); await unlock(); await selectClient('Alice Able');
    go('Monthly budget');await screen.findByText('No saved budget yet');
    fireEvent.click(screen.getByRole('button', { name: 'Add income' }));
    fireEvent.change(screen.getByLabelText('Category or source'), { target: { value: 'Alice private wage estimate' } });
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    await switchClient('Bob Baker');
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
    render(<App />); await unlock(); await selectClient('Alice Able');go('Monthly budget');
    await switchClient('Bob Baker');
    await screen.findByRole('heading', { name: 'Bob Baker' });
    go('Monthly budget');await screen.findByText('No saved budget yet');
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
    go('Profile & intake');expect(screen.getByRole('button', { name: 'Save changes' })).toBeInTheDocument();
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
    await selectClient('Alice Able');go('Session notes');
    await switchClient('Bob Baker');
    await screen.findByRole('heading', { name: 'Bob Baker' });
    await waitFor(() => expect(screen.getByLabelText('New session note')).toBeEnabled());
    await act(async () => { previousNotes.resolve([aliceNote]); });
    expect(screen.queryByText(aliceNote.body)).not.toBeInTheDocument();
    expect(screen.getByLabelText('New session note')).toHaveValue('');
    go('Profile & intake');expect(screen.getByLabelText('Counseling goals')).toHaveValue(bob.goals);
  });

  it('clears an unsaved note when switching clients and saves the new note to the current client only', async () => {
    render(<App />);
    await unlock();
    await selectClient('Alice Able');go('Session notes');
    await waitFor(() => expect(screen.getByLabelText('New session note')).toBeEnabled());
    fireEvent.change(screen.getByLabelText('New session note'), { target: { value: 'Unsaved private Alice draft' } });
    await waitFor(() => expect(window.confirm).not.toHaveBeenCalled());
    await switchClient('Bob Baker');
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
    await selectClient('Alice Able');go('Session notes');
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


it('opens all twelve client sections lazily with only three application-level pages',async()=>{
 render(<App/>);await unlock();const nav=screen.getByRole('navigation',{name:'Main navigation'});expect(within(nav).getAllByRole('button')).toHaveLength(3);
 fireEvent.click(within(nav).getByRole('button',{name:'Clients'}));expect(screen.queryByLabelText('New session note')).not.toBeInTheDocument();expect(mocks.getClientImports).not.toHaveBeenCalled();expect(mocks.getClientBudget).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Open workspace for Alice Able'}));await screen.findByRole('heading',{name:'Alice Able'});await screen.findByRole('heading',{name:'Overview'});expect(within(screen.getByRole('navigation',{name:'Client workspace sections'})).getAllByRole('button')).toHaveLength(12);
 for(const page of ['Monthly budget','Debt payoff','Savings & sinking funds','Cash-flow calendar','Scenario comparisons','Import statements','Review & reconciliation','Financial dashboard','Assessments & reports']){
  go(page);expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');await waitFor(()=>expect(screen.queryByText('Loading imported records…')).not.toBeInTheDocument());
 }
 expect(screen.getByText('Saved counselor assessments')).toBeInTheDocument();expect(screen.queryByLabelText('Local CSV file')).not.toBeInTheDocument();expect(screen.queryByText('Client review queue')).not.toBeInTheDocument();
 fireEvent.click(within(nav).getByRole('button',{name:'Settings'}));expect(screen.getByRole('heading',{name:'Organization & counselor'})).toBeInTheDocument();
 fireEvent.click(screen.getByRole('tab',{name:'Alice Able'}));await screen.findByRole('heading',{name:'Alice Able'});go('Profile & intake');expect(screen.getByLabelText(/First name/)).toHaveValue('Alice');expect(screen.queryByText('No saved budget yet')).not.toBeInTheDocument();
 go('Session notes');fireEvent.click(screen.getByRole('button',{name:'History & records'}));expect(screen.getByRole('heading',{name:'Client history and records'})).toBeInTheDocument();await waitFor(()=>expect(screen.getByRole('button',{name:'Open report and action-plan history'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'Open report and action-plan history'}));await screen.findByText('Saved counselor assessments');
});
it('guards page, future-area and client changes, then discards explicitly without saving private drafts',async()=>{
 render(<App/>);await unlock();await selectClient('Alice Able');go('Monthly budget');go('Monthly budget');await screen.findByText('No saved budget yet');fireEvent.click(screen.getByRole('button',{name:'Add income'}));fireEvent.change(screen.getByLabelText('Category or source'),{target:{value:'Private prototype draft'}});
 vi.mocked(window.confirm).mockReturnValue(false);
 go('Import statements');expect(screen.getByLabelText('Category or source')).toHaveValue('Private prototype draft');
 fireEvent.click(screen.getByRole('button',{name:/Paycheck allocation/}));expect(screen.getByLabelText('Category or source')).toHaveValue('Private prototype draft');
 await switchClient('Bob Baker');expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');expect(mocks.getClient).toHaveBeenCalledTimes(1);
 vi.mocked(window.confirm).mockReturnValue(true);fireEvent.click(screen.getByRole('button',{name:/Paycheck allocation/}));expect(screen.getByRole('region',{name:'Paycheck allocation planned feature'})).toHaveTextContent('does not create records');expect(screen.queryByDisplayValue('Private prototype draft')).not.toBeInTheDocument();expect(mocks.saveClientBudget).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Working tools'}));go('Monthly budget');await screen.findByText('No saved budget yet');expect(screen.queryByDisplayValue('Private prototype draft')).not.toBeInTheDocument();
 await act(async()=>mocks.lockHandler?.());expect(screen.queryByLabelText('Client for Budget Planner')).not.toBeInTheDocument();expect(document.documentElement.outerHTML).not.toContain('Private prototype draft');
});
it('ignores delayed client selections after leaving and permits retry after a read error',async()=>{
 const pending=deferred<Client>();mocks.getClient.mockReturnValueOnce(pending.promise);render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Clients'}));fireEvent.click(screen.getByRole('button',{name:'Open workspace for Alice Able'}));expect(screen.getByRole('status')).toHaveTextContent('Opening client record');fireEvent.click(screen.getByRole('button',{name:'Settings'}));await act(async()=>pending.resolve(alice));expect(screen.getByRole('heading',{name:'Organization & counselor'})).toBeInTheDocument();expect(screen.queryByRole('tab',{name:'Alice Able'})).not.toBeInTheDocument();expect(mocks.getClientImports).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('tab',{name:'All clients'}));mocks.getClient.mockRejectedValueOnce(new Error('Synthetic selection failure'));fireEvent.click(screen.getByRole('button',{name:'Open workspace for Alice Able'}));expect(await screen.findByRole('alert')).toHaveTextContent('Synthetic selection failure');fireEvent.click(screen.getByRole('button',{name:'Return to all clients'}));fireEvent.click(screen.getByRole('button',{name:'Open workspace for Bob Baker'}));await screen.findByRole('heading',{name:'Bob Baker'});expect(screen.getByRole('tab',{name:'Bob Baker'})).toHaveAttribute('aria-selected','true');
});
it('exposes labeled future areas and every administration page without fake save controls',async()=>{
 render(<App/>);await unlock();await selectClient('Alice Able');go('Import statements');fireEvent.click(screen.getByRole('button',{name:/Scanned documents \/ OCR/}));expect(screen.getByRole('region',{name:'Scanned documents / OCR planned feature'})).toHaveTextContent('mandatory verification');expect(screen.queryByRole('button',{name:/Save OCR/})).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('button',{name:'Settings'}));
 for(const tab of ['Branding','Counselors & roles','Data management','Licensing','Help & about','Security','Backup & recovery']){
  await waitFor(()=>expect(screen.getByRole('button',{name:tab})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:tab}));expect(screen.getByRole('button',{name:tab})).toHaveAttribute('aria-current','page');if(tab==='Branding')await screen.findByLabelText('Firm display name');
 }
 expect(screen.getByRole('heading',{name:'Encrypted workspace backup'})).toBeInTheDocument();expect(screen.getByRole('button',{name:'Lock workspace to restore a backup'})).toBeInTheDocument();
});

it('creates, reloads and removes a saved budget row through its dedicated page',async()=>{
 let budget=await mocks.getClientBudget(alice.id);mocks.getClientBudget.mockImplementation(async()=>budget);mocks.saveClientBudget.mockImplementation(async(clientId,input)=>{budget={...budget,clientId,revision:budget.revision+1,lines:input.lines,summary:{...budget.summary,monthlyAmounts:input.lines.length?['2500.00']:[],monthlyIncome:input.lines.length?'2500.00':'0.00',surplusBeforeAllocations:input.lines.length?'2500.00':'0.00',availableForDebt:input.lines.length?'2500.00':'0.00',unallocated:input.lines.length?'2500.00':'0.00'}};return budget;});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Monthly budget');go('Monthly budget');await screen.findByText('No saved budget yet');fireEvent.click(screen.getByRole('button',{name:'Add income'}));fireEvent.change(screen.getByLabelText('Category or source'),{target:{value:'Fictional prototype wages'}});fireEvent.change(screen.getByLabelText('Amount (USD)'),{target:{value:'2500.00'}});fireEvent.click(screen.getByRole('button',{name:'Save budget and calculate'}));await screen.findByText('Budget saved and calculated.');expect(mocks.saveClientBudget).toHaveBeenLastCalledWith(alice.id,{expectedRevision:0,lines:[{label:'Fictional prototype wages',kind:'income',amount:'2500.00',frequency:'monthly',estimated:false}]});
 await waitFor(()=>expect(within(screen.getByRole('navigation',{name:'Client workspace sections'})).getByRole('button',{name:'Import statements'})).toBeEnabled());go('Import statements');await screen.findByText('No source documents yet. Inspect a local statement above to begin.');go('Monthly budget');await screen.findByDisplayValue('Fictional prototype wages');fireEvent.click(screen.getByRole('button',{name:'Remove line 1'}));fireEvent.click(screen.getByRole('button',{name:'Save budget and calculate'}));await screen.findByText('Budget saved and calculated.');expect(mocks.saveClientBudget).toHaveBeenLastCalledWith(alice.id,{expectedRevision:1,lines:[]});
});

it('keeps backup operations on their page until completion and clears the password immediately',async()=>{
 const pending=deferred<{path:string;createdAt:string;auditRecorded:boolean;durabilityWarning:boolean}>();mocks.createWorkspaceBackup.mockReturnValueOnce(pending.promise);render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Backup & recovery'}));fireEvent.change(screen.getByLabelText('Backup folder'),{target:{value:'/fictional/backups'}});fireEvent.change(screen.getByLabelText('Current workspace password'),{target:{value:'Fictional-test-password-123'}});fireEvent.click(screen.getByRole('button',{name:'Create encrypted backup'}));expect(screen.getByLabelText('Current workspace password')).toHaveValue('');expect(screen.getByRole('button',{name:'Branding'})).toBeDisabled();fireEvent.click(screen.getByRole('tab',{name:'All clients'}));expect(screen.getByRole('heading',{name:'Encrypted workspace backup'})).toBeInTheDocument();await act(async()=>pending.resolve({path:'/fictional/backups/test.ebcbackup',createdAt:'2026-10-15T00:00:00Z',auditRecorded:true,durabilityWarning:false}));expect(await screen.findByText('Encrypted backup saved.')).toBeInTheDocument();fireEvent.click(screen.getByRole('tab',{name:'All clients'}));expect(screen.getByRole('region',{name:'Client directory'})).toBeInTheDocument();
});

it('keeps the collection area when selecting a client and guards checklist drafts on navigation and lock',async()=>{
 render(<App/>);await unlock();await selectClient('Alice Able');go('Import statements');fireEvent.click(screen.getByRole('button',{name:'Collection checklist'}));await screen.findByText('No document requests yet. Add a request to begin.');fireEvent.click(screen.getByRole('button',{name:'Add document request'}));fireEvent.change(screen.getByLabelText('Request title'),{target:{value:'Private fictional collection draft'}});vi.mocked(window.confirm).mockReturnValue(false);go('Assessments & reports');expect(screen.getByLabelText('Request title')).toHaveValue('Private fictional collection draft');await switchClient('Bob Baker');expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional collection draft');expect(mocks.saveDocumentCollection).not.toHaveBeenCalled();
});

it('opens working completeness and report checklist areas for the selected client without planned badges',async()=>{
 render(<App/>);await unlock();await selectClient('Alice Able');go('Financial dashboard');
 const coverage=screen.getByRole('button',{name:'Completeness & final validation'});expect(coverage).not.toHaveTextContent('Planned');fireEvent.click(coverage);
 expect(screen.getByRole('heading',{name:'Alice Able'})).toBeInTheDocument();
 await screen.findByRole('region',{name:'Financial completeness review'});
 expect(mocks.getFinancialCompleteness).not.toHaveBeenCalled();
 go('Assessments & reports');fireEvent.click(screen.getByRole('button',{name:'Final report checklist'}));
 await screen.findByLabelText('Saved assessment for validation');expect(mocks.listAssessmentRecords).toHaveBeenCalledWith(alice.id);
 await act(async()=>mocks.lockHandler?.());expect(screen.queryByRole('region',{name:'Saved report checklist'})).not.toBeInTheDocument();
});

it('guards persistent completeness drafts across navigation and clears them on workspace lock',async()=>{
 mocks.getCoverageReview.mockResolvedValue({clientId:alice.id,latestRevision:0,snapshot:null,history:[],choices:{accounts:[],sources:[]},currentSourceRevision:0,currentProfileUpdatedAt:alice.updatedAt,needsReview:false,reviewReasons:[],unresolved:[]});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Financial dashboard');fireEvent.click(screen.getByRole('button',{name:'Completeness & final validation'}));await screen.findByText('Saved completeness reviews');fireEvent.click(screen.getByText('Saved completeness reviews'));await screen.findByLabelText('Missing information');fireEvent.change(screen.getByLabelText('Missing information'),{target:{value:'Private fictional completeness draft'}});vi.mocked(window.confirm).mockReturnValue(false);go('Import statements');expect(screen.getByLabelText('Missing information')).toHaveValue('Private fictional completeness draft');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional completeness draft');
});

it.each(['v1','v2'])('guards %s final approval drafts across navigation and clears private approvals on lock',async(policy)=>{
 mocks.listAssessmentRecords.mockResolvedValue({clientId:alice.id,records:[{id:'assessment',title:'Fictional reviewed assessment',periodStart:'2026-09-01',periodEnd:'2026-09-30',needsReview:false}]});
 mocks.getAssessmentRecord.mockResolvedValue({clientId:alice.id,digest:'digest',payload:{id:'assessment',title:'Fictional reviewed assessment',client:alice},reviewReasons:[],validation:{version:'coverage-review-v1',checks:[],coverage:{periodStart:'2026-09-01',periodEnd:'2026-09-30',revision:1,approved:2,rejected:0,pending:0,clarification:0,undated:0,outOfPeriod:0,assignmentIssues:0,blockingReasons:[],accounts:[],sources:[]}}});
 mocks.reviewFinalReport.mockResolvedValue({receipt:'receipt',approvalId:null,readiness:{policy:`counselor-final-report-${policy}`,clientId:alice.id,assessmentId:'assessment',assessmentDigest:'digest',reviewedOn:'2026-10-15',ready:true,gates:[],accounts:[],planning:'notApplicable',disclosures:[]}});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Assessments & reports');fireEvent.click(screen.getByRole('button',{name:'Final report checklist'}));await screen.findByLabelText('Saved assessment for validation');await waitFor(()=>expect(screen.getByLabelText('Saved assessment for validation')).toBeEnabled());await act(async()=>{fireEvent.change(screen.getByLabelText('Saved assessment for validation'),{target:{value:'assessment'}});});await screen.findByRole('button',{name:'Review final-report readiness'});await act(async()=>fireEvent.click(screen.getByRole('button',{name:'Review final-report readiness'})));await screen.findByLabelText('Professional approval notes');fireEvent.change(screen.getByLabelText('Professional approval notes'),{target:{value:'Private fictional approval draft'}});
 if(policy==='v2')fireEvent.change(screen.getByLabelText('Card evidence and repayment review'),{target:{value:'Private fictional card approval decision'}});expect(screen.getByLabelText('Saved assessment for validation')).toBeDisabled();expect(screen.getByRole('button',{name:'Refresh report checklist'})).toBeDisabled();vi.mocked(window.confirm).mockReturnValue(false);go('Import statements');expect(screen.getByLabelText('Professional approval notes')).toHaveValue('Private fictional approval draft');await switchClient('Bob Baker');expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional approval draft');expect(document.documentElement.outerHTML).not.toContain('Private fictional card approval decision');expect(mocks.approveFinalReport).not.toHaveBeenCalled();
});

it('opens credit-card review, guards its private source draft and clears it on lock',async()=>{
 mocks.getCardStatements.mockImplementation(async(clientId:string)=>({clientId,revision:0,statements:[]}));render(<App/>);await unlock();await selectClient('Alice Able');go('Import statements');fireEvent.click(screen.getByRole('button',{name:'Credit-card statements'}));await screen.findByText('No card statements recorded for this client.');fireEvent.click(screen.getByText('Import a card CSV statement'));fireEvent.change(screen.getByLabelText('Card CSV file'),{target:{value:'/private/fictional-card.csv'}});vi.mocked(window.confirm).mockReturnValue(false);go('Assessments & reports');expect(screen.getByLabelText('Card CSV file')).toHaveValue('/private/fictional-card.csv');await switchClient('Bob Baker');expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('/private/fictional-card.csv');
});

it('guards card account drafts across pages and clears them on locking',async()=>{
 mocks.getCardReview.mockResolvedValue({clientId:alice.id,sourceRevision:0,snapshot:null,history:[],cards:{clientId:alice.id,revision:0,statements:[]},bank:{clientId:alice.id,revision:0,accounts:[],documents:[],records:[],history:[],merchantGroups:[],recurringHistory:[],summary:{}},staleRepayments:[]});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Financial dashboard');await screen.findByRole('heading',{name:'Approved financial activity'});fireEvent.click(screen.getByRole('button',{name:'Bank & card activity'}));await screen.findByText('No durable card accounts recorded.');fireEvent.change(screen.getByLabelText('Reviewed card account name'),{target:{value:'Private fictional card identity'}});await waitFor(()=>expect(within(screen.getByRole('navigation',{name:'Client workspace sections'})).getByRole('button',{name:'Assessments & reports'})).toBeEnabled());vi.mocked(window.confirm).mockReturnValue(false);go('Assessments & reports');expect(screen.getByLabelText('Reviewed card account name')).toHaveValue('Private fictional card identity');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional card identity');
});

it('guards private card completeness drafts across navigation and clears them on lock',async()=>{
 mocks.getCardFacts.mockResolvedValue({clientId:alice.id,currentSourceRevision:0,currentProfileUpdatedAt:alice.updatedAt,latestRevision:0,snapshot:null,history:[],choices:{linkRevision:0,accounts:[],sources:[],facts:null},needsReview:false,reviewReasons:[]});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Financial dashboard');await screen.findByRole('heading',{name:'Approved financial activity'});fireEvent.click(screen.getByRole('button',{name:'Card completeness'}));await screen.findByLabelText('Reviewed card inventory notes');await waitFor(()=>expect(screen.getByLabelText('Reviewed card inventory notes')).toBeEnabled());fireEvent.change(screen.getByLabelText('Reviewed card inventory notes'),{target:{value:'Private fictional inventory review'}});vi.mocked(window.confirm).mockReturnValue(false);go('Assessments & reports');expect(screen.getByLabelText('Reviewed card inventory notes')).toHaveValue('Private fictional inventory review');await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional inventory review');
});

it('opens working client activity history and clears its private results on lock',async()=>{
 mocks.queryTransactionActivity.mockResolvedValue({clientId:alice.id,snapshot:'a'.repeat(64),query:{offset:0,text:'',sourceKind:null,status:null,from:null,through:null},total:0,pageSize:25,sources:[],events:[]});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Review & reconciliation');const tab=screen.getByRole('button',{name:'Activity history'});expect(tab).not.toHaveTextContent('Planned');fireEvent.click(tab);await screen.findByText('No activity matches these filters.');expect(mocks.queryTransactionActivity).toHaveBeenCalledWith(alice.id,expect.objectContaining({offset:0}));await act(async()=>mocks.lockHandler?.());expect(screen.queryByRole('region',{name:'Transaction activity history'})).not.toBeInTheDocument();
});

it('opens report presentation, holds navigation during PDF export and clears the private preview on lock',async()=>{
 const pending=deferred<{path:string;pages:number;recordId:string}>();mocks.exportReportPresentation.mockReturnValueOnce(pending.promise);
 mocks.listAssessmentRecords.mockResolvedValue({clientId:alice.id,records:[{id:'assessment',title:'Fictional report',periodStart:'2026-09-01',periodEnd:'2026-09-30',needsReview:false}]});
 mocks.getReportPresentation.mockResolvedValue({clientId:alice.id,assessmentId:'assessment',receipt:'receipt',presentation:'counselor-summary-v1',internal:{recordId:'assessment',kind:'internal',title:'Private fictional report',firmName:'Fictional firm',clientName:'Alice Able',periodStart:'2026-09-01',periodEnd:'2026-09-30',reportDate:'2026-10-15',status:'Internal assessment',basis:'Bank-only',figures:[],balanceAsOf:null,needsReview:false,reviewReasons:[],digest:'digest',disclosure:'Original figures'},finalReport:null});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Assessments & reports');const tab=screen.getByRole('button',{name:'Print & presentation'});expect(tab).not.toHaveTextContent('Planned');fireEvent.click(tab);await screen.findByLabelText('Saved assessment for presentation');await waitFor(()=>expect(screen.getByLabelText('Saved assessment for presentation')).toBeEnabled());fireEvent.change(screen.getByLabelText('Saved assessment for presentation'),{target:{value:'assessment'}});await screen.findByText('Private fictional report');await waitFor(()=>expect(screen.getByLabelText('Presentation PDF destination')).toBeEnabled());fireEvent.change(screen.getByLabelText('Presentation PDF destination'),{target:{value:'/fictional/new.pdf'}});fireEvent.click(screen.getByRole('button',{name:'Export presentation PDF'}));go('Import statements');expect(screen.getByLabelText('Presentation PDF destination')).toHaveValue('/fictional/new.pdf');await act(async()=>mocks.lockHandler?.());await act(async()=>pending.resolve({path:'/fictional/new.pdf',pages:2,recordId:'assessment'}));expect(document.documentElement.outerHTML).not.toContain('Private fictional report');expect(document.documentElement.outerHTML).not.toContain('/fictional/new.pdf');
});

it('opens working classification assistance with current client scope and clears it on lock',async()=>{
 mocks.queryCategorySuggestions.mockResolvedValue({clientId:alice.id,revision:0,snapshot:'proof',policy:'approved-merchant-category-v1',query:{text:'',offset:0,expectedSnapshot:null},total:0,pageSize:25,candidates:[]});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Review & reconciliation');const tab=screen.getByRole('button',{name:'Classification assistance'});expect(tab).not.toHaveTextContent('Planned');fireEvent.click(tab);await screen.findByText(/No missing-category expenses or fees/);expect(mocks.queryCategorySuggestions).toHaveBeenCalledWith(alice.id,{text:'',offset:0,expectedSnapshot:null});expect(mocks.prepareCategoryDraft).not.toHaveBeenCalled();await act(async()=>mocks.lockHandler?.());expect(screen.queryByRole('region',{name:'Local category suggestions'})).not.toBeInTheDocument();
});
it('guards reviewed note corrections across pages and clients and clears private drafts on lock',async()=>{
 mocks.listClientNotes.mockResolvedValue([aliceNote]);mocks.getNoteHistory.mockResolvedValue({clientId:alice.id,fingerprint:'note-context',original:aliceNote,amendments:[]});
 render(<App/>);await unlock();await selectClient('Alice Able');go('Review & reconciliation');await screen.findByText('No imported transactions yet');fireEvent.click(screen.getByRole('button',{name:'Session notes'}));await screen.findByText(aliceNote.body);fireEvent.click(screen.getByRole('button',{name:/Review correction for note/}));await screen.findByDisplayValue(aliceNote.body);await waitFor(()=>expect(screen.getByLabelText('Corrected session note')).toBeEnabled());fireEvent.change(screen.getByLabelText('Corrected session note'),{target:{value:'Private fictional correction draft'}});fireEvent.change(screen.getByLabelText('Note correction reason'),{target:{value:'Private reviewed reason'}});
 vi.mocked(window.confirm).mockReturnValue(false);fireEvent.click(screen.getByRole('button',{name:'History & records'}));expect(screen.getByLabelText('Corrected session note')).toHaveValue('Private fictional correction draft');await switchClient('Bob Baker');expect(screen.getByRole('tab',{name:'Alice Able'})).toHaveAttribute('aria-selected','true');expect(screen.getByLabelText('Corrected session note')).toHaveValue('Private fictional correction draft');expect(screen.getByRole('button',{name:'Save session note'})).toBeDisabled();await act(async()=>mocks.lockHandler?.());expect(document.documentElement.outerHTML).not.toContain('Private fictional correction draft');expect(document.documentElement.outerHTML).not.toContain('Private reviewed reason');expect(mocks.correctClientNote).not.toHaveBeenCalled();
});

it('locks and clears workspace state after password change and shows retained-copy semantics',async()=>{
 const pending=deferred<{durabilityWarning:boolean}>();mocks.changeWorkspacePassword.mockReturnValueOnce(pending.promise);
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Security'}));await screen.findByLabelText('Inactivity timeout (minutes)');
 for(const [label,value] of [['Current password','Synthetic-original-123'],['New password','Synthetic-new-456'],['Confirm new password','Synthetic-new-456']])fireEvent.change(screen.getByLabelText(label),{target:{value}});
 fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Change password and lock'}));
 expect(screen.getByRole('button',{name:'Branding'})).toBeDisabled();fireEvent.click(screen.getByRole('tab',{name:'All clients'}));expect(screen.getByRole('heading',{name:'Change workspace password'})).toBeInTheDocument();
 await act(async()=>pending.resolve({durabilityWarning:true}));expect(await screen.findByRole('button',{name:'Unlock workspace'})).toBeInTheDocument();
 expect(screen.getByText(/Workspace password changed/)).toHaveTextContent('original passwords');expect(screen.getByText(/Workspace password changed/)).toHaveTextContent('Keep both passwords');
 expect(screen.queryByLabelText('New password')).not.toBeInTheDocument();expect(screen.queryByRole('button',{name:'Settings'})).not.toBeInTheDocument();
});

it('clears private UI on password-change lock notification before the command response arrives',async()=>{
 const pending=deferred<{durabilityWarning:boolean}>();mocks.changeWorkspacePassword.mockReturnValueOnce(pending.promise);
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Security'}));await screen.findByLabelText('Inactivity timeout (minutes)');
 for(const [label,value] of [['Current password','Synthetic-original-123'],['New password','Synthetic-new-456'],['Confirm new password','Synthetic-new-456']])fireEvent.change(screen.getByLabelText(label),{target:{value}});
 fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Change password and lock'}));
 act(()=>{(mocks.lockHandler as unknown as (event:{payload:unknown})=>void)({payload:{reason:'password_changed',durabilityWarning:true}});});
 expect(await screen.findByRole('button',{name:'Unlock workspace'})).toBeInTheDocument();expect(screen.queryByLabelText('New password')).not.toBeInTheDocument();expect(screen.queryByRole('button',{name:'Settings'})).not.toBeInTheDocument();expect(screen.getByText(/Workspace password changed/)).toHaveTextContent('Keep both passwords');
 await act(async()=>pending.resolve({durabilityWarning:false}));expect(screen.getByText(/Workspace password changed/)).toHaveTextContent('Keep both passwords');
});
it('preserves inactivity drafts when a sibling password form clears and guards other security controls while saving',async()=>{
 const pending=deferred<import('./types').SecurityPolicy>();mocks.saveSecurityPolicy.mockReturnValueOnce(pending.promise);
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Settings'}));fireEvent.click(screen.getByRole('button',{name:'Security'}));
 const minutes=await screen.findByLabelText('Inactivity timeout (minutes)');fireEvent.change(minutes,{target:{value:'5'}});fireEvent.change(screen.getByLabelText('New password'),{target:{value:'Synthetic-private-password-draft'}});fireEvent.click(screen.getByRole('button',{name:'Clear password fields'}));
 vi.mocked(window.confirm).mockReturnValueOnce(false);fireEvent.click(screen.getByRole('button',{name:'Branding'}));expect(screen.getByLabelText('Inactivity timeout (minutes)')).toHaveValue(5);
 fireEvent.click(screen.getByRole('button',{name:'Save inactivity timeout'}));expect(screen.getByRole('button',{name:'Change password and lock'})).toBeDisabled();fireEvent.click(screen.getByRole('tab',{name:'All clients'}));expect(screen.getByRole('heading',{name:'Inactivity locking'})).toBeInTheDocument();
 await act(async()=>pending.resolve({revision:1,idleMinutes:5,totalChanges:1,history:[{revision:1,idleMinutes:5,savedAt:'2026-10-16T13:00:00Z'}]}));expect(screen.getByRole('status')).toHaveTextContent('Inactivity timeout saved');
 act(()=>mocks.lockHandler?.());expect(screen.queryByLabelText('Inactivity timeout (minutes)')).not.toBeInTheDocument();expect(document.documentElement.outerHTML).not.toContain('Synthetic-private-password-draft');
});

it('opens one stable-ID tab per client, remembers each section and closes to the most recently used tab',async()=>{
 render(<App/>);await unlock();await selectClient('Alice Able','Session notes');await screen.findByLabelText('New session note');
 await selectClient('Bob Baker','Monthly budget');await screen.findByText('No saved budget yet');
 expect(screen.getByRole('tabpanel',{name:'Bob Baker'})).toBeInTheDocument();expect(screen.getAllByRole('tab')).toHaveLength(3);expect(screen.queryByLabelText('New session note')).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('tab',{name:'Alice Able'}));await screen.findByLabelText('New session note');expect(screen.queryByRole('button',{name:'Add income'})).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('tab',{name:'All clients'}));fireEvent.click(screen.getByRole('button',{name:'Open workspace for Alice Able'}));await screen.findByLabelText('New session note');expect(screen.getAllByRole('tab')).toHaveLength(3);
 fireEvent.click(screen.getByRole('button',{name:'Close workspace for Alice Able'}));await screen.findByText('No saved budget yet');await waitFor(()=>expect(screen.getByRole('tab',{name:'Bob Baker'})).toHaveFocus());
 fireEvent.click(screen.getByRole('button',{name:'Close workspace for Bob Baker'}));expect(screen.getByRole('region',{name:'Client directory'})).toBeInTheDocument();await waitFor(()=>expect(screen.getByRole('tab',{name:'All clients'})).toHaveFocus());expect(screen.getAllByRole('tab')).toHaveLength(1);
});
it('guards tab switching and closing dirty records, then discards only after explicit acceptance',async()=>{
 render(<App/>);await unlock();await selectClient('Bob Baker');await selectClient('Alice Able');fireEvent.change(screen.getByLabelText('Counseling goals'),{target:{value:'Private Alice tab draft'}});
 vi.mocked(window.confirm).mockReturnValue(false);fireEvent.click(screen.getByRole('tab',{name:'Bob Baker'}));fireEvent.click(screen.getByRole('button',{name:'Close workspace for Alice Able'}));expect(screen.getByLabelText('Counseling goals')).toHaveValue('Private Alice tab draft');expect(mocks.saveClient).not.toHaveBeenCalled();expect(window.confirm).toHaveBeenCalledTimes(2);
 vi.mocked(window.confirm).mockReturnValue(true);fireEvent.click(screen.getByRole('tab',{name:'Bob Baker'}));await screen.findByRole('heading',{name:'Bob Baker'});expect(screen.getByLabelText('Counseling goals')).toHaveValue(bob.goals);expect(document.documentElement.outerHTML).not.toContain('Private Alice tab draft');
 fireEvent.click(screen.getByRole('tab',{name:'Alice Able'}));await screen.findByRole('heading',{name:'Alice Able'});expect(screen.getByLabelText('Counseling goals')).toHaveValue(alice.goals);
 await act(async()=>mocks.lockHandler?.());expect(screen.queryByRole('tab')).not.toBeInTheDocument();expect(document.documentElement.outerHTML).not.toContain('Alice Able');
});
it('discards late overview replies and never mounts or loads another client’s inactive tools',async()=>{
 const pending=deferred<unknown>();mocks.getClientWorkspaceOverview.mockReturnValueOnce(pending.promise);
 render(<App/>);await unlock();await selectClient('Alice Able','');expect(mocks.getClientBudget).not.toHaveBeenCalled();expect(mocks.getClientImports).not.toHaveBeenCalled();
 await selectClient('Bob Baker','');await screen.findByRole('heading',{name:'Overview'});await act(async()=>pending.resolve({clientId:alice.id,monthlyIncome:'12345.67',warnings:['Private Alice delayed overview']}));expect(document.documentElement.outerHTML).not.toContain('Private Alice delayed overview');expect(screen.queryByText('$12,345.67')).not.toBeInTheDocument();
 fireEvent.keyDown(screen.getByRole('tab',{name:'Bob Baker'}),{key:'Home'});expect(screen.getByRole('region',{name:'Client directory'})).toBeInTheDocument();await waitFor(()=>expect(screen.getByRole('tab',{name:'All clients'})).toHaveFocus());
});
it('rejects a mismatched client response without creating a tab or showing its private fields',async()=>{
 mocks.getClient.mockResolvedValueOnce(bob);render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Clients'}));fireEvent.click(screen.getByRole('button',{name:'Open workspace for Alice Able'}));expect(await screen.findByRole('alert')).toHaveTextContent('did not match');expect(screen.queryByRole('tab',{name:'Alice Able'})).not.toBeInTheDocument();expect(screen.queryByLabelText('Counseling goals')).not.toBeInTheDocument();expect(mocks.getClientWorkspaceOverview).not.toHaveBeenCalled();
});
it('keeps same-name clients in distinct ID-keyed tabs and retains identity after a rename',async()=>{
 const sameName={...bob,firstName:alice.firstName,lastName:alice.lastName};mocks.listClients.mockResolvedValue([alice,sameName]);mocks.getClient.mockImplementation(async(id:string)=>id===alice.id?alice:sameName);
 render(<App/>);await unlock();fireEvent.click(screen.getByRole('button',{name:'Clients'}));fireEvent.click(within(screen.getByText(alice.id).closest('tr')!).getByRole('button'));await screen.findByRole('heading',{name:'Alice Able'});
 fireEvent.click(screen.getByRole('tab',{name:'All clients'}));fireEvent.click(within(screen.getByText(bob.id).closest('tr')!).getByRole('button'));await screen.findByRole('tab',{name:`Alice Able · ${bob.id}`});expect(screen.getAllByRole('tab')).toHaveLength(3);expect(screen.getByRole('tab',{name:`Alice Able · ${alice.id}`})).toBeInTheDocument();
 go('Profile & intake');fireEvent.change(screen.getByLabelText(/Last name/),{target:{value:'Renamed'}});fireEvent.click(screen.getByRole('button',{name:'Save changes'}));expect(await screen.findByRole('tab',{name:'Alice Renamed'})).toHaveAttribute('data-client-tab',bob.id);expect(screen.getAllByRole('tab')).toHaveLength(3);
});
