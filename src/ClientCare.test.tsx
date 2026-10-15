import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {beforeEach,describe,expect,it,vi} from 'vitest';
import ClientCare from './ClientCare';
import type {ClientCareRecord} from './types';

const mocks=vi.hoisted(()=>({getClientCare:vi.fn(),saveClientCare:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const empty:ClientCareRecord={clientId:'a',revision:0,updatedAt:null,reviewDate:'2026-10-15',overdue:0,dueToday:0,data:{intake:{address:'',household:'',employment:'',employer:'',occupation:'',incomeNotes:''},followUps:[]}};
const dirty=vi.fn(),busy=vi.fn();
const props={clientId:'a',onDirty:dirty,onBusy:busy,otherSaving:false};
function deferred<T>() {let resolve!:(v:T)=>void;const promise=new Promise<T>(r=>{resolve=r;});return {promise,resolve};}
async function open() {fireEvent.click(screen.getByText('Intake and follow-ups'));await screen.findByLabelText('Mailing address');}
function change(label:string,value:string) {fireEvent.change(screen.getByLabelText(label),{target:{value}});}
beforeEach(()=>{vi.clearAllMocks();mocks.getClientCare.mockResolvedValue(structuredClone(empty));});

describe('client intake and follow-ups',()=>{
  it('loads optional intake and saves an explicitly dated appointment with revision protection',async()=>{
    mocks.saveClientCare.mockImplementation(async(clientId,input)=>({...empty,clientId,revision:1,updatedAt:'2026-10-15T12:00:00Z',data:{...input.data,followUps:input.data.followUps.map((f:object)=>({...f,id:'server-assigned'}))}}));
    render(<ClientCare {...props}/>);await open();
    expect(screen.getByRole('button',{name:'Save intake and follow-ups'})).toBeDisabled();
    change('Household composition','Two fictional adults and one dependent');
    change('Employment status','employed');change('Employer','Example Employer');
    fireEvent.click(screen.getByRole('button',{name:'Add follow-up'}));
    change('Kind','appointment');change('Title','Review statements');change('Due date','2026-11-02');change('Appointment time (optional)','09:30');
    await waitFor(()=>expect(dirty).toHaveBeenLastCalledWith(true));
    fireEvent.click(screen.getByRole('button',{name:'Save intake and follow-ups'}));
    await screen.findByText('Intake and follow-ups saved locally.');
    expect(mocks.saveClientCare).toHaveBeenCalledWith('a',{expectedRevision:0,data:{intake:{...empty.data.intake,household:'Two fictional adults and one dependent',employment:'employed',employer:'Example Employer'},followUps:[{id:'',kind:'appointment',title:'Review statements',dueDate:'2026-11-02',time:'09:30',status:'open',notes:''}]}});
    expect(screen.queryByRole('button',{name:'Remove unsaved follow-up 1'})).not.toBeInTheDocument();
    await waitFor(()=>expect(dirty).toHaveBeenLastCalledWith(false));
  });

  it('retains a rejected draft and requires explicit discard before reloading the saved version',async()=>{
    mocks.saveClientCare.mockRejectedValueOnce(new Error('Changed in another window'));
    render(<ClientCare {...props}/>);await open();change('Mailing address','Unsaved fictional address');
    fireEvent.click(screen.getByRole('button',{name:'Save intake and follow-ups'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('Changed in another window');
    expect(screen.getByLabelText('Mailing address')).toHaveValue('Unsaved fictional address');
    expect(mocks.saveClientCare).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button',{name:'Discard changes and reload intake'}));
    await waitFor(()=>expect(screen.getByLabelText('Mailing address')).toHaveValue(''));
    expect(mocks.getClientCare).toHaveBeenCalledTimes(2);
  });

  it('shows failed loads and rejects a response belonging to another client',async()=>{
    mocks.getClientCare.mockRejectedValueOnce(new Error('Storage unavailable'));
    render(<ClientCare {...props}/>);fireEvent.click(screen.getByText('Intake and follow-ups'));
    expect(await screen.findByRole('alert')).toHaveTextContent('Storage unavailable');
    expect(screen.queryByLabelText('Mailing address')).not.toBeInTheDocument();
    mocks.getClientCare.mockResolvedValueOnce({...empty,clientId:'foreign'});
    fireEvent.click(screen.getByRole('button',{name:'Retry intake loading'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match this client');
    expect(screen.queryByLabelText('Mailing address')).not.toBeInTheDocument();
  });

  it('ignores delayed client reads and saves after a client change',async()=>{
    const pending=deferred<ClientCareRecord>();mocks.getClientCare.mockReturnValueOnce(pending.promise);
    const view=render(<ClientCare {...props}/>);fireEvent.click(screen.getByText('Intake and follow-ups'));
    await waitFor(()=>expect(mocks.getClientCare).toHaveBeenCalledWith('a'));
    mocks.getClientCare.mockResolvedValue({...empty,clientId:'b'});view.rerender(<ClientCare {...props} clientId="b"/>);
    await screen.findByLabelText('Mailing address');
    await act(async()=>pending.resolve({...empty,data:{...empty.data,intake:{...empty.data.intake,address:'Private A'}}}));
    expect(screen.getByLabelText('Mailing address')).toHaveValue('');
    const save=deferred<ClientCareRecord>();mocks.saveClientCare.mockReturnValueOnce(save.promise);
    change('Employer','B fictional employer');fireEvent.click(screen.getByRole('button',{name:'Save intake and follow-ups'}));
    await waitFor(()=>expect(busy).toHaveBeenLastCalledWith(true));
    mocks.getClientCare.mockResolvedValue({...empty,clientId:'c'});view.rerender(<ClientCare {...props} clientId="c"/>);
    await waitFor(()=>expect(screen.getByLabelText('Employer')).toBeEnabled());
    await act(async()=>save.resolve({...empty,clientId:'b'}));
    expect(screen.getByLabelText('Employer')).toHaveValue('');
    expect(screen.queryByText('Intake and follow-ups saved locally.')).not.toBeInTheDocument();
  });

  it('keeps saved completed tasks and outcomes while respecting other writes',async()=>{
    mocks.getClientCare.mockResolvedValue({...empty,revision:1,overdue:1,data:{...empty.data,followUps:[{id:'saved-id',kind:'task',title:'Reviewed records',dueDate:'2026-10-01',time:'',status:'completed',notes:'Completed fictional review'}]}});
    const view=render(<ClientCare {...props}/>);await open();
    expect(screen.getByLabelText('Status')).toHaveValue('completed');
    expect(screen.getByLabelText('Follow-up notes / outcome')).toHaveValue('Completed fictional review');
    expect(screen.queryByRole('button',{name:'Remove unsaved follow-up 1'})).not.toBeInTheDocument();
    view.rerender(<ClientCare {...props} otherSaving/>);
    expect(screen.getByLabelText('Status')).toBeDisabled();
    expect(screen.getByRole('button',{name:'Refresh intake and reminders'})).toBeDisabled();
  });
});
