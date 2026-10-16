import ConfirmationDialog,{confirmDiscard} from './ConfirmationDialog';
import ClientDirectory,{clientName} from './ClientDirectory';
import ClientWorkspaceLayout from './ClientWorkspaceLayout';
import ClientOverview from './ClientOverview';
import SpendingAnalysis from './SpendingAnalysis';
import './client-workspace.css';
import Icon from './Icons';
import ClientHistory from './ClientHistory';
import NoteCorrection from './NoteCorrection';
import ReportPresentation from './ReportPresentation';
import TransactionActivity from './TransactionActivity';
import CardCompleteness from './CardCompleteness';
import CardFinancialReview from './CardFinancialReview';
import CardStatements from './CardStatements';
import CompletenessReview,{SavedReportChecklist} from './CompletenessReview';
import StatementAccounts from './StatementAccounts';
import DocumentCollection from './DocumentCollection';
import AdministrationPages from './AdministrationPages';
import PlannedWorkspaceArea,{plannedAreas,workingAreas} from './PlannedWorkspaceAreas';
import {workspacePages,isModule,type View,type ClientSection} from './workspaceNavigation';
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

function ClientEditor({ client, workspace, onSaved, onCancel, onDirty, onBusy, followUpId, section='profile', onNavigate, onClientSection }: {
  client: Client | null; workspace: WorkspaceInfo; followUpId?:string; section?:ClientSection; onNavigate:(view:View)=>void; onClientSection?:(section:ClientSection)=>void;
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

  const shows=(...sections:ClientSection[])=>sections.includes(section);
  return <div className="client-editor">
    {shows('profile')&&<div className="panel editor-panel">
      <div className="panel-heading"><div><p className="eyebrow">{client ? 'CLIENT RECORD' : 'CLIENT INTAKE'}</p><h2>{client ? 'Client profile' : 'Add a new client'}</h2></div>{client && <Status status={client.status} />}</div>
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
    {client&&section==='history'&&<ClientHistory clientId={client.id} disabled={dirty||saving||noteSaving||careBusy||importsBusy||scenarioBusy} onBusy={setScenarioBusy} onOpen={kind=>{if(kind==='profile'||kind==='note'||kind==='intake')onClientSection?.(kind==='note'?'notes':kind==='intake'?'intake':'profile');else onNavigate(kind==='document'?'Documents':kind==='scenario'?'Financial Scenarios':'Reports');}}/>}
  </div>;
}

function ClientNotes({ client, workspace, onBusy, onDirty, clientSaving }: { client: Client; workspace: WorkspaceInfo; onBusy: (busy: boolean) => void; onDirty: (dirty: boolean) => void; clientSaving: boolean }) {
  const [notes, setNotes] = useState<ClientNote[]>([]);
  const [correctionId,setCorrectionId]=useState(''),[correctionBusy,setCorrectionBusy]=useState(false),[correctionDirty,setCorrectionDirty]=useState(false),[notice,setNotice]=useState('');
  const [body, setBody] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [loadError, setLoadError] = useState('');
  const [attempt, setAttempt] = useState(0);
  const mounted = useRef(true);
  useEffect(() => { onDirty(body.trim().length > 0||correctionDirty); return()=>onDirty(false); }, [body, correctionDirty, onDirty]);
  useEffect(()=>{onBusy(saving||correctionBusy);return()=>onBusy(false);},[saving,correctionBusy,onBusy]);
  useEffect(() => {
    mounted.current = true;
    let current = true;
    setNotes([]);
    setCorrectionId('');
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
    try {
      const note = await api.addClientNote(client.id, body.trim());
      if (!mounted.current) return;
      setNotes(previous => [note, ...previous]);
      setBody('');
    } catch (caught) {
      if (mounted.current) setError(message(caught));
    } finally {
      if (mounted.current) { setSaving(false); }
    }
  }

  return <section className="panel notes-panel" aria-labelledby="notes-title">
    <div className="panel-heading"><div><p className="eyebrow">COUNSELING HISTORY</p><h2 id="notes-title">Session notes</h2></div><span className="count-badge">{notes.length}</span></div>
    <p className="muted small">Saved notes retain their author and date. Review an explicit correction to amend a note; its original text and all corrections remain in history. Current versions are shown below.</p>
    <form onSubmit={add} noValidate>
      <label htmlFor="new-note">New session note<textarea id="new-note" rows={4} maxLength={8000} value={body} onChange={e => setBody(e.target.value)} placeholder="Record the discussion, decisions, and next steps…" disabled={saving || clientSaving || correctionBusy || correctionDirty || loading || !!loadError} /></label>
      <div className="field-footer"><span>Author: {workspace.counselorName}</span><span>{body.length.toLocaleString()} / 8,000</span></div>
      <ErrorMessage>{error}</ErrorMessage>
      <div className="note-actions"><button type="submit" className="button primary" disabled={saving || clientSaving || correctionBusy || correctionDirty || loading || !!loadError}>{saving ? 'Saving note…' : 'Save session note'}</button></div>
    </form>
    {notice&&<p className="success-message" role="status">{notice}</p>}
    {correctionId&&<NoteCorrection clientId={client.id} noteId={correctionId} disabled={clientSaving||saving||body.trim().length>0} onDirty={setCorrectionDirty} onBusy={setCorrectionBusy} onClose={()=>setCorrectionId('')} onSaved={()=>{setCorrectionId('');setNotice('Reviewed note correction saved. Original notes and report evidence retained.');setAttempt(v=>v+1);}}/>}
    <div className="notes-history">
      {loading ? <p className="muted" role="status">Loading session notes…</p> : loadError ? <><ErrorMessage>{loadError}</ErrorMessage><button className="button secondary" onClick={() => setAttempt(previous => previous + 1)}>Retry loading notes</button></> : notes.length === 0 ? <p className="empty-note">No session notes yet. Your first saved note will appear here.</p> : notes.map(note => <article className="session-note" key={note.id}><div className="note-meta"><strong>{note.counselorName}</strong><time dateTime={note.createdAt}>{dateLabel(note.createdAt, true)}</time></div><p>{note.body}</p><button className="button secondary" disabled={saving||clientSaving||correctionBusy||correctionDirty||body.trim().length>0||correctionId!==''} onClick={()=>{setNotice('');setCorrectionId(note.id);}}>Review correction for note {dateLabel(note.createdAt,true)}</button></article>)}
    </div>
  </section>;
}

function Status({ status }: { status: string }) {
  return <span className={`status-tag ${status === 'Archived' ? 'archived' : status === 'New Intake' ? 'intake' : ''}`}><span aria-hidden="true" />{status}</span>;
}

function Initials({ client }: { client: Client }) {
  return <span className="avatar" aria-hidden="true">{client.firstName.slice(0, 1)}{client.lastName.slice(0, 1)}</span>;
}

function passwordNotice(durabilityWarning:boolean):string {
  return `Workspace password changed. Unlock with the new password. Older backups and retained copies still require their original passwords.${durabilityWarning?' The disk did not confirm directory synchronization. Keep both passwords and your backups until restart verifies the active workspace.':''}`;
}

export default function App() {
  const [phase, setPhase] = useState<Phase>('checking');
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [authBusy, setAuthBusy] = useState(false);
  const [authError, setAuthError] = useState('');
  const [recoveryNotice,setRecoveryNotice]=useState('');
  const [statusAttempt, setStatusAttempt] = useState(0);
  const [view, setView] = useState<View>('Dashboard');
  const [clientSection,setClientSection]=useState<ClientSection>('overview');
  const [tabs,setTabs]=useState<{id:string;client:Client;view:View;section:ClientSection;area:string}[]>([]);
  const [recentTabs,setRecentTabs]=useState<string[]>([]);
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
  const requestedFocus=useRef<string|null>(null);
  useEffect(()=>{
    if(phase!=='unlocked')return;
    const clientPage=selected&&view!=='Dashboard'&&view!=='Settings';
    const heading=clientPage?document.getElementById('client-workspace-heading'):pageHeading.current;
    heading?.focus({preventScroll:true});(clientPage?document.querySelector('.client-workspace-header'):heading?.closest('.page-heading'))?.scrollIntoView?.({block:'start'});
    const target=requestedFocus.current;
    if(target==='create'&&editing&&!selected){document.getElementById('first-name')?.focus();requestedFocus.current=null;}
    else if(target==='directory'&&view==='Clients'&&!selected){document.getElementById('directory-tab')?.focus({preventScroll:true});requestedFocus.current=null;}
    else if(target&&selected?.id===target&&clientPage){document.getElementById(`client-tab-${target}`)?.focus({preventScroll:true});requestedFocus.current=null;}
  },[view,clientSection,moduleArea,phase,selected?.id,editing]);

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
    const locked = (event?: {payload?: unknown}) => {
      expire();
      const payload = event?.payload;
      if (payload && typeof payload === 'object' && 'reason' in payload && payload.reason === 'password_changed' && 'durabilityWarning' in payload && typeof payload.durabilityWarning === 'boolean') {
        setRecoveryNotice(passwordNotice(payload.durabilityWarning));
      }
    };
    listen('workspace-locked', locked).then(remove => { if (cancelled) remove(); else unlisten = remove; }).catch(() => {
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
    resetApiSession();requestedFocus.current=null;
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
    setClientSection('overview');setModuleArea('Working tools');setTabs([]);setRecentTabs([]);
    dirty.current = false;
    editorBusy.current = false;
  }

  useEffect(()=>{
    if(!selected||view==='Dashboard'||view==='Settings')return;
    setTabs(previous=>previous.map(t=>t.id===selected.id?{...t,client:selected,view,section:clientSection,area:moduleArea}:t));
  },[selected,view,clientSection,moduleArea]);

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

  function passwordChanged(result:import('./types').PasswordChangeResult) {
    clearWorkspace();setAuthError('');setAuthBusy(false);setPhase('locked');
    setRecoveryNotice(passwordNotice(result.durabilityWarning));
  }

  function restored(result:RestoreResult) {
    clearWorkspace();setAuthError('');setAuthBusy(false);setPhase('locked');
    setRecoveryNotice(`Workspace restored. Unlock it using the password from this backup.${result.durabilityWarning?' The disk did not confirm directory synchronization; keep your backup available.':''}`);
  }

  function canLeave(resume:()=>void): boolean {
    if (editorBusy.current) return false;
    if(!dirty.current)return true;
    const generation=session.current,request=selectionRequest.current;
    confirmDiscard('You have unsaved client changes. Discard them and continue?',()=>{
      if(generation!==session.current||request!==selectionRequest.current||editorBusy.current)return;
      dirty.current=false;resume();
    });
    return false;
  }

  function cancelClient(){if(canLeave(cancelClient)){requestedFocus.current='directory';clearSelection();document.getElementById('directory-tab')?.focus();}}
  function selectArea(area:string){if(moduleArea!==area&&canLeave(()=>selectArea(area))){setModuleArea(area);dirty.current=false;}}

  function clearSelection() {
    selectionRequest.current += 1;
    setSelected(null);
    setTargetFollowUp(undefined);
    setEditing(false);
    setSelectionLoading(false);
    setSelectionError('');
    dirty.current = false;
  }

  function navigate(next: View, section?:ClientSection) {
    if (!canLeave(()=>navigate(next,section))) return;
    selectionRequest.current++;setSelectionLoading(false);setSelectionError('');
    dirty.current=false;editorBusy.current=false;setTargetFollowUp(undefined);
    if(next==='Clients'&&section){setView('Clients');setClientSection(section);setEditing(Boolean(selected));}
    else if(next==='Clients'){requestedFocus.current='directory';clearSelection();setView('Clients');document.getElementById('directory-tab')?.focus();}
    else {setView(next);setEditing(Boolean(selected));setModuleArea('Working tools');}
  }

  function createClient() {
    if (!canLeave(createClient)) return;
    requestedFocus.current='create';clearSelection();setView('Clients');setClientSection('profile');setEditing(true);
  }

  async function openClient(id: string, followUpId?:string, targetView?:View) {
    if(selected?.id===id&&!followUpId&&!targetView&&view!=='Dashboard'&&view!=='Settings'){
      document.getElementById(`client-tab-${id}`)?.focus();return;
    }
    if (!canLeave(()=>void openClient(id,followUpId,targetView))) return;
    const existing=tabs.find(t=>t.id===id);
    clearSelection();
    const request = selectionRequest.current;
    const currentSession = session.current;
    const nextView=followUpId?'Clients':targetView??existing?.view??'Clients';
    setView(nextView);setModuleArea(targetView?'Working tools':existing?.area??'Working tools');
    setClientSection(followUpId?'intake':targetView?'overview':existing?.section??'overview');
    setSelectionLoading(true);
    try {
      const result = await api.getClient(id);
      if (request === selectionRequest.current && currentSession === session.current) {
        if(result.id!==id)throw new Error('The client response did not match the selected record.');
        requestedFocus.current=id;setTargetFollowUp(followUpId);setSelected(result);setEditing(true);
        setTabs(previous=>previous.some(t=>t.id===id)?previous: [...previous,{id,client:result,view:nextView,section:followUpId?'intake':'overview',area:'Working tools'}]);
        setRecentTabs(previous=>[...previous.filter(t=>t!==id),id]);
      }
    } catch (caught) {
      if (request === selectionRequest.current && currentSession === session.current) setSelectionError(message(caught));
    } finally {
      if (request === selectionRequest.current && currentSession === session.current) setSelectionLoading(false);
    }
  }

  function closeClientTab(id:string){
    const active=selected?.id===id&&view!=='Dashboard'&&view!=='Settings';
    if(active&&!canLeave(()=>closeClientTab(id)))return;
    setTabs(previous=>previous.filter(t=>t.id!==id));setRecentTabs(previous=>previous.filter(t=>t!==id));
    if(active){dirty.current=false;editorBusy.current=false;clearSelection();const next=[...recentTabs].reverse().find(t=>t!==id&&tabs.some(tab=>tab.id===t));if(next)void openClient(next);else{requestedFocus.current='directory';setView('Clients');}}
    else document.getElementById(selected&&selected.id!==id?`client-tab-${selected.id}`:'directory-tab')?.focus();
  }

  function tabKeys(event:import('react').KeyboardEvent<HTMLDivElement>){
    if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key)||!(event.target instanceof HTMLElement)||event.target.getAttribute('role')!=='tab')return;
    const ids=['directory',...tabs.map(t=>t.id)];const active=event.target.dataset.clientTab??'directory';const index=ids.indexOf(active);
    const next=event.key==='Home'?ids[0]:event.key==='End'?ids[ids.length-1]:ids[(index+(event.key==='ArrowRight'?1:-1)+ids.length)%ids.length];
    event.preventDefault();if(next==='directory'){navigate('Clients');}else void openClient(next);
  }

  async function lock() {
    if (!canLeave(()=>void lock())) return;
    clearWorkspace();
    setPhase('locked');
    setAuthBusy(true);
    setAuthError('');
    try { await api.lockWorkspace(); } catch (caught) { setAuthError(message(caught)); } finally { setAuthBusy(false); }
  }

  function savedClient(client: Client) {
    setClients(previous => [client, ...previous.filter(item => item.id !== client.id)].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)));
    if(!selected){setClientSection('overview');setView('Clients');}
    setSelected(client);setEditing(true);
    setTabs(previous=>previous.some(t=>t.id===client.id)?previous.map(t=>t.id===client.id?{...t,client}:t):[...previous,{id:client.id,client,view:'Clients',section:'overview',area:'Working tools'}]);
    setRecentTabs(previous=>[...previous.filter(id=>id!==client.id),client.id]);
  }

  if (!desktopAvailable()) return <main className="standalone"><Brand /><div className="panel standalone-panel"><Icon name="lock" size={32} /><h1>Open the desktop application</h1><p className="muted">This workspace runs inside the Eisenbart & Co. desktop application. A browser preview cannot access your local database.</p></div><DevelopmentNotice /></main>;
  if (phase === 'checking' || phase === 'error') return <main className="standalone"><Brand /><div className="panel standalone-panel"><h1>{phase === 'checking' ? 'Opening your workspace…' : 'Unable to open the workspace'}</h1>{phase === 'checking' ? <p className="muted" role="status">Checking local storage.</p> : <><ErrorMessage>{authError}</ErrorMessage><button className="button primary" onClick={() => setStatusAttempt(previous => previous + 1)}>Try again</button><RestorePanel onRestored={restored}/></>}</div></main>;
  if (phase !== 'unlocked' || !workspace) return <AuthForm key={phase} setup={phase === 'setup'} busy={authBusy} error={authError} notice={recoveryNotice} onSubmit={authenticate} onRestored={restored} />;

  const activeClients = clients.filter(client => client.status !== 'Archived');
  const newIntakes = clients.filter(client => client.status === 'New Intake');
  const followUps = clients.filter(client => client.status === 'Follow-Up');
  const recentClients = [...clients].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 5);


  const tabLabel=(client:Client)=>tabs.some(t=>t.id!==client.id&&clientName(t.client)===clientName(client))?`${clientName(client)} · ${client.id}`:clientName(client);

  return <div className="app-shell">
    <aside className="sidebar">
      <Brand compact />
      <div className="workspace-label"><span>WORKSPACE</span><strong title={workspace.organizationName}>{workspace.organizationName}</strong></div>
      <nav aria-label="Main navigation">{workspacePages.filter(item=>['Dashboard','Clients','Settings'].includes(item.id)).map(item=><button key={item.id} title={item.title} className={`nav-item ${(view===item.id||(item.id==='Clients'&&isModule(view)))?'active':''}`} aria-label={item.id} aria-current={view===item.id||(item.id==='Clients'&&isModule(view))?'page':undefined} onClick={()=>navigate(item.id)}><Icon name={item.icon}/><span>{item.title}</span>{item.id==='Clients'&&<span className="nav-count">{clients.length}</span>}</button>)}</nav>
      <div className="sidebar-bottom"><div className="local-indicator"><span aria-hidden="true" />Local workspace</div><div className="counselor"><div className="counselor-avatar" aria-hidden="true">{workspace.counselorName.slice(0, 1)}</div><div><strong title={workspace.counselorName}>{workspace.counselorName}</strong><span>Counselor</span></div></div><button className="lock-button" onClick={lock}><Icon name="lock" size={16} />Lock workspace</button></div>
    </aside>
    <div className="main-shell">
      <header className="topbar"><div className="breadcrumbs">Workspace<Icon name="arrow" size={13} /><strong>{selected&&view!=='Dashboard'&&view!=='Settings'?clientName(selected):view}</strong></div><div className="offline-status"><Icon name="shield" size={15} />Offline-first · Local storage</div></header>
      <div className="workspace-tabbar" role="tablist" aria-label="Open workspace tabs" onKeyDown={tabKeys}>
        <div className={`workspace-app-tab ${view==='Clients'&&!selected&&!selectionLoading?'active':''}`}><button id="directory-tab" role="tab" data-client-tab="directory" aria-selected={view==='Clients'&&!selected&&!selectionLoading} aria-controls="workspace-page" tabIndex={!selected||view==='Dashboard'||view==='Settings'?0:-1} onClick={()=>navigate('Clients')}><Icon name="clients" size={15}/>All clients</button></div>
        {tabs.map(tab=>{const active=selected?.id===tab.id&&view!=='Dashboard'&&view!=='Settings';return <div className={`workspace-app-tab ${active?'active':''}`} key={tab.id}><button id={`client-tab-${tab.id}`} role="tab" aria-label={tabLabel(tab.client)} data-client-tab={tab.id} aria-selected={active} aria-controls="workspace-page" tabIndex={active?0:-1} onClick={()=>void openClient(tab.id)} title={`${clientName(tab.client)} · ${tab.id}`}><Icon name="clients" size={14}/><span>{clientName(tab.client)}</span></button><button className="workspace-tab-close" aria-label={`Close workspace for ${tabLabel(tab.client)}`} onClick={()=>closeClientTab(tab.id)}>×</button></div>;})}
      </div>
      <DevelopmentNotice />
      <main className="main-content"><div id="workspace-page" role={view==='Dashboard'||view==='Settings'?undefined:'tabpanel'} aria-labelledby={view==='Dashboard'||view==='Settings'?undefined:selected?`client-tab-${selected.id}`:'directory-tab'}>
        {(!selected||view==='Dashboard'||view==='Settings')&&<div className="page-heading"><div><p className="eyebrow">{view==='Dashboard'?'YOUR COUNSELING WORKSPACE':view==='Settings'?'WORKSPACE ADMINISTRATION':'CLIENT WORKSPACE'}</p><h1 ref={pageHeading} tabIndex={-1}>{view==='Dashboard'?'A clear view of your practice.':workspacePages.find(p=>p.id===view)?.title}</h1><p className="muted">{workspacePages.find(p=>p.id===view)?.description}</p></div>{(view==='Dashboard'||view==='Clients')&&<button className="button primary" onClick={createClient} disabled={clientsLoading}><Icon name="plus" size={18}/>Add client</button>}</div>}
        {clientsError && <div className="load-error"><ErrorMessage>{clientsError}</ErrorMessage><button className="button secondary" onClick={loadClients} disabled={clientsLoading}>{clientsLoading ? 'Loading…' : 'Retry loading clients'}</button></div>}
        {view === 'Dashboard' && <>
          <div className="stats-grid">{[{ label: 'Active clients', value: activeClients.length, caption: 'Clients currently in your care', icon: 'clients' }, { label: 'New intakes', value: newIntakes.length, caption: 'At the start of their journey', icon: 'file' }, { label: 'Follow-up stage', value: followUps.length, caption: 'Clients marked for follow-up', icon: 'check' }].map(stat => <div className="stat-card" key={stat.label}><div className="stat-header"><span>{stat.label}</span><div className="stat-icon"><Icon name={stat.icon} size={18} /></div></div><strong className="stat-value">{clientsLoading || clientsError ? '—' : stat.value}</strong><p>{stat.caption}</p></div>)}</div>
          <FollowUpOverview organizationId={workspace.organizationId} counselorId={workspace.counselorId} onOpen={(id,follow)=>void openClient(id,follow)}/>
          <div className="dashboard-grid"><section className="panel recent-panel"><div className="panel-heading"><div><h2>Recent client activity</h2><p className="muted small">The most recently updated client records.</p></div><button className="text-button" onClick={() => navigate('Clients')}>View all<Icon name="arrow" size={15} /></button></div>{clientsLoading ? <div className="empty-state" role="status">Loading client records…</div> : clientsError ? <div className="empty-state">Client records could not be loaded.</div> : recentClients.length ? <div className="recent-list">{recentClients.map(client => <button className="recent-row" key={client.id} onClick={() => openClient(client.id)}><Initials client={client} /><div className="recent-name"><strong>{client.firstName} {client.lastName}</strong><span>Updated {dateLabel(client.updatedAt)}</span></div><Status status={client.status} /><Icon name="arrow" size={16} /></button>)}</div> : <div className="empty-state"><div className="empty-icon"><Icon name="clients" size={30} /></div><h3>Every journey starts with a conversation.</h3><p>Add your first fictional client to begin organizing intake and counseling notes.</p><button className="button secondary" onClick={createClient}><Icon name="plus" size={16} />Add your first client</button></div>}</section>
          <aside className="practice-card"><div className="practice-icon"><Icon name="shield" size={24} /></div><p className="eyebrow">BUILT FOR FOCUSED WORK</p><h2>Your workspace.<br />On your computer.</h2><p>Client records and notes are saved locally, so you can continue your work without an internet connection.</p><div className="practice-divider" /><h3>A working foundation</h3><p>Client intake, counseling status, notes, monthly budgets, debt payoff comparisons, savings goals, cash-flow calendars, CSV and selectable-text PDF transaction review, merchant/recurring-payment review, and statement running-balance checks are available.</p><button className="text-button" onClick={() => navigate('Settings')}>View workspace details<Icon name="arrow" size={15} /></button></aside></div>
          <section className="panel progress-panel"><div><h2>Counseling progress</h2><p className="muted small">Client counts by their current counseling stage.</p></div><div className="stage-grid">{CLIENT_STATUSES.filter(status => status !== 'Archived').map(status => <div className="stage-item" key={status}><span>{status}</span><strong>{clientsLoading || clientsError ? '—' : clients.filter(client => client.status === status).length}</strong></div>)}</div></section>
        </>}
        {view==='Clients'&&!selected&&!selectionLoading&&<ClientDirectory clients={clients} loading={clientsLoading} error={clientsError} search={search} setSearch={setSearch} status={statusFilter} setStatus={setStatusFilter} onOpen={id=>void openClient(id)} onAdd={createClient}/>}
        {selectionLoading&&<div className="panel selection-loading" role="status">Opening client record…</div>}
        {selectionError&&<div className="panel selection-loading"><ErrorMessage>{selectionError}</ErrorMessage><button className="button secondary" onClick={()=>navigate('Clients')}>Return to all clients</button></div>}
        {view==='Clients'&&editing&&!selected&&!selectionLoading&&<div className="client-modal-backdrop"><div className="client-create-modal" role="dialog" aria-modal="true" aria-label="Add a new client" onKeyDown={e=>{if(e.key==='Escape')cancelClient();if(e.key==='Tab'){const controls=[...e.currentTarget.querySelectorAll<HTMLElement>('button,input,select,textarea')].filter(el=>!el.matches(':disabled'));if(e.shiftKey&&document.activeElement===controls[0]){e.preventDefault();controls.at(-1)?.focus();}else if(!e.shiftKey&&document.activeElement===controls.at(-1)){e.preventDefault();controls[0]?.focus();}}}}><ClientEditor section="profile" client={null} workspace={workspace} onSaved={savedClient} onDirty={onDirty} onBusy={onBusy} onNavigate={navigate} onCancel={cancelClient}/></div></div>}
        {selected&&view==='Clients'&&<ClientWorkspaceLayout client={selected} view={view} section={clientSection} onNavigate={navigate}>
          {clientSection==='overview'?<ClientOverview key={selected.id} client={selected} onNavigate={navigate}/>:<>
            <nav className="section-navigation client-subtabs" aria-label="Client record sections">{(clientSection==='notes'||clientSection==='history'?[['notes','Session notes'],['history','History & records']]:[['profile','Profile'],['intake','Intake & follow-ups']]).map(([id,label])=><button key={id} type="button" aria-current={clientSection===id?'page':undefined} onClick={()=>{if(clientSection!==id)navigate('Clients',id as ClientSection);}}>{label}</button>)}</nav>
            <ClientEditor key={`${selected.id}:${clientSection}`} section={clientSection} onClientSection={next=>navigate('Clients',next)} onNavigate={navigate} client={selected} workspace={workspace} followUpId={targetFollowUp} onSaved={savedClient} onDirty={onDirty} onBusy={onBusy} onCancel={()=>navigate('Clients')}/>
          </>}
        </ClientWorkspaceLayout>}
        {isModule(view)&&selected&&<ClientWorkspaceLayout client={selected} view={view} section={clientSection} onNavigate={navigate}>
         {view==='Spending Analysis'?<SpendingAnalysis key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy} onOpenReview={()=>navigate('Transactions')}/>:<>
         {view==='Financial Scenarios'&&<p className="field-help">Temporary previews reset when leaving this page. Save a reviewed scenario to retain it.</p>}
         <div className="workspace-module-heading"><p className="eyebrow">{view==='Documents'?'DOCUMENTS & REVIEW':view==='Transactions'?'DOCUMENTS & REVIEW':view==='Reports'?'ASSESSMENTS & REPORTS':'CLIENT TOOLS'}</p><h2>{view==='Documents'?'Import statements':view==='Transactions'?'Review & reconciliation':view==='Financial Review'?'Financial dashboard':view==='Budget Planner'?'Monthly budget':view==='Debt Planner'?'Debt payoff':view==='Savings Planner'?'Savings & sinking funds':view==='Financial Scenarios'?'Scenario comparisons':view}</h2><p>{workspacePages.find(p=>p.id===view)?.description}</p></div>
         <nav className="section-navigation" aria-label={`${view} sections`}>{['Working tools',...(workingAreas[view]??[]),...plannedAreas[view].map(a=>a.title)].map(area=><button key={area} type="button" aria-current={moduleArea===area?'page':undefined} onClick={()=>selectArea(area)}>{area==='Working tools'?(view==='Transactions'?'Review queue':view==='Financial Review'?'Approved activity':view==='Reports'?'Assessments & action plans':view==='Documents'?'Import & sources':area):area}{plannedAreas[view].some(a=>a.title===area)&&<span className="planned-tab-label">Planned</span>}</button>)}</nav>
         {selectionError&&<ErrorMessage>{selectionError}</ErrorMessage>}
         {selectionLoading?<p className="panel settings-panel" role="status">Opening client record…</p>:moduleArea!=='Working tools'&&!workingAreas[view]?.includes(moduleArea)?<PlannedWorkspaceArea view={view} title={moduleArea}/>:selected?view==='Reports'&&moduleArea==='Print & presentation'?<ReportPresentation key={selected.id} clientId={selected.id} onBusy={onBusy}/>:view==='Transactions'&&moduleArea==='Classification assistance'?<ImportsEditor key={`${selected.id}:classification`} client={selected} page="classification" onDirty={onDirty} onBusy={onBusy} otherSaving={false}/>:view==='Transactions'&&moduleArea==='Activity history'?<TransactionActivity key={selected.id} clientId={selected.id}/>:(view==='Financial Review'||view==='Transactions')&&moduleArea==='Card completeness'?<CardCompleteness key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:(view==='Financial Review'||view==='Transactions')&&moduleArea==='Bank & card activity'?<CardFinancialReview key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:(view==='Financial Review'||view==='Transactions')&&moduleArea==='Completeness & final validation'?<CompletenessReview key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Reports'&&moduleArea==='Final report checklist'?<SavedReportChecklist key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Documents'&&moduleArea==='Credit-card statements'?<CardStatements key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Documents'&&moduleArea==='Statement accounts'?<StatementAccounts key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Documents'&&moduleArea==='Collection checklist'?<DocumentCollection key={selected.id} clientId={selected.id} onDirty={onDirty} onBusy={onBusy}/>:view==='Transactions'?<ImportsEditor key={`${selected.id}:${moduleArea}`} client={selected} page={({"Verified figures":'facts',"Running balances":'balances',"Recurring payments":'recurring',"Transfer matching":'transfers',"Source checks":'sources'} as const)[moduleArea as 'Verified figures']??'transactions'} onDirty={onDirty} onBusy={onBusy} otherSaving={false}/>:view==='Financial Review'?<ImportsEditor key={selected.id} client={selected} page="dashboard" onDirty={onDirty} onBusy={onBusy} otherSaving={false}/>:<ClientEditor key={`${selected.id}:${view}`} section={view} client={selected} workspace={workspace} onSaved={savedClient} onDirty={onDirty} onBusy={onBusy} onNavigate={navigate} onCancel={cancelClient}/>:<section className="panel module-empty"><Icon name={workspacePages.find(p=>p.id===view)?.icon??'file'} size={32}/><h2>Select a client to open {view}</h2><p className="muted">{clientsLoading?'Loading the client directory…':clientsError?'Reload the client directory using the retry above.':clients.length?'Select a client above. Their saved data will load here.':'No clients yet. Add a fictional client to try this workflow.'}</p></section>}
                 </>}
        </ClientWorkspaceLayout>}
        {view==='Settings'&&<AdministrationPages onPasswordChanged={passwordChanged} workspace={workspace} onLock={()=>void lock()} onBusy={onBusy} onDirty={onDirty} onWorkspaceChanged={name=>setWorkspace(w=>w?{...w,organizationName:name}:w)}/>}

      </div></main>
      <footer className="app-footer"><span>Eisenbart & Co. Financial Counseling Suite</span><span>Financial planning · Development build</span></footer>
    </div>
    <ConfirmationDialog/>
  </div>;
}
