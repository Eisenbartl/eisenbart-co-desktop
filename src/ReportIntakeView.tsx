import type {ClientIntake,IntakeSection} from './types';
const labels:Record<string,string>={'':'Not recorded',employed:'Employed',selfEmployed:'Self-employed',unemployed:'Unemployed',retired:'Retired',student:'Student',other:'Other'};
export default function ReportIntakeView({values,sections}:{values:ClientIntake;sections:IntakeSection[]}) {
  const rows:[string,string][]=[];
  if(sections.includes('address'))rows.push(['Mailing address',values.address]);
  if(sections.includes('household'))rows.push(['Household composition',values.household]);
  if(sections.includes('employment'))rows.push(['Employment status',labels[values.employment]||'Not recorded'],['Employer',values.employer],['Occupation',values.occupation],['Employment and income context',values.incomeNotes]);
  return <section className="report-intake-context"><h4>Selected intake context</h4><p className="field-help">Counselor-entered information, separate from verified financial facts and model inputs. Follow-up tasks and outcome notes are excluded. Only selected sections are included.</p>{rows.length?<dl className="report-intake-values">{rows.map(([label,value])=><div key={label}><dt>{label}</dt><dd>{value||'Not recorded'}</dd></div>)}</dl>:<p className="field-help">Choose at least one intake section to include.</p>}</section>;
}
