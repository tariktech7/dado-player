export const samsung = () =>
  typeof window !== "undefined" && !!(window as any).webapis?.avplay;
export function registerKeys(): void {
  const input = (window as any).tizen?.tvinputdevice;
  for (const key of [
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
export function resetFocusMemory() {
  zoneMemory.clear();
}
export function rememberFocus(target: HTMLElement) {
  const zone = target.closest<HTMLElement>("[data-focus-zone]");
  if (!zone) return;
  const scope =
    zone.closest<HTMLElement>("[data-focus-scope]")?.dataset.focusScope || "";
  zoneMemory.set(scope + ":" + zone.dataset.focusZone, {
    key: target.dataset.focusKey || target.textContent || "",
    index: focusables(zone).indexOf(target),
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
export function moveFocus(
  direction: string,
  root: ParentNode = document,
): void {
  const all = focusables(root),
    current = document.activeElement as HTMLElement;
  if (!all.includes(current)) {
    all[0]?.focus();
    return;
  }
  rememberFocus(current);
  const a = current.getBoundingClientRect(),
    ax = a.x + a.width / 2,
    ay = a.y + a.height / 2;
  const horizontal = direction === "ArrowLeft" || direction === "ArrowRight",
    sign = direction === "ArrowLeft" || direction === "ArrowUp" ? -1 : 1;
  const rail = horizontal
    ? current.closest<HTMLElement>(".card-rail,.season-buttons")
    : null;
  if (rail) {
    const controls = focusables(rail),
      index = controls.indexOf(current),
      next = controls[index + sign];
    if (next) {
      next.focus({ preventScroll: true });
      scrollFocused(next);
      return;
    }
  }
  const zone = current.closest<HTMLElement>("[data-focus-zone]");
  if (horizontal && zone && !current.closest(".modal,.fullscreen")) {
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
  const candidates = all
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
  }
}
export function scrollFocused(target: HTMLElement): void {
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
      if (focusables(parent)[0] === target) parent.scrollLeft = 0;
      else if (rect.left < box.left + pad)
        parent.scrollLeft -= (box.left + pad - rect.left) / scale;
      else if (rect.right > box.right - pad)
        parent.scrollLeft += (rect.right - box.right + pad) / scale;
    }
    if (
      !/(auto|scroll)/.test(style.overflowY) ||
      parent.scrollHeight <= parent.clientHeight
    )
      continue;
    const first = focusables(parent)[0],
      virtual = parent.classList.contains("channel-list");
    if (
      (virtual &&
        target.closest("[data-list-index]")?.getAttribute("data-list-index") ===
          "0") ||
      (!virtual && first === target)
    )
      parent.scrollTop = 0;
    {
      const box = parent.getBoundingClientRect(),
        rect = target.getBoundingClientRect(),
        scale = innerWidth / 1920,
        pad = 8 * scale;
      if (rect.top < box.top + pad)
        parent.scrollTop -= (box.top + pad - rect.top) / scale;
      else if (rect.bottom > box.bottom - pad)
        parent.scrollTop += (rect.bottom - box.bottom + pad) / scale;
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
