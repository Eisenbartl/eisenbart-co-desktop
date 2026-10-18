import {act,fireEvent,render,screen} from '@testing-library/react';
import {expect,it,vi} from 'vitest';
import PaycheckAllocation from './PaycheckAllocation';
const get=vi.hoisted(()=>vi.fn());vi.mock('./api',()=>({api:{getPaycheckAllocation:get}}));
it('renders authoritative exact earmarks and refreshes without writing plans',async()=>{
 get.mockResolvedValue({clientId:'a',revisions:{budget:3,cashflow:2},labels:['Rent'],warnings:['No automatic payments'],rows:[{date:'2026-11-06',label:'Wages',accountId:'checking',certainty:'estimated',paycheck:'500.00',allocations:['500.01'],total:'500.01',remainder:'-0.01',nextPaycheck:null,datedCommitments:'200.00',warnings:['Earmarks exceed this paycheck']}]});
 render(<PaycheckAllocation clientId="a"/>);expect(await screen.findByRole('table',{name:'Budget earmarks for 2026-11-06'})).toHaveTextContent('$500.01');expect(screen.getByText('-$0.01')).toBeInTheDocument();expect(screen.getByText('Earmarks exceed this paycheck')).toBeInTheDocument();
 fireEvent.click(screen.getByText('Refresh paycheck allocations'));await screen.findByRole('table');expect(get).toHaveBeenCalledTimes(2);
});
it('clears client data and ignores a late private response after client switch',async()=>{
 let resolve!:(v:unknown)=>void;get.mockReturnValueOnce(new Promise(r=>{resolve=r;})).mockRejectedValueOnce(Error('Unavailable'));
 const {rerender}=render(<PaycheckAllocation clientId="a"/>);rerender(<PaycheckAllocation clientId="b"/>);expect(await screen.findByRole('alert')).toHaveTextContent('Unavailable');await act(async()=>resolve({clientId:'a',labels:[],rows:[],warnings:['Private client a']}));expect(screen.queryByText('Private client a')).not.toBeInTheDocument();
});
