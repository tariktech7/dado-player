import type { ComponentChildren, RefObject } from "preact";
import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "preact/hooks";
import { createPortal } from "preact/compat";
import { Item, Category, Watched, itemKey } from "./types";
import { TVInput } from "./input";
import { useEpisodeFrames } from "./episode-frames";
import { FavoriteMark } from "./favorite-mark";
import appearance from "./shell-appearance";
import policy from '../../../shared/product-policy.json' with { type: 'json' };

export function resumeEpisode(episodes: Item[], history: Watched[]) {
  const ordered = [...episodes].sort(
    (a, b) =>
      (a.season || 0) - (b.season || 0) ||
      (a.episode || 0) - (b.episode || 0) ||
      a.id.localeCompare(b.id),
  );
  const watched = [...history]
    .sort((a, b) => b.at - a.at)
    .find((h) => ordered.some((e) => itemKey(e) === itemKey(h.item)));
  const index = watched
    ? ordered.findIndex((e) => itemKey(e) === itemKey(watched.item))
    : 0;
  if (!ordered.length) return null;
  if (
    watched &&
    watched.position > 0 &&
    watched.duration > 0 &&
    watched.duration - watched.position <= policy.resumeEndMarginSeconds
  )
    return { item: ordered[index + 1] || ordered[0], position: 0 };
  return { item: ordered[index], position: watched?.position || 0 };
}

export type EpisodeLoad = "loading" | "ready" | "failed";

export function EpisodePicker({episodes, history, play, playDirect, t, actions, previews, seriesCover, profileId, episodeLoad, reloadEpisodes}: {
  episodes: Item[];
  history: Watched[];
  play: (item: Item, position: number) => void;
  playDirect: (item: Item, position: number, untracked?: boolean) => void;
  t: (de: string, en: string) => string;
  actions?: ComponentChildren;
  previews: boolean;
  profileId: string;
  seriesCover: string;
  episodeLoad: EpisodeLoad;
  reloadEpisodes: () => void;
}) {
  const ordered = useMemo(() => [...episodes].sort((a, b) =>
    (a.season ?? 1) - (b.season ?? 1) || (a.episode ?? 1) - (b.episode ?? 1) || a.id.localeCompare(b.id)
  ), [episodes]);
  const seasons = useMemo(() => [...new Set(ordered.map(e => e.season ?? 1))], [ordered]);
  const target = useMemo(() => resumeEpisode(ordered, history), [ordered, history]);
  const [chosenSeason, setChosenSeason] = useState<number | null>(null);
  const season = chosenSeason ?? target?.item.season ?? seasons[0] ?? 1;
  const tabs = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => ordered.filter(e => (e.season ?? 1) === season), [ordered, season]);
  const frames = useEpisodeFrames(rows, list, previews, profileId);
  const artSource = (url?: string) => (url || "").trim().split(/[?#]/, 1)[0];
  const artCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const episode of ordered) {
      const source = artSource(episode.preview);
      if (source) counts.set(source, (counts.get(source) ?? 0) + 1);
    }
    return counts;
  }, [ordered]);
  const watched = useMemo(() => new Map(history.map(h => [itemKey(h.item), h])), [history]);

  useLayoutEffect(() => {
    const rail = tabs.current;
    const selected = rail?.querySelector<HTMLElement>(".selected");
    if (!rail || !selected) return;
    if (selected.offsetLeft < rail.scrollLeft) rail.scrollLeft = selected.offsetLeft - 4;
    if (selected.offsetLeft + selected.offsetWidth > rail.scrollLeft + rail.clientWidth)
      rail.scrollLeft = selected.offsetLeft + selected.offsetWidth - rail.clientWidth + 4;
  }, [season, seasons]);
  useLayoutEffect(() => {
    const rail = list.current;
    if (!rail) return;
    const current = rail.querySelector<HTMLElement>('[aria-current="true"]');
    rail.scrollLeft = current ? Math.max(0, current.offsetLeft + current.offsetWidth - rail.clientWidth + 8) : 0;
  }, [rows, target?.item.id]);

  return (
    <div class="series-episodes">
      <div class="movie-actions series-actions">
        <button class="primary movie-play series-play" disabled={!target && episodeLoad !== "failed"}
          onClick={() => {
            if (episodeLoad === "failed") reloadEpisodes();
            else if (target) { frames.stop(); playDirect(target.item, target.position); }
          }}>
          {episodeLoad === "failed" ? t("Folgen erneut laden", "Reload episodes") : `▶ ${t("Abspielen", "Play")}`}
        </button>
        {!!ordered.length && <button class="series-random" onClick={() => {
          const episode = ordered[Math.floor(Math.random() * ordered.length)];
          if (episode) { frames.stop(); playDirect(episode, 0, true); }
        }}>{t("Zufällige Folge", "Random episode")}</button>}
        {actions}
      </div>
      {ordered.length ? <>
        <h3>{t("Staffeln", "Seasons")}</h3>
        <div ref={tabs} class="season-buttons series-rail" role="group" aria-label={t("Staffeln", "Seasons")}>
          {seasons.map(s => (
            <button key={s} data-focus-key={`season:${s}`} class={s === season ? "selected" : ""}
              aria-pressed={s === season} onClick={() => setChosenSeason(s)}>
              {t("Staffel", "Season")} {s}
            </button>
          ))}
        </div>
        <div ref={list} class="episode-list series-rail" role="group" aria-label={t("Folgen", "Episodes")}>
          {rows.map((e, index) => {
            const saved = watched.get(itemKey(e));
            const frame = previews ? frames.image(e) : "";
            const art = artSource(e.preview);
            const image = frame || (previews && art && art !== artSource(seriesCover) && artCounts.get(art) === 1 ? e.preview : "");
            const active = target && itemKey(target.item) === itemKey(e);
            const progress = saved && saved.duration > 0
              ? Math.max(0, Math.min(1, saved.position / saved.duration)) : 0;
            return (
              <button key={e.id} data-episode-index={index} data-focus-key={itemKey(e)} aria-current={active ? "true" : undefined}
                onClick={() => { frames.stop(); play(e, saved?.position || 0); }}>
                <span class="episode-art" aria-hidden="true">
                  <span>▷</span>
                  {image && <img key={image} src={image} alt="" loading="lazy"
                    onLoad={event => {
                      const img = event.currentTarget;
                      img.hidden = !frame && img.naturalWidth < img.naturalHeight * 1.25;
                    }}
                    onError={event => { event.currentTarget.hidden = true; }} />}
                  {progress > 0 && <progress value={progress} max={1} />}
                </span>
                <strong>{t("Folge", "Episode")} {e.episode ?? 1}</strong>
              </button>
            );
          })}
        </div>
      </> : episodeLoad !== "failed" && <p role="status">{episodeLoad === "ready"
        ? t("Keine Folgen verfügbar.", "No episodes available.")
        : t("Folgen werden geladen …", "Loading episodes…")}</p>}
    </div>
  );
}

function Description({ text, action }: { text: string; action?: string }) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null),
    close = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => {
    if (open) close.current?.focus();
  }, [open]);
  useLayoutEffect(() => {
    if (!open) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape" || e.keyCode === 10009) {
        e.preventDefault();
        e.stopPropagation();
        setOpen(false);
        trigger.current?.focus();
      }
    };
    document.addEventListener("keydown", key, true);
    return () => document.removeEventListener("keydown", key, true);
  }, [open]);
  return (
    <>
      {!action && <p class="description-excerpt">
        {text || "Informationen werden vom Anbieter bereitgestellt."}
      </p>}
      {text && (
        <button ref={trigger} class="read-more" onClick={() => setOpen(true)}>
          {action || "Mehr lesen"}
        </button>
      )}
      {open &&
        createPortal(
          <div class="scrim">
            <section
              class="modal glass"
              role="dialog"
              aria-modal="true"
              aria-label="Beschreibung"
            >
              <h2>Beschreibung</h2>
              <p>{text}</p>
              <button
                ref={close}
                onClick={() => {
                  setOpen(false);
                  trigger.current?.focus();
                }}
              >
                Schließen
              </button>
            </section>
          </div>,
          document.getElementById("app")!,
        )}
    </>
  );
}

// Only the viewport and adjacent rows exist in the DOM, irrespective of catalogue size.
function PosterGrid({
  items,
  selected,
  select,
  favoriteKeys,
  emptyMessage,
  classic,
  resetKey,
}: {
  items: Item[];
  selected: Item | null;
  select: (i: Item) => void;
  favoriteKeys: ReadonlySet<string>;
  emptyMessage: string;
  classic: boolean;
  resetKey: string;
}) {
  const root = useRef<HTMLDivElement>(null);
  const [top, setTop] = useState(0),
    [width, setWidth] = useState(600),
    [height, setHeight] = useState(700);
  const columns = Math.max(2, Math.floor(width / (classic ? appearance.classicPosterWidth : appearance.catalogPosterWidth))),
    rowHeight = classic ? appearance.classicPosterRowHeight : 248;
  useLayoutEffect(() => {
    const el = root.current!;
    const resize = () => {
      setWidth(el.clientWidth);
      setHeight(el.clientHeight);
    };
    resize();
    if (typeof ResizeObserver !== "undefined") {
      const observer = new ResizeObserver(resize);
      observer.observe(el);
      return () => observer.disconnect();
    }
    // The navigation changes the grid width without resizing the TV viewport.
    const observer = new MutationObserver(resize);
    const shell = el.closest(".shell");
    if (shell) observer.observe(shell, { attributes: true, attributeFilter: ["class"] });
    window.addEventListener("resize", resize);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", resize);
    };
  }, []);
  useLayoutEffect(() => {
    if (root.current) root.current.scrollTop = 0;
    setTop(0);
  }, [resetKey]);
  const previousColumns = useRef(columns);
  useLayoutEffect(() => {
    if (previousColumns.current !== columns && root.current) {
      const anchor =
        Math.floor(root.current.scrollTop / rowHeight) *
        previousColumns.current;
      const next = Math.floor(anchor / columns) * rowHeight;
      root.current.scrollTop = next;
      setTop(next);
      previousColumns.current = columns;
    }
  }, [columns]);
  const start = Math.max(0, Math.floor(top / rowHeight) - 1),
    end = Math.min(
      Math.ceil(items.length / columns),
      start + Math.ceil(height / rowHeight) + 3,
    );
  const visible = items.slice(start * columns, end * columns);
  return (
    <div
      ref={root}
      class="catalog-title-list"
      onScroll={(e) => { const next = Math.floor(e.currentTarget.scrollTop / rowHeight) * rowHeight; setTop(old => old === next ? old : next); }}
    >
      <div style={{ height: start * rowHeight }} />
      <div
        class="catalog-grid"
        data-focus-grid={columns}
        style={{ gridTemplateColumns: `repeat(${columns},minmax(0,1fr))` }}
      >
        {visible.map((i, n) => (
          <button
            key={itemKey(i)}
            data-grid-index={start * columns + n}
            data-focus-key={itemKey(i)}
            data-favorite={favoriteKeys.has(itemKey(i))}
            class={`catalog-poster ${selected?.id === i.id ? "selected" : ""}`}
            onClick={() => select(i)}
          >
            <span class="poster-art">
              <span class="poster-favorite"><FavoriteMark marked={favoriteKeys.has(itemKey(i))} /></span>
              {i.logo ? (
                <img
                  src={i.logo}
                  alt=""
                  loading="lazy"
                  decoding="async"
                  onError={(e) => {
                    e.currentTarget.style.visibility = "hidden";
                  }}
                />
              ) : (
                <span>▷</span>
              )}
            </span>
            <strong>{i.name}</strong>
          </button>
        ))}
      </div>
      <div
        style={{
          height:
            Math.max(0, Math.ceil(items.length / columns) - end) * rowHeight,
        }}
      />
      {!items.length && <p class="muted">{emptyMessage}</p>}
    </div>
  );
}

export function CatalogBrowser({
  profileId,
  legend,
  kind,
  t,
  classic,
  items,
  categories,
  category,
  setCategory,
  query,
  selected,
  select,
  close,
  favorites,
  detail,
  episodes,
  episodeLoad,
  reloadEpisodes,
  history,
  watchHistory,
  toggle,
  play,
  playDirect,
  allowEpisodePreviews,
}: {
  profileId: string;
  legend: import("preact").ComponentChildren;
  kind: "series" | "vod";
  t: (de: string, en: string) => string;
  classic: boolean;
  items: Item[];
  categories: Category[];
  category: string;
  setCategory: (c: string) => void;
  query: string;
  selected: Item | null;
  select: (i: Item) => void;
  close: () => void;
  favorites: Item[];
  detail: Item | null;
  episodes: Item[];
  episodeLoad: EpisodeLoad;
  reloadEpisodes: (item: Item) => void;
  history: Watched[];
  watchHistory: boolean;
  toggle: (i: Item) => void;
  play: (i: Item, pos: number) => void;
  allowEpisodePreviews: (entry: Item) => boolean;
  playDirect: (item: Item, position: number, untracked?: boolean) => void;
}) {
  const favoriteKeys = useMemo(() => new Set(favorites.map(itemKey)), [favorites]);
  const categoryFavorites = category === "favorites" ? favoriteKeys : null;
  const [alphabetic, setAlphabetic] = useState(false);
  const continueIds = useMemo(() => category === "continue" ? new Set(history.flatMap(({item}) =>
    kind === "series" && item.seriesId ? [item.seriesId] : kind === "vod" && item.kind === "vod" ? [item.id] : [],
  )) : null, [category, history, kind]);
  const categoryCounts = useMemo(() => {
    const counts = new Map<string, number>();
    if (classic) for (const item of items) counts.set(item.category, (counts.get(item.category) || 0) + 1);
    return counts;
  }, [classic, items]);
  const choices = [
    { id: "favorites", name: t("Favoriten", "Favorites") },
    { id: "all", name: t("Alle Titel", "All titles") },
    ...(classic ? [
      { id: "recent", name: t("Neu hinzugefügt", "Recently added") },
      ...(watchHistory ? [{ id: "continue", name: t("Fortsetzen", "Continue watching") }] : []),
    ] : []),
    ...categories,
  ];
  const count = (id: string) => id === "all" ? items.length : id === "favorites" ? favorites.filter((i) => i.kind === kind).length : categoryCounts.get(id);
  const filtered = useMemo(() => {
    const q = query.trim().toLocaleLowerCase();
    const result = items.filter(
      (i) =>
        (q.length > 0 || category === "all" || category === "recent" ||
          (category === "favorites"
            ? categoryFavorites?.has(itemKey(i))
            : category === "continue" ? continueIds?.has(i.id) : i.category === category)) &&
        i.name.toLocaleLowerCase().includes(q),
    );
    if (alphabetic) result.sort((left, right) => left.name.localeCompare(right.name));
    else if (category === "recent") result.sort((left, right) => (right.added || 0) - (left.added || 0));
    return result;
  }, [items, category, query, categoryFavorites, continueIds, classic, alphabetic]);
  const item = detail?.id === selected?.id ? detail : selected;
  const preview = useRef<HTMLElement>(null);
  const [activation, setActivation] = useState(0);
  const activated = useRef<{key: string; origin: Element | null; waiting: HTMLElement | null} | null>(null);
  const activate = (value: Item) => {
    activated.current = {key: itemKey(value), origin: document.activeElement, waiting: null};
    select(value);
    setActivation((revision) => revision + 1);
  };
  useLayoutEffect(() => {
    const request = activated.current;
    if (!item || request?.key !== itemKey(item)) return;
    if (document.activeElement !== document.body && document.activeElement !== request.origin && document.activeElement !== request.waiting) {
      activated.current = null;
      return;
    }
    const primary = preview.current?.querySelector<HTMLButtonElement>("button.primary:not(:disabled)");
    if (primary) {
      primary.focus({preventScroll: true});
      activated.current = null;
    } else if (!request.waiting) {
      request.waiting = preview.current?.querySelector<HTMLButtonElement>("button") || null;
      request.waiting?.focus({preventScroll: true});
    }
  }, [activation, item, episodes]);
  return (
    <div class="catalog-browser">
      <section class="categories" data-focus-zone="categories">
        <h1>{t("Kategorien", "Categories")}</h1>
        {classic && <div class="classic-catalog-tools">
          <button class="catalog-sort" onClick={() => setAlphabetic(!alphabetic)} aria-pressed={alphabetic}>
            {alphabetic ? t("Sortierung: A–Z", "Sort: A–Z") : t("Sortierung: Anbieter", "Sort: Provider")}
          </button>
        </div>}
        <div class="category-list">
          {choices.map((c) => (
            <button
              key={c.id}
              data-focus-key={`category:${c.id}`}
              class={`${category === c.id ? "selected" : ""}${classic && count(c.id) !== undefined ? " with-category-count" : ""}`}
              title={c.name}
              onClick={() => setCategory(c.id)}
            >
              {c.name}
              {classic && count(c.id) !== undefined && <small class="category-count">{count(c.id)}</small>}
            </button>
          ))}
        </div>
      </section>
      <section class="catalog-titles" data-focus-zone="titles">
        <header class="catalog-heading">
          <h1>{classic ? choices.find((choice) => choice.id === category)?.name : kind === "series" ? t("Serien", "Series") : t("Filme", "Movies")}</h1>
          {legend}
        </header>
        {!classic && <div class="catalog-tools"><button class="catalog-sort" onClick={() => setAlphabetic(!alphabetic)} aria-pressed={alphabetic}>
          {alphabetic ? t("Sortierung: A–Z", "Sort: A–Z") : t("Sortierung: Anbieter", "Sort: Provider")}
        </button></div>}
        <PosterGrid items={filtered} selected={selected} select={activate} favoriteKeys={favoriteKeys} classic={classic}
          resetKey={JSON.stringify([category, query, alphabetic])}
          emptyMessage={t("Keine passenden Titel.", "No matching titles.")} />
      </section>
      {item && <TitleDetail item={item} profileId={profileId} t={t} panel={preview} close={close}
        episodeLoad={episodeLoad} reloadEpisodes={() => reloadEpisodes(item)}
        episodes={episodes} history={history} marked={favoriteKeys.has(itemKey(item))}
        toggle={toggle} play={play} playDirect={playDirect} previews={allowEpisodePreviews(item)} />}
    </div>
  );
}


export function TitleDetail({item, profileId, t, panel, close, episodes, history, marked, toggle, play, playDirect, previews, episodeLoad, reloadEpisodes}: {
  profileId: string;
  item: Item;
  t: (de: string, en: string) => string;
  panel: RefObject<HTMLElement>;
  close: () => void;
  episodes: Item[];
  history: Watched[];
  marked: boolean;
  previews: boolean;
  episodeLoad: EpisodeLoad;
  reloadEpisodes: () => void;
  toggle: (item: Item) => void;
  play: (item: Item, position: number) => void;
  playDirect: (item: Item, position: number, untracked?: boolean) => void;
}) {
  const position = item.kind === "vod" ? history.find(h => itemKey(h.item) === itemKey(item))?.position || 0 : 0;
  const favorite = <button class="favorite-toggle" aria-label={t("Favorit", "Favorite")}
    aria-pressed={marked} onClick={() => toggle(item)}>
    <svg aria-hidden="true" viewBox="0 0 24 24" fill={marked ? "currentColor" : "none"}
      stroke="currentColor" stroke-width="1.8" stroke-linejoin="round">
      <path d="M12 20.5 3.7 12a5 5 0 0 1 7.1-7L12 6.2 13.2 5a5 5 0 0 1 7.1 7Z" />
    </svg>
  </button>;
  return (
      <aside ref={panel} class={`catalog-details movie-detail${item.kind === "series" ? " series-detail" : ""}`} data-focus-zone="details">
        {item.logo && <img key={itemKey(item)} alt="" class="movie-art" src={item.logo}
          onError={(event) => { event.currentTarget.hidden = true; }} />}
        <header class="catalog-detail-header">
          <button class="menu-back" onClick={close}>{t("‹ Zurück", "‹ Back")}</button>
        </header>
        <div class="title-copy" key={`copy:${itemKey(item)}`}>
          <h2>{item.name}</h2>
          {item.description && <p>{item.description}</p>}
        </div>
        {item.kind === "vod" ? (
          <div class="movie-controls" key={itemKey(item)}>
            <div class="movie-actions">
              <button class="primary movie-play" onClick={() => play(item, position)}>
                {t("▶ Abspielen", "▶ Play")}
              </button>
              {favorite}
            </div>
          </div>
        ) : (
          <div class="series-controls" key={itemKey(item)}>
            <EpisodePicker episodes={episodes} history={history} play={play} playDirect={playDirect} t={t} previews={previews} seriesCover={item.logo} profileId={profileId}
              episodeLoad={episodeLoad} reloadEpisodes={reloadEpisodes}
              actions={<>{favorite}{item.description && <Description text={item.description}
                action={t("Mehr lesen", "Read more")} />}</>} />
          </div>
        )}
      </aside>
  );
}
