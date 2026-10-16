import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import type {AssessmentDirectory,ReportPresentationContext,ReportPresentationKind,ReportPresentationLayout,ReportPresentationExport} from './types';
const noop=(_v:boolean)=>{};
const message=(e:unknown)=>e instanceof Error?e.message:String(e);
export default function ReportPresentation({clientId,onBusy=noop}:{clientId:string;onBusy?:(v:boolean)=>void}) {
 const [directory,setDirectory]=useState<AssessmentDirectory|null>(null),[id,setId]=useState(''),[context,setContext]=useState<ReportPresentationContext|null>(null);
 const [kind,setKind]=useState<ReportPresentationKind>('internal'),[layout,setLayout]=useState<ReportPresentationLayout>('summaryAppendix'),[path,setPath]=useState('');
 const [busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState('');
 const [saved,setSaved]=useState<ReportPresentationExport|null>(null),[consent,setConsent]=useState(false),[opened,setOpened]=useState(false);
 function clearSaved(){setSaved(null);setConsent(false);setOpened(false);}
 const generation=useRef(0);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 async function loadDirectory(){
  const g=++generation.current;clearSaved();setBusy(true);setError('');setNotice('');setDirectory(null);setId('');setContext(null);setPath('');setKind('internal');
  try{const d=await api.listAssessmentRecords(clientId);if(g!==generation.current)return;if(d.clientId!==clientId)throw new Error('Saved reports did not match the selected client.');setDirectory(d);}
  catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}
 }
 useEffect(()=>{setLayout('summaryAppendix');void loadDirectory();return()=>{generation.current++;};},[clientId]);
 async function select(assessmentId:string,refresh=false){
  const g=++generation.current;clearSaved();setId(assessmentId);setContext(null);setError('');setNotice('');if(!refresh){setPath('');setKind('internal');}if(!assessmentId)return;
  setBusy(true);
  try{const c=await api.getReportPresentation(clientId,assessmentId);if(g!==generation.current)return;
   if(c.clientId!==clientId||c.assessmentId!==assessmentId||c.internal.recordId!==assessmentId)throw new Error('Report presentation did not match the selected client and assessment.');
   setContext(c);if(!c.finalReport)setKind('internal');if(refresh)setNotice('Presentation refreshed. Review the summary and current context before exporting.');
  }catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}
 }
 const summary=context?.clientId===clientId?(kind==='final'?context.finalReport:context.internal):null;
 async function exportPdf(choose:boolean){
  if(!context||!summary||busy)return;
  const g=generation.current;setBusy(true);setError('');setNotice('');
  clearSaved();
  try{let target=path;
   if(choose){const p=await api.chooseReportPresentationPdf(clientId,id,kind,layout);if(g!==generation.current||p===null)return;target=p;setPath(target);}
   if(!target.trim())throw new Error('Choose a local PDF destination.');
   const result=await api.exportReportPresentation(clientId,{assessmentId:id,kind,layout,expectedReceipt:context.receipt,path:target});
   if(g!==generation.current)return;if(result.recordId!==summary.recordId)throw new Error('PDF export did not match the selected report.');
   setSaved(result);setNotice(`Report PDF saved (${result.pages} pages): ${result.path}`);
  }catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}
 }
 async function openSaved(){
  if(!saved?.viewerSupported||!consent||opened||busy)return;
  const g=generation.current;setBusy(true);setError('');setNotice('');
  try{const result=await api.openReportPdf(clientId,saved.viewerTicket);if(g!==generation.current)return;
   if(result.path!==saved.path)throw new Error('Windows opening response did not match this exported PDF. Check your viewer before trying again.');
   if(result.status==='cancelled')setNotice('The Windows opening request was cancelled. No printing was confirmed. Your exported PDF is unchanged.');
   else if(result.status==='requested'){setOpened(true);setNotice('Windows accepted the PDF opening request. Choose Print in your PDF viewer and select your printer there. This app cannot confirm viewer display or printing.');}
   else throw new Error('Windows returned an unknown opening result. Check your viewer before trying again.');
  }catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}
 }
 return <section className="panel report-presentation" aria-label="Report print and presentation">
  <h2>Print & presentation</h2>
  <p>Create a printable counselor PDF with client, period and financial summary first, followed by the detailed evidence appendix. The original detailed layout is also available. Both layouts use the same saved report figures and original firm identity.</p>
  <p className="field-help">These counselor reports may include private notes, original source references and approval details. Use the separate client action-plan workflow in Working tools for a client handout. After exporting, you can ask Windows to open the saved PDF, then choose Print in your PDF viewer. Printer selection, cancellation and job status are managed by that viewer.</p>
  <button className="button secondary" type="button" disabled={busy} onClick={()=>void loadDirectory()}>Reload saved reports</button>
  {busy&&<p role="status">Working on report presentation…</p>}{error&&<p role="alert" className="alert error">{error}</p>}{notice&&<p role="status" className="success-message">{notice}</p>}
  {directory?.clientId===clientId&&<><fieldset disabled={busy}>
   <label htmlFor="presentation-assessment">Saved assessment for presentation<select id="presentation-assessment" value={id} onChange={e=>void select(e.target.value)}><option value="">Choose a saved assessment ({directory.records.length})</option>{directory.records.map(r=><option key={r.id} value={r.id}>{r.title} · {r.periodStart} through {r.periodEnd}{r.needsReview?' · Needs review':''}</option>)}</select></label>
   {directory.records.length===0&&<p>No saved assessments for this client. Record one in Reports → Working tools first.</p>}
   {id&&<button type="button" className="button secondary" onClick={()=>void select(id,true)}>Refresh presentation preview</button>}
   {context?.clientId===clientId&&<div className="form-grid">
    <label htmlFor="presentation-kind">Counselor report to export<select id="presentation-kind" value={kind} onChange={e=>{clearSaved();setKind(e.target.value as ReportPresentationKind);setPath('');setNotice('');setError('');}}><option value="internal">Internal assessment (not a final approval)</option><option value="final" disabled={!context.finalReport}>{context.finalReport?'Original final counselor approval':'No final approval recorded'}</option></select></label>
    <label htmlFor="presentation-layout">Report PDF layout<select id="presentation-layout" value={layout} onChange={e=>{clearSaved();setLayout(e.target.value as ReportPresentationLayout);setPath('');setNotice('');setError('');}}><option value="summaryAppendix">Summary with evidence appendix</option><option value="detailed">Original detailed layout</option></select></label>
   </div>}
  </fieldset>
  {summary&&<>
   <section className="presentation-summary" aria-label="Saved report summary preview">
    <p className="eyebrow">{summary.firmName}</p><h3>{summary.title}</h3><p><strong>{summary.clientName}</strong> · {summary.periodStart} through {summary.periodEnd}</p>
    <p>Report date {summary.reportDate} (America/Chicago) · {summary.status}</p>
    {summary.needsReview&&<p className="alert warning">Current context needs review. The PDF preserves original figures and includes this historical status.</p>}
    <p>{summary.basis}</p><dl className="budget-totals presentation-figures">{summary.figures.map(f=><div key={f.label}><dt>{f.label}</dt><dd>{f.value===null?'Unknown':`USD ${f.value}`}</dd></div>)}</dl>
    <p>Balance facts as of {summary.balanceAsOf??'Unknown'}.</p><p className="field-help">{summary.disclosure}</p>
    {summary.reviewReasons.length>0&&<details><summary>Current review reasons</summary><ul>{summary.reviewReasons.map((r,i)=><li key={i}>{r}</li>)}</ul></details>}
    <p className="field-help">Original record {summary.recordId} · SHA-256 {summary.digest}. This is a record integrity check, not a digital signature. Summary presentation {context?.presentation}; financial capture and approval versions are unchanged.</p>
   </section>
   <fieldset disabled={busy}><p>PDFs contain readable confidential information outside workspace encryption. Choose a private local folder and a new filename. Existing files are never overwritten.</p>
    <label htmlFor="presentation-pdf-path">Presentation PDF destination<input id="presentation-pdf-path" value={path} onChange={e=>{clearSaved();setPath(e.target.value);}}/></label>
    <div className="budget-add"><button type="button" className="button secondary" onClick={()=>void exportPdf(true)}>Choose and export report PDF</button><button type="button" className="button primary" disabled={!path.trim()} onClick={()=>void exportPdf(false)}>Export presentation PDF</button></div>
   </fieldset>
   {saved&&<section className="saved-pdf-viewer" aria-label="Open exported PDF to print">
    <h3>Open saved PDF to print</h3><p className="saved-pdf-path">{saved.path}</p>
    <p>This readable export remains in your chosen folder after lock, sign-out, restart or recovery. A PDF viewer may retain its own copy or recent-file history. Close the viewer and manage the exported file yourself when finished. This app does not delete it.</p>
    {saved.viewerSupported?<fieldset disabled={busy||opened}>
     <label htmlFor="presentation-viewer-consent"><input id="presentation-viewer-consent" type="checkbox" checked={consent} onChange={e=>setConsent(e.target.checked)}/>I want to open this confidential export in my Windows PDF viewer.</label>
     <button className="button secondary" type="button" disabled={!consent} onClick={()=>void openSaved()}>Open saved PDF to print</button>
     <p className="field-help">Opening is available for the most recent export for 10 minutes in this unlocked session. If unavailable, open the saved PDF manually. This action does not submit a print job.</p>
    </fieldset>:<p>Opening from this app is available on Windows only. Open this exported PDF manually in a local viewer and choose Print.</p>}
   </section>}
  </>}
  </>}
 </section>;
}
