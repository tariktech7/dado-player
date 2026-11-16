// One clock per moving pane. Retarget the running motion instead of restarting
// Chromium's slow-start smooth-scroll animation on every remote repeat.
type Plan = {left: number; top: number; frame: number; at: number; x: number; y: number; fx: number; fy: number};
const plans = new WeakMap<HTMLElement, Plan>();
export function scrollDestination(el: HTMLElement) {
  const plan = plans.get(el);
  return plan || {left:el.scrollLeft, top:el.scrollTop};
}
export function moveScroll(el: HTMLElement, left: number, top: number) {
  left = Math.max(0, Math.min(left, el.scrollWidth - el.clientWidth));
  top = Math.max(0, Math.min(top, el.scrollHeight - el.clientHeight));
  const old = plans.get(el);
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
    cancelScroll(el);el.scrollTo({left,top,behavior:'auto'});return;
  }
  if(old){old.left=left;old.top=top;return;}
  if(Math.abs(el.scrollLeft-left)<0.5 && Math.abs(el.scrollTop-top)<0.5)return;
  const plan:Plan={left,top,frame:0,at:performance.now(),x:el.scrollLeft,y:el.scrollTop,fx:el.scrollLeft,fy:el.scrollTop};
  plans.set(el,plan);
  const tick=(now:number)=>{
    if(!el.isConnected){cancelScroll(el);return;}
    // Wheel/touch/native restoration owns the pane as soon as it interrupts us.
    if(Math.abs(el.scrollLeft-plan.x)>1.5 || Math.abs(el.scrollTop-plan.y)>1.5){cancelScroll(el);return;}
    const dt=Math.max(0,now-plan.at);plan.at=now;
    const fraction=1-Math.exp(-dt/32);
    // Keep subpixels ourselves: integer scroll offsets on older TV engines
    // otherwise round the last pixel away forever and never finish the clock.
    const x=plan.fx+(plan.left-plan.fx)*fraction;
    const y=plan.fy+(plan.top-plan.fy)*fraction;
    plan.fx=x;plan.fy=y;
    const done=Math.abs(plan.left-x)<0.5 && Math.abs(plan.top-y)<0.5;
    el.scrollTo({left:done?plan.left:x,top:done?plan.top:y,behavior:'auto'});
    plan.x=el.scrollLeft;plan.y=el.scrollTop;
    if(done){plans.delete(el);return;}
    plan.frame=requestAnimationFrame(tick);
  };
  plan.frame=requestAnimationFrame(tick);
}
export function cancelScroll(el:HTMLElement) {
  const old=plans.get(el);if(old)cancelAnimationFrame(old.frame);
  plans.delete(el);
}
