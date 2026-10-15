import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {beforeEach,describe,expect,it,vi} from 'vitest';
import FollowUpOverview from './FollowUpOverview';
import type {FollowUpItem,FollowUpOverviewData,FollowUpQuery} from './types';
const mock=vi.hoisted(()=>({getFollowUpOverview:vi.fn()}));
vi.mock('./api',()=>({api:mock}));
const input:FollowUpQuery={view:'open',kind:'all',includeArchived:false,search:'',page:0};
const item:FollowUpItem={clientId:'client-a',clientName:'Alex Fictional',clientStatus:'Planning',followUpId:'saved-follow-up',kind:'appointment',title:'Review statements',dueDate:'2026-10-15',time:'',status:'open',careRevision:2};
const base:FollowUpOverviewData={organizationId:'org',counselorId:'counselor',reviewDate:'2026-10-15',timeZone:'America/Chicago',input,overdue:0,dueToday:1,upcoming:0,closed:0,total:1,pageSize:25,items:[item]};
const open=vi.fn();
const props={organizationId:'org',counselorId:'counselor',onOpen:open};
function deferred<T>(){let resolve!:(v:T)=>void;const promise=new Promise<T>(r=>{resolve=r;});return {promise,resolve};}
beforeEach(()=>{vi.clearAllMocks();mock.getFollowUpOverview.mockImplementation(async(q:FollowUpQuery)=>({...base,input:q}));});
describe('practice follow-up overview',()=>{
  it('shows saved appointments without inventing a time and opens the exact client/item',async()=>{
    render(<FollowUpOverview {...props}/>);await screen.findByText('Review statements');
    expect(screen.getByText('Time not set')).toBeInTheDocument();
    expect(screen.getByText(/Archived clients excluded/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button',{name:'Open Review statements for Alex Fictional'}));
    expect(open).toHaveBeenCalledWith('client-a','saved-follow-up');
  });
  it('applies explicit filters and paginates the applied query',async()=>{
    mock.getFollowUpOverview.mockImplementation(async(q:FollowUpQuery)=>({...base,input:q,total:26,items:[{...item,title:q.page?'Second page appointment':'First page appointment'}]}));
    render(<FollowUpOverview {...props}/>);await screen.findByText('First page appointment');
    fireEvent.change(screen.getByLabelText('Follow-up view'),{target:{value:'upcoming'}});
    fireEvent.change(screen.getByLabelText('Follow-up kind'),{target:{value:'appointment'}});
    fireEvent.change(screen.getByLabelText('Find client or follow-up'),{target:{value:'  Alex  '}});
    fireEvent.click(screen.getByLabelText('Include archived clients'));
    expect(mock.getFollowUpOverview).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button',{name:'Apply follow-up filters'}));
    await waitFor(()=>expect(mock.getFollowUpOverview).toHaveBeenLastCalledWith({...input,view:'upcoming',kind:'appointment',includeArchived:true,search:'Alex'}));
    await screen.findByText(/Active and archived clients/);
    fireEvent.click(screen.getByRole('button',{name:'Next follow-ups'}));
    await screen.findByText('Second page appointment');
    expect(mock.getFollowUpOverview).toHaveBeenLastCalledWith({...input,view:'upcoming',kind:'appointment',includeArchived:true,search:'Alex',page:1});
    expect(screen.getByText('Page 2 of 2')).toBeInTheDocument();
    expect(screen.getByRole('button',{name:'Next follow-ups'})).toBeDisabled();
  });
  it('clears stale results on refresh and rejects workspace/filter mismatches',async()=>{
    render(<FollowUpOverview {...props}/>);await screen.findByText('Review statements');
    mock.getFollowUpOverview.mockRejectedValueOnce(new Error('Storage unavailable'));
    fireEvent.click(screen.getByRole('button',{name:'Refresh follow-ups'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('Storage unavailable');
    expect(screen.queryByText('Review statements')).not.toBeInTheDocument();
    mock.getFollowUpOverview.mockResolvedValueOnce({...base,organizationId:'foreign'});
    fireEvent.click(screen.getByRole('button',{name:'Refresh follow-ups'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match this workspace');
    mock.getFollowUpOverview.mockResolvedValueOnce({...base,input:{...input,view:'closed'}});
    fireEvent.click(screen.getByRole('button',{name:'Refresh follow-ups'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('filter request');
    expect(screen.queryByText('Review statements')).not.toBeInTheDocument();
  });
  it('ignores a delayed response superseded by another filter request',async()=>{
    const pending=deferred<FollowUpOverviewData>();mock.getFollowUpOverview.mockReturnValueOnce(pending.promise);
    render(<FollowUpOverview {...props}/>);
    fireEvent.change(screen.getByLabelText('Follow-up view'),{target:{value:'closed'}});
    mock.getFollowUpOverview.mockResolvedValueOnce({...base,input:{...input,view:'closed'},items:[],total:0,closed:1});
    fireEvent.click(screen.getByRole('button',{name:'Apply follow-up filters'}));
    await screen.findByText('No follow-ups match these filters.');
    await act(async()=>pending.resolve(base));
    expect(screen.queryByText('Review statements')).not.toBeInTheDocument();
  });
  it('clears private rows on workspace changes and handles an empty workspace',async()=>{
    const view=render(<FollowUpOverview {...props}/>);await screen.findByText('Review statements');
    const pending=deferred<FollowUpOverviewData>();mock.getFollowUpOverview.mockReturnValueOnce(pending.promise);
    view.rerender(<FollowUpOverview {...props} organizationId="other"/>);
    expect(screen.queryByText('Review statements')).not.toBeInTheDocument();
    await act(async()=>pending.resolve({...base,organizationId:'other',items:[],total:0,dueToday:0}));
    expect(await screen.findByText('No follow-ups recorded for the included clients.')).toBeInTheDocument();
  });
});
