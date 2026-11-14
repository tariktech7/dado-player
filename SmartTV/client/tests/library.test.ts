import { test } from "node:test";
import assert from "node:assert/strict";
import { resumeEpisode } from "../src/library";
import { Item } from "../src/types";
const episode = (id: string, season: number, number: number): Item => ({
  id,
  season,
  episode: number,
  seriesId: "s",
  kind: "episode",
  name: "avx.raw.title",
  category: "",
  logo: "",
  url: "https://example.invalid/a.mp4",
});
test("series continue orders seasons, advances at 60 seconds and ignores other series", () => {
  const rows = [episode("3", 2, 1), episode("2", 1, 2), episode("1", 1, 1)];
  assert.equal(resumeEpisode(rows, [])?.item.id, "1");
  const watched = { item: rows[1], position: 1739, duration: 1800, at: 2 };
  assert.equal(resumeEpisode(rows, [watched])?.position, 1739);
  assert.equal(
    resumeEpisode(rows, [{ ...watched, position: 1740 }])?.item.id,
    "3",
  );
  assert.equal(
    resumeEpisode(rows, [{ ...watched, duration: 0 }])?.item.id,
    "2",
  );
  assert.equal(
    resumeEpisode(rows, [{ ...watched, item: episode("other", 9, 9) }])?.item
      .id,
    "1",
  );
});
