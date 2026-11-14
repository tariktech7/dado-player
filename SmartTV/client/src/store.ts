import { Saved, Settings, Item, Watched, itemKey } from "./types";
export const defaults: Settings = {
  theme: "dark",
  autoplayLast: false,
  liveFormat: "m3u8",
  player: "avplay",
  aspect: "fit",
  bufferSeconds: 20,
  timeoutSeconds: 20,
  userAgent: "",
  subtitleSize: 32,
  networkMode: "direct",
  companionUrl: "",
  companionToken: "",
  density: "compact",
  audioLanguage: "",
  subtitleLanguage: "",
  subtitles: false,
};
export const fresh = (): Saved => ({
  version: 1,
  profiles: [],
  activeProfile: null,
  signedOut: false,
  settings: { ...defaults },
  favorites: {},
  history: {},
});
const KEY = "dado.tv.v1";
export function load(storage: Pick<Storage, "getItem"> = localStorage): Saved {
  try {
    const s = JSON.parse(storage.getItem(KEY) || "null");
    if (s?.version !== 1 || !Array.isArray(s.profiles)) return fresh();
    return {
      ...fresh(),
      ...s,
      settings: { ...defaults, ...s.settings },
      favorites: s.favorites || {},
      history: s.history || {},
    };
  } catch {
    return fresh();
  }
}
export function save(
  state: Saved,
  storage: Pick<Storage, "setItem"> = localStorage,
): void {
  try {
    storage.setItem(KEY, JSON.stringify(state));
  } catch {
    throw new Error(
      "Speicher voll oder nicht verfügbar. Änderungen konnten nicht gespeichert werden.",
    );
  }
}
export function remember(
  state: Saved,
  profile: string,
  item: Item,
  position: number,
  duration: number,
): Watched[] {
  const rows = state.history[profile] || [];
  return [
    {
      item,
      position: item.kind === "live" ? 0 : position,
      duration,
      at: Date.now(),
    },
    ...rows.filter((r) => itemKey(r.item) !== itemKey(item)),
  ].slice(0, 150);
}
export function favorite(state: Saved, profile: string, item: Item): Item[] {
  const rows = state.favorites[profile] || [],
    key = itemKey(item);
  return rows.some((i) => itemKey(i) === key)
    ? rows.filter((i) => itemKey(i) !== key)
    : [...rows, item];
}
