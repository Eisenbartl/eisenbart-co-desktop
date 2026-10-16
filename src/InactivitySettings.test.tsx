import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {beforeEach,expect,it,vi} from 'vitest';
import InactivitySettings from './InactivitySettings';
import type {SecurityPolicy} from './types';
const mocks=vi.hoisted(()=>({getSecurityPolicy:vi.fn(),saveSecurityPolicy:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const original:SecurityPolicy={revision:0,idleMinutes:15,totalChanges:0,history:[]};
const saved:SecurityPolicy={revision:1,idleMinutes:5,totalChanges:1,history:[{revision:1,idleMinutes:5,savedAt:'2026-10-16T13:00:00Z'}]};
function view(){const onBusy=vi.fn(),onDirty=vi.fn();return{...render(<InactivitySettings onBusy={onBusy} onDirty={onDirty}/>),onBusy,onDirty};}
beforeEach(()=>{vi.clearAllMocks();mocks.getSecurityPolicy.mockResolvedValue(original);mocks.saveSecurityPolicy.mockResolvedValue(saved);});
it('loads legacy default and saves strings with the exact revision and retained history',async()=>{
 const v=view();const field=await screen.findByLabelText('Inactivity timeout (minutes)');expect(field).toHaveValue(15);expect(screen.getByRole('button',{name:'Save inactivity timeout'})).toBeDisabled();
 fireEvent.change(field,{target:{value:'5'}});expect(v.onDirty).toHaveBeenLastCalledWith(true);fireEvent.click(screen.getByRole('button',{name:'Save inactivity timeout'}));
 expect(await screen.findByRole('status')).toHaveTextContent('Inactivity timeout saved.');expect(mocks.saveSecurityPolicy).toHaveBeenCalledWith({expectedRevision:0,idleMinutes:'5'});expect(v.onDirty).toHaveBeenLastCalledWith(false);
 fireEvent.click(screen.getByText('Security setting history'));expect(screen.getByText(/Revision 1: 5 minutes/)).toBeInTheDocument();expect(screen.getByText(/America\/Chicago/)).toBeInTheDocument();
});
it('shows authoritative validation and stale errors without replaying an uncertain save',async()=>{
 view();const field=await screen.findByLabelText('Inactivity timeout (minutes)');mocks.saveSecurityPolicy.mockRejectedValueOnce('Security settings changed in another instance. Reload before saving.');fireEvent.change(field,{target:{value:'5'}});fireEvent.click(screen.getByRole('button',{name:'Save inactivity timeout'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('Reload before saving');expect(screen.getByLabelText('Inactivity timeout (minutes)')).toHaveValue(5);expect(screen.getByRole('button',{name:'Save inactivity timeout'})).toBeDisabled();
 const confirm=vi.spyOn(window,'confirm').mockReturnValueOnce(false).mockReturnValueOnce(true);fireEvent.click(screen.getByRole('button',{name:'Reload security policy'}));expect(mocks.getSecurityPolicy).toHaveBeenCalledTimes(1);
 mocks.getSecurityPolicy.mockResolvedValueOnce({...saved,idleMinutes:10});fireEvent.click(screen.getByRole('button',{name:'Reload security policy'}));await waitFor(()=>expect(screen.getByLabelText('Inactivity timeout (minutes)')).toHaveValue(10));expect(mocks.saveSecurityPolicy).toHaveBeenCalledTimes(1);confirm.mockRestore();
});
it('prevents duplicate writes while pending and ignores late save results after unmount',async()=>{
 let resolve!:(value:SecurityPolicy)=>void;mocks.saveSecurityPolicy.mockReturnValueOnce(new Promise(r=>{resolve=r;}));const v=view();fireEvent.change(await screen.findByLabelText('Inactivity timeout (minutes)'),{target:{value:'5'}});fireEvent.click(screen.getByRole('button',{name:'Save inactivity timeout'}));expect(v.onBusy).toHaveBeenLastCalledWith(true);expect(screen.getByRole('button',{name:'Reload security policy'})).toBeDisabled();
 fireEvent.submit(screen.getByRole('button',{name:'Saving inactivity timeout…'}).closest('form')!);expect(mocks.saveSecurityPolicy).toHaveBeenCalledTimes(1);v.unmount();await act(async()=>resolve(saved));expect(document.documentElement.outerHTML).not.toContain('Inactivity timeout saved');expect(v.onBusy).toHaveBeenLastCalledWith(false);expect(v.onDirty).toHaveBeenLastCalledWith(false);
});
it('offers retry after a read error and ignores late policy reads after unmount',async()=>{
 mocks.getSecurityPolicy.mockRejectedValueOnce('Synthetic policy read error');const v=view();expect(await screen.findByRole('alert')).toHaveTextContent('Synthetic policy read error');fireEvent.click(screen.getByRole('button',{name:'Reload security policy'}));await screen.findByLabelText('Inactivity timeout (minutes)');expect(mocks.getSecurityPolicy).toHaveBeenCalledTimes(2);v.unmount();
 let resolve!:(value:SecurityPolicy)=>void;mocks.getSecurityPolicy.mockReturnValueOnce(new Promise(r=>{resolve=r;}));const second=view();second.unmount();await act(async()=>resolve(saved));expect(screen.queryByLabelText('Inactivity timeout (minutes)')).not.toBeInTheDocument();
});
it('releases the parent busy guard when a fast reload completes within one render batch',async()=>{
 const v=view();await screen.findByLabelText('Inactivity timeout (minutes)');v.onBusy.mockClear();
 await act(async()=>{screen.getByRole('button',{name:'Reload security policy'}).click();await Promise.resolve();});
 expect(v.onBusy).toHaveBeenCalledWith(true);expect(v.onBusy).toHaveBeenLastCalledWith(false);
});
