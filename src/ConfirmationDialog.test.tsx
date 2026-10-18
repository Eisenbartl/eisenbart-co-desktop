import {act,fireEvent,render,screen} from '@testing-library/react';
import {expect,it,vi} from 'vitest';
import ConfirmationDialog,{confirmDiscard} from './ConfirmationDialog';
it('does not treat an asynchronous native confirmation Promise as approval',async()=>{
 let resolve!:(value:boolean)=>void;const pending=new Promise<boolean>(r=>{resolve=r;});
 const confirm=vi.spyOn(window,'confirm').mockImplementation((()=>pending) as unknown as typeof window.confirm);const discard=vi.fn();
 confirmDiscard('Discard a fictional draft?',discard);expect(discard).not.toHaveBeenCalled();await act(async()=>resolve(false));expect(discard).not.toHaveBeenCalled();
 confirm.mockImplementation((()=>Promise.resolve(true)) as unknown as typeof window.confirm);confirmDiscard('Discard another fictional draft?',discard);expect(discard).not.toHaveBeenCalled();await act(async()=>{});expect(discard).toHaveBeenCalledTimes(1);confirm.mockRestore();
});
it('requires an explicit in-app choice, defaults to keeping edits and contains keyboard focus',()=>{
 const discard=vi.fn();render(<><button>Original action</button><ConfirmationDialog/></>);screen.getByRole('button',{name:'Original action'}).focus();
 act(()=>confirmDiscard('A fictional draft has unsaved changes.',discard));expect(screen.getByRole('alertdialog')).toHaveTextContent('A fictional draft');expect(screen.getByRole('button',{name:'Keep editing'})).toHaveFocus();
 fireEvent.keyDown(screen.getByRole('alertdialog'),{key:'Tab',shiftKey:true});expect(screen.getByRole('button',{name:'Discard changes'})).toHaveFocus();fireEvent.keyDown(screen.getByRole('alertdialog'),{key:'Escape'});expect(discard).not.toHaveBeenCalled();expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
 act(()=>confirmDiscard('Discard now?',discard));fireEvent.click(screen.getByRole('button',{name:'Discard changes'}));expect(discard).toHaveBeenCalledTimes(1);
});
it('cancels pending confirmations on host unmount and never replays an obsolete action',()=>{
 const discard=vi.fn(),second=vi.fn();const v=render(<ConfirmationDialog/>);act(()=>{confirmDiscard('Private pending draft',discard);confirmDiscard('Second navigation',second);});expect(screen.getByRole('alertdialog')).toHaveTextContent('Private pending draft');v.unmount();render(<ConfirmationDialog/>);expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();expect(discard).not.toHaveBeenCalled();expect(second).not.toHaveBeenCalled();
});
