import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "preact/hooks";
import { createPortal } from "preact/compat";
import { Item, Category, Watched, itemKey } from "./types";
import { TVInput } from "./input";
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

export function EpisodePicker({
  episodes,
  history,
  play,
}: {
  episodes: Item[];
  history: Watched[];
  play: (i: Item, pos: number) => void;
}) {
  const ordered = useMemo(
    () =>
      [...episodes].sort(
        (a, b) =>
          (a.season || 1) - (b.season || 1) ||
          (a.episode || 1) - (b.episode || 1),
      ),
    [episodes],
  );
  const seasons = useMemo(
    () => [...new Set(ordered.map((e) => e.season || 1))],
    [ordered],
  );
  const target = useMemo(
    () => resumeEpisode(ordered, history),
    [ordered, history],
  );
  const [season, setSeason] = useState(target?.item.season || 1);
  const tabs = useRef<HTMLDivElement>(null),
    list = useRef<HTMLDivElement>(null);
  useLayoutEffect(
    () => setSeason(target?.item.season || seasons[0] || 1),
    [episodes, target?.item.id],
  );
  useLayoutEffect(() => {
    const selected = tabs.current?.querySelector<HTMLElement>(".selected");
    if (selected && tabs.current)
      tabs.current.scrollLeft = Math.max(0, selected.offsetLeft - 12);
    const episode = list.current?.querySelector<HTMLElement>(
      '[aria-current="true"]',
    );
    if (list.current)
      list.current.scrollTop = episode ? Math.max(0, episode.offsetTop - 6) : 0;
  }, [season, episodes]);
  return (
    <div class="series-episodes">
      {target && (
        <button
          class="primary series-play"
          onClick={() => play(target.item, target.position)}
        >
          ▶ {target.position > 1 ? "Fortsetzen" : "Abspielen"} · S
          {target.item.season || 1} F{target.item.episode || 1}
        </button>
      )}
      <div ref={tabs} class="season-buttons" aria-label="Staffeln">
        {seasons.map((s) => (
          <button
            key={s}
            data-focus-key={`season:${s}`}
            class={s === season ? "selected" : ""}
            onClick={() => setSeason(s)}
          >
            Staffel {s}
          </button>
        ))}
      </div>
      <div ref={list} class="episode-list">
        {ordered
          .filter((e) => (e.season || 1) === season)
          .map((e) => (
            <button
              key={e.id}
              data-focus-key={itemKey(e)}
              aria-current={target?.item.id === e.id ? "true" : undefined}
              onClick={() =>
                play(
                  e,
                  history.find((h) => itemKey(h.item) === itemKey(e))
                    ?.position || 0,
                )
              }
            >
              <strong>Folge {e.episode || 1}</strong>
              <span>
                {target?.item.id === e.id && target.position > 1
                  ? "Fortsetzen ▷"
                  : "▷"}
              </span>
            </button>
          ))}
      </div>
      {!episodes.length && <p>Folgen werden geladen …</p>}
    </div>
  );
}

function Description({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null),
    close = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => {
    if (open) close.current?.focus();
  }, [open]);
  useEffect(() => {
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
      <p class="description-excerpt">
        {text || "Informationen werden vom Anbieter bereitgestellt."}
      </p>
      {text && (
        <button ref={trigger} class="read-more" onClick={() => setOpen(true)}>
          Mehr lesen
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
}: {
  items: Item[];
  selected: Item | null;
  select: (i: Item) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const [top, setTop] = useState(0),
    [width, setWidth] = useState(600),
    [height, setHeight] = useState(700);
  const columns = Math.max(2, Math.floor(width / 146)),
    rowHeight = 248;
  useLayoutEffect(() => {
    const el = root.current!;
    const resize = () => {
      setWidth(el.clientWidth);
      setHeight(el.clientHeight);
    };
    resize();
    const obs = new ResizeObserver(resize);
    obs.observe(el);
    return () => obs.disconnect();
  }, []);
  useLayoutEffect(() => {
    if (root.current) root.current.scrollTop = 0;
    setTop(0);
  }, [items]);
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
            class={`catalog-poster ${selected?.id === i.id ? "selected" : ""}`}
            onClick={() => select(i)}
          >
            <span class="poster-art">
              {i.logo ? (
                <img
                  src={i.logo}
                  alt=""
                  loading="lazy"
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
      {!items.length && <p>Keine passenden Titel.</p>}
    </div>
  );
}

export function CatalogBrowser({
  items,
  categories,
  category,
  setCategory,
  query,
  setQuery,
  selected,
  select,
  favorites,
  detail,
  episodes,
  history,
  toggle,
  play,
}: {
  items: Item[];
  categories: Category[];
  category: string;
  setCategory: (c: string) => void;
  query: string;
  setQuery: (q: string) => void;
  selected: Item | null;
  select: (i: Item) => void;
  favorites: Item[];
  detail: Item | null;
  episodes: Item[];
  history: Watched[];
  toggle: (i: Item) => void;
  play: (i: Item, pos: number) => void;
}) {
  const filtered = useMemo(() => {
    const keys = new Set(favorites.map(itemKey)),
      q = query.toLocaleLowerCase();
    return items.filter(
      (i) =>
        (category === "all" ||
          (category === "favorites"
            ? keys.has(itemKey(i))
            : i.category === category)) &&
        i.name.toLocaleLowerCase().includes(q),
    );
  }, [items, category, query, favorites]);
  const item = detail?.id === selected?.id ? detail : selected;
  const preview = useRef<HTMLElement>(null);
  const activated = useRef<string | null>(null);
  const activate = (value: Item) => { activated.current = itemKey(value); select(value); };
  useLayoutEffect(() => {
    if (!item || activated.current !== itemKey(item)) return;
    const primary = preview.current?.querySelector<HTMLButtonElement>('.series-play,button.primary');
    if (primary) { primary.focus({preventScroll:true}); activated.current = null; }
  }, [item, episodes]);
  return (
    <div class="catalog-browser">
      <section class="categories" data-focus-zone="categories">
        <h1>Kategorien</h1>
        <div class="category-list">
          {[
            { id: "favorites", name: "Favoriten" },
            { id: "all", name: "Alle Titel" },
            ...categories,
          ].map((c) => (
            <button
              key={c.id}
              data-focus-key={`category:${c.id}`}
              class={category === c.id ? "selected" : ""}
              title={c.name}
              onClick={() => setCategory(c.id)}
            >
              {c.name}
            </button>
          ))}
        </div>
      </section>
      <section class="catalog-titles" data-focus-zone="titles">
        <h1>{items[0]?.kind === "series" ? "Serien" : "Filme"}</h1>
        <TVInput
          type="search"
          aria-label="Titel suchen"
          placeholder="Titel suchen"
          value={query}
          onInput={(e) => setQuery(e.currentTarget.value)}
        />
        <PosterGrid items={filtered} selected={selected} select={activate} />
      </section>
      <aside ref={preview} class="catalog-details" data-focus-zone="details">
        <h2>{item?.kind === "series" ? "Serie & Folgen" : "Details"}</h2>
        {item ? (
          <div class="catalog-detail-scroll" key={itemKey(item)}>
            <div class="catalog-heading">
              {item.logo && <img src={item.logo} alt="" />}
              <div>
                <h2>{item.name}</h2>
                <button onClick={() => toggle(item)}>
                  {favorites.some((f) => itemKey(f) === itemKey(item))
                    ? "★ Favorit"
                    : "☆ Merken"}
                </button>
              </div>
            </div>
            <Description text={item.description || ""} />
            {item.kind === "series" ? (
              <EpisodePicker
                episodes={episodes}
                history={history}
                play={play}
              />
            ) : (
              <button
                class="primary"
                onClick={() =>
                  play(
                    item,
                    history.find((h) => itemKey(h.item) === itemKey(item))
                      ?.position || 0,
                  )
                }
              >
                ▶ Abspielen / Fortsetzen
              </button>
            )}
          </div>
        ) : (
          <div class="empty-preview">
            <span>▷</span>
            <h3>Titel auswählen</h3>
            <p>Informationen und Wiedergabe bleiben hier sichtbar.</p>
          </div>
        )}
      </aside>
    </div>
  );
}
