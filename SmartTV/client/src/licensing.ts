import config from '../../../licensing/public-config.json';
import {fingerprint,deviceCode,normalizeMac,macAlias,verifyLicense,statusText,type Claims} from '../../../licensing/shared/protocol.mjs';

export interface LicenseSnapshot {
 deviceId:string; subject:string; mac:string|null; identitySource:string;
 claims:Claims|null; busy:boolean; ready:boolean; error:string; text:string; allowed:boolean;
}
const STORAGE='dado.license.v1';
type LicenseConfig={enabled:boolean;apiUrl:string;keys:Record<string,JsonWebKey>};
export class LicenseClient {
 state:LicenseSnapshot={deviceId:'Wird ermittelt …',subject:'',mac:null,identitySource:'',claims:null,busy:false,ready:false,error:'',text:'Testzeit noch nicht gestartet',allowed:true};
 private settings:LicenseConfig;
 constructor(options:LicenseConfig=config){this.settings=options;this.state.allowed=!options.enabled;}
 private resume:(()=>void)|undefined;
 dispose(){if(this.timer)clearTimeout(this.timer);if(this.resume){document.removeEventListener('visibilitychange',this.resume);window.removeEventListener('focus',this.resume);}this.listeners.clear();}
 private lastCheck=0;
 private listeners=new Set<()=>void>();
 private high=0; private originTime=0; private originTick=performance.now();
 private token=''; private timer:ReturnType<typeof setTimeout>|undefined;
 private initialized:Promise<void>|null=null;
 subscribe(listener:()=>void){this.listeners.add(listener);return()=>{this.listeners.delete(listener);};}
 private now(){return Math.max(Math.floor(Date.now()/1000),this.high,this.originTime+Math.floor((performance.now()-this.originTick)/1000));}
 private emit(){this.state={...this.state};for(const l of this.listeners)l();}
 private update(){
  const now=this.now(), c=this.state.claims;
  this.state.text=statusText(c,now);
  this.state.allowed=!this.settings.enabled || !!c && (c.kind==='permanent'||now<c.expires_at!);
  if(this.timer) clearTimeout(this.timer);
  if(c?.kind==='trial' && now<c.expires_at!) {
   const remaining=c.expires_at!-now;
   this.timer=setTimeout(()=>{this.update();this.emit();},Math.min(remaining,(remaining%86400)||86400)*1000+50);
  }
 }
 private persist(lastCheck:number){this.lastCheck=lastCheck;localStorage.setItem(STORAGE,JSON.stringify({token:this.token,high:this.now(),lastCheck}));}
 init(){return this.initialized ||= this.initialize();}
 private async initialize(){
  try {
   const api=(window as any).webapis;
   let raw='';
   try {raw=String(api?.productinfo?.getDuid()||'');} catch { /* unavailable */ }
   if(!raw){
    if(api?.avplay) throw Error('Gerätekennung nicht verfügbar. App-Berechtigung prüfen.');
    raw=localStorage.getItem('dado.preview-device')||crypto.randomUUID();
    localStorage.setItem('dado.preview-device',raw);
    this.state.identitySource='Browser-Testgerät';
   } else this.state.identitySource='Samsung DUID';
   try {this.state.mac=normalizeMac(api?.network?.getMac()||'');} catch { /* MAC not exposed */ }
   this.state.subject=await fingerprint('samsung',raw);this.state.deviceId=deviceCode(this.state.subject);
   let saved:{token?:string;high?:number;lastCheck?:number}={};
   try {saved=JSON.parse(localStorage.getItem(STORAGE)||'{}');} catch {this.state.error='Gespeicherte Lizenz ist beschädigt. Bitte wiederherstellen.';}
   this.lastCheck=saved.lastCheck||0;this.high=Number.isSafeInteger(saved.high)?saved.high!:0;this.originTime=this.now();
   if(saved.token){
    try {this.state.claims=await verifyLicense(saved.token,this.settings.keys,this.state.subject,'samsung',this.now());this.token=saved.token;this.high=Math.max(this.high,this.state.claims.issued_at);}
    catch {this.state.error='Lizenz konnte nicht geprüft werden. Bitte wiederherstellen.';}
   }
   this.state.ready=true;this.update();this.emit();
   // A permanent cached entitlement never triggers an automatic server request.
   if(this.settings.enabled && this.state.claims?.kind==='trial' && this.now()-(saved.lastCheck||0)>=21600) await this.request('status');
   const resume=()=>{if(this.state.claims?.kind==='trial'){this.update();try{this.persist(this.lastCheck);}catch{this.state.error='Lizenzspeicher nicht verfügbar';}this.emit();}};
   this.resume=resume;document.addEventListener('visibilitychange',resume);window.addEventListener('focus',resume);
  }catch(e){this.state.ready=true;this.state.error=String((e as Error).message);this.emit();}
 }
 startTrial(){return this.request('trial');}
 refresh(){return this.request('activation');}
 private async request(mode:'trial'|'activation'|'status'){
  if(this.state.busy || !this.state.subject) return;
  if(!this.settings.enabled || !this.settings.apiUrl){this.state.error='Der Lizenzdienst ist noch nicht eingerichtet.';this.emit();return;}
  this.state.busy=true;this.state.error='';this.emit();
  const controller=new AbortController(), timeout=setTimeout(()=>controller.abort(),10000);
  try {
   const path=mode==='trial'?'/v1/trial/start':mode==='activation'?'/v1/license/activation':'/v1/license';
   const response=await fetch(this.settings.apiUrl+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({subject:this.state.subject,platform:'samsung',mac_hash:this.state.mac?await macAlias(this.state.mac):null}),signal:controller.signal});
   const data=await response.json();
   if(!response.ok) throw Error(data.code==='NOT_APPROVED'?'Noch keine Freischaltung vorhanden.':data.code==='TRIAL_NOT_STARTED'?'Testversion noch nicht gestartet.':'Freischaltung derzeit nicht abrufbar. Bitte erneut versuchen.');
   const claims=await verifyLicense(data.license,this.settings.keys,this.state.subject,'samsung',this.now());
   if(mode==='activation' && claims.kind!=='permanent') throw Error('Noch keine Freischaltung vorhanden.');
   this.high=Math.max(this.now(),claims.issued_at);this.originTime=this.high;this.originTick=performance.now();this.token=data.license;
   this.persist(this.now());this.state.claims=claims;this.update();
  }catch(e){this.state.error=(e as Error).message;}
  finally{clearTimeout(timeout);this.state.busy=false;this.emit();}
 }
}
export const licenseClient=new LicenseClient();
