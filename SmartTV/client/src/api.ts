import {
  Profile,
  Settings,
  Item,
  Category,
  Catalog,
  Epg,
  emptyCatalog,
} from "./types";
import { parseXmltv } from "./xmltv";
// Match native Xtream extension normalization; preserve real MKV/AVI files.
export function streamExtension(value: unknown): string {
  const clean = String(value ?? "").trim().replace(/^\.+/, "").toLowerCase();
  return /^[a-z0-9]{1,8}$/.test(clean) ? clean : "mp4";
}
export function httpUrl(value: string): string {
  const u = new URL(value);
  if (!["http:", "https:"].includes(u.protocol))
    throw new Error("Nur HTTP- oder HTTPS-Adressen sind möglich.");
  return u.href;
}
export function hostUrl(value: string): string {
  return httpUrl(value.includes("://") ? value : `http://${value}`).replace(
    /\/$/,
    "",
  );
}
export function decode(value: unknown): string {
  const s = String(value ?? "");
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(s) || s.length % 4 !== 0) return s;
  try {
    const bytes = Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
    const out = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    return /[\x00-\x08\x0e-\x1f]/.test(out) ? s : out;
  } catch {
    return s;
  }
}
export class Provider {
  private xmltv: Promise<Map<string, Epg[]>> | null = null;
  constructor(
    public profile: Profile,
    public settings: Settings,
    private fetcher: typeof fetch = fetch,
  ) {}
  async request(url: string, text = false): Promise<any> {
    const fetcher = this.fetcher;
    const controller = new AbortController(),
      timer = setTimeout(
        () => controller.abort(),
        this.settings.timeoutSeconds * 1000,
      );
    try {
      let response: Response;
      if (this.settings.networkMode === "companion") {
        const base = httpUrl(this.settings.companionUrl).replace(/\/$/, "");
        response = await fetcher(base + "/fetch", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${this.settings.companionToken}`,
          },
          body: JSON.stringify({ url: httpUrl(url) }),
          signal: controller.signal,
        });
      } else
        response = await fetcher(httpUrl(url), { signal: controller.signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return text ? await response.text() : await response.json();
    } catch (e) {
      if ((e as Error).name === "AbortError")
        throw new Error("Zeitüberschreitung beim Anbieter.");
      throw new Error(
        "Anbieter nicht erreichbar oder Antwort ungültig. Server, Zugang und CORS prüfen; gegebenenfalls Companion-Modus verwenden.",
      );
    } finally {
      clearTimeout(timer);
    }
  }
  endpoint(action = ""): string {
    const p = this.profile,
      u = new URL(hostUrl(p.host) + "/player_api.php");
    u.searchParams.set("username", p.username);
    u.searchParams.set("password", p.password);
    if (action) u.searchParams.set("action", action);
    return u.href;
  }
  stream(kind: string, id: unknown, extension: string): string {
    const p = this.profile;
    return `${hostUrl(p.host)}/${kind}/${encodeURIComponent(p.username)}/${encodeURIComponent(p.password)}/${encodeURIComponent(String(id))}.${streamExtension(extension)}`;
  }
  async catalog(previous?: Catalog): Promise<Catalog> {
    if (this.profile.kind === "M3u")
      return parseM3u(await this.request(this.profile.m3uUrl, true));
    const auth = await this.request(this.endpoint());
    if (Number(auth?.user_info?.auth) !== 1)
      throw new Error("Anmeldung abgelehnt. Zugangsdaten prüfen.");
    const result = emptyCatalog();
    await Promise.all(
      (["live", "vod", "series"] as const).map(async (kind) => {
        await Promise.all([
          (async () => {
            try {
              const rows = await this.request(
                this.endpoint(`get_${kind}_categories`),
              );
              if (!Array.isArray(rows)) throw 0;
              result.categories[kind] = rows.map((c) => ({
                id: String(c.category_id),
                name: String(c.category_name || "Ohne Kategorie"),
              }));
            } catch {
              if (previous) result.categories[kind] = previous.categories[kind];
              result.issues.push(
                `${kind}: Kategorien konnten nicht geladen werden.`,
              );
            }
          })(),
          (async () => {
            try {
              const action =
                kind === "live"
                  ? "get_live_streams"
                  : kind === "vod"
                    ? "get_vod_streams"
                    : "get_series";
              const rows = await this.request(this.endpoint(action));
              if (!Array.isArray(rows)) throw 0;
              result[kind] = rows.filter((row) => {
                const id = kind === "series" ? row.series_id : row.stream_id;
                const type = String(row.stream_type || "").toLowerCase();
                return id != null && String(id) !== "0" &&
                  !(kind === "vod" && ["series", "live"].includes(type)) &&
                  !(kind === "series" && ["movie", "vod", "live"].includes(type));
              }).map((row) => ({
                id: String(kind === "series" ? row.series_id : row.stream_id),
                kind,
                name: String(row.name || "Ohne Titel"),
                category: String(row.category_id ?? ""),
                logo: safeImage(row.stream_icon || row.cover),
                url:
                  kind === "series"
                    ? ""
                    : this.stream(
                        kind === "live" ? "live" : "movie",
                        row.stream_id,
                        kind === "live"
                          ? this.settings.liveFormat
                          : row.container_extension || "mp4",
                      ),
                epgId: String(row.epg_channel_id || ""),
                description: String(row.plot || ""),
              }));
            } catch {
              if (previous) result[kind] = previous[kind];
              result.issues.push(
                `${kind}: Inhalte konnten nicht geladen werden.`,
              );
            }
          })(),
        ]);
      }),
    );
    return result;
  }
  async epg(item: Item): Promise<Epg[]> {
    if (this.profile.kind === "M3u") {
      if (!this.profile.epgUrl || !item.epgId) return [];
      if (!this.xmltv)
        this.xmltv = this.request(this.profile.epgUrl, true)
          .then(parseXmltv)
          .catch((error) => {
            this.xmltv = null;
            throw error;
          });
      return (await this.xmltv).get(item.epgId) || [];
    }
    const u = new URL(this.endpoint("get_short_epg"));
    u.searchParams.set("stream_id", item.id);
    u.searchParams.set("limit", "8");
    const data = await this.request(u.href);
    return (data.epg_listings || [])
      .map((e: any) => ({
        title: decode(e.title),
        description: decode(e.description),
        start: Number(e.start_timestamp) || Date.parse(e.start) / 1000,
        end: Number(e.stop_timestamp) || Date.parse(e.end) / 1000,
      }))
      .filter((e: Epg) => Number.isFinite(e.start) && Number.isFinite(e.end))
      .sort((a: Epg, b: Epg) => a.start - b.start);
  }
  async episodes(item: Item): Promise<Item[]> {
    const u = new URL(this.endpoint("get_series_info"));
    u.searchParams.set("series_id", item.id);
    const data = await this.request(u.href);
    return Object.values(data.episodes || {})
      .flatMap((rows: any) => (Array.isArray(rows) ? rows : []))
      .map((e: any) => ({
        id: String(e.id),
        kind: "episode" as const,
        name: `Staffel ${Number(e.season) || 1} · Folge ${Number(e.episode_num) || 1}`,
        season: Number(e.season) || 1,
        episode: Number(e.episode_num) || 1,
        seriesId: item.id,
        category: item.category,
        logo: safeImage(e.info?.movie_image) || item.logo,
        url: this.stream("series", e.id, e.container_extension || "mp4"),
        description: String(e.info?.plot || ""),
      }))
      .sort((a, b) => a.season! - b.season! || a.episode! - b.episode!);
  }
  async details(item: Item): Promise<string> {
    if (item.kind !== "vod" || this.profile.kind === "M3u")
      return item.description || "";
    const u = new URL(this.endpoint("get_vod_info"));
    u.searchParams.set("vod_id", item.id);
    const data = await this.request(u.href);
    return String(data.info?.plot || data.info?.description || "");
  }
}
export function safeImage(value: unknown): string {
  try {
    return value ? httpUrl(String(value)) : "";
  } catch {
    return "";
  }
}
export function parseM3u(input: string): Catalog {
  if (!input.trimStart().startsWith("#EXTM3U"))
    throw new Error("Keine gültige M3U-Wiedergabeliste.");
  const result = emptyCatalog(),
    cats = new Map<string, Category>();
  let meta = "",
    ordinal = 0;
  for (const raw of input.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith("#EXTINF:")) meta = line;
    else if (line && !line.startsWith("#")) {
      let url: string;
      try {
        url = httpUrl(line);
      } catch {
        continue;
      }
      const attr = (key: string) =>
        new RegExp(`${key}="([^"]*)"`).exec(meta)?.[1] || "";
      const group = attr("group-title") || "Alle Sender";
      cats.set(group, { id: group, name: group });
      // URL-derived identity remains stable if the playlist is reordered.
      let hash = 2166136261;
      for (const c of url) {
        hash ^= c.charCodeAt(0);
        hash = Math.imul(hash, 16777619);
      }
      ordinal++;
      result.live.push({
        id: `m3u-${hash >>> 0}`,
        kind: "live",
        name: meta.slice(meta.lastIndexOf(",") + 1) || `Sender ${ordinal}`,
        category: group,
        logo: safeImage(attr("tvg-logo")),
        epgId: attr("tvg-id"),
        url,
      });
      meta = "";
    }
  }
  result.categories.live = [...cats.values()];
  return result;
}
