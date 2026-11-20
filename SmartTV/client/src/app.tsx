import { animateNavigation } from "./navigation-motion";
import appearance from "./shell-appearance";
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
import playbackLanguages from "../../../shared/playback-languages.json" with { type: "json" };
import settingsSections from "../../../shared/settings-sections.json" with { type: "json" };
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

function RailIcon({kind}: {kind: "settings" | "refresh"}) {
  return <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    {kind === "settings" ? <>
      <path d="M10 2h4l.6 3 2 .9 2.6-1.7 2.8 2.8-1.7 2.6.9 2 3 .6v4l-3 .6-.9 2 1.7 2.6-2.8 2.8-2.6-1.7-2 .9-.6 3h-4l-.6-3-2-.9-2.6 1.7L2 21.4l1.7-2.6-.9-2-3-.6v-4l3-.6.9-2L2 7l2.8-2.8 2.6 1.7 2-.9z" transform="translate(2 1) scale(.83)" />
      <circle cx="12" cy="13" r="3" />
    </> : <>
      <path d="M20 8a8 8 0 1 0-.8 9.1M20 3v5h-5" />
    </>}
  </svg>;
}

const nav = [
  ["home", "⌂", "Startseite", "Home"],
  ["live", "▣", "Live TV", "Live TV"],
  ["vod", "▷", "Filme", "Movies"],
  ["series", "▤", "Serien", "Series"],
  ["history", "↻", "Fortsetzen", "Continue watching"],
  ["favorites", "☆", "Favoriten", "Favorites"],
  ["search", "⌕", "Suche", "Search"],
  ["settings", "⚙", "Einstellungen", "Settings"],
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
const clock = (n: number, format: Settings["clockFormat"]) => {
  const date = new Date(n * 1000);
  if (!Number.isFinite(date.getTime())) return "–";
  const hours = date.getHours();
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return format === "H12"
    ? `${String(hours % 12 || 12).padStart(2, "0")}:${minutes} ${hours < 12 ? "AM" : "PM"}`
    : `${String(hours).padStart(2, "0")}:${minutes}`;
};
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
  const [settingsSection, setSettingsSection] = useState<string | null>("general");
  const t = (de: string, en: string) => saved.settings.language === "en" ? en : de;
  const navCollapsed = useRef(false);
  const navHover = useRef(false);
  const navCloseTimer = useRef<ReturnType<typeof setTimeout>>();
  const stopNavCloseTimer = () => clearTimeout(navCloseTimer.current);
  const updateNav = () => {
    const collapsed = navCollapsed.current && !navHover.current;
    void animateNavigation(document.querySelector<HTMLElement>(".shell"), collapsed).then(() => {
      playerRef.current?.position();
    });
  };
  const setNavCollapsed = (collapsed: boolean) => {
    if (navCollapsed.current === collapsed && !navHover.current) return;
    stopNavCloseTimer();
    navHover.current = false;
    navCollapsed.current = collapsed;
    updateNav();
  };
  const enterNav = (event: PointerEvent) => {
    if (event.pointerType !== "mouse" || !navCollapsed.current) return;
    stopNavCloseTimer();
    if (navHover.current) return;
    navHover.current = true;
    updateNav();
  };
  const leaveNav = (event: PointerEvent) => {
    if (event.pointerType !== "mouse" || !navHover.current) return;
    stopNavCloseTimer();
    navCloseTimer.current = setTimeout(() => {
      navHover.current = false;
      updateNav();
    }, appearance.navigationHoverCloseMs);
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
  const latestLive = useRef<Item | null>(null);
  const [selectedTitle, setSelectedTitle] = useState<Item | null>(null);
  const [epg, setEpg] = useState<Record<string, Epg[]>>({}),
    [resume, setResume] = useState<{ item: Item; position: number } | null>(
      null,
    ),
    [exit, setExit] = useState(false),
    [tracks, setTracks] = useState(false),
    [playerOptions, setPlayerOptions] = useState<"options" | "aspect" | null>(null),
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
    playerOptionsRef = useRef(playerOptions),
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
  playerOptionsRef.current = playerOptions;
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
  const playerOrigin = useRef<HTMLElement | null>(null);
  const playerReturn = useRef<Item | null>(null);
  const openFullscreen = () => {
    if (!fullRef.current) {
      const target = document.activeElement;
      playerOrigin.current = target instanceof HTMLElement && target.closest(".shell")
        ? target
        : null;
    }
    setFull(true);
  };
  const stop = () => {
    loadToken.current++;
    record();
    playerRef.current?.stop();
    previewRef.current = null;
    setPreview(null);
    setFull(false);
    setTracks(false);
  };
  const leavePlayer = (keepLivePreview = true) => {
    playerReturn.current = previewRef.current;
    if (keepLivePreview && view === "live" && previewRef.current?.kind === "live") {
      setFull(false);
    } else {
      stop();
    }
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
    latestLive.current = null;
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
  const [refreshing, setRefreshing] = useState(false);
  const refreshRequest = useRef<number | null>(null);
  async function refreshCatalog() {
    if (!session || busy || refreshRequest.current === generation.current) return;
    const request = generation.current;
    refreshRequest.current = request;
    setRefreshing(true);
    setMessage("");
    try {
      const provider = new Provider(session, savedRef.current.settings);
      const library = await provider.catalog(catalog);
      if (request !== generation.current) return;
      generation.current++;
      detailRequest.current++;
      detailCache.current.clear();
      epgPending.current.clear();
      providerRef.current = provider;
      setCatalog(library);
      setEpg({});
      if (previewRef.current?.kind === "live") void getEpg(previewRef.current);
      if (selectedTitle?.kind === "series") void openDetail(selectedTitle);
      setMessage(library.issues.length
        ? t("Liste teilweise aktualisiert.", "Library partially refreshed.")
        : t("Liste aktualisiert.", "Library refreshed."));
    } catch (error) {
      if (request === generation.current) {
        setMessage(t("Liste konnte nicht aktualisiert werden. Die bisherigen Inhalte sind weiter verfügbar.",
          "The library could not be refreshed. Existing titles are still available."));
      }
    } finally {
      if (refreshRequest.current === request) {
        refreshRequest.current = null;
        setRefreshing(false);
      }
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
      openFullscreen();
      return;
    }
    record();
    if (item.kind === "live" && latestLive.current?.id !== item.id) {
      previousLive.current = latestLive.current;
      latestLive.current = item;
    }
    setPreview(item);
    previewRef.current = item;
    if (fullscreen) openFullscreen();
    else setFull(false);
    if (view !== "vod" && view !== "series") setDetail(null);
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
    if (playerOptionsRef.current) {
      setPlayerOptions(null);
      document.querySelector<HTMLElement>(".player-options-toggle")?.focus();
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
      leavePlayer();
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
  const stepLive = (delta: number) => {
    const list = view === "live" ? filteredLive : catalog.live;
    if (!list.length) return;
    const index = list.findIndex((item) => item.id === previewRef.current?.id);
    const next = index < 0 ? (delta < 0 ? list.length - 1 : 0) : (index + delta + list.length) % list.length;
    void start(list[next], fullRef.current);
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
    const livePlayer = fullRef.current && previewRef.current?.kind === "live";
    const editingText = (e.target as HTMLElement | null)?.matches(
      'input:not([type="checkbox"]):not([type="radio"]),textarea,[contenteditable="true"]',
    );
    if ((livePlayer || view === "live") && !tracksRef.current && !playerOptionsRef.current && !parental.open
      && !document.querySelector(".modal") && !editingText
      && (e.key === "0" || e.code === "Numpad0" || code === 48 || code === 96)) {
      e.preventDefault();
      if (e.repeat) return;
      const item = previousLive.current || (!previewRef.current ? latestLive.current : null);
      if (item && item.id !== previewRef.current?.id) {
        showControls();
        void start(item, livePlayer);
      }
      return;
    }
    if (livePlayer && !tracksRef.current && !playerOptionsRef.current && !parental.open
      && !document.querySelector(".modal") && e.key.startsWith("Arrow")) {
      e.preventDefault();
      showControls();
      if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        stepLive(e.key === "ArrowUp" ? 1 : -1);
      } else {
        safe(() => playerRef.current?.seek(e.key === "ArrowLeft" ? -policy.seekStepSeconds : policy.seekStepSeconds));
      }
      return;
    }
    if (fullRef.current && previewRef.current?.kind !== "live" && !tracksRef.current && !playerOptionsRef.current && !document.querySelector(".modal")) {
      if (!controlsRef.current) playerMenuNavigation.current = false;
      if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        playerMenuNavigation.current = true;
      } else if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && !playerMenuNavigation.current) {
        e.preventDefault();
        safe(() => playerRef.current?.seek(e.key === "ArrowLeft" ? -policy.seekStepSeconds : policy.seekStepSeconds));
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
      if (code === 413) leavePlayer(false);
      else if (code === 417) safe(() => p?.seek(policy.seekStepSeconds));
      else if (code === 412) safe(() => p?.seek(-policy.seekStepSeconds));
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
          document.querySelector(".player-options") ||
          document.querySelector(".fullscreen") ||
          document,
      );
    }
  };
  useEffect(() => {
    registerKeys();
    resizeCanvas();
    const p = new Player(
      { ...savedRef.current.settings },
      () => repaint((v) => v + 1),
      () => surface.current,
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
      stopNavCloseTimer();
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
    playerRef.current?.position();
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
  const returnChannel = !full && view === "live" && playerReturn.current?.kind === "live"
    ? filteredLive.findIndex(i => i.id === playerReturn.current?.id)
    : -1;
  useLayoutEffect(() => {
    if (full || !playerReturn.current) return;
    const item = playerReturn.current;
    const origin = playerOrigin.current;
    playerReturn.current = null;
    playerOrigin.current = null;
    if (returnChannel >= 0) return;
    const key = itemKey(item);
    const series = item.seriesId ? `series:${item.seriesId}` : null;
    const targets = [...document.querySelectorAll<HTMLElement>(".shell [data-focus-key]")];
    const target = targets.find(el => el.dataset.focusKey === key) ||
      targets.find(el => el.dataset.focusKey === series) ||
      (origin?.isConnected ? origin : null);
    target?.focus({ preventScroll: true });
  }, [full]);
  const p = playerRef.current,
    status = p?.status;
  useEffect(() => {
    showControls();
    if (!full) {
      setPlayerOptions(null);
      return;
    }
    const timer = setInterval(() => {
      if (
        !tracksRef.current &&
        !playerOptionsRef.current &&
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
  useEffect(() => {
    if (playerOptions) {
      const popup = document.querySelector(".player-options");
      (popup?.querySelector<HTMLElement>('button[aria-pressed="true"]') ??
        popup?.querySelector<HTMLElement>("button"))?.focus();
    }
  }, [playerOptions]);
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
      data-focus-key={itemKey(item)}
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
    label?: string,
  ) => <TVSelect value={value} onChange={onChange} options={options} label={label} />;
  const [customLanguage, setCustomLanguage] = useState<"audioLanguage" | "subtitleLanguage" | null>(null);
  const playbackLanguage = (key: "audioLanguage" | "subtitleLanguage", label: string) => {
    const value = saved.settings[key];
    const custom = customLanguage === key || !playbackLanguages.some((option) => option.code === value);
    const options: [string, string][] = playbackLanguages.map((option) => [option.code, option[saved.settings.language]]);
    options.push(["custom", t("Andere …", "Other …")]);
    return <>
      <Field label={label}>
        {choose(custom ? "custom" : value, (next) => {
          setCustomLanguage(next === "custom" ? key : null);
          if (next !== "custom") setting(key, next);
        }, options, label)}
      </Field>
      {custom && <Field label={t("Eigener Sprachcode", "Custom language code")}>
        <TVInput value={value} placeholder="ISO 639" onInput={(event) => setting(key, event.currentTarget.value)} />
      </Field>}
    </>;
  };
  useEffect(() => { document.documentElement.lang = saved.settings.language; }, [saved.settings.language]);
  const settingsView = () => {
    const active = settingsSections.find((section) => section.id === settingsSection);
    const renderSection = (id: string) => {
      switch (id) {
        case "general": return <>
<Field label={t("App-Sprache", "App language")}>
            {choose(saved.settings.language, (value) => setting("language", value as Settings["language"]), [["de", "Deutsch"], ["en", "English"]], t("App-Sprache", "App language"))}
          </Field>
          <Field label={t("Zeitformat", "Time format")}>
            {choose(saved.settings.clockFormat, (value) => setting("clockFormat", value as Settings["clockFormat"]), [["H24", "24 h · 18:30"], ["H12", "12 h · 06:30 PM"]], t("Zeitformat", "Time format"))}
          </Field>
          <Field label={t("Farbschema", "Appearance")}>
            {choose(
              saved.settings.theme,
              (v) => setting("theme", v as Settings["theme"]),
              [
                ["warm", t("Blau", "Blue")],
                ["dark", t("Dunkel", "Dark")],
                ["light", t("Hell", "Light")],
                ["system", "System"],
              ],
            )}
          </Field>
          <Field label={t("Senderdichte", "Channel density")}>
            {choose(
              saved.settings.density,
              (v) => setting("density", v as Settings["density"]),
              [
                ["compact", "Kompakt · 10 Sender"],
                ["comfortable", "Komfortabel · 8 Sender"],
              ],
            )}
          </Field>

        </>;
        case "playback": return <>
          <Field label={t("Bildformat", "Aspect ratio")}>
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
          <Field label={t("Letzten Titel beim Start abspielen", "Open the last channel at startup")}>
            <TVInput
              type="checkbox"
              checked={saved.settings.autoplayLast}
              onChange={(e) => setting("autoplayLast", e.currentTarget.checked)}
            />
          </Field>
        </>;
        case "audio": return <>
          {playbackLanguage("audioLanguage", t("Bevorzugte Audiosprache", "Preferred audio language"))}
          {playbackLanguage("subtitleLanguage", t("Untertitelsprache", "Subtitle language"))}
          <Field label={t("Untertitel automatisch", "Enable subtitles automatically")}>
            <TVInput
              type="checkbox"
              checked={saved.settings.subtitles}
              onChange={(e) => setting("subtitles", e.currentTarget.checked)}
            />
          </Field>
          <Field label={t("Untertitelgröße", "Subtitle size")}>
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
        </>;
        case "profiles": return <>
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

          <ConnectPanel importProfiles={importCloudProfiles} />
        </>;
        case "parental": return <>
          <p>{parental.enabled?'Aktiv · gesperrte Inhalte benötigen die Eltern-PIN':'Vierstellige PIN für Sender, Filme und Serien einrichten'}</p>
          <button onClick={()=>{stop();parental.manage();}}>{parental.enabled?'Elternsperre verwalten':'Eltern-PIN einrichten'}</button>
        </>;
        case "updates": return <>
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
        </>;
        case "license": return <>
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
        </>;
        case "advanced": return <>
          <Field label={t("Expertenmodus", "Expert mode")}>
            <TVInput type="checkbox" checked={saved.settings.expertMode}
              onChange={(e) => setting("expertMode", e.currentTarget.checked)} />
          </Field>

          {saved.settings.expertMode && <>

          <Field label={t("Wiedergabe-Engine", "Playback engine")}>
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
          <Field label={t("Live-Streamformat", "Live stream format")}>
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

          <Field label={t("Stream-Puffer", "Stream buffer")}>
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
          <Field label={t("Zeitlimit", "Timeout")}>
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
          <Field label={t("Player User-Agent", "Player user agent")}>
            <TVInput
              value={saved.settings.userAgent}
              onInput={(e) => setting("userAgent", e.currentTarget.value)}
              placeholder="Standard des Fernsehers"
            />
          </Field>

          <Field label={t("Anbieterzugriff", "Provider access")}>
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
              <Field label={t("Companion-Adresse", "Companion address")}>
                <TVInput
                  value={saved.settings.companionUrl}
                  onInput={(e) =>
                    setting("companionUrl", e.currentTarget.value)
                  }
                  placeholder="http://192.168.178.x:18498"
                />
              </Field>
              <Field label={t("Companion-Zugangsschlüssel", "Companion access key")}>
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
          <button onClick={() => void refreshCatalog()} disabled={refreshing}>
            Bibliothek neu laden
          </button>
          </>}
        </>;
        default: return null;
      }
    };
    return (
      <div class="settings-page">
        <h1>{t("Einstellungen", "Settings")}</h1>
        <div class="settings-grid">
          <nav class="settings-sections" data-focus-zone="settings-sections" aria-label={t("Bereiche", "Sections")}>
            {settingsSections.map((section) => (
              <button key={section.id} data-focus-key={section.id}
                aria-expanded={settingsSection === section.id} aria-controls="settings-options"
                class={settingsSection === section.id ? "selected" : ""}
                onClick={() => setSettingsSection(section.id)}>
                <span>{section[saved.settings.language]}</span><span aria-hidden="true">›</span>
              </button>
            ))}
          </nav>
          {active && <section key={active.id} id="settings-options" class="glass settings-options"
              data-focus-zone="settings-options" data-settings-section={active.id}
              data-license-status={active.id === "license" ? "" : undefined}>
            <h2>{active[saved.settings.language]}</h2>
            {renderSection(active.id)}
          </section>}
        </div>
      </div>
    );
  };
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
            class={`shell ${navCollapsed.current && !navHover.current ? "nav-collapsed" : ""} ${view === "live" ? "live-shell" : ""} ${view === "live" ? "with-preview" : ""}`}
          >
            <aside
              class="navigation"
              data-focus-zone="navigation"
              onPointerEnter={enterNav}
              onPointerLeave={leaveNav}
              onClickCapture={(event) => {
                if ((event.target as Element).closest("button")) setNavCollapsed(false);
              }}
            >
              <div class="brand">
                <><img class="brand-normal" src="d7.svg" /><img class="brand-dark" src="d7-dark.svg" /></>
                <div class="brand-copy"><strong>Dado Player</strong>
                  {license.claims?.kind==='trial' && <small class="trial-badge" data-trial-badge>Testversion · {license.text.replace(' Testzeit','').replace('Noch','noch')}</small>}
                </div>
              </div>
              {license.claims?.kind==='trial' && <small class="trial-badge-compact" title={license.text}>Trial · {license.text.match(/\d+/)?.[0]} T</small>}
              <nav>
                {nav.map(([id, icon, de, en]) => (
                  <button
                    aria-label={`${icon} ${t(de, en)}`}
                    title={t(de, en)}
                    data-focus-key={id}
                    class={view === id ? "selected" : ""}
                    onClick={() => go(id)}
                  >
                    <span>{id === "settings" ? <RailIcon kind="settings" /> : icon}</span>
                    <span class="nav-label">{t(de, en)}</span>
                  </button>
                ))}
                <button class="refresh-library" data-focus-key="refresh-library"
                  aria-label={t("Liste aktualisieren", "Refresh library")}
                  title={t("Liste aktualisieren", "Refresh library")}
                  aria-busy={refreshing} onClick={() => void refreshCatalog()}>
                  <span><RailIcon kind="refresh" /></span>
                  <span class="nav-label">{refreshing ? t("Wird aktualisiert …", "Refreshing …") : t("Liste aktualisieren", "Refresh library")}</span>
                </button>
              </nav>
              <footer>
                <span>{session?.name}</span>
                <small>Tizen {APP_VERSION}</small>
              </footer>
            </aside>
            {view === "live" ? (
              <>
                <section class="categories" data-focus-zone="categories">
                  <h1>{t("Kategorien", "Categories")}</h1>
                  <div class="category-list">
                    <button
                      class={category === "all" ? "selected" : ""}
                      onClick={() => {
                        setCategory("all");
                      }}
                    >
                      Alle Sender
                    </button>
                    <button
                      class={category === "favorites" ? "selected" : ""}
                      onClick={() => {
                        setCategory("favorites");
                      }}
                    >
                      Favoriten
                    </button>
                    {catalog.categories.live.map((c) => (
                      <button
                        class={category === c.id ? "selected" : ""}
                        onClick={() => {
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
                    placeholder={t("Sender suchen", "Find a channel")}
                    aria-label="Sender suchen"
                    value={query}
                    onInput={(e) => setQuery(e.currentTarget.value)}
                  />
                  <VirtualChannels
                    items={filteredLive}
                    row={channelRow}
                    epg={getEpg}
                    density={saved.settings.density}
                    returnIndex={returnChannel}
                  />
                </section>
                {!preview && (
                  <aside class="preview-panel" data-focus-zone="details">
                    <header>
                      <h2>{t("Vorschau", "Preview")}</h2>
                    </header>
                    <div class="preview-surface empty-preview">
                      <span>▷</span>
                    </div>
                    <h2>{t("Sender auswählen", "Choose a channel")}</h2>
                    <p>
                      Hier erscheinen Vorschau, Programm und
                      Senderinformationen.
                    </p>
                  </aside>
                )}
                {preview && (
                  <aside class="preview-panel" data-focus-zone="details">
                    <header>
                      <h2>{t("Vorschau", "Preview")}</h2>
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
                    <button class="preview-open" onClick={openFullscreen}>
                      Vollbild öffnen <span>↗</span>
                    </button>
                    <PlayerFeedback status={status} />
                    <h2>{preview.name}</h2>
                    <div class="epg-detail" tabIndex={0}>
                      {now ? (
                        <>
                          <small>
                            {t("Jetzt", "Now")} · {clock(now.start, saved.settings.clockFormat)} — {clock(now.end, saved.settings.clockFormat)}
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
                            {t("Danach", "Next")} · {clock(next.start, saved.settings.clockFormat)} — {clock(next.end, saved.settings.clockFormat)}
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
                    <h1>{t("Fortsetzen", "Continue watching")}</h1>
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
                    <h1>{t("Favoriten", "Favorites")}</h1>
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
                    <button onClick={() => void refreshCatalog()} disabled={refreshing}>
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
                <button onClick={() => leavePlayer()}>{t("← Zurück", "← Back")}</button>
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
                <button aria-label={t("Favorit", "Favorite")} aria-pressed={isFavorite(preview)}
                  onClick={() => toggleFavorite(preview)}>
                  {isFavorite(preview) ? "★" : "☆"}
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
              {playerOptions === "options" && (
                <div class="player-options glass" aria-label={t("Player-Optionen", "Player options")}>
                  <button onClick={() => {
                    setPlayerOptions(null);
                    p?.refreshTracks();
                    setTracks(true);
                  }}>{t("Ton & Untertitel", "Audio & subtitles")}</button>
                  <button onClick={() => setPlayerOptions("aspect")}>{t("Bildformat", "Aspect ratio")}</button>
                  <button onClick={() => leavePlayer(false)}>{t("Beenden", "Stop")}</button>
                </div>
              )}
              {playerOptions === "aspect" && (
                <div class="player-options glass" aria-label={t("Bildformat", "Aspect ratio")}>
                  {([
                    ["fill", t("Automatisch", "Automatic")],
                    ["fit", t("Einpassen", "Fit")],
                    ["stretch", t("Strecken", "Stretch")],
                  ] as const).map(([value, label]) => (
                    <button key={value} aria-pressed={saved.settings.aspect === value} onClick={() => {
                      setting("aspect", value);
                      p?.setAspect(value);
                      setPlayerOptions(null);
                      document.querySelector<HTMLElement>(".player-options-toggle")?.focus();
                    }}>{label}</button>
                  ))}
                </div>
              )}
              <div class="player-controls glass">
                <div class="transport">
                  <button onClick={() => preview.kind === "live" ? stepLive(-1) : safe(() => p?.seek(-policy.seekStepSeconds))}>
                    {preview.kind === "live" ? t("‹ Sender", "‹ Channel") : "↶ " + policy.seekStepSeconds + " s"}
                  </button>
                  <button class="primary" onClick={() => p?.toggle()}>
                    {status?.state === "paused" ? t("▷ Abspielen", "▷ Play") : t("Ⅱ Pause", "Ⅱ Pause")}
                  </button>
                  <button onClick={() => preview.kind === "live" ? stepLive(1) : safe(() => p?.seek(policy.seekStepSeconds))}>
                    {preview.kind === "live" ? t("Sender ›", "Channel ›") : policy.seekStepSeconds + " s ↷"}
                  </button>
                  <button class="player-options-toggle" aria-expanded={!!playerOptions}
                    onClick={() => setPlayerOptions(playerOptions ? null : "options")}>
                    {t("Optionen", "Options")}
                  </button>
                </div>
                <div class="timeline">
                  <span>
                    {preview.kind === "live"
                      ? "● LIVE"
                      : time(status?.time || 0)}
                  </span>
                  {preview.kind === "live" ? (
                    <progress value={now ? Math.max(0, Date.now() / 1000 - now.start) : 0}
                      max={now ? Math.max(1, now.end - now.start) : 1} />
                  ) : (
                    <div class="seek-bar" onClick={(event) => {
                      const duration = status?.duration || 0;
                      const bounds = event.currentTarget.getBoundingClientRect();
                      if (duration > 0 && bounds.width > 0) {
                        safe(() => p?.seekTo(((event.clientX - bounds.left) / bounds.width) * duration));
                      }
                    }}>
                      <progress value={status?.time || 0} max={status?.duration || 1} />
                    </div>
                  )}
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
  returnIndex,
}: {
  items: Item[];
  row: (i: Item) => ComponentChildren;
  epg: (i: Item) => Promise<void>;
  density: Settings["density"];
  returnIndex: number;
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
  useLayoutEffect(() => {
    if (returnIndex < 0 || !ref.current) return;
    const nextTop = Math.max(0, returnIndex * height - ref.current.clientHeight / 2);
    ref.current.scrollTop = nextTop;
    pendingFocus.current = returnIndex;
    setTop(ref.current.scrollTop);
    setFocusIndex(returnIndex);
  }, [returnIndex, height]);
  const visibleStart = Math.max(0, Math.floor(top / height) - 5);
  const start = focusIndex === null ? visibleStart : Math.max(0, Math.min(visibleStart, focusIndex-2), focusIndex-32),
    end = Math.min(items.length, Math.max(start + 24, (focusIndex ?? start) + 3));
  useLayoutEffect(() => {
    if (pendingFocus.current === null) return;
    const target = ref.current?.querySelector<HTMLElement>(`[data-list-index="${pendingFocus.current}"] button`);
    if (target) { pendingFocus.current=null; target.focus({preventScroll:true}); scrollFocused(target); }
  }, [focusIndex, start, returnIndex]);
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
