import {fireEvent,render,screen,within} from '@testing-library/react';
import {expect,it,vi} from 'vitest';
import ClientDirectory from './ClientDirectory';
import type {Client} from './types';
const clients:Client[]=Array.from({length:32},(_,i)=>({id:`stable-client-${i}`,organizationId:'org',firstName:'Fictional',lastName:`Client ${String(i).padStart(2,'0')}`,email:`person${i}@example.test`,phone:`555-${String(i).padStart(4,'0')}`,householdSize:1,status:i===0?'New Intake':i===1?'Financial Review':i===2?'Archived':'Planning',goals:'',createdAt:'2026-10-16T00:00:00Z',updatedAt:'2026-10-16T00:00:00Z'}));
it('uses real lifecycle counts, stable IDs and bounded pages without any individual record form',()=>{
 const onOpen=vi.fn();render(<ClientDirectory clients={clients} loading={false} error="" search="" setSearch={vi.fn()} status="All statuses" setStatus={vi.fn()} onOpen={onOpen} onAdd={vi.fn()}/>);
 expect(screen.getByText('Total clients').parentElement).toHaveTextContent('32');expect(screen.getByText('New intakes').parentElement).toHaveTextContent('1');expect(screen.getByText('In progress').parentElement).toHaveTextContent('29');expect(screen.getByText('Clients in review').parentElement).toHaveTextContent('1');
 expect(within(screen.getByRole('table')).getAllByRole('row')).toHaveLength(26);expect(screen.queryByLabelText('New session note')).not.toBeInTheDocument();
 fireEvent.click(screen.getByRole('button',{name:'Open workspace for Fictional Client 00'}));expect(onOpen).toHaveBeenLastCalledWith('stable-client-0');
 fireEvent.keyDown(screen.getByRole('row',{name:'Open Fictional Client 01 workspace'}),{key:'Enter'});expect(onOpen).toHaveBeenLastCalledWith('stable-client-1');
 fireEvent.click(screen.getByRole('button',{name:'Next clients'}));expect(screen.getByText('26–32 of 32 clients')).toBeInTheDocument();expect(screen.queryByText('Fictional Client 00')).not.toBeInTheDocument();
});
it('combines literal contact search and real status filtering without fabricating a last-activity date',()=>{
 const props={clients,loading:false,error:'',setSearch:vi.fn(),setStatus:vi.fn(),onOpen:vi.fn(),onAdd:vi.fn()};
 const v=render(<ClientDirectory {...props} search="555-0001" status="All statuses"/>);expect(screen.getByText('Fictional Client 01')).toBeInTheDocument();expect(screen.queryByText('Fictional Client 00')).not.toBeInTheDocument();
 expect(screen.getByRole('columnheader',{name:'Profile updated'})).toBeInTheDocument();
 v.rerender(<ClientDirectory {...props} search="person1@example.test" status="Planning"/>);expect(screen.getByText('No matching clients')).toBeInTheDocument();
});
