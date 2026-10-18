import {act,fireEvent,render,screen,waitFor,within} from '@testing-library/react';
import {beforeEach,describe,expect,it,vi} from 'vitest';
import SavingsEditor from './SavingsEditor';
import type {Client,ClientSavingsPlan,SavingsPlanData,SavingsSummary} from './types';
const mocks=vi.hoisted(()=>({getClientSavingsPlan:vi.fn(),saveClientSavingsPlan:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const client:Client={id:'client-a',organizationId:'org-a',firstName:'Alex',lastName:'Fictional',email:'',phone:'',householdSize:1,status:'Planning',goals:'',createdAt:'',updatedAt:''};
const empty:ClientSavingsPlan={clientId:client.id,revision:0,updatedAt:null,data:null,summary:null,context:{budgetRevision:1,debtRevision:1,necessaryExpenses:'1633.33',monthlySurplus:'1416.67',budgetSavings:'300.00',budgetDebtPayments:'400.00',debtStartMonth:'2026-11',debtMonthlyPayment:'100.00',debtChanges:[],debtExtras:[],warnings:[]}};
const data:SavingsPlanData={startMonth:'2026-11',targetMonths:3,liquidSavings:'2600.00',restrictedSavings:'5000.00',obligationsReserved:'600.00',emergencyContribution:'200.00',alternativeContributions:[],goals:[{label:'Synthetic insurance',targetAmount:'1200.00',targetDate:'2027-02-28',reservedAmount:'1000.00',monthlyContribution:'50.00'}],prioritiesNote:'Protect essential reserves.'};
const summary:SavingsSummary={accessibleEmergency:'1000.00',obligationsReserved:'600.00',sinkingFundsReserved:'1000.00',restrictedSavings:'5000.00',coverageMonths:'0.61',emergencyTarget:'4899.99',emergency:{status:'projected',remaining:'3899.99',contribution:'200.00',monthsNeeded:20,completionDate:'2028-06-30'},alternatives:[],goals:[{suggestedContribution:'50.00',contributionMonthsBeforeDue:4,onTrack:true,forecast:{status:'projected',remaining:'200.00',contribution:'50.00',monthsNeeded:4,completionDate:'2027-02-28'},warnings:[]}],requestedMonthlySavings:'250.00',warnings:['Savings allocations differ from the saved budget.'],schedule:[{month:'2026-11',emergencyContribution:'200.00',goalContributions:['50.00'],totalSavings:'250.00',debtReserved:'400.00',unallocated:'766.67'},{month:'2028-06',emergencyContribution:'99.99',goalContributions:['0.00'],totalSavings:'99.99',debtReserved:'400.00',unallocated:'916.68'}]};
const saved:ClientSavingsPlan={...empty,revision:1,updatedAt:'2026-10-13T20:00:00Z',data,summary};
function deferred<T>(){let resolve!:(value:T)=>void;const promise=new Promise<T>(accept=>{resolve=accept;});return{promise,resolve};}
function open(){return render(<SavingsEditor client={client} onDirty={vi.fn()} onBusy={vi.fn()} otherSaving={false}/>);}
beforeEach(()=>{vi.clearAllMocks();mocks.getClientSavingsPlan.mockResolvedValue(empty);mocks.saveClientSavingsPlan.mockResolvedValue(saved);});

describe('client savings editor',()=>{
  it('keeps unknown inputs, sends exact strings and revisions, and uses the native forecast',async()=>{
    open();await screen.findByText('No saved savings plan yet');
    expect(screen.getByLabelText('Restricted savings (USD)')).toHaveValue('');
    fireEvent.change(screen.getByLabelText('First contribution month'),{target:{value:'2026-11'}});
    fireEvent.click(screen.getByRole('button',{name:'6 months'}));expect(screen.getByLabelText('Emergency target (months)')).toHaveValue(6);
    fireEvent.click(screen.getByRole('button',{name:'3 months'}));
    fireEvent.change(screen.getByLabelText('Total accessible savings before earmarks (USD)'),{target:{value:'2600'}});
    fireEvent.change(screen.getByLabelText('Reserved for upcoming bills (USD)'),{target:{value:'600'}});
    fireEvent.change(screen.getByLabelText('Monthly emergency contribution (USD)'),{target:{value:'200'}});
    fireEvent.click(screen.getByRole('button',{name:'Save savings plan and calculate'}));await screen.findByText('Savings plan saved and calculated.');
    expect(mocks.saveClientSavingsPlan).toHaveBeenCalledWith(client.id,{expectedRevision:0,data:expect.objectContaining({startMonth:'2026-11',liquidSavings:'2600',restrictedSavings:null,obligationsReserved:'600',emergencyContribution:'200',targetMonths:3})});
    expect(screen.getByLabelText('Total accessible savings before earmarks (USD)')).toHaveValue('2600.00');
    expect(screen.getByText('0.61 months')).toBeInTheDocument();expect(screen.getByText('2028-06-30')).toBeInTheDocument();
    expect(screen.getByRole('table',{name:'Sinking fund forecasts'})).toHaveTextContent('$50.00');
    fireEvent.change(screen.getByLabelText('Allocation month'),{target:{value:'1'}});
    expect(screen.getAllByText('$99.99')).toHaveLength(2);
    fireEvent.change(screen.getByLabelText('Monthly emergency contribution (USD)'),{target:{value:'200.001'}});
    expect(screen.queryByRole('heading',{name:'Emergency savings summary'})).not.toBeInTheDocument();
    mocks.saveClientSavingsPlan.mockRejectedValueOnce('At most two decimal places are allowed.');
    fireEvent.click(screen.getByRole('button',{name:'Save savings plan and calculate'}));await screen.findByRole('alert');expect(screen.getByLabelText('Monthly emergency contribution (USD)')).toHaveValue('200.001');
    expect(mocks.saveClientSavingsPlan.mock.calls[1][1].expectedRevision).toBe(1);
  });
  it('sends reserved goal money and an alternative contribution without changing the selected plan',async()=>{
    mocks.getClientSavingsPlan.mockResolvedValue(saved);open();await screen.findByDisplayValue('Synthetic insurance');
    fireEvent.click(screen.getByRole('button',{name:'Add sinking fund'}));
    fireEvent.change(screen.getAllByLabelText('Goal name')[1],{target:{value:'Future repair'}});
    fireEvent.change(screen.getAllByLabelText('Already reserved in accessible savings (USD)')[1],{target:{value:'0'}});
    fireEvent.click(screen.getByText('Compare emergency contributions'));
    fireEvent.click(screen.getByRole('button',{name:'Add alternative contribution'}));
    fireEvent.change(screen.getByLabelText('Alternative monthly contribution (USD)'),{target:{value:'300.01'}});
    fireEvent.click(screen.getByRole('button',{name:'Save savings plan and calculate'}));await waitFor(()=>expect(mocks.saveClientSavingsPlan).toHaveBeenCalled());
    expect(mocks.saveClientSavingsPlan.mock.calls[0][1].data).toMatchObject({emergencyContribution:'200.00',alternativeContributions:['300.01'],goals:[data.goals[0],{label:'Future repair',targetAmount:null,targetDate:null,reservedAmount:'0',monthlyContribution:null}]});
  });
  it('refreshes budget references without losing edits or adopting a concurrently changed savings plan',async()=>{
    mocks.getClientSavingsPlan.mockResolvedValue(saved);open();await screen.findByDisplayValue('Synthetic insurance');
    fireEvent.change(screen.getByLabelText('Monthly emergency contribution (USD)'),{target:{value:'123.45'}});
    mocks.getClientSavingsPlan.mockResolvedValueOnce({...saved,context:{...saved.context,budgetRevision:2,necessaryExpenses:'2000.00'}});
    fireEvent.click(screen.getByRole('button',{name:'Refresh budget and debt references'}));
    await screen.findByText(/budget revision 2/);expect(screen.getByLabelText('Monthly emergency contribution (USD)')).toHaveValue('123.45');
    mocks.getClientSavingsPlan.mockResolvedValueOnce({...saved,revision:2,data:{...data,emergencyContribution:'999.00'}});
    fireEvent.click(screen.getByRole('button',{name:'Refresh budget and debt references'}));await screen.findByRole('alert');
    expect(screen.getByRole('alert')).toHaveTextContent('changed elsewhere');expect(screen.getByLabelText('Monthly emergency contribution (USD)')).toHaveValue('123.45');
  });
  it('shows unknown affordability and missing inputs without inventing a completion date',async()=>{
    mocks.getClientSavingsPlan.mockResolvedValue({...saved,summary:{...summary,accessibleEmergency:null,coverageMonths:null,emergency:{...summary.emergency,status:'unknown',remaining:null,monthsNeeded:null,completionDate:null},schedule:[],alternatives:[{forecast:{...summary.emergency,status:'unknown',completionDate:null},allocationWarning:null}]}});
    open();await screen.findAllByText('Missing inputs — not projected');
    expect(screen.queryByText('2028-06-30')).not.toBeInTheDocument();
    expect(screen.getByRole('table',{name:'Emergency contribution scenarios'})).toHaveTextContent('Unavailable');
    expect(screen.queryByText('Within entered funds')).not.toBeInTheDocument();
  });
  it('blocks an empty editable replacement after loading fails and can retry',async()=>{
    mocks.getClientSavingsPlan.mockRejectedValueOnce('Storage unavailable.');open();await screen.findByRole('alert');
    expect(screen.queryByRole('button',{name:'Save savings plan and calculate'})).not.toBeInTheDocument();
    mocks.getClientSavingsPlan.mockResolvedValueOnce(saved);fireEvent.click(screen.getByRole('button',{name:'Retry loading savings plan'}));await screen.findByDisplayValue('Synthetic insurance');
    expect(mocks.saveClientSavingsPlan).not.toHaveBeenCalled();
  });
  it('ignores old client loads and pending saves after private state unmounts',async()=>{
    const pending=deferred<ClientSavingsPlan>();mocks.getClientSavingsPlan.mockReturnValueOnce(pending.promise);
    const first=open();first.unmount();const next={...client,id:'client-b',firstName:'Robin'};
    mocks.getClientSavingsPlan.mockResolvedValueOnce({...empty,clientId:next.id});
    const view=render(<SavingsEditor client={next} onDirty={vi.fn()} onBusy={vi.fn()} otherSaving={false}/>);await screen.findByText('No saved savings plan yet');
    await act(async()=>pending.resolve(saved));expect(screen.queryByDisplayValue('Synthetic insurance')).not.toBeInTheDocument();
    const write=deferred<ClientSavingsPlan>();mocks.saveClientSavingsPlan.mockReturnValueOnce(write.promise);
    fireEvent.change(screen.getByLabelText('Funding priorities and notes'),{target:{value:'Robin private savings priority'}});
    fireEvent.click(screen.getByRole('button',{name:'Save savings plan and calculate'}));expect(screen.getByRole('button',{name:'Calculating savings…'})).toBeDisabled();
    view.unmount();await act(async()=>write.resolve({...saved,clientId:next.id}));expect(document.documentElement.outerHTML).not.toContain('Robin private savings priority');
  });
});
it('collects separate funding and drawdowns without manufacturing dates or savings',async()=>{
 mocks.getClientSavingsPlan.mockResolvedValue(saved);open();await screen.findByText('Saved savings plan');fireEvent.click(screen.getByText('Funding sources and dated drawdowns'));
 fireEvent.click(screen.getByRole('button',{name:'Add separate funding source'}));fireEvent.change(screen.getByLabelText('Funding source 1 name'),{target:{value:'Separate fictional gift'}});fireEvent.change(screen.getByLabelText('Funding source 1 available (USD)'),{target:{value:'120.25'}});
 fireEvent.click(screen.getByRole('button',{name:'Add dated savings movement'}));expect(screen.getByLabelText('Savings event 1 date')).toHaveValue('');expect(screen.getByLabelText('Savings event 1 amount (USD)')).toHaveValue('');
 fireEvent.change(screen.getByLabelText('Savings event 1 date'),{target:{value:'2026-11-10'}});fireEvent.change(screen.getByLabelText('Savings event 1 amount (USD)'),{target:{value:'100.01'}});fireEvent.change(screen.getByLabelText('Savings event 1 purpose / source explanation'),{target:{value:'Discussed repair reserve'}});
 expect(mocks.saveClientSavingsPlan).not.toHaveBeenCalled();fireEvent.click(screen.getByRole('button',{name:'Save savings plan and calculate'}));await waitFor(()=>expect(mocks.saveClientSavingsPlan).toHaveBeenCalled());expect(mocks.saveClientSavingsPlan.mock.calls[0][1].data.funding).toMatchObject({sources:[{id:expect.any(String),label:'Separate fictional gift',available:'120.25'}],events:[{date:'2026-11-10',kind:'drawdown',bucket:null,destination:null,sourceId:null,amount:'100.01',note:'Discussed repair reserve'}]});
});
