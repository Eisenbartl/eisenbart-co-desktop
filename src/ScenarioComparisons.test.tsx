import {act,fireEvent,render,screen,waitFor,within} from '@testing-library/react';
import {beforeEach,expect,it,vi} from 'vitest';
import ScenarioComparisons from './ScenarioComparisons';
import type {AssessmentPlans,ScenarioComparison,ScenarioInput,ScenarioExpense} from './types';
const mocks=vi.hoisted(()=>({getAssessmentPlanning:vi.fn(),compareFinancialScenarios:vi.fn(),listClientScenarios:vi.fn(),getClientScenario:vi.fn(),saveClientScenario:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const plans:AssessmentPlans={clientId:'client-a',budget:{clientId:'client-a',revision:2,updatedAt:'2026-10-14T14:00:00Z',lines:[{label:'Entered wages',kind:'income',amount:'1500.00',frequency:'biweekly',estimated:false}],summary:{monthlyIncome:'3250.00',necessaryExpenses:'0.00',discretionaryExpenses:'0.00',totalSpending:'0.00',surplusBeforeAllocations:'3250.00',proposedSavings:'0.00',availableForDebt:'3250.00',plannedDebtPayments:'0.00',unallocated:'3250.00',monthlyAmounts:['3250.00'],missingCount:0,estimatedCount:0,warnings:['Entered estimates only']}},debt:{clientId:'client-a',revision:0,updatedAt:null,data:null,comparison:null,budgetContext:{revision:2,monthlyCapacity:'3250.00',plannedDebtPayments:'0.00',warnings:[]}},savings:{clientId:'client-a',revision:0,updatedAt:null,data:null,summary:null,context:{budgetRevision:2,debtRevision:0,necessaryExpenses:null,monthlySurplus:null,budgetSavings:'0.00',budgetDebtPayments:'0.00',debtStartMonth:null,debtMonthlyPayment:null,debtChanges:[],debtExtras:[],warnings:[]}},cashflow:{clientId:'client-a',revision:0,updatedAt:null,data:null,summary:null,context:{budgetRevision:2,debtRevision:0,savingsRevision:0,monthlySurplus:null,budgetDebt:'0.00',budgetSavings:'0.00',debtMonthlyPayment:null,savingsMonthlyContribution:null}}};
plans.debt.data={startMonth:'2026-11',monthlyPayment:'100.00',accounts:[],extras:[],paymentChanges:[]};plans.debt.revision=1;
plans.savings.data={startMonth:'2026-11',targetMonths:3,liquidSavings:'2600.00',restrictedSavings:'5000.00',obligationsReserved:'600.00',emergencyContribution:'200.00',alternativeContributions:[],goals:[],prioritiesNote:''};plans.savings.revision=1;
plans.savings.data.goals=[{label:'Car repair',targetAmount:'1200.00',targetDate:'2027-02-28',reservedAmount:'1000.00',monthlyContribution:'50.00'}];
plans.cashflow.data={startDate:'2026-11-01',endDate:'2026-11-30',accounts:[{id:'checking',label:'Checking',openingBalance:'500.00'}],schedules:[{label:'Rent',kind:'bill',accountId:'checking',destinationId:null,amount:'1200.00',firstDate:'2026-11-03',frequency:'once',secondDay:null,untilDate:null,certainty:'confirmed'}],notes:''};plans.cashflow.revision=1;
const initial:ScenarioInput={expectedRevisions:{budget:2,debt:1,savings:1,cashflow:1},label:'Temporary combined proposal',assumptions:'',baselineStrategy:'avalanche',proposedStrategy:'avalanche',budgetChanges:[],debtPayment:null,emergencyContribution:null,cashChanges:[]};
const comparison:ScenarioComparison={clientId:'client-a',calculator:'combined-preview-v1',createdAt:'2026-10-14T16:00:00Z',revisions:initial.expectedRevisions,input:initial,baseline:{strategy:'avalanche',firstMonthBuffer:'3000.00',plans},proposed:{strategy:'avalanche',firstMonthBuffer:'2866.67',plans:{...plans,budget:{...plans.budget,summary:{...plans.budget.summary,monthlyIncome:'3466.67'}}}},warnings:['Temporary assumptions only; no saved plans changed']};
const props=()=>({clientId:'client-a',disabled:false,onBusy:vi.fn()});
function deferred<T>(){let resolve!:(v:T)=>void;const promise=new Promise<T>(r=>resolve=r);return{promise,resolve};}
async function load(){fireEvent.click(screen.getByText('Compare combined financial scenarios'));fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));await screen.findByLabelText('Proposal label');await waitFor(()=>expect(screen.getByRole('button',{name:'Compare temporary proposal'})).toBeEnabled());}
beforeEach(()=>{vi.resetAllMocks();mocks.listClientScenarios.mockResolvedValue({clientId:'client-a',records:[]});mocks.getAssessmentPlanning.mockResolvedValue(plans);mocks.compareFinancialScenarios.mockImplementation(async (_id:string,input:ScenarioInput)=>({...comparison,input}));});
it('loads explicitly and sends only changed strings and all baseline versions, displaying backend outcomes',async()=>{
 render(<ScenarioComparisons {...props()}/>);expect(mocks.getAssessmentPlanning).not.toHaveBeenCalled();await load();expect(mocks.compareFinancialScenarios).not.toHaveBeenCalled();fireEvent.click(screen.getByText('Change entered budget amounts'));fireEvent.change(screen.getByLabelText(/Entered wages/),{target:{value:'1600'}});fireEvent.change(screen.getByLabelText('Proposed debt base monthly payment (USD)'),{target:{value:'200'}});fireEvent.change(screen.getByLabelText('Proposed emergency monthly contribution (USD)'),{target:{value:'300'}});fireEvent.click(screen.getByText('Explicit dated cash-flow changes'));fireEvent.change(screen.getByLabelText('Proposed Rent first date (YYYY-MM-DD)'),{target:{value:'2026-11-08'}});fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));const region=await screen.findByRole('region',{name:'Combined scenario comparison'});expect(mocks.compareFinancialScenarios).toHaveBeenCalledWith('client-a',{...initial,budgetChanges:[{index:0,amount:'1600'}],debtPayment:{amount:'200'},emergencyContribution:{amount:'300'},cashChanges:[{index:0,amount:'1200.00',firstDate:'2026-11-08'}]});expect(region).toHaveTextContent('$3,466.67');expect(region).toHaveTextContent('$2,866.67');expect(region).toHaveTextContent('America/Chicago');expect(within(region).getByRole('table')).toHaveTextContent('Unavailable');expect(mocks.compareFinancialScenarios.mock.calls[0][1]).not.toHaveProperty('outcomes');
});
it('hides old outcomes after edits and preserves a rejected proposal for correction',async()=>{
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});fireEvent.change(screen.getByLabelText('Proposed debt base monthly payment (USD)'),{target:{value:'12.345'}});expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();mocks.compareFinancialScenarios.mockRejectedValueOnce(new Error('Invalid cents or stale baseline'));fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));expect(await screen.findByRole('alert')).toHaveTextContent('Invalid cents');expect(screen.getByLabelText('Proposed debt base monthly payment (USD)')).toHaveValue('12.345');fireEvent.click(screen.getByText('Explicit dated cash-flow changes'));fireEvent.change(screen.getByLabelText('Proposed Rent first date (YYYY-MM-DD)'),{target:{value:'2026-02-30'}});mocks.compareFinancialScenarios.mockRejectedValueOnce(new Error('Enter a real calendar date'));fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));expect(await screen.findByRole('alert')).toHaveTextContent('calendar date');expect(screen.getByLabelText('Proposed Rent first date (YYYY-MM-DD)')).toHaveValue('2026-02-30');expect(mocks.compareFinancialScenarios.mock.calls.at(-1)?.[1].cashChanges).toEqual([{index:0,amount:'1200.00',firstDate:'2026-02-30'}]);
});
it('passes blank changed amounts as unknown, blocks concurrent requests and invalidates baseline on parent edits',async()=>{
 const pending=deferred<ScenarioComparison>(),p=props();const view=render(<ScenarioComparisons {...p}/>);await load();fireEvent.change(screen.getByLabelText('Proposed debt base monthly payment (USD)'),{target:{value:''}});mocks.compareFinancialScenarios.mockReturnValueOnce(pending.promise);fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));expect(mocks.compareFinancialScenarios.mock.calls[0][1].debtPayment).toEqual({amount:null});expect(screen.getByRole('button',{name:'Reload saved scenario baseline'})).toBeDisabled();expect(screen.getByRole('button',{name:'Compare temporary proposal'})).toBeDisabled();view.rerender(<ScenarioComparisons {...p} disabled/>);await act(async()=>pending.resolve(comparison));expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();view.rerender(<ScenarioComparisons {...p}/>);expect(screen.queryByLabelText('Proposal label')).not.toBeInTheDocument();expect(screen.getByRole('button',{name:'Reload saved scenario baseline'})).toBeEnabled();
});
it('drops late client replies and rejects result versions, calculator and nested client identity',async()=>{
 const pending=deferred<ScenarioComparison>(),p=props();const view=render(<ScenarioComparisons {...p}/>);await load();mocks.compareFinancialScenarios.mockReturnValueOnce(pending.promise);fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));view.rerender(<ScenarioComparisons {...p} clientId="client-b"/>);await act(async()=>pending.resolve(comparison));expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();view.rerender(<ScenarioComparisons {...p}/>);fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));await screen.findByLabelText('Proposal label');for(const bad of [{...comparison,revisions:{...comparison.revisions,debt:99}},{...comparison,calculator:'other'},{...comparison,proposed:{...comparison.proposed,plans:{...plans,cashflow:{...plans.cashflow,clientId:'foreign'}}}}]){mocks.compareFinancialScenarios.mockResolvedValueOnce(bad);fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));expect(await screen.findByRole('alert')).toHaveTextContent('did not match');await waitFor(()=>expect(screen.getByRole('button',{name:'Compare temporary proposal'})).toBeEnabled());expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();}
});
it('rejects wrong-client and late baseline loads and permits a read-only reload after edits',async()=>{
 const p=props();mocks.getAssessmentPlanning.mockResolvedValueOnce({...plans,clientId:'foreign'});render(<ScenarioComparisons {...p}/>);fireEvent.click(screen.getByText('Compare combined financial scenarios'));fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));expect(await screen.findByRole('alert')).toHaveTextContent('did not match');fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));await screen.findByLabelText('Proposal label');fireEvent.change(screen.getByLabelText('Proposal label'),{target:{value:'Scratch only'}});fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));await waitFor(()=>expect(screen.getByLabelText('Proposal label')).toHaveValue('Temporary combined proposal'));expect(mocks.compareFinancialScenarios).not.toHaveBeenCalled();
});

const saved={clientId:'client-a',digest:'a'.repeat(64),needsReview:false,reviewReasons:[],payload:{id:'scenario-a',format:1,appVersion:'0.20.0',workspace:{organizationId:'org-a',organizationName:'Fictional',counselorId:'counselor-a',counselorName:'Morgan'},comparison:{...comparison,calculator:'combined-scenario-v1',warnings:['Saved hypothetical proposal only']}}};
it('records only reviewed input and expected history count, then opens immutable saved outcomes',async()=>{
 mocks.saveClientScenario.mockResolvedValue(saved);render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));const article=await screen.findByRole('article',{name:'Saved combined scenario'});expect(mocks.saveClientScenario).toHaveBeenCalledWith('client-a',{expectedCount:0,proposal:initial});expect(article).toHaveTextContent('$3,466.67');expect(article).toHaveTextContent('Saved hypothetical proposal');expect(screen.queryByRole('button',{name:'Save reviewed combined scenario'})).not.toBeInTheDocument();expect(screen.getByRole('status')).toHaveTextContent('Live plans and approved records are unchanged');
});
it('reopens history without loading a live baseline and displays stale records as historical',async()=>{
 mocks.listClientScenarios.mockResolvedValue({clientId:'client-a',records:[{id:'scenario-a',label:'Saved proposal',createdAt:saved.payload.comparison.createdAt,digest:saved.digest,needsReview:true}]});mocks.getClientScenario.mockResolvedValue({...saved,needsReview:true,reviewReasons:['Budget versions changed']});render(<ScenarioComparisons {...props()}/>);fireEvent.click(screen.getByText('Compare combined financial scenarios'));fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario history'}));await screen.findByLabelText('Open saved combined scenario');fireEvent.change(screen.getByLabelText('Open saved combined scenario'),{target:{value:'scenario-a'}});expect(await screen.findByRole('article',{name:'Saved combined scenario'})).toHaveTextContent('Budget versions changed');expect(screen.getByRole('article')).toHaveTextContent('historical baseline');expect(mocks.getAssessmentPlanning).not.toHaveBeenCalled();expect(mocks.saveClientScenario).not.toHaveBeenCalled();
});
it('preserves failed saves without retry and drops late saved-history replies across clients',async()=>{
 const p=props(),view=render(<ScenarioComparisons {...p}/>);await load();fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});mocks.saveClientScenario.mockRejectedValueOnce(new Error('History changed. Check for an existing save.'));fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));expect(await screen.findByRole('alert')).toHaveTextContent('Check for an existing save');expect(mocks.saveClientScenario).toHaveBeenCalledTimes(1);expect(screen.getByLabelText('Proposal label')).toHaveValue(initial.label);
 const pending=deferred<typeof saved>();mocks.saveClientScenario.mockReturnValueOnce(pending.promise);fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));view.rerender(<ScenarioComparisons {...p} clientId="client-b"/>);await act(async()=>pending.resolve(saved));expect(screen.queryByRole('article',{name:'Saved combined scenario'})).not.toBeInTheDocument();
});

it('sends extra payment and indexed goal assumptions only and records the reviewed proposal',async()=>{
 const canonical={...initial,debtExtras:[{month:'2026-11',amount:'300.00'}],goalContributions:[{index:0,amount:'100.00'}]};
 mocks.compareFinancialScenarios.mockResolvedValueOnce({...comparison,input:canonical});
 mocks.saveClientScenario.mockResolvedValueOnce({...saved,payload:{...saved.payload,comparison:{...saved.payload.comparison,input:canonical}}});
 render(<ScenarioComparisons {...props()}/>);await load();
 fireEvent.click(screen.getByText('Propose extra debt payments'));
 fireEvent.click(screen.getByRole('button',{name:'Add proposed extra payment'}));
 fireEvent.change(screen.getByLabelText('Extra payment 1 month (YYYY-MM)'),{target:{value:'2026-11'}});
 fireEvent.change(screen.getByLabelText('Extra payment 1 amount (USD)'),{target:{value:'300'}});
 fireEvent.click(screen.getByText('Change savings-goal contributions'));
 fireEvent.change(screen.getByLabelText('Proposed Car repair monthly contribution (USD)'),{target:{value:'100'}});
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));
 const region=await screen.findByRole('region',{name:'Combined scenario comparison'});
 const proposal={...initial,debtExtras:[{month:'2026-11',amount:'300'}],goalContributions:[{index:0,amount:'100'}]};
 expect(mocks.compareFinancialScenarios).toHaveBeenCalledWith('client-a',proposal);
 expect(region).toHaveTextContent('Additional debt payment: $300.00 in 2026-11');
 expect(region).toHaveTextContent('Car repair monthly contribution: $100.00');
 expect(within(region).getByRole('table')).toHaveTextContent('Car repair completion date');
 fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));
 expect(await screen.findByRole('article',{name:'Saved combined scenario'})).toHaveTextContent('Additional debt payment');
 expect(mocks.saveClientScenario).toHaveBeenCalledWith('client-a',{expectedCount:0,proposal});
 expect(proposal).not.toHaveProperty('plans');
});

it('retains rejected extras, clears a stale preview on removal, and distinguishes unknown from zero goal contributions',async()=>{
 render(<ScenarioComparisons {...props()}/>);await load();
 fireEvent.click(screen.getByText('Propose extra debt payments'));fireEvent.click(screen.getByRole('button',{name:'Add proposed extra payment'}));
 fireEvent.change(screen.getByLabelText('Extra payment 1 month (YYYY-MM)'),{target:{value:'2026-11'}});
 fireEvent.change(screen.getByLabelText('Extra payment 1 amount (USD)'),{target:{value:'12.345'}});
 fireEvent.click(screen.getByText('Change savings-goal contributions'));
 fireEvent.change(screen.getByLabelText('Proposed Car repair monthly contribution (USD)'),{target:{value:''}});
 mocks.compareFinancialScenarios.mockRejectedValueOnce(new Error('Use at most two decimal places'));
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('two decimal places');
 expect(mocks.compareFinancialScenarios).toHaveBeenCalledTimes(1);
 expect(screen.getByLabelText('Extra payment 1 amount (USD)')).toHaveValue('12.345');
 expect(mocks.compareFinancialScenarios.mock.calls[0][1].goalContributions).toEqual([{index:0,amount:null}]);
 fireEvent.change(screen.getByLabelText('Extra payment 1 amount (USD)'),{target:{value:'100'}});
 fireEvent.change(screen.getByLabelText('Proposed Car repair monthly contribution (USD)'),{target:{value:'0'}});
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});
 expect(mocks.compareFinancialScenarios.mock.calls[1][1].goalContributions).toEqual([{index:0,amount:'0'}]);
 fireEvent.click(screen.getByRole('button',{name:'Remove proposed extra payment 1'}));
 expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();
 expect(screen.queryByRole('button',{name:'Save reviewed combined scenario'})).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});
 expect(mocks.compareFinancialScenarios.mock.calls[2][1]).not.toHaveProperty('debtExtras');
 fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));
 await waitFor(()=>expect(screen.getByLabelText('Proposed Car repair monthly contribution (USD)')).toHaveValue('50.00'));
 expect(screen.queryByLabelText('Extra payment 1 amount (USD)')).not.toBeInTheDocument();
});

it('limits proposed extras using the saved event count and discards all new inputs across clients',async()=>{
 const loaded=structuredClone(plans);loaded.debt.data!.extras=Array.from({length:24},(_,i)=>({month:`2027-${String(i%12+1).padStart(2,'0')}`,amount:'1.00'}));
 mocks.getAssessmentPlanning.mockResolvedValueOnce(loaded);
 const p=props(),view=render(<ScenarioComparisons {...p}/>);await load();
 fireEvent.click(screen.getByText('Propose extra debt payments'));
 expect(screen.getByRole('button',{name:'Add proposed extra payment'})).toBeDisabled();
 view.rerender(<ScenarioComparisons {...p} clientId="client-b"/>);
 expect(screen.queryByLabelText('Proposed Car repair monthly contribution (USD)')).not.toBeInTheDocument();
 expect(screen.queryByRole('button',{name:'Add proposed extra payment'})).not.toBeInTheDocument();
 expect(mocks.compareFinancialScenarios).not.toHaveBeenCalled();
});

it('reviews and saves a new expense with only explicitly selected dated account and payment assumptions',async()=>{
 const expense:ScenarioExpense={label:'Child care',kind:'necessary',amount:'75.00',frequency:'monthly',dated:{accountId:'checking',firstDate:'2026-11-15',secondDay:null,untilDate:'2026-11-30'}};
 const input={...initial,recurringExpenses:[expense]};
 mocks.compareFinancialScenarios.mockResolvedValueOnce({...comparison,input});
 mocks.saveClientScenario.mockResolvedValueOnce({...saved,payload:{...saved.payload,comparison:{...saved.payload.comparison,input}}});
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByText('Add new recurring expenses'));fireEvent.click(screen.getByRole('button',{name:'Add proposed recurring expense'}));
 fireEvent.change(screen.getByLabelText('New expense 1 name'),{target:{value:'Child care'}});
 fireEvent.change(screen.getByLabelText('New expense 1 amount (USD)'),{target:{value:'75'}});
 expect(screen.queryByLabelText('New expense 1 payment account')).not.toBeInTheDocument();
 fireEvent.click(screen.getByLabelText('Also add new expense 1 to the dated forecast'));
 expect(screen.getByLabelText('New expense 1 payment account')).toHaveValue('');expect(screen.getByLabelText('New expense 1 first date (YYYY-MM-DD)')).toHaveValue('');
 fireEvent.change(screen.getByLabelText('New expense 1 payment account'),{target:{value:'checking'}});
 fireEvent.change(screen.getByLabelText('New expense 1 first date (YYYY-MM-DD)'),{target:{value:'2026-11-15'}});
 fireEvent.change(screen.getByLabelText('New expense 1 last date (optional)'),{target:{value:'2026-11-30'}});
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));const region=await screen.findByRole('region',{name:'Combined scenario comparison'});
 const proposal={...initial,recurringExpenses:[{...expense,amount:'75'}]};
 expect(mocks.compareFinancialScenarios).toHaveBeenCalledWith('client-a',proposal);
 expect(region).toHaveTextContent('New necessary expense Child care: $75.00 per monthly');expect(region).toHaveTextContent('Dated forecast: Checking, first 2026-11-15');
 fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));await screen.findByRole('article',{name:'Saved combined scenario'});
 expect(mocks.saveClientScenario).toHaveBeenCalledWith('client-a',{expectedCount:0,proposal});expect(proposal.recurringExpenses[0]).not.toHaveProperty('monthlyTotal');
});

it('retains rejected dates and unknown expense amounts, distinguishes zero, and invalidates review on removal',async()=>{
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByText('Add new recurring expenses'));fireEvent.click(screen.getByRole('button',{name:'Add proposed recurring expense'}));
 fireEvent.change(screen.getByLabelText('New expense 1 name'),{target:{value:'Uncertain bill'}});fireEvent.click(screen.getByLabelText('Also add new expense 1 to the dated forecast'));
 fireEvent.change(screen.getByLabelText('New expense 1 payment account'),{target:{value:'checking'}});fireEvent.change(screen.getByLabelText('New expense 1 first date (YYYY-MM-DD)'),{target:{value:'2026-02-30'}});
 mocks.compareFinancialScenarios.mockRejectedValueOnce(new Error('Enter a real calendar date'));
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));expect(await screen.findByRole('alert')).toHaveTextContent('calendar date');
 expect(mocks.compareFinancialScenarios).toHaveBeenCalledTimes(1);expect(mocks.compareFinancialScenarios.mock.calls[0][1].recurringExpenses[0].amount).toBeNull();expect(screen.getByLabelText('New expense 1 first date (YYYY-MM-DD)')).toHaveValue('2026-02-30');
 fireEvent.change(screen.getByLabelText('New expense 1 first date (YYYY-MM-DD)'),{target:{value:'2026-11-15'}});fireEvent.change(screen.getByLabelText('New expense 1 amount (USD)'),{target:{value:'0.00'}});
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});expect(mocks.compareFinancialScenarios.mock.calls[1][1].recurringExpenses[0].amount).toBe('0.00');
 fireEvent.click(screen.getByRole('button',{name:'Remove proposed expense 1'}));expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();expect(screen.queryByRole('button',{name:'Save reviewed combined scenario'})).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});expect(mocks.compareFinancialScenarios.mock.calls[2][1]).not.toHaveProperty('recurringExpenses');
});

it('permits budget-only annual expenses without a cash-flow model and clears new expense drafts across clients',async()=>{
 const p=props(),loaded={...plans,cashflow:{...plans.cashflow,revision:0,data:null,summary:null}};mocks.getAssessmentPlanning.mockResolvedValueOnce(loaded);
 const view=render(<ScenarioComparisons {...p}/>);await load();fireEvent.click(screen.getByText('Add new recurring expenses'));fireEvent.click(screen.getByRole('button',{name:'Add proposed recurring expense'}));
 expect(screen.getByLabelText('Also add new expense 1 to the dated forecast')).toBeDisabled();
 fireEvent.change(screen.getByLabelText('New expense 1 name'),{target:{value:'Annual insurance'}});fireEvent.change(screen.getByLabelText('New expense 1 amount (USD)'),{target:{value:'120.00'}});fireEvent.change(screen.getByLabelText('New expense 1 frequency'),{target:{value:'annual'}});
 mocks.compareFinancialScenarios.mockImplementationOnce(async (_id,input)=>({...comparison,input,revisions:{...comparison.revisions,cashflow:0},baseline:{...comparison.baseline,plans:loaded},proposed:{...comparison.proposed,plans:loaded}}));
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await waitFor(()=>expect(mocks.compareFinancialScenarios).toHaveBeenCalled());
 expect(mocks.compareFinancialScenarios.mock.calls[0][1].recurringExpenses).toEqual([{label:'Annual insurance',kind:'necessary',amount:'120.00',frequency:'annual',dated:null}]);
 view.rerender(<ScenarioComparisons {...p} clientId="client-b"/>);expect(screen.queryByLabelText('New expense 1 name')).not.toBeInTheDocument();expect(screen.queryByRole('region',{name:'Combined scenario comparison'})).not.toBeInTheDocument();
});

it('sends bounded temporary income assumptions and displays returned monthly deficit and recovery separately',async()=>{
 const reduction={budgetIndex:0,fromMonth:'2026-11',throughMonth:'2026-11',amount:'400.00',cashSchedule:null};
 const input={...initial,incomeReductions:[reduction]};
 const incomePeriods=[{month:'2026-11',reduced:true,normalIncome:'3250.00',income:'866.67',debtReserved:'100.00',savingsReserved:'250.00',normalBuffer:'1700.00',buffer:'-683.33'},{month:'2026-12',reduced:false,normalIncome:'3250.00',income:'3250.00',debtReserved:'100.00',savingsReserved:'250.00',normalBuffer:'1700.00',buffer:'1700.00'}];
 mocks.compareFinancialScenarios.mockResolvedValueOnce({...comparison,input,incomePeriods});
 mocks.saveClientScenario.mockResolvedValueOnce({...saved,payload:{...saved.payload,comparison:{...saved.payload.comparison,input,incomePeriods}}});
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByText('Model temporary income reductions'));fireEvent.click(screen.getByRole('button',{name:'Add temporary income period'}));
 fireEvent.change(screen.getByLabelText('Income period 1 budget line'),{target:{value:'0'}});fireEvent.change(screen.getByLabelText('Income period 1 reduced amount per payment (USD)'),{target:{value:'400'}});fireEvent.change(screen.getByLabelText('Income period 1 first month (YYYY-MM)'),{target:{value:'2026-11'}});fireEvent.change(screen.getByLabelText('Income period 1 last month (YYYY-MM)'),{target:{value:'2026-11'}});
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));const region=await screen.findByRole('region',{name:'Temporary income funding'});
 expect(region).toHaveTextContent('-$683.33');expect(region).toHaveTextContent('Normal / recovery');expect(region).toHaveTextContent('2026-12');
 const proposal={...initial,incomeReductions:[{...reduction,amount:'400'}]};expect(mocks.compareFinancialScenarios).toHaveBeenCalledWith('client-a',proposal);
 fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));await screen.findByRole('article',{name:'Saved combined scenario'});expect(mocks.saveClientScenario).toHaveBeenCalledWith('client-a',{expectedCount:0,proposal});
});

it('retains rejected income ranges and paycheck selections, sends unknown amounts explicitly and clears review on removal',async()=>{
 const loaded=structuredClone(plans);loaded.cashflow.data!.schedules.push({...loaded.cashflow.data!.schedules[0],label:'Dated wages',kind:'paycheck',frequency:'biweekly',amount:'1500.00'});mocks.getAssessmentPlanning.mockResolvedValueOnce(loaded);
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByText('Model temporary income reductions'));fireEvent.click(screen.getByRole('button',{name:'Add temporary income period'}));fireEvent.change(screen.getByLabelText('Income period 1 budget line'),{target:{value:'0'}});fireEvent.change(screen.getByLabelText('Income period 1 first month (YYYY-MM)'),{target:{value:'2026-13'}});fireEvent.change(screen.getByLabelText('Income period 1 last month (YYYY-MM)'),{target:{value:'2026-11'}});fireEvent.change(screen.getByLabelText('Income period 1 paycheck schedule (optional)'),{target:{value:'1'}});
 mocks.compareFinancialScenarios.mockRejectedValueOnce(new Error('Choose valid planning months'));
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));expect(await screen.findByRole('alert')).toHaveTextContent('valid planning months');expect(screen.getByLabelText('Income period 1 first month (YYYY-MM)')).toHaveValue('2026-13');expect(screen.getByLabelText('Income period 1 paycheck schedule (optional)')).toHaveValue('1');expect(mocks.compareFinancialScenarios.mock.calls[0][1].incomeReductions[0]).toEqual({budgetIndex:0,fromMonth:'2026-13',throughMonth:'2026-11',amount:null,cashSchedule:1});
 expect(mocks.compareFinancialScenarios).toHaveBeenCalledTimes(1);fireEvent.click(screen.getByRole('button',{name:'Remove income period 1'}));expect(screen.queryByLabelText('Income period 1 first month (YYYY-MM)')).not.toBeInTheDocument();expect(screen.queryByRole('button',{name:'Save reviewed combined scenario'})).not.toBeInTheDocument();
});

it('drops temporary income fields on client changes and explicit baseline reloads',async()=>{
 const p=props(),view=render(<ScenarioComparisons {...p}/>);await load();fireEvent.click(screen.getByText('Model temporary income reductions'));fireEvent.click(screen.getByRole('button',{name:'Add temporary income period'}));fireEvent.change(screen.getByLabelText('Income period 1 reduced amount per payment (USD)'),{target:{value:'0'}});fireEvent.click(screen.getByRole('button',{name:'Reload saved scenario baseline'}));await waitFor(()=>expect(screen.queryByLabelText('Income period 1 reduced amount per payment (USD)')).not.toBeInTheDocument());fireEvent.click(screen.getByRole('button',{name:'Add temporary income period'}));view.rerender(<ScenarioComparisons {...p} clientId="client-b"/>);expect(screen.queryByLabelText('Income period 1 budget line')).not.toBeInTheDocument();expect(mocks.saveClientScenario).not.toHaveBeenCalled();
});

it('collects new goals, explicit funding sources and targeted accounts, accepts v2 and saves only the reviewed proposal',async()=>{
 const loaded=structuredClone(plans);loaded.debt.data!.accounts=[{label:'Named fictional loan',debtType:'personalLoan',balance:'1000.00',apr:'0',minimumPayment:'10.00',monthlyFee:'0',interestModel:'fixedMonthly',verified:true,termsNote:''}];mocks.getAssessmentPlanning.mockResolvedValueOnce(loaded);
 mocks.compareFinancialScenarios.mockImplementationOnce(async(_id,input:ScenarioInput)=>{const proposed=structuredClone(loaded);proposed.savings.data!.goals.push({label:input.newGoals![0].label,targetAmount:'300.00',targetDate:'2027-01-31',reservedAmount:'0.00',monthlyContribution:'100.00'});return {...comparison,calculator:'combined-preview-v2',input,baseline:{...comparison.baseline,plans:loaded},proposed:{...comparison.proposed,plans:proposed}};});
 mocks.saveClientScenario.mockImplementationOnce(async(_id,{proposal:input})=>({...saved,payload:{...saved.payload,comparison:{...comparison,calculator:'combined-scenario-v2',input,baseline:{...comparison.baseline,plans:loaded},proposed:{...comparison.proposed,plans:loaded}}}}));
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByText('New goals, targeted payments and funding sources'));fireEvent.click(screen.getByRole('button',{name:'Add proposed savings goal'}));expect(screen.getByLabelText('New goal 1 target date')).toHaveValue('');fireEvent.change(screen.getByLabelText('New goal 1 name'),{target:{value:'New repair reserve'}});fireEvent.change(screen.getByLabelText('New goal 1 target (USD)'),{target:{value:'300'}});fireEvent.change(screen.getByLabelText('New goal 1 target date'),{target:{value:'2027-01-31'}});fireEvent.change(screen.getByLabelText('New goal 1 monthly contribution (USD)'),{target:{value:'100'}});
 fireEvent.click(screen.getByRole('button',{name:'Add account-targeted extra'}));expect(screen.getByLabelText('Targeted extra 1 debt account')).toHaveValue('');expect(screen.getByLabelText('Targeted extra 1 month')).toHaveValue('');fireEvent.change(screen.getByLabelText('Targeted extra 1 debt account'),{target:{value:'0'}});fireEvent.change(screen.getByLabelText('Targeted extra 1 month'),{target:{value:'2026-11'}});fireEvent.change(screen.getByLabelText('Targeted extra 1 amount (USD)'),{target:{value:'50.01'}});
 fireEvent.click(screen.getByText('Funding sources and dated drawdowns'));fireEvent.click(screen.getByRole('button',{name:'Add separate funding source'}));fireEvent.change(screen.getByLabelText('Funding source 1 name'),{target:{value:'Separate gift'}});fireEvent.change(screen.getByLabelText('Funding source 1 available (USD)'),{target:{value:'100'}});
 fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));const region=await screen.findByRole('region',{name:'Combined scenario comparison'});expect(within(region).getByRole('table')).toHaveTextContent('New repair reserve monthly contributionNot in saved baseline$100.00');expect(region).toHaveTextContent('Named fictional loan');expect(region).toHaveTextContent('Separate gift');expect(mocks.saveClientScenario).not.toHaveBeenCalled();
 const sent=mocks.compareFinancialScenarios.mock.calls[0][1];expect(sent).toMatchObject({newGoals:[{id:expect.any(String),label:'New repair reserve',targetAmount:'300',targetDate:'2027-01-31',monthlyContribution:'100'}],targetedExtras:[{accountIndex:0,month:'2026-11',amount:'50.01'}],savingsFunding:{sources:[{id:expect.any(String),label:'Separate gift',available:'100'}],events:[]}});expect(sent).not.toHaveProperty('plans');fireEvent.click(screen.getByRole('button',{name:'Save reviewed combined scenario'}));await screen.findByRole('article',{name:'Saved combined scenario'});expect(mocks.saveClientScenario).toHaveBeenCalledWith('client-a',{expectedCount:0,proposal:sent});
});

it('collects an explicit transfer availability change while keeping other dated assumptions unchanged',async()=>{
 const loaded=structuredClone(plans);loaded.cashflow.data!.accounts.push({id:'reserve',label:'Reserve',openingBalance:'1000'});loaded.cashflow.data!.schedules[0]={...loaded.cashflow.data!.schedules[0],label:'Transfer',kind:'transfer',destinationId:'reserve'};mocks.getAssessmentPlanning.mockResolvedValueOnce(loaded);
 render(<ScenarioComparisons {...props()}/>);await load();fireEvent.click(screen.getByText('Explicit dated cash-flow changes'));fireEvent.change(screen.getByLabelText('Proposed Transfer settlement'),{target:{value:'delayed'}});expect(screen.getByLabelText('Proposed Transfer first available date')).toHaveValue('');fireEvent.change(screen.getByLabelText('Proposed Transfer first available date'),{target:{value:'2026-11-10'}});fireEvent.click(screen.getByRole('button',{name:'Compare temporary proposal'}));await screen.findByRole('region',{name:'Combined scenario comparison'});expect(mocks.compareFinancialScenarios.mock.calls[0][1].cashChanges).toEqual([{index:0,amount:'1200.00',firstDate:'2026-11-03',settlementChange:{delayed:true,firstAvailableDate:'2026-11-10'}}]);expect(mocks.saveClientScenario).not.toHaveBeenCalled();
});
