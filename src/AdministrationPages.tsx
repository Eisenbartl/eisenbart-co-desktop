import {useCallback,useState} from 'react';
import FirmBranding from './FirmBranding';
import PasswordChange from './PasswordChange';
import {BackupPanel} from './RecoveryPanel';
import type {WorkspaceInfo} from './types';
const tabs=['Workspace','Security','Backup & recovery','Branding','Counselors & roles','Data management','Licensing','Help & about'] as const;
type Tab=typeof tabs[number];
const planned:Partial<Record<Tab,{title:string;description:string;items:string[]}>>={
 'Counselors & roles':{title:'Counselors and permissions',description:'This installation currently supports one counselor. Additional accounts and permission editing are planned.',items:['Counselor account creation and lifecycle','Client access and record permissions','Organization administration and role history']},
 'Data management':{title:'Data management',description:'Encrypted backup and recovery are available in Backup & recovery. Client export and permanent deletion are planned and cannot be performed here.',items:['Portable client export','Reviewed retention and deletion workflows','Audit and retained-recovery copy management']},
 Licensing:{title:'Licensing and updates',description:'This is a development build. Commercial activation, license administration and signed updates are planned.',items:['Offline license management','Commercial notices and support information','Signed distribution and update procedures']},
};
export default function AdministrationPages({workspace,onLock,onBusy,onDirty,onWorkspaceChanged,onPasswordChanged}:{workspace:WorkspaceInfo;onLock:()=>void;onPasswordChanged:(result:import('./types').PasswordChangeResult)=>void;onBusy:(busy:boolean)=>void;onDirty:(dirty:boolean)=>void;onWorkspaceChanged:(name:string)=>void}) {
 const [busy,setBusy]=useState(false),[dirty,setDirty]=useState(false);
 const brandingDirty=useCallback((value:boolean)=>{setDirty(value);onDirty(value);},[onDirty]);
 const backupBusy=useCallback((value:boolean)=>{setBusy(value);onBusy(value);},[onBusy]);
 const [tab,setTab]=useState<Tab>('Workspace');const pending=planned[tab];
 return <div className="administration-pages">
  <nav className="section-navigation" aria-label="Settings sections">{tabs.map(t=><button type="button" disabled={busy} key={t} aria-current={tab===t?'page':undefined} onClick={()=>{if(t!==tab&&(!dirty||window.confirm("Discard unsaved settings edits?")))setTab(t);}}>{t}</button>)}</nav>
  <div className="settings-page" key={tab}>
   {tab==='Workspace'&&<section className="panel settings-panel"><p className="eyebrow">WORKSPACE IDENTITY</p><h2>Organization & counselor</h2><dl className="details-list"><div><dt>Organization</dt><dd>{workspace.organizationName}</dd></div><div><dt>Counselor</dt><dd>{workspace.counselorName}</dd></div><div><dt>Workspace type</dt><dd>Single counselor · Local desktop</dd></div></dl><p className="field-help">Change the firm's display name and report presentation in Branding. Counselor editing remains planned.</p></section>}
   {tab==='Branding'&&<FirmBranding organizationId={workspace.organizationId} onDirty={brandingDirty} onBusy={backupBusy} onSaved={onWorkspaceChanged}/>}
   {tab==='Security'&&<><section className="panel settings-panel"><h2>Storage & security</h2><dl className="details-list"><div><dt>Storage</dt><dd>Encrypted local SQLCipher database</dd></div><div><dt>Essential operations</dt><dd>Available offline</dd></div><div><dt>Access</dt><dd>Workspace password and session locking</dd></div></dl><button className="button secondary" onClick={onLock}>Lock workspace</button><p className="field-help">Keep your password safe. Forgotten-password reset is unavailable. Configurable locking and independent security review remain planned.</p></section><PasswordChange onChanged={onPasswordChanged} onBusy={backupBusy} onDirty={brandingDirty}/></>}
   {tab==='Backup & recovery'&&<div className="settings-grid"><BackupPanel onBusy={backupBusy}/><section className="panel settings-panel"><h2>Restore a workspace</h2><p className="muted small">Verification and explicit confirmation are required before activating a backup. Keep the password used when it was created.</p><button className="button secondary" onClick={onLock}>Lock workspace to restore a backup</button></section></div>}
   {pending&&<section className="panel settings-panel planned-page"><span className="feature-state">Planned</span><h2>{pending.title}</h2><p className="muted">{pending.description}</p><ul>{pending.items.map(i=><li key={i}>{i}</li>)}</ul></section>}
   {tab==='Help & about'&&<section className="panel settings-panel"><h2>Using this prototype</h2><ol><li>Create a fictional client in Clients.</li><li>Use the main navigation to open each workflow. Select a client before viewing their data.</li><li>Save editable records explicitly. Navigation asks before discarding unsaved edits.</li><li>Import statements in Documents; review them in Transactions and Financial Review.</li><li>Save plans, compare scenarios, then record and export assessments and client plans in Reports.</li><li>Create an encrypted backup before evaluating recovery.</li></ol><h3>Record changes and history</h3><p>Clients can be archived through their counseling status and restored to an active status. Draft plan rows can be removed and saved. Session notes, imported source evidence, decisions, saved scenarios and recorded reports retain history; permanent deletion is not implemented.</p><p className="field-help">Use fictional records. Planned pages are labeled and do not save changes. Clean installed Windows acceptance and commercial release work remain open.</p></section>}
  </div>
 </div>;
}
