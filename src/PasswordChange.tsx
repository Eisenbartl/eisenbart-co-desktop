import {useEffect, useRef, useState, type FormEvent} from 'react';
import {api} from './api';
import type {PasswordChangeResult} from './types';

export default function PasswordChange({onChanged,onBusy,onDirty}:{onChanged:(result:PasswordChangeResult)=>void;onBusy:(busy:boolean)=>void;onDirty:(dirty:boolean)=>void}) {
  const [currentPassword,setCurrent]=useState('');
  const [newPassword,setNew]=useState('');
  const [confirmation,setConfirmation]=useState('');
  const [acknowledge,setAcknowledge]=useState(false);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const mounted=useRef(true),submitting=useRef(false);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;};},[]);
  useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
  useEffect(()=>{onDirty(Boolean(currentPassword||newPassword||confirmation||acknowledge));return()=>onDirty(false);},[currentPassword,newPassword,confirmation,acknowledge,onDirty]);
  function clear(){setCurrent('');setNew('');setConfirmation('');setAcknowledge(false);}
  async function submit(event:FormEvent){
    event.preventDefault();if(submitting.current)return;
    setError('');
    if(!currentPassword||Array.from(newPassword).length<12||Array.from(newPassword).length>1024){setError('Enter the current password and a new password with 12 to 1,024 characters.');return;}
    if(newPassword!==confirmation){setError('The new passwords do not match.');return;}
    if(currentPassword===newPassword){setError('Choose a new password different from the current password.');return;}
    if(!acknowledge){setError('Confirm that older encrypted copies keep their original passwords.');return;}
    const input={currentPassword,newPassword,confirmation,acknowledgeRetainedCopies:acknowledge};
    submitting.current=true;clear();setBusy(true);onBusy(true);onDirty(false);
    try{const result=await api.changeWorkspacePassword(input);if(mounted.current)onChanged(result);}
    catch(caught){if(mounted.current)setError(caught instanceof Error?caught.message:String(caught));}
    finally{submitting.current=false;if(mounted.current)setBusy(false);}
  }
  return <section className="panel settings-panel password-change">
    <h2>Change workspace password</h2>
    <p>Your current password is required. The app creates and verifies a new encrypted workspace copy before switching. Close other app instances first.</p>
    <p>The previous workspace files are retained for recovery. Older backups and retained copies still open with their original passwords; changing this password does not rewrite or revoke them. Keep those passwords securely. There is no forgotten-password reset.</p>
    <p>After a successful change, the workspace locks and clears private drafts and previews. Unlock with the new password, check your records, then create and verify a fresh backup.</p>
    <form onSubmit={event=>void submit(event)} noValidate><fieldset disabled={busy}>
      <label htmlFor="password-change-current">Current password<input id="password-change-current" type="password" autoComplete="current-password" maxLength={2048} value={currentPassword} onChange={e=>setCurrent(e.target.value)}/></label>
      <label htmlFor="password-change-new">New password<input id="password-change-new" type="password" autoComplete="new-password" maxLength={2048} value={newPassword} onChange={e=>setNew(e.target.value)}/></label>
      <p className="field-help">Use 12 to 1,024 characters. Passwords are not trimmed.</p>
      <label htmlFor="password-change-confirmation">Confirm new password<input id="password-change-confirmation" type="password" autoComplete="new-password" maxLength={2048} value={confirmation} onChange={e=>setConfirmation(e.target.value)}/></label>
      <label className="check-label"><input id="password-change-acknowledge" type="checkbox" checked={acknowledge} onChange={e=>setAcknowledge(e.target.checked)}/>I understand that older backups and retained workspace copies keep their original passwords.</label>
      {error&&<div className="alert error" role="alert">{error}</div>}
      <div className="budget-add"><button type="button" className="button secondary" onClick={()=>{clear();setError('');}}>Clear password fields</button><button className="button primary" type="submit">{busy?'Changing workspace password…':'Change password and lock'}</button></div>
    </fieldset></form>
  </section>;
}
