import type {AssessmentPlans,ScenarioExpense} from './types';

export interface ExpenseDraft {
 label:string;kind:ScenarioExpense['kind'];amount:string;frequency:ScenarioExpense['frequency'];
 includeDated:boolean;accountId:string;firstDate:string;secondDay:string;untilDate:string;
}
export function expenseInputs(expenses:ExpenseDraft[]):ScenarioExpense[]{
 return expenses.map(e=>({label:e.label,kind:e.kind,amount:e.amount.trim()||null,frequency:e.frequency,
  dated:e.includeDated?{accountId:e.accountId,firstDate:e.firstDate,
   secondDay:e.frequency==='semimonthly'&&e.secondDay.trim()?Number(e.secondDay):null,
   untilDate:e.untilDate.trim()||null}:null}));
}
export default function ScenarioExpenseControls({plans,expenses,onChange}:{plans:AssessmentPlans;expenses:ExpenseDraft[];onChange:(d:ExpenseDraft[])=>void}){
 function edit(index:number,patch:Partial<ExpenseDraft>){onChange(expenses.map((e,i)=>i===index?{...e,...patch}:e));}
 return <details className="scenario-new-expenses"><summary>Add new recurring expenses</summary>
  <p className="field-help">Add estimated necessary or discretionary spending to this proposal. Existing bills remain in effect; check for duplicate obligations. The monthly budget assumption applies throughout the model. Optional payment dates affect only the dated forecast. Nothing is applied to live plans.</p>
  {expenses.map((e,index)=><div className="budget-line" key={index}>
   <div className="form-grid">
    <label htmlFor={`scenario-expense-label-${index}`}>New expense {index+1} name<input id={`scenario-expense-label-${index}`} maxLength={120} value={e.label} onChange={v=>edit(index,{label:v.target.value})}/></label>
    <label htmlFor={`scenario-expense-kind-${index}`}>New expense {index+1} purpose<select id={`scenario-expense-kind-${index}`} value={e.kind} onChange={v=>edit(index,{kind:v.target.value as ExpenseDraft['kind']})}><option value="necessary">Necessary</option><option value="discretionary">Discretionary</option></select></label>
    <label htmlFor={`scenario-expense-amount-${index}`}>New expense {index+1} amount (USD)<input id={`scenario-expense-amount-${index}`} inputMode="decimal" value={e.amount} onChange={v=>edit(index,{amount:v.target.value})}/></label>
    <label htmlFor={`scenario-expense-frequency-${index}`}>New expense {index+1} frequency<select id={`scenario-expense-frequency-${index}`} value={e.frequency} onChange={v=>edit(index,{frequency:v.target.value as ExpenseDraft['frequency']})}>{['weekly','biweekly','semimonthly','monthly','annual'].map(f=><option key={f} value={f}>{f}</option>)}</select></label>
   </div>
   <label className="check-label"><input id={`scenario-expense-dated-${index}`} type="checkbox" disabled={!plans.cashflow.data} checked={e.includeDated} onChange={v=>edit(index,{includeDated:v.target.checked})}/>Also add new expense {index+1} to the dated forecast</label>
   {!plans.cashflow.data&&<p className="field-help">Save a cash-flow plan before choosing an account and dates. This proposal can still include the budget expense.</p>}
   {e.includeDated&&<div className="form-grid">
    <p className="field-help">Use the same entered amount and frequency. Annual expenses support budget estimates only. Saved forecast: {plans.cashflow.data?.startDate} through {plans.cashflow.data?.endDate}. Blank last date means end of forecast; no opening balance changes.</p>
    <label htmlFor={`scenario-expense-account-${index}`}>New expense {index+1} payment account<select id={`scenario-expense-account-${index}`} value={e.accountId} onChange={v=>edit(index,{accountId:v.target.value})}><option value="">Choose an account</option>{plans.cashflow.data?.accounts.map(a=><option key={a.id} value={a.id}>{a.label}</option>)}</select></label>
    <label htmlFor={`scenario-expense-first-${index}`}>New expense {index+1} first date (YYYY-MM-DD)<input id={`scenario-expense-first-${index}`} maxLength={10} placeholder="YYYY-MM-DD" value={e.firstDate} onChange={v=>edit(index,{firstDate:v.target.value})}/></label>
    {e.frequency==='semimonthly'&&<label htmlFor={`scenario-expense-second-${index}`}>New expense {index+1} second day (1–31)<input id={`scenario-expense-second-${index}`} inputMode="numeric" value={e.secondDay} onChange={v=>edit(index,{secondDay:v.target.value})}/></label>}
    <label htmlFor={`scenario-expense-until-${index}`}>New expense {index+1} last date (optional)<input id={`scenario-expense-until-${index}`} maxLength={10} placeholder="YYYY-MM-DD" value={e.untilDate} onChange={v=>edit(index,{untilDate:v.target.value})}/></label>
   </div>}
   <button type="button" className="text-button" onClick={()=>onChange(expenses.filter((_,i)=>i!==index))}>Remove proposed expense {index+1}</button>
  </div>)}
  <button type="button" className="text-button" disabled={expenses.length>=20||plans.budget.lines.length+expenses.length>=100} onClick={()=>onChange([...expenses,{label:'',kind:'necessary',amount:'',frequency:'monthly',includeDated:false,accountId:'',firstDate:'',secondDay:'',untilDate:''}])}>Add proposed recurring expense</button>
 </details>;
}
