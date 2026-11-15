// Let Chromium's scrolling/compositor animate; do not render the catalogue per pixel.
const plans = new WeakMap<HTMLElement, {left: number; top: number; at: number}>();
export function scrollDestination(el: HTMLElement) {
  const plan = plans.get(el);
  return plan && performance.now() - plan.at < 900 ? plan : {left:el.scrollLeft, top:el.scrollTop};
}
export function moveScroll(el: HTMLElement, left: number, top: number) {
  left = Math.max(0, Math.min(left, el.scrollWidth - el.clientWidth));
  top = Math.max(0, Math.min(top, el.scrollHeight - el.clientHeight));
  const old = plans.get(el);
  if (old && performance.now()-old.at < 900 && Math.abs(old.left-left)<0.5 && Math.abs(old.top-top)<0.5) return;
  plans.set(el,{left,top,at:performance.now()});
  el.scrollTo({left,top,behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'auto':'smooth'});
}
