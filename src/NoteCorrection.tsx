import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import type {NoteThread} from './types';
const message=(e:unknown)=>e instanceof Error?e.message:String(e);
export default function NoteCorrection({clientId,noteId,disabled,onBusy,onDirty,onSaved,onClose}:{clientId:string;noteId:string;disabled:boolean;onBusy:(v:boolean)=>void;onDirty:(v:boolean)=>void;onSaved:()=>void;onClose:()=>void}){
 const [thread,setThread]=useState<NoteThread|null>(null),[body,setBody]=useState(''),[reason,setReason]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const generation=useRef(0);
 const current=thread?.amendments.at(-1)?.note??thread?.original;
 const dirty=reason!==''||(current!==undefined&&body!==current.body);
 useEffect(()=>{onDirty(dirty);return()=>onDirty(false);},[dirty,onDirty]);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 function checked(t:NoteThread){if(t.clientId!==clientId||t.original.clientId!==clientId||t.amendments.some(a=>a.note.clientId!==clientId)||![t.original.id,...t.amendments.map(a=>a.note.id)].includes(noteId))throw new Error('Note history did not match this client and note.');return t;}
 async function load(){const g=++generation.current;setBusy(true);setError('');setThread(null);setBody('');setReason('');try{const t=checked(await api.getNoteHistory(clientId,noteId));if(g!==generation.current)return;setThread(t);setBody((t.amendments.at(-1)?.note??t.original).body);}catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}}
 useEffect(()=>{void load();return()=>{generation.current++;};},[clientId,noteId]);
 async function save(){if(!thread||!current||busy||disabled)return;const g=generation.current;setBusy(true);setError('');try{checked(await api.correctClientNote(clientId,{noteId:current.id,expectedFingerprint:thread.fingerprint,body,reason}));if(g!==generation.current)return;onSaved();}catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setBusy(false);}}
 return <section className="note-correction" aria-label="Reviewed note correction"><h3>Reviewed note correction</h3><p>The original note and every correction remain unchanged in history. Saving creates a new dated version with a reason. Earlier reports keep their original note text and require review; new report selections use the current version.</p>
 {busy&&<p role="status">Working on note history…</p>}{error&&<p role="alert" className="alert error">{error}</p>}
 {thread&&thread.clientId===clientId&&<><details className="note-version-history"><summary>Original note and correction history ({thread.amendments.length+1})</summary><article><h4>Original note</h4><p>{thread.original.body}</p><p>{thread.original.createdAt} · {thread.original.counselorName}</p></article>{thread.amendments.map(a=><article key={a.note.id}><h4>Correction {a.revision}</h4><p>{a.note.body}</p><p>Reason: {a.reason}</p><p>{a.note.createdAt} · {a.note.counselorName}</p></article>)}</details>
 <form onSubmit={e=>{e.preventDefault();void save();}}><fieldset disabled={disabled||busy}><label htmlFor="corrected-note">Corrected session note<textarea id="corrected-note" rows={5} maxLength={8000} value={body} onChange={e=>setBody(e.target.value)}/></label><label htmlFor="note-correction-reason">Note correction reason<textarea id="note-correction-reason" rows={2} maxLength={500} value={reason} onChange={e=>setReason(e.target.value)}/></label><button className="button primary" type="submit">Save reviewed note correction</button></fieldset></form></>}
 <button className="button secondary" disabled={busy||disabled} onClick={()=>{if(!dirty||window.confirm('Discard unsaved note correction and reload its history?'))void load();}}>Reload note history</button>
 <button className="button secondary" disabled={busy} onClick={()=>{if(!dirty||window.confirm('Discard this unsaved note correction?'))onClose();}}>Close note correction</button>
 </section>;
}
