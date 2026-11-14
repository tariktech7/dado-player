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
  const a = current.getBoundingClientRect(),
    ax = a.x + a.width / 2,
    ay = a.y + a.height / 2;
  const horizontal = direction === "ArrowLeft" || direction === "ArrowRight",
    sign = direction === "ArrowLeft" || direction === "ArrowUp" ? -1 : 1;
  const candidates = all
    .filter((el) => el !== current)
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
