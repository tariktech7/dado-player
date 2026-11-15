export type Kind = "live" | "vod" | "series" | "episode";
export interface Profile {
  id: string;
  name: string;
  kind: "Xtream" | "M3u";
  host: string;
  username: string;
  password: string;
  m3uUrl: string;
  epgUrl: string;
}
export interface Category {
  id: string;
  name: string;
}
export interface Item {
  id: string;
  kind: Kind;
  name: string;
  category: string;
  logo: string;
  url: string;
  epgId?: string;
  description?: string;
  season?: number;
  episode?: number;
  seriesId?: string;
}
export interface Epg {
  title: string;
  description: string;
  start: number;
  end: number;
}
export interface Catalog {
  live: Item[];
  vod: Item[];
  series: Item[];
  categories: Record<"live" | "vod" | "series", Category[]>;
  issues: string[];
}
export interface Watched {
  item: Item;
  position: number;
  duration: number;
  at: number;
}
export interface Settings {
  theme: "dark" | "light" | "system";
  autoplayLast: boolean;
  liveFormat: "m3u8" | "ts";
  player: "auto" | "avplay" | "html";
  expertMode: boolean;
  aspect: "fit" | "fill" | "stretch";
  bufferSeconds: number;
  timeoutSeconds: number;
  userAgent: string;
  subtitleSize: number;
  networkMode: "direct" | "companion";
  companionUrl: string;
  companionToken: string;
  density: "compact" | "comfortable";
  audioLanguage: string;
  subtitleLanguage: string;
  subtitles: boolean;
}
export interface Saved {
  version: 1;
  profiles: Profile[];
  activeProfile: string | null;
  signedOut: boolean;
  settings: Settings;
  favorites: Record<string, Item[]>;
  history: Record<string, Watched[]>;
}
export interface Track {
  language?: string;
  type: "AUDIO" | "TEXT";
  index: number;
  label: string;
}
export interface PlayerStatus {
  state: "idle" | "loading" | "playing" | "paused" | "error" | "ended";
  time: number;
  duration: number;
  buffer: number;
  error: string;
  tracks: Track[];
}
export const emptyCatalog = (): Catalog => ({
  live: [],
  vod: [],
  series: [],
  categories: { live: [], vod: [], series: [] },
  issues: [],
});
export const itemKey = (item: Item) => `${item.kind}:${item.id}`;
