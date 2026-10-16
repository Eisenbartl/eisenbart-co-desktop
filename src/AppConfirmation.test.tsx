import {act,fireEvent,render,screen,waitFor,within} from '@testing-library/react';
import {expect,it,vi} from 'vitest';
import App from './App';
const fixture=vi.hoisted(()=>{
 const workspace={organizationId:'org',organizationName:'Fictional practice',counselorId:'counselor',counselorName:'Synthetic counselor'};
 const client={id:'stable-client',organizationId:'org',firstName:'Alex',lastName:'Synthetic',email:'',phone:'',householdSize:1,status:'Planning',goals:'Original saved goal',createdAt:'2026-10-16T00:00:00Z',updatedAt:'2026-10-16T00:00:00Z'};
 return {workspace,client,locked:undefined as (()=>void)|undefined,save:vi.fn()};
});
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async(_name,handler)=>{fixture.locked=handler;return()=>{fixture.locked=undefined;};})}));
vi.mock('./api',()=>({desktopAvailable:()=>true,resetApiSession:vi.fn(),api:{
 setupStatus:vi.fn(async()=>({configured:true})),unlockWorkspace:vi.fn(async()=>fixture.workspace),listClients:vi.fn(async()=>[fixture.client]),getClient:vi.fn(async()=>fixture.client),saveClient:fixture.save,
 getFollowUpOverview:vi.fn(async input=>({...fixture.workspace,input,reviewDate:'2026-10-16',timeZone:'America/Chicago',overdue:0,dueToday:0,upcoming:0,closed:0,total:0,pageSize:25,items:[]})),
 getClientWorkspaceOverview:vi.fn(async clientId=>({clientId,monthlyIncome:null,necessaryExpenses:null,unallocated:null,enteredDebt:null,debtMonthlyPayment:null,budgetDebtPayment:null,emergencyTarget:null,accessibleSavings:null,restrictedSavings:null,payoffMonth:null,warnings:[]})),
 getClientCare:vi.fn(async clientId=>({clientId,revision:0,updatedAt:null,reviewDate:'2026-10-16',overdue:0,dueToday:0,data:{intake:{},followUps:[]}})),listClientNotes:vi.fn(async()=>[]),
}}));
it('uses the real in-app cancel/discard choices and clears a pending private dialog on native locking',async()=>{
 render(<App/>);await screen.findByRole('button',{name:'Unlock workspace'});fireEvent.change(screen.getByLabelText('Workspace password'),{target:{value:'Synthetic-password-123'}});fireEvent.click(screen.getByRole('button',{name:'Unlock workspace'}));await screen.findByRole('heading',{name:'A clear view of your practice.'});
 fireEvent.click(screen.getByRole('button',{name:'Clients'}));fireEvent.click(await screen.findByRole('button',{name:'Open workspace for Alex Synthetic'}));await screen.findByRole('heading',{name:'Alex Synthetic'});fireEvent.click(within(screen.getByRole('navigation',{name:'Client workspace sections'})).getByRole('button',{name:'Profile & intake'}));
 fireEvent.change(screen.getByLabelText('Counseling goals'),{target:{value:'Private unsaved goal'}});fireEvent.click(screen.getByRole('tab',{name:'All clients'}));expect(screen.getByRole('alertdialog')).toHaveTextContent('Discard unsaved changes?');expect(screen.getByLabelText('Counseling goals')).toHaveValue('Private unsaved goal');
 fireEvent.click(screen.getByRole('button',{name:'Keep editing'}));expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();expect(screen.getByLabelText('Counseling goals')).toHaveValue('Private unsaved goal');
 fireEvent.click(screen.getByRole('tab',{name:'All clients'}));fireEvent.click(screen.getByRole('button',{name:'Discard changes'}));expect(screen.getByRole('region',{name:'Client directory'})).toBeInTheDocument();expect(document.documentElement.outerHTML).not.toContain('Private unsaved goal');expect(fixture.save).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Open workspace for Alex Synthetic'}));await screen.findByLabelText('Counseling goals');expect(screen.getByLabelText('Counseling goals')).toHaveValue('Original saved goal');fireEvent.change(screen.getByLabelText('Counseling goals'),{target:{value:'Second private goal'}});fireEvent.click(screen.getByRole('button',{name:'Close workspace for Alex Synthetic'}));expect(screen.getByRole('alertdialog')).toBeInTheDocument();
 await act(async()=>fixture.locked?.());await waitFor(()=>expect(screen.getByRole('heading',{name:'Welcome back'})).toBeInTheDocument());expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();expect(screen.queryByRole('tab')).not.toBeInTheDocument();expect(document.documentElement.outerHTML).not.toContain('Second private goal');expect(fixture.save).not.toHaveBeenCalled();
});
