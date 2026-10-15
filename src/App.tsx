import CardCompleteness from './CardCompleteness';
import CardFinancialReview from './CardFinancialReview';
import CardStatements from './CardStatements';
import CompletenessReview,{SavedReportChecklist} from './CompletenessReview';
import StatementAccounts from './StatementAccounts';
import DocumentCollection from './DocumentCollection';
import AdministrationPages from './AdministrationPages';
import PlannedWorkspaceArea,{plannedAreas,workingAreas} from './PlannedWorkspaceAreas';
import {workspacePages,clientSections,isModule,type View,type ClientSection} from './workspaceNavigation';
import FollowUpOverview from './FollowUpOverview';
import ClientCare from './ClientCare';
import ImportsEditor from './ImportsEditor';
import {RestorePanel} from './RecoveryPanel';
import CashflowEditor from './CashflowEditor';
import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react';
import SavingsEditor from './SavingsEditor';
import DebtEditor from './DebtEditor';
import BudgetEditor from './BudgetEditor';
import ScenarioComparisons from './ScenarioComparisons';
import { listen } from '@tauri-apps/api/event';
import { api, desktopAvailable, resetApiSession } from './api';
import { CLIENT_STATUSES, type Client, type ClientInput, type ClientNote, type SetupInput, type RestoreResult, type WorkspaceInfo } from './types';

type Phase = 'checking' | 'setup' | 'locked' | 'unlocked' | 'error';

function Icon({ name, size = 20 }: { name: string; size?: number }) {
  const paths: Record<string, ReactNode> = {
    dashboard: <><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></>,
    clients: <><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M22 21v-2a4 4 0 0 0-3-3.87" /><circle cx="9" cy="7" r="4" /><path d="M16 3.13a4 4 0 0 1 0 7.75" /></>,
    settings: <><path d="M12 3v3m0 12v3M3 12h3m12 0h3M5.6 5.6l2.1 2.1m8.6 8.6 2.1 2.1M5.6 18.4l2.1-2.1m8.6-8.6 2.1-2.1" /><circle cx="12" cy="12" r="5" /></>,
    review: <><path d="M9 3H5v18h14V3h-4M9 3v4h6V3Z"/><path d="m8 14 2 2 5-5"/></>,
    transactions: <><path d="M3 7h17m-4-4 4 4-4 4M21 17H4m4-4-4 4 4 4"/></>,
    budget: <><rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 7h8M8 12h2m4 0h2M8 17h2m4 0h2"/></>,
    debt: <><path d="M4 4v16h16M7 8l4 4 4-2 5 7"/></>,
    savings: <><circle cx="12" cy="12" r="9"/><path d="M15 8h-4a2 2 0 0 0 0 4h2a2 2 0 0 1 0 4H9m3-10v12"/></>,
    calendar: <><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M7 3v4m10-4v4M3 10h18m-13 4h3m3 0h3m-9 4h3"/></>,
    scenarios: <><path d="M4 3v18m0-10h8l7-7M12 11l7 7m-4-14h4v4m-4 10h4v-4"/></>,
    lock: <><rect x="5" y="10" width="14" height="11" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /><path d="M12 14v3" /></>,
    plus: <path d="M12 5v14M5 12h14" />,
    search: <><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 4.5 4.5" /></>,
    arrow: <path d="m9 5 7 7-7 7" />,
    check: <path d="m5 12 4 4L19 6" />,
    file: <><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z" /><path d="M14 2v6h6M8 13h8M8 17h5" /></>,
    shield: <><path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6Z" /><path d="m8 12 3 3 5-6" /></>,
  };
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">{paths[name] ?? paths.file}</svg>;
}

function message(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'Something went wrong. Please try again.';
}

function dateLabel(value: string, includeTime = false): string {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return value;
  return date.toLocaleString(undefined, includeTime
    ? { dateStyle: 'medium', timeStyle: 'short' }
    : { dateStyle: 'medium' });
}

function ErrorMessage({ children }: { children: ReactNode }) {
  return children ? <div className="alert error" role="alert">{children}</div> : null;
}

function Brand({ compact = false }: { compact?: boolean }) {
  return <div className={`brand ${compact ? 'compact' : ''}`}>
    <div className="brand-mark" aria-hidden="true">E<span>&</span></div>
    <div><strong>Eisenbart & Co.</strong><span>FINANCIAL COUNSELING</span></div>
  </div>;
}

function DevelopmentNotice() {
  return <div className="development-notice"><span className="development-dot" aria-hidden="true" /><strong>Development build</strong><span>Use fictional records only. This version is not ready for confidential client information.</span></div>;
}

function AuthForm({ setup, busy, error, notice, onSubmit, onRestored }: {
  setup: boolean; busy: boolean; error: string; notice: string; onRestored:(result:RestoreResult)=>void;
  onSubmit: (input: SetupInput) => Promise<void>;
}) {
  const [organizationName, setOrganizationName] = useState('Eisenbart & Co.');
  const [counselorName, setCounselorName] = useState('');
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [validation, setValidation] = useState('');

  async function submit(event: FormEvent) {
    event.preventDefault();
    setValidation('');
    if (setup && (!organizationName.trim() || !counselorName.trim())) {
      setValidation('Enter your organization and counselor names.');
      return;
    }
    if (setup && password.length < 12) {
      setValidation('Choose a password with at least 12 characters.');
      return;
    }
    if (!password) {
      setValidation('Enter your workspace password.');
      return;
    }
    if (setup && password !== confirmation) {
      setValidation('The passwords do not match. Enter them again.');
      return;
    }
    const input = { organizationName: organizationName.trim(), counselorName: counselorName.trim(), password };
    setPassword('');
    setConfirmation('');
    await onSubmit(input);
  }

  return <div className="auth-layout">
    <div className="auth-intro">
      <Brand />
      <div className="auth-copy"><p className="eyebrow">A CLEARER PATH FORWARD</p><h1>A thoughtful workspace<br />for financial counseling.</h1><p>Keep client intake, counseling goals, and session notes together in your desktop workspace.</p><div className="local-label"><Icon name="shield" /> Local storage · Works offline</div></div>
      <p className="auth-footer">Eisenbart & Co. Financial Counseling Suite</p>
    </div>
    <main className="auth-main">
      <div className="auth-card">
        <div className="auth-icon"><Icon name="lock" size={25} /></div>
        <p className="eyebrow">{setup ? 'GETTING STARTED' : 'YOUR LOCAL WORKSPACE'}</p>
        <h2>{setup ? 'Set up your workspace' : 'Welcome back'}</h2>
        <p className="muted">{setup ? 'Create a workspace on this computer and choose a password to protect access.' : 'Enter your password to unlock the workspace on this computer.'}</p>
        {notice && <p className="success-message" role="status">{notice}</p>}
        <form onSubmit={submit} noValidate>
          <fieldset disabled={busy}>
            {setup && <>
              <label htmlFor="organization">Organization name<input id="organization" value={organizationName} onChange={e => setOrganizationName(e.target.value)} maxLength={120} autoComplete="organization" required /></label>
              <label htmlFor="counselor">Counselor name<input id="counselor" value={counselorName} onChange={e => setCounselorName(e.target.value)} maxLength={120} autoComplete="name" required /></label>
            </>}
            <label htmlFor="password">Workspace password<input id="password" type="password" value={password} onChange={e => setPassword(e.target.value)} maxLength={1024} autoComplete={setup ? 'new-password' : 'current-password'} aria-describedby={setup ? 'password-help' : undefined} required /></label>
            {setup && <><p className="field-help" id="password-help">Use at least 12 characters. Keep your password somewhere safe; password recovery is not available in this milestone.</p><label htmlFor="confirm-password">Confirm password<input id="confirm-password" type="password" value={confirmation} onChange={e => setConfirmation(e.target.value)} maxLength={1024} autoComplete="new-password" required /></label></>}
            <ErrorMessage>{validation || error}</ErrorMessage>
            <button type="submit" className="button primary auth-submit">{busy ? (setup ? 'Creating workspace…' : 'Unlocking…') : (setup ? 'Create workspace' : 'Unlock workspace')}<Icon name="arrow" size={17} /></button>
          </fieldset>
        </form>
        <RestorePanel onRestored={onRestored}/>
        <p className="auth-storage"><Icon name="lock" size={14} />Your workspace data stays on this computer.</p>
      </div>
      <DevelopmentNotice />
    </main>
  </div>;
}

const emptyClient = (): ClientInput => ({ firstName: '', lastName: '', email: '', phone: '', householdSize: 1, status: 'New Intake', goals: '' });

function clientInput(client: Client): ClientInput {
  return { id: client.id, firstName: client.firstName, lastName: client.lastName, email: client.email, phone: client.phone, householdSize: client.householdSize, status: client.status, goals: client.goals };
}

function ClientEditor({ client, workspace, onSaved, onCancel, onDirty, onBusy, followUpId, section='record', onNavigate }: {
  client: Client | null; workspace: WorkspaceInfo; followUpId?:string; section?:ClientSection; onNavigate:(view:View)=>void;
  onSaved: (client: Client) => void; onCancel: () => void;
  onDirty: (dirty: boolean) => void; onBusy: (busy: boolean) => void;
}) {
  const [draft, setDraft] = useState<ClientInput>(() => client ? clientInput(client) : emptyClient());
  const [baseline, setBaseline] = useState<ClientInput>(() => client ? clientInput(client) : emptyClient());
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);
  const [scenarioBusy,setScenarioBusy]=useState(false);
  const [careBusy,setCareBusy]=useState(false),[careDirty,setCareDirty]=useState(false);
  const [noteSaving, setNoteSaving] = useState(false);
  const [noteDirty, setNoteDirty] = useState(false);
  const [budgetDirty, setBudgetDirty] = useState(false);
  const [budgetSaving, setBudgetSaving] = useState(false);
  const [debtSaving, setDebtSaving] = useState(false);
  const [debtDirty, setDebtDirty] = useState(false);
  const [importsDirty,setImportsDirty]=useState(false);
  const [importsBusy,setImportsBusy]=useState(false);
  const [cashflowDirty,setCashflowDirty]=useState(false);
  const [cashflowSaving,setCashflowSaving]=useState(false);
  const [savingsDirty,setSavingsDirty]=useState(false);
  const [savingsSaving,setSavingsSaving]=useState(false);
  const [saved, setSaved] = useState(false);
  const mounted = useRef(true);
  const dirty = JSON.stringify(draft) !== JSON.stringify(baseline) || noteDirty || budgetDirty || debtDirty || savingsDirty || cashflowDirty || importsDirty || careDirty;
  useEffect(() => { onDirty(dirty); }, [dirty, onDirty]);
  useEffect(() => { onBusy(saving || noteSaving || budgetSaving || debtSaving || savingsSaving || cashflowSaving || importsBusy || scenarioBusy || careBusy); }, [saving, noteSaving, budgetSaving, debtSaving, savingsSaving, cashflowSaving, importsBusy, scenarioBusy, careBusy, onBusy]);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; onDirty(false); onBusy(false); };
  }, [onBusy, onDirty]);

  function update<K extends keyof ClientInput>(field: K, value: ClientInput[K]) {
    setDraft(previous => ({ ...previous, [field]: value }));
    setSaved(false);
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    setError('');
    const input = { ...draft, firstName: draft.firstName.trim(), lastName: draft.lastName.trim(), email: draft.email.trim(), phone: draft.phone.trim(), goals: draft.goals.trim() };
    if (!input.firstName || !input.lastName) { setError('Enter a first and last name.'); return; }
    if (input.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(input.email)) { setError('Enter a valid email address, or leave it blank.'); return; }
    if (!Number.isInteger(input.householdSize) || input.householdSize < 1 || input.householdSize > 30) { setError('Household size must be a whole number from 1 to 30.'); return; }
    setSaving(true);
    try {
      const result = await api.saveClient(input);
      if (!mounted.current) return;
      const next = clientInput(result);
      setDraft(next);
      setBaseline(next);
      setSaved(true);
      onSaved(result);
    } catch (caught) {
      if (mounted.current) setError(message(caught));
    } finally {
      if (mounted.current) setSaving(false);
    }
  }

  const shows=(...sections:ClientSection[])=>section==='record'||sections.includes(section);
  return <div className="client-editor">
    {shows('profile')&&<div className="panel editor-panel">
      <div className="panel-heading"><div><p className="eyebrow">{client ? 'CLIENT RECORD' : 'CLIENT INTAKE'}</p><h2>{client ? `${client.firstName} ${client.lastName}` : 'Add a new client'}</h2></div>{client && <Status status={client.status} />}</div>
      <form onSubmit={save} noValidate>
        <fieldset disabled={saving || noteSaving || budgetSaving || debtSaving || savingsSaving || cashflowSaving || importsBusy || scenarioBusy || careBusy}>
          <div className="form-section"><h3>Personal information</h3><p className="muted small">Start with the essentials. You can add details as counseling progresses.</p></div>
          <div className="form-grid">
            <label htmlFor="first-name">First name <span className="required">*</span><input id="first-name" value={draft.firstName} onChange={e => update('firstName', e.target.value)} maxLength={100} autoComplete="given-name" required /></label>
            <label htmlFor="last-name">Last name <span className="required">*</span><input id="last-name" value={draft.lastName} onChange={e => update('lastName', e.target.value)} maxLength={100} autoComplete="family-name" required /></label>
            <label htmlFor="email">Email address <span className="optional">Optional</span><input id="email" type="email" value={draft.email} onChange={e => update('email', e.target.value)} maxLength={254} autoComplete="email" /></label>
            <label htmlFor="phone">Phone number <span className="optional">Optional</span><input id="phone" type="tel" value={draft.phone} onChange={e => update('phone', e.target.value)} maxLength={40} autoComplete="tel" /></label>
            <label htmlFor="household-size">Household size<input id="household-size" type="number" min={1} max={30} step={1} value={Number.isNaN(draft.householdSize) ? '' : draft.householdSize} onChange={e => update('householdSize', e.target.valueAsNumber)} required /></label>
            <label htmlFor="client-status">Counseling status<select id="client-status" value={draft.status} onChange={e => update('status', e.target.value as ClientInput['status'])}>{CLIENT_STATUSES.map(status => <option key={status}>{status}</option>)}</select></label>
          </div>
          <div className="form-section goals-heading"><h3>Counseling goals</h3><p className="muted small">What would this client like to work toward?</p></div>
          <label htmlFor="goals" className="sr-only">Counseling goals</label><textarea id="goals" value={draft.goals} onChange={e => update('goals', e.target.value)} maxLength={4000} rows={4} placeholder="Record the client's priorities and goals…" />
          <div className="field-footer"><span>Required fields are marked *</span><span>{draft.goals.length.toLocaleString()} / 4,000</span></div>
          <ErrorMessage>{error}</ErrorMessage>
          <div className="form-actions"><div className="save-status" role="status">{saved ? <><Icon name="check" size={16} />Client saved</> : dirty ? 'Unsaved changes' : client ? `Updated ${dateLabel(client.updatedAt)}` : ''}</div><button className="button secondary" type="button" onClick={onCancel}>Close</button><button className="button primary" type="submit">{saving ? 'Saving…' : client ? 'Save changes' : 'Create client'}</button></div>
        </fieldset>
      </form>
    </div>}
    {client && shows('intake') && <ClientCare key={client.id} clientId={client.id} focusFollowUp={followUpId} onDirty={setCareDirty} onBusy={setCareBusy} otherSaving={saving || noteSaving || budgetSaving || debtSaving || savingsSaving || cashflowSaving || importsBusy || scenarioBusy} />}
    {shows('notes')&&(client ? <ClientNotes key={client.id} client={client} workspace={workspace} onBusy={setNoteSaving} onDirty={setNoteDirty} clientSaving={saving || budgetSaving || debtSaving || savingsSaving || cashflowSaving || importsBusy || scenarioBusy || careBusy} /> : <div className="panel notes-placeholder"><Icon name="file" size={25} /><div><h3>Session notes</h3><p className="muted small">Save this client first to start a dated counseling history.</p></div></div>)}
    {client && shows('Budget Planner') && <BudgetEditor key={client.id} client={client} onDirty={setBudgetDirty} onBusy={setBudgetSaving} otherSaving={saving || noteSaving || debtSaving || savingsSaving || cashflowSaving || importsBusy || scenarioBusy || careBusy} />}
    {client && shows('Debt Planner') && <DebtEditor key={client.id} client={client} onDirty={setDebtDirty} onBusy={setDebtSaving} otherSaving={saving || noteSaving || budgetSaving || savingsSaving || cashflowSaving || importsBusy || scenarioBusy || careBusy} />}
    {client && shows('Savings Planner') && <SavingsEditor key={client.id} client={client} onDirty={setSavingsDirty} onBusy={setSavingsSaving} otherSaving={saving || noteSaving || budgetSaving || debtSaving || cashflowSaving || importsBusy || scenarioBusy || careBusy} />}
    {client && shows('Cash-Flow Calendar') && <CashflowEditor key={client.id} client={client} onDirty={setCashflowDirty} onBusy={setCashflowSaving} otherSaving={saving || noteSaving || budgetSaving || debtSaving || savingsSaving || importsBusy || scenarioBusy || careBusy} />}
    {client && shows('Financial Scenarios') && <ScenarioComparisons key={client.id} clientId={client.id} disabled={dirty || saving || noteSaving || budgetSaving || debtSaving || savingsSaving || cashflowSaving || importsBusy || careBusy} onBusy={setScenarioBusy} />}
    {client && shows('Documents','Transactions','Financial Review','Reports') && <ImportsEditor key={client.id} page={section==='Documents'?'documents':section==='Transactions'?'transactions':section==='Financial Review'?'review':section==='Reports'?'reports':'all'} client={client} onDirty={setImportsDirty} onBusy={setImportsBusy} otherSaving={saving || noteSaving || budgetSaving || debtSaving || savingsSaving || cashflowSaving || scenarioBusy || careBusy} />}
    {client&&section==='history'&&<section className="panel settings-panel"><h2>Client history and records</h2><dl className="details-list"><div><dt>Created</dt><dd>{dateLabel(client.createdAt,true)}</dd></div><div><dt>Profile last updated</dt><dd>{dateLabel(client.updatedAt,true)}</dd></div><div><dt>Current stage</dt><dd>{client.status}</dd></div></dl><p className="muted small">Session notes, review decisions, scenarios and reports retain their own dated history.</p><div className="page-shortcuts"><button className="button secondary" onClick={()=>onNavigate('Transactions')}>Open transaction decisions</button><button className="button secondary" onClick={()=>onNavigate('Financial Scenarios')}>Open saved scenarios</button><button className="button secondary" onClick={()=>onNavigate('Reports')}>Open report and action-plan history</button></div><p className="field-help">A combined client activity timeline is planned. Archive or reactivate this client through Counseling status in Profile; permanent deletion is not available.</p></section>}
  </div>;
}

function ClientNotes({ client, workspace, onBusy, onDirty, clientSaving }: { client: Client; workspace: WorkspaceInfo; onBusy: (busy: boolean) => void; onDirty: (dirty: boolean) => void; clientSaving: boolean }) {
  const [notes, setNotes] = useState<ClientNote[]>([]);
  const [body, setBody] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [loadError, setLoadError] = useState('');
  const [attempt, setAttempt] = useState(0);
  const mounted = useRef(true);
  useEffect(() => { onDirty(body.trim().length > 0); }, [body, onDirty]);
  useEffect(() => {
    mounted.current = true;
    let current = true;
    setNotes([]);
    setBody('');
    setLoading(true);
    setLoadError('');
    api.listClientNotes(client.id).then(result => {
      if (current) setNotes(result);
    }).catch(caught => {
      if (current) setLoadError(message(caught));
    }).finally(() => { if (current) setLoading(false); });
    return () => { current = false; mounted.current = false; };
  }, [client.id, attempt]);

  async function add(event: FormEvent) {
    event.preventDefault();
    setError('');
    if (!body.trim()) { setError('Write a note before saving.'); return; }
    setSaving(true);
    onBusy(true);
    try {
      const note = await api.addClientNote(client.id, body.trim());
      if (!mounted.current) return;
      setNotes(previous => [note, ...previous]);
      setBody('');
    } catch (caught) {
      if (mounted.current) setError(message(caught));
    } finally {
      if (mounted.current) { setSaving(false); onBusy(false); }
    }
  }

  return <section className="panel notes-panel" aria-labelledby="notes-title">
    <div className="panel-heading"><div><p className="eyebrow">COUNSELING HISTORY</p><h2 id="notes-title">Session notes</h2></div><span className="count-badge">{notes.length}</span></div>
    <p className="muted small">Saved notes retain their author and date. Add a new note to correct or update a previous entry.</p>
    <form onSubmit={add} noValidate>
      <label htmlFor="new-note">New session note<textarea id="new-note" rows={4} maxLength={8000} value={body} onChange={e => setBody(e.target.value)} placeholder="Record the discussion, decisions, and next steps…" disabled={saving || clientSaving || loading || !!loadError} /></label>
      <div className="field-footer"><span>Author: {workspace.counselorName}</span><span>{body.length.toLocaleString()} / 8,000</span></div>
      <ErrorMessage>{error}</ErrorMessage>
      <div className="note-actions"><button type="submit" className="button primary" disabled={saving || clientSaving || loading || !!loadError}>{saving ? 'Saving note…' : 'Save session note'}</button></div>
    </form>
    <div className="notes-history">
      {loading ? <p className="muted" role="status">Loading session notes…</p> : loadError ? <><ErrorMessage>{loadError}</ErrorMessage><button className="button secondary" onClick={() => setAttempt(previous => previous + 1)}>Retry loading notes</button></> : notes.length === 0 ? <p className="empty-note">No session notes yet. Your first saved note will appear here.</p> : notes.map(note => <article className="session-note" key={note.id}><div className="note-meta"><strong>{note.counselorName}</strong><time dateTime={note.createdAt}>{dateLabel(note.createdAt, true)}</time></div><p>{note.body}</p></article>)}
    </div>
  </section>;
}

function Status({ status }: { status: string }) {
  return <span className={`status-tag ${status === 'Archived' ? 'archived' : status === 'New Intake' ? 'intake' : ''}`}><span aria-hidden="true" />{status}</span>;
}

function Initials({ client }: { client: Client }) {
  return <span className="avatar" aria-hidden="true">{client.firstName.slice(0, 1)}{client.lastName.slice(0, 1)}</span>;
}

export default function App() {
  const [phase, setPhase] = useState<Phase>('checking');
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [authBusy, setAuthBusy] = useState(false);
  const [authError, setAuthError] = useState('');
  const [recoveryNotice,setRecoveryNotice]=useState('');
  const [statusAttempt, setStatusAttempt] = useState(0);
  const [view, setView] = useState<View>('Dashboard');
  const [clientSection,setClientSection]=useState<ClientSection>('record');
  const [moduleArea,setModuleArea]=useState('Working tools');
  const [clients, setClients] = useState<Client[]>([]);
  const [clientsLoading, setClientsLoading] = useState(false);
  const [clientsError, setClientsError] = useState('');
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState('All statuses');
  const [selected, setSelected] = useState<Client | null>(null);
  const [targetFollowUp,setTargetFollowUp]=useState<string|undefined>(undefined);
  const [editing, setEditing] = useState(false);
  const [selectionLoading, setSelectionLoading] = useState(false);
  const [selectionError, setSelectionError] = useState('');
  const pageHeading=useRef<HTMLHeadingElement>(null);
  useEffect(()=>{if(phase==='unlocked'){pageHeading.current?.focus({preventScroll:true});pageHeading.current?.scrollIntoView?.({block:'start'});}},[view,clientSection,moduleArea,phase]);
  const session = useRef(0);
  const selectionRequest = useRef(0);
  const dirty = useRef(false);
  const editorBusy = useRef(false);
  const onDirty = useRef((value: boolean) => { dirty.current = value; }).current;
  const onBusy = useRef((value: boolean) => { editorBusy.current = value; }).current;

  useEffect(() => {
    if (!desktopAvailable()) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const expire = () => {
      clearWorkspace();
      setPhase('locked');
      setAuthError('');
    };
    window.addEventListener('workspace-access-expired', expire);
    listen('workspace-locked', expire).then(remove => { if (cancelled) remove(); else unlisten = remove; }).catch(() => {
      // The command layer still enforces locking if event delivery is unavailable.
    });
    return () => { cancelled = true; unlisten?.(); window.removeEventListener('workspace-access-expired', expire); };
  }, []);

  useEffect(() => {
    if (phase !== 'unlocked') return;
    let lastActivity = Date.now();
    const activity = () => {
      if (Date.now() - lastActivity < 30_000) return;
      lastActivity = Date.now();
      api.workspaceInfo().catch(() => {
        // The API wrapper clears expired access using the request's session epoch.
      });
    };
    window.addEventListener('pointerdown', activity);
    window.addEventListener('keydown', activity);
    return () => {
      window.removeEventListener('pointerdown', activity);
      window.removeEventListener('keydown', activity);
    };
  }, [phase]);

  useEffect(() => {
    if (!desktopAvailable()) return;
    let current = true;
    setPhase('checking');
    setAuthError('');
    api.setupStatus().then(result => {
      if (current) setPhase(result.configured ? 'locked' : 'setup');
    }).catch(caught => {
      if (current) { setAuthError(message(caught)); setPhase('error'); }
    });
    return () => { current = false; };
  }, [statusAttempt]);

  function clearWorkspace() {
    resetApiSession();
    session.current += 1;
    selectionRequest.current += 1;
    setWorkspace(null);
    setClients([]);
    setClientsError('');
    setClientsLoading(false);
    setSelected(null);
    setTargetFollowUp(undefined);
    setEditing(false);
    setSelectionLoading(false);
    setSelectionError('');
    setSearch('');
    setStatusFilter('All statuses');
    setView('Dashboard');
    setClientSection('record');setModuleArea('Working tools');
    dirty.current = false;
    editorBusy.current = false;
  }

  async function loadClients() {
    const currentSession = session.current;
    setClientsLoading(true);
    setClientsError('');
    try {
      const result = await api.listClients();
      if (currentSession === session.current) setClients(result);
    } catch (caught) {
      if (currentSession === session.current) setClientsError(message(caught));
    } finally {
      if (currentSession === session.current) setClientsLoading(false);
    }
  }

  async function authenticate(input: SetupInput) {
    const configuring = phase === 'setup';
    setAuthBusy(true);
    setAuthError('');
    try {
      const result = configuring ? await api.setupWorkspace(input) : await api.unlockWorkspace(input.password);
      clearWorkspace();
      setWorkspace(result);
      setPhase('unlocked');
      await loadClients();
    } catch (caught) {
      setAuthError(message(caught));
    } finally {
      setAuthBusy(false);
    }
  }

  function restored(result:RestoreResult) {
    clearWorkspace();setAuthError('');setAuthBusy(false);setPhase('locked');
    setRecoveryNotice(`Workspace restored. Unlock it using the password from this backup.${result.durabilityWarning?' The disk did not confirm directory synchronization; keep your backup available.':''}`);
  }

  function canLeave(): boolean {
    if (editorBusy.current) return false;
    return !dirty.current || window.confirm('You have unsaved client changes. Discard them and continue?');
  }

  function clearSelection() {
    selectionRequest.current += 1;
    setSelected(null);
    setTargetFollowUp(undefined);
    setEditing(false);
    setSelectionLoading(false);
    setSelectionError('');
    dirty.current = false;
  }

  function navigate(next: View) {
    if (view === next || !canLeave()) return;
    selectionRequest.current++;setSelectionLoading(false);setSelectionError('');
    setEditing(Boolean(selected));setTargetFollowUp(undefined);
    dirty.current=false;setModuleArea('Working tools');
    setView(next);
  }

  function createClient() {
    if (!canLeave()) return;
    clearSelection();
    setView('Clients');setClientSection('record');
    setEditing(true);
  }

  async function openClient(id: string, followUpId?:string, targetView:View='Clients') {
    if (!canLeave()) return;
    clearSelection();
    const request = selectionRequest.current;
    const currentSession = session.current;
    setView(targetView);if(targetView!==view)setModuleArea('Working tools');
    if(followUpId)setClientSection('record');
    setSelectionLoading(true);
    try {
      const result = await api.getClient(id);
      if (request === selectionRequest.current && currentSession === session.current) {
        if(result.id!==id)throw new Error('The client response did not match the selected record.');
        setTargetFollowUp(followUpId);
        setSelected(result);
        setEditing(true);
      }
    } catch (caught) {
      if (request === selectionRequest.current && currentSession === session.current) setSelectionError(message(caught));
    } finally {
      if (request === selectionRequest.current && currentSession === session.current) setSelectionLoading(false);
    }
  }

  async function lock() {
    if (!canLeave()) return;
    clearWorkspace();
    setPhase('locked');
    setAuthBusy(true);
    setAuthError('');
    try { await api.lockWorkspace(); } catch (caught) { setAuthError(message(caught)); } finally { setAuthBusy(false); }
  }

  function savedClient(client: Client) {
    setClients(previous => [client, ...previous.filter(item => item.id !== client.id)].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)));
    setSelected(client);
    setEditing(true);
  }

  if (!desktopAvailable()) return <main className="standalone"><Brand /><div className="panel standalone-panel"><Icon name="lock" size={32} /><h1>Open the desktop application</h1><p className="muted">This workspace runs inside the Eisenbart & Co. desktop application. A browser preview cannot access your local database.</p></div><DevelopmentNotice /></main>;
  if (phase === 'checking' || phase === 'error') return <main className="standalone"><Brand /><div className="panel standalone-panel"><h1>{phase === 'checking' ? 'Opening your workspace…' : 'Unable to open the workspace'}</h1>{phase === 'checking' ? <p className="muted" role="status">Checking local storage.</p> : <><ErrorMessage>{authError}</ErrorMessage><button className="button primary" onClick={() => setStatusAttempt(previous => previous + 1)}>Try again</button><RestorePanel onRestored={restored}/></>}</div></main>;
  if (phase !== 'unlocked' || !workspace) return <AuthForm key={phase} setup={phase === 'setup'} busy={authBusy} error={authError} notice={recoveryNotice} onSubmit={authenticate} onRestored={restored} />;

  const activeClients = clients.filter(client => client.status !== 'Archived');
  const newIntakes = clients.filter(client => client.status === 'New Intake');
  const followUps = clients.filter(client => client.status === 'Follow-Up');
  const recentClients = [...clients].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 5);
  const query = search.trim().toLocaleLowerCase();
  const visibleClients = clients.filter(client => (statusFilter === 'All statuses' || client.status === statusFilter) && `${client.firstName} ${client.lastName} ${client.email} ${client.phone}`.toLocaleLowerCase().includes(query));

  return <div className="app-shell">
    <aside className="sidebar">
      <Brand compact />
      <div className="workspace-label"><span>WORKSPACE</span><strong title={workspace.organizationName}>{workspace.organizationName}</strong></div>
      <nav aria-label="Main navigation">{workspacePages.map(item=><button key={item.id} title={item.title} className={`nav-item ${view===item.id?'active':''}`} aria-label={item.id} aria-current={view===item.id?'page':undefined} onClick={()=>navigate(item.id)}><Icon name={item.icon}/><span>{item.title}</span>{item.id==='Clients'&&<span className="nav-count">{clients.length}</span>}</button>)}</nav>
      <div className="sidebar-bottom"><div className="local-indicator"><span aria-hidden="true" />Local workspace</div><div className="counselor"><div className="counselor-avatar" aria-hidden="true">{workspace.counselorName.slice(0, 1)}</div><div><strong title={workspace.counselorName}>{workspace.counselorName}</strong><span>Counselor</span></div></div><button className="lock-button" onClick={lock}><Icon name="lock" size={16} />Lock workspace</button></div>
    </aside>
    <div className="main-shell">
      <header className="topbar"><div className="breadcrumbs">Workspace<Icon name="arrow" size={13} /><strong>{view}</strong></div><div className="offline-status"><Icon name="shield" size={15} />Offline-first · Local storage</div></header>
      <DevelopmentNotice />
      <main className="main-content">
        <div className="page-heading"><div><p className="eyebrow">{view==='Dashboard'?'YOUR COUNSELING WORKSPACE':view==='Settings'?'WORKSPACE ADMINISTRATION':'CLIENT WORKSPACE'}</p><h1 ref={pageHeading} tabIndex={-1}>{view==='Dashboard'?'A clear view of your practice.':workspacePages.find(p=>p.id===view)?.title}</h1><p className="muted">{workspacePages.find(p=>p.id===view)?.description}</p></div>{view!=='Settings'&&<button className="button primary" onClick={createClient} disabled={clientsLoading}><Icon name="plus" size={18}/>Add client</button>}</div>
        {clientsError && <div className="load-error"><ErrorMessage>{clientsError}</ErrorMessage><button className="button secondary" onClick={loadClients} disabled={clientsLoading}>{clientsLoading ? 'Loading…' : 'Retry loading clients'}</button></div>}
        {view === 'Dashboard' && <>
          <div className="stats-grid">{[{ label: 'Active clients', value: activeClients.length, caption: 'Clients currently in your care', icon: 'clients' }, { label: 'New intakes', value: newIntakes.length, caption: 'At the start of their journey', icon: 'file' }, { label: 'Follow-up stage', value: followUps.length, caption: 'Clients marked for follow-up', icon: 'check' }].map(stat => <div className="stat-card" key={stat.label}><div className="stat-header"><span>{stat.label}</span><div className="stat-icon"><Icon name={stat.icon} size={18} /></div></div><strong className="stat-value">{clientsLoading || clientsError ? '—' : stat.value}</strong><p>{stat.caption}</p></div>)}</div>
          <FollowUpOverview organizationId={workspace.organizationId} counselorId={workspace.counselorId} onOpen={(id,follow)=>void openClient(id,follow)}/>
          <div className="dashboard-grid"><section className="panel recent-panel"><div className="panel-heading"><div><h2>Recent client activity</h2><p className="muted small">The most recently updated client records.</p></div><button className="text-button" onClick={() => navigate('Clients')}>View all<Icon name="arrow" size={15} /></button></div>{clientsLoading ? <div className="empty-state" role="status">Loading client records…</div> : clientsError ? <div className="empty-state">Client records could not be loaded.</div> : recentClients.length ? <div className="recent-list">{recentClients.map(client => <button className="recent-row" key={client.id} onClick={() => openClient(client.id)}><Initials client={client} /><div className="recent-name"><strong>{client.firstName} {client.lastName}</strong><span>Updated {dateLabel(client.updatedAt)}</span></div><Status status={client.status} /><Icon name="arrow" size={16} /></button>)}</div> : <div className="empty-state"><div className="empty-icon"><Icon name="clients" size={30} /></div><h3>Every journey starts with a conversation.</h3><p>Add your first fictional client to begin organizing intake and counseling notes.</p><button className="button secondary" onClick={createClient}><Icon name="plus" size={16} />Add your first client</button></div>}</section>
          <aside className="practice-card"><div className="practice-icon"><Icon name="shield" size={24} /></div><p className="eyebrow">BUILT FOR FOCUSED WORK</p><h2>Your workspace.<br />On your computer.</h2><p>Client records and notes are saved locally, so you can continue your work without an internet connection.</p><div className="practice-divider" /><h3>A working foundation</h3><p>Client intake, counseling status, notes, monthly budgets, debt payoff comparisons, savings goals, cash-flow calendars, CSV and selectable-text PDF transaction review, merchant/recurring-payment review, and statement running-balance checks are available.</p><button className="text-button" onClick={() => navigate('Settings')}>View workspace details<Icon name="arrow" size={15} /></button></aside></div>
          <section className="panel progress-panel"><div><h2>Counseling progress</h2><p className="muted small">Client counts by their current counseling stage.</p></div><div className="stage-grid">{CLIENT_STATUSES.filter(status => status !== 'Archived').map(status => <div className="stage-item" key={status}><span>{status}</span><strong>{clientsLoading || clientsError ? '—' : clients.filter(client => client.status === status).length}</strong></div>)}</div></section>
        </>}
        {view === 'Clients' && <><nav className="section-navigation" aria-label="Client record sections">{clientSections.map(([id,label])=><button key={id} type="button" aria-current={clientSection===id?'page':undefined} onClick={()=>{if(clientSection!==id&&canLeave()){setClientSection(id);dirty.current=false;}}}>{label}</button>)}</nav><div className={`clients-layout ${editing || selectionLoading ? 'with-editor' : ''}`}>
          <section className="panel client-list-panel" aria-label="Client directory"><div className="directory-heading"><h2>Client directory <span className="count-badge">{clients.length}</span></h2></div><div className="directory-controls"><div className="search-input"><Icon name="search" size={17} /><label htmlFor="search-clients" className="sr-only">Search clients</label><input id="search-clients" type="search" placeholder="Search name, email, or phone" value={search} onChange={e => setSearch(e.target.value)} /></div><label htmlFor="filter-status" className="sr-only">Filter by status</label><select id="filter-status" value={statusFilter} onChange={e => setStatusFilter(e.target.value)}><option>All statuses</option>{CLIENT_STATUSES.map(status => <option key={status}>{status}</option>)}</select></div>
            {clientsLoading ? <div className="empty-state" role="status">Loading clients…</div> : clientsError ? <div className="empty-state">Use Retry loading clients to recover the directory.</div> : visibleClients.length ? <div className="client-list">{visibleClients.map(client => <button className={`client-row ${selected?.id === client.id ? 'selected' : ''}`} key={client.id} onClick={() => openClient(client.id)} aria-pressed={selected?.id === client.id}><Initials client={client} /><div className="client-row-text"><strong>{client.firstName} {client.lastName}</strong><span>{client.email || client.phone || 'No contact details added'}</span><Status status={client.status} /></div><Icon name="arrow" size={15} /></button>)}</div> : <div className="empty-state"><Icon name={clients.length ? 'search' : 'clients'} size={28} /><h3>{clients.length ? 'No matching clients' : 'Your client directory starts here.'}</h3><p>{clients.length ? 'Try a different search or status filter.' : 'Add a fictional client to create your first record.'}</p>{!clients.length && <button className="button secondary" onClick={createClient}>Add client</button>}</div>}
            <div className="directory-footer">{visibleClients.length} {visibleClients.length === 1 ? 'client' : 'clients'}{query || statusFilter !== 'All statuses' ? ` of ${clients.length}` : ''}</div>
          </section>
          {selectionLoading ? <div className="panel selection-loading" role="status">Opening client record…</div> : editing ? <ClientEditor key={`${selected?.id??'new'}:${clientSection}`} section={selected?clientSection:'profile'} onNavigate={navigate} client={selected} workspace={workspace} followUpId={targetFollowUp} onSaved={savedClient} onDirty={onDirty} onBusy={onBusy} onCancel={() => { if (canLeave()) clearSelection(); }} /> : clients.length > 0 && <div className="panel client-selection-hint"><Icon name="file" size={34} /><h2>A complete client picture</h2><p className="muted">Select a client to review intake details, update their counseling stage, and add a session note.</p></div>}
          {selectionError && <ErrorMessage>{selectionError}</ErrorMessage>}
        </div></>}
        {isModule(view)&&<>
         {view==='Financial Scenarios'&&<p className="field-help">Temporary previews reset when leaving this page. Save a reviewed scenario to retain it.</p>}
         <section className="panel module-context" aria-label="Current client context"><div><label htmlFor="workspace-client">Client for {view}<select id="workspace-client" value={selected?.id??''} disabled={clientsLoading||selectionLoading||Boolean(clientsError)} onChange={e=>{if(e.target.value)void openClient(e.target.value,undefined,view);else if(canLeave())clearSelection();}}><option value="">Choose a client</option>{clients.map(c=><option key={c.id} value={c.id}>{c.firstName} {c.lastName}{c.status==='Archived'?' — Archived':''}</option>)}</select></label></div><div>{selected?<><strong>{selected.firstName} {selected.lastName}</strong><Status status={selected.status}/><p className="field-help">Only this client’s records are shown. Save changes before moving to another section.</p></>:<p className="muted small">Choose an existing client or use Add client to start a record.</p>}</div></section>
         <nav className="section-navigation" aria-label={`${view} sections`}>{['Working tools',...(workingAreas[view]??[]),...plannedAreas[view].map(a=>a.title)].map(area=><button key={area} type="button" aria-current={moduleArea===area?'page':undefined} onClick={()=>{if(moduleArea!==area&&canLeave()){setModuleArea(area);dirty.current=false;}}}>{area}{plannedAreas[view].some(a=>a.title===area)&&<span className="planned-tab-label">Planned</span>}</button>)}</nav>
         {selectionError&&<ErrorMessage>{selectionError}</ErrorMessage>}
         {selectionLoading?<p className="panel settings-panel" role="status">Opening client record…</p>:moduleArea!=='Working tools'&&!workingAreas[view]?.includes(moduleArea)?<PlannedWorkspaceArea view={view} title={moduleArea}/>:selected?view==='Financial Review'&&moduleArea==='Card completeness'?<CardCompleteness key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Financial Review'&&moduleArea==='Bank & card activity'?<CardFinancialReview key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Financial Review'&&moduleArea==='Completeness & final validation'?<CompletenessReview key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Reports'&&moduleArea==='Final report checklist'?<SavedReportChecklist key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Documents'&&moduleArea==='Credit-card statements'?<CardStatements key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Documents'&&moduleArea==='Statement accounts'?<StatementAccounts key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Documents'&&moduleArea==='Collection checklist'?<DocumentCollection key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:<ClientEditor key={`${selected.id}:${view}`} section={view} client={selected} workspace={workspace} onSaved={savedClient} onDirty={onDirty} onBusy={onBusy} onNavigate={navigate} onCancel={()=>{if(canLeave())clearSelection();}}/>:<section className="panel module-empty"><Icon name={workspacePages.find(p=>p.id===view)?.icon??'file'} size={32}/><h2>Select a client to open {view}</h2><p className="muted">{clientsLoading?'Loading the client directory…':clientsError?'Reload the client directory using the retry above.':clients.length?'Select a client above. Their saved data will load here.':'No clients yet. Add a fictional client to try this workflow.'}</p></section>}
        </>}
        {view==='Settings'&&<AdministrationPages workspace={workspace} onLock={()=>void lock()} onBusy={onBusy} onDirty={onDirty} onWorkspaceChanged={name=>setWorkspace(w=>w?{...w,organizationName:name}:w)}/>}

      </main>
      <footer className="app-footer"><span>Eisenbart & Co. Financial Counseling Suite</span><span>Financial planning · Development build</span></footer>
    </div>
  </div>;
}
