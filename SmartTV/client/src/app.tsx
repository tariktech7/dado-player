import { animateNavigation } from "./navigation-motion";
import { render, ComponentChildren } from "preact";
import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "preact/hooks";
import {
  Catalog,
  Epg,
  Item,
  Profile,
  Saved,
  Settings,
  emptyCatalog,
  itemKey,
} from "./types";
import { load, save, favorite, remember } from "./store";
import { Provider, hostUrl, httpUrl } from "./api";
import { Player } from "./player";
import { TVSelect } from "./select";
import { TVInput } from "./input";
import { CatalogBrowser, EpisodePicker, resumeEpisode } from "./library";
import policy from '../../../shared/product-policy.json' with { type: 'json' };
import { Home } from "./home";
import {licenseClient} from './licensing';
import {ConnectPanel,mergeCloudProfiles,prepareCloudDevice} from './connect';
import packageInfo from "../package.json";
const APP_VERSION = packageInfo.version;
import {
  exitApp,
  rememberFocus,
  resetFocusMemory,
  focusables,
  moveFocus,
  focusNavigation,
  registerKeys,
  resizeCanvas,
  scrollFocused,
  samsung,
} from "./platform";
import "./style.css";
import "./keyboard.css";
import {ParentalControls} from './parental';
import {ParentalPanel} from './parental-ui';
import shellAppearance,{installShellAppearance} from './shell-appearance';
installShellAppearance();

const nav = [
  ["home", "⌂", "Startseite"],
  ["live", "▣", "Live TV"],
  ["vod", "▷", "Filme"],
  ["series", "▤", "Serien"],
  ["history", "↻", "Fortsetzen"],
  ["favorites", "☆", "Favoriten"],
  ["search", "⌕", "Suche"],
  ["settings", "⚙", "Einstellungen"],
] as const;
type View = (typeof nav)[number][0];
const EMPTY_ITEMS: Item[] = [];
const EMPTY_HISTORY: Saved["history"][string] = [];
const emptyProfile = (): Profile => ({
  id: `p-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
  name: "Mein Profil",
  kind: "Xtream",
  host: "",
  username: "",
  password: "",
  m3uUrl: "",
  epgUrl: "",
});
const time = (n: number) =>
  Number.isFinite(n)
    ? `${Math.floor(n / 3600) ? Math.floor(n / 3600) + ":" : ""}${String(Math.floor((n % 3600) / 60)).padStart(2, "0")}:${String(Math.floor(n % 60)).padStart(2, "0")}`
    : "00:00";
const clock = (n: number) =>
  new Date(n * 1000).toLocaleTimeString("de-DE", {
    hour: "2-digit",
    minute: "2-digit",
  });
const currentNext = (rows: Epg[]) => {
  const now = Date.now() / 1000;
  return [
    rows.find((e) => e.start <= now && now < e.end),
    rows.find((e) => e.start > now),
  ];
};
function Logo({ item, large = false }: { item: Item; large?: boolean }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [item.logo]);
  return (
    <span class={`logo ${large ? "large" : ""}`}>
      {item.logo && !failed ? (
        <img
          src={item.logo}
          alt=""
          loading="lazy"
          onError={() => setFailed(true)}
        />
      ) : (
        <span>{item.name.slice(0, 2).toUpperCase()}</span>
      )}
    </span>
  );
}
function Field({
  label,
  children,
}: {
  label: string;
  children: ComponentChildren;
}) {
  return (
    <label class="field">
      <span>{label}</span>
      {children}
    </label>
  );
}
function App() {
  const [license,setLicense]=useState(licenseClient.state);
  useEffect(()=>{void prepareCloudDevice().catch(()=>{});},[]);
  useEffect(()=>{const unsubscribe=licenseClient.subscribe(()=>setLicense(licenseClient.state));void licenseClient.init();return unsubscribe;},[]);
  useEffect(()=>{if(license.ready && !license.allowed){playerRef.current?.stop();setFull(false);}},[license.ready,license.allowed]);
  const [saved, setSaved] = useState<Saved>(load),
    savedRef = useRef(saved);
  savedRef.current = saved;
  const parental = useMemo(()=>new ParentalControls(),[]);
  const [,parentalRepaint]=useState(0);
  const [view, setView] = useState<View>("home"),
    [catalog, setCatalog] = useState<Catalog>(emptyCatalog),
    [busy, setBusy] = useState(false),
    [message, setMessage] = useState("");
  const [session, setSession] = useState<Profile | null>(null),
    [form, setForm] = useState<Profile>(emptyProfile),
    [editing, setEditing] = useState(false);
  const [favoriteKind, setFavoriteKind] = useState("live");
  const [historyKind, setHistoryKind] = useState("live");
  const navCollapsed = useRef(false);
  const setNavCollapsed = (collapsed: boolean) => {
    if (navCollapsed.current === collapsed) return;
    navCollapsed.current = collapsed;
    void animateNavigation(document.querySelector<HTMLElement>(".shell"), collapsed).then(() => {
      if (surface.current && previewRef.current)
        playerRef.current?.position(surface.current.getBoundingClientRect());
    });
  };
  const detailCache = useRef(
    new Map<string, { item: Item; episodes: Item[] }>(),
  );
  const detailRequest = useRef(0);
  const continueRequest = useRef(0);
  const [category, setCategory] = useState("all"),
    [query, setQuery] = useState(""),
    [preview, setPreview] = useState<Item | null>(null),
    [full, setFull] = useState(false),
    [detail, setDetail] = useState<Item | null>(null),
    [episodes, setEpisodes] = useState<Item[]>([]);
  const memories = useRef<
    Partial<
      Record<
        View,
        {
          category: string;
          query: string;
          detail: Item | null;
          episodes: Item[];
          scroll: number[];
        }
      >
    >
  >({});
  const previousLive = useRef<Item | null>(null);
  const [selectedTitle, setSelectedTitle] = useState<Item | null>(null);
  const [epg, setEpg] = useState<Record<string, Epg[]>>({}),
    [resume, setResume] = useState<{ item: Item; position: number } | null>(
      null,
    ),
    [exit, setExit] = useState(false),
    [tracks, setTracks] = useState(false),
    [controlsVisible, setControlsVisible] = useState(true),
    [update, setUpdate] = useState(""),
    [updateUrl, setUpdateUrl] = useState("");
  const [, repaint] = useState(0),
    playerRef = useRef<Player | null>(null),
    providerRef = useRef<Provider | null>(null),
    generation = useRef(0),
    epgPending = useRef(new Set<string>()),
    loadToken = useRef(0),
    previewRef = useRef<Item | null>(null),
    fullRef = useRef(false),
    resumeRef = useRef(resume),
    detailRef = useRef(detail),
    exitRef = useRef(exit),
    tracksRef = useRef(tracks),
    controlsRef = useRef(true),
    playerMenuNavigation = useRef(false),
    lastInteraction = useRef(Date.now()),
    keyHandler = useRef<(e: KeyboardEvent) => void>(() => {});
  previewRef.current = preview;
  fullRef.current = full;
  resumeRef.current = resume;
  detailRef.current = detail;
  exitRef.current = exit;
  tracksRef.current = tracks;
  controlsRef.current = controlsVisible;
  const showControls = () => {
    lastInteraction.current = Date.now();
    setControlsVisible(true);
  };
  const changed = (next: Saved) => {
    savedRef.current = next;
    setSaved(next);
    try {
      save(next);
    } catch (e) {
      setMessage((e as Error).message);
    }
  };
  const setting = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    changed({
      ...savedRef.current,
      settings: { ...savedRef.current.settings,
          ...(key === "expertMode" && !value ? {player: "auto" as const, bufferSeconds: 20, timeoutSeconds: 20, userAgent: ""} : {}),
          [key]: value },
    });
  };
  const pid = session?.id || "",
    favorites = saved.favorites[pid] || EMPTY_ITEMS,
    history = useMemo(
      () =>
        (saved.history[pid] || EMPTY_HISTORY).map((h) => {
          const source =
            h.item.kind === "episode"
              ? catalog.series.find((s) => s.id === h.item.seriesId)
              : catalog[h.item.kind === "vod" ? "vod" : "live"].find(
                  (i) => i.id === h.item.id,
                );
          return source
            ? { ...h, item: { ...h.item, logo: source.logo || h.item.logo } }
            : h;
        }),
      [saved.history, pid, catalog],
    );
  const favoriteTarget = (i: Item) =>
    i.kind === "episode"
      ? catalog.series.find((s) => s.id === i.seriesId) || i
      : i;
  const isFavorite = (i: Item) =>
    favorites.some((f) => itemKey(f) === itemKey(favoriteTarget(i)));
  const toggleFavorite = (i: Item) =>
    changed({
      ...savedRef.current,
      favorites: {
        ...savedRef.current.favorites,
        [pid]: favorite(savedRef.current, pid, favoriteTarget(i)),
      },
    });
  const record = () => {
    const p = playerRef.current,
      profile = savedRef.current.activeProfile;
    if (!p?.current || !profile) return;
    changed({
      ...savedRef.current,
      history: {
        ...savedRef.current.history,
        [profile]: remember(
          savedRef.current,
          profile,
          p.current,
          p.status.time,
          p.status.duration,
        ),
      },
    });
  };
  const stop = () => {
    loadToken.current++;
    record();
    playerRef.current?.stop();
    setPreview(null);
    setFull(false);
    setTracks(false);
  };
  const safe = (fn: () => void) => {
    try {
      fn();
    } catch (e) {
      setMessage((e as Error).message);
    }
  };
  async function login(profile: Profile, automatic = false) {
    const gen = ++generation.current;
    setBusy(true);
    setMessage("");
    stop();
    setCatalog(emptyCatalog());
    memories.current = {};
    resetFocusMemory();
    detailCache.current.clear();
    detailRequest.current++;
    setNavCollapsed(false);
    previousLive.current = null;
    setSelectedTitle(null);
    setEpg({});
    epgPending.current.clear();
    setDetail(null);
    setEpisodes([]);
    try {
      const p = { ...profile };
      if (p.kind === "Xtream") {
        p.host = hostUrl(p.host);
        if (!p.username || !p.password)
          throw new Error("Benutzername und Passwort fehlen.");
      } else {
        p.m3uUrl = httpUrl(p.m3uUrl);
        if (p.epgUrl) p.epgUrl = httpUrl(p.epgUrl);
      }
      const provider = new Provider(p, savedRef.current.settings);
      const lib = await provider.catalog();
      if (gen !== generation.current) return;
      providerRef.current = provider;
      setCatalog(lib);
      setSession(p);
      setEditing(false);
      setView("home");
      setCategory("all");
      setQuery("");
      changed({
        ...savedRef.current,
        profiles: [
          ...savedRef.current.profiles.filter((v) => v.id !== p.id),
          p,
        ],
        activeProfile: p.id,
        signedOut: false,
      });
      if (automatic && savedRef.current.settings.autoplayLast) {
        const last = savedRef.current.history[p.id]?.[0];
        if (last) {
          const fresh =
            last.item.kind === "live"
              ? lib.live.find((i) => i.id === last.item.id)
              : last.item;
          if (fresh) void start(fresh, true, last.position);
        }
      }
    } catch (e) {
      if (gen === generation.current) {
        setMessage((e as Error).message);
        setSession(null);
        setForm(profile);
        setEditing(!automatic);
      }
    } finally {
      if (gen === generation.current) setBusy(false);
    }
  }
  const getEpg = async (item: Item) => {
    if (item.kind !== "live") return;
    const key = item.id;
    if (
      epgPending.current.has(key) ||
      epg[key]?.some((e) => e.end > Date.now() / 1000) ||
      !providerRef.current
    )
      return;
    epgPending.current.add(key);
    const gen = generation.current;
    try {
      const rows = await providerRef.current.epg(item);
      if (gen === generation.current)
        setEpg((old) => ({ ...old, [key]: rows }));
    } catch {
      if (gen === generation.current) setEpg((old) => ({ ...old, [key]: [] }));
    } finally {
      if (gen === generation.current) epgPending.current.delete(key);
    }
  };
  async function start(item: Item, fullscreen = false, position = 0) {
    const profileId=savedRef.current.activeProfile || '', generationAtRequest=generation.current;
    if(parental.locked(profileId,item)) stop();
    if(!await parental.request(profileId,item) || generationAtRequest!==generation.current) return;
    if(!licenseClient.state.allowed){setView('settings');setMessage('Bitte Testversion starten oder Freischaltung abrufen.');return;}
    if (item.kind === "live")
      item = catalog.live.find((i) => i.id === item.id) || item;
    if (item.kind === "series") {
      void openDetail(item);
      return;
    }
    const p = playerRef.current;
    if (!p) return;
    playerMenuNavigation.current = false;
    if (
      previewRef.current &&
      itemKey(previewRef.current) === itemKey(item) &&
      p.current
    ) {
      setFull(true);
      return;
    }
    record();
    if (
      item.kind === "live" &&
      previewRef.current?.kind === "live" &&
      previewRef.current.id !== item.id
    )
      previousLive.current = previewRef.current;
    setPreview(item);
    previewRef.current = item;
    setFull(fullscreen);
    setDetail(null);
    setTracks(false);
    const token = ++loadToken.current;
    p.settings = { ...savedRef.current.settings };
    void getEpg(item);
    await p.open(item, position);
    if (token !== loadToken.current) return;
    if (p.status.state === "playing") record();
  }
  const playTitle = (item: Item) => {
    const h = history.find((h) => itemKey(h.item) === itemKey(item));
    if (
      item.kind !== "live" &&
      item.kind !== "series" &&
      h &&
      h.position > 10 &&
      (!h.duration || h.position < h.duration - 30)
    )
      setResume({ item, position: h.position });
    else void start(item, true);
  };
  async function continueWatching(item: Item, position: number) {
    const request = ++continueRequest.current, playing = loadToken.current;
    const watched = history.find(h => itemKey(h.item)===itemKey(item));
    if(watched && position > 0 && watched.duration > 0 && watched.duration-position <= policy.resumeEndMarginSeconds) {
      if(item.kind==='episode' && item.seriesId) {
        const series=catalog.series.find(s=>s.id===item.seriesId);
        if(series) {
          try {
            const gen=generation.current;
            const rows=detailCache.current.get(itemKey(series))?.episodes || await providerRef.current?.episodes(series) || [];
            if(gen!==generation.current || request!==continueRequest.current || playing!==loadToken.current)return;
            const target=resumeEpisode(rows,[{...watched,position}]);
            if(target){await start(target.item,true,target.position);return;}
          } catch {setMessage('Die nächste Folge konnte nicht geladen werden. Bitte erneut versuchen.');return;}
        }
      }
      position=0;
    }
    await start(item,true,position);
  }
  async function openDetail(item: Item) {
    const token = ++detailRequest.current,
      gen = generation.current,
      key = itemKey(item),
      cached = detailCache.current.get(key);
    detailRef.current = item;
    setDetail(cached?.item || item);
    setEpisodes(cached ? [...cached.episodes] : []);
    if (cached) return;
    try {
      const rows =
        item.kind === "series"
          ? (await providerRef.current?.episodes(item)) || []
          : [];
      const description =
        item.kind === "vod"
          ? (await providerRef.current?.details(item)) || item.description
          : item.description;
      if (gen !== generation.current) return;
      const value = { item: { ...item, description }, episodes: rows };
      if (detailCache.current.size >= 64)
        detailCache.current.delete(detailCache.current.keys().next().value!);
      detailCache.current.set(key, value);
      if (token === detailRequest.current) {
        setDetail(value.item);
        setEpisodes(rows);
      }
    } catch (e) {
      if (token === detailRequest.current) setMessage((e as Error).message);
    }
  }
  const go = (next: View) => {
    if (next === view) return;
    const scroll = [
      ...document.querySelectorAll<HTMLElement>(
        ".shell main, .shell .category-list, .shell .channel-list, .shell .catalog-title-list, .shell .catalog-detail-scroll",
      ),
    ].map((e) => e.scrollTop);
    memories.current[view] = {
      category,
      query,
      detail: selectedTitle || detail,
      episodes,
      scroll,
    };
    stop();
    detailRequest.current++;
    const memory = memories.current[next];
    setView(next);
    setCategory(memory?.category || "all");
    setQuery(memory?.query || "");
    setSelectedTitle(memory?.detail || null);
    setDetail(memory?.detail || null);
    setEpisodes(memory?.episodes || []);
  };
  const logout = () => {
    generation.current++;
    stop();
    providerRef.current = null;
    setSession(null);
    setCatalog(emptyCatalog());
    setEpg({});
    setEditing(false);
    changed({ ...savedRef.current, signedOut: true });
  };
  const importCloudProfiles = (profiles:Profile[]) => {
    const merged=mergeCloudProfiles(savedRef.current.profiles,profiles);
    const next={...savedRef.current,profiles:merged.profiles};
    save(next);
    savedRef.current=next;setSaved(next);
    return merged.added;
  };
  const back = () => {
    if(parental.open){parental.close();return;}
    if (tracksRef.current) {
      setTracks(false);
      return;
    }
    if (resumeRef.current) {
      setResume(null);
      return;
    }
    if (exitRef.current) {
      setExit(false);
      return;
    }
    if (detailRef.current && view !== "vod" && view !== "series") {
      setDetail(null);
      return;
    }
    if (fullRef.current && !document.querySelector(".modal")) {
      if(previewRef.current?.kind === 'live')setFull(false);else stop();
      return;
    }
    if (previewRef.current) {
      stop();
      return;
    }
    if (editing) {
      setEditing(false);
      return;
    }
    if (!document.activeElement?.closest(".navigation")) {
      focusNavigation();
      return;
    }
    setExit(true);
  };
  keyHandler.current = (e: KeyboardEvent) => {
    const code = e.keyCode;
    if (!licenseClient.state.allowed) {
      if (e.key.startsWith('Arrow')) {
        e.preventDefault();
        moveFocus(e.key, document.querySelector('[data-license-gate]') || document);
      }
      // Do not navigate the hidden catalogue or start playback behind the gate.
      if (code === 10009 || e.key === 'Escape' || e.key === 'BrowserBack') e.preventDefault();
      return;
    }
    if (code === 10009 || e.key === "Escape" || e.key === "BrowserBack") {
      e.preventDefault();
      back();
      return;
    }
    if (
      fullRef.current &&
      previewRef.current?.kind === "live" &&
      !tracksRef.current &&
      !document.querySelector(".modal") &&
      (e.key === "ArrowUp" ||
        e.key === "ArrowDown" ||
        e.key === "0" ||
        code === 48 ||
        code === 96)
    ) {
      e.preventDefault();
      showControls();
      if (e.key === "0" || code === 48 || code === 96) {
        if (previousLive.current) void start(previousLive.current, true);
      } else {
        const list = view === "live" ? filteredLive : catalog.live;
        const index = list.findIndex((i) => i.id === previewRef.current?.id);
        if (list.length)
          void start(
            list[
              (index + (e.key === "ArrowUp" ? 1 : -1) + list.length) %
                list.length
            ],
            true,
          );
      }
      return;
    }
    if (fullRef.current && previewRef.current?.kind !== "live" && !tracksRef.current && !document.querySelector(".modal")) {
      if (!controlsRef.current) playerMenuNavigation.current = false;
      if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        playerMenuNavigation.current = true;
      } else if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && !playerMenuNavigation.current) {
        e.preventDefault();
        safe(() => playerRef.current?.seek(e.key === "ArrowLeft" ? -10 : 10));
        showControls();
        return;
      }
    }
    if (fullRef.current) {
      const hidden = !controlsRef.current;
      showControls();
      if (hidden && (e.key === "Enter" || e.key.startsWith("Arrow"))) {
        e.preventDefault();
        setTimeout(
          () => focusables(document.querySelector(".fullscreen")!)[0]?.focus(),
          20,
        );
        return;
      }
    }
    if ([10252, 415, 19, 413, 417, 412, 427, 428].includes(code)) {
      e.preventDefault();
      const p = playerRef.current;
      if (code === 413) stop();
      else if (code === 417) safe(() => p?.seek(30));
      else if (code === 412) safe(() => p?.seek(-10));
      else if (code === 427 || code === 428) {
        const list = view === "live" ? filteredLive : catalog.live;
        const index = list.findIndex((i) => i.id === previewRef.current?.id);
        if (list.length)
          void start(
            list[(index + (code === 427 ? 1 : -1) + list.length) % list.length],
            fullRef.current,
          );
      } else if (code === 415) {
        if (p?.status.state === "paused") p.toggle();
      } else if (code === 19) {
        if (p?.status.state === "playing") p.toggle();
      } else p?.toggle();
      return;
    }
    const target = e.target as HTMLElement;
    if (
      target.matches('textarea,input:not([type="checkbox"]):not([type="radio"])') &&
      !(target as HTMLInputElement).readOnly &&
      (e.key === "ArrowLeft" || e.key === "ArrowRight")
    )
      return;
    if (target.matches("select")) return;
    if (e.key.startsWith("Arrow")) {
      e.preventDefault();
      moveFocus(
        e.key,
        [...document.querySelectorAll(".modal")].pop() ||
          document.querySelector(".fullscreen") ||
          document,
      );
    }
  };
  useEffect(() => {
    registerKeys();
    resizeCanvas();
    const p = new Player({ ...savedRef.current.settings }, () =>
      repaint((v) => v + 1),
    );
    playerRef.current = p;
    const key = (e: KeyboardEvent) => keyHandler.current(e),
      resize = () => {
        resizeCanvas();
        repaint((v) => v + 1);
      },
      visibility = () => {
        if (document.hidden) record();
        p.visibility(document.hidden);
      };
    document.addEventListener("keydown", key);
    window.addEventListener("resize", resize);
    window.visualViewport?.addEventListener("resize", resize);
    const focus = (e: FocusEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest(".shell"))
        setNavCollapsed(!target.closest(".navigation"));
      rememberFocus(target);
      scrollFocused(target);
    };
    document.addEventListener("focusin", focus);
    document.addEventListener("visibilitychange", visibility);
    const timer = setInterval(() => {
      if (p.status.state === "playing") record();
    }, 15000);
    const selected = savedRef.current.profiles.find(
      (p) => p.id === savedRef.current.activeProfile,
    );
    if (selected && !savedRef.current.signedOut) void login(selected, true);
    setTimeout(() => {
      if (document.activeElement === document.body) focusables()[0]?.focus();
    }, 80);
    return () => {
      clearInterval(timer);
      document.removeEventListener("keydown", key);
      window.removeEventListener("resize", resize);
      window.visualViewport?.removeEventListener("resize", resize);
      document.removeEventListener("focusin", focus);
      document.removeEventListener("visibilitychange", visibility);
      p.dispose();
    };
  }, []);
  useEffect(() => {
    const nodes = [
      ...document.querySelectorAll<HTMLElement>(
        ".shell main, .shell .category-list, .shell .channel-list, .shell .catalog-title-list, .shell .catalog-detail-scroll",
      ),
    ];
    const positions = memories.current[view]?.scroll || [];
    nodes.forEach((e, i) => {
      e.scrollTop = positions[i] || 0;
      e.dispatchEvent(new Event("scroll"));
    });
    const root = document.getElementById("app");
    if (root) root.scrollTop = 0;
  }, [view, session?.id]);
  useEffect(() => {
    const d = saved.settings.theme;
    document.documentElement.dataset.theme =
      d === "system"
        ? matchMedia("(prefers-color-scheme:light)").matches
          ? "light"
          : "dark"
        : d;
  }, [saved.settings.theme]);
  useEffect(() => {
    if (providerRef.current) providerRef.current.settings = saved.settings;
  }, [saved.settings]);
  useEffect(() => {
    const id = setTimeout(() => {
      const root =
        [...document.querySelectorAll(".modal")].pop() ||
        document.querySelector(".fullscreen") ||
        document.getElementById("app")!;
      if (!root.contains(document.activeElement)) focusables(root)[0]?.focus();
    }, 50);
    return () => clearTimeout(id);
  }, [view, !!session, editing, !!detail, !!resume, tracks, exit, full, license.ready, license.allowed]);
  const surface = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (preview && surface.current)
      playerRef.current?.position(surface.current.getBoundingClientRect());
  });
  const filteredLive = useMemo(
    () =>
      catalog.live.filter(
        (i) =>
          (category === "all" || category === "favorites"
            ? category !== "favorites" ||
              favorites.some((f) => itemKey(f) === itemKey(i))
            : i.category === category) &&
          i.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
      ),
    [catalog, category, query, favorites],
  );
  const p = playerRef.current,
    status = p?.status;
  useEffect(() => {
    showControls();
    if (!full) return;
    const timer = setInterval(() => {
      if (
        !tracksRef.current &&
        !document.querySelector(".modal") &&
        playerRef.current?.status.state === "playing" &&
        Date.now() - lastInteraction.current > policy.controlsHideMs
      ) {
        playerMenuNavigation.current = false;
        setControlsVisible(false);
      }
    }, 100);
    return () => clearInterval(timer);
  }, [full]);
  const [now, next] = preview ? currentNext(epg[preview.id] || []) : [];
  const favoriteRows = favorites.filter((i) => i.kind === "live");
  useEffect(() => {
    if (view === "home")
      favoriteRows.slice(0, 20).forEach((i) => void getEpg(i));
  }, [view, session, catalog]);
  const info = (item: Item) => currentNext(epg[item.id] || []);
  const channelRow = (item: Item) => (
    <button
      class={`channel ${preview?.id === item.id ? "selected" : ""}`}
      onFocus={() => void getEpg(item)}
      onClick={() => void start(item)}
      data-channel={item.id}
      data-focus-key={itemKey(item)}
    >
      <Logo item={item} />
      <span class="channel-copy">
        <strong>{item.name}</strong>
        <span class="epg-pair">
          <small>Jetzt: {info(item)[0]?.title || "Keine Programmdaten"}</small>
          <small>Danach: {info(item)[1]?.title || "Keine Programmdaten"}</small>
        </span>
      </span>
    </button>
  );
  const card = (item: Item, continuing = false) => (
    <button
      class={`media-card ${item.kind === "live" ? "live-card" : ""}`}
      onClick={() =>
        continuing
          ? void start(
              item,
              true,
              history.find((h) => itemKey(h.item) === itemKey(item))
                ?.position || 0,
            )
          : item.kind === "live"
            ? void start(item, true)
            : void openDetail(item)
      }
    >
      <Logo item={item} large />
      <strong>{item.name}</strong>
      {item.kind === "live" && (
        <small>{info(item)[0]?.title || "Live-TV"}</small>
      )}
    </button>
  );
  const section = (title: string, items: Item[]) =>
    items.length > 0 && (
      <section class="home-section">
        <h2>{title}</h2>
        <div class="card-rail">
          {items.slice(0, 30).map((item) => (
            <span key={itemKey(item)}>
              {card(item, title.startsWith("Weiter ansehen"))}
            </span>
          ))}
        </div>
      </section>
    );
  const checkUpdates = async () => {
    setUpdate("Version wird geprüft …");
    try {
      const r = await fetch(
        "https://api.github.com/repos/tariktech7/dado-player/releases/latest",
      );
      if (!r.ok) throw 0;
      const data = await r.json();
      const asset = data.assets?.find((a: any) =>
        /^Dado-Player-Tizen-.*\.wgt$/.test(a.name),
      );
      setUpdateUrl(data.html_url);
      setUpdate(
        asset
          ? `Verfügbar: ${asset.name}. Tizen-Pakete müssen über den PC für diesen TV signiert und installiert werden.`
          : "Kein neueres Tizen-Paket veröffentlicht.",
      );
    } catch {
      setUpdate(
        "GitHub ist nicht erreichbar oder benötigt Zugriff auf das private Repository. Update über den PC prüfen.",
      );
    }
  };
  const choose = (
    value: string,
    onChange: (v: string) => void,
    options: [string, string][],
  ) => <TVSelect value={value} onChange={onChange} options={options} />;
  const settingsView = () => (
    <div class="settings-page">
      <header>
        <span class="eyebrow">DADO PLAYER · TIZEN {APP_VERSION}</span>
        <h1>Dein Fernseher. Dein Bild.</h1>
        <p>
          Samsung AVPlay mit der Dado-Steuerung. Einstellungen bleiben auf
          diesem Gerät gespeichert.
        </p>
      </header>
      <div class="settings-grid">
        <section class="glass" data-license-status data-focus-zone="settings-license">
          <h2>Lizenz & Gerät</h2>
          <p><strong>{license.text}</strong></p>
          <p>Geräte-ID: <strong>{license.deviceId}</strong></p>
          <p>MAC-Adresse: <strong>{license.mac || 'Nicht verfügbar'}</strong></p>
          <p class="muted">{license.identitySource}. Die Geräte-ID bleibt die Kennung für deine Freischaltung.</p>
          {license.error && <p role="status">{license.error}</p>}
          {!license.claims && <button disabled={license.busy || !license.ready} onClick={()=>void licenseClient.startTrial()}>7 Tage kostenlos testen</button>}
          <button disabled={license.busy || !license.ready} onClick={()=>void licenseClient.refresh()}>
            {license.busy?'Wird abgerufen …':license.claims?.kind==='permanent'?'Lizenz wiederherstellen':'Freischaltung abrufen'}
          </button>
          <p class="muted">Nach manueller Freischaltung einmal abrufen. Unlimited-Lizenzen werden anschließend offline genutzt.</p>
        </section>
        <section class="glass" data-focus-zone="settings-appearance">
          <h2>Darstellung</h2>
          <Field label="Expertenmodus">
            <TVInput type="checkbox" checked={saved.settings.expertMode}
              onChange={(e) => setting("expertMode", e.currentTarget.checked)} />
          </Field>
          <p class="muted">Standard: automatische Wiedergabe mit Dado-Steuerung. Zusätzliche Player- und Netzwerkoptionen im Expertenmodus.</p>
          <Field label="Farbschema">
            {choose(
              saved.settings.theme,
              (v) => setting("theme", v as Settings["theme"]),
              [
                ["warm", "Blau"],
                ["dark", "Dunkel"],
                ["light", "Hell"],
                ["system", "System"],
              ],
            )}
          </Field>
          <Field label="Senderdichte">
            {choose(
              saved.settings.density,
              (v) => setting("density", v as Settings["density"]),
              [
                ["compact", "Kompakt · 10 Sender"],
                ["comfortable", "Komfortabel · 8 Sender"],
              ],
            )}
          </Field>
          <p class="muted">
            Die Oberfläche skaliert mit Full HD und 4K. Logos behalten ihr
            Seitenverhältnis.
          </p>
        </section>
        <section class="glass" data-focus-zone="settings-playback">
          <h2>Wiedergabe</h2>
          {saved.settings.expertMode && <>
          <Field label="Wiedergabe-Engine">
            {choose(
              saved.settings.player,
              (v) => setting("player", v as Settings["player"]),
              [
                ["auto", "Automatisch · empfohlen"],
                ["avplay", "Samsung AVPlay"],
                ["html", "HTML-Video · Alternative"],
              ],
            )}
          </Field>
          <Field label="Live-Streamformat">
            {choose(
              saved.settings.liveFormat,
              (v) => {
                setting("liveFormat", v as Settings["liveFormat"]);
                setCatalog((old) => ({
                  ...old,
                  live: old.live.map((i) => ({
                    ...i,
                    url:
                      session?.kind === "Xtream"
                        ? providerRef.current!.stream("live", i.id, v)
                        : i.url,
                  })),
                }));
              },
              [
                ["m3u8", "HLS (.m3u8)"],
                ["ts", "MPEG-TS (.ts)"],
              ],
            )}
          </Field>
          </>}
          <Field label="Bildformat">
            {choose(
              saved.settings.aspect,
              (v) => {
                setting("aspect", v as Settings["aspect"]);
                p?.setAspect(v as Settings["aspect"]);
              },
              [
                ["fit", "Originalformat · einpassen"],
                ["fill", "Automatisches Seitenverhältnis"],
                ["stretch", "Bildschirm ausfüllen · strecken"],
              ],
            )}
          </Field>
          <Field label="Letzten Titel beim Start abspielen">
            <TVInput
              type="checkbox"
              checked={saved.settings.autoplayLast}
              onChange={(e) => setting("autoplayLast", e.currentTarget.checked)}
            />
          </Field>
        </section>
        <section class="glass" data-focus-zone="settings-audio">
          <h2>Ton & Untertitel</h2>
          <Field label="Bevorzugte Audiosprache">
            <TVInput
              placeholder="z. B. deu oder ger"
              value={saved.settings.audioLanguage}
              onInput={(e) => setting("audioLanguage", e.currentTarget.value)}
            />
          </Field>
          <Field label="Untertitelsprache">
            <TVInput
              placeholder="z. B. deu"
              value={saved.settings.subtitleLanguage}
              onInput={(e) =>
                setting("subtitleLanguage", e.currentTarget.value)
              }
            />
          </Field>
          <Field label="Untertitel automatisch">
            <TVInput
              type="checkbox"
              checked={saved.settings.subtitles}
              onChange={(e) => setting("subtitles", e.currentTarget.checked)}
            />
          </Field>
          <Field label="Untertitelgröße">
            {choose(
              String(saved.settings.subtitleSize),
              (v) => setting("subtitleSize", Number(v)),
              [
                ["26", "Klein"],
                ["32", "Normal"],
                ["40", "Groß"],
                ["48", "Sehr groß"],
              ],
            )}
          </Field>
          <p class="muted">
            Spuren hängen vom Stream ab und sind im Player auswählbar.
            Tonausgabe und Passthrough verwaltet der Fernseher.
          </p>
        </section>
        <section class="glass" data-focus-zone="settings-network">
          <h2>Netzwerk</h2>
          {saved.settings.expertMode && <>
          <Field label="Stream-Puffer">
            {choose(
              String(saved.settings.bufferSeconds),
              (v) => setting("bufferSeconds", Number(v)),
              [
                ["5", "5 Sekunden"],
                ["10", "10 Sekunden"],
                ["20", "20 Sekunden"],
                ["45", "45 Sekunden"],
              ],
            )}
          </Field>
          <Field label="Zeitlimit">
            {choose(
              String(saved.settings.timeoutSeconds),
              (v) => setting("timeoutSeconds", Number(v)),
              [
                ["10", "10 Sekunden"],
                ["20", "20 Sekunden"],
                ["40", "40 Sekunden"],
                ["60", "60 Sekunden"],
              ],
            )}
          </Field>
          <Field label="Player User-Agent">
            <TVInput
              value={saved.settings.userAgent}
              onInput={(e) => setting("userAgent", e.currentTarget.value)}
              placeholder="Standard des Fernsehers"
            />
          </Field>
          </>}
          <Field label="Anbieterzugriff">
            {choose(
              saved.settings.networkMode,
              (v) => setting("networkMode", v as Settings["networkMode"]),
              [
                ["direct", "Direkt"],
                ["companion", "Companion im Heimnetz (API/EPG)"],
              ],
            )}
          </Field>
          {saved.settings.networkMode === "companion" && (
            <>
              <Field label="Companion-Adresse">
                <TVInput
                  value={saved.settings.companionUrl}
                  onInput={(e) =>
                    setting("companionUrl", e.currentTarget.value)
                  }
                  placeholder="http://192.168.178.x:18498"
                />
              </Field>
              <Field label="Companion-Zugangsschlüssel">
                <TVInput
                  type="password"
                  value={saved.settings.companionToken}
                  onInput={(e) =>
                    setting("companionToken", e.currentTarget.value)
                  }
                />
              </Field>
            </>
          )}
          <p class="muted">
            TLS-Zertifikate werden geprüft. Ein deaktivierter TLS-Schutz oder
            MPV-/Direct3D-Filter stehen auf Tizen nicht zur Verfügung.
            Netzwerkänderungen greifen beim erneuten Laden.
          </p>
          <button onClick={() => session && void login(session)}>
            Bibliothek neu laden
          </button>
        </section>
        <section class="glass" data-focus-zone="settings-profiles">
          <h2>Profile & Updates</h2>
          <p>
            {session?.name} · {samsung() ? "Samsung-TV" : "Browser-Vorschau"}
          </p>
          <button
            onClick={() => {
              setForm(emptyProfile());
              setEditing(true);
            }}
          >
            Profil hinzufügen
          </button>
          <button onClick={logout}>Profil wechseln / Abmelden</button>
          <button onClick={() => void checkUpdates()}>
            GitHub-Update prüfen
          </button>
          <button
            onClick={() =>
              void start(
                {
                  id: "dado-diagnostic",
                  kind: "vod",
                  name: "Dado Wiedergabetest",
                  category: "",
                  logo: "",
                  url: new URL("test-video.mp4", location.href).href,
                },
                true,
              )
            }
          >
            Integriertes Testvideo abspielen
          </button>
          {update && <p role="status">{update}</p>}
          {updateUrl && (
            <p class="muted">github.com/tariktech7/dado-player/releases</p>
          )}
          <p class="muted">{APP_VERSION} · Tizen 6.5+ · D7</p>
        </section>
        <section class="glass" data-focus-zone="settings-parental">
          <h2>Elternsperre</h2>
          <p>{parental.enabled?'Aktiv · gesperrte Inhalte benötigen die Eltern-PIN':'Vierstellige PIN für Sender, Filme und Serien einrichten'}</p>
          <button onClick={()=>{stop();parental.manage();}}>{parental.enabled?'Elternsperre verwalten':'Eltern-PIN einrichten'}</button>
        </section>
        <ConnectPanel importProfiles={importCloudProfiles} />
      </div>
    </div>
  );
  const modal = (title: string, children: ComponentChildren) => (
    <div class="scrim">
      <section
        class="modal glass"
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <h1>{title}</h1>
        {children}
      </section>
    </div>
  );
  const profileForm = () => (
    <form
      class="profile-form"
      onSubmit={(e) => {
        e.preventDefault();
        void login(form);
      }}
    >
      <h1>
        {saved.profiles.length
          ? "Profil verbinden"
          : "Willkommen bei Dado Player"}
      </h1>
      <p>Deine Sender. Dein Programm. Direkt auf deinem Samsung-TV.</p>
      <Field label="Profilname">
        <TVInput
          value={form.name}
          required
          onInput={(e) => setForm({ ...form, name: e.currentTarget.value })}
        />
      </Field>
      <Field label="Anmeldemethode">
        {choose(
          form.kind,
          (v) => setForm({ ...form, kind: v as Profile["kind"] }),
          [
            ["Xtream", "Xtream Codes"],
            ["M3u", "M3U-Wiedergabeliste"],
          ],
        )}
      </Field>
      {form.kind === "Xtream" ? (
        <>
          <Field label="Server-Adresse">
            <TVInput
              value={form.host}
              required
              placeholder="https://anbieter.example:8080"
              onInput={(e) => setForm({ ...form, host: e.currentTarget.value })}
            />
          </Field>
          <Field label="Benutzername">
            <TVInput
              value={form.username}
              required
              autocomplete="username"
              onInput={(e) =>
                setForm({ ...form, username: e.currentTarget.value })
              }
            />
          </Field>
          <Field label="Passwort">
            <TVInput
              type="password"
              required
              value={form.password}
              autocomplete="current-password"
              onInput={(e) =>
                setForm({ ...form, password: e.currentTarget.value })
              }
            />
          </Field>
        </>
      ) : (
        <>
          <Field label="M3U-Adresse">
            <TVInput
              value={form.m3uUrl}
              required
              placeholder="https://anbieter.example/playlist.m3u"
              onInput={(e) =>
                setForm({ ...form, m3uUrl: e.currentTarget.value })
              }
            />
          </Field>
          <Field label="XMLTV-EPG-Adresse (optional)">
            <TVInput
              value={form.epgUrl}
              placeholder="https://anbieter.example/guide.xml"
              onInput={(e) =>
                setForm({ ...form, epgUrl: e.currentTarget.value })
              }
            />
          </Field>
        </>
      )}
      <button class="primary" type="submit" disabled={busy}>
        {busy ? "Verbinden …" : "Verbinden & speichern"}
      </button>
      {saved.profiles.length > 0 && (
        <button type="button" onClick={() => setEditing(false)}>
          Zurück zu Profilen
        </button>
      )}
      <button
        type="button"
        onClick={() => {
          setEditing(false);
          setView("settings");
          setSession({
            id: "setup",
            name: "Einrichtung",
            kind: "M3u",
            host: "",
            username: "",
            password: "",
            m3uUrl: "",
            epgUrl: "",
          });
        }}
      >
        Einstellungen & Wiedergabetest
      </button>
      <p class="muted">
        Zugangsdaten werden ausschließlich lokal im App-Speicher abgelegt. Keine
        Dado-Cloud.
      </p>
    </form>
  );
  if (!license.allowed) return (
    <div class="welcome" data-license-gate>
      <div class="welcome-panel glass" role="region" aria-label="Lizenz & Freischaltung">
        <span class="eyebrow">DADO PLAYER · LIZENZ & GERÄT</span>
        <h1>{!license.ready ? 'Lizenz wird geprüft …' : license.claims?.kind==='trial' ? 'Ihre Testversion ist abgelaufen' : 'Willkommen bei Dado Player'}</h1>
        <p>{license.claims?.kind==='trial' ? 'Die 7 Tage Testzeit sind vorbei. Zur weiteren Nutzung benötigen Sie eine dauerhafte Freischaltung.' : 'Testen Sie Dado Player 7 Tage kostenlos. Starten Sie die Testversion einmal online oder rufen Sie Ihre vorhandene Freischaltung ab.'}</p>
        <p><strong>{license.text}</strong></p>
        <p>Geräte-ID: <strong>{license.deviceId}</strong></p>
        <p>MAC-Adresse: <strong>{license.mac || 'Nicht verfügbar'}</strong></p>
        <p class="muted">Nennen Sie für die manuelle Freigabe die Geräte-ID oder Ihre registrierte MAC-Adresse. Wählen Sie nach der Freigabe „Freischaltung abrufen“. Profile und Favoriten bleiben gespeichert.</p>
        {license.error && <p role="status">{license.error}</p>}
        {!license.claims && <button disabled={license.busy || !license.ready} onClick={()=>void licenseClient.startTrial()}>7 Tage kostenlos testen</button>}
        <button disabled={license.busy || !license.ready} onClick={()=>void licenseClient.refresh()}>
          {license.busy?'Wird abgerufen …':'Freischaltung abrufen'}
        </button>
        <button onClick={()=>exitApp()}>App schließen</button>
        <p class="muted">Nach der dauerhaften Freischaltung: Unlimited, ohne automatische Lizenz-Serverabfragen.</p>
      </div>
    </div>
  );
  return (
    <>
      {!session && !busy ? (
        <div class="welcome">
          <div class="welcome-brand">
            <><img class="brand-normal" src="d7.svg" /><img class="brand-dark" src="d7-dark.svg" /></>
            <strong>Dado Player</strong>
            <span>SAMSUNG TV</span>
          </div>
          <div class="welcome-panel glass">
            {editing || !saved.profiles.length ? (
              profileForm()
            ) : (
              <>
                <span class="eyebrow">DEINE PROFILE</span>
                <h1>Schön, dass du da bist.</h1>
                <div class="profile-list">
                  {saved.profiles.map((profile) => (
                    <div>
                      <button onClick={() => void login(profile)}>
                        <strong>{profile.name}</strong>
                        <small>{profile.kind}</small>
                      </button>
                      <button
                        aria-label={`${profile.name} bearbeiten`}
                        onClick={() => {
                          setForm(profile);
                          setEditing(true);
                        }}
                      >
                        Bearbeiten
                      </button>
                      <button
                        aria-label={`${profile.name} entfernen`}
                        onClick={() =>
                          changed({
                            ...savedRef.current,
                            profiles: savedRef.current.profiles.filter(
                              (p) => p.id !== profile.id,
                            ),
                            activeProfile:
                              saved.activeProfile === profile.id
                                ? null
                                : saved.activeProfile,
                          })
                        }
                      >
                        Entfernen
                      </button>
                    </div>
                  ))}
                </div>
                <button
                  class="primary"
                  onClick={() => {
                    setForm(emptyProfile());
                    setEditing(true);
                  }}
                >
                  Neues Profil
                </button>
                <button
                  onClick={() => {
                    setView("settings");
                    setSession({
                      id: "setup",
                      name: "Einrichtung",
                      kind: "M3u",
                      host: "",
                      username: "",
                      password: "",
                      m3uUrl: "",
                      epgUrl: "",
                    });
                  }}
                >
                  Netzwerk einrichten
                </button>
              </>
            )}
            <ConnectPanel compact importProfiles={importCloudProfiles} />
          </div>
        </div>
      ) : busy ? (
        <div class="loading">
          <><img class="brand-normal" src="d7.svg" /><img class="brand-dark" src="d7-dark.svg" /></>
          <h1>Deine Bibliothek wird geladen</h1>
          <p>Live-TV, Filme und Serien …</p>
          <button
            onClick={() => {
              generation.current++;
              setBusy(false);
            }}
          >
            Abbrechen
          </button>
        </div>
      ) : (
        <>
          <div
            data-focus-scope={view}
            class={`shell ${navCollapsed.current ? "nav-collapsed" : ""} ${view === "live" ? "live-shell" : ""} ${view === "live" ? "with-preview" : ""}`}
          >
            <aside class="navigation" data-focus-zone="navigation">
              <div class="brand">
                <><img class="brand-normal" src="d7.svg" /><img class="brand-dark" src="d7-dark.svg" /></>
                <div class="brand-copy"><strong>Dado Player</strong>
                  {license.claims?.kind==='trial' && <small class="trial-badge" data-trial-badge>Testversion · {license.text.replace(' Testzeit','').replace('Noch','noch')}</small>}
                </div>
              </div>
              {license.claims?.kind==='trial' && <small class="trial-badge-compact" title={license.text}>Trial · {license.text.match(/\d+/)?.[0]} T</small>}
              <nav>
                {nav.map(([id, icon, label]) => (
                  <button
                    aria-label={`${icon} ${label}`}
                    title={label}
                    data-focus-key={id}
                    class={view === id ? "selected" : ""}
                    onClick={() => go(id)}
                  >
                    <span>{icon}</span>
                    <span class="nav-label">{label}</span>
                  </button>
                ))}
              </nav>
              <footer>
                <span>{session?.name}</span>
                <small>Tizen {APP_VERSION}</small>
              </footer>
            </aside>
            {view === "live" ? (
              <>
                <section class="categories" data-focus-zone="categories">
                  <h1>Kategorien</h1>
                  <div class="category-list">
                    <button
                      class={category === "all" ? "selected" : ""}
                      onClick={() => {
                        stop();
                        setCategory("all");
                      }}
                    >
                      Alle Sender
                    </button>
                    <button
                      class={category === "favorites" ? "selected" : ""}
                      onClick={() => {
                        stop();
                        setCategory("favorites");
                      }}
                    >
                      Favoriten
                    </button>
                    {catalog.categories.live.map((c) => (
                      <button
                        class={category === c.id ? "selected" : ""}
                        onClick={() => {
                          stop();
                          setCategory(c.id);
                        }}
                      >
                        {c.name}
                      </button>
                    ))}
                  </div>
                </section>
                <section
                  class={`channels ${saved.settings.density}`}
                  data-focus-zone="titles"
                >
                  <h1>
                    {category === "all"
                      ? "Alle Sender"
                      : category === "favorites"
                        ? "Favoriten"
                        : catalog.categories.live.find((c) => c.id === category)
                            ?.name || "Sender"}
                  </h1>
                  <TVInput
                    class="channel-search"
                    type="search"
                    placeholder="Sender suchen"
                    aria-label="Sender suchen"
                    value={query}
                    onInput={(e) => setQuery(e.currentTarget.value)}
                  />
                  <VirtualChannels
                    items={filteredLive}
                    row={channelRow}
                    epg={getEpg}
                    density={saved.settings.density}
                  />
                </section>
                {!preview && (
                  <aside class="preview-panel" data-focus-zone="details">
                    <header>
                      <h2>Vorschau</h2>
                    </header>
                    <div class="preview-surface empty-preview">
                      <span>▷</span>
                    </div>
                    <h2>Sender auswählen</h2>
                    <p>
                      Hier erscheinen Vorschau, Programm und
                      Senderinformationen.
                    </p>
                  </aside>
                )}
                {preview && (
                  <aside class="preview-panel" data-focus-zone="details">
                    <header>
                      <h2>Vorschau</h2>
                      <button
                        class="icon-button"
                        aria-label="Sender als Favorit"
                        onClick={() => toggleFavorite(preview)}
                      >
                        {isFavorite(preview) ? "★" : "☆"}
                      </button>
                    </header>
                    <div
                      ref={!full ? surface : undefined}
                      class="preview-surface"
                    />
                    <button class="preview-open" onClick={() => setFull(true)}>
                      Vollbild öffnen <span>↗</span>
                    </button>
                    <PlayerFeedback status={status} />
                    <h2>{preview.name}</h2>
                    <div class="epg-detail" tabIndex={0}>
                      {now ? (
                        <>
                          <small>
                            Jetzt · {clock(now.start)} — {clock(now.end)}
                          </small>
                          <h3>{now.title}</h3>
                          <p>{now.description}</p>
                        </>
                      ) : (
                        <p>Keine aktuellen Programmdaten.</p>
                      )}
                      {next && (
                        <>
                          <hr />
                          <small>
                            Danach · {clock(next.start)} — {clock(next.end)}
                          </small>
                          <h3>{next.title}</h3>
                          <p>{next.description}</p>
                        </>
                      )}
                      <hr />
                      <small>
                        {session?.kind} ·{" "}
                        {preview.url
                          .match(/\.([a-z0-9]{2,5})(?:[?#]|$)/i)?.[1]
                          .toUpperCase() || "STREAM"}{" "}
                        ·{" "}
                        {saved.settings.player !== "html" && samsung()
                          ? "Samsung AVPlay"
                          : "HTML-Video"}
                      </small>
                    </div>
                  </aside>
                )}
              </>
            ) : (
              <main
                data-focus-zone={
                  ["home", "vod", "series", "settings"].includes(view)
                    ? undefined
                    : "content"
                }
                class={
                  view === "vod" || view === "series" ? "catalog-main" : ""
                }
              >
                {view === "home" ? (
                  <Home
                    name={session?.name || ""}
                    catalog={catalog}
                    history={history}
                    favorites={favoriteRows}
                    program={(i) =>
                      info(i)[0]?.title || "Keine Programmdaten verfügbar"
                    }
                    play={(i, pos) => void continueWatching(i, pos)}
                    section={section}
                    logo={(i) => <Logo item={i} />}
                  />
                ) : view === "settings" ? (
                  settingsView()
                ) : view === "history" ? (
                  <>
                    <h1>Fortsetzen</h1>
                    <div
                      class="kind-tabs"
                      role="tablist"
                      aria-label="Fortsetzen aufteilen"
                    >
                      {[
                        ["live", "Live-TV"],
                        ["series", "Serien"],
                        ["vod", "Filme"],
                      ].map(([kind, label]) => (
                        <button
                          role="tab"
                          aria-selected={historyKind === kind}
                          class={historyKind === kind ? "selected" : ""}
                          onClick={() => setHistoryKind(kind)}
                        >
                          {label}
                        </button>
                      ))}
                    </div>
                    <div class="history-list">
                      {!history.length && <p>Noch nichts angesehen.</p>}
                      {history
                        .filter((h) =>
                          historyKind === "series"
                            ? h.item.kind === "episode" ||
                              h.item.kind === "series"
                            : h.item.kind === historyKind,
                        )
                        .map((h) => (
                          <div>
                            <button onClick={() => void continueWatching(h.item,h.position)}>
                              <Logo item={h.item} />
                              <span>
                                <strong>{h.item.name}</strong>
                                <small>
                                  {h.item.kind === "live"
                                    ? "Live-TV"
                                    : `Bei ${time(h.position)} · ${time(h.duration)}`}
                                </small>
                              </span>
                              <span>▷</span>
                            </button>
                            <button
                              aria-label={`${h.item.name} aus Verlauf entfernen`}
                              onClick={() =>
                                changed({
                                  ...savedRef.current,
                                  history: {
                                    ...savedRef.current.history,
                                    [pid]: history.filter(
                                      (v) =>
                                        itemKey(v.item) !== itemKey(h.item),
                                    ),
                                  },
                                })
                              }
                            >
                              Entfernen
                            </button>
                          </div>
                        ))}
                    </div>
                  </>
                ) : view === "favorites" ? (
                  <>
                    <h1>Favoriten</h1>
                    <div
                      class="kind-tabs"
                      role="tablist"
                      aria-label="Favoriten aufteilen"
                    >
                      {[
                        ["live", "Live-TV"],
                        ["series", "Serien"],
                        ["vod", "Filme"],
                      ].map(([kind, label]) => (
                        <button
                          role="tab"
                          aria-selected={favoriteKind === kind}
                          class={favoriteKind === kind ? "selected" : ""}
                          onClick={() => setFavoriteKind(kind)}
                        >
                          {label}
                        </button>
                      ))}
                    </div>
                    <div class="media-grid">
                      {favorites
                        .filter((i) =>
                          favoriteKind === "series"
                            ? i.kind === "series" || i.kind === "episode"
                            : i.kind === favoriteKind,
                        )
                        .map((i) => card(i))}
                    </div>
                    {!favorites.length && (
                      <p>Deine gemerkten Sender und Titel erscheinen hier.</p>
                    )}
                  </>
                ) : view === "search" ? (
                  <>
                    <h1>Was möchtest du sehen?</h1>
                    <TVInput
                      class="global-search"
                      type="search"
                      aria-label="Bibliothek durchsuchen"
                      placeholder="Sender, Film oder Serie suchen"
                      value={query}
                      onInput={(e) => setQuery(e.currentTarget.value)}
                    />
                    {query.trim().length >= 2 ? (
                      <div class="media-grid">
                        {[...catalog.live, ...catalog.vod, ...catalog.series]
                          .filter((i) =>
                            i.name.toLowerCase().includes(query.toLowerCase()),
                          )
                          .slice(0, 120)
                          .map((i) => card(i))}
                      </div>
                    ) : (
                      <p class="muted">Mindestens zwei Zeichen eingeben.</p>
                    )}
                  </>
                ) : (
                  <CatalogBrowser
                    key={view}
                    items={catalog[view === "vod" ? "vod" : "series"]}
                    categories={
                      catalog.categories[view === "vod" ? "vod" : "series"]
                    }
                    category={category}
                    setCategory={(c) => {
                      setCategory(c);
                      setSelectedTitle(null);
                      setDetail(null);
                    }}
                    query={query}
                    setQuery={setQuery}
                    selected={selectedTitle}
                    select={(i) => {
                      setSelectedTitle(i);
                      void openDetail(i);
                    }}
                    favorites={favorites}
                    detail={detail}
                    episodes={episodes}
                    history={history}
                    toggle={toggleFavorite}
                    play={(i, pos) => void start(i, true, pos)}
                  />
                )}
                {catalog.issues.length > 0 && (
                  <div class="catalog-issues" role="status">
                    {catalog.issues.join(" ")}{" "}
                    <button onClick={() => session && void login(session)}>
                      Erneut laden
                    </button>
                  </div>
                )}
              </main>
            )}
          </div>
          {editing && modal("Profil hinzufügen", profileForm())}
          {full && preview && (
            <div
              class={`fullscreen ${status?.state === "error" ? "playback-failed" : ""} ${controlsVisible ? "" : "controls-hidden"}`}
              onMouseMove={showControls}
              onClick={showControls}
            >
              <div ref={surface} class="full-surface" />
              <div class="player-top glass">
                <button onClick={() => previewRef.current?.kind === "live" ? setFull(false) : stop()}>← Zurück</button>
                <span>
                  <small>
                    {preview.kind === "live"
                      ? "LIVE"
                      : preview.kind === "episode"
                        ? "SERIE"
                        : "FILM"}
                  </small>
                  <strong>{preview.name}</strong>
                </span>
                <button onClick={() => toggleFavorite(preview)}>
                  {isFavorite(preview) ? "★ Favorit" : "☆ Favorit"}
                </button>
              </div>
              <div class="player-feedback">
                <PlayerFeedback status={status} />
                {status?.state === "error" && (
                  <button onClick={() => void startRetry()}>
                    Erneut versuchen
                  </button>
                )}
              </div>
              {saved.settings.subtitles && p?.subtitle && (
                <div
                  class="subtitles"
                  style={{ fontSize: `${saved.settings.subtitleSize}px` }}
                >
                  {p.subtitle}
                </div>
              )}
              <div class="player-controls glass">
                <div class="transport">
                  <button
                    disabled={preview.kind === "live"}
                    onClick={() => safe(() => p?.seek(-10))}
                  >
                    ↶ 10 s
                  </button>
                  <button class="primary" onClick={() => p?.toggle()}>
                    {status?.state === "paused" ? "▷ Abspielen" : "Ⅱ Pause"}
                  </button>
                  <button
                    disabled={preview.kind === "live"}
                    onClick={() => safe(() => p?.seek(30))}
                  >
                    30 s ↷
                  </button>
                  <button
                    onClick={() => {
                      p?.refreshTracks();
                      setTracks(true);
                    }}
                  >
                    Ton & Untertitel
                  </button>
                  {choose(
                    saved.settings.aspect,
                    (v) => {
                      setting("aspect", v as Settings["aspect"]);
                      p?.setAspect(v as Settings["aspect"]);
                    },
                    [
                      ["fit", "Einpassen"],
                      ["fill", "Automatisch"],
                      ["stretch", "Strecken"],
                    ],
                  )}
                  <button onClick={stop}>Beenden</button>
                </div>
                <div class="timeline">
                  <span>
                    {preview.kind === "live"
                      ? "● LIVE"
                      : time(status?.time || 0)}
                  </span>
                  <progress
                    value={status?.time || 0}
                    max={status?.duration || 1}
                  />
                  <span>
                    {preview.kind === "live"
                      ? now?.title || ""
                      : time(status?.duration || 0)}
                  </span>
                </div>
              </div>
            </div>
          )}
          {detail &&
            view !== "vod" &&
            view !== "series" &&
            modal(
              detail.name,
              <>
                <div class="detail-body">
                  <Logo item={detail} large />
                  <div>
                    <p>
                      {detail.description ||
                        "Informationen werden vom Anbieter bereitgestellt."}
                    </p>
                    <button onClick={() => toggleFavorite(detail)}>
                      {isFavorite(detail)
                        ? "★ Aus Favoriten entfernen"
                        : "☆ Merken"}
                    </button>
                    {detail.kind !== "series" && (
                      <button class="primary" onClick={() => playTitle(detail)}>
                        Abspielen
                      </button>
                    )}
                  </div>
                </div>
                {detail.kind === "series" && (
                  <EpisodePicker
                    episodes={episodes}
                    history={history}
                    play={(i, pos) => void start(i, true, pos)}
                  />
                )}
                <button onClick={() => setDetail(null)}>Zurück</button>
              </>,
            )}
          {resume &&
            modal(
              "Weiterschauen?",
              <>
                <p>
                  {resume.item.name} · Bei {time(resume.position)}
                </p>
                <button
                  class="primary"
                  onClick={() => {
                    const r = resume;
                    setResume(null);
                    void start(r.item, true, r.position);
                  }}
                >
                  Fortsetzen
                </button>
                <button
                  onClick={() => {
                    const r = resume;
                    setResume(null);
                    void start(r.item, true, 0);
                  }}
                >
                  Von vorne
                </button>
                <button onClick={() => setResume(null)}>Abbrechen</button>
              </>,
            )}
          {tracks &&
            modal(
              "Ton & Untertitel",
              <>
                <h2>Tonspuren</h2>
                {status?.tracks
                  .filter((t) => t.type === "AUDIO")
                  .map((t) => (
                    <button onClick={() => safe(() => p?.selectTrack(t))}>
                      {t.label}
                    </button>
                  ))}
                {!status?.tracks.some((t) => t.type === "AUDIO") && (
                  <p>Keine umschaltbaren Tonspuren verfügbar.</p>
                )}
                <h2>Untertitel</h2>
                <button
                  onClick={() => {
                    p?.subtitleOff();
                    setting("subtitles", false);
                  }}
                >
                  Aus
                </button>
                {status?.tracks
                  .filter((t) => t.type === "TEXT")
                  .map((t) => (
                    <button
                      onClick={() =>
                        safe(() => {
                          p?.selectTrack(t);
                          setting("subtitles", true);
                        })
                      }
                    >
                      {t.label}
                    </button>
                  ))}
                <button onClick={() => setTracks(false)}>
                  Zurück zum Player
                </button>
              </>,
            )}
        </>
      )}
      <ParentalPanel controls={parental} catalog={catalog} profile={session} onChange={()=>parentalRepaint(n=>n+1)}/>
      {message && (
        <div class="notification" role="alert">
          <span>{message}</span>
          <button aria-label="Meldung schließen" onClick={() => setMessage("")}>
            ×
          </button>
        </div>
      )}
      {exit &&
        modal(
          "Dado Player beenden?",
          <>
            <p>Profile und Wiedergabefortschritt bleiben gespeichert.</p>
            <button onClick={() => setExit(false)}>Weitersehen</button>
            <button
              onClick={() => {
                record();
                p?.stop();
                exitApp();
              }}
            >
              Beenden
            </button>
          </>,
        )}
    </>
  );
  async function startRetry() {
    const item = previewRef.current;
    if (!item) return;
    stop();
    await start(item, true);
  }
}

function PlayerFeedback({ status }: { status: Player["status"] | undefined }) {
  if(status?.notice)return <p class="muted" role="status">{status.notice}</p>;
  return status?.state === "loading" ? (
    <p class="muted" role="status">
      Stream wird geladen … {status.buffer}%
    </p>
  ) : status?.state === "error" ? (
    <p class="error" role="alert">
      {status.error}
    </p>
  ) : status?.state === "ended" ? (
    <p>Wiedergabe beendet.</p>
  ) : null;
}
function VirtualChannels({
  items,
  row,
  epg,
  density,
}: {
  items: Item[];
  row: (i: Item) => ComponentChildren;
  epg: (i: Item) => Promise<void>;
  density: Settings["density"];
}) {
  const [focusIndex, setFocusIndex] = useState<number | null>(null);
  const pendingFocus = useRef<number | null>(null);
  const [top, setTop] = useState(0),
    ref = useRef<HTMLDivElement>(null),
    height = shellAppearance.channelRowHeight * (density === "compact" ? 1 : 1.25);
  useEffect(() => {
    if (ref.current) ref.current.scrollTop = 0;
    setTop(0);
  }, [items]);
  const visibleStart = Math.max(0, Math.floor(top / height) - 5);
  const start = focusIndex === null ? visibleStart : Math.max(0, Math.min(visibleStart, focusIndex-2), focusIndex-32),
    end = Math.min(items.length, Math.max(start + 24, (focusIndex ?? start) + 3));
  useLayoutEffect(() => {
    if (pendingFocus.current === null) return;
    const target = ref.current?.querySelector<HTMLElement>(`[data-list-index="${pendingFocus.current}"] button`);
    if (target) { pendingFocus.current=null; target.focus({preventScroll:true}); scrollFocused(target); }
  }, [focusIndex, start]);
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      for (const item of items.slice(start, Math.min(end, start + 16))) {
        if (cancelled) return;
        await epg(item);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [items, start]);
  return (
    <div
      class="channel-list"
      ref={ref}
      onFocus={(e) => { const index=(e.target as HTMLElement).closest('[data-list-index]')?.getAttribute('data-list-index'); if(index!==null && index!==undefined)setFocusIndex(Number(index)); }}
      onBlur={(e) => { if(!e.currentTarget.contains(e.relatedTarget as Node))setFocusIndex(null); }}
      onKeyDown={(e) => {
        if(document.querySelector('.fullscreen,.modal'))return;
        if(e.key!=="ArrowUp" && e.key!=="ArrowDown")return;
        const index=Number((e.target as HTMLElement).closest('[data-list-index]')?.getAttribute('data-list-index'));
        if(!Number.isFinite(index))return;
        e.preventDefault();e.stopPropagation();
        const next=Math.max(0,Math.min(items.length-1,(pendingFocus.current??index)+(e.key==='ArrowDown'?1:-1)));
        pendingFocus.current=next;setFocusIndex(next);
        const target=ref.current?.querySelector<HTMLElement>(`[data-list-index="${next}"] button`);
        if(target){pendingFocus.current=null;target.focus({preventScroll:true});scrollFocused(target);}
      }}
      onScroll={(e) => { const next = Math.floor(e.currentTarget.scrollTop / height) * height; setTop(old => old === next ? old : next); }}
      style={{ "--row-height": `${height}px` }}
    >
      {!items.length ? (
        <p class="muted">Keine passenden Sender.</p>
      ) : (
        <>
          <div style={{ height: start * height }} />
          {items.slice(start, end).map((item, index) => (
            <div
              key={item.id}
              data-list-index={start + index}
              style={{ height }}
            >
              {row(item)}
            </div>
          ))}
          <div style={{ height: (items.length - end) * height }} />
        </>
      )}
    </div>
  );
}
render(<App />, document.getElementById("app")!);
