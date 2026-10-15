import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import type {StatementAccountInput,StatementAccountsDirectory} from './types';
const text=(e:unknown)=>e instanceof Error?e.message:String(e);
const when=(v:string)=>new Intl.DateTimeFormat('en-US',{dateStyle:'medium',timeStyle:'short',timeZone:'America/Chicago'}).format(new Date(v));
export default function StatementAccounts({clientId,onDirty,onBusy}:{clientId:string;onDirty:(v:boolean)=>void;onBusy:(v:boolean)=>void}){
 const [saved,setSaved]=useState<StatementAccountsDirectory|null>(null),[draft,setDraft]=useState<StatementAccountInput|null>(null),[base,setBase]=useState('');
 const [loading,setLoading]=useState(true),[saving,setSaving]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState(''),[attempt,setAttempt]=useState(0),[filter,setFilter]=useState('active');
 const generation=useRef(0);const busy=loading||saving;const dirty=draft!==null&&JSON.stringify(draft)!==base;
 useEffect(()=>{onDirty(dirty);return()=>onDirty(false);},[dirty,onDirty]);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 useEffect(()=>{
  const g=++generation.current;setSaved(null);setDraft(null);setBase('');setLoading(true);setSaving(false);setError('');setNotice('');setFilter('active');
  api.getStatementAccounts(clientId).then(r=>{if(g!==generation.current)return;if(r.clientId!==clientId)throw new Error('The account response did not match this client.');setSaved(r);}).catch(e=>{if(g===generation.current)setError(text(e));}).finally(()=>{if(g===generation.current)setLoading(false);});
  return()=>{generation.current++;};
 },[clientId,attempt]);
 function open(id:string|null){if(!saved||busy||dirty)return;const a=saved.accounts.find(a=>a.id===id);const d:StatementAccountInput={id,expectedRevision:a?.revision??0,expectedImportRevision:saved.importRevision,label:a?.label??'',kind:a?.kind??'checking',archived:a?.archived??false};setDraft(d);setBase(id?JSON.stringify(d):'');setError('');setNotice('');}
 async function save(e:FormEvent){e.preventDefault();if(!draft||!saved||busy||!dirty)return;const g=generation.current;setSaving(true);setError('');setNotice('');try{const r=await api.saveStatementAccount(clientId,draft);if(g!==generation.current)return;const a=draft.id?r.accounts.find(a=>a.id===draft.id):r.accounts.find(a=>!saved.accounts.some(old=>old.id===a.id));if(r.clientId!==clientId||!a||a.revision!==draft.expectedRevision+1)throw new Error('The saved account did not match this client or expected version. Discard and reload before continuing.');setSaved(r);setDraft(null);setBase('');setNotice('Statement account saved locally.');}catch(e){if(g===generation.current)setError(text(e));}finally{if(g===generation.current)setSaving(false);}}
 const selected=saved?.accounts.find(a=>a.id===draft?.id);
 return <section className="panel statement-accounts" aria-labelledby="accounts-title"><p className="eyebrow">CLIENT DOCUMENT ACCOUNTS</p><h2 id="accounts-title">Statement accounts</h2>
  <p className="muted small">Create checking or savings accounts before importing statements. Current names and archive status are managed here; original names remain in financial evidence, collection links and saved reports. These accounts do not create balances or change planning accounts.</p>
  {loading&&<p role="status">Loading statement accounts…</p>}{error&&<p role="alert" className="alert error">{error}</p>}{notice&&<p role="status" className="success-message">{notice}</p>}
  {!loading&&!saved&&<button className="button secondary" onClick={()=>setAttempt(v=>v+1)}>Retry account loading</button>}
  {saved&&<><p>{saved.accounts.length} of 20 account slots used (including archived accounts).</p><label htmlFor="accounts-filter">Show accounts<select id="accounts-filter" value={filter} onChange={e=>setFilter(e.target.value)}><option value="active">Active accounts</option><option value="archived">Archived accounts</option><option value="all">All accounts</option></select></label>
   <div className="table-scroll"><table className="collection-table"><caption className="sr-only">Statement account directory</caption><thead><tr><th>Current name</th><th>Type / status</th><th>Sources</th><th>Manage</th></tr></thead><tbody>{saved.accounts.filter(a=>filter==='all'||a.archived===(filter==='archived')).map(a=><tr key={a.id}><td><strong>{a.label}</strong><span>Original name: {a.originalLabel}</span></td><td>{a.kind} · {a.archived?'Archived':'Active'}</td><td>{a.sourceCount} documents</td><td><button className="text-button" disabled={busy||dirty} onClick={()=>open(a.id)} aria-label={`Manage account ${a.label}`}>Manage</button></td></tr>)}</tbody></table></div>
   {!saved.accounts.length&&<p className="empty-note">No statement accounts yet. Add an account or create one during import.</p>}
   <button className="button secondary" disabled={busy||dirty||saved.accounts.length>=20} onClick={()=>open(null)}>Add statement account</button>
   {draft&&<form onSubmit={e=>void save(e)} noValidate><fieldset disabled={busy}><h3>{draft.id?'Manage statement account':'New statement account'}</h3><label htmlFor="account-label">Current account name<input id="account-label" maxLength={120} value={draft.label} onChange={e=>setDraft({...draft,label:e.target.value})}/></label><label htmlFor="account-kind">Account type<select id="account-kind" disabled={draft.id!==null} value={draft.kind} onChange={e=>setDraft({...draft,kind:e.target.value as StatementAccountInput['kind']})}><option value="checking">Checking</option><option value="savings">Savings</option></select></label>
    {draft.id&&<label className="checkbox-label"><input id="account-archived" type="checkbox" checked={draft.archived} onChange={e=>setDraft({...draft,archived:e.target.checked})}/>Archive this account (keep documents and history)</label>}
    <p className="field-help">Archived accounts remain available for historical review. Reactivate an account by clearing Archive and saving. Original names and account types cannot be changed; names remain reserved to prevent confusing source associations.</p>
    <button className="button primary" disabled={!dirty} type="submit">Save statement account</button><button className="text-button" type="button" onClick={()=>{setDraft(null);setBase('');setError('');}}>Discard account edits</button>
   </fieldset>
   {selected&&<section aria-label="Account change history"><h3>Account change history</h3><p>Original name: {selected.originalLabel} · Current saved version {selected.revision}</p>{selected.history.length?<ol>{selected.history.slice().reverse().map(v=><li key={v.revision}>Version {v.revision}: {v.label} · {v.archived?'Archived':'Active'} · {when(v.updatedAt)} (America/Chicago)</li>)}</ol>:<p>No metadata changes since the account was created during import.</p>}</section>}</form>}
   <button className="text-button" disabled={busy||dirty} onClick={()=>setAttempt(v=>v+1)}>Refresh statement accounts</button>
  </>}
 </section>;
}
