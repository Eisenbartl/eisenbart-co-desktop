export const workspacePages = [
  {id:'Dashboard',title:'Dashboard',description:'Your practice overview, recent clients, appointments and follow-ups.',icon:'dashboard'},
  {id:'Clients',title:'Clients',description:'Create and update profiles, record intake, add session notes and archive clients.',icon:'clients'},
  {id:'Financial Review',title:'Financial Review',description:'Review approved activity, verified balances, statement checks and recurring payments.',icon:'review'},
  {id:'Transactions',title:'Transactions',description:'Find imported transactions, review original evidence and save approval or correction decisions.',icon:'transactions'},
  {id:'Documents',title:'Documents',description:'Import local statements and review this client’s source documents and account inventory.',icon:'file'},
  {id:'Budget Planner',title:'Budget Planner',description:'Create and revise income, expenses and allocations in a saved monthly budget.',icon:'budget'},
  {id:'Debt Planner',title:'Debt Planner',description:'Maintain debts and compare repayment strategies using entered terms.',icon:'debt'},
  {id:'Savings Planner',title:'Savings Planner',description:'Maintain emergency savings, reserves, sinking funds and contribution plans.',icon:'savings'},
  {id:'Cash-Flow Calendar',title:'Cash-Flow Calendar',description:'Maintain dated accounts, paychecks, bills and transfers; inspect projected shortfalls.',icon:'calendar'},
  {id:'Financial Scenarios',title:'Financial Scenarios',description:'Compare hypothetical changes and save reviewed scenarios without rewriting live plans.',icon:'scenarios'},
  {id:'Reports',title:'Reports',description:'Record counselor assessments and client action plans; reopen history and export local PDFs.',icon:'file'},
  {id:'Settings',title:'Settings and Administration',description:'Workspace identity, security, backup and recovery, customization and administration.',icon:'settings'},
] as const;
export type View = typeof workspacePages[number]['id'];
export type ModuleView = Exclude<View,'Dashboard'|'Clients'|'Settings'>;
export type ClientSection = 'record'|'profile'|'intake'|'notes'|'history'|ModuleView;
export const clientSections = [
  ['record','Complete record'],['profile','Profile'],['intake','Intake & follow-ups'],['notes','Session notes'],['history','History & records'],
] as const;
export function isModule(view:View):view is ModuleView {return view!=='Dashboard'&&view!=='Clients'&&view!=='Settings';}
