import {act,fireEvent,render,screen} from '@testing-library/react';
import {beforeEach,expect,it,vi} from 'vitest';
import PasswordChange from './PasswordChange';
const mocks=vi.hoisted(()=>({changeWorkspacePassword:vi.fn()}));
vi.mock('./api',()=>({api:mocks}));
const old='Synthetic-original-password-12',next='Synthetic-new-password-34';
function form(){const onChanged=vi.fn(),onBusy=vi.fn(),onDirty=vi.fn();const view=render(<PasswordChange onChanged={onChanged} onBusy={onBusy} onDirty={onDirty}/>);return{...view,onChanged,onBusy,onDirty};}
function fill(){fireEvent.change(screen.getByLabelText('Current password'),{target:{value:old}});fireEvent.change(screen.getByLabelText('New password'),{target:{value:next}});fireEvent.change(screen.getByLabelText('Confirm new password'),{target:{value:next}});}
function submit(){fireEvent.click(screen.getByRole('button',{name:'Change password and lock'}));}
beforeEach(()=>{vi.clearAllMocks();mocks.changeWorkspacePassword.mockResolvedValue({durabilityWarning:false});});
it('requires matching different passwords and explicit retained-copy acknowledgment',async()=>{
 form();fill();fireEvent.change(screen.getByLabelText('Confirm new password'),{target:{value:'Synthetic-different-password'}});submit();expect(screen.getByRole('alert')).toHaveTextContent('do not match');expect(mocks.changeWorkspacePassword).not.toHaveBeenCalled();
 fireEvent.change(screen.getByLabelText('Confirm new password'),{target:{value:next}});submit();expect(screen.getByRole('alert')).toHaveTextContent('original passwords');expect(mocks.changeWorkspacePassword).not.toHaveBeenCalled();
 fireEvent.change(screen.getByLabelText('New password'),{target:{value:old}});fireEvent.change(screen.getByLabelText('Confirm new password'),{target:{value:old}});submit();expect(screen.getByRole('alert')).toHaveTextContent('different');
});
it('clears all secrets before IPC, prevents duplicate submissions and reports committed warnings',async()=>{
 let resolve!:(result:{durabilityWarning:boolean})=>void;const pending=new Promise<{durabilityWarning:boolean}>(r=>{resolve=r;});mocks.changeWorkspacePassword.mockReturnValueOnce(pending);
 const view=form();fill();fireEvent.click(screen.getByRole('checkbox'));submit();
 for(const label of ['Current password','New password','Confirm new password'])expect(screen.getByLabelText(label)).toHaveValue('');
 expect(screen.getByRole('checkbox')).not.toBeChecked();expect(view.onBusy).toHaveBeenLastCalledWith(true);expect(screen.getByRole('button',{name:'Changing workspace password…'})).toBeDisabled();
 fireEvent.submit(screen.getByRole('button',{name:'Changing workspace password…'}).closest('form')!);expect(mocks.changeWorkspacePassword).toHaveBeenCalledTimes(1);
 expect(mocks.changeWorkspacePassword).toHaveBeenCalledWith({currentPassword:old,newPassword:next,confirmation:next,acknowledgeRetainedCopies:true});
 await act(async()=>resolve({durabilityWarning:true}));expect(view.onChanged).toHaveBeenCalledWith({durabilityWarning:true});
});
it('leaves failed requests empty and requires fresh reviewed credentials to retry',async()=>{
 mocks.changeWorkspacePassword.mockRejectedValueOnce('The current password is incorrect.');const view=form();fill();fireEvent.click(screen.getByRole('checkbox'));submit();expect(await screen.findByRole('alert')).toHaveTextContent('current password is incorrect');expect(view.onChanged).not.toHaveBeenCalled();
 expect(screen.getByLabelText('Current password')).toHaveValue('');expect(screen.getByLabelText('New password')).toHaveValue('');expect(screen.getByLabelText('Confirm new password')).toHaveValue('');expect(screen.getByRole('checkbox')).not.toBeChecked();
});
it('clears a draft explicitly and ignores late results after unmount',async()=>{
 const view=form();fill();expect(view.onDirty).toHaveBeenLastCalledWith(true);fireEvent.click(screen.getByRole('button',{name:'Clear password fields'}));expect(view.onDirty).toHaveBeenLastCalledWith(false);expect(screen.getByLabelText('New password')).toHaveValue('');
 let resolve!:(result:{durabilityWarning:boolean})=>void;mocks.changeWorkspacePassword.mockReturnValueOnce(new Promise(r=>{resolve=r;}));fill();fireEvent.click(screen.getByRole('checkbox'));submit();view.unmount();await act(async()=>resolve({durabilityWarning:false}));expect(view.onChanged).not.toHaveBeenCalled();expect(view.onBusy).toHaveBeenLastCalledWith(false);
});
