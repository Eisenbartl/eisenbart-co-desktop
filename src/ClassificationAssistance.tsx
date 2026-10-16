import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import type {CategoryDraft,CategoryPage,CategoryQuery} from './types';
const message=(e:unknown)=>e instanceof Error?e.message:String(e);
export default function ClassificationAssistance({clientId,revision,disabled,onBusy,onDraft}:{clientId:string;revision:number;disabled:boolean;onBusy:(v:boolean)=>void;onDraft:(v:CategoryDraft)=>void}) {
 const [page,setPage]=useState<CategoryPage|null>(null),[text,setText]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const generation=useRef(0);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 async function load(query:CategoryQuery){
  const g=++generation.current;setBusy(true);setError('');setPage(null);
  try{const p=await api.queryCategorySuggestions(clientId,query);if(g!==generation.current)return;
   if(p.clientId!==clientId||p.revision!==revision)throw new Error('Suggestions do not match the loaded client records. Reload imported records.');setPage(p);
  }catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}
 }
 useEffect(()=>{setText('');void load({text:'',offset:0,expectedSnapshot:null});return()=>{generation.current++;};},[clientId,revision]);
 async function draft(recordId:string){
  if(!page||disabled||busy)return;
  const g=generation.current;setBusy(true);setError('');
  try{const d=await api.prepareCategoryDraft(clientId,{expectedRevision:page.revision,expectedSnapshot:page.snapshot,recordId});if(g!==generation.current)return;
   if(d.clientId!==clientId||d.revision!==revision||d.recordId!==recordId)throw new Error('Category draft did not match the selected client and source record.');onDraft(d);
  }catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}
 }
 return <section className="classification-assistance" aria-label="Local category suggestions">
  <h3>Local category suggestions</h3>
  <p>Missing categories on unreviewed bank expenses and fees only. Matching uses the same account, transaction type and merchant, or the exact description when no merchant was entered. Only case and repeated spaces are normalized.</p>
  <p>Two to five current approvals on distinct dates must agree. Conflicts, possible duplicates, larger evidence groups and unknown source facts remain unresolved. Suggestions do not classify income, transfers, refunds, debt roles or card records. No percentage confidence or automatic approval is used.</p>
  <form onSubmit={e=>{e.preventDefault();void load({text,offset:0,expectedSnapshot:null});}}><fieldset disabled={disabled||busy}>
   <label htmlFor="classification-search">Literal description or merchant search<input id="classification-search" maxLength={100} value={page&&page.clientId!==clientId?'':text} onChange={e=>setText(e.target.value)}/></label>
   <button className="button secondary" type="submit">Search category suggestions</button>
  </fieldset></form>
  {busy&&<p role="status">Checking category evidence…</p>}{error&&<p role="alert" className="alert error">{error} Use Reload imported records to start again.</p>}
  {page?.clientId===clientId&&page.revision===revision&&<><p className="classification-count">{page.total} unreviewed records with missing categories · Showing {page.candidates.length? page.query.offset+1:0}–{page.query.offset+page.candidates.length}</p>
   {!page.candidates.length&&<p>No missing-category expenses or fees match this search. Use Working tools for other transaction types or manual correction.</p>}
   <ul className="classification-results">{page.candidates.map(c=><li key={c.recordId} data-record-id={c.recordId}>
    <h4>{c.fields.description||'Unknown description'}</h4><p>{c.fields.date??'Unknown date'} · USD {c.fields.amount??'Unknown'} · {c.fields.kind} · {c.status}</p>
    <p>{c.accountLabel} · {c.fileName} · {c.page?`page ${c.page}, extracted line ${c.line}`:`source line ${c.line}`}</p>
    <p><strong>{c.category===null?'No suggestion':`Suggested category: ${c.category}`}</strong> · {c.reason}</p>
    <details><summary>Current approved evidence ({c.evidenceCount}; up to five shown)</summary><ul>{c.evidence.map(e=><li key={e.approvalId}>{e.fields.date??'Unknown date'} · USD {e.fields.amount??'Unknown'} · {e.fields.description} · Category {e.fields.category||'Unknown'}<br/>Source {e.documentId} · {e.page?`page ${e.page}, `:''}line {e.line} · Record {e.recordId} · Approval {e.approvalId}</li>)}</ul></details>
    <button type="button" className="button secondary" disabled={disabled||busy||c.category===null} onClick={()=>void draft(c.recordId)}>Use suggested category in unsaved review</button>
   </li>)}</ul>
   <div className="budget-add"><button type="button" className="button secondary" disabled={disabled||busy||page.query.offset===0} onClick={()=>void load({...page.query,offset:page.query.offset-page.pageSize,expectedSnapshot:page.snapshot})}>Previous suggestion page</button><button type="button" className="button secondary" disabled={disabled||busy||page.query.offset+page.pageSize>=page.total} onClick={()=>void load({...page.query,offset:page.query.offset+page.pageSize,expectedSnapshot:page.snapshot})}>Next suggestion page</button></div>
   <p className="field-help">Using a suggestion changes only an unsaved category field below. Review the original source, enter a reason and choose a decision before saving. The existing review controls enforce approval and warning checks.</p>
  </>}
 </section>;
}
