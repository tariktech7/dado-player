import {PRODUCT, TRIAL_SECONDS, deviceCode, macAlias, signLicense} from './shared/protocol.mjs';
import {adminPage} from './admin.mjs';

const json = (value, status=200) => new Response(JSON.stringify(value), {status,headers:{'Content-Type':'application/json','Cache-Control':'no-store','Access-Control-Allow-Origin':'*','X-Content-Type-Options':'nosniff'}});
async function body(request) {
  if (Number(request.headers.get('content-length') || 0) > 4096) throw new RangeError('Anfrage zu groß');
  const reader=request.body?.getReader();if(!reader)throw new SyntaxError('Leere Anfrage');
  const chunks=[];let size=0;
  while(true){const {value,done}=await reader.read();if(done)break;size+=value.length;if(size>4096){await reader.cancel();throw new RangeError('Anfrage zu groß');}chunks.push(value);}
  const bytes=new Uint8Array(size);let offset=0;for(const c of chunks){bytes.set(c,offset);offset+=c.length;}
  return JSON.parse(new TextDecoder().decode(bytes));
}
async function adminIdentity(request, env, context) {
  const expected=String(env.ADMIN_EMAIL||'').trim().toLowerCase();
  if(!expected)return null;
  if(context?.access){
    const identity=await context.access.getIdentity();
    return typeof identity?.email==='string' && identity.email.toLowerCase()===expected ? identity.email : null;
  }
  // Access is mandatory in production; do not trust its email header alone.
  const jwt = request.headers.get('Cf-Access-Jwt-Assertion');
  if (!env.ACCESS_AUD || !env.ACCESS_TEAM || !env.ADMIN_EMAIL || !jwt) return null;
  const {jwtVerify, createRemoteJWKSet} = await import('jose');
  const issuer = `https://${env.ACCESS_TEAM}.cloudflareaccess.com`;
  const {payload} = await jwtVerify(jwt, createRemoteJWKSet(new URL(`${issuer}/cdn-cgi/access/certs`)), {issuer,audience:env.ACCESS_AUD,algorithms:['RS256']});
  return typeof payload.email==='string' && payload.email.toLowerCase()===expected ? payload.email : null;
}
export function createHandler({clock=()=>Math.floor(Date.now()/1000), authenticate=adminIdentity}={}) {
 return async function fetch(request, env, context) {
  const url = new URL(request.url);
  if (request.method === 'OPTIONS') return new Response(null,{headers:{'Access-Control-Allow-Origin':'*','Access-Control-Allow-Methods':'POST, GET, OPTIONS','Access-Control-Allow-Headers':'Content-Type'}});
  try {
   if (url.pathname.startsWith('/admin')) {
    if(env.ROLE==='api')return json({error:'Verwaltung nur über den privaten Verwaltungsdienst erreichbar'},403);
    const actor = await authenticate(request, env, context);
    if (!actor) return json({error:'Keine Admin-Berechtigung'},403);
    if (url.pathname === '/admin' && request.method==='GET') return new Response(adminPage, {headers:{'Content-Type':'text/html; charset=utf-8','Cache-Control':'no-store','Content-Security-Policy':"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",'X-Frame-Options':'DENY'}});
    if (request.method==='POST' && request.headers.get('Origin') !== url.origin) return json({error:'Ungültiger Ursprung'},403);
    if (url.pathname==='/admin/lookup' && request.method==='POST') {
     const input=await body(request), q=String(input.query || '').trim().toUpperCase();
     let rows;
     const mac=await macAlias(q);
     if(mac) rows=await env.DB.prepare('SELECT devices.* FROM devices JOIN aliases ON aliases.subject=devices.subject WHERE aliases.mac_hash=?').bind(mac).all();
     else rows=await env.DB.prepare('SELECT * FROM devices WHERE code=? OR subject=?').bind(q,q.toLowerCase()).all();
     return json({devices:rows.results});
    }
    if(url.pathname==='/admin/approve' && request.method==='POST') {
     const input=await body(request), subject=String(input.subject || ''), reference=String(input.reference || '').trim();
     if(!/^[a-f0-9]{64}$/.test(subject) || reference.length>160) return json({error:'Ungültige Eingabe'},400);
     const row=await env.DB.prepare('SELECT * FROM devices WHERE subject=?').bind(subject).first();
     if(!row) return json({error:'Gerät ist noch nicht registriert'},404);
     await env.DB.batch([
      env.DB.prepare('UPDATE devices SET approved_at=COALESCE(approved_at,?), reference=? WHERE subject=?').bind(clock(),reference,subject),
      env.DB.prepare('INSERT INTO audit(at,actor,subject,action,reference) VALUES(?,?,?,?,?)').bind(clock(),actor,subject,'approve',reference)
     ]);
     return json({approved:true,code:row.code});
    }
    return json({error:'Nicht gefunden'},404);
   }
   if(env.ROLE==='admin'){
    if(url.pathname==='/' && request.method==='GET')return Response.redirect(url.origin+'/admin',302);
    return json({error:'Nicht gefunden'},404);
   }
   if(url.pathname==='/health' && request.method==='GET') return json({ok:true,product:PRODUCT});
   if(url.pathname!=='/v1/license' || request.method!=='POST') return json({error:'Nicht gefunden'},404);
   if(!env.SIGNING_JWK || !env.KEY_ID) return json({error:'Lizenzdienst wird eingerichtet'},503);
   const input=await body(request);
   if(!/^[a-f0-9]{64}$/.test(input.subject || '') || !['windows','samsung','lg','android-tv'].includes(input.platform) || (input.mac_hash != null && !/^[a-f0-9]{64}$/.test(input.mac_hash))) return json({error:'Ungültige Gerätekennung'},400);
   // Persistent per-IP minute budget; no in-memory-only limiter on distributed isolates.
   const now=clock(), minute=Math.floor(now/60), ip=request.headers.get('CF-Connecting-IP') || 'local';
   const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(ip));
   const bucket=minute+':'+[...new Uint8Array(digest)].map(b=>b.toString(16).padStart(2,'0')).join('');
   const limit=await env.DB.prepare('INSERT INTO rate_limits(bucket,count,expires_at) VALUES(?,1,?) ON CONFLICT(bucket) DO UPDATE SET count=count+1 RETURNING count').bind(bucket,now+120).first();
   if(limit.count>30) return json({error:'Bitte später erneut versuchen'},429);
   await env.DB.prepare('DELETE FROM rate_limits WHERE expires_at < ?').bind(now).run();
   const code=deviceCode(input.subject);
   await env.DB.prepare('INSERT INTO devices(subject,code,platform,started_at,license_id) VALUES(?,?,?,?,?) ON CONFLICT(subject) DO NOTHING').bind(input.subject,code,input.platform,now,crypto.randomUUID()).run();
   const row=await env.DB.prepare('SELECT * FROM devices WHERE subject=?').bind(input.subject).first();
   if(row.platform!==input.platform) return json({error:'Geräteplattform stimmt nicht überein'},400);
   if(input.mac_hash != null) {
    if(!/^[a-f0-9]{64}$/.test(input.mac_hash)) return json({error:'Ungültige MAC-Kennung'},400);
    await env.DB.prepare('INSERT INTO aliases(mac_hash,subject) VALUES(?,?) ON CONFLICT DO NOTHING').bind(input.mac_hash,input.subject).run();
   }
   const claims={v:1,product:PRODUCT,subject:row.subject,platform:row.platform,license_id:row.license_id,kind:row.approved_at==null?'trial':'permanent',issued_at:now,started_at:row.started_at,expires_at:row.approved_at==null?row.started_at+TRIAL_SECONDS:null};
   return json({license:await signLicense(claims,JSON.parse(env.SIGNING_JWK),env.KEY_ID),server_time:now});
  } catch(error) {
   // Never return DB/key/account details or echo identifiers to an unauthenticated caller.
   return json({error: error instanceof SyntaxError || error instanceof RangeError ? 'Ungültige Anfrage' : 'Lizenzdienst vorübergehend nicht verfügbar'},error instanceof SyntaxError || error instanceof RangeError?400:503);
  }
 };
}
export default {fetch:createHandler()};
