import {test} from 'node:test';
import assert from 'node:assert/strict';
import {ParentalControls,emptyPolicy,lockKey,withPin,verifyPin} from '../src/parental';
import type {Item} from '../src/types';
const item=(kind:Item['kind'],id='1',seriesId?:string):Item=>({id,kind,seriesId,name:'Test',category:'',logo:'',url:'https://example.invalid/video'});
function storage(){const data=new Map<string,string>();return {getItem:(k:string)=>data.get(k)||null,setItem:(k:string,v:string)=>{data.set(k,v);}};}
test('PIN validation, salted hashing, persistence and series/profile isolation',async()=>{
  const s=storage(),c=new ParentalControls(s),series=item('series','8');
  await assert.rejects(()=>withPin(emptyPolicy(),'123'));await assert.rejects(()=>withPin(emptyPolicy(),'１２３４'));
  const p=await withPin(emptyPolicy(),'0123');assert(await verifyPin(p,'0123'));assert(!await verifyPin(p,'1234'));
  assert.notDeepEqual((await withPin(emptyPolicy(),'0123')).hash,p.hash);
  c.persist({...p,blocked:[lockKey('a',series)]});
  const restored=new ParentalControls(s);assert(restored.locked('a',item('episode','20','8')));assert(!restored.locked('b',item('episode','20','8')));assert(!restored.locked('a',item('vod','8')));
  assert(!s.getItem('dado.parental.v1')!.includes('0123'));
});
test('cancel denies playback; successful PIN grants only the pending request, never future playback',async()=>{
  const c=new ParentalControls(storage()),i=item('live');c.persist({...await withPin(emptyPolicy(),'1234'),blocked:[lockKey('a',i)]});
  let allowed=c.request('a',i);c.close();assert.equal(await allowed,false);
  allowed=c.request('a',i);await c.authenticate('1234');assert.equal(await allowed,true);
  assert(c.locked('a',i));allowed=c.request('a',i);c.close();assert.equal(await allowed,false);
});
test('failed attempts survive cancel and throttle further PIN checks',async()=>{
  const c=new ParentalControls(storage());c.persist(await withPin(emptyPolicy(),'1234'));
  for(let n=0;n<5;n++){c.manage();await assert.rejects(()=>c.authenticate('0000'));c.close();}
  c.manage();await assert.rejects(()=>c.authenticate('1234'),/30 Sekunden/);
});
test('cancelled asynchronous authentication cannot authorize a newer request',async()=>{
  const c=new ParentalControls(storage()),i=item('vod');c.persist({...await withPin(emptyPolicy(),'1234'),blocked:[lockKey('a',i)]});
  const first=c.request('a',i),check=c.authenticate('1234');c.close();const next=c.request('a',i);
  await assert.rejects(()=>check,/Abgebrochen/);assert.equal(await first,false);assert(c.open);c.close();assert.equal(await next,false);
});
test('storage failures keep previous protection; corrupted protection fails closed',async()=>{
  const c=new ParentalControls({getItem:()=>null,setItem:()=>{throw Error('full');}});
  assert.throws(()=>c.persist({...emptyPolicy(),hash:[1]}));assert(!c.enabled);
  const bad=new ParentalControls({getItem:()=>'{',setItem:()=>{}});assert(bad.locked('a',item('live')));
});
