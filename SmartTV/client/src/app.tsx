import { animateNavigation, animatePage } from "./navigation-motion";
import { cancelScroll } from "./scroll-motion";
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
  resumeHistory,
} from "./types";
import { load, save, favorite, remember } from "./store";
import { Provider, hostUrl, httpUrl, liveExtension } from "./api";
import { Player } from "./player";
import { TVSelect } from "./select";
import { TVInput } from "./input";
import { CatalogBrowser, TitleDetail, resumeEpisode } from "./library";
import playbackLanguages from "../../../shared/playback-languages.json" with { type: "json" };
import settingsSections from "../../../shared/settings-sections.json" with { type: "json" };
import policy from '../../../shared/product-policy.json' with { type: 'json' };
import releaseInfo from '../../../release.json' with { type: 'json' };
import { Home } from "./home";
import { FavoriteMark } from "./favorite-mark";
import {licenseClient} from './licensing';
import {ConnectPanel,mergeCloudProfiles,prepareCloudDevice} from './connect';
import { accessStorage, readSlot, saveSlot } from './update-format';
import { stageUpdate } from './update-client';
import { UpdateTokenTransfer } from './update-transfer';
import packageInfo from "../package.json";
const APP_VERSION = window.dadoTvBoot?.running.version || packageInfo.version;
const APP_BUILD = window.dadoTvBoot?.running.build || releaseInfo.tizenBuild;
import {
  exitApp,
  rememberFocus,
  resetFocusMemory,
  focusables,
  moveFocus,
  focusNavigation,
  isInfoKey,
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

type RefreshStatus = "idle" | "loading" | "success" | "error";

function ShortcutLegend({t, labels, onAction, marked}: {
  t: (de: string, en: string) => string;
  labels?: readonly string[];
  onAction?: (key: string) => void;
  marked?: boolean;
}) {
  return (
    <ul class="shortcut-legend" aria-label={t("Farbtasten", "Color buttons")}>
      {appearance.remoteShortcuts.map(({key, color, de, en}, index) => {
        const label = labels?.[index] ?? (onAction && key === "F2" ? t("Senderliste", "Channel list")
          : onAction && key === "F4" ? t("Bildformat", "Aspect ratio")
          : t(de, en));
        const dot = <span class="shortcut-dot" style={{backgroundColor: color}} aria-hidden="true" />;
        return <li key={key}>{onAction ? (
          <button type="button"
            aria-pressed={key === "F3" ? marked : undefined} onClick={() => onAction(key)}>
            {dot}{label}
          </button>
        ) : <>{dot}{label}</>}</li>;
      })}
    </ul>
  );
}

function RailIcon({kind}: {kind: "settings" | "refresh" | "success" | "error"}) {
  return <svg class={kind} aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    {kind === "settings" ? <>
      <path d="M10 2h4l.6 3 2 .9 2.6-1.7 2.8 2.8-1.7 2.6.9 2 3 .6v4l-3 .6-.9 2 1.7 2.6-2.8 2.8-2.6-1.7-2 .9-.6 3h-4l-.6-3-2-.9-2.6 1.7L2 21.4l1.7-2.6-.9-2-3-.6v-4l3-.6.9-2L2 7l2.8-2.8 2.6 1.7 2-.9z" transform="translate(2 1) scale(.83)" />
      <circle cx="12" cy="13" r="3" />
    </> : kind === "refresh" ? <path d="M20 8a8 8 0 1 0-.8 9.1M20 3v5h-5" />
      : kind === "success" ? <path d="m5 12 4.5 4.5L19 7" />
      : <path d="m6 6 12 12M18 6 6 18" />}
  </svg>;
}

function SettingsTileIcon({section}: {section: string}) {
  const shapes: Record<string, string> = {
    stream: "M3 5h18v13H3zM9 22h6m-3-4v4m-3-13 6 3-6 3z",
    clock: "M12 3a9 9 0 1 0 0 18 9 9 0 1 0 0-18zm0 4v5l4 2",
    player: "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zm5 5 6 4-6 4z",
    playback: "M5 12a7 7 0 1 0 2-5M5 4v5h5m1 0 5 3-5 3z",
    audio: "M3 9h4l5-4v14l-5-4H3zm12-1a5 5 0 0 1 0 8m3-11a9 9 0 0 1 0 14",
    profiles: "M12 4a4 4 0 1 0 0 8 4 4 0 1 0 0-8zM4 21a8 6 0 0 1 16 0z",
    parental: "m12 2 8 4v6c0 5-4 8-8 10-4-2-8-5-8-10V6zm-4 10 3 3 5-5",
    cache: "M5 6h14m-12 0 1 15h8l1-15m-7 0V3h4v3m-4 4v7m4-7v7",
    updates: "M20 9a8 8 0 1 0-1 8m1-13v5h-5",
    license: "M5 3h14v18H5zm3 6h8m-8 4h8m-8 4h5",
    advanced: "M5 4v16m7-16v16m7-16v16M2 9h6m1 7h6m1-7h6",
  };
  const shape = shapes[section];
  return shape ? <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" stroke-linecap="round"><path d={shape} /></svg>
    : <RailIcon kind="settings" />;
}

function LibraryIcon({kind}: {kind: "live" | "vod" | "series"}) {
  return <svg class="classic-media-icon" viewBox="0 0 48 48" aria-hidden="true" fill="none"
    stroke="currentColor" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round">
    {kind === "live" ? <>
      <rect x="4" y="10" width="40" height="26" rx="6" />
      <path d="M15 43h18m-9-7v7m-12-19h7l4-7 4 13 4-6h5" />
    </> : kind === "vod" ? <>
      <rect x="5" y="5" width="38" height="38" rx="8" />
      <path d="M11 13h5m-5 7h5m-5 8h5m-5 7h5m16-22h5m-5 7h5m-5 8h5m-5 7h5" />
      <path d="M20 17l10 7-10 7z" stroke-width="1.5" fill="currentColor" />
    </> : <>
      <path d="M12 12V9a4 4 0 0 1 4-4h24a4 4 0 0 1 4 4v20" />
      <rect x="4" y="14" width="34" height="27" rx="7" />
      <path d="M12 24h18m-18 7h13" />
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
const time = (n: number, showHours = false) =>
  Number.isFinite(n)
    ? `${showHours || n >= 3600 ? String(Math.floor(n / 3600)).padStart(showHours ? 2 : 1, "0") + ":" : ""}${String(Math.floor((n % 3600) / 60)).padStart(2, "0")}:${String(Math.floor(n % 60)).padStart(2, "0")}`
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
  useEffect(() => window.dadoTvBoot?.ready(), []);
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
  const [settingsSection, setSettingsSection] = useState<string | null>(null);
  const displayedPage = useRef<string>();
  useLayoutEffect(() => {
    const previous = displayedPage.current;
    const current = session ? `${view}:${view === "settings" ? settingsSection ?? "" : ""}` : undefined;
    displayedPage.current = current;
    if (!previous || !current || previous === current) return;
    const animations = animatePage(document.querySelector(".shell"), view,
      view === "home" || (view === "settings" && !settingsSection && previous.startsWith("settings:")));
    return () => animations.forEach((animation) => animation.cancel());
  }, [view, settingsSection, session?.id]);
  const [settingsDraft, setSettingsDraft] = useState<string | null>(null);
  const [cacheCleared, setCacheCleared] = useState(false);
  const epgCacheEpoch = useRef(0);
  const closeSettingsSection = () => {
    if (!settingsSection) return false;
    const key = settingsSection;
    setSettingsSection(null);
    setSettingsDraft(null);
    requestAnimationFrame(() => document.querySelector<HTMLElement>(`.settings-sections [data-focus-key="${key}"]`)?.focus());
    return true;
  };
  const t = (de: string, en: string) => saved.settings.language === "en" ? en : de;
  const classic = saved.settings.interfaceLayout === "classic";
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
    if (event.pointerType !== "mouse" || !navCollapsed.current || restoringPosterFocus.current) return;
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
  const restoringPosterFocus = useRef(false);
  const catalogOrigin = useRef<{titleTop: number; categoryTop: number} | null>(null);
  const [epg, setEpg] = useState<Record<string, Epg[]>>({}),
    [resume, setResume] = useState<{ item: Item; position: number } | null>(
      null,
    ),
    [exit, setExit] = useState(false),
    [tracks, setTracks] = useState(false),
    [playerOptions, setPlayerOptions] = useState(false),
    [aspectNotice, setAspectNotice] = useState<{value: Settings["aspect"]} | null>(null),
    [controlsVisible, setControlsVisible] = useState(true),
    [update, setUpdate] = useState(""),
    [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateToken, setUpdateToken] = useState(() => {
    try { return localStorage.getItem(accessStorage) || ''; } catch { return ''; }
  });
  const [updateReady, setUpdateReady] = useState<'manual' | 'automatic' | null>(null);
  const updateInProgress = useRef(false);
  const aspectNoticeTimer = useRef<ReturnType<typeof setTimeout>>();
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
        (saved.settings.watchHistory ? saved.history[pid] || EMPTY_HISTORY : EMPTY_HISTORY).map((h) => {
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
      [saved.history, pid, catalog, saved.settings.watchHistory],
    );
  const favoriteTarget = (i: Item) =>
    i.kind === "episode"
      ? catalog.series.find((s) => s.id === i.seriesId) || i
      : i;
  const favoriteKeys = useMemo(() => new Set(favorites.map(itemKey)), [favorites]);
  const allowEpisodePreviews = (item: Item) => !full && !parental.open && !resume && !parental.locked(pid, item);
  const isFavorite = (i: Item) => favoriteKeys.has(itemKey(favoriteTarget(i)));
  const toggleFavorite = (i: Item) =>
    changed({
      ...savedRef.current,
      favorites: {
        ...savedRef.current.favorites,
        [pid]: favorite(savedRef.current, pid, favoriteTarget(i)),
      },
    });
  const untrackedPlayback = useRef(false);
  const record = () => {
    if (untrackedPlayback.current || !savedRef.current.settings.watchHistory) return;
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
  const returnFromRandom = useRef(false);
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
    untrackedPlayback.current = false;
    previewRef.current = null;
    setPreview(null);
    setFull(false);
    setTracks(false);
  };
  const leavePlayer = (keepLivePreview = true, openLiveList = false) => {
    playerReturn.current = previewRef.current;
    returnFromRandom.current = untrackedPlayback.current;
    if (keepLivePreview && (view === "live" || openLiveList) && previewRef.current?.kind === "live") {
      if (openLiveList) {
        if (view !== "live" || !filteredLive.some((item) => item.id === previewRef.current?.id)) {
          setCategory("all");
          setQuery("");
        }
        setView("live");
      }
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
  const normalizedProfile = (profile: Profile) => {
      const p = { ...profile };
      if (p.kind === "Xtream") {
        p.host = hostUrl(p.host);
        if (!p.username || !p.password)
          throw new Error("Benutzername und Passwort fehlen.");
      } else {
        p.m3uUrl = httpUrl(p.m3uUrl);
        if (p.epgUrl) p.epgUrl = httpUrl(p.epgUrl);
      }
      if (!p.name.trim()) throw new Error("Profilname fehlt.");
      return {...p, name: p.name.trim()};
  };
  const storeProfile = (profiles: Profile[], profile: Profile) =>
    profiles.some(item => item.id === profile.id)
      ? profiles.map(item => item.id === profile.id ? profile : item)
      : [...profiles, profile];
  const addProfile = () => {
    try {
      const profile = normalizedProfile(form);
      const next = {...savedRef.current, profiles: storeProfile(savedRef.current.profiles, profile)};
      save(next);
      savedRef.current = next;
      setSaved(next);
      setEditing(false);
      requestAnimationFrame(() => document.querySelector<HTMLElement>(".profile-add")?.focus());
    } catch (error) {
      setMessage((error as Error).message);
    }
  };
  async function login(profile: Profile, automatic = false) {
    const gen = ++generation.current;
    setBusy(true);
    setMessage("");
    stop();
    try {
      const p = normalizedProfile(profile);
      const provider = new Provider(p, savedRef.current.settings);
      const lib = await provider.catalog();
      if (gen !== generation.current) return;
      providerRef.current = provider;
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
      setCatalog(lib);
      setSession(p);
      setEditing(false);
      setView("home");
      setSettingsSection(null);
      setCategory("all");
      setQuery("");
      changed({
        ...savedRef.current,
        profiles: storeProfile(savedRef.current.profiles, p),
        activeProfile: p.id,
        signedOut: false,
      });
      if (automatic && savedRef.current.settings.watchHistory && savedRef.current.settings.autoplayLast) {
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
        if (!session) {
          setForm(profile);
          setEditing(!automatic);
        }
      }
    } finally {
      if (gen === generation.current) setBusy(false);
    }
  }
  const [refreshStatus, setRefreshStatus] = useState<RefreshStatus>("idle");
  const refreshing = refreshStatus === "loading";
  const refreshRequest = useRef<number | null>(null);
  useEffect(() => {
    if (refreshStatus !== "success" && refreshStatus !== "error") return;
    const timer = setTimeout(() => setRefreshStatus("idle"), shellAppearance.refreshFeedbackMs);
    return () => clearTimeout(timer);
  }, [refreshStatus]);
  async function refreshCatalog() {
    if (!session || busy || refreshRequest.current === generation.current) return;
    const request = generation.current;
    refreshRequest.current = request;
    setRefreshStatus("loading");
    let outcome: RefreshStatus = "idle";
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
      outcome = library.issues.length ? "error" : "success";
    } catch {
      if (request === generation.current) outcome = "error";
    } finally {
      if (refreshRequest.current === request) {
        refreshRequest.current = null;
        setRefreshStatus(outcome);
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
    const gen = generation.current, cacheEpoch = epgCacheEpoch.current;
    try {
      const rows = await providerRef.current.epg(item);
      if (gen === generation.current && cacheEpoch === epgCacheEpoch.current)
        setEpg((old) => ({ ...old, [key]: rows }));
    } catch {
      if (gen === generation.current && cacheEpoch === epgCacheEpoch.current) setEpg((old) => ({ ...old, [key]: [] }));
    } finally {
      if (gen === generation.current && cacheEpoch === epgCacheEpoch.current) epgPending.current.delete(key);
    }
  };
  async function start(item: Item, fullscreen = false, position = 0, untracked = false) {
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
      p.current && untrackedPlayback.current === untracked
    ) {
      openFullscreen();
      return;
    }
    record();
    untrackedPlayback.current = untracked;
    if (item.kind === "live" && latestLive.current?.id !== item.id) {
      previousLive.current = latestLive.current;
      latestLive.current = item;
    }
    setPreview(item);
    previewRef.current = item;
    if (fullscreen) openFullscreen();
    else setFull(false);
    setTracks(false);
    const token = ++loadToken.current;
    p.settings = { ...savedRef.current.settings };
    void getEpg(item);
    await p.open(item, position >= policy.resumeMinimumSeconds ? position : 0);
    if (token !== loadToken.current) return;
    if (p.status.state === "playing") record();
  }
  function offerResume(item: Item, position?: number) {
    if (item.kind !== "vod" && item.kind !== "episode") return false;
    const watched = history.find((entry) => itemKey(entry.item) === itemKey(item));
    const savedPosition = position ?? watched?.position ?? 0;
    if (!Number.isFinite(savedPosition) || savedPosition < policy.resumeMinimumSeconds ||
        (watched?.duration && watched.duration - savedPosition <= policy.resumeEndMarginSeconds)) {
      return false;
    }
    setResume({ item, position: savedPosition });
    return true;
  }
  const playDirect = (item: Item, position: number, untracked = false) =>
    void start(item, true, position, untracked);
  const playTitle = (item: Item, position?: number) => {
    if (!offerResume(item, position)) void start(item, true);
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
  const externalTitle = view !== "vod" && view !== "series" && (detail?.kind === "vod" || detail?.kind === "series") ? detail : null;
  const externalTitlePanel = useRef<HTMLElement>(null);
  const externalTitleOrigin = useRef<HTMLElement | null>(null);
  const externalTitleKey = externalTitle ? itemKey(externalTitle) : null;
  useLayoutEffect(() => {
    const panel = externalTitlePanel.current;
    if (!externalTitleKey || full || !panel || resume) return;
    const active = document.activeElement;
    if (active && panel.contains(active) && active !== panel.querySelector(".menu-back")) return;
    const control = panel.querySelector<HTMLElement>("button.primary:not(:disabled)") ||
      panel.querySelector<HTMLElement>(".menu-back");
    control?.focus({preventScroll: true});
  }, [externalTitleKey, episodes, full, resume]);
  const closeExternalTitle = () => {
    const origin = externalTitleOrigin.current;
    externalTitleOrigin.current = null;
    detailRequest.current++;
    setDetail(null);
    setEpisodes([]);
    requestAnimationFrame(() => {
      if (origin?.isConnected) origin.focus({preventScroll: true});
    });
  };
  async function openDetail(item: Item, promptForResume = false) {
    if (view !== "vod" && view !== "series") {
      const focus = document.activeElement;
      externalTitleOrigin.current = focus instanceof HTMLElement ? focus : null;
    }
    const token = ++detailRequest.current,
      gen = generation.current,
      key = itemKey(item),
      cached = detailCache.current.get(key);
    detailRef.current = item;
    setDetail(cached?.item || item);
    setEpisodes(cached ? [...cached.episodes] : []);
    function offerSeriesResume(rows: Item[]) {
      if (promptForResume && item.kind === "series") {
        const target = resumeEpisode(rows, history);
        if (target) offerResume(target.item, target.position);
      }
    }
    if (promptForResume && item.kind === "vod") offerResume(item);
    if (cached) {
      offerSeriesResume(cached.episodes);
      return;
    }
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
        offerSeriesResume(rows);
      }
    } catch (e) {
      if (token === detailRequest.current) setMessage((e as Error).message);
    }
  }
  const go = (next: View) => {
    if (next === view || (next === "history" && !saved.settings.watchHistory)) return;
    if (view === "settings") { setSettingsSection(null); setSettingsDraft(null); }
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
    const previousCategory = memory?.category === "continue" && !saved.settings.watchHistory ? undefined : memory?.category;
    setCategory(previousCategory && (classic || !["recent", "continue"].includes(previousCategory)) ? previousCategory : "all");
    setQuery(memory?.query || "");
    setSelectedTitle(memory?.detail || null);
    setDetail(memory?.detail || null);
    setEpisodes(memory?.episodes || []);
    if (next === "live" || next === "vod" || next === "series") {
      requestAnimationFrame(() => {
        const content = document.querySelector<HTMLElement>(`.shell[data-focus-scope="${next}"] .category-list button`);
        content?.focus({preventScroll:true});
      });
    }
  };
  const importCloudProfiles = (profiles:Profile[]) => {
    const merged=mergeCloudProfiles(savedRef.current.profiles,profiles);
    const next={...savedRef.current,profiles:merged.profiles};
    save(next);
    savedRef.current=next;setSaved(next);
    return merged.added;
  };
  const closeCatalogDetail = () => {
    if (!selectedTitle || (view !== "vod" && view !== "series")) return false;
    const key = itemKey(selectedTitle);
    const origin = catalogOrigin.current;
    catalogOrigin.current = null;
    restoringPosterFocus.current = true;
    detailRequest.current++;
    setSelectedTitle(null);
    setDetail(null);
    setEpisodes([]);
    requestAnimationFrame(() => {
      const shell = document.querySelector(`.shell[data-focus-scope="${view}"]`);
      if (!shell) {
        restoringPosterFocus.current = false;
        return;
      }
      if (origin) {
        for (const [selector, top] of [
          [".catalog-title-list", origin.titleTop], [".category-list", origin.categoryTop],
        ] as const) {
          const pane = shell.querySelector<HTMLElement>(selector);
          if (!pane) continue;
          cancelScroll(pane);
          pane.scrollTop = top;
          pane.dispatchEvent(new Event("scroll"));
        }
      }
      requestAnimationFrame(() => {
        try {
          Array.from(shell.querySelectorAll<HTMLElement>(".catalog-poster"))
            .find((element) => element.dataset.focusKey === key)?.focus({preventScroll: true});
        } finally {
          restoringPosterFocus.current = false;
        }
      });
    });
    return true;
  };
  const back = () => {
    if(parental.open){parental.close();return;}
    if (tracksRef.current) {
      setTracks(false);
      return;
    }
    if (playerOptionsRef.current) {
      setPlayerOptions(false);
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
    if (fullRef.current && !document.querySelector(".modal")) {
      leavePlayer();
      return;
    }
    if (previewRef.current && view !== "live") {
      stop();
      return;
    }
    if (editing) {
      setEditing(false);
      if (session) requestAnimationFrame(() => document.querySelector<HTMLElement>(".profile-add")?.focus());
      return;
    }
    if (externalTitle) { closeExternalTitle(); return; }
    if (view === "settings" && closeSettingsSection()) return;
    if (closeCatalogDetail()) return;
    if (classic) {
      if (view === "home") setExit(true);
      else {
        const from = view;
        go("home");
        requestAnimationFrame(() => document.querySelector<HTMLElement>(`.classic-dashboard button[data-focus-key="${from}"]`)?.focus());
      }
      return;
    }
    if (!document.activeElement?.closest(".navigation")) {
      focusNavigation();
      return;
    }
    setExit(true);
  };
  const goShortcut = (next: View) => {
    go(next);
    requestAnimationFrame(() => {
      if (next === "live" || next === "vod" || next === "series") return;
      if (next === "search") {
        document.querySelector<HTMLElement>(".header-search-field")?.focus({preventScroll:true});
        return;
      }
      if (next === "settings" && classic) {
        const first = document.querySelector<HTMLElement>(".settings-sections button, .settings-options button");
        if (first) { first.focus(); return; }
      }
      (document.querySelector<HTMLElement>(`.navigation button[data-focus-key="${next}"]`) ||
        document.querySelector<HTMLElement>(next === "home" ? ".classic-primary button" : ".classic-header button"))?.focus();
    });
  };
  const stepLive = (delta: number) => {
    const list = view === "live" ? filteredLive : catalog.live;
    if (!list.length) return;
    const index = list.findIndex((item) => item.id === previewRef.current?.id);
    const next = index < 0 ? (delta < 0 ? list.length - 1 : 0) : (index + delta + list.length) % list.length;
    void start(list[next], fullRef.current);
  };
  const applyPlayerAspect = (value: Settings["aspect"]) => {
    const changed = savedRef.current.settings.aspect !== value;
    setting("aspect", value);
    playerRef.current?.setAspect(value);
    if (changed) {
      clearTimeout(aspectNoticeTimer.current);
      setAspectNotice({value});
      aspectNoticeTimer.current = setTimeout(() => setAspectNotice(null), 1000);
    }
    showControls();
  };
  const cycleLiveAspect = () => {
    const aspect = savedRef.current.settings.aspect;
    applyPlayerAspect(aspect === "fill" ? "fit" : aspect === "fit" ? "stretch" : "fill");
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
    const ok = e.key === "Enter" || code === 13;
    if (ok && e.repeat && (livePlayer || view === "live") && !document.querySelector(".modal")) {
      e.preventDefault();
      return;
    }
    if (livePlayer && ok && !tracksRef.current && !playerOptionsRef.current && !parental.open
      && !document.querySelector(".modal")) {
      e.preventDefault();
      leavePlayer(true, true);
      return;
    }
    const editingText = (e.target as HTMLElement | null)?.matches(
      'input:not([type="checkbox"]):not([type="radio"]),textarea,[contenteditable="true"]',
    );
    const browsingCatalog = !fullRef.current && !selectedTitle && (view === "vod" || view === "series");
    if (code >= 403 && code <= 406) {
      e.preventDefault();
      if (e.repeat) return;
      if (!session || editingText || tracksRef.current || parental.open || document.querySelector(".modal")) return;
      if (code === 403) {
        manageParental();
      } else if (code === 404) {
        if (livePlayer) leavePlayer(true, true);
        else if (view === "live" && fullRef.current) leavePlayer();
        else if (view === "live") {
          const category = document.querySelector<HTMLElement>(".live-shell .category-list button.selected")
            || document.querySelector<HTMLElement>(".live-shell .category-list button");
          category?.focus();
        } else if (browsingCatalog) document.querySelector<HTMLElement>(".header-search-field")?.focus({preventScroll:true});
        else goShortcut("live");
      } else if (code === 405) {
        const key = document.activeElement?.closest<HTMLElement>("[data-focus-key]")?.dataset.focusKey;
        const matches = (i: Item) => key === itemKey(i);
        const item = fullRef.current ? previewRef.current
          : (key ? catalog.live.find(matches) || catalog.vod.find(matches) ||
            catalog.series.find(matches) || episodes.find(matches) ||
            history.find(h => matches(h.item))?.item || favorites.find(matches) : undefined) ||
            (view === "live" ? previewRef.current : selectedTitle);
        if (item) {
          toggleFavorite(item);
          if (fullRef.current) showControls();
        }
      } else if (livePlayer) {
        cycleLiveAspect();
      } else if (fullRef.current) {
        showControls();
        setPlayerOptions(!playerOptionsRef.current);
      } else if (browsingCatalog) {
        document.querySelector<HTMLButtonElement>(".catalog-sort")?.click();
      } else if (view === "live") {
        if (previewRef.current?.kind === "live") openFullscreen();
      } else {
        goShortcut("settings");
      }
      return;
    }
    if (livePlayer && isInfoKey(e) && !tracksRef.current && !playerOptionsRef.current && !parental.open && !document.querySelector(".modal")) {
      e.preventDefault();
      if (!e.repeat) {
        showControls();
        playerRef.current?.refreshTracks();
        setTracks(true);
      }
      return;
    }
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
        if (!playerMenuNavigation.current) {
          e.preventDefault();
          playerMenuNavigation.current = true;
          showControls();
          requestAnimationFrame(() => document.querySelector<HTMLElement>(".player-play-toggle")?.focus());
          return;
        }
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
          () => document.querySelector<HTMLElement>(".player-play-toggle")?.focus(),
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
    let pointerPoster: Element | null = null;
    const pointerDown = (e: PointerEvent) => {
      pointerPoster = (e.target as Element).closest(".catalog-poster");
    };
    const pointerUp = () => { pointerPoster = null; };
    document.addEventListener("pointerdown", pointerDown, true);
    document.addEventListener("pointerup", pointerUp, true);
    document.addEventListener("pointercancel", pointerUp, true);
    const focus = (e: FocusEvent) => {
      const target = e.target as HTMLElement;
      if (target !== pointerPoster && !restoringPosterFocus.current &&
          target.closest(".shell") && !target.closest(".catalog-details"))
        setNavCollapsed(!target.closest(".navigation"));
      rememberFocus(target);
      if (!restoringPosterFocus.current) scrollFocused(target);
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
      document.removeEventListener("pointerdown", pointerDown, true);
      document.removeEventListener("pointerup", pointerUp, true);
      document.removeEventListener("pointercancel", pointerUp, true);
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
      if (restoringPosterFocus.current) return;
      const root =
        [...document.querySelectorAll(".modal")].pop() ||
        document.querySelector(".fullscreen") ||
        document.getElementById("app")!;
      if (!root.contains(document.activeElement)) {
        if (root instanceof HTMLElement && root.classList.contains("live-player")) root.focus({preventScroll: true});
        else focusables(root)[0]?.focus();
      }
    }, 50);
    return () => clearTimeout(id);
  }, [view, !!session, editing, !!detail, !!resume, tracks, exit, full, license.ready, license.allowed]);
  const surface = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    playerRef.current?.position();
  });
  const liveFavorites = category === "favorites" ? favoriteKeys : null;
  const filteredLive = useMemo(
    () => {
      const normalizedQuery = query.toLocaleLowerCase();
      return catalog.live.filter(
        (i) =>
          (normalizedQuery.length > 0 || (category === "all" || category === "favorites"
            ? category !== "favorites" || liveFavorites?.has(itemKey(i))
            : i.category === category)) &&
          i.name.toLocaleLowerCase().includes(normalizedQuery),
      );
    },
    [catalog.live, category, query, liveFavorites],
  );
  const returnChannel = !full && view === "live" && playerReturn.current?.kind === "live"
    ? filteredLive.findIndex(i => i.id === playerReturn.current?.id)
    : -1;
  useLayoutEffect(() => {
    if (full || !playerReturn.current) return;
    const item = playerReturn.current;
    const origin = playerOrigin.current;
    const random = returnFromRandom.current;
    returnFromRandom.current = false;
    playerReturn.current = null;
    playerOrigin.current = null;
    if (returnChannel >= 0) return;
    const key = itemKey(item);
    const series = item.seriesId ? `series:${item.seriesId}` : null;
    const targets = [...document.querySelectorAll<HTMLElement>(".shell [data-focus-key]")]
      .filter((element) => getComputedStyle(element).visibility === "visible");
    const target = (random && origin?.isConnected ? origin : null) ||
      targets.find(el => el.dataset.focusKey === key) ||
      targets.find(el => el.dataset.focusKey === series) ||
      (origin?.isConnected ? origin : null);
    target?.focus({ preventScroll: true });
  }, [full]);
  const p = playerRef.current,
    status = p?.status;
  useLayoutEffect(() => {
    if (classic && view === "home" && session && !full && !externalTitle && !document.activeElement?.closest(".classic-dashboard")) {
      document.querySelector<HTMLElement>(".classic-primary button")?.focus({preventScroll: true});
    }
  }, [classic, view, session, full]);
  useEffect(() => () => clearTimeout(aspectNoticeTimer.current), []);
  useEffect(() => {
    showControls();
    if (!full) {
      clearTimeout(aspectNoticeTimer.current);
      setAspectNotice(null);
      setPlayerOptions(false);
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
  const resumeRows = useMemo(() => resumeHistory(history), [history]);
  const liveCategoryCounts = useMemo(() => {
    const counts = new Map<string, number>();
    if (classic) for (const item of catalog.live) counts.set(item.category, (counts.get(item.category) || 0) + 1);
    return counts;
  }, [classic, catalog.live]);
  const previewPrograms = preview ? epg[preview.id] || [] : [];
  const [now, next] = currentNext(previewPrograms);
  const [following, setFollowing] = useState<{source: string; item: Item} | null>(null);
  useEffect(() => {
    if (!full || preview?.kind !== "episode" || !preview.seriesId) return;
    const key = itemKey(preview);
    const series = catalog.series.find((entry) => entry.id === preview.seriesId);
    if (!series) return;
    const order = (left: Item, right: Item) => (left.season || 0) - (right.season || 0) ||
      (left.episode || 0) - (right.episode || 0) || left.id.localeCompare(right.id);
    let active = true;
    const cached = detailCache.current.get(itemKey(series));
    void Promise.resolve(cached?.episodes || providerRef.current?.episodes(series) || []).then((rows) => {
      if (!active) return;
      const list = [...rows].sort(order);
      const index = list.findIndex((episode) => itemKey(episode) === key);
      const item = index >= 0 ? list[index + 1] : undefined;
      setFollowing(item ? {source: key, item} : null);
    }).catch(() => { if (active) setFollowing(null); });
    return () => { active = false; };
  }, [full, preview, catalog.series]);
  const playToggle = () => <button class="primary player-play-toggle" onClick={() => p?.toggle()}
    aria-label={status?.state === "paused" ? t("Abspielen", "Play") : t("Pause", "Pause")}>
    <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" fill="currentColor">
      {status?.state === "paused" ? <path d="M7 4l13 8-13 8z" /> : <path d="M6 4h4v16H6zm8 0h4v16h-4z" />}
    </svg>
  </button>;
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
      data-favorite={isFavorite(item)}
    >
      {classic && <span class="channel-number">{item.number || ""}</span>}
      <Logo item={item} />
      <span class="channel-copy">
        <strong>{item.name}</strong>
        {!classic && <span class="epg-pair">
          <small>Jetzt: {info(item)[0]?.title || "Keine Programmdaten"}</small>
          <small>Danach: {info(item)[1]?.title || "Keine Programmdaten"}</small>
        </span>}
      </span>
      <FavoriteMark marked={isFavorite(item)} />
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
            : item.kind === "episode" ? playTitle(item) : void openDetail(item, true)
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
          <div class="home-rail-track">
            {items.slice(0, 30).map((item) => (
              <span key={itemKey(item)}>
                {card(item, title.startsWith("Weiter ansehen"))}
              </span>
            ))}
          </div>
        </div>
      </section>
    );
  const checkUpdates = async (manual: boolean) => {
    if (updateInProgress.current) return;
    updateInProgress.current = true;
    setCheckingUpdate(true);
    if (manual) setUpdate(t("Update wird geprüft …", "Checking for updates …"));
    try {
      const result = await stageUpdate(updateToken);
      if (result === 'ready') {
        setUpdateReady(manual ? 'manual' : 'automatic');
        setUpdate(t('Update geprüft und geladen. Neustart, sobald keine Wiedergabe läuft.', 'Verified update downloaded. Restart once playback is idle.'));
      } else if (result === 'reinstall') {
        setUpdate(t('Diese Version benötigt ein neues signiertes TV-Paket vom PC.', 'This version requires a new signed TV package installed from a PC.'));
      } else if (manual) {
        setUpdate(result === 'current' ? t('Du verwendest die aktuelle Version.', 'You are using the current version.') :
          result === 'blocked' ? t('Dieses Update wurde nach einem Startfehler zurückgesetzt.', 'This update was rolled back after a startup error.') :
          t('GitHub-Zugriff fehlt. Prüfe den Lese-Token und den Update-Kanal.', 'Check the read-only GitHub token and the update channel.'));
      }
    } catch {
      if (manual) setUpdate(t('Update konnte nicht sicher geladen werden.', 'The update could not be downloaded safely.'));
    } finally {
      updateInProgress.current = false;
      setCheckingUpdate(false);
    }
  };
  useEffect(() => {
    if (!updateToken || !window.dadoTvBoot) return;
    const first = setTimeout(() => void checkUpdates(false), 30_000);
    const interval = setInterval(() => void checkUpdates(false), 6 * 3600_000);
    return () => { clearTimeout(first); clearInterval(interval); };
  }, [updateToken, saved.settings.language]);
  useEffect(() => {
    if (!updateReady || busy || preview || full || resume || exit || refreshing ||
        (view !== 'home' && !(view === 'settings' && updateReady === 'manual'))) return;
    const restart = setInterval(() => {
      if (!previewRef.current && !fullRef.current &&
          !document.querySelector('.modal, input:focus:not([readonly])')) location.reload();
    }, 2500);
    return () => clearInterval(restart);
  }, [updateReady, busy, preview, full, resume, exit, view, refreshing]);
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
  const manageParental = () => {
    stop();
    parental.manage();
  };
  const settingsView = () => {
    const active = settingsSections.find((section) => section.id === settingsSection);
    const quickChoice = (current: string, choices: [string, string][], commit: (value: string) => void) => <>
      <div class="settings-choices" role="group" aria-label={active?.[saved.settings.language]}>
        {choices.map(([value, label]) => <button key={value}
          data-current={value === current ? "" : undefined}
          aria-pressed={(settingsDraft ?? current) === value}
          onClick={() => setSettingsDraft(value)}>
          <span class="settings-choice-check" aria-hidden="true">{value === current ? "✓" : ""}</span>
          {label}
        </button>)}
      </div>
      <button class="primary" onClick={() => { commit(settingsDraft ?? current); closeSettingsSection(); }}>
        {t("Änderungen speichern", "Save changes")}
      </button>
    </>;
    const renderSection = (id: string) => {
      switch (id) {
        case "clock": return quickChoice(saved.settings.clockFormat, [
          ["H24", "24 h · 18:30"], ["H12", "12 h · 06:30 PM"],
        ], value => setting("clockFormat", value as Settings["clockFormat"]));
        case "stream": return <>
          <p>{t("Gilt für Xtream-Live-TV. M3U verwendet das Format der Wiedergabeliste.",
            "Applies to Xtream live TV. M3U uses the playlist's stream format.")}</p>
          {quickChoice(saved.settings.liveFormat, [
            ["default", t("Standard", "Default")], ["ts", "MPEG-TS (.ts)"], ["m3u8", "HLS (.m3u8)"],
          ], value => {
            const format = value as Settings["liveFormat"];
            setting("liveFormat", format);
            const provider = providerRef.current;
            if (session?.kind === "Xtream" && provider) {
              setCatalog(old => ({...old, live: old.live.map(item => ({
                ...item, url: provider.stream("live", item.id, liveExtension(format)),
              }))}));
            }
          })}
        </>;
        case "player": return quickChoice(saved.settings.player, [
          ["auto", t("Automatisch · empfohlen", "Automatic · recommended")],
          ["avplay", "Samsung AVPlay"], ["html", "HTML-Video"],
        ], value => changed({...savedRef.current, settings: {...savedRef.current.settings,
          expertMode: savedRef.current.settings.expertMode || value !== "auto", player: value as Settings["player"]}}));
        case "cache": return <>
          <p>{t("Temporäre Programmdaten und Episodeninfos leeren?", "Clear temporary programme data and episode details?")}</p>
          <p class="muted">{t("Profile, Favoriten, Verlauf und Lizenz bleiben erhalten.", "Profiles, favourites, history and licence are kept.")}</p>
          <button disabled={busy || refreshing} onClick={() => {
            epgCacheEpoch.current++;
            epgPending.current.clear();
            detailRequest.current++;
            detailCache.current.clear();
            setEpg({});
            for (const memory of Object.values(memories.current)) { memory.detail = null; memory.episodes = []; }
            setCacheCleared(true);
          }}>{t("Jetzt leeren", "Clear now")}</button>
          {cacheCleared && <p role="status">{t("Cache geleert.", "Cache cleared.")}</p>}
        </>;
        case "general": return <>
          <label class="field">
            <span>{t("Oberfläche", "Interface layout")}</span>
            {choose(saved.settings.interfaceLayout, (value) => setting("interfaceLayout", value === "classic" ? "classic" : "modern"), [["modern", "Modern"], ["classic", t("Klassisch", "Classic")]], t("Oberfläche", "Interface layout"))}
          </label>
<Field label={t("App-Sprache", "App language")}>
            {choose(saved.settings.language, (value) => setting("language", value as Settings["language"]), [["de", "Deutsch"], ["en", "English"]], t("App-Sprache", "App language"))}
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
          <Field label={t("Fortsetzen und Wiedergabeverlauf", "Continue watching and watch history")}>
            <TVInput type="checkbox" checked={saved.settings.watchHistory}
              onChange={(event) => setting("watchHistory", event.currentTarget.checked)} />
          </Field>
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
              disabled={!saved.settings.watchHistory}
              checked={saved.settings.watchHistory && saved.settings.autoplayLast}
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
        case "profiles": return <div class="settings-profiles" data-settings-ordered>
          <ul class="settings-profile-list">
            {saved.profiles.map(profile => <li key={profile.id}>
              <strong title={profile.name}>{profile.name}</strong>
              <button type="button" disabled={busy || session?.id === profile.id}
                aria-label={`${t("Auswählen", "Select")} · ${profile.name}`}
                onClick={() => void login(profile)}>
                {t("Auswählen", "Select")}
              </button>
            </li>)}
          </ul>
          <button class="profile-add" onClick={() => {
            setMessage("");
            setForm(emptyProfile());
            setEditing(true);
          }}>{t("Profil hinzufügen", "Add profile")}</button>
          <ConnectPanel importProfiles={importCloudProfiles} profileList />
        </div>;
        case "parental": return <>
          <p>{parental.enabled?'Aktiv · gesperrte Inhalte benötigen die Eltern-PIN':'Vierstellige PIN für Sender, Filme und Serien einrichten'}</p>
          <button onClick={manageParental}>{parental.enabled?'Elternsperre verwalten':'Eltern-PIN einrichten'}</button>
        </>;
        case "updates": {
          const storeUpdateToken = (value: string) => {
            try {
              if (value) localStorage.setItem(accessStorage, value);
              else localStorage.removeItem(accessStorage);
              setUpdateToken(value);
              return true;
            } catch {
              setUpdate(t('Speichern fehlgeschlagen.', 'Could not save the token.'));
              return false;
            }
          };
          return <div class="settings-update-stack" data-settings-ordered>
          <UpdateTokenTransfer t={t} onToken={storeUpdateToken} />
          <Field label={t('GitHub-Lese-Token', 'Read-only GitHub token')}>
            <TVInput type="password" autocomplete="off" value={updateToken}
              onChange={(e) => {
                storeUpdateToken(e.currentTarget.value.trim());
              }} />
          </Field>
          <p class="settings-update-help muted">{t(
            "Auf GitHub einen Fine-grained Token für tariktech7/dado-player erstellen. Unter Contents genügt Read-only.",
            "Create a fine-grained GitHub token for tariktech7/dado-player. Set Contents to Read-only."
          )}</p>
          <div class="settings-update-actions">
          <button disabled={checkingUpdate} onClick={() => void checkUpdates(true)}>
            {t("GitHub-Update prüfen und installieren", "Check and install GitHub update")}
          </button>
          {window.dadoTvBoot?.fromUpdate && <button onClick={() => {
            try { saveSlot({ blocked: Math.max(readSlot().blocked || 0, APP_BUILD) }); location.reload(); }
            catch { setUpdate(t('Rückkehr fehlgeschlagen.', 'Could not roll back the update.')); }
          }}>{t('Installierte Version wiederherstellen', 'Restore installed version')}</button>}
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
          </div>
          {update && <p role="status">{update}</p>}
          <p class="muted">{t('Mit GitHub-Lesezugriff prüft Dado Player automatisch alle sechs Stunden auf signierte Web-Updates.',
            'With read access to GitHub, Dado Player checks for signed web updates every six hours.')}</p>
          <p class="muted">{APP_VERSION}{releaseInfo.channel === "beta" ? ` Beta ${releaseInfo.betaNumber}` : ""} · Build {APP_BUILD}</p>
        </div>;
        }
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
      <div class={`settings-page${active ? " has-setting" : ""}`}>
        {!active ? <><h1>{t("Einstellungen", "Settings")}</h1>
        <nav class="settings-sections" data-focus-zone="settings-sections" aria-label={t("Bereiche", "Sections")}>
          {settingsSections.filter(section => !query.trim() || section[saved.settings.language].toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())).map(section => (
            <button key={section.id} data-focus-key={section.id} onClick={() => {
              setSettingsDraft(null);
              setCacheCleared(false);
              setSettingsSection(section.id);
              requestAnimationFrame(() => document.querySelector<HTMLElement>(".settings-options button")?.focus());
            }}>
              <SettingsTileIcon section={section.id} />
              <span>{section[saved.settings.language]}</span>
            </button>
          ))}
        </nav></> : <section key={active.id} id="settings-options" class="glass settings-options"
            data-focus-zone="settings-options" data-settings-section={active.id}
            data-license-status={active.id === "license" ? "" : undefined}>
          <button class="settings-back menu-back" onClick={closeSettingsSection}>{t("‹ Alle Einstellungen", "‹ All settings")}</button>
          <h2>{active[saved.settings.language]}</h2>
          <div class="settings-content">{renderSection(active.id)}</div>
        </section>}
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
  const profileForm = (saveOnly = false) => (
    <form
      class="profile-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (saveOnly) addProfile();
        else void login(form);
      }}
    >
      {!saveOnly && <><h1>
        {saved.profiles.length
          ? "Profil verbinden"
          : "Willkommen bei Dado Player"}
      </h1>
      <p>Deine Sender. Dein Programm. Direkt auf deinem Samsung-TV.</p></>}
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
        {saveOnly ? t("Profil speichern", "Save profile") : busy ? "Verbinden …" : "Verbinden & speichern"}
      </button>
      {saveOnly && message && <p role="alert">{message}</p>}
      {(saveOnly || saved.profiles.length > 0) && (
        <button type="button" onClick={() => {setEditing(false); if(saveOnly) setMessage("");}}>
          Zurück zu Profilen
        </button>
      )}
      {!saveOnly && <><button
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
      </p></>}
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
            class={`shell ${classic ? "classic-shell" : navCollapsed.current && !navHover.current ? "nav-collapsed" : ""} ${view === "live" ? "live-shell with-preview" : ""} ${view === "vod" || view === "series" ? `catalog-shell${selectedTitle ? " has-title-detail" : ""}` : ""}${externalTitle ? " has-title-detail external-title-detail" : ""}${full ? " behind-player" : ""}`}
          >
            <header class={classic ? "classic-header" : "classic-header modern-header"}
              data-focus-zone={classic ? "navigation" : undefined}>
              {classic && view !== "home" && <button class="menu-back classic-back" onClick={back}>{t("‹ Zurück", "‹ Back")}</button>}
              {classic && <><img class="brand-normal" src="d7.svg" alt="" /><img class="brand-dark" src="d7-dark.svg" alt="" />
              <strong>Dado Player</strong></>}
              {classic && license.claims?.kind === 'trial' && <small class="trial-badge" data-trial-badge>Testversion · {license.text.replace(' Testzeit', '').replace('Noch', 'noch')}</small>}
              <div class="header-search">
                <button type="button" aria-label={t("Suche", "Search")} onClick={() => {
                  if (view === "home") goShortcut("search");
                  else document.querySelector<HTMLElement>(".header-search-field")?.focus({preventScroll:true});
                }}>⌕</button>
                <TVInput class="header-search-field" type="search"
                  placeholder={t("Suchen", "Search")}
                  aria-label={t("Suchen", "Search titles")} value={view === "home" ? "" : query}
                  onInput={(e) => {
                    const text = e.currentTarget.value;
                    if (view === "home") goShortcut("search");
                    setQuery(text);
                  }} />
              </div>
              <button class="refresh-library" title={t("Liste aktualisieren", "Refresh library")}
                aria-label={t("Liste aktualisieren", "Refresh library")} aria-busy={refreshing}
                data-refresh-status={refreshStatus} onClick={() => void refreshCatalog()}>
                <span class="refresh-icon"><RailIcon kind="refresh" /><RailIcon kind="success" /><RailIcon kind="error" /></span>
              </button>
              <time>{clock(Date.now() / 1000, saved.settings.clockFormat)}</time>
            </header>
            {!classic && <aside
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
                {nav.filter(([id]) => saved.settings.watchHistory || id !== "history").map(([id, icon, de, en]) => (
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
                  aria-busy={refreshing} data-refresh-status={refreshStatus} onClick={() => void refreshCatalog()}>
                  <span class="refresh-icon">
                    <RailIcon kind="refresh" /><RailIcon kind="success" /><RailIcon kind="error" />
                  </span>
                  <span class="nav-label">{t("Liste aktualisieren", "Refresh library")}</span>
                </button>
                <span class="refresh-announcement" role="status">
                  {refreshStatus === "loading" ? t("Liste wird aktualisiert", "Refreshing library")
                    : refreshStatus === "success" ? t("Liste aktualisiert", "Library refreshed")
                    : refreshStatus === "error" ? t("Aktualisierung unvollständig oder fehlgeschlagen", "Refresh incomplete or failed")
                    : ""}
                </span>
              </nav>
              <footer>
                <span>{session?.name}</span>
                <small>Tizen {APP_VERSION}</small>
              </footer>
            </aside>}
            {view === "live" ? (
              <>
                <section class="categories" data-focus-zone="categories">
                  <h1>{t("Kategorien", "Categories")}</h1>
                  <div class="category-list">
                    <button
                      class={`${category === "all" ? "selected" : ""}${classic ? " with-category-count" : ""}`}
                      onClick={() => {
                        setCategory("all");
                      }}
                    >
                      {t("Alle Sender", "All channels")}
                      {classic && <small class="category-count">{catalog.live.length}</small>}
                    </button>
                    <button
                      class={`${category === "favorites" ? "selected" : ""}${classic ? " with-category-count" : ""}`}
                      onClick={() => {
                        setCategory("favorites");
                      }}
                    >
                      {t("Favoriten", "Favorites")}
                      {classic && <small class="category-count">{favoriteRows.length}</small>}
                    </button>
                    {catalog.categories.live.map((c) => (
                      <button
                        class={`${category === c.id ? "selected" : ""}${classic ? " with-category-count" : ""}`}
                        onClick={() => {
                          setCategory(c.id);
                        }}
                      >
                        {c.name}
                        {classic && <small class="category-count">{liveCategoryCounts.get(c.id) || 0}</small>}
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
                  <VirtualChannels
                    items={filteredLive}
                    row={channelRow}
                    epg={getEpg}
                    density={saved.settings.density}
                    classic={classic}
                    returnIndex={returnChannel}
                  />
                </section>
                <aside class="preview-panel" data-focus-zone="details">
                  <header>
                    <h2>{t("Vorschau", "Preview")}</h2>
                    {!full && <ShortcutLegend t={t} labels={[
                      t("Elternsperre", "Parental lock"), t("Kategorien", "Categories"),
                      t("Favorit", "Favorite"), t("Vollbild", "Fullscreen"),
                    ]} />}
                    {preview && (
                      <button
                        class="icon-button"
                        aria-label={t("Sender als Favorit", "Favorite channel")}
                        aria-pressed={isFavorite(preview)}
                        onClick={() => toggleFavorite(preview)}
                      >
                        <FavoriteMark marked={isFavorite(preview)} />
                      </button>
                    )}
                  </header>
                  <div
                    ref={preview && !full ? surface : undefined}
                    class={`preview-surface${preview ? "" : " empty-preview"}`}
                  >
                    {!preview && <span aria-hidden="true">▷</span>}
                  </div>
                  {preview ? <>
                    <button class="preview-open" onClick={openFullscreen}>
                      Vollbild öffnen <span>↗</span>
                    </button>
                    <PlayerFeedback status={status} />
                    <h2>{preview.name}</h2>
                    <div class="epg-detail" tabIndex={0}>
                      {classic ? (
                        previewPrograms.some((entry) => entry.end > Date.now() / 1000) ? <ol class="classic-epg">
                          {previewPrograms.filter((entry) => entry.end > Date.now() / 1000).slice(0, 8).map((entry) => <li key={entry.start}>
                            <time>{clock(entry.start, saved.settings.clockFormat)} – {clock(entry.end, saved.settings.clockFormat)}</time>
                            <strong>{entry.title}</strong><p>{entry.description}</p>
                          </li>)}
                        </ol> : <p>{t("Keine Programmdaten verfügbar.", "No programme data available.")}</p>
                      ) : now ? (
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
                      {!classic && next && (
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
                  </> : <>
                    <h2>{t("Sender auswählen", "Choose a channel")}</h2>
                    <p>{t("Hier erscheinen Vorschau, Programm und Senderinformationen.", "The preview, schedule and channel information will appear here.")}</p>
                  </>}
                </aside>
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
                {view === "home" && classic ? (
                  <div class="classic-dashboard">
                    <header><h1>{t("Bibliothek", "Library")}</h1><span>{session?.name}</span></header>
                    <div class="classic-deck">
                      <div class="classic-primary" data-focus-zone="library">
                        {([
                          ["live", t("Live-TV", "Live TV")],
                          ["vod", t("Filme", "Movies")],
                          ["series", t("Serien", "Series")],
                        ] as const).map(([id, label]) => <button key={id} data-focus-key={id} onClick={() => goShortcut(id)}>
                          <span class="classic-icon-plate"><LibraryIcon kind={id} /></span>
                          <span class="classic-tile-title">{label}</span>
                        </button>)}
                      </div>
                      <div class="classic-actions" data-focus-zone="actions">
                        {saved.settings.watchHistory && <button data-focus-key="history" onClick={() => goShortcut("history")}>
                          <svg aria-hidden="true" viewBox="0 0 24 24" fill="currentColor"><path d="M7 4l13 8-13 8z" /></svg>
                          {t("Fortsetzen", "Continue")}
                        </button>}
                        <button data-focus-key="favorites" onClick={() => goShortcut("favorites")}>
                          <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="m12 2 3 6.6 7 .9-5.2 5 1.3 7.1-6.1-3.5-6.1 3.5 1.3-7.1L2 9.5l7-.9z" /></svg>
                          {t("Favoriten", "Favorites")}
                        </button>
                        <button onClick={() => { setSettingsSection("profiles"); goShortcut("settings"); }}>
                          <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="8" r="3.6" /><path d="M4 21v-1.5a8 6 0 0 1 16 0V21z" /></svg>
                          {t("Profile", "Profiles")}
                        </button>
                        <button data-focus-key="settings" onClick={() => { setSettingsSection(null); goShortcut("settings"); }}>
                          <RailIcon kind="settings" />{t("Einstellungen", "Settings")}
                        </button>
                      </div>
                    </div>
                  </div>
                ) : view === "home" ? (
                  <Home
                    name={session?.name || ""}
                    catalog={catalog}
                    history={resumeRows}
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
                      {resumeRows
                        .filter((h) => !query.trim() || h.item.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
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
                        .filter(i => !query.trim() || i.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
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
                    legend={<ShortcutLegend t={t} labels={[
                      t("Elternsperre", "Parental lock"), t("Suche", "Search"),
                      t("Favorit", "Favorite"), t("Sortierung", "Sort"),
                    ]} />}
                    key={view}
                    kind={view === "vod" ? "vod" : "series"}
                    t={t}
                    classic={classic}
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
                    selected={selectedTitle}
                    close={closeCatalogDetail}
                    select={(i) => {
                      catalogOrigin.current = {
                        titleTop: document.querySelector(".catalog-title-list")?.scrollTop || 0,
                        categoryTop: document.querySelector(".catalog-browser .category-list")?.scrollTop || 0,
                      };
                      setSelectedTitle(i);
                      void openDetail(i, true);
                    }}
                    favorites={favorites}
                    detail={detail}
                    episodes={episodes}
                    history={history}
                    watchHistory={saved.settings.watchHistory}
                    toggle={toggleFavorite}
                    play={playTitle}
                    playDirect={playDirect}
                    allowEpisodePreviews={allowEpisodePreviews}
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
            {externalTitle && <TitleDetail item={externalTitle} t={t} panel={externalTitlePanel}
              close={closeExternalTitle} episodes={episodes} history={history}
              marked={isFavorite(externalTitle)} toggle={toggleFavorite} play={playTitle} playDirect={playDirect}
              previews={allowEpisodePreviews(externalTitle)} />}
          </div>
          {editing && modal(t("Profil hinzufügen", "Add profile"), profileForm(true))}
          {full && preview && (
            <div
              class={`fullscreen ${preview.kind === "live" ? "live-player" : "vod-player"} ${status?.state === "error" ? "playback-failed" : ""} ${controlsVisible ? "" : "controls-hidden"}`}
              tabIndex={preview.kind === "live" ? -1 : undefined}
              onMouseMove={showControls}
              onClick={showControls}
            >
              <div ref={surface} class="full-surface" />
              {aspectNotice && <div class="player-aspect-notice" role="status">
                {aspectNotice.value === "fill" ? t("Automatisch", "Automatic")
                  : aspectNotice.value === "fit" ? t("Einpassen", "Fit") : t("Strecken", "Stretch")}
              </div>}
              {preview.kind === "live" && <button class="live-back" onClick={() => leavePlayer(true, true)}>
                {t("← Zurück", "← Back")}
              </button>}
              {preview.kind !== "live" && <div class="player-top glass">
                <button onClick={() => leavePlayer()}>{t("← Zurück", "← Back")}</button>
                <span>
                  <small>
                    {preview.kind === "episode" ? t("SERIE", "SERIES") : t("FILM", "MOVIE")}
                  </small>
                  <strong>{preview.name}</strong>
                </span>
                <button aria-label={t("Favorit", "Favorite")} aria-pressed={isFavorite(preview)}
                  onClick={() => toggleFavorite(preview)}>
                  <FavoriteMark marked={isFavorite(preview)} />
                </button>
              </div>}
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
              {playerOptions && (
                <div class="player-options glass" aria-label={t("Bildformat", "Aspect ratio")}>
                  {([
                    ["fill", t("Automatisch", "Automatic")],
                    ["fit", t("Einpassen", "Fit")],
                    ["stretch", t("Strecken", "Stretch")],
                  ] as const).map(([value, label]) => (
                    <button key={value} aria-pressed={saved.settings.aspect === value} onClick={() => {
                      applyPlayerAspect(value);
                      setPlayerOptions(false);
                      document.querySelector<HTMLElement>(".player-options-toggle")?.focus();
                    }}>{label}</button>
                  ))}
                </div>
              )}
              {preview.kind === "live" ? <div class="player-controls">
                <div class="on-air">
                  {preview.number ? <span class="on-air-label">{preview.number}</span> : null}
                  <Logo item={preview} />
                  <span class="on-air-copy"><strong>{preview.name}</strong>{now && <small>{now.title}</small>}</span>
                </div>
                <ShortcutLegend t={t} marked={isFavorite(preview)} onAction={(key) => {
                  switch (key) {
                    case "F1": manageParental(); break;
                    case "F2": leavePlayer(true, true); break;
                    case "F3": toggleFavorite(preview); break;
                    case "F4": cycleLiveAspect(); break;
                  }
                }} />
              </div> : <>
                <div class="player-controls">
                <div class="timeline seek-transport">
                  {playToggle()}
                  <span>{time(status?.time || 0)}</span>
                  <div class="seek-bar" onClick={(event) => {
                    const duration = status?.duration || 0;
                    const bounds = event.currentTarget.getBoundingClientRect();
                    if (duration > 0 && bounds.width > 0) {
                      safe(() => p?.seekTo(((event.clientX - bounds.left) / bounds.width) * duration));
                    }
                  }}><progress value={status?.time || 0} max={status?.duration || 1} /></div>
                  <span>{time(status?.duration || 0)}</span>
                </div>
                <div class="transport">
                  <button onClick={() => { p?.refreshTracks(); setTracks(true); }}>Audio</button>
                  <button class="player-options-toggle" aria-expanded={playerOptions}
                    onClick={() => setPlayerOptions(!playerOptions)}>
                    {t("Bildformat", "Aspect ratio")}
                  </button>
                  {following?.source === itemKey(preview) && <button onClick={() => {
                    const position = history.find((entry) => itemKey(entry.item) === itemKey(following.item))?.position || 0;
                    void start(following.item, true, untrackedPlayback.current ? 0 : position, untrackedPlayback.current);
                  }}>{t("Nächste Folge", "Next episode")}</button>}
                </div>
                </div>
              </>}
            </div>
          )}
          {resume &&
            modal(
              t("Wiedergabe fortsetzen?", "Resume playback?"),
              <>
                <p>
                  {resume.item.name} · {t("Zuletzt gesehen bei", "Last watched at")} {time(resume.position, true)}
                </p>
                <button
                  class="primary"
                  onClick={() => {
                    const r = resume;
                    setResume(null);
                    void start(r.item, true, r.position);
                  }}
                >
                  {t("Fortsetzen", "Resume")}
                </button>
                <button
                  onClick={() => {
                    const r = resume;
                    setResume(null);
                    void start(r.item, true, 0);
                  }}
                >
                  {t("Von vorne anfangen", "Start from beginning")}
                </button>
                <button onClick={() => setResume(null)}>{t("Abbrechen", "Cancel")}</button>
              </>,
            )}
          {tracks &&
            modal(
              preview && preview.kind !== "live" ? "Audio" : t("Ton & Untertitel", "Audio & subtitles"),
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
    const untracked = untrackedPlayback.current;
    stop();
    await start(item, true, 0, untracked);
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
  classic,
  returnIndex,
}: {
  items: Item[];
  row: (i: Item) => ComponentChildren;
  epg: (i: Item) => Promise<void>;
  density: Settings["density"];
  classic: boolean;
  returnIndex: number;
}) {
  const [focusIndex, setFocusIndex] = useState<number | null>(null);
  const pendingFocus = useRef<number | null>(null);
  const [top, setTop] = useState(0),
    ref = useRef<HTMLDivElement>(null),
    height = (classic ? 56 : shellAppearance.channelRowHeight) * (density === "compact" ? 1 : 1.25);
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
    if (classic) return;
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
  }, [items, start, classic]);
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
