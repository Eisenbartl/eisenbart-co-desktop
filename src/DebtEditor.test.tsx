import { act, fireEvent, render, screen, within, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import DebtEditor from './DebtEditor';
import type { Client, ClientDebtPlan, DebtPlanData, DebtProjection } from './types';

const mocks=vi.hoisted(()=>({getClientDebtPlan:vi.fn(),saveClientDebtPlan:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const client:Client={id:'client-a',organizationId:'org-a',firstName:'Alex',lastName:'Fictional',email:'',phone:'',householdSize:1,status:'Planning',goals:'',createdAt:'',updatedAt:''};
const empty:ClientDebtPlan={clientId:client.id,revision:0,updatedAt:null,data:null,comparison:null,budgetContext:{revision:1,monthlyCapacity:'100.00',plannedDebtPayments:'100.00',warnings:[]}};
const data:DebtPlanData={startMonth:'2026-11',monthlyPayment:'100.00',accounts:[{label:'Synthetic loan',debtType:'personalLoan',balance:'1000.00',apr:'12.0000',minimumPayment:'100.00',monthlyFee:'0.00',interestModel:'fixedMonthly',verified:true,termsNote:'Synthetic lender fixture'}],extras:[],paymentChanges:[]};
const projection:DebtProjection={status:'paidOff',payoffMonth:'2027-09',accountPayoffMonths:['2027-09'],totalInterest:'58.98',totalFees:'0.00',totalPaid:'1058.98',remainingBalance:'0.00',negativeAmortizationAccounts:[],warnings:[],schedule:[{month:'2026-11',availablePayment:'100.00',payments:['100.00'],interest:['10.00'],fees:['0.00'],balances:['910.00'],totalPayment:'100.00',remainingBalance:'910.00'},{month:'2027-09',availablePayment:'100.00',payments:['58.98'],interest:['0.58'],fees:['0.00'],balances:['0.00'],totalPayment:'58.98',remainingBalance:'0.00'}]};
const saved:ClientDebtPlan={...empty,revision:1,updatedAt:'2026-10-13T12:00:00Z',data,comparison:{warnings:['Planning estimate with fixed monthly assumptions.'],snowball:projection,avalanche:projection}};
function deferred<T>(){let resolve!:(v:T)=>void;const promise=new Promise<T>(accept=>{resolve=accept;});return{promise,resolve};}
function open(){return render(<DebtEditor client={client} onDirty={vi.fn()} onBusy={vi.fn()} otherSaving={false}/>);}
beforeEach(()=>{vi.clearAllMocks();mocks.getClientDebtPlan.mockResolvedValue(empty);mocks.saveClientDebtPlan.mockResolvedValue(saved);});

describe('client debt editor',()=>{
  it('starts with unknown terms and sends exact strings with the saved revision',async()=>{
    open();await screen.findByText('No saved debt plan yet');
    fireEvent.change(screen.getByLabelText('First planning month'),{target:{value:'2026-11'}});
    fireEvent.change(screen.getByLabelText('Total monthly debt payment (USD)'),{target:{value:'100'}});
    fireEvent.click(screen.getByRole('button',{name:'Add debt'}));
    expect(screen.getByLabelText('APR (%)')).toHaveValue('');
    expect(screen.getByLabelText('Known monthly fee (USD)')).toHaveValue('');
    expect(screen.getByLabelText('Interest model')).toHaveValue('unknown');
    fireEvent.change(screen.getByLabelText('Debt name'),{target:{value:'Synthetic loan'}});
    fireEvent.change(screen.getByLabelText('Outstanding balance (USD)'),{target:{value:'1000'}});
    fireEvent.change(screen.getByLabelText('APR (%)'),{target:{value:'12'}});
    fireEvent.change(screen.getByLabelText('Monthly minimum (USD)'),{target:{value:'100'}});
    fireEvent.change(screen.getByLabelText('Known monthly fee (USD)'),{target:{value:'0'}});
    fireEvent.change(screen.getByLabelText('Interest model'),{target:{value:'fixedMonthly'}});
    fireEvent.click(screen.getByRole('button',{name:'Save debt plan and compare'}));
    await screen.findByText('Debt plan saved and compared.');
    const sent=mocks.saveClientDebtPlan.mock.calls[0];
    expect(sent[0]).toBe(client.id);expect(sent[1].expectedRevision).toBe(0);
    expect(sent[1].data.accounts[0]).toMatchObject({balance:'1000',apr:'12',minimumPayment:'100',monthlyFee:'0',verified:false});
    expect(sent[1]).not.toHaveProperty('organizationId');
    expect(screen.getByLabelText('Outstanding balance (USD)')).toHaveValue('1000.00');
    expect(screen.getByRole('table', {name:'Debt strategy comparison'})).toHaveTextContent('$58.98');
  });
  it('shows native strategy totals and lets the counselor inspect a selected monthly schedule',async()=>{
    mocks.getClientDebtPlan.mockResolvedValue(saved);open();await screen.findByDisplayValue('Synthetic loan');
    const tables=screen.getAllByRole('table');expect(within(tables[0]).getAllByText('$58.98')).toHaveLength(2);
    expect(within(tables[1]).getByText('$910.00')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Schedule month'),{target:{value:'1'}});
    expect(within(screen.getAllByRole('table')[1]).getByText('$0.58')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Schedule strategy'),{target:{value:'avalanche'}});
    expect(screen.getByLabelText('Schedule month')).toHaveValue('0');
    fireEvent.change(screen.getByLabelText('APR (%)'),{target:{value:'12.12345'}});
    expect(screen.queryByRole('heading',{name:'Strategy comparison'})).not.toBeInTheDocument();
    mocks.saveClientDebtPlan.mockRejectedValueOnce('APR permits at most four decimal places.');
    fireEvent.click(screen.getByRole('button',{name:'Save debt plan and compare'}));await screen.findByRole('alert');
    expect(screen.getByLabelText('APR (%)')).toHaveValue('12.12345');
    expect(mocks.saveClientDebtPlan.mock.calls[0][1].expectedRevision).toBe(1);
  });
  it('saves incomplete and unmodeled accounts without fabricating a payoff date',async()=>{
    mocks.getClientDebtPlan.mockResolvedValue({...saved,data:{...data,accounts:[{...data.accounts[0],apr:null,interestModel:'variable'}]},comparison:{warnings:['Missing or unmodeled terms.'],snowball:null,avalanche:null}});
    open();await screen.findByText('Missing or unmodeled terms.');
    expect(screen.getByText(/Comparison unavailable/)).toBeInTheDocument();
    expect(screen.queryByText('2027-09')).not.toBeInTheDocument();expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.getByLabelText('APR (%)')).toHaveValue('');
  });
  it('sends one-time extra payments and monthly replacements without calculating money in React',async()=>{
    mocks.getClientDebtPlan.mockResolvedValue(saved);open();await screen.findByDisplayValue('Synthetic loan');
    fireEvent.click(screen.getByText('Extra payments and future payment changes'));
    fireEvent.click(screen.getByRole('button',{name:'Add extra payment'}));
    fireEvent.change(screen.getByLabelText('Extra amount (USD)'),{target:{value:'250.01'}});
    fireEvent.click(screen.getByRole('button',{name:'Add payment change'}));
    fireEvent.change(screen.getByLabelText('New total payment (USD)'),{target:{value:'150'}});
    fireEvent.change(screen.getAllByLabelText('Month (YYYY-MM)')[1],{target:{value:'2027-01'}});
    fireEvent.click(screen.getByRole('button',{name:'Save debt plan and compare'}));
    await waitFor(()=>expect(mocks.saveClientDebtPlan).toHaveBeenCalled());
    expect(mocks.saveClientDebtPlan.mock.calls[0][1].data).toMatchObject({extras:[{month:'2026-11',amount:'250.01'}],paymentChanges:[{month:'2027-01',amount:'150'}]});
  });
  it('preserves failed-load data and offers a retry instead of an empty replacement',async()=>{
    mocks.getClientDebtPlan.mockRejectedValueOnce('Storage unavailable.');open();await screen.findByRole('alert');
    expect(screen.queryByRole('button',{name:'Save debt plan and compare'})).not.toBeInTheDocument();
    mocks.getClientDebtPlan.mockResolvedValueOnce(saved);fireEvent.click(screen.getByRole('button',{name:'Retry loading debt plan'}));
    await screen.findByDisplayValue('Synthetic loan');expect(mocks.saveClientDebtPlan).not.toHaveBeenCalled();
  });
  it('ignores delayed loads from an old client and saves after the private view unmounts',async()=>{
    const pending=deferred<ClientDebtPlan>();mocks.getClientDebtPlan.mockReturnValueOnce(pending.promise);
    const first=open();first.unmount();
    const next={...client,id:'client-b',firstName:'Robin'};mocks.getClientDebtPlan.mockResolvedValueOnce({...empty,clientId:next.id});
    const view=render(<DebtEditor client={next} onDirty={vi.fn()} onBusy={vi.fn()} otherSaving={false}/>);
    await screen.findByText('No saved debt plan yet');await act(async()=>pending.resolve(saved));
    expect(screen.queryByDisplayValue('Synthetic loan')).not.toBeInTheDocument();
    const pendingSave=deferred<ClientDebtPlan>();mocks.saveClientDebtPlan.mockReturnValueOnce(pendingSave.promise);
    fireEvent.click(screen.getByRole('button',{name:'Add debt'}));fireEvent.change(screen.getByLabelText('Debt name'),{target:{value:'Robin private debt'}});
    fireEvent.click(screen.getByRole('button',{name:'Save debt plan and compare'}));
    expect(screen.getByRole('button',{name:'Comparing…'})).toBeDisabled();
    view.unmount();await act(async()=>pendingSave.resolve({...saved,clientId:next.id}));
    expect(document.documentElement.outerHTML).not.toContain('Robin private debt');
  });
});
it('collects known advanced terms and dates without changing monthly payment meaning',async()=>{
 mocks.getClientDebtPlan.mockResolvedValue(saved);open();await screen.findByText('Saved debt plan');fireEvent.click(screen.getByText('Advanced interest terms and payment frequency'));
 fireEvent.change(screen.getByLabelText('Interest model'),{target:{value:'promotional'}});fireEvent.change(screen.getByLabelText('Debt 1 day-count basis'),{target:{value:'365'}});fireEvent.click(screen.getByRole('button',{name:'Add debt 1 known APR change'}));
 fireEvent.change(screen.getByLabelText('Debt 1 rate 1 month'),{target:{value:'2027-01'}});fireEvent.change(screen.getByLabelText('Debt 1 rate 1 APR (%)'),{target:{value:'24.1250'}});
 fireEvent.change(screen.getByLabelText('Repayment schedule'),{target:{value:'biweekly'}});expect(screen.getByLabelText('First debt payment date')).toHaveValue('');fireEvent.change(screen.getByLabelText('First debt payment date'),{target:{value:'2026-11-06'}});
 expect(mocks.saveClientDebtPlan).not.toHaveBeenCalled();fireEvent.click(screen.getByRole('button',{name:'Save debt plan and compare'}));await waitFor(()=>expect(mocks.saveClientDebtPlan).toHaveBeenCalled());expect(mocks.saveClientDebtPlan.mock.calls[0][1].data).toMatchObject({monthlyPayment:'100.00',advanced:{accounts:[{accountIndex:0,dayBasis:365,rateChanges:[{month:'2027-01',apr:'24.1250'}],fees:[]}],payments:{firstDate:'2026-11-06',frequency:'biweekly',secondDay:null}}});
});
it('saves an explicitly targeted extra and protects its account from accidental reassignment',async()=>{
 mocks.getClientDebtPlan.mockResolvedValue(saved);open();await screen.findByText('Saved debt plan');fireEvent.click(screen.getByText('Target an extra payment to a specific account'));fireEvent.click(screen.getByRole('button',{name:'Add account-targeted extra'}));fireEvent.change(screen.getByLabelText('Targeted extra 1 debt account'),{target:{value:'0'}});fireEvent.change(screen.getByLabelText('Targeted extra 1 month'),{target:{value:'2026-11'}});fireEvent.change(screen.getByLabelText('Targeted extra 1 amount (USD)'),{target:{value:'25.01'}});expect(screen.getByRole('button',{name:'Remove debt 1'})).toBeDisabled();fireEvent.click(screen.getByRole('button',{name:'Save debt plan and compare'}));await waitFor(()=>expect(mocks.saveClientDebtPlan).toHaveBeenCalled());expect(mocks.saveClientDebtPlan.mock.calls[0][1].data.targetedExtras).toEqual([{accountIndex:0,month:'2026-11',amount:'25.01'}]);
});
