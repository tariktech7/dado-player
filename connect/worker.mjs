import {page,script,css,privacy} from './web.mjs';
import {ApiError,fail,random,hash,keyed,equal,passwordHash,password,text,email,mac,profile,encrypt,decrypt,json,body,safeError} from './protocol.mjs';
const SESSION='__Host-dado_connect',DAYS=86400;
const token=request=>request.headers.get('Authorization')?.match(/^Bearer ([A-Za-z0-9_-]{43})$/)?.[1]||fail('Geräteberechtigung fehlt.',401);
const address=h=>'02:'+h.slice(0,10).match(/../g).join(':').toUpperCase();
const publicUser=u=>({id:u.id,email:u.email,name:u.name,status:u.status});
async function limit(env,key,now,max,seconds){
 const bucket=Math.floor(now/seconds)+':'+await keyed(key,env.AUTH_PEPPER);
 const row=await env.DB.prepare('INSERT INTO rate_limits(bucket,count,expires) VALUES(?,1,?) ON CONFLICT(bucket) DO UPDATE SET count=count+1 RETURNING count').bind(bucket,now+seconds*2).first();
 if(row.count>max)fail('Zu viele Versuche. Bitte später erneut versuchen.',429);
}
async function user(request,env,now,approved=true){
 const cookie=request.headers.get('Cookie')?.split(';').map(v=>v.trim()).find(v=>v.startsWith(SESSION+'='))?.slice(SESSION.length+1);
 if(!cookie||!/^[A-Za-z0-9_-]{43}$/.test(cookie))fail('Bitte anmelden.',401);
 const row=await env.DB.prepare('SELECT accounts.* FROM sessions JOIN accounts ON accounts.id=sessions.user_id WHERE sessions.hash=? AND sessions.expires>?').bind(await hash(cookie),now).first();
 if(!row)fail('Bitte erneut anmelden.',401);
 if(approved&&row.status!=='approved')fail('Dein Konto ist noch nicht freigegeben oder wurde gesperrt.',403);
 return row;
}
async function device(request,env){const h=await hash(token(request));const row=await env.DB.prepare('SELECT * FROM devices WHERE token_hash=?').bind(h).first();if(!row)fail('Gerät zuerst verbinden.',401);return row;}
async function deviceState(env,d){const owner=d.owner_id?await env.DB.prepare('SELECT email,status FROM accounts WHERE id=?').bind(d.owner_id).first():null;return {deviceId:d.id,address:d.address,owner:owner?.email||null,ownerStatus:owner?.status||null};}
const cookie=(value,age=30*DAYS)=>`${SESSION}=${value}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=${age}`;
export function createHandler({clock=()=>Math.floor(Date.now()/1000)}={}){return async function fetch(request,env){
 const url=new URL(request.url),path=url.pathname,now=clock(),isDevice=path.startsWith('/v1/device/');
 let response;
 try{
  if(path==='/health')return json({ok:true,service:'dado-connect',protocol:1});
  if(request.method==='OPTIONS'&&isDevice)return new Response(null,{status:204,headers:{'Access-Control-Allow-Origin':'*','Access-Control-Allow-Methods':'POST, OPTIONS','Access-Control-Allow-Headers':'Authorization, Content-Type'}});
  if(request.method==='GET'&&['/','/privacy','/portal.js','/style.css'].includes(path)){
   return new Response(path==='/'?page:path==='/privacy'?privacy:path==='/portal.js'?script:css,{headers:{'Content-Type':['/','/privacy'].includes(path)?'text/html; charset=utf-8':path==='/portal.js'?'text/javascript; charset=utf-8':'text/css; charset=utf-8','Cache-Control':'no-store','Content-Security-Policy':"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",'Referrer-Policy':'no-referrer','X-Content-Type-Options':'nosniff','X-Frame-Options':'DENY'}});
  }
  if(!env.AUTH_PEPPER||!env.CRYPTO_KEY)fail('Dienst wird eingerichtet.',503);
  if(!isDevice&&request.method==='POST'&&request.headers.get('Origin')!==url.origin)fail('Ungültiger Ursprung.',403);
  const ip=request.headers.get('CF-Connecting-IP')||'local';
  if(path.startsWith('/api/auth/')&&request.method==='POST'){
   await limit(env,'auth:'+ip,now,20,900);
   const input=await body(request);
   if(path==='/api/auth/register'){
    await limit(env,'signup:'+ip,now,4,3600);
    const mail=email(input.email),name=text(input.name,80),salt=random(16),pw=password(input.password);
    const existing=await env.DB.prepare('SELECT id FROM accounts WHERE email=?').bind(mail).first();
    if(existing)fail('Konto kann nicht angelegt werden. Bitte Anmeldung oder Kontakt zur Verwaltung verwenden.',409);
    await env.DB.prepare('INSERT INTO accounts(id,email,name,salt,password_hash,created_at) VALUES(?,?,?,?,?,?)').bind(crypto.randomUUID(),mail,name,salt,await passwordHash(pw,salt,env.AUTH_PEPPER),now).run();
    response=json({ok:true,message:'Konto angelegt. Der Betreiber muss es freigeben. Es wird keine automatische E-Mail versendet.'},201);
   }else if(path==='/api/auth/login'){
    const mail=email(input.email),pw=password(input.password);
    await limit(env,'login:'+mail,now,10,900);
    const row=await env.DB.prepare('SELECT * FROM accounts WHERE email=?').bind(mail).first();
    const calculated=await passwordHash(pw,row?.salt||'AAAAAAAAAAAAAAAAAAAAAA',env.AUTH_PEPPER);
    if(!row||!equal(calculated,row.password_hash))fail('E-Mail oder Passwort stimmt nicht.',401);
    const session=random();await env.DB.prepare('INSERT INTO sessions(hash,user_id,expires) VALUES(?,?,?)').bind(await hash(session),row.id,now+30*DAYS).run();
    response=json({user:publicUser(row)},200,{'Set-Cookie':cookie(session)});
   }else if(path==='/api/auth/logout'){
    const raw=request.headers.get('Cookie')?.split(';').map(v=>v.trim()).find(v=>v.startsWith(SESSION+'='))?.slice(SESSION.length+1);
    if(raw)await env.DB.prepare('DELETE FROM sessions WHERE hash=?').bind(await hash(raw)).run();
    response=json({ok:true},200,{'Set-Cookie':cookie('',0)});
   }else if(path==='/api/auth/reset'){
    const reset=text(input.token,100),pw=password(input.password),salt=random(16);
    const changed=await env.DB.prepare('UPDATE accounts SET salt=?,password_hash=?,reset_hash=NULL,reset_expires=NULL WHERE reset_hash=? AND reset_expires>? RETURNING id').bind(salt,await passwordHash(pw,salt,env.AUTH_PEPPER),await hash(reset),now).first();
    if(!changed)fail('Reset-Link ist ungültig oder abgelaufen.',400);
    await env.DB.prepare('DELETE FROM sessions WHERE user_id=?').bind(changed.id).run();response=json({ok:true});
   }else fail('Nicht gefunden.',404);
  }else if(path==='/api/session'&&request.method==='GET'){
   const u=await user(request,env,now,false);response=json({user:publicUser(u)});
  }else if(path.startsWith('/api/')){
   const u=await user(request,env,now);await limit(env,'account:'+u.id,now,60,60);
   if(path==='/api/devices'&&request.method==='GET'){
    const rows=await env.DB.prepare('SELECT id,address,platform,label FROM devices WHERE owner_id=? ORDER BY created_at DESC LIMIT 50').bind(u.id).all();response=json({devices:rows.results});
   }else if(path==='/api/pair'&&request.method==='POST'){
    await limit(env,'pair:'+u.id,now,12,900);const input=await body(request),addr=mac(input.address);
    if(!addr)fail('Bitte die am Gerät angezeigte MAC-/Geräteadresse eingeben.');
    const code=text(input.code,20).toUpperCase().replace(/[ -]/g,'');
    const matches=await env.DB.prepare('SELECT * FROM devices WHERE (address=? OR mac_hash=?) AND owner_id IS NULL AND pair_expires>? LIMIT 5').bind(address(addr.slice(2).toLowerCase()),await keyed('mac:'+addr,env.AUTH_PEPPER),now).all();
    let found=null;for(const d of matches.results)if(equal(d.pair_hash||'',await hash(d.id+':'+code)))found=d;
    if(!found)fail('Adresse oder Kopplungscode ist ungültig, abgelaufen oder bereits gekoppelt.',400);
    const total=await env.DB.prepare('SELECT COUNT(*) AS n FROM devices WHERE owner_id=?').bind(u.id).first();if(total.n>=50)fail('Gerätelimit erreicht.',409);
    const claimed=await env.DB.prepare('UPDATE devices SET owner_id=?,label=?,pair_hash=NULL,pair_expires=NULL WHERE id=? AND owner_id IS NULL AND pair_expires>? RETURNING id').bind(u.id,text(input.label||found.platform,80),found.id,now).first();
    if(!claimed)fail('Kopplung bitte neu starten.',409);response=json({ok:true});
   }else if(path==='/api/unpair'&&request.method==='POST'){
    const input=await body(request);const d=await env.DB.prepare('SELECT id FROM devices WHERE id=? AND owner_id=?').bind(String(input.deviceId),u.id).first();if(!d)fail('Gerät nicht gefunden.',404);
    await env.DB.batch([env.DB.prepare('DELETE FROM messages WHERE device_id=? AND state=?').bind(d.id,'queued'),env.DB.prepare('UPDATE devices SET owner_id=NULL,pair_hash=NULL,pair_expires=NULL WHERE id=? AND owner_id=?').bind(d.id,u.id)]);response=json({ok:true});
   }else if(path==='/api/messages'&&request.method==='POST'){
    const input=await body(request),id=text(input.id,80),deviceId=text(input.deviceId,80);if(!/^[a-f0-9-]{36}$/.test(id))fail('Ungültige Sende-ID.');
    const d=await env.DB.prepare('SELECT id FROM devices WHERE id=? AND owner_id=?').bind(deviceId,u.id).first();if(!d)fail('Gerät nicht gefunden.',404);
    const old=await env.DB.prepare('SELECT id FROM messages WHERE id=? AND sender_id=? AND device_id=?').bind(id,u.id,d.id).first();if(old)return json({ok:true,id});
    const count=await env.DB.prepare('SELECT COUNT(*) AS n FROM messages WHERE device_id=? AND state=? AND expires>?').bind(d.id,'queued',now).first();if(count.n>=50)fail('Am Gerät bitte zuerst Aktualisieren drücken.',409);
    const p=profile(input.profile),cipher=await encrypt(p,env.CRYPTO_KEY,id+':'+d.id+':'+u.id);
    const sent=await env.DB.prepare('INSERT INTO messages(id,sender_id,device_id,name,kind,cipher,created_at,expires) SELECT ?,?,?,?,?,?,?,? WHERE EXISTS(SELECT 1 FROM devices JOIN accounts ON accounts.id=devices.owner_id WHERE devices.id=? AND devices.owner_id=? AND accounts.status=?) AND (SELECT COUNT(*) FROM messages WHERE device_id=? AND state=? AND expires>?)<50 RETURNING id').bind(id,u.id,d.id,p.name,p.kind,cipher,now,now+7*DAYS,d.id,u.id,'approved',d.id,'queued',now).first();
    if(!sent)fail('Gerätezuordnung geändert oder Postfach voll. Bitte erneut prüfen.',409);
    response=json({ok:true,id},201);
   }else if(path==='/api/messages'&&request.method==='GET'){
    const rows=await env.DB.prepare('SELECT id,device_id,name,kind,state,created_at,expires,received_at FROM messages WHERE sender_id=? ORDER BY created_at DESC LIMIT 100').bind(u.id).all();response=json({messages:rows.results.map(m=>({...m,state:m.state==='queued'&&m.expires<=now?'expired':m.state}))});
   }else fail('Nicht gefunden.',404);
  }else if(isDevice&&request.method==='POST'){
   await limit(env,'device-ip:'+ip,now,60,60);
   const input=await body(request),secret=token(request),h=await hash(secret);
   if(path==='/v1/device/register'){
    const platform=text(input.platform,30);if(!['windows','android','android-tv','ios','samsung','lg'].includes(platform))fail('Unbekannte Plattform.');
    const id='D7C-'+h.slice(0,32),m=mac(input.mac);
    const existingDevice=await env.DB.prepare('SELECT id FROM devices WHERE id=?').bind(id).first();
    if(!existingDevice)await limit(env,'new-device:'+ip,now,20,3600);
    await env.DB.prepare('INSERT INTO devices(id,token_hash,address,mac_hash,platform,created_at) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO NOTHING').bind(id,h,address(h),m?await keyed('mac:'+m,env.AUTH_PEPPER):null,platform,now).run();
    response=json(await deviceState(env,await device(request,env)));
   }else{
    const d=await device(request,env);await limit(env,'device:'+d.id,now,30,60);
    if(path==='/v1/device/pairing'){
     if(d.owner_id&&!input.reset)fail('Gerät ist bereits gekoppelt. Zum Wechseln zuerst Verbindung trennen.',409);
     const code=random(8).toUpperCase().replace(/[^A-Z0-9]/g,'').slice(0,10);if(code.length<8)fail('Codeerzeugung bitte erneut versuchen.',503);
     await env.DB.batch([env.DB.prepare('DELETE FROM messages WHERE device_id=? AND state=?').bind(d.id,'queued'),env.DB.prepare('UPDATE devices SET owner_id=NULL,pair_hash=?,pair_expires=? WHERE id=?').bind(await hash(d.id+':'+code),now+600,d.id)]);
     response=json({...await deviceState(env,{...d,owner_id:null}),pairingCode:code,pairingExpires:now+600});
    }else if(path==='/v1/device/pull'){
     const state=await deviceState(env,d);let messages=[];
     if(state.ownerStatus==='approved'){
      const rows=await env.DB.prepare('SELECT * FROM messages WHERE device_id=? AND sender_id=? AND state=? AND expires>? ORDER BY created_at LIMIT 50').bind(d.id,d.owner_id,'queued',now).all();
      messages=await Promise.all(rows.results.map(async m=>({id:m.id,profile:await decrypt(m.cipher,env.CRYPTO_KEY,m.id+':'+d.id+':'+m.sender_id)})));
     }response=json({...state,messages});
    }else if(path==='/v1/device/ack'){
     if(!Array.isArray(input.ids)||input.ids.length>50||input.ids.some(id=>typeof id!=='string'||!/^[a-f0-9-]{36}$/.test(id)))fail('Ungültige Empfangsbestätigung.');
     if(input.ids.length)await env.DB.prepare('UPDATE messages SET state=?,cipher=NULL,received_at=? WHERE id IN ('+input.ids.map(()=>'?').join(',')+') AND device_id=? AND sender_id=? AND state=?').bind('delivered',now,...input.ids,d.id,d.owner_id||'','queued').run();response=json({ok:true});
    }else fail('Nicht gefunden.',404);
   }
  }else fail('Nicht gefunden.',404);
 }catch(error){response=safeError(error);}
 if(isDevice){response=new Response(response.body,response);response.headers.set('Access-Control-Allow-Origin','*');}
 return response;
};}
export async function cleanup(env,now=Math.floor(Date.now()/1000)){
 await env.DB.batch([env.DB.prepare('DELETE FROM sessions WHERE expires<?').bind(now),env.DB.prepare('DELETE FROM rate_limits WHERE expires<?').bind(now),env.DB.prepare('UPDATE messages SET state=?,cipher=NULL WHERE state=? AND expires<=?').bind('expired','queued',now),env.DB.prepare('DELETE FROM messages WHERE created_at<?').bind(now-30*DAYS),env.DB.prepare('DELETE FROM audit WHERE at<?').bind(now-90*DAYS)]);
}
export default {fetch:createHandler(),scheduled:(_event,env,ctx)=>ctx.waitUntil(cleanup(env))};
