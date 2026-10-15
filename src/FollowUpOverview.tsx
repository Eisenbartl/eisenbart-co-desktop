import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import type {FollowUpQuery,FollowUpOverviewData} from './types';

const initial:FollowUpQuery={view:'open',kind:'all',includeArchived:false,search:'',page:0};
const views:[FollowUpQuery['view'],string][]=[['open','All open'],['overdue','Overdue'],['today','Due today'],['upcoming','Upcoming (after today)'],['closed','Completed / cancelled']];
const message=(e:unknown)=>e instanceof Error?e.message:String(e);

export default function FollowUpOverview({organizationId,counselorId,onOpen}:{organizationId:string;counselorId:string;onOpen:(clientId:string,followUpId:string)=>void}) {
  const [filters,setFilters]=useState(initial),[query,setQuery]=useState(initial),[attempt,setAttempt]=useState(0);
  const [result,setResult]=useState<FollowUpOverviewData|null>(null),[loading,setLoading]=useState(true),[error,setError]=useState('');
  const generation=useRef(0);
  useEffect(()=>{
    const g=++generation.current;setResult(null);setLoading(true);setError('');
    api.getFollowUpOverview(query).then(r=>{
      if(g!==generation.current)return;
      if(r.organizationId!==organizationId||r.counselorId!==counselorId||r.timeZone!=='America/Chicago'
        ||r.input.view!==query.view||r.input.kind!==query.kind||r.input.includeArchived!==query.includeArchived||r.input.search!==query.search
        ||r.input.page>query.page||r.input.page<0||r.pageSize!==25||r.items.length>25)throw new Error('The follow-up overview did not match this workspace or filter request.');
      setResult(r);
    }).catch(e=>{if(g===generation.current)setError(message(e));}).finally(()=>{if(g===generation.current)setLoading(false);});
    return()=>{generation.current++;};
  },[organizationId,counselorId,query,attempt]);
  function apply(e:FormEvent){e.preventDefault();setQuery({...filters,search:filters.search.trim(),page:0});}
  const changed=filters.view!==query.view||filters.kind!==query.kind||filters.includeArchived!==query.includeArchived||filters.search.trim()!==query.search;
  return <section className="panel follow-up-overview" aria-label="Practice appointments and follow-ups">
    <div className="panel-heading"><div><h2>Appointments and follow-ups</h2><p className="muted small">Saved tasks and appointments across your clients.</p></div><button className="button secondary" type="button" disabled={loading} onClick={()=>setAttempt(v=>v+1)}>Refresh follow-ups</button></div>
    <form className="overview-filters" onSubmit={apply}>
      <label htmlFor="follow-up-view">Follow-up view<select id="follow-up-view" value={filters.view} onChange={e=>setFilters(f=>({...f,view:e.target.value as FollowUpQuery['view']}))}>{views.map(([v,l])=><option key={v} value={v}>{l}</option>)}</select></label>
      <label htmlFor="follow-up-kind">Follow-up kind<select id="follow-up-kind" value={filters.kind} onChange={e=>setFilters(f=>({...f,kind:e.target.value as FollowUpQuery['kind']}))}><option value="all">Tasks and appointments</option><option value="task">Tasks</option><option value="appointment">Appointments</option></select></label>
      <label htmlFor="follow-up-search">Find client or follow-up<input id="follow-up-search" maxLength={100} value={filters.search} onChange={e=>setFilters(f=>({...f,search:e.target.value}))}/></label>
      <label className="overview-archived" htmlFor="follow-up-archived"><input id="follow-up-archived" type="checkbox" checked={filters.includeArchived} onChange={e=>setFilters(f=>({...f,includeArchived:e.target.checked}))}/>Include archived clients</label>
      <button className="button primary" type="submit">Apply follow-up filters</button>
    </form>
    {changed&&<p className="field-help" role="status">Filters changed. Apply follow-up filters to update the list below.</p>}
    {loading&&<p role="status">Loading saved follow-ups…</p>}
    {error&&<p className="alert error" role="alert">{error} Use Refresh follow-ups to try again.</p>}
    {result&&<>
      <p className="field-help">Reviewed {result.reviewDate} · America/Chicago · {result.input.includeArchived?'Active and archived clients':'Archived clients excluded'}. Counts cover all included clients; view, kind and search filters affect the list.</p>
      <div className="overview-counts" aria-label="Saved follow-up counts">{[['Overdue',result.overdue],['Due today',result.dueToday],['Upcoming',result.upcoming],['Completed / cancelled',result.closed]].map(([label,n])=><div key={label}><span>{label}</span><strong>{n}</strong></div>)}</div>
      <p className="overview-basis">Showing {views.find(([v])=>v===result.input.view)?.[1]} · {result.total} matching {result.total===1?'item':'items'}.</p>
      {result.items.length?<div className="table-scroll"><table className="overview-table"><caption className="sr-only">Saved follow-ups matching the applied filters</caption><thead><tr><th>Client</th><th>Follow-up</th><th>Due</th><th>Status</th><th>Review</th></tr></thead><tbody>{result.items.map(f=><tr key={`${f.clientId}:${f.followUpId}`} data-follow-up-id={f.followUpId}>
        <td>{f.clientName}{f.clientStatus==='Archived'&&<span className="muted small"> · Archived</span>}</td>
        <td><strong>{f.title}</strong><span className="muted small">{f.kind==='appointment'?'Appointment':'Task'}</span></td>
        <td>{f.dueDate}<span className="muted small">{f.kind==='appointment'?(f.time||'Time not set'):'Date only'}</span></td>
        <td>{f.status==='open'?'Open':f.status==='completed'?'Completed':'Cancelled'}</td>
        <td><button type="button" className="button secondary overview-open" aria-label={`Open ${f.title} for ${f.clientName}`} onClick={()=>onOpen(f.clientId,f.followUpId)}>Open client</button></td>
      </tr>)}</tbody></table></div>:<p className="overview-empty">{result.overdue+result.dueToday+result.upcoming+result.closed===0?'No follow-ups recorded for the included clients.':'No follow-ups match these filters.'}</p>}
      <div className="overview-pagination"><button className="button secondary" type="button" disabled={loading||result.input.page===0} onClick={()=>setQuery(q=>({...q,page:result.input.page-1}))}>Previous follow-ups</button><span>Page {result.input.page+1} of {Math.max(1,Math.ceil(result.total/result.pageSize))}</span><button className="button secondary" type="button" disabled={loading||(result.input.page+1)*result.pageSize>=result.total} onClick={()=>setQuery(q=>({...q,page:result.input.page+1}))}>Next follow-ups</button></div>
      <p className="field-help">Open a client to review or change the saved item. Times are entered local times; items without a time appear first within their date. Refresh after changes in another window or on a new day. Reminders appear inside the app.</p>
    </>}
  </section>;
}
