const encoder=new TextEncoder();
export class ApiError extends Error{constructor(message,status=400){super(message);this.status=status;}}
export const fail=(message,status=400)=>{throw new ApiError(message,status);};
export const random=(size=32)=>b64(crypto.getRandomValues(new Uint8Array(size)));
export function b64(bytes){return btoa(String.fromCharCode(...bytes)).replaceAll('+','-').replaceAll('/','_').replace(/=+$/,'');}
export function unb64(value){return Uint8Array.from(atob(value.replaceAll('-','+').replaceAll('_','/')),c=>c.charCodeAt(0));}
export async function hash(value){return [...new Uint8Array(await crypto.subtle.digest('SHA-256',encoder.encode(value)))].map(v=>v.toString(16).padStart(2,'0')).join('');}
export async function keyed(value,secret){const key=await crypto.subtle.importKey('raw',encoder.encode(secret),{name:'HMAC',hash:'SHA-256'},false,['sign']);return b64(new Uint8Array(await crypto.subtle.sign('HMAC',key,encoder.encode(value))));}
export function equal(a,b){let difference=a.length^b.length;for(let i=0;i<Math.max(a.length,b.length);i++)difference|=(a.charCodeAt(i)||0)^(b.charCodeAt(i)||0);return difference===0;}
export async function passwordHash(password,salt,pepper){
 const material=await keyed(password,pepper);
 const key=await crypto.subtle.importKey('raw',encoder.encode(material),'PBKDF2',false,['deriveBits']);
 return b64(new Uint8Array(await crypto.subtle.deriveBits({name:'PBKDF2',salt:unb64(salt),iterations:100000,hash:'SHA-256'},key,256)));
}
export function password(value){if(typeof value!=='string'||value.length<12||value.length>256)fail('Das Passwort muss 12 bis 256 Zeichen lang sein.');return value;}
export function text(value,max=100){if(typeof value!=='string'||!value.trim()||value.length>max||/[\u0000-\u001f]/.test(value))fail('Ungültige Texteingabe.');return value.trim();}
export function email(value){const v=text(value,254).toLowerCase();if(!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v))fail('Bitte eine gültige E-Mail-Adresse eingeben.');return v;}
export function mac(value){const v=String(value||'').toUpperCase().replace(/[:-]/g,'');return /^[0-9A-F]{12}$/.test(v)?v:null;}
export function webUrl(value,optional=false){if(optional&&!value)return '';let u;try{u=new URL(text(value,4096));}catch{fail('Bitte eine vollständige HTTP- oder HTTPS-Adresse eingeben.');}if(!['http:','https:'].includes(u.protocol)||u.username||u.password)fail('Nur HTTP/HTTPS ohne Zugangsdaten im Adressfeld verwenden.');return u.href;}
export function profile(value){
 if(!value||!['Xtream','M3u'].includes(value.kind))fail('Unbekannte Profilart.');
 const p={name:text(value.name,80),kind:value.kind,host:'',username:'',password:'',m3uUrl:'',epgUrl:webUrl(value.epgUrl,true)};
 if(p.kind==='Xtream'){
  p.host=webUrl(value.host).replace(/\/$/,'');const u=new URL(p.host);
  if(u.search||u.hash)fail('Die Xtream-Serveradresse darf keine Abfrageparameter enthalten.');
  for(const key of ['username','password']){if(typeof value[key]!=='string'||!value[key].length||value[key].length>256||/[\u0000-\u001f]/.test(value[key]))fail('Ungültige Anbieter-Zugangsdaten.');p[key]=value[key];}
 }else p.m3uUrl=webUrl(value.m3uUrl);
 return p;
}
export async function encrypt(value,secret,aad){const key=await crypto.subtle.importKey('raw',unb64(secret),'AES-GCM',false,['encrypt']);const iv=crypto.getRandomValues(new Uint8Array(12));const cipher=await crypto.subtle.encrypt({name:'AES-GCM',iv,additionalData:encoder.encode(aad)},key,encoder.encode(JSON.stringify(value)));return b64(iv)+'.'+b64(new Uint8Array(cipher));}
export async function decrypt(value,secret,aad){const [iv,cipher]=value.split('.');const key=await crypto.subtle.importKey('raw',unb64(secret),'AES-GCM',false,['decrypt']);return JSON.parse(new TextDecoder().decode(await crypto.subtle.decrypt({name:'AES-GCM',iv:unb64(iv),additionalData:encoder.encode(aad)},key,unb64(cipher))));}
export function json(value,status=200,headers={}){return new Response(JSON.stringify(value),{status,headers:{'Content-Type':'application/json; charset=utf-8','Cache-Control':'no-store','X-Content-Type-Options':'nosniff',...headers}});}
export async function body(request){
 if(!request.headers.get('content-type')?.startsWith('application/json'))fail('JSON-Anfrage erforderlich.',415);
 const reader=request.body?.getReader();if(!reader)fail('Leere Anfrage.');let size=0;const chunks=[];
 while(true){const {done,value}=await reader.read();if(done)break;size+=value.length;if(size>16384){await reader.cancel();fail('Anfrage zu groß.',413);}chunks.push(value);}
 const bytes=new Uint8Array(size);let offset=0;for(const c of chunks){bytes.set(c,offset);offset+=c.length;}
 try{return JSON.parse(new TextDecoder().decode(bytes));}catch{fail('Ungültige Anfrage.');}
}
export const safeError=error=>json({error:error instanceof ApiError?error.message:'Dienst vorübergehend nicht verfügbar.'},error instanceof ApiError?error.status:503);
