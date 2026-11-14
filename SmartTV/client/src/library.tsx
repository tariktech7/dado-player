import { useEffect, useState } from "preact/hooks";
import { Item, Category, Watched, itemKey } from "./types";
import { TVInput } from "./input";

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
    watched.duration - watched.position <= 60
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
  const seasons = [...new Set(episodes.map((e) => e.season || 1))].sort(
    (a, b) => a - b,
  );
  const target = resumeEpisode(episodes, history);
  const [season, setSeason] = useState(target?.item.season || seasons[0] || 1);
  useEffect(
    () => setSeason(target?.item.season || seasons[0] || 1),
    [episodes],
  );
  return (
    <div class="series-episodes">
      {target && (
        <button
          class="primary series-play"
          onClick={() => play(target.item, target.position)}
        >
          ▶ {target.position > 1 ? "Fortsetzen" : "Abspielen"} · Staffel{" "}
          {target.item.season || 1} · Folge {target.item.episode || 1}
        </button>
      )}
      <div class="season-buttons" aria-label="Staffeln">
        {seasons.map((s) => (
          <button
            class={s === season ? "selected" : ""}
            onClick={() => setSeason(s)}
          >
            Staffel {s}
          </button>
        ))}
      </div>
      <div class="episode-list">
        {episodes
          .filter((e) => (e.season || 1) === season)
          .sort((a, b) => (a.episode || 0) - (b.episode || 0))
          .map((e) => (
            <button
              onClick={() =>
                play(
                  e,
                  history.find((h) => itemKey(h.item) === itemKey(e))
                    ?.position || 0,
                )
              }
            >
              <strong>Folge {e.episode || 1}</strong>
              <span>▷</span>
            </button>
          ))}
      </div>
      {!episodes.length && (
        <p>Folgen werden geladen oder sind nicht verfügbar.</p>
      )}
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
  const filtered = items.filter(
    (i) =>
      (category === "all" ||
        (category === "favorites"
          ? favorites.some((f) => itemKey(f) === itemKey(i))
          : i.category === category)) &&
      i.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  const item = detail?.id === selected?.id ? detail : selected;
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
              class={category === c.id ? "selected" : ""}
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
        <div class="catalog-title-list">
          {filtered.map((i) => (
            <button
              data-focus-key={itemKey(i)}
              class={`catalog-title ${selected?.id === i.id ? "selected" : ""}`}
              onClick={() => select(i)}
            >
              {i.logo && <img src={i.logo} loading="lazy" alt="" />}
              <strong>{i.name}</strong>
            </button>
          ))}
          {!filtered.length && <p>Keine passenden Titel.</p>}
        </div>
      </section>
      <aside class="catalog-details" data-focus-zone="details">
        <h2>{item?.kind === "series" ? "Serie & Folgen" : "Details"}</h2>
        {item ? (
          <div class="catalog-detail-scroll">
            <div class="catalog-heading">
              {item.logo && <img src={item.logo} alt="" />}
              <h2>{item.name}</h2>
            </div>
            <button onClick={() => toggle(item)}>
              {favorites.some((f) => itemKey(f) === itemKey(item))
                ? "★ Favorit"
                : "☆ Merken"}
            </button>
            {item.kind !== "series" && (
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
            <p>
              {item.description ||
                "Informationen werden vom Anbieter bereitgestellt."}
            </p>
            {item.kind === "series" && (
              <EpisodePicker
                episodes={episodes}
                history={history}
                play={play}
              />
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
