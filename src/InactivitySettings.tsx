import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import type {SecurityPolicy} from './types';
const message=(error:unknown)=>error instanceof Error?error.message:String(error);
export default function InactivitySettings({onBusy,onDirty}:{onBusy:(value:boolean)=>void;onDirty:(value:boolean)=>void}) {
 const [policy,setPolicy]=useState<SecurityPolicy|null>(null),[minutes,setMinutes]=useState('');
 const [busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState(''),[needsReload,setNeedsReload]=useState(false);
 const mounted=useRef(true),pending=useRef(false);
 const dirty=policy!==null&&minutes!==String(policy.idleMinutes);
 useEffect(()=>{onDirty(dirty);return()=>onDirty(false);},[dirty,onDirty]);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 async function load(){
  if(pending.current)return;
  pending.current=true;setBusy(true);onBusy(true);setError('');setNotice('');
  try{const value=await api.getSecurityPolicy();if(mounted.current){setPolicy(value);setMinutes(String(value.idleMinutes));setNeedsReload(false);}}
  catch(e){if(mounted.current){setError(message(e));setNeedsReload(true);}}
  finally{pending.current=false;if(mounted.current){setBusy(false);onBusy(false);}}
 }
 useEffect(()=>{mounted.current=true;void load();return()=>{mounted.current=false;};},[]);
 async function save(event:FormEvent){
  event.preventDefault();if(pending.current||!policy||needsReload)return;
  pending.current=true;setBusy(true);onBusy(true);setError('');setNotice('');
  try{const value=await api.saveSecurityPolicy({expectedRevision:policy.revision,idleMinutes:minutes});if(mounted.current){setPolicy(value);setMinutes(String(value.idleMinutes));setNotice('Inactivity timeout saved. The policy applies to all instances of this workspace.');}}
  catch(e){if(mounted.current){setError(message(e));setNeedsReload(true);}}
  finally{pending.current=false;if(mounted.current){setBusy(false);onBusy(false);}}
 }
 return <section className="panel settings-panel inactivity-settings">
  <h2>Inactivity locking</h2>
  <p>Choose 1–60 whole minutes. Existing workspaces default to 15 minutes. Automatic locking cannot be disabled.</p>
  <p>Saving starts a new inactivity interval in this instance. Other idle instances running this version use the new limit without restarting their interval and may lock immediately. Locking clears private drafts and previews. Close older application versions before changing this policy.</p>
  <p className="field-help">Workspace operations enforce the timeout, and a background check runs every 15 seconds. Background checks do not count as activity. Restore reviews and import previews still expire after 15 minutes.</p>
  {policy&&<><p className="field-help">Current saved policy: {policy.idleMinutes} minutes · Revision {policy.revision}{policy.revision===0?' (default)':''}</p>
   <form onSubmit={event=>void save(event)} noValidate><fieldset disabled={busy||needsReload}>
    <label htmlFor="inactivity-minutes">Inactivity timeout (minutes)<input id="inactivity-minutes" type="number" min="1" max="60" step="1" value={minutes} onChange={e=>{setMinutes(e.target.value);setNotice('');}}/></label>
    <button className="button primary" type="submit" disabled={!dirty}>{busy?'Saving inactivity timeout…':'Save inactivity timeout'}</button>
   </fieldset></form>
   {policy.history.length>0&&<details className="inactivity-history"><summary>Security setting history</summary><p className="field-help">Latest {policy.history.length} of {policy.totalChanges} changes. Times shown in America/Chicago. Older versions remain in encrypted storage.</p><ol>{policy.history.map(item=><li key={item.revision}>Revision {item.revision}: {item.idleMinutes} minutes — <time dateTime={item.savedAt}>{new Date(item.savedAt).toLocaleString('en-US',{timeZone:'America/Chicago'})}</time></li>)}</ol></details>}
  </>}
  {busy&&!policy&&<p role="status">Loading security policy…</p>}
  {error&&<div className="alert error" role="alert">{error} Reload the saved policy before trying again.</div>}
  {notice&&<p role="status">{notice}</p>}
  <button className="button secondary" type="button" disabled={busy} onClick={()=>{if(!dirty||window.confirm('Discard unsaved inactivity timeout edits?'))void load();}}>Reload security policy</button>
 </section>;
}
