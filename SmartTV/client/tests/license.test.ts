import {test} from 'node:test';
import assert from 'node:assert/strict';
import {LicenseClient} from '../src/licensing';
import {fingerprint,signLicense,TRIAL_SECONDS,type Claims} from '../../../licensing/shared/protocol.mjs';

async function fixture(t:any,kind:'trial'|'permanent',saved=true){
 const now=Math.floor(Date.now()/1000), subject=await fingerprint('samsung','TV-test-only');
 const pair=await crypto.subtle.generateKey({name:'ECDSA',namedCurve:'P-256'},true,['sign','verify']);
 const pub=await crypto.subtle.exportKey('jwk',pair.publicKey),secret=await crypto.subtle.exportKey('jwk',pair.privateKey);
 const claims:Claims={v:1,product:'dado-player',platform:'samsung',subject,kind,issued_at:now,started_at:now,expires_at:kind==='trial'?now+TRIAL_SECONDS:null,license_id:'test-license'};
 const token=await signLicense(claims,secret,'test');
 const values=new Map<string,string>();if(saved)values.set('dado.license.v1',JSON.stringify({token,high:now,lastCheck:now}));
 const prior={window:globalThis.window,document:globalThis.document,localStorage:globalThis.localStorage,fetch:globalThis.fetch};
 const win=new EventTarget() as any;win.webapis={productinfo:{getDuid:()=> 'TV-test-only'},network:{getMac:()=> 'aa:bb:cc:dd:ee:10'},avplay:{}};
 Object.assign(globalThis,{window:win,document:new EventTarget(),localStorage:{getItem:(key:string)=>values.get(key)||null,setItem:(key:string,value:string)=>values.set(key,value)}});
 let calls=0;const paths:string[]=[];
 globalThis.fetch=async(input)=>{calls++;paths.push(new URL(String(input)).pathname);return new Response(JSON.stringify({license:token,server_time:now}),{status:200});};
 const client=new LicenseClient({enabled:true,apiUrl:'https://licenses.example',keys:{test:pub}});
 t.after(()=>{client.dispose();Object.assign(globalThis,prior);});
 return {client,values,claims,now,calls:()=>calls,win,paths};
}
test('saved permanent license starts Unlimited with zero network requests',async t=>{
 const f=await fixture(t,'permanent');await f.client.init();
 assert.equal(f.calls(),0);assert.equal(f.client.state.text,'Unlimited');assert.equal(f.client.state.allowed,true);
 assert.equal(f.client.state.mac,'AA:BB:CC:DD:EE:10');
 f.win.dispatchEvent(new Event('focus'));assert.equal(f.calls(),0);
});
test('trial registers once, persists signed token and reuses fresh cache without another call',async t=>{
 const f=await fixture(t,'trial',false);await f.client.init();assert.equal(f.calls(),0);await f.client.startTrial();
 assert.equal(f.calls(),1);assert.equal(f.client.state.allowed,true);assert.match(f.client.state.text,/7 Tage/);
 assert.ok(JSON.parse(f.values.get('dado.license.v1')!).token);
 assert.deepEqual(f.paths,['/v1/trial/start']);
 f.win.dispatchEvent(new Event('focus'));assert.equal(f.calls(),1);
});
test('expiry after resume blocks playback locally without a heartbeat',async t=>{
 const f=await fixture(t,'trial');await f.client.init();
 const original=Date.now;t.after(()=>{Date.now=original;});Date.now=()=> (f.now+TRIAL_SECONDS)*1000;
 f.win.dispatchEvent(new Event('focus'));
 assert.equal(f.client.state.allowed,false);assert.equal(f.client.state.text,'Testzeit abgelaufen');assert.equal(f.calls(),0);
});
test('a failed refresh preserves a previously verified permanent license',async t=>{
 const f=await fixture(t,'permanent');await f.client.init();globalThis.fetch=async()=>{throw Error('offline');};
 await f.client.refresh();assert.equal(f.client.state.allowed,true);assert.equal(f.client.state.text,'Unlimited');assert.match(f.client.state.error,/offline/);
});

test('activation rejects even a correctly signed trial returned by a misconfigured server',async t=>{
 const f=await fixture(t,'trial',false);await f.client.init();await f.client.refresh();
 assert.deepEqual(f.paths,['/v1/license/activation']);
 assert.equal(f.client.state.allowed,false);assert.equal(f.client.state.claims,null);
 assert.equal(f.values.has('dado.license.v1'),false);assert.match(f.client.state.error,/Noch keine Freischaltung/);
 await f.client.startTrial();assert.equal(f.client.state.allowed,true);
});

test('background refresh only reads existing trial status and never starts one',async t=>{
 const f=await fixture(t,'trial');
 const saved=JSON.parse(f.values.get('dado.license.v1')!);saved.lastCheck=f.now-21600;
 f.values.set('dado.license.v1',JSON.stringify(saved));
 await f.client.init();assert.deepEqual(f.paths,['/v1/license']);assert.equal(f.client.state.allowed,true);
});
