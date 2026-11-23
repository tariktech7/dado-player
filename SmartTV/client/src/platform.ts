import { moveScroll, scrollDestination } from './scroll-motion';
export const samsung = () =>
  typeof window !== "undefined" && !!(window as any).webapis?.avplay;
let infoKeyCode: number | undefined;
export const isInfoKey = (event: KeyboardEvent) => event.key === "Info" || event.keyCode === infoKeyCode;
export function registerKeys(): void {
  const input = (window as any).tizen?.tvinputdevice;
  for (const key of [
    "ColorF0Red",
    "ColorF1Green",
    "ColorF2Yellow",
    "ColorF3Blue",
    "MediaPlayPause",
    "MediaPlay",
    "MediaPause",
    "MediaStop",
    "MediaRewind",
    "MediaFastForward",
    "ChannelUp",
    "ChannelDown",
    "Info",
    "0",
  ]) {
    try {
      input?.registerKey(key);
      if (key === "Info") infoKeyCode = input?.getKey?.(key)?.code;
    } catch {
      /* Optional keys vary by remote. */
    }
  }
}
export function screensaver(enabled: boolean): void {
  try {
    (window as any).webapis?.appcommon?.setScreenSaver(
      enabled ? "SCREEN_SAVER_ON" : "SCREEN_SAVER_OFF",
      () => {},
      () => {},
    );
  } catch {
    /* Browser preview has no Samsung APIs. */
  }
}
export function exitApp(): void {
  try {
    (window as any).tizen.application.getCurrentApplication().exit();
  } catch {
    /* Desktop preview stays open. */
  }
}
const selector =
  'button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex="0"]';
export function focusables(root: ParentNode = document): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(selector)].filter((el) => {
    const r = el.getBoundingClientRect();
    return (
      r.width > 0 &&
      r.height > 0 &&
      getComputedStyle(el).visibility !== "hidden" &&
      !el.closest('[hidden],[aria-hidden="true"]')
    );
  });
}
const zoneMemory = new Map<string, { key: string; index: number }>();
const homeRowMemory = new WeakMap<Element, HTMLElement>();
export function resetFocusMemory() {
  zoneMemory.clear();
}
export function rememberFocus(target: HTMLElement) {
  const homeRow = target.closest(".home-primary > section");
  if (homeRow) homeRowMemory.set(homeRow, target);
  const zone = target.closest<HTMLElement>("[data-focus-zone]");
  if (!zone) return;
  const scope =
    zone.closest<HTMLElement>("[data-focus-scope]")?.dataset.focusScope || "";
  zoneMemory.set(scope + ":" + zone.dataset.focusZone, {
    key: target.dataset.focusKey || target.textContent || "",
    // Semantic keys already identify virtual rows. Scanning/measuring every
    // category here on every focus event made long provider lists expensive.
    index: target.dataset.focusKey ? 0 : focusables(zone).indexOf(target),
  });
}
function zoneTarget(zone: HTMLElement) {
  const scope =
    zone.closest<HTMLElement>("[data-focus-scope]")?.dataset.focusScope || "";
  const memory = zoneMemory.get(scope + ":" + zone.dataset.focusZone),
    controls = focusables(zone);
  return (
    (memory &&
      controls.find(
        (e) => (e.dataset.focusKey || e.textContent || "") === memory.key,
      )) ||
    controls.find(
      (e) =>
        e.classList.contains("selected") ||
        e.getAttribute("aria-selected") === "true",
    ) ||
    controls[memory?.index || 0] ||
    controls[0]
  );
}
export function focusNavigation(): void {
  const zone = document.querySelector<HTMLElement>(".navigation[data-focus-zone]");
  const target = zone && zoneTarget(zone);
  if (target) { target.focus({preventScroll: true}); scrollFocused(target); }
}
export function moveFocus(
  direction: string,
  root: ParentNode = document,
): void {
  const current = document.activeElement as HTMLElement;
  if (!current || !root.contains(current) || !current.matches(selector)) {
    focusables(root)[0]?.focus();
    return;
  }
  const horizontal = direction === "ArrowLeft" || direction === "ArrowRight",
    sign = direction === "ArrowLeft" || direction === "ArrowUp" ? -1 : 1;
  const rail = horizontal
    ? current.closest<HTMLElement>(".card-rail,.season-buttons,.kind-tabs")
    : null;
  if (rail) {
    let next: HTMLElement | undefined;
    if (rail.matches(".home-primary .card-rail")) {
      const card = current.parentElement;
      const adjacent = sign < 0 ? card?.previousElementSibling : card?.nextElementSibling;
      next = adjacent?.querySelector<HTMLElement>(selector) ?? undefined;
    } else {
      const controls = focusables(rail);
      next = controls[controls.indexOf(current) + sign];
    }
    if (next) {
      next.focus({ preventScroll: true });
      scrollFocused(next);
      return;
    }
  }
  const grid = current.closest<HTMLElement>("[data-focus-grid]");
  if (grid) {
    const cols = Number(grid.dataset.focusGrid),
      index = Number(current.dataset.gridIndex),
      next = index + sign * (horizontal ? 1 : cols);
    if (!horizontal || Math.floor(next / cols) === Math.floor(index / cols)) {
      const target = grid.querySelector<HTMLElement>(
        `[data-grid-index="${next}"]`,
      );
      if (target) {
        target.focus({ preventScroll: true });
        scrollFocused(target);
        return;
      }
    }
    // At the first/last row stay in the grid. Never use animated screen
    // coordinates to guess the next row while the current row is moving.
    if(!horizontal)return;
  }
  const zone = current.closest<HTMLElement>("[data-focus-zone]");
  const homeRow = !horizontal && current.closest(".home-primary > section");
  if (homeRow) {
    for (let row = sign < 0 ? homeRow.previousElementSibling : homeRow.nextElementSibling;
      row; row = sign < 0 ? row.previousElementSibling : row.nextElementSibling) {
      const remembered = homeRowMemory.get(row);
      const target = remembered && row.contains(remembered)
        ? remembered
        : row.querySelector<HTMLElement>(selector);
      if (target) {
        target.focus({preventScroll: true});
        scrollFocused(target);
        break;
      }
    }
    return;
  }
  if(!horizontal && current.parentElement?.matches('.category-list,.episode-list,.channel-list,.home-favorites .favorite-programs')) {
    const sibling=sign<0?current.previousElementSibling:current.nextElementSibling;
    if(sibling instanceof HTMLElement && sibling.matches(selector)) {
      sibling.focus({preventScroll:true});scrollFocused(sibling);return;
    }
  }
  const settingsTiles = current.closest<HTMLElement>(".settings-sections");
  if (horizontal && zone && !settingsTiles && !current.closest(".modal,.fullscreen")) {
    const scope = zone.closest<HTMLElement>("[data-focus-scope]")!;
    const zones = [
      ...scope.querySelectorAll<HTMLElement>("[data-focus-zone]"),
    ].filter((z) => focusables(z).length > 0);
    const next = zones[zones.indexOf(zone) + sign],
      target = next && zoneTarget(next);
    if (target) {
      target.focus({ preventScroll: true });
      rememberFocus(target);
      scrollFocused(target);
      return;
    }
  }
  const a = current.getBoundingClientRect(),ax=a.x+a.width/2,ay=a.y+a.height/2;
  const candidates = focusables(settingsTiles || (!horizontal && zone ? zone : root))
    .filter(
      (el) => el !== current && (horizontal || !zone || zone.contains(el)),
    )
    .map((el) => {
      const b = el.getBoundingClientRect(),
        bx = b.x + b.width / 2,
        by = b.y + b.height / 2;
      const primary = ((horizontal ? bx : by) - (horizontal ? ax : ay)) * sign;
      const cross = Math.abs((horizontal ? by : bx) - (horizontal ? ay : ax));
      return { el, primary, score: primary + cross * 3 };
    })
    .filter((c) => c.primary > 2)
    .sort((a, b) => a.score - b.score);
  const target = candidates[0]?.el;
  if (target) {
    target.focus({ preventScroll: true });
    rememberFocus(target);
    scrollFocused(target);
  } else if (settingsTiles && horizontal && sign < 0) {
    focusNavigation();
  }
}
let pendingFocus:HTMLElement|null=null, focusFrame=0;
export function scrollFocused(target: HTMLElement): void {
  pendingFocus=target;
  if(focusFrame)return;
  focusFrame=requestAnimationFrame(()=>{
    focusFrame=0;const next=pendingFocus;pendingFocus=null;
    if(next?.isConnected && document.activeElement===next)revealFocused(next);
  });
}
function revealFocused(target: HTMLElement): void {
  // Native scrollIntoView also scrolls hidden canvas/document ancestors and
  // destroys the TV safe-area placement. Only scroll our actual content panes.
  for (
    let parent = target.parentElement;
    parent && parent !== document.body;
    parent = parent.parentElement
  ) {
    if (parent.id === "app") break;
    const style = getComputedStyle(parent);
    if (
      /(auto|scroll)/.test(style.overflowX) &&
      parent.scrollWidth > parent.clientWidth
    ) {
      const box = parent.getBoundingClientRect(),
        rect = target.getBoundingClientRect(),
        scale = innerWidth / 1920,
        pad = 8 * scale;
      const dest = scrollDestination(parent), shift = (dest.left-parent.scrollLeft)*scale;
      let left = dest.left;
      if (parent.querySelector(selector) === target) left = 0;
      else if (rect.left-shift < box.left + pad) left -= (box.left+pad-rect.left+shift)/scale;
      else if (rect.right-shift > box.right-pad) left += (rect.right-shift-box.right+pad)/scale;
      moveScroll(parent,left,dest.top);
    }
    if (
      !/(auto|scroll)/.test(style.overflowY) ||
      parent.scrollHeight <= parent.clientHeight
    )
      continue;
    const first = parent.querySelector(selector),
      virtual = parent.classList.contains("channel-list") || parent.classList.contains("catalog-title-list");
    let top = scrollDestination(parent).top;
    const atStart = (
      (virtual &&
        (target.closest("[data-list-index]")?.getAttribute("data-list-index") ?? target.dataset.gridIndex) ===
          "0") ||
      (!virtual && first === target)
    );
    if (atStart) top = 0;
    {
      const box = parent.getBoundingClientRect(),
        rect = target.getBoundingClientRect(),
        scale = innerWidth / 1920,
        pad = 8 * scale;
      const shift = (top-parent.scrollTop)*scale;
      if (!atStart && rect.top-shift < box.top+pad) top -= (box.top+pad-rect.top+shift)/scale;
      else if (!atStart && rect.bottom-shift > box.bottom-pad) top += (rect.bottom-shift-box.bottom+pad)/scale;
      moveScroll(parent,scrollDestination(parent).left,top);
    }
  }
  const root = document.getElementById("app");
  if (root) {
    root.scrollTop = 0;
    root.scrollLeft = 0;
  }
  document.documentElement.scrollTop = 0;
  document.body.scrollTop = 0;
}
export function resizeCanvas(): void {
  // Samsung's IME reduces viewport height. Keep TV typography tied to width,
  // and scroll the form instead of shrinking the whole application.
  const scale = innerWidth / 1920,
    root = document.getElementById("app");
  if (root) {
    const visibleHeight = Math.min(
      innerHeight,
      window.visualViewport?.height || innerHeight,
    );
    const keyboard = visibleHeight < 1080 * scale * 0.8;
    root.classList.toggle("keyboard-open", keyboard);
    root.style.setProperty("--visible-height", `${visibleHeight / scale}px`);
    root.style.transform = `scale(${scale})`;
    root.style.left = `${(innerWidth - 1920 * scale) / 2}px`;
    root.style.top = `${Math.max(0, (innerHeight - 1080 * scale) / 2)}px`;
    const active = document.activeElement as HTMLElement | null;
    if (active && root.contains(active))
      requestAnimationFrame(() => scrollFocused(active));
  }
}
