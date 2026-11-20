import {test} from 'node:test';
import assert from 'node:assert/strict';
import {moveScroll,scrollDestination} from '../src/scroll-motion';

function clock(hz:number,integer=false,reduced=false) {
  let now=performance.now(),id=0;
  const frames=new Map<number,FrameRequestCallback>();
  Object.assign(globalThis,{
    requestAnimationFrame:(fn:FrameRequestCallback)=>{frames.set(++id,fn);return id;},
    cancelAnimationFrame:(n:number)=>frames.delete(n),
    matchMedia:()=>({matches:reduced}),
  });
  const pane={isConnected:true,scrollLeft:0,scrollTop:0,scrollWidth:4000,scrollHeight:20000,clientWidth:800,clientHeight:800,
    scrollTo({left,top}:{left:number;top:number}){this.scrollLeft=integer?Math.round(left):left;this.scrollTop=integer?Math.round(top):top;},
  };
  return {pane:pane as unknown as HTMLElement,frames,
    step(ms=1000/hz){now+=ms;const batch=[...frames.values()];frames.clear();batch.forEach(fn=>fn(now));},
  };
}

test('scroll follows elapsed time at 60/120/144 Hz and terminates even with integer offsets',()=>{
  for(const hz of [60,120,144]) {
    const c=clock(hz,true);moveScroll(c.pane,1500,2000);
    for(let i=0;i<Math.ceil(hz*.12);i++)c.step();
    assert.ok(c.pane.scrollTop>1900 && c.pane.scrollTop<2000);
    for(let i=0;i<hz;i++)c.step();
    assert.equal(c.pane.scrollTop,2000);assert.equal(c.frames.size,0);
    moveScroll(c.pane,0,0);for(let i=0;i<hz;i++)c.step();
    assert.equal(c.pane.scrollLeft,0);assert.equal(c.pane.scrollTop,0);assert.equal(c.frames.size,0);
  }
});
test('remote repeats retarget a single running clock, including reversal',()=>{
  const c=clock(120);moveScroll(c.pane,0,600);c.step();
  const before=c.pane.scrollTop;
  moveScroll(c.pane,0,1200);moveScroll(c.pane,0,1800);
  assert.equal(c.frames.size,1);assert.equal(scrollDestination(c.pane).top,1800);
  c.step();assert.ok(c.pane.scrollTop>before);
  moveScroll(c.pane,0,0);for(let i=0;i<120;i++)c.step();
  assert.equal(c.pane.scrollTop,0);assert.equal(c.frames.size,0);
});
test('home repeats accelerate continuously without a jump after a dropped frame',()=>{
  const c=clock(60,true);
  Object.defineProperty(c.pane,"firstElementChild",{
    value:{classList:{contains:(name:string)=>name==="home-dashboard"}},
  });
  moveScroll(c.pane,0,1000);
  c.step(100);
  assert.ok(c.pane.scrollTop>0 && c.pane.scrollTop<=89);
  const before=c.pane.scrollTop;
  moveScroll(c.pane,0,1500);
  moveScroll(c.pane,0,2000);
  assert.equal(c.frames.size,1);
  c.step();
  assert.ok(c.pane.scrollTop>before && c.pane.scrollTop-before<=44);
  moveScroll(c.pane,0,0);
  for(let i=0;i<120;i++)c.step();
  assert.equal(c.pane.scrollTop,0);
  assert.equal(c.frames.size,0);
});
test('direct user scrolling interrupts an old remote animation',()=>{
  const c=clock(60);moveScroll(c.pane,0,2000);c.step();
  c.pane.scrollTop=9000;c.step();
  assert.equal(c.pane.scrollTop,9000);assert.equal(c.frames.size,0);
  assert.equal(scrollDestination(c.pane).top,9000);
});
test('reduced motion scrolls immediately without starting an animation',()=>{
  const c=clock(60,false,true);moveScroll(c.pane,0,500);
  assert.equal(c.pane.scrollTop,500);assert.equal(c.frames.size,0);
});
