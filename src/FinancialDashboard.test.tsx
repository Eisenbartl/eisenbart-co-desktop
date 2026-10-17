import {healthFixture} from './test-fixtures/health';
import {act,fireEvent,render,screen,within} from '@testing-library/react';
import {beforeEach,expect,it,vi} from 'vitest';
import FinancialDashboard from './FinancialDashboard';
import type {ClientImports,FinancialAssessment} from './types';
const mocks=vi.hoisted(()=>({getClientDashboard:vi.fn(),getFinancialHealth:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const totals={pending:1,needsClarification:0,approved:3,rejected:1,income:'1000.01',spending:'300.02',refunds:'20.03',netSpending:'279.99',netCash:'520.02',accountNet:[] as [string,string][],warnings:[]};
const source:ClientImports={clientId:'client-a',revision:7,accounts:[],documents:[],records:[],history:[],merchantGroups:[],recurringHistory:[],summary:totals};
const result:FinancialAssessment={clientId:'client-a',revision:7,periodStart:'2026-09-01',periodEnd:'2026-09-30',calculatedOnUtc:'2026-10-14',latestApprovedDate:'2026-09-30',outdated:false,totals,operatingCashflow:'720.02',months:[{month:'2026-09',totals,operatingCashflow:'720.02',incomeScale:1000,spendingScale:300,cashflowScale:1000}],categories:[{label:'<script>Synthetic Housing</script>',spending:'300.02',scale:1000}],undatedRecords:1,reviewedPercent:80,documentIds:['doc-a'],warnings:['Incomplete assessment: statements may be missing.']};
function deferred<T>(){let resolve!:(v:T)=>void;const promise=new Promise<T>(r=>{resolve=r;});return{promise,resolve};}
async function bankDetails(){fireEvent.click(await screen.findByText('Existing approved bank dashboard details'));}
function apply(){fireEvent.change(screen.getByLabelText('Assessment from month'),{target:{value:'2026-09'}});fireEvent.change(screen.getByLabelText('Assessment through month'),{target:{value:'2026-09'}});fireEvent.click(screen.getByRole('button',{name:'View financial dashboard'}));}
beforeEach(()=>{vi.clearAllMocks();mocks.getClientDashboard.mockResolvedValue(result);mocks.getFinancialHealth.mockResolvedValue(healthFixture);});
it('requires explicit months, renders exact authoritative figures and local accessible charts with basis',async()=>{
 render(<FinancialDashboard source={source} disabled={false}/>);expect(mocks.getClientDashboard).not.toHaveBeenCalled();apply();
 await bankDetails();await screen.findByText('95 / 100');
 const categories=await screen.findByRole('table',{name:'Approved spending categories'});
 expect(categories).toHaveTextContent('<script>Synthetic Housing</script>');expect(categories.querySelector('script')).toBeNull();
 expect(screen.getByRole('table',{name:'Monthly approved income and spending'})).toHaveTextContent('$1,000.01');
 expect(screen.getByRole('table',{name:'Monthly operating cash flow'})).toHaveTextContent('$720.02');
 expect(document.querySelector('.assessment-basis')).toHaveTextContent('2026-09-01 through 2026-09-30 · Approved records only · Incomplete');
 expect(document.querySelector('.assessment-review')).toHaveTextContent('Decided rows: 80%');
 expect(screen.getByText(/Necessary\/discretionary expenses, verified/)).toHaveTextContent('unavailable');
 expect(mocks.getClientDashboard).toHaveBeenCalledWith('client-a',{fromMonth:'2026-09',throughMonth:'2026-09'});
 fireEvent.change(screen.getByLabelText('Assessment through month'),{target:{value:'2026-10'}});
 expect(screen.getByRole('status')).toHaveTextContent('Results below still use the displayed assessment period');expect(mocks.getClientDashboard).toHaveBeenCalledTimes(1);
});
it('clears stale totals on a new review revision and ignores an older reply',async()=>{
 const old=deferred<FinancialAssessment>();mocks.getClientDashboard.mockReturnValueOnce(old.promise);
 const view=render(<FinancialDashboard source={source} disabled={false}/>);apply();
 mocks.getClientDashboard.mockResolvedValue({...result,revision:8,operatingCashflow:'1020.04',categories:[]});
 mocks.getFinancialHealth.mockResolvedValue({...healthFixture,revision:8});
 view.rerender(<FinancialDashboard source={{...source,revision:8}} disabled={false}/>);
 await bankDetails();
 await screen.findByText('No approved spending in this period.');
 await act(async()=>old.resolve(result));
 expect(document.querySelector('.assessment-basis')).toHaveTextContent('Source revision 8');expect(screen.queryByText('<script>Synthetic Housing</script>')).not.toBeInTheDocument();
});
it('shows failures without earlier totals and supports a read-only retry; mismatched client data stays hidden',async()=>{
 mocks.getClientDashboard.mockRejectedValueOnce(new Error('Synthetic read failure'));
 render(<FinancialDashboard source={source} disabled={false}/>);apply();
 expect(await screen.findByRole('alert')).toHaveTextContent('Synthetic read failure');
 mocks.getClientDashboard.mockResolvedValue({...result,clientId:'client-b'});fireEvent.click(screen.getByRole('button',{name:'Retry financial dashboard'}));
 expect(await screen.findByText(/The source records changed/)).toBeInTheDocument();expect(screen.queryByRole('table',{name:'Approved spending categories'})).not.toBeInTheDocument();
 mocks.getClientDashboard.mockResolvedValue(result);fireEvent.click(screen.getByRole('button',{name:'Retry financial dashboard'}));
 await screen.findByRole('table',{name:'Approved spending categories'});
});
it('guards conflicting operations and shows negative values and outdated periods without inventing balances',async()=>{
 mocks.getClientDashboard.mockResolvedValue({...result,outdated:true,operatingCashflow:'-20.01',months:[{...result.months[0],operatingCashflow:'-20.01'}]});
 const view=render(<FinancialDashboard source={source} disabled={true}/>);expect(screen.getByRole('button',{name:'View financial dashboard'})).toBeDisabled();
 view.rerender(<FinancialDashboard source={source} disabled={false}/>);apply();
 const table=await screen.findByRole('table',{name:'Monthly operating cash flow'});expect(within(table).getByText('-$20.01')).toBeInTheDocument();
 expect(document.querySelector('.assessment-basis')).toHaveTextContent('Outdated period');expect(table.querySelector('.deficit')).not.toBeNull();
});
