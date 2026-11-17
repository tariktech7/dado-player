import {useState,useLayoutEffect,useRef} from 'preact/hooks';
import {createPortal} from 'preact/compat';
import {Profile} from './types';
import config from '../../../connect/public-config.json';
type Device={deviceId:string;address:string;owner:string|null;pairingCode?:string;pairingExpires?:number};
type Saved=Partial<Device>&{token:string};
const KEY='dado.connect.v1';
function saved():Saved{try{return JSON.parse(localStorage.getItem(KEY)||'{"token":""}');}catch{return {token:''};}}
function persist(value:Saved){localStorage.setItem(KEY,JSON.stringify(value));}
function token(){return btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(32)))).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');}
async function post<T>(secret:string,path:string,data:unknown):Promise<T>{
 const abort=new AbortController(),timer=setTimeout(()=>abort.abort(),25000);
 try{
  const response=await fetch(config.apiUrl+'/v1/device/'+path,{method:'POST',headers:{Authorization:'Bearer '+secret,'Content-Type':'application/json'},body:JSON.stringify(data),signal:abort.signal,credentials:'omit'});
  const raw=await response.text();if(raw.length>512*1024)throw Error('Cloud-Antwort zu groß.');
  const value=JSON.parse(raw);if(!response.ok)throw Error(typeof value.error==='string'?value.error:'Cloud-Anfrage fehlgeschlagen.');return value;
 }finally{clearTimeout(timer);}
}
export function cloudProfile(value:unknown,id:string):Profile{
 const p=value as Profile;
 const url=(value:string)=>{try{const u=new URL(value);return ['http:','https:'].includes(u.protocol)&&!u.username&&!u.password;}catch{return false;}};
 if(!/^[a-f0-9-]{36}$/.test(id)||!p||typeof p.name!=='string'||!p.name.trim()||p.name.length>80||!['Xtream','M3u'].includes(p.kind))throw Error('Ungültiges Cloud-Profil.');
 if(p.kind==='Xtream'&&(!url(p.host)||typeof p.username!=='string'||!p.username||typeof p.password!=='string'||!p.password)||p.kind==='M3u'&&!url(p.m3uUrl))throw Error('Anbieteradresse oder Zugangsdaten fehlen.');
 return {id:'cloud-'+id,name:p.name,kind:p.kind,host:p.kind==='Xtream'?p.host:'',username:p.kind==='Xtream'?p.username:'',password:p.kind==='Xtream'?p.password:'',m3uUrl:p.kind==='M3u'?p.m3uUrl:'',epgUrl:''};
}
export function ConnectPanel({importProfiles,compact=false}:{importProfiles:(profiles:Profile[])=>number;compact?:boolean}){
 const [open,setOpen]=useState(false),modal=useRef<HTMLElement>(null);
 useLayoutEffect(()=>{
  if(!compact||!open)return;
  const previous=document.activeElement as HTMLElement;
  const key=(e:KeyboardEvent)=>{if(e.key==='Escape'||e.keyCode===10009||e.key==='BrowserBack'){e.preventDefault();e.stopImmediatePropagation();setOpen(false);}};
  document.addEventListener('keydown',key,true);modal.current?.querySelector('button')?.focus({preventScroll:true});
  return()=>{document.removeEventListener('keydown',key,true);previous?.focus({preventScroll:true});};
 },[compact,open]);
 const [state,setState]=useState<Saved>(saved),[busy,setBusy]=useState(false),[message,setMessage]=useState(''),[reset,setReset]=useState(false);
 async function run(pair=false,rebind=false){
  if(busy)return;setBusy(true);setMessage('Cloud-Verbindung wird hergestellt …');setReset(false);
  let next={...saved()};
  try{
   if(!next.token){next.token=token();persist(next);}
   let mac:string|null=null;try{mac=(window as any).webapis?.network?.getMac?.()||null;}catch{}
   const device=await post<Device>(next.token,'register',{platform:'samsung',mac});
   if(pair){const paired=await post<Device>(next.token,'pairing',{reset:rebind});next={...next,...paired};persist(next);setState(next);setMessage('Adresse und Code im Portal eintragen. Der Code gilt zehn Minuten.');}
   else{
    const inbox=await post<Device&{messages:{id:string;profile:unknown}[]}>(next.token,'pull',{});
    next={...next,...device,...inbox};delete (next as any).messages;if(next.owner){delete next.pairingCode;delete next.pairingExpires;}persist(next);setState(next);
    const profiles=inbox.messages.map(m=>cloudProfile(m.profile,m.id));
    if(!profiles.length){setMessage(next.owner?'Keine neuen Profile vorhanden.':'Noch nicht gekoppelt. Bitte zuerst Gerät verbinden.');return;}
    const count=importProfiles(profiles); // Must durably save or throw BEFORE ack.
    try{await post(next.token,'ack',{ids:inbox.messages.map(m=>m.id)});setMessage(count+' neue Profile gespeichert. Unter Profile auswählen und anmelden.');}
    catch{setMessage('Profile gespeichert. Empfangsbestätigung fehlt; bitte Aktualisieren erneut drücken.');}
   }
  }catch(error){setMessage(error instanceof Error&&error.name!=='AbortError'?error.message:'Cloud-Dienst nicht erreichbar. Verbindung prüfen.');}
  finally{setBusy(false);}
 }
 const panel=<section class="glass connect-panel" data-focus-zone="connect"><h2>Profile vom Handy/PC</h2><p>Dado Connect · Konto im Portal erstellen und vom Betreiber freigeben lassen.</p><p class="connect-url">{config.portalUrl}</p>
 {state.address&&<p>Geräteadresse: <strong>{state.address}</strong></p>}{state.owner&&<p>Verbunden mit: {state.owner}</p>}
 {state.pairingCode&&<p>Kopplungscode: <strong>{(state.pairingExpires||0)>Date.now()/1000?state.pairingCode:'Abgelaufen – bitte neu erzeugen'}</strong><small> Nur an die Person weitergeben, die Profile senden darf.</small></p>}
 {!state.owner&&<button disabled={busy} onClick={()=>void run(true)}>Gerät verbinden</button>} <button disabled={busy} onClick={()=>void run()}>{busy?'Wird abgerufen …':'Aktualisieren'}</button>
 {state.owner&&<button disabled={busy} onClick={()=>setReset(true)}>Verbindung trennen</button>}
 {reset&&<div><p>Zuordnung und ungelesene Sendungen entfernen? Gespeicherte Profile bleiben erhalten.</p><button onClick={()=>void run(true,true)}>Trennen bestätigen</button><button onClick={()=>setReset(false)}>Abbrechen</button></div>}
 <p role="status">{message}</p><small>Abruf nur auf Knopfdruck. Keine automatische Anmeldung oder Wiedergabe.</small></section>;
 return compact?<><button onClick={()=>setOpen(true)}>Profile vom Handy/PC</button>{open&&createPortal(<div class="scrim"><section ref={modal} class="modal glass" role="dialog" aria-modal="true" aria-label="Dado Connect"><button onClick={()=>setOpen(false)}>Zurück zu Profilen</button>{panel}</section></div>,document.getElementById('app')!)}</>:panel;
}
