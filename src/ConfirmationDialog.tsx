import {useEffect,useRef,useState} from 'react';
type Request={message:string;onDiscard:()=>void;returnFocus:HTMLElement|null;title?:string;actionLabel?:string;cancelLabel?:string;help?:string};
let receive:((request:Request)=>void)|null=null;

/** Tauri's window.confirm returns a Promise. Never treat it as a boolean. */
export function confirmDiscard(message:string,onDiscard:()=>void):void {
 if(receive){receive({message,onDiscard,returnFocus:document.activeElement instanceof HTMLElement?document.activeElement:null});return;}
 // Standalone consumers without an app host may use a browser/native dialog.
 // Support both browser booleans and Tauri's asynchronous result explicitly.
 const result:boolean|Promise<boolean>=window.confirm(message);
 if(typeof result==='boolean'){if(result)onDiscard();}
 else void Promise.resolve(result).then(accepted=>{if(accepted)onDiscard();});
}

export function confirmAction(message:string,onConfirm:()=>void,actionLabel='Confirm change',title='Confirm household change'):void {
 if(receive){receive({message,onDiscard:onConfirm,returnFocus:document.activeElement instanceof HTMLElement?document.activeElement:null,title,actionLabel,cancelLabel:'Cancel',help:'Review the change before confirming. Existing historical records will be preserved.'});return;}
 const result:boolean|Promise<boolean>=window.confirm(message);if(typeof result==='boolean'){if(result)onConfirm();}else void Promise.resolve(result).then(accepted=>{if(accepted)onConfirm();});
}

export default function ConfirmationDialog(){
 const [request,setRequest]=useState<Request|null>(null),pending=useRef<Request|null>(null),keep=useRef<HTMLButtonElement>(null);
 useEffect(()=>{const handler=(next:Request)=>{if(pending.current)return;pending.current=next;setRequest(next);};receive=handler;return()=>{if(receive===handler)receive=null;pending.current=null;};},[]);
 useEffect(()=>{if(request)keep.current?.focus();},[request]);
 function finish(discard:boolean){const current=pending.current;if(!current)return;pending.current=null;setRequest(null);if(discard)current.onDiscard();queueMicrotask(()=>{if(current.returnFocus?.isConnected)current.returnFocus.focus({preventScroll:true});});}
 if(!request)return null;
 return <div className="discard-dialog-backdrop"><section className="discard-dialog panel" role="alertdialog" aria-modal="true" aria-labelledby="discard-dialog-title" aria-describedby="discard-dialog-message" onKeyDown={e=>{if(e.key==='Escape'){e.preventDefault();finish(false);}if(e.key==='Tab'){const controls=[...e.currentTarget.querySelectorAll<HTMLButtonElement>('button')];if(e.shiftKey&&document.activeElement===controls[0]){e.preventDefault();controls.at(-1)?.focus();}else if(!e.shiftKey&&document.activeElement===controls.at(-1)){e.preventDefault();controls[0]?.focus();}}}}><p className="eyebrow">{request.title?'REVIEW CHANGE':'UNSAVED CHANGES'}</p><h2 id="discard-dialog-title">{request.title??'Discard unsaved changes?'}</h2><p id="discard-dialog-message">{request.message}</p><p className="field-help">{request.help??'Choose Keep editing to return and save your changes first.'}</p><div className="discard-dialog-actions"><button className="button secondary" ref={keep} onClick={()=>finish(false)}>{request.cancelLabel??'Keep editing'}</button><button className="button primary" onClick={()=>finish(true)}>{request.actionLabel??'Discard changes'}</button></div></section></div>;
}
