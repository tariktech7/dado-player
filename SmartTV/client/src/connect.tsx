import {useState} from 'preact/hooks';
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
 const p=value as Profile&{sourceId?:string};
 const url=(value:string)=>{try{const u=new URL(value);return ['http:','https:'].includes(u.protocol)&&!u.username&&!u.password;}catch{return false;}};
 if(!/^[a-f0-9-]{36}$/.test(id)||!p||typeof p.name!=='string'||!p.name.trim()||p.name.length>80||!['Xtream','M3u'].includes(p.kind))throw Error('Ungültiges Cloud-Profil.');
 if(p.kind==='Xtream'&&(!url(p.host)||typeof p.username!=='string'||!p.username||typeof p.password!=='string'||!p.password)||p.kind==='M3u'&&!url(p.m3uUrl))throw Error('Anbieteradresse oder Zugangsdaten fehlen.');
 if(p.sourceId!==undefined&&!/^[a-f0-9-]{36}$/.test(p.sourceId))throw Error('Ungültige Cloud-Vorlage.');
 return {id:p.sourceId?'cloud-profile-'+p.sourceId:'cloud-'+id,name:p.name,kind:p.kind,host:p.kind==='Xtream'?p.host:'',username:p.kind==='Xtream'?p.username:'',password:p.kind==='Xtream'?p.password:'',m3uUrl:p.kind==='M3u'?p.m3uUrl:'',epgUrl:''};
}
export function mergeCloudProfiles(existing:Profile[],incoming:Profile[]):{profiles:Profile[];added:number}{
 const profiles=[...existing];let added=0;
 for(const p of incoming){const index=profiles.findIndex(old=>old.id===p.id);if(index<0){profiles.push(p);added++;}else if(p.id.startsWith('cloud-profile-'))profiles[index]=p;}
 return {profiles,added};
}

let registration:Promise<Saved>|null=null;
export function prepareCloudDevice(force=false):Promise<Saved>{
 if(registration&&!force)return registration;
 registration=(async()=>{
  const previous=saved();
  if(previous.deviceId&&previous.address&&!force)return previous;
  if(!previous.token){previous.token=token();persist(previous);}
  let mac:string|null=null;try{mac=(window as any).webapis?.network?.getMac?.()||null;}catch{}
  const device=await post<Device>(previous.token,'register',{platform:'samsung',mac});
  const next={...previous,...device};delete next.pairingCode;delete next.pairingExpires;persist(next);return next;
 })();
 return registration;
}
export function ConnectPanel({importProfiles,compact=false}:{importProfiles:(profiles:Profile[])=>number;compact?:boolean}){
 const [busy,setBusy]=useState(false),[message,setMessage]=useState('');
 async function refresh(){
  if(busy)return;setBusy(true);setMessage('');
  try{
   const state=await prepareCloudDevice(true);
   const inbox=await post<Device&{messages:{id:string;profile:unknown}[]}>(state.token,'pull',{});
   const next={...state,...inbox};delete (next as any).messages;persist(next);
   if(!inbox.messages.length){setMessage('Keine neuen Profile vorhanden.');return;}
   importProfiles(inbox.messages.map(m=>cloudProfile(m.profile,m.id)));
   try{await post(state.token,'ack',{ids:inbox.messages.map(m=>m.id)});setMessage('Profile gespeichert. In der Profilwahl auswählen.');}
   catch{setMessage('Profile gespeichert. Bitte Aktualisieren erneut drücken.');}
  }catch(error){setMessage(error instanceof Error&&error.name!=='AbortError'?error.message:'Profile konnten nicht abgerufen werden. Verbindung prüfen.');}
  finally{setBusy(false);}
 }
 return <div class={compact?'profile-refresh':'connect-panel'}><button disabled={busy} onClick={()=>void refresh()}>{busy?'Profile werden aktualisiert …':'Profile aktualisieren'}</button>{message&&<p role="status">{message}</p>}</div>;
}
