import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import type {BackupResult,RestoreReview,RestoreResult} from './types';
const message=(error:unknown)=>error instanceof Error?error.message:String(error);
export function BackupPanel({onBusy}:{onBusy?:(busy:boolean)=>void}={}) {
  const [directory,setDirectory]=useState(''),[password,setPassword]=useState('');
  const [busy,setBusy]=useState(false),[error,setError]=useState(''),[result,setResult]=useState<BackupResult|null>(null);
  useEffect(()=>{onBusy?.(busy);return()=>onBusy?.(false);},[busy,onBusy]);
  const current=useRef(true);useEffect(()=>{current.current=true;return()=>{current.current=false;};},[]);
  async function choose(){setBusy(true);setError('');try{const folder=await api.chooseBackupFolder();if(current.current&&folder)setDirectory(folder);}catch(e){if(current.current)setError(message(e));}finally{if(current.current)setBusy(false);}}
  async function backup(event:FormEvent){
    event.preventDefault();if(busy)return;setError('');setResult(null);
    if(!directory.trim()||!password){setError('Choose a backup folder and enter the current workspace password.');return;}
    const secret=password;setPassword('');setBusy(true);
    try{const value=await api.createWorkspaceBackup(directory.trim(),secret);if(current.current)setResult(value);}catch(e){if(current.current)setError(message(e));}finally{if(current.current)setBusy(false);}
  }
  return <section className="panel recovery-panel" aria-labelledby="backup-title"><p className="eyebrow">PROTECT YOUR WORK</p><h2 id="backup-title">Encrypted workspace backup</h2>
    <p className="muted small">Save all clients, notes, financial plans, and audit history in one encrypted file. Choose an external drive or another folder outside the app's data folder. Keep more than one dated backup.</p>
    <form onSubmit={backup} noValidate><fieldset disabled={busy}>
      <label htmlFor="backup-folder">Backup folder<input id="backup-folder" value={directory} onChange={e=>{setDirectory(e.target.value);setResult(null);}} placeholder="Choose a folder or enter its full path"/></label>
      <button type="button" className="button secondary" onClick={()=>void choose()}>Choose backup folder</button>
      <label htmlFor="backup-password">Current workspace password<input id="backup-password" type="password" autoComplete="current-password" maxLength={1024} value={password} onChange={e=>setPassword(e.target.value)}/></label>
      <p className="field-help">This password will also be required to restore the backup. Losing it prevents recovery. The app never stores it in the backup.</p>
      {error && <div role="alert" className="alert error">{error}</div>}
      <button className="button primary" type="submit">{busy?'Creating encrypted backup…':'Create encrypted backup'}</button>
    </fieldset></form>
    {result && <div className="backup-success" role="status"><strong>Encrypted backup saved.</strong><p>Created {result.createdAt}</p><p className="backup-file-path">{result.path}</p>{!result.auditRecorded&&<p>The file was saved, but the audit entry could not be recorded. Check disk space before continuing.</p>}{result.durabilityWarning&&<p>The file was saved, but the drive did not confirm directory synchronization. Safely eject it and test recovery from this backup.</p>}</div>}
    <p className="field-help">To recover, lock the workspace and choose “Restore an encrypted backup” on the sign-in screen. Backups restore the whole workspace; they do not merge records. Changes after the backup will not appear in the restored view.</p>
  </section>;
}
export function RestorePanel({onRestored}:{onRestored:(result:RestoreResult)=>void}) {
  const [path,setPath]=useState(''),[password,setPassword]=useState(''),[confirmation,setConfirmation]=useState('');
  const [busy,setBusy]=useState(false),[error,setError]=useState(''),[review,setReview]=useState<RestoreReview|null>(null);
  const current=useRef(true),token=useRef<string|null>(null);
  useEffect(()=>{current.current=true;return()=>{current.current=false;if(token.current)void api.cancelWorkspaceRestore(token.current).catch(()=>{});};},[]);
  async function choose(){if(busy)return;setBusy(true);setError('');try{const file=await api.chooseRestoreFile();if(current.current&&file)setPath(file);}catch(e){if(current.current)setError(message(e));}finally{if(current.current)setBusy(false);}}
  async function verify(event:FormEvent){
    event.preventDefault();if(busy)return;setError('');
    if(!path.trim()||!password){setError('Choose an encrypted backup and enter the password used when it was created.');return;}
    const secret=password;setPassword('');setBusy(true);
    try{const value=await api.prepareWorkspaceRestore(path.trim(),secret);if(!current.current){void api.cancelWorkspaceRestore(value.token).catch(()=>{});return;}token.current=value.token;setReview(value);setConfirmation('');}
    catch(e){if(current.current)setError(message(e));}finally{if(current.current)setBusy(false);}
  }
  async function cancel(){if(!review||busy)return;setBusy(true);try{await api.cancelWorkspaceRestore(review.token);if(current.current){token.current=null;setReview(null);setConfirmation('');setError('');}}catch(e){if(current.current)setError(message(e));}finally{if(current.current)setBusy(false);}}
  async function restore(event:FormEvent){
    event.preventDefault();if(!review||busy)return;setError('');
    if(confirmation!=='RESTORE'){setError('Type RESTORE exactly to confirm this whole-workspace recovery.');return;}
    setBusy(true);
    try{const result=await api.confirmWorkspaceRestore(review.token,confirmation);token.current=null;if(current.current){setReview(null);setPath('');setConfirmation('');onRestored(result);}}
    catch(e){if(current.current){token.current=null;setReview(null);setConfirmation('');setError(message(e));}}finally{if(current.current)setBusy(false);}
  }
  return <details className="restore-panel"><summary>Restore an encrypted backup</summary><p className="field-help">Use this to recover missing or damaged workspace data, or move a backup to another computer. Close other app instances first. The current files are preserved separately; restored records replace the view and are not merged. A forgotten password cannot be reset through restore.</p>
    {!review?<form onSubmit={verify} noValidate><fieldset disabled={busy}>
      <label htmlFor="restore-file">Encrypted backup file<input id="restore-file" value={path} placeholder="Choose a backup or enter its full path" onChange={e=>setPath(e.target.value)}/></label>
      <button className="button secondary" type="button" onClick={()=>void choose()}>Choose backup file</button>
      <label htmlFor="restore-password">Password when backup was created<input id="restore-password" type="password" autoComplete="current-password" maxLength={1024} value={password} onChange={e=>setPassword(e.target.value)}/></label>
      <button className="button primary" type="submit">{busy?'Verifying backup…':'Verify backup for recovery'}</button>
    </fieldset></form>:<div className="restore-review"><h3>Verified backup — review before restoring</h3>
      <dl className="details-list"><div><dt>Organization</dt><dd>{review.organizationName}</dd></div><div><dt>Counselor</dt><dd>{review.counselorName}</dd></div><div><dt>Clients in this workspace</dt><dd>{review.clientCount}</dd></div><div><dt>Backup created</dt><dd>{review.createdAt}</dd></div></dl>
      <p className="field-help">{review.currentWorkspacePresent?'Existing workspace files will be retained separately. Changes made after this backup are absent from the restored view.':'This backup will become the workspace on this computer.'} Review expires after 15 minutes.</p>
      <form onSubmit={restore} noValidate><fieldset disabled={busy}><label htmlFor="restore-confirmation">Type RESTORE to confirm<input id="restore-confirmation" autoComplete="off" value={confirmation} onChange={e=>setConfirmation(e.target.value)}/></label><div className="budget-add"><button className="button secondary" type="button" onClick={()=>void cancel()}>Cancel recovery</button><button className="button primary" type="submit">{busy?'Restoring workspace…':'Restore verified workspace'}</button></div></fieldset></form>
    </div>}
    {error&&<div className="alert error" role="alert">{error}</div>}
  </details>;
}
