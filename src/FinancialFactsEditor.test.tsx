import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {expect,it,vi} from 'vitest';
import FinancialFactsEditor from './FinancialFactsEditor';
import type {ClientImports,ImportedRecord,FinancialFactsData,FinancialFactsInput} from './types';
const fields={date:'2026-09-01',amount:'1000.01',description:'Fictional payroll',merchant:'',category:'Wages',kind:'income' as const};
const row:ImportedRecord={id:'r1',documentId:'d1',accountId:'a1',fields,status:'Approved',raw:{extractionMethod:'csv-utf8-v1',recordNumber:2,line:2,reportedDate:'2026-09-01',reportedAmount:'1000.01',reportedDebit:null,reportedCredit:null,reportedDescription:'Fictional payroll',reportedMerchant:null,reportedCategory:'Wages',reportedKind:'income',reportedReference:null,reportedBalance:null,extracted:fields,reference:null,balance:null,issues:[]},linkedId:null,reason:'Verified',warnings:[],duplicateIds:[],bulkEligible:false};
const history={id:'approval-1',recordId:'r1',fromStatus:'Pending' as const,toStatus:'Approved' as const,fields,reason:'Verified',reviewedAt:'2026-10-14T00:00:00Z',counselorName:'Morgan'};
const source:ClientImports={clientId:'client-a',revision:1,accounts:[],documents:[],records:[row],history:[history],merchantGroups:[],recurringHistory:[],summary:{pending:0,needsClarification:0,approved:1,rejected:0,income:'1000.01',spending:'0.00',refunds:'0.00',netSpending:'0.00',netCash:'1000.01',accountNet:[],warnings:[]},financialFacts:{data:null,history:[]}};
const data:FinancialFactsData={asOf:'2026-09-30',fullInventory:true,balances:[{label:'Checking',kind:'cash',amount:'0.00',monthlyPayment:null,evidence:'Verified fictional September balance',documentId:null}],classifications:[{recordId:'r1',approvalId:'approval-1',role:'takeHomePay'}]};
function show(){fireEvent.click(screen.getByText('Verified classifications and dated balance facts'));}
it('collects explicit binding, unknown balance and confirmation without changing source amounts',async()=>{
 const save=vi.fn(async()=>true),dirty=vi.fn();render(<FinancialFactsEditor saved={source} disabled={false} onDirty={dirty} onReview={save}/>);show();
 fireEvent.change(screen.getByLabelText('Classification for Fictional payroll (2)'),{target:{value:'takeHomePay'}});
 fireEvent.change(screen.getByLabelText('Balance facts as-of date'),{target:{value:'2026-09-30'}});fireEvent.click(screen.getByRole('button',{name:'Add verified balance'}));
 fireEvent.change(screen.getByLabelText('Balance 1 label'),{target:{value:'Checking'}});fireEvent.change(screen.getByLabelText('Balance 1 verification evidence'),{target:{value:'Reviewed local fictional balance'}});
 fireEvent.change(screen.getByLabelText('Financial facts review reason'),{target:{value:'Checked the evidence'}});
 fireEvent.click(screen.getByLabelText(/I checked these facts/));fireEvent.click(screen.getByRole('button',{name:'Save verified financial facts'}));
 await waitFor(()=>expect(save).toHaveBeenCalled());
 const i=save.mock.calls[0] as unknown as [FinancialFactsInput];expect(i[0].data.classifications).toEqual([{recordId:'r1',approvalId:'approval-1',role:'takeHomePay'}]);expect(i[0].data.balances[0].amount).toBeNull();expect(i[0].acknowledge).toBe(true);expect(source.records[0].fields.amount).toBe('1000.01');
});
it('retains failed drafts and permits an explicit discard with dirty state cleared',async()=>{
 const dirty=vi.fn(),save=vi.fn(async()=>false);render(<FinancialFactsEditor saved={source} disabled={false} onDirty={dirty} onReview={save}/>);show();
 fireEvent.change(screen.getByLabelText('Financial facts review reason'),{target:{value:'Keep my failed draft'}});fireEvent.click(screen.getByRole('button',{name:'Save verified financial facts'}));await waitFor(()=>expect(save).toHaveBeenCalled());expect(screen.getByLabelText('Financial facts review reason')).toHaveValue('Keep my failed draft');expect(dirty).toHaveBeenLastCalledWith(true);
 fireEvent.click(screen.getByRole('button',{name:'Discard financial fact edits'}));expect(screen.getByLabelText('Financial facts review reason')).toHaveValue('');expect(dirty).toHaveBeenLastCalledWith(false);
});
it('same facts under a new approval do not auto-reactivate; explicit reclassification binds the new decision',async()=>{
 const save=vi.fn(async()=>false);const changed={...source,revision:5,history:[history,{...history,id:'new-approval'}],financialFacts:{data,history:[]}};
 render(<FinancialFactsEditor saved={changed} disabled={false} onDirty={vi.fn()} onReview={save}/>);show();
 expect(screen.getByText(/Needs re-verification: prior/)).toBeInTheDocument();expect(screen.getByLabelText('Classification for Fictional payroll (2)')).toHaveValue('');
 fireEvent.change(screen.getByLabelText('Classification for Fictional payroll (2)'),{target:{value:'takeHomePay'}});fireEvent.click(screen.getByRole('button',{name:'Save verified financial facts'}));await waitFor(()=>expect(save).toHaveBeenCalled());
 const i=save.mock.calls[0] as unknown as [FinancialFactsInput];expect(i[0].data.classifications[0].approvalId).toBe('new-approval');
});
it('guards competing operations, resets client context and exposes immutable balance history',async()=>{
 const saved={...source,financialFacts:{data,history:[{id:'facts-1',data,reason:'Verified original snapshot',reviewedAt:'2026-10-14T00:00:00Z',counselorName:'Morgan'}]}};
 const view=render(<FinancialFactsEditor saved={saved} disabled={true} onDirty={vi.fn()} onReview={vi.fn()}/>);show();expect(screen.getByRole('button',{name:'Save verified financial facts'})).toBeDisabled();
 fireEvent.click(screen.getByText('Financial fact history (1)'));fireEvent.click(screen.getByText(/Verified original snapshot/));expect(screen.getByText(/Verified fictional September balance/)).toBeInTheDocument();
 await act(async()=>view.rerender(<FinancialFactsEditor saved={{...source,clientId:'client-b',records:[],history:[]}} disabled={false} onDirty={vi.fn()} onReview={vi.fn()}/>));expect(screen.getByLabelText('Balance facts as-of date')).toHaveValue('');expect(screen.queryByLabelText('Balance 1 label')).not.toBeInTheDocument();expect(screen.queryByText(/Verified original snapshot/)).not.toBeInTheDocument();
});
