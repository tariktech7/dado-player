import {test} from 'node:test';
import assert from 'node:assert/strict';
import {DatabaseSync} from 'node:sqlite';
import {readFileSync} from 'node:fs';
import {createHandler} from '../worker.mjs';
import {fingerprint,macAlias,verifyLicense,TRIAL_SECONDS,statusText,deviceCode} from '../shared/protocol.mjs';

function database() {
 const db=new DatabaseSync(':memory:');
 db.exec(readFileSync(new URL('../migrations/0001_licenses.sql',import.meta.url),'utf8'));
 const prepare=(sql,values=[])=>({bind:(...v)=>prepare(sql,v),first:async()=>db.prepare(sql).get(...values)||null,all:async()=>({results:db.prepare(sql).all(...values)}),run:async()=>db.prepare(sql).run(...values),sql,values});
 return {prepare, batch:async statements=>{db.exec('BEGIN');try{const r=statements.map(s=>db.prepare(s.sql).run(...s.values));db.exec('COMMIT');return r;}catch(e){db.exec('ROLLBACK');throw e;}}};
}
async function setup() {
 const pair=await crypto.subtle.generateKey({name:'ECDSA',namedCurve:'P-256'},true,['sign','verify']);
 const privateKey=await crypto.subtle.exportKey('jwk',pair.privateKey),publicKey=await crypto.subtle.exportKey('jwk',pair.publicKey);
 const env={DB:database(),SIGNING_JWK:JSON.stringify(privateKey),KEY_ID:'test'};
 let now=1800000000;
 const handler=createHandler({clock:()=>now,authenticate:async r=>r.headers.get('X-Test-Admin')==='yes'?'owner@example.invalid':null});
 const post=(path,data,admin=false)=>handler(new Request('https://licenses.example'+path,{method:'POST',headers:{'Content-Type':'application/json','Origin':'https://licenses.example',...(admin?{'X-Test-Admin':'yes'}:{})},body:JSON.stringify(data)}),env);
 return {env,post,keys:{test:publicKey},setTime:t=>now=t,getTime:()=>now};
}
test('server owns trial start; reinstall cannot renew; admin activation creates non-expiring device license',async()=>{
 const s=await setup(),subject=await fingerprint('samsung','fixture-duid');
 const input={subject,platform:'samsung',mac_hash:await macAlias('AA-BB-CC-DD-EE-10'),started_at:9999999999,paid:true};
 let r=await s.post('/v1/license',input);assert.equal(r.status,200);
 let c=await verifyLicense((await r.json()).license,s.keys,subject,'samsung',s.getTime());
 assert.equal(c.kind,'trial');assert.equal(c.started_at,s.getTime());assert.equal(c.expires_at,c.started_at+TRIAL_SECONDS);
 const started=c.started_at;s.setTime(started+TRIAL_SECONDS);
 c=await verifyLicense((await (await s.post('/v1/license',input)).json()).license,s.keys,subject,'samsung',s.getTime());
 assert.equal(c.started_at,started);assert.equal(c.expired,true);assert.equal(statusText(c,s.getTime()),'Testzeit abgelaufen');
 assert.equal((await s.post('/admin/approve',{subject})).status,403);
 const lookup=await (await s.post('/admin/lookup',{query:'aa:bb:cc:dd:ee:10'},true)).json();
 assert.equal(lookup.devices[0].code,deviceCode(subject));
 assert.equal((await s.post('/admin/approve',{subject,reference:'manual fixture'},true)).status,200);
 const token=(await (await s.post('/v1/license',input)).json()).license;
 c=await verifyLicense(token,s.keys,subject,'samsung',s.getTime()+20*365*86400);
 assert.equal(c.kind,'permanent');assert.equal(c.expires_at,null);assert.equal(statusText(c,s.getTime()),'Unlimited');
 await assert.rejects(()=>verifyLicense(token,s.keys,awaitableSubject,'samsung',s.getTime()));
 const parts=token.split('.');parts[1]=parts[1].slice(0,-1)+(parts[1].endsWith('A')?'B':'A');
 await assert.rejects(()=>verifyLicense(parts.join('.'),s.keys,subject,'samsung',s.getTime()));
});
const awaitableSubject='a'.repeat(64);
test('unknown admin, CSRF, malformed devices and flooding fail closed',async()=>{
 const s=await setup();
 assert.equal((await s.post('/admin/lookup',{query:'anything'})).status,403);
 const subject=await fingerprint('windows','fixture-machine');
 assert.equal((await s.post('/v1/license',{subject:'invalid',platform:'windows'})).status,400);
 const r=await createHandler({authenticate:async()=> 'owner'})(new Request('https://licenses.example/admin/approve',{method:'POST',body:JSON.stringify({subject})}),s.env);
 assert.equal(r.status,403);
 for(let i=0;i<30;i++) assert.equal((await s.post('/v1/license',{subject,platform:'windows'})).status,200);
 assert.equal((await s.post('/v1/license',{subject,platform:'windows'})).status,429);
});
test('trial boundary and wrong platform/key do not unlock',async()=>{
 const s=await setup(),subject=await fingerprint('samsung','clock');
 const token=(await (await s.post('/v1/license',{subject,platform:'samsung'})).json()).license;
 let c=await verifyLicense(token,s.keys,subject,'samsung',s.getTime()+TRIAL_SECONDS-1);assert.equal(c.expired,false);
 c=await verifyLicense(token,s.keys,subject,'samsung',s.getTime()+TRIAL_SECONDS);assert.equal(c.expired,true);
 await assert.rejects(()=>verifyLicense(token,s.keys,subject,'windows',s.getTime()));
 await assert.rejects(()=>verifyLicense(token,{},subject,'samsung',s.getTime()));
});
test('public API and private admin worker remain separated',async()=>{
 const s=await setup();s.env.ROLE='api';
 assert.equal((await s.post('/admin/lookup',{query:'anything'},true)).status,403);
 s.env.ROLE='admin';
 assert.equal((await s.post('/v1/license',{subject:'a'.repeat(64),platform:'samsung'})).status,404);
 assert.equal((await s.post('/admin/lookup',{query:'anything'},true)).status,200);
});
test('platform Access identity is checked against owner; arbitrary email headers do not authorize',async()=>{
 const s=await setup();s.env.ROLE='admin';s.env.ADMIN_EMAIL='owner@example.invalid';
 const handler=createHandler();const request=new Request('https://admin.example/admin');
 assert.equal((await handler(request,s.env,{access:{getIdentity:async()=>({email:'owner@example.invalid'})}})).status,200);
 assert.equal((await handler(request,s.env,{access:{getIdentity:async()=>({email:'someone@example.invalid'})}})).status,403);
 assert.equal((await handler(new Request(request,{headers:{'Cf-Access-Authenticated-User-Email':'owner@example.invalid'}}),s.env)).status,403);
});
