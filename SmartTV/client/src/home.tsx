import { ComponentChildren } from "preact";
import { Catalog, Item, Watched } from "./types";

export function Home({
  name,
  catalog,
  history,
  favorites,
  program,
  play,
  section,
  logo,
}: {
  name: string;
  catalog: Catalog;
  history: Watched[];
  favorites: Item[];
  program: (i: Item) => string;
  play: (i: Item, pos: number) => void;
  section: (title: string, items: Item[]) => ComponentChildren;
  logo: (i: Item) => ComponentChildren;
}) {
  const featured =
    history.find((h) => h.item.kind !== "live") ||
    (catalog.vod[0]
      ? { item: catalog.vod[0], position: 0, duration: 0 }
      : null);
  return (
    <div class="home-dashboard">
      <div class="home-primary" data-focus-zone="home-content">
        <header class="page-title">
          <span class="eyebrow">DEIN PROGRAMM</span>
          <h1>Guten Tag, {name}.</h1>
          <p>Alles an einem Ort. Einfach weiterschauen.</p>
        </header>
        {featured ? (
          <section class="home-hero glass">
            <div class="home-hero-copy">
              <span class="eyebrow">
                {featured.position > 1 ? "WEITER ANSEHEN" : "FÜR DICH"}
              </span>
              <h2>{featured.item.name}</h2>
              <p>
                {featured.position > 1
                  ? `Bei ${Math.floor(featured.position / 60)} Minuten`
                  : "Dein nächster Film wartet schon."}
              </p>
              <button
                class="primary"
                onClick={() => play(featured.item, featured.position)}
              >
                ▶ {featured.position > 1 ? "Fortsetzen" : "Abspielen"}
              </button>
              {featured.duration > 0 && (
                <progress value={featured.position} max={featured.duration} />
              )}
            </div>
            <div class="home-hero-art">
              {featured.item.logo ? (
                <img src={featured.item.logo} alt="" />
              ) : (
                <><img class="hero-fallback brand-normal" src="d7.svg" alt="" /><img class="hero-fallback brand-dark" src="d7-dark.svg" alt="" /></>
              )}
            </div>
          </section>
        ) : (
          <section class="home-hero glass">
            <div class="home-hero-copy">
              <span class="eyebrow">WILLKOMMEN ZUHAUSE</span>
              <h2>Dein Programm. Dein Moment.</h2>
              <p>
                Starte einen Sender oder Titel. Hier kannst du später direkt
                weiterschauen.
              </p>
            </div>
            <div class="home-hero-art">
              <><img class="hero-fallback brand-normal" src="d7.svg" alt="" /><img class="hero-fallback brand-dark" src="d7-dark.svg" alt="" /></>
            </div>
          </section>
        )}
        {section(
          "Zuletzt gesehene Sender",
          history.filter((h) => h.item.kind === "live").map((h) => h.item),
        )}
        {section(
          "Weiter ansehen · Serien",
          history.filter((h) => h.item.kind === "episode").map((h) => h.item),
        )}
        {section(
          "Weiter ansehen · Filme",
          history.filter((h) => h.item.kind === "vod").map((h) => h.item),
        )}
        {section("Live-TV entdecken", catalog.live.slice(0, 20))}
        {section("Filme entdecken", catalog.vod.slice(0, 20))}
        {section("Serien entdecken", catalog.series.slice(0, 20))}
      </div>
      <aside class="home-favorites glass" data-focus-zone="home-favorites">
        <span class="eyebrow">DEINE SENDER · JETZT</span>
        <h2>Das läuft auf deinen Favoriten</h2>
        {favorites.length ? (
          <div class="favorite-programs">
            {favorites.slice(0, 30).map((item) => (
              <button onClick={() => play(item, 0)}>
                {logo(item)}
                <span>
                  <strong>{item.name}</strong>
                  <span>{program(item)}</span>
                </span>
                <span>▷</span>
              </button>
            ))}
          </div>
        ) : (
          <p class="muted">Markiere Sender mit dem Stern in der Vorschau.</p>
        )}
      </aside>
    </div>
  );
}
