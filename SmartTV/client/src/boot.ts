import { readSlot, saveSlot, verifyBundle, type Boot, type Bundle } from './update-format';

declare const INSTALLED: Boot['installed'];

function addScript(src: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const node = document.createElement('script');
    node.onload = () => resolve();
    node.onerror = () => reject(new Error('Script failed to load'));
    node.src = src;
    document.head.appendChild(node);
  });
}

async function boot(): Promise<void> {
  if ('tizen' in window) await addScript('$WEBAPIS/webapis/webapis.js').catch(() => {});
  const state = { installed: INSTALLED, running: INSTALLED, fromUpdate: false, ready: () => {} };
  window.dadoTvBoot = state;
  const slot = readSlot();
  let bundle: Bundle | undefined;
  if (slot.bundle) {
    try {
      const candidate = await verifyBundle(slot.bundle);
      if (candidate.build > INSTALLED.build && candidate.variant === INSTALLED.variant &&
          candidate.compatibility === INSTALLED.compatibility && candidate.build > (slot.blocked || 0)) {
        if (slot.attempting === candidate.build) saveSlot({ blocked: candidate.build });
        else {
          saveSlot({ ...slot, attempting: candidate.build });
          bundle = candidate;
        }
      }
    } catch { /* The installed files are the startup fallback. */ }
  }
  if (!bundle) {
    await addScript('app.js');
    return;
  }

  const running = bundle;
  state.running = running;
  state.fromUpdate = true;
  let started = false;
  const failed = () => {
    if (started) return;
    started = true;
    try { saveSlot({ blocked: running.build }); } catch { /* The persisted attempt still causes fallback. */ }
    location.reload();
  };
  const timeout = setTimeout(failed, 15_000);
  window.addEventListener('error', failed);
  state.ready = () => {
    if (started) return;
    started = true;
    clearTimeout(timeout);
    window.removeEventListener('error', failed);
    try { saveSlot({ bundle: slot.bundle, blocked: slot.blocked }); }
    catch { /* An interrupted storage write returns to the installed package on reboot. */ }
  };
  const css = document.createElement('style');
  css.textContent = running.css;
  document.head.appendChild(css);
  document.querySelector<HTMLLinkElement>('#packaged-css')!.disabled = true;
  const url = URL.createObjectURL(new Blob([running.script], { type: 'text/javascript' }));
  try { await addScript(url); } catch { failed(); } finally { URL.revokeObjectURL(url); }
}

void boot();
