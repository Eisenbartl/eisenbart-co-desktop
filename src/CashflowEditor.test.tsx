import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {beforeEach,describe,expect,it,vi} from 'vitest';
import CashflowEditor from './CashflowEditor';
import type {Client,ClientCashflowPlan,CashflowPlanData,CashflowSummary} from './types';
const mocks=vi.hoisted(()=>({getClientCashflowPlan:vi.fn(),previewClientCashflowPlan:vi.fn(),saveClientCashflowPlan:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const client:Client={id:'client-a',organizationId:'org-a',firstName:'Alex',lastName:'Example',email:'',phone:'',householdSize:1,status:'Planning',goals:'',createdAt:'',updatedAt:''};
const context={budgetRevision:1,debtRevision:1,savingsRevision:1,monthlySurplus:'1416.67',budgetDebt:'400.00',budgetSavings:'300.00',debtMonthlyPayment:'100.00',savingsMonthlyContribution:'250.00'};
const data:CashflowPlanData={startDate:'2026-11-01',endDate:'2026-11-30',accounts:[{id:'checking',label:'Synthetic checking',openingBalance:'500.00'},{id:'reserve',label:'Synthetic reserve',openingBalance:'1000.00'}],schedules:[{label:'Synthetic rent',accountId:'checking',destinationId:null,kind:'bill',amount:'1200.00',firstDate:'2026-11-03',frequency:'once',secondDay:null,untilDate:null,certainty:'confirmed'}],notes:'Synthetic private priority'};
const summary:CashflowSummary={combinedClosing:'3300.00',closingBalances:[{accountId:'checking',amount:'2100.00'},{accountId:'reserve',amount:'1200.00'}],warnings:['Same-day debits precede credits.'],days:[{date:'2026-11-03',combinedBalance:'300.00',balances:[{accountId:'checking',amount:'-700.00'},{accountId:'reserve',amount:'1000.00'}],entryIndices:[0],warnings:['Synthetic checking: potential shortfall during the day.']}],ledger:[{scheduleIndex:0,date:'2026-11-03',label:'Synthetic rent',kind:'bill',certainty:'confirmed',accountId:'checking',destinationId:null,amount:'1200.00',sourceBalance:'-700.00',destinationBalance:null,combinedBalance:'300.00',nextPaycheck:'2026-11-06',warnings:['Payment occurs before the next entered paycheck (2026-11-06).']}]};
const empty:ClientCashflowPlan={clientId:client.id,revision:0,updatedAt:null,data:null,summary:null,context};
const saved:ClientCashflowPlan={...empty,revision:1,data,summary};
function deferred<T>(){let resolve!:(v:T)=>void;const promise=new Promise<T>(r=>{resolve=r;});return{promise,resolve};}
function open(){return render(<CashflowEditor client={client} onDirty={vi.fn()} onBusy={vi.fn()} otherSaving={false}/>);}
beforeEach(()=>{vi.clearAllMocks();mocks.getClientCashflowPlan.mockResolvedValue(empty);mocks.previewClientCashflowPlan.mockResolvedValue({...saved,revision:0});mocks.saveClientCashflowPlan.mockResolvedValue(saved);});
describe('cash-flow editor',()=>{
  it('collects exact strings and real dates without inventing activity amounts or dates',async()=>{
    open();await screen.findByText('No saved cash-flow plan yet');
    expect(screen.getByRole('button',{name:'Save cash-flow plan and calculate'})).toBeDisabled();
    fireEvent.click(screen.getByRole('button',{name:'Add cash-flow account'}));
    expect(screen.getByLabelText('Opening balance (USD)')).toHaveValue('');
    fireEvent.change(screen.getByLabelText('Account name'),{target:{value:'New checking'}});
    fireEvent.change(screen.getByLabelText('Opening balance (USD)'),{target:{value:'0'}});
    fireEvent.click(screen.getByRole('button',{name:'Add dated activity'}));
    expect(screen.getByLabelText('First activity date')).toHaveValue('');expect(screen.getByLabelText('Activity amount (USD)')).toHaveValue('');
    fireEvent.change(screen.getByLabelText('Activity name'),{target:{value:'Pay'}});
    fireEvent.change(screen.getByLabelText('First activity date'),{target:{value:'2026-11-06'}});
    fireEvent.change(screen.getByLabelText('Activity amount (USD)'),{target:{value:'1500.01'}});
    fireEvent.change(screen.getByLabelText('Repeat'),{target:{value:'biweekly'}});
    fireEvent.click(screen.getByRole('button',{name:'Save cash-flow plan and calculate'}));await screen.findByText('Cash-flow plan saved and calculated.');
    expect(mocks.saveClientCashflowPlan).toHaveBeenCalledWith(client.id,{expectedRevision:0,data:expect.objectContaining({accounts:[expect.objectContaining({openingBalance:'0'})],schedules:[expect.objectContaining({amount:'1500.01',firstDate:'2026-11-06',frequency:'biweekly',certainty:'estimated'})]})});
    expect(screen.getAllByLabelText('Opening balance (USD)')[0]).toHaveValue('500.00');
  });
  it('shows per-account warnings and the native ledger while combined balance is positive',async()=>{
    mocks.getClientCashflowPlan.mockResolvedValue(saved);open();await screen.findByText('Saved cash-flow forecast');
    fireEvent.click(screen.getByRole('button',{name:'Review 2026-11-03'}));
    expect(screen.getByText('Synthetic checking: -$700.00')).toBeInTheDocument();
    expect(screen.getByText(/potential shortfall during the day/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button',{name:'Chronological ledger'}));
    const table=screen.getByRole('table',{name:'Cash-flow chronological ledger'});
    expect(table).toHaveTextContent('-$700.00');expect(table).toHaveTextContent('$300.00');expect(table).toHaveTextContent('confirmed');expect(table).toHaveTextContent('2026-11-06');
  });
  it('previews date changes, refreshes references, and keeps the saved revision and dirty draft',async()=>{
    mocks.getClientCashflowPlan.mockResolvedValue(saved);open();await screen.findByText('Saved cash-flow forecast');
    fireEvent.change(screen.getByLabelText('First activity date'),{target:{value:'2026-11-08'}});
    expect(screen.queryByText('Saved cash-flow forecast')).not.toBeInTheDocument();
    mocks.previewClientCashflowPlan.mockResolvedValue({...saved,data:{...data,schedules:[{...data.schedules[0],firstDate:'2026-11-08'}]},context:{...context,budgetRevision:2}});
    fireEvent.click(screen.getByRole('button',{name:'Preview cash flow / refresh references'}));await screen.findByText('Draft cash-flow preview');
    expect(screen.getByLabelText('First activity date')).toHaveValue('2026-11-08');expect(screen.getByText(/budget revision 2/)).toBeInTheDocument();expect(mocks.saveClientCashflowPlan).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button',{name:'Save cash-flow plan and calculate'}));await waitFor(()=>expect(mocks.saveClientCashflowPlan).toHaveBeenCalled());
    expect(mocks.saveClientCashflowPlan.mock.calls[0][1]).toMatchObject({expectedRevision:1,data:{schedules:[expect.objectContaining({firstDate:'2026-11-08'})]}});
  });
  it('adds paired transfers and semimonthly inputs without allowing account removal in use',async()=>{
    mocks.getClientCashflowPlan.mockResolvedValue(saved);open();await screen.findByText('Saved cash-flow forecast');
    expect(screen.getByRole('button',{name:'Remove cash-flow account 1'})).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Activity type'),{target:{value:'transfer'}});
    fireEvent.change(screen.getByLabelText('Destination account'),{target:{value:'reserve'}});
    fireEvent.change(screen.getByLabelText('Repeat'),{target:{value:'semimonthly'}});
    fireEvent.change(screen.getByLabelText('Second day of month'),{target:{value:'20'}});
    fireEvent.click(screen.getByRole('button',{name:'Save cash-flow plan and calculate'}));await waitFor(()=>expect(mocks.saveClientCashflowPlan).toHaveBeenCalled());
    expect(mocks.saveClientCashflowPlan.mock.calls[0][1].data.schedules[0]).toMatchObject({kind:'transfer',destinationId:'reserve',frequency:'semimonthly',secondDay:20});
  });
  it('preserves invalid drafts and unknown amounts and blocks mismatched concurrent previews',async()=>{
    mocks.getClientCashflowPlan.mockResolvedValue({...saved,summary:{...summary,combinedClosing:null,closingBalances:[{accountId:'checking',amount:null}],ledger:[],days:[]}});open();await screen.findByText('Saved cash-flow forecast');expect(screen.getAllByText('Unknown').length).toBeGreaterThan(0);
    fireEvent.change(screen.getByLabelText('Activity amount (USD)'),{target:{value:'1.001'}});
    mocks.saveClientCashflowPlan.mockRejectedValueOnce('Use at most two decimal places.');
    fireEvent.click(screen.getByRole('button',{name:'Save cash-flow plan and calculate'}));await screen.findByRole('alert');expect(screen.getByLabelText('Activity amount (USD)')).toHaveValue('1.001');
    mocks.previewClientCashflowPlan.mockResolvedValueOnce({...saved,revision:2});fireEvent.click(screen.getByRole('button',{name:'Preview cash flow / refresh references'}));await screen.findByText(/response did not match/);expect(screen.queryByText('Draft cash-flow preview')).not.toBeInTheDocument();
  });
  it('blocks replacement after failed loads and retries without saving',async()=>{
    mocks.getClientCashflowPlan.mockRejectedValueOnce('Storage unavailable.');open();await screen.findByRole('alert');expect(screen.queryByRole('button',{name:'Save cash-flow plan and calculate'})).not.toBeInTheDocument();
    mocks.getClientCashflowPlan.mockResolvedValueOnce(saved);fireEvent.click(screen.getByRole('button',{name:'Retry loading cash-flow plan'}));await waitFor(()=>expect(screen.getAllByLabelText('Account name')[0]).toHaveValue('Synthetic checking'));expect(mocks.saveClientCashflowPlan).not.toHaveBeenCalled();
  });
  it('discards old loads and previews after changing clients or locking',async()=>{
    const pending=deferred<ClientCashflowPlan>();mocks.getClientCashflowPlan.mockReturnValueOnce(pending.promise);
    const view=open();const next={...client,id:'client-b'};mocks.getClientCashflowPlan.mockResolvedValueOnce({...empty,clientId:next.id});view.rerender(<CashflowEditor client={next} onDirty={vi.fn()} onBusy={vi.fn()} otherSaving={false}/>);
    await screen.findByText('No saved cash-flow plan yet');await act(async()=>pending.resolve(saved));expect(screen.queryByDisplayValue('Synthetic checking')).not.toBeInTheDocument();
    view.unmount();mocks.getClientCashflowPlan.mockResolvedValueOnce(saved);const fresh=open();await screen.findByText('Saved cash-flow forecast');
    const calculation=deferred<ClientCashflowPlan>();mocks.previewClientCashflowPlan.mockReturnValueOnce(calculation.promise);fireEvent.click(screen.getByRole('button',{name:'Preview cash flow / refresh references'}));expect(screen.getByRole('button',{name:'Calculating cash flow…'})).toBeDisabled();fresh.unmount();await act(async()=>calculation.resolve(saved));expect(document.documentElement.outerHTML).not.toContain('Synthetic private priority');
  });
});
