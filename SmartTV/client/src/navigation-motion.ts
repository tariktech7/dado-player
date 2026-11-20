// Change the grid once, then animate the visual offset on the compositor.
import appearance from './shell-appearance';
// Animating grid-template-columns relayouts every channel/poster on every frame.
const running = new WeakMap<HTMLElement, Animation[]>();
export async function animateNavigation(shell: HTMLElement | null, collapsed: boolean): Promise<void> {
  if (!shell || shell.classList.contains("nav-collapsed") === collapsed) return;
  const nodes = [...shell.children].flatMap((child) => {
    if (child.classList.contains("navigation"))
      return [...child.querySelectorAll<HTMLElement>(".brand img, nav button > span:first-child")];
    const catalogue = child.querySelector(".catalog-browser");
    return catalogue ? [...catalogue.children] as HTMLElement[] : [child as HTMLElement];
  });
  // Read the current *visual* positions before cancelling, so reversing with
  // the remote midway through an animation continues without snapping.
  const before = nodes.map((node) => node.getBoundingClientRect());
  for (const animation of running.get(shell) || []) animation.cancel();
  shell.classList.toggle("nav-collapsed", collapsed);
  const after = nodes.map((node) => node.getBoundingClientRect());
  const scale = shell.getBoundingClientRect().width / shell.offsetWidth || 1;
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const animations: Animation[] = [];
  nodes.forEach((node, index) => {
    const x = (before[index].left - after[index].left) / scale;
    const y = (before[index].top - after[index].top) / scale;
    if ((Math.abs(x) > 0.5 || Math.abs(y) > 0.5) && node.animate) {
      const from: Keyframe = {transform: `translate3d(${x}px, ${y}px, 0)`};
      const to: Keyframe = {transform: "translate3d(0, 0, 0)"};
      if (x > 0 && node.classList.contains("channels")) {
        // The grid gives this column its final width before the animation.
        // Clip the translated extra width at the fixed preview boundary.
        from.clipPath = `inset(-54px ${x - 10}px -54px -10px)`;
        to.clipPath = "inset(-54px -10px -54px -10px)";
      }
      animations.push(node.animate([from, to], {
        duration: appearance.navigationDurationMs,
        easing: `cubic-bezier(${appearance.navigationEasing.join(',')})`,
      }));
    }
  });
  running.set(shell, animations);
  await Promise.all(animations.map((animation) => animation.finished.catch(() => {})));
}
