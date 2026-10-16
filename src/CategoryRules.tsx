import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import type {CategoryRuleInput,CategoryRulesDirectory,CategoryRuleVersion} from './types';
const message=(e:unknown)=>e instanceof Error?e.message:String(e);
export default function CategoryRules({clientId,disabled,onDirty,onBusy,onSaved,revision=0}:{revision?:number;clientId:string;disabled:boolean;onDirty:(v:boolean)=>void;onBusy:(v:boolean)=>void;onSaved:()=>void}){
 const [directory,setDirectory]=useState<CategoryRulesDirectory|null>(null),[draft,setDraft]=useState<CategoryRuleInput|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState('');
 const generation=useRef(0);
 useEffect(()=>{onDirty(draft!==null);return()=>onDirty(false);},[draft,onDirty]);useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 async function load(){const g=++generation.current;setBusy(true);setError('');setNotice('');setDirectory(null);setDraft(null);try{const d=await api.getCategoryRules(clientId);if(g!==generation.current)return;if(d.clientId!==clientId)throw new Error('Rules did not match the selected client.');setDirectory(d);}catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}}
 useEffect(()=>{void load();return()=>{generation.current++;};},[clientId,revision]);
 function edit(v:CategoryRuleVersion|null){if(!directory)return;setError('');setNotice('');setDraft({id:v?.id??null,expectedRevision:v?.revision??0,expectedFingerprint:directory.fingerprint,reason:'',data:v?{...v.data}:{label:'',accountId:directory.accounts.find(a=>!a.archived)?.id??'',kind:'expense',mode:'merchant',pattern:'',category:'',archived:false}});}
 async function save(){if(!draft||!directory||busy||disabled)return;const g=generation.current;setBusy(true);setError('');setNotice('');try{const d=await api.saveCategoryRule(clientId,draft);if(g!==generation.current)return;if(d.clientId!==clientId)throw new Error('Saved rule result did not match the client. Reload rules to check the stored result.');setDirectory(d);setDraft(null);setNotice('Category rule saved. No transaction or financial figures were changed.');onSaved();}catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}}
 const current=directory?.clientId===clientId?directory:null;
 return <section className="category-rules" aria-label="Reusable category rules"><h3>Reusable category rules</h3>
  <p>Rules belong to this client and one bank account, for expenses or fees only. Match exact entered merchant text, or the description only when the merchant is blank; case and repeated spaces are normalized. Rules propose an unsaved category review and never approve transactions automatically.</p>
  <p>Conflicts with current approved categories remain unresolved. Editing or archiving a rule preserves prior versions and saved applications. Archived accounts cannot activate rules.</p>
  {busy&&<p role="status">Working on category rules…</p>}{error&&<p role="alert" className="alert error">{error}</p>}{notice&&<p role="status" className="success-message">{notice}</p>}
  <button type="button" className="button secondary" disabled={busy||disabled||draft!==null} onClick={()=>void load()}>Reload category rules</button>
  {current&&<>
   <button type="button" className="button secondary" disabled={busy||disabled||draft!==null||!current.accounts.some(a=>!a.archived)} onClick={()=>edit(null)}>Create category rule</button>
   {!current.accounts.some(a=>!a.archived)&&<p>Add or reactivate a bank account in Documents → Statement accounts before creating a rule.</p>}
   {!current.rules.length&&<p>No category rules saved for this client.</p>}
   <ul className="category-rule-list">{current.rules.map(r=><li key={r.current.id} data-rule-id={r.current.id}>
    <h4>{r.current.data.label} · {r.current.data.archived?'Archived':'Active'} · Version {r.current.revision}</h4>
    <p>{current.accounts.find(a=>a.id===r.current.data.accountId)?.label??'Original account'} · {r.current.data.kind} · {r.current.data.mode}: {r.current.data.pattern} → {r.current.data.category}</p>
    <button type="button" className="button secondary" disabled={busy||disabled||draft!==null} onClick={()=>edit(r.current)}>Edit rule {r.current.data.label}</button>
    <details><summary>Rule history ({r.history.length})</summary><ul>{r.history.map(v=><li key={v.revision}>Version {v.revision} · {v.data.archived?'Archived':'Active'} · {v.data.label} · {v.data.mode}: {v.data.pattern} → {v.data.category}<br/>{v.updatedAt} · {v.counselorName} · {v.reason}</li>)}</ul></details>
   </li>)}</ul>
   {draft&&<form className="category-rule-form" onSubmit={e=>{e.preventDefault();void save();}}><h4>{draft.id?'Edit category rule':'Create category rule'}</h4><fieldset disabled={busy||disabled}>
    <label htmlFor="rule-label">Rule label<input id="rule-label" maxLength={120} value={draft.data.label} onChange={e=>setDraft({...draft,data:{...draft.data,label:e.target.value}})}/></label>
    <label htmlFor="rule-account">Rule bank account<select id="rule-account" value={draft.data.accountId} onChange={e=>setDraft({...draft,data:{...draft.data,accountId:e.target.value}})}><option value="">Choose an account</option>{current.accounts.map(a=><option key={a.id} value={a.id} disabled={a.archived&&!draft.data.archived}>{a.label}{a.archived?' (archived)':''}</option>)}</select></label>
    <label htmlFor="rule-kind">Rule transaction type<select id="rule-kind" value={draft.data.kind} onChange={e=>setDraft({...draft,data:{...draft.data,kind:e.target.value as 'expense'|'fee'}})}><option value="expense">Expense</option><option value="fee">Fee</option></select></label>
    <label htmlFor="rule-mode">Rule match basis<select id="rule-mode" value={draft.data.mode} onChange={e=>setDraft({...draft,data:{...draft.data,mode:e.target.value as 'merchant'|'description'}})}><option value="merchant">Entered merchant</option><option value="description">Description (only when merchant is blank)</option></select></label>
    <label htmlFor="rule-pattern">Exact rule match text<input id="rule-pattern" maxLength={120} value={draft.data.pattern} onChange={e=>setDraft({...draft,data:{...draft.data,pattern:e.target.value}})}/></label>
    <label htmlFor="rule-category">Rule category<input id="rule-category" maxLength={80} value={draft.data.category} onChange={e=>setDraft({...draft,data:{...draft.data,category:e.target.value}})}/></label>
    <label className="rule-archive-label" htmlFor="rule-archived"><input id="rule-archived" type="checkbox" checked={draft.data.archived} onChange={e=>setDraft({...draft,data:{...draft.data,archived:e.target.checked}})}/>Archived (retain history; do not suggest)</label>
    <label htmlFor="rule-reason">Rule change reason<input id="rule-reason" maxLength={500} value={draft.reason} onChange={e=>setDraft({...draft,reason:e.target.value})}/></label>
    <button className="button primary" type="submit">Save category rule</button>
   </fieldset><button type="button" className="button secondary" disabled={busy} onClick={()=>{setDraft(null);setError('');}}>Discard rule edits</button></form>}
   <details className="rule-applications"><summary>Saved rule applications ({current.applicationCount}; most recent 25)</summary><ul>{current.applications.map(a=><li key={a.decisionId}>{a.rule.data.label} · Original version {a.rule.revision} · Category {a.category} · {a.status}<br/>{a.reviewedAt} · Record {a.recordId} · Decision {a.decisionId}<br/>Original match: {a.rule.data.mode} · {a.rule.data.pattern}</li>)}</ul></details>
  </>}
 </section>;
}
