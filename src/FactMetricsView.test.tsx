import {render,screen} from '@testing-library/react';
import {expect,it} from 'vitest';
import FactMetricsView from './FactMetricsView';
import type {FactMetrics} from './types';
const value:FactMetrics={necessarySpending:'300.02',discretionarySpending:'0.01',unclassifiedSpending:'0.00',takeHomePay:'500.01',unclassifiedIncome:'0.00',staleClassifications:0,asOf:'2026-09-30',balanceComplete:true,balanceAligned:true,balanceOutdated:false,cash:'100.01',accessibleSavings:'600.05',restrictedSavings:'25.02',debt:'200.03',netWorth:'525.05',monthlyNecessaryEstimate:'300.02',emergencyMonthsEstimate:'2.00',takeHomePaymentPercentEstimate:'5.00',debtBalances:[{label:'Fictional loan',amount:'200.03',scale:1000}]};
it('shows canonical dated metrics, relative debt chart and clearly labeled ratios',()=>{
 render(<FactMetricsView value={value}/>);expect(document.querySelector('.facts-basis')).toHaveTextContent('As of 2026-09-30 · Complete inventory declared');
 expect(document.querySelector('.verified-balance-totals')).toHaveTextContent('Estimated net worth at balance date$525.05');expect(document.querySelector('.verified-ratios')).toHaveTextContent('2.00 months');expect(screen.getByRole('table',{name:'Verified debt balance chart'})).toHaveTextContent('$200.03');expect(screen.getByText(/Coverage uses accessible/)).toHaveTextContent("not a lender's gross-income debt-to-income ratio");
});
it('keeps missing facts and outdated/misaligned bases explicit and never substitutes zero for unknown',()=>{
 render(<FactMetricsView value={{...value,asOf:'2026-03-01',balanceComplete:false,balanceAligned:false,balanceOutdated:true,cash:null,netWorth:null,emergencyMonthsEstimate:null,takeHomePaymentPercentEstimate:null,staleClassifications:1,debtBalances:[{label:'Unverified amount',amount:null,scale:0}]}}/>);
 expect(document.querySelector('.facts-basis')).toHaveTextContent('Incomplete inventory · Outdated balance facts · Date differs from assessment end');expect(document.querySelector('.verified-balance-totals')).toHaveTextContent('Recorded cash / checkingUnavailable');expect(document.querySelector('.verified-ratios')).toHaveTextContent('Accessible savings coverage — estimateUnavailable');expect(screen.getByRole('table',{name:'Verified debt balance chart'}).querySelector('svg')).toBeNull();
});
