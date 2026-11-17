import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {random} from '../protocol.mjs';
const base='https://dado-connect.dado-licensing.workers.dev',root=fileURLToPath(new URL('../',import.meta.url)),wrangler=fileURLToPath(new URL('../../licensing/node_modules/wrangler/bin/wrangler.js',import.meta.url));
const email='smoke-'+crypto.randomUUID()+'@example.invalid',password=random(),secret=random();let userId,deviceId;
async function call(path,data,extra={}){const r=await fetch(base+path,{method:data===undefined?'GET':'POST',headers:{Origin:base,'Content-Type':'application/json',...extra},...(data===undefined?{}:{body:JSON.stringify(data)})});const value=await r.json();return {status:r.status,value,cookie:r.headers.get('set-cookie')?.split(';')[0]};}
function sql(command){const r=spawnSync(process.execPath,[wrangler,'d1','execute','dado-connect','--remote','--command',command,'--json'],{cwd:root,encoding:'utf8'});if(r.status!==0)throw Error('Controlled smoke DB operation failed');return JSON.parse(r.stdout)[0].results;}
try{
 assert.equal((await call('/health')).status,200);console.log('Public health OK');
 const signup=await call('/api/auth/register',{email,name:'Disposable deployment smoke',password});assert.equal(signup.status,201,signup.value.error);console.log('Registration OK');
 const login=await call('/api/auth/login',{email,password});assert.equal(login.status,200,login.value.error);userId=login.value.user.id;const cookie={Cookie:login.cookie};
 assert.equal((await call('/api/devices',undefined,cookie)).status,403);console.log('Pending account blocked OK');
 sql(`UPDATE accounts SET status='approved',approved_at=unixepoch() WHERE id='${userId}' AND email='${email}'`);
 const auth={Authorization:'Bearer '+secret};const register=await call('/v1/device/register',{platform:'samsung',mac:'AA:BB:CC:DD:EE:91'},auth);assert.equal(register.status,200,register.value.error);deviceId=register.value.deviceId;
 const pair=await call('/v1/device/pairing',{},auth);assert.equal(pair.status,200,pair.value.error);
 assert.equal((await call('/api/pair',{address:register.value.address,code:pair.value.pairingCode,label:'Disposable QA TV'},cookie)).status,200);console.log('Device pairing OK');
 const id=crypto.randomUUID(),profile={name:'Disposable QA Profile',kind:'Xtream',host:'https://provider.invalid',username:'qa',password:'smoke-not-real-provider-secret'};
 const sent=await call('/api/messages',{id,deviceId,profile},cookie);assert.equal(sent.status,201,sent.value.error);
 const db=sql(`SELECT cipher FROM messages WHERE id='${id}'`)[0];assert.ok(db.cipher&&!db.cipher.includes(profile.password));console.log('Encrypted queue OK');
 const pull=await call('/v1/device/pull',{},auth);assert.equal(pull.status,200,pull.value.error);assert.equal(pull.value.messages[0].profile.password,profile.password);
 assert.equal((await call('/v1/device/ack',{ids:[id]},auth)).status,200);assert.equal(sql(`SELECT cipher FROM messages WHERE id='${id}'`)[0].cipher,null);console.log('Pull + acknowledgement + credential deletion OK');
 assert.equal((await call('/v1/device/pull',{},auth)).value.messages.length,0);
 const admin=await fetch('https://dado-admin.dado-licensing.workers.dev/admin/connect',{redirect:'manual'});assert.ok([302,303,403].includes(admin.status));console.log('Protected admin does not admit anonymous users OK');
 console.log('Production smoke passed. No real provider or customer data used.');
}finally{
 if(deviceId)sql(`DELETE FROM devices WHERE id='${deviceId}'`);
 sql(`DELETE FROM accounts WHERE email='${email}'`);
 console.log('Disposable smoke account and device removed.');
}
