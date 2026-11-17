import test from "node:test";
import assert from "node:assert/strict";
import { Provider, parseM3u, hostUrl, decode, streamExtension } from "../src/api";
import { fresh, load, save, favorite, remember } from "../src/store";
import { Profile, Item } from "../src/types";
import {cloudProfile,mergeCloudProfiles} from '../src/connect';
test('saved cloud templates retain identity across sends and update only their own imported profile',()=>{
 const sourceId='11111111-1111-4111-8111-111111111111';
 const first=cloudProfile({...profile,sourceId},'22222222-2222-4222-8222-222222222222');
 const second=cloudProfile({...profile,sourceId,password:'new-secret'},'33333333-3333-4333-8333-333333333333');
 assert.equal(first.id,second.id);
 const result=mergeCloudProfiles([profile,first],[second,second]);
 assert.equal(result.profiles.length,2);assert.equal(result.added,0);
 assert.equal(result.profiles[0],profile);assert.equal(result.profiles[1].password,'new-secret');
});
test('warm default migrates automatic appearance once and preserves explicit dark/white choices',()=>{
  assert.equal(fresh().settings.theme,'warm');
  for(const theme of ['system','dark','light'] as const){
    const s=fresh();s.settings.theme=theme;delete s.settings.appearanceVersion;
    assert.equal(load({getItem:()=>JSON.stringify(s)}).settings.theme,theme==='system'?'warm':theme);
  }
  const s=fresh();s.settings.theme='system';
  assert.equal(load({getItem:()=>JSON.stringify(s)}).settings.theme,'system');
});
import { xmltvTime } from "../src/xmltv";
test('provider movie and series catalogues reject the opposite identifier and stream type',async()=>{
  const fetcher: typeof fetch = async(input)=>{
    const action=new URL(String(input)).searchParams.get('action');
    if(!action) return Response.json({user_info:{auth:1},server_info:{}});
    const rows=action==='get_vod_streams'?
      [{stream_id:7,name:'Film',stream_type:'movie'},{series_id:8,name:'Wrong show'},{stream_id:9,name:'Tagged show',stream_type:'series'}]:
      action==='get_series'?
      [{series_id:7,name:'Show',stream_type:'series'},{stream_id:8,name:'Wrong movie'},{series_id:9,name:'Tagged movie',stream_type:'movie'}]:[];
    return Response.json(rows);
  };
  const catalogue=await new Provider(profile,fresh().settings,fetcher).catalog();
  assert.deepEqual(catalogue.vod.map(v=>v.name),['Film']);
  assert.deepEqual(catalogue.series.map(v=>v.name),['Show']);
});
import { resolvePackagedMedia } from "../src/player";
const profile: Profile = {
  id: "p1",
  name: "Test",
  kind: "Xtream",
  host: "https://provider.invalid",
  username: "a+b",
  password: "p/a?",
  m3uUrl: "",
  epgUrl: "",
};
const item: Item = {
  id: "1",
  name: "Test",
  kind: "live",
  category: "all",
  url: "https://provider.invalid/1",
  logo: "",
};
test("Samsung packaged test media resolves the real widget file URI, not WebKit virtual root", async () => {
  const url = await resolvePackagedMedia("file:///test-video.mp4", {
    resolve: (path: string, success: (file: any) => void) => {
      assert.equal(path, "wgt-package/test-video.mp4");
      success({
        toURI: () => "file:///opt/usr/apps/DadoPlayTV/res/wgt/test-video.mp4",
      });
    },
  });
  assert.match(url, /\/opt\/usr\/apps\/DadoPlayTV/);
  assert.equal(
    await resolvePackagedMedia("https://example.invalid/live.ts", null),
    "https://example.invalid/live.ts",
  );
});
test("Xtream credentials are encoded in API and stream URLs", () => {
  const p = new Provider(profile, fresh().settings);
  assert.equal(new URL(p.endpoint()).searchParams.get("username"), "a+b");
  assert.match(p.stream("live", 1, "m3u8"), /a%2Bb\/p%2Fa%3F\/1.m3u8$/);
  assert.equal(hostUrl("provider.invalid/"), "http://provider.invalid");
  assert.throws(() => hostUrl("file:///etc/passwd"));
});
test("partial provider failures retain other libraries and report the missing area", async () => {
  const p = new Provider(profile, fresh().settings, (async (input) => {
    const action = new URL(String(input)).searchParams.get("action");
    if (action === "get_vod_streams") return new Response("", { status: 503 });
    if (!action) return Response.json({ user_info: { auth: 1 } });
    return Response.json(
      action.includes("categories")
        ? [{ category_id: 1, category_name: "Test" }]
        : [{ stream_id: 1, series_id: 2, name: "Test", category_id: 1 }],
    );
  }) as typeof fetch);
  const lib = await p.catalog();
  assert.equal(lib.live.length, 1);
  assert.equal(lib.series.length, 1);
  assert.equal(lib.vod.length, 0);
  assert.match(lib.issues.join(" "), /vod/);
});
test("network errors do not include secrets or request URLs", async () => {
  const p = new Provider(profile, fresh().settings, (async () => {
    throw new Error("https://provider.invalid?password=SECRET");
  }) as typeof fetch);
  await assert.rejects(
    p.catalog(),
    (e) =>
      !String(e).includes("SECRET") && !String(e).includes("provider.invalid"),
  );
});
test("M3U ordering does not change channel identity; unsupported protocols are rejected", () => {
  const a =
    '#EXTM3U\n#EXTINF:-1 group-title="Sport" tvg-id="one",One\nhttps://example.invalid/one\n#EXTINF:-1,Two\nhttps://example.invalid/two';
  const b =
    '#EXTM3U\n#EXTINF:-1,Two\nhttps://example.invalid/two\n#EXTINF:-1 group-title="Sport",One\nhttps://example.invalid/one';
  assert.equal(parseM3u(a).live[0].id, parseM3u(b).live[1].id);
  assert.equal(
    parseM3u("#EXTM3U\n#EXTINF:-1,Unsafe\njavascript:alert(1)").live.length,
    0,
  );
  assert.throws(() => parseM3u("<html>not a playlist</html>"));
});
test("favorites and history stay profile-scoped; Live never resumes at a position", () => {
  const s = fresh();
  s.favorites.p1 = favorite(s, "p1", item);
  assert.equal(s.favorites.p1.length, 1);
  assert.equal(favorite(s, "p2", item).length, 1);
  s.favorites.p1 = favorite(s, "p1", item);
  assert.equal(s.favorites.p1.length, 0);
  s.history.p1 = remember(s, "p1", item, 123, 500);
  assert.equal(s.history.p1[0].position, 0);
  assert.equal(s.history.p2, undefined);
});
test("corrupted storage recovers and persistence failures remain visible", () => {
  assert.deepEqual(load({ getItem: () => "{broken" }), fresh());
  assert.throws(
    () =>
      save(fresh(), {
        setItem: () => {
          throw 0;
        },
      }),
    /Speicher/,
  );
  assert.equal(decode("R3LDvMOfZQ=="), "Grüße");
});
test("XMLTV timezone offsets are honored; omitted zone defaults to UTC", () => {
  assert.equal(
    xmltvTime("20261114180000 +0100"),
    Date.UTC(2026, 10, 14, 17) / 1000,
  );
  assert.equal(
    xmltvTime("20261114180000 -0300"),
    Date.UTC(2026, 10, 14, 21) / 1000,
  );
  assert.equal(xmltvTime("20261114180000"), Date.UTC(2026, 10, 14, 18) / 1000);
  assert.ok(Number.isNaN(xmltvTime("broken")));
});

test("VOD/episode URLs preserve provider containers and normalize decorated extensions", () => {
  const p = new Provider(profile, fresh().settings);
  for (const [raw, expected] of [[" .MKV ", "mkv"], ["avi", "avi"], ["MP4", "mp4"], ["../../bad", "mp4"], ["", "mp4"]]) {
    assert.equal(streamExtension(raw), expected);
    assert.ok(p.stream("series", "episode01", raw).endsWith("/episode01." + expected));
    assert.ok(p.stream("movie", 7, raw).endsWith("/7." + expected));
  }
});
