import { useEffect, useRef, useState, type FormEvent } from 'react';
import { api } from './api';
import type { BudgetFrequency, BudgetKind, BudgetLine, Client, ClientBudget } from './types';

const kinds: [BudgetKind, string][] = [['income', 'Take-home income'], ['necessary', 'Necessary expense'], ['discretionary', 'Discretionary expense'], ['savings', 'Savings allocation'], ['debtPayment', 'Debt payment allocation']];
const frequencies: [BudgetFrequency, string][] = [['weekly', 'Weekly'], ['biweekly', 'Every two weeks'], ['semimonthly', 'Twice a month'], ['monthly', 'Monthly'], ['annual', 'Annual'], ['irregular', 'Irregular (monthly estimate)']];
function message(error: unknown) { return error instanceof Error ? error.message : String(error); }
// Format the exact decimal returned by Rust without a floating-point conversion.
export function usd(value: string): string {
  const negative = value.startsWith('-');
  const [whole, cents] = (negative ? value.slice(1) : value).split('.');
  return `${negative ? '-' : ''}$${whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',')}.${cents}`;
}

export default function BudgetEditor({ client, onDirty, onBusy, otherSaving }: {
  client: Client; onDirty: (value: boolean) => void; onBusy: (value: boolean) => void; otherSaving: boolean;
}) {
  const [saved, setSaved] = useState<ClientBudget | null>(null);
  const [lines, setLines] = useState<BudgetLine[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [loadError, setLoadError] = useState('');
  const [error, setError] = useState('');
  const [attempt, setAttempt] = useState(0);
  const [notice, setNotice] = useState('');
  const mounted = useRef(true);
  const dirty = saved !== null && JSON.stringify(lines) !== JSON.stringify(saved.lines);
  useEffect(() => { onDirty(dirty); }, [dirty, onDirty]);
  useEffect(() => { onBusy(saving); }, [saving, onBusy]);
  useEffect(() => {
    mounted.current = true;
    let current = true;
    setSaved(null); setLines([]); setLoading(true); setLoadError(''); setError(''); setNotice('');
    api.getClientBudget(client.id).then(result => {
      if (!current) return;
      if (result.clientId !== client.id) { setLoadError('The budget could not be loaded for this client.'); return; }
      setSaved(result); setLines(result.lines);
    }).catch(caught => { if (current) setLoadError(message(caught)); })
      .finally(() => { if (current) setLoading(false); });
    return () => { current = false; mounted.current = false; };
  }, [client.id, attempt]);

  function update(index: number, patch: Partial<BudgetLine>) {
    setLines(previous => previous.map((line, i) => i === index ? { ...line, ...patch } : line));
    setNotice(''); setError('');
  }
  function add(kind: BudgetKind) {
    setLines(previous => [...previous, { label: '', kind, amount: null, frequency: 'monthly', estimated: false }]);
    setNotice(''); setError('');
  }
  async function save(event: FormEvent) {
    event.preventDefault();
    if (!saved || saving || otherSaving) return;
    setError(''); setNotice(''); setSaving(true);
    onBusy(true);
    try {
      const result = await api.saveClientBudget(client.id, { expectedRevision: saved.revision, lines });
      if (!mounted.current) return;
      if (result.clientId !== client.id) { setError('The saved budget response did not match this client. Reopen the record.'); return; }
      setSaved(result); setLines(result.lines); setNotice('Budget saved and calculated.');
    } catch (caught) { if (mounted.current) setError(message(caught)); }
    finally { if (mounted.current) { setSaving(false); onBusy(false); } }
  }

  const summary = saved?.summary;
  return <section className="panel budget-panel" aria-labelledby="budget-title">
    <div className="panel-heading"><div><p className="eyebrow">CLIENT SPENDING PLAN</p><h2 id="budget-title">Monthly budget</h2></div><span className="count-badge">USD</span></div>
    <p className="muted small">Plan for {client.firstName} {client.lastName}. Enter take-home income and customize your spending categories. These planning amounts do not change historical transactions.</p>
    {loading ? <p role="status">Loading budget…</p> : loadError ? <><div className="error-message" role="alert">{loadError}</div><button type="button" className="button secondary" onClick={() => setAttempt(value => value + 1)}>Retry loading budget</button></> : <>
      <form onSubmit={save} noValidate>
        <fieldset disabled={saving || otherSaving}>
          <p className="field-help">Amounts are in USD, without commas or dollar signs (for example, 1250.50). Leave an unknown amount blank; enter 0 for a known zero. Irregular amounts are estimated monthly totals.</p>
          {!lines.length && <p className="empty-note">No budget lines yet. Add income, expenses, and any planned allocations.</p>}
          <div className="budget-lines">{lines.map((line, index) => <div className="budget-line" key={index}>
            <div className="budget-line-heading"><span>Line {index + 1}</span><button type="button" className="text-button" aria-label={`Remove line ${index + 1}`} onClick={() => { setLines(previous => previous.filter((_, i) => i !== index)); setNotice(''); }}>Remove</button></div>
            <div className="form-grid">
              <label htmlFor={`budget-label-${index}`}>Category or source<input id={`budget-label-${index}`} value={line.label} maxLength={120} onChange={e => update(index, { label: e.target.value })} /></label>
              <label htmlFor={`budget-kind-${index}`}>Type<select id={`budget-kind-${index}`} value={line.kind} onChange={e => update(index, { kind: e.target.value as BudgetKind })}>{kinds.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
              <label htmlFor={`budget-amount-${index}`}>Amount (USD)<input id={`budget-amount-${index}`} type="text" inputMode="decimal" value={line.amount ?? ''} maxLength={16} placeholder="Unknown" onChange={e => update(index, { amount: e.target.value === '' ? null : e.target.value })} /></label>
              <label htmlFor={`budget-frequency-${index}`}>Frequency<select id={`budget-frequency-${index}`} value={line.frequency} onChange={e => update(index, { frequency: e.target.value as BudgetFrequency, estimated: e.target.value === 'irregular' || line.estimated })}>{frequencies.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
            </div>
            <label className="budget-estimate" htmlFor={`budget-estimated-${index}`}><input id={`budget-estimated-${index}`} type="checkbox" checked={line.estimated || line.frequency === 'irregular'} disabled={line.frequency === 'irregular'} onChange={e => update(index, { estimated: e.target.checked })} />Estimated amount</label>
            {!dirty && summary && <p className="muted small">Monthly equivalent: <strong>{summary.monthlyAmounts[index] === null ? 'Unknown — excluded from totals' : usd(summary.monthlyAmounts[index])}</strong></p>}
          </div>)}</div>
          <div className="budget-add"><button type="button" className="button secondary" disabled={lines.length >= 100} onClick={() => add('income')}>Add income</button><button type="button" className="button secondary" disabled={lines.length >= 100} onClick={() => add('necessary')}>Add expense</button><button type="button" className="button secondary" disabled={lines.length >= 100} onClick={() => add('savings')}>Add allocation</button></div>
          {error && <div className="error-message" role="alert">{error}</div>}
          <div className="form-actions"><span className="save-status" role="status">{notice || (dirty ? 'Unsaved budget changes' : saved?.revision ? 'Saved client plan' : 'No saved budget yet')}</span><button className="button primary" type="submit">{saving ? 'Calculating…' : 'Save budget and calculate'}</button></div>
        </fieldset>
      </form>
      {dirty ? <p className="budget-pending" role="status">Save the budget to calculate totals for your changes.</p> : summary && <div className="budget-results">
        <h3>Monthly plan summary</h3>
        <dl className="budget-totals">{([
          ['Take-home income', summary.monthlyIncome], ['Necessary expenses', summary.necessaryExpenses], ['Discretionary expenses', summary.discretionaryExpenses],
          ['Total spending before allocations', summary.totalSpending], ['Surplus / deficit before allocations', summary.surplusBeforeAllocations],
          ['Proposed savings', summary.proposedSavings], ['Capacity for debt payments after savings', summary.availableForDebt],
          ['Planned debt payments', summary.plannedDebtPayments], ['Unallocated / deficit after allocations', summary.unallocated],
        ]).map(([label, value]) => <div key={label}><dt>{label}</dt><dd className={value.startsWith('-') ? 'budget-deficit' : ''}>{usd(value)}</dd></div>)}</dl>
        {summary.warnings.length > 0 && <div className="budget-warnings"><h4>Review this plan</h4><ul>{summary.warnings.map(warning => <li key={warning}>{warning}</li>)}</ul></div>}
        <p className="field-help">Monthly averages use 52 weekly or 26 biweekly payments per year, 2 twice-monthly payments per month, and annual amounts ÷ 12. Each line rounds to cents before totals are added. Averages do not predict the timing of paychecks or bills. Debt payments belong in debt allocations, not both expenses and allocations.</p>
      </div>}
    </>}
  </section>;
}
