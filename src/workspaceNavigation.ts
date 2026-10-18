export const workspacePages = [
  {id:'Dashboard',title:'Dashboard',description:'Your practice overview, recent clients, appointments and follow-ups.',icon:'dashboard'},
  {id:'Clients',title:'Clients',description:'Your client directory. Search, review progress, and open a dedicated workspace for each client.',icon:'clients'},
  {id:'Spending Analysis',title:'Spending Analysis',description:'Understand approved categorized spending and record reviewed spending purposes without changing source imports.',icon:'spending'},
  {id:'Financial Review',title:'Financial Review',description:'Review approved activity, verified balances, statement checks and recurring payments.',icon:'review'},
  {id:'Transactions',title:'Transactions',description:'Find imported transactions, review original evidence and save approval or correction decisions.',icon:'transactions'},
  {id:'Documents',title:'Documents',description:'Import local statements and review this client’s source documents and account inventory.',icon:'file'},
  {id:'Budget Planner',title:'Budget Planner',description:'Create and revise income, expenses and allocations in a saved monthly budget.',icon:'budget'},
  {id:'Debt Planner',title:'Debt Planner',description:'Maintain debts and compare repayment strategies using entered terms.',icon:'debt'},
  {id:'Savings Planner',title:'Savings Planner',description:'Maintain emergency savings, reserves, sinking funds and contribution plans.',icon:'savings'},
  {id:'Cash-Flow Calendar',title:'Cash-Flow Calendar',description:'Maintain dated accounts, paychecks, bills and transfers; inspect projected shortfalls.',icon:'calendar'},
  {id:'Financial Scenarios',title:'Financial Scenarios',description:'Compare hypothetical changes and save reviewed scenarios without rewriting live plans.',icon:'scenarios'},
  {id:'Reports',title:'Reports',description:'Record counselor assessments and client action plans; reopen history and export local PDFs.',icon:'file'},
  {id:'Settings',title:'Settings',description:'Workspace identity, security, backup and recovery, customization and administration.',icon:'settings'},
] as const;
export type View = typeof workspacePages[number]['id'];
export type ModuleView = Exclude<View,'Dashboard'|'Clients'|'Settings'>;
export type ClientSection = 'overview'|'profile'|'intake'|'notes'|'history'|ModuleView;
export const clientSections = [
  ['profile','Profile'],['intake','Intake & follow-ups'],['notes','Session notes'],['history','History & records'],
] as const;
export function isModule(view:View):view is ModuleView {return view!=='Dashboard'&&view!=='Clients'&&view!=='Settings';}

export const clientWorkspaceGroups = [
 {title:'Workspace',items:[{view:'Clients',section:'overview',label:'Overview',icon:'dashboard'},{view:'Clients',section:'profile',label:'Profile & intake',icon:'clients'},{view:'Clients',section:'notes',label:'Session notes',icon:'file'}]},
 {title:'Planning tools',items:[{view:'Budget Planner',label:'Monthly budget',icon:'budget'},{view:'Debt Planner',label:'Debt payoff',icon:'debt'},{view:'Savings Planner',label:'Savings & sinking funds',icon:'savings'},{view:'Cash-Flow Calendar',label:'Cash-flow calendar',icon:'calendar'},{view:'Financial Scenarios',label:'Scenario comparisons',icon:'scenarios'}]},
 {title:'Documents & review',items:[{view:'Documents',label:'Import statements',icon:'file'},{view:'Transactions',label:'Review & reconciliation',icon:'review'},{view:'Spending Analysis',label:'Spending Analysis',icon:'spending'},{view:'Financial Review',label:'Financial dashboard',icon:'dashboard'},{view:'Reports',label:'Assessments & reports',icon:'file'}]},
] as const;
