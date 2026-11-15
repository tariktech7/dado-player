import { test, expect, Page } from "@playwright/test";
import { fresh } from "../../src/store";
import {readFileSync} from 'node:fs';
import {fingerprint,signLicense} from '../../../../licensing/shared/protocol.mjs';
const profile = {
  id: "qa",
  name: "TV Demo",
  kind: "Xtream" as const,
  host: "https://provider.invalid",
  username: "demo",
  password: "demo",
  m3uUrl: "",
  epgUrl: "",
};
async function fixture(page: Page, seed = true, native = false, licenseKind: 'permanent'|'expired'|'missing' = 'permanent') {
  const subject=await fingerprint('samsung','tv-ui-fixture'),now=Math.floor(Date.now()/1000);
  const key=JSON.parse(readFileSync(process.env.DADO_LICENSE_TEST_KEYS!,'utf8'));
  const token=await signLicense({v:1,product:'dado-player',subject,platform:'samsung',license_id:'ui-only',kind:licenseKind==='expired'?'trial':'permanent',started_at:licenseKind==='expired'?now-604801:now,issued_at:now,expires_at:licenseKind==='expired'?now-1:null},key,'ui-test');
  await page.addInitScript(({token,now,licenseKind})=>{
    const api=(window as any).webapis ||= {};
    api.productinfo={getDuid:()=> 'tv-ui-fixture'};api.network={getMac:()=> 'AA:BB:CC:DD:EE:10'};
    if(licenseKind!=='missing' && !localStorage.getItem('dado.license.v1'))localStorage.setItem('dado.license.v1',JSON.stringify({token,high:now,lastCheck:now}));
  },{token,now,licenseKind});
  await page.route("https://provider.invalid/**", async (route) => {
    const url = new URL(route.request().url()),
      action = url.searchParams.get("action"),
      now = Math.floor(Date.now() / 1000);
    let data: unknown = [];
    if (url.pathname !== "/player_api.php") {
      return route.fulfill({ status: 404, body: "" });
    }
    if (!action) data = { user_info: { auth: 1 } };
    else if (action.endsWith("_categories"))
      data = Array.from({ length: 12 }, (_, i) => ({
        category_id: String(i + 1),
        category_name:
          ["Sport", "Nachrichten", "Dokumentation", "Unterhaltung"][i % 4] +
          " " +
          (i + 1),
      }));
    else if (action === "get_live_streams")
      data = Array.from({ length: 4800 }, (_, i) => ({
        stream_id: i + 1,
        name: `Dado Kanal ${String(i + 1).padStart(4, "0")}`,
        category_id: String((i % 12) + 1),
        stream_icon: `https://logos.invalid/${i % 2 ? "tall" : "wide"}.svg`,
      }));
    else if (action === "get_vod_streams")
      data = [
        {
          stream_id: 1,
          name: "Demo Film",
          category_id: "1",
          container_extension: "mp4",
        },
      ];
    else if (action === "get_series")
      data = [
        {
          series_id: 2,
          name: "Demo Serie",
          category_id: "1",
          cover: "https://logos.invalid/wide.svg",
        },
      ];
    else if (action === "get_short_epg")
      data = {
        epg_listings: [
          {
            title: "Sport am Abend",
            description: "Analysen, Interviews und die Höhepunkte des Tages.",
            start_timestamp: now - 600,
            stop_timestamp: now + 1200,
          },
          {
            title: "Die Nachrichten",
            description: "Danach im Programm.",
            start_timestamp: now + 1200,
            stop_timestamp: now + 2400,
          },
        ],
      };
    else if (action === "get_vod_info")
      data = { info: { plot: "Ein Film für die Testbibliothek." } };
    else if (action === "get_series_info")
      data = {
        episodes: {
          "1": [
            {
              id: 10,
              title: "Der Anfang",
              season: 1,
              episode_num: 1,
              container_extension: "mp4",
            },
            {
              id: 11,
              title: "Die Fortsetzung",
              season: 1,
              episode_num: 2,
              container_extension: "mp4",
            },
          ],
          "2": [
            {
              id: 12,
              title: "avx.skyfall.raw",
              season: 2,
              episode_num: 1,
              container_extension: "mp4",
            },
          ],
        },
      };
    await route.fulfill({
      json: data,
      headers: { "Access-Control-Allow-Origin": "*" },
    });
  });
  await page.route("https://logos.invalid/**", (route) =>
    route.fulfill({
      contentType: "image/svg+xml",
      body: `<svg xmlns="http://www.w3.org/2000/svg" width="${route.request().url().includes("wide") ? 320 : 80}" height="${route.request().url().includes("wide") ? 80 : 160}"><rect width="100%" height="100%" rx="12" fill="#71dfd1"/><circle cx="50%" cy="50%" r="22" fill="white"/></svg>`,
    }),
  );
  if (seed) {
    const s = fresh();
    s.profiles = [profile];
    s.activeProfile = profile.id;
    await page.addInitScript((state) => {
      if (!localStorage.getItem("dado.tv.v1"))
        localStorage.setItem("dado.tv.v1", JSON.stringify(state));
    }, s);
  }
  if (native) {
    await page.route("**/$WEBAPIS/webapis/webapis.js", (route) =>
      route.fulfill({ contentType: "text/javascript", body: "" }),
    );
    await page.addInitScript(() => {
      let state = "NONE",
        listener: any,
        time = 0;
      const log: any[] = [];
      (window as any).__avLog = log;
      (window as any).tizen = {
        tvinputdevice: { registerKey() {} },
        application: {
          getCurrentApplication: () => ({ exit: () => log.push(["exit"]) }),
        },
      };
      (window as any).webapis = {
        productinfo:{getDuid:()=> 'tv-ui-fixture'},
        network:{getMac:()=> 'AA:BB:CC:DD:EE:10'},
        appcommon: {
          setScreenSaver: (...a: any[]) => log.push(["screensaver", a[0]]),
        },
        avplay: {
          getState: () => state,
          open: (url: string) => {
            state = "IDLE";
            log.push(["open", url]);
          },
          setListener: (l: any) => (listener = l),
          setBufferingParam() {},
          setStreamingProperty() {},
          prepareAsync: (ok: any) =>
            setTimeout(() => {
              state = "READY";
              ok();
            }, 20),
          setDisplayRect: (...r: any[]) => log.push(["rect", ...r]),
          setDisplayMethod: (m: any) => log.push(["aspect", m]),
          play: () => {
            state = "PLAYING";
            log.push(["play"]);
          },
          pause: () => {
            state = "PAUSED";
            log.push(["pause"]);
          },
          seekTo: (n: number, ok: any) => {
            time = n;
            log.push(["seek", n]);
            ok?.();
          },
          stop: () => (state = "IDLE"),
          close: () => {
            state = "NONE";
            log.push(["close"]);
          },
          getCurrentTime: () => time,
          getDuration: () => 120000,
          getTotalTrackInfo: () => [
            {
              type: "AUDIO",
              index: 0,
              extra_info: '{"language":"deu","codec":"AAC"}',
            },
            { type: "TEXT", index: 1, extra_info: '{"language":"eng"}' },
          ],
          setSelectTrack: (...a: any[]) => log.push(["track", ...a]),
          setSilentSubtitle: () => {},
          suspend: () => log.push(["suspend"]),
          restoreAsync: (_u: any, _n: any, _b: any, ok: any) => {
            state = "PLAYING";
            ok();
          },
        },
      };
    });
  }
}
test('license settings show real MAC and Unlimited, and paid restarts never query the license server',async({page})=>{
  await fixture(page,true,true);
  let requests=0;page.on('request',r=>{if(r.url().startsWith('https://license-fixture.invalid'))requests++;});
  await page.goto('/');
  await page.getByRole('button',{name:'⚙ Einstellungen',exact:true}).click();
  const section=page.locator('[data-license-status]');
  await expect(section.getByText('Unlimited',{exact:true})).toBeVisible();
  await expect(section).toContainText('AA:BB:CC:DD:EE:10');
  await expect(section).toContainText('D7-');
  await page.reload();
  await page.getByRole('button',{name:'⚙ Einstellungen',exact:true}).click();
  await expect(section.getByText('Unlimited',{exact:true})).toBeVisible();
  expect(requests).toBe(0);
});

test('expired trial replaces catalogue with device gate and verified activation restores access',async({page})=>{
  await fixture(page,true,true,'expired');
  const now=Math.floor(Date.now()/1000),subject=await fingerprint('samsung','tv-ui-fixture');
  const key=JSON.parse(readFileSync(process.env.DADO_LICENSE_TEST_KEYS!,'utf8'));
  const token=await signLicense({v:1,product:'dado-player',subject,platform:'samsung',license_id:'ui-only',kind:'permanent',started_at:now-604801,issued_at:now,expires_at:null},key,'ui-test');
  let calls=0;
  await page.route('https://license-fixture.invalid/v1/license/activation',async route=>{calls++;await route.fulfill({json:{license:token}});});
  await page.goto('/');
  const gate=page.locator('[data-license-gate]');
  await expect(gate.getByRole('heading',{name:'Ihre Testversion ist abgelaufen'})).toBeVisible();
  await expect(gate).toContainText('AA:BB:CC:DD:EE:10');
  await expect(gate).toContainText('D7-');
  expect((await gate.locator('.welcome-panel').boundingBox())!.width).toBeGreaterThan(800);
  await page.screenshot({path:test.info().outputPath('license-expired.png')});
  await expect(page.locator('.navigation')).toHaveCount(0);
  await page.keyboard.press('ArrowRight');await page.keyboard.press('Escape');
  await expect(gate).toBeVisible();expect(calls).toBe(0);
  await gate.getByRole('button',{name:'Freischaltung abrufen',exact:true}).click();
  await expect(gate).toHaveCount(0);expect(calls).toBe(1);
  await page.reload();await expect(gate).toHaveCount(0);expect(calls).toBe(1);
});

test('first launch requires explicit trial start and offline failure keeps device gate available',async({page})=>{
  await fixture(page,false,true,'missing');
  let calls=0;
  await page.route('https://license-fixture.invalid/v1/trial/start',async route=>{calls++;await route.abort();});
  await page.goto('/');
  const gate=page.locator('[data-license-gate]');
  const start=gate.getByRole('button',{name:'7 Tage kostenlos testen',exact:true});
  await expect(start).toBeEnabled();expect(calls).toBe(0);
  await start.click();await expect(gate.getByRole('status')).toBeVisible();
  await expect(start).toBeEnabled();await expect(gate).toContainText('AA:BB:CC:DD:EE:10');
  expect(calls).toBe(1);
});

test('activation cannot start a trial; explicit start shows remaining days and paid activation removes badge',async({page})=>{
  await fixture(page,true,true,'missing');
  const now=Math.floor(Date.now()/1000),subject=await fingerprint('samsung','tv-ui-fixture');
  const key=JSON.parse(readFileSync(process.env.DADO_LICENSE_TEST_KEYS!,'utf8'));
  const base={v:1 as const,product:'dado-player',subject,platform:'samsung' as const,license_id:'ui-only',started_at:now,issued_at:now};
  const trial=await signLicense({...base,kind:'trial',expires_at:now+604800},key,'ui-test');
  const paid=await signLicense({...base,kind:'permanent',expires_at:null},key,'ui-test');
  const paths:string[]=[];let approved=false;
  await page.route('https://license-fixture.invalid/**',async route=>{
    const path=new URL(route.request().url()).pathname;paths.push(path);
    if(path==='/v1/trial/start')return route.fulfill({json:{license:trial}});
    if(path==='/v1/license/activation')return approved?route.fulfill({json:{license:paid}}):route.fulfill({status:409,json:{code:'NOT_APPROVED',error:'Noch keine Freischaltung vorhanden.'}});
    return route.fulfill({status:500});
  });
  await page.goto('/');const gate=page.locator('[data-license-gate]');
  await gate.getByRole('button',{name:'Freischaltung abrufen',exact:true}).click();
  await expect(gate.getByRole('status')).toHaveText('Noch keine Freischaltung vorhanden.');
  await expect(gate).toBeVisible();expect(paths).toEqual(['/v1/license/activation']);
  await gate.getByRole('button',{name:'7 Tage kostenlos testen',exact:true}).click();
  await expect(gate).toHaveCount(0);
  await expect(page.locator('[data-trial-badge]')).toContainText('7 Tage');
  await page.evaluate(()=>{const original=Date.now;Date.now=()=>original()+86400000;window.dispatchEvent(new Event('focus'));});
  await expect(page.locator('[data-trial-badge]')).toContainText('6 Tage');
  await page.getByRole('button',{name:'▣ Live TV',exact:true}).click();
  await page.locator('[data-focus-zone="categories"] button').first().focus();
  await expect(page.locator('.trial-badge-compact')).toBeVisible();
  await expect(page.locator('.trial-badge-compact')).toContainText('6 T');
  await page.screenshot({path:test.info().outputPath('trial-indicator.png')});
  approved=true;
  await page.getByRole('button',{name:'⚙ Einstellungen',exact:true}).click();
  const status=page.locator('[data-license-status]');
  await status.getByRole('button',{name:'Freischaltung abrufen',exact:true}).click();
  await expect(status.getByText('Unlimited',{exact:true})).toBeVisible();
  await expect(page.locator('[data-trial-badge]')).toHaveCount(0);
  await page.reload();await expect(gate).toHaveCount(0);
  expect(paths).toEqual(['/v1/license/activation','/v1/trial/start','/v1/license/activation']);
});

test("login is inline, saved profile resumes without a profile prompt", async ({
  page,
}) => {
  await fixture(page, false);
  await page.goto("/");
  await page.getByLabel("Profilname", { exact: true }).click();
  await page.getByLabel("Profilname", { exact: true }).fill("TV Demo");
  await page.getByLabel("Server-Adresse").click();
  await page.getByLabel("Server-Adresse").fill(profile.host);
  await page.getByLabel("Benutzername").click();
  await page.getByLabel("Benutzername").fill("demo");
  await page.getByLabel("Passwort", { exact: true }).click();
  await page.getByLabel("Passwort", { exact: true }).fill("demo");
  await page.getByRole("button", { name: "Verbinden & speichern" }).click();
  await expect(
    page.getByRole("heading", { name: "Guten Tag, TV Demo." }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Guten Tag, TV Demo." }),
  ).toBeVisible();
  await expect(page.getByText("Schön, dass du da bist.")).toHaveCount(0);
});
test("Full HD and 4K keep aligned headings, ten rows, side-by-side EPG and contain logos", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await expect(page.locator(".channel")).toHaveCount(24);
  await page.locator(".channel").first().click();
  await expect(page.locator(".preview-panel")).toBeVisible();
  await expect(page.locator(".epg-detail")).toContainText("Sport am Abend");
  for (const size of [
    { width: 1920, height: 1080 },
    { width: 3840, height: 2160 },
  ]) {
    await page.setViewportSize(size);
    await page.waitForTimeout(100);
    const geometry = await page.evaluate(() => {
      const rect = (s: string) =>
          document.querySelector(s)!.getBoundingClientRect(),
        brand = rect(".brand"),
        cat = rect(".categories>h1"),
        title = rect(".channels>h1"),
        list = rect(".channel-list");
      const visible = [...document.querySelectorAll(".channel")].filter(
        (el) => {
          const r = el.getBoundingClientRect();
          return r.top >= list.top && r.bottom <= list.bottom;
        },
      ).length;
      const [a, b] = [...document.querySelectorAll(".epg-pair small")]
        .slice(0, 2)
        .map((el) => el.getBoundingClientRect());
      return {
        brand: brand.y + brand.height / 2,
        cat: cat.y + cat.height / 2,
        title: title.y + title.height / 2,
        visible,
        gap: b.left - a.right,
        dy: a.y - b.y,
        fit: getComputedStyle(document.querySelector(".logo img")!).objectFit,
        dom: document.querySelectorAll(".channel").length,
      };
    });
    expect(Math.abs(geometry.brand - geometry.cat)).toBeLessThan(2);
    expect(Math.abs(geometry.cat - geometry.title)).toBeLessThan(2);
    expect(geometry.visible).toBeGreaterThanOrEqual(10);
    expect(geometry.gap).toBeGreaterThanOrEqual(47);
    expect(geometry.dy).toBe(0);
    expect(geometry.fit).toBe("contain");
    expect(geometry.dom).toBeLessThan(40);
    await page.screenshot({
      path: `test-results/tizen-live-${size.width}.png`,
    });
  }
});
test("Samsung player preserves stream across preview/fullscreen, exposes tracks and handles Back in order", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await page.locator(".channel").first().click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).__avLog.filter((a: any) => a[0] === "play").length,
      ),
    )
    .toBe(1);
  await page.locator(".channel").first().click();
  await expect(page.locator(".fullscreen")).toBeVisible();
  await page.getByRole("button", { name: "Ton & Untertitel" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByRole("button", { name: "deu · AAC" }).click();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".fullscreen")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".fullscreen")).toHaveCount(0);
  await expect(page.locator(".preview-panel")).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as any).__avLog.filter((a: any) => a[0] === "open").length,
    ),
  ).toBe(1);
  await page.keyboard.press("Escape");
  await expect(page.locator(".preview-panel")).toContainText(
    "Sender auswählen",
  );
});
test("favorites remain separate from history; series and settings work with remote input", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await page.locator(".channel").first().click();
  await page.getByRole("button", { name: "Sender als Favorit" }).click();
  await page.getByRole("button", { name: "⌂ Startseite", exact: true }).click();
  await expect(page.locator(".favorite-programs button")).toHaveCount(1);
  await expect(page.locator(".favorite-programs")).toContainText(
    "Dado Kanal 0001",
  );
  await page.getByRole("button", { name: "▤ Serien", exact: true }).click();
  await page.getByRole("button", { name: "Demo Serie" }).click();
  await expect(page.locator(".episode-list button")).toHaveCount(2);
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "⚙ Einstellungen", exact: true })
    .click();
  await page.getByRole("combobox", { name: "Live-Streamformat" }).click();
  await page.getByRole("option", { name: "MPEG-TS (.ts)" }).click();
  await page.reload();
  await page
    .getByRole("button", { name: "⚙ Einstellungen", exact: true })
    .click();
  await expect(
    page.getByRole("combobox", { name: "Live-Streamformat" }),
  ).toContainText("MPEG-TS (.ts)");
  await page.getByRole("button", { name: "⌂ Startseite", exact: true }).focus();
  await page.keyboard.press("ArrowDown");
  await expect(
    page.getByRole("button", { name: "▣ Live TV", exact: true }),
  ).toBeFocused();
});

test("virtualized remote navigation scrolls beyond the first window of channels", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await page.waitForTimeout(100);
  await page.locator(".channel").first().focus();
  for (let i = 0; i < 30; i++) await page.keyboard.press("ArrowDown");
  await expect(page.locator('.channel[data-channel="31"]')).toBeFocused();
  expect(await page.locator(".channel").count()).toBeLessThan(40);
});

test("TV choices only open with OK and never trap plain up/down navigation", async ({
  page,
}) => {
  await fixture(page, false);
  await page.goto("/");
  const method = page.getByRole("combobox", {
    name: "Anmeldemethode",
    exact: true,
  });
  await method.focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByLabel("Server-Adresse")).toBeFocused();
  await expect(method).toHaveAttribute("aria-expanded", "false");
  await expect(method).toContainText("Xtream Codes");
  await method.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("listbox", { name: "Anmeldemethode" }),
  ).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(method).toContainText("M3U-Wiedergabeliste");
  await expect(page.getByLabel("M3U-Adresse", { exact: true })).toBeVisible();
});

test("keyboard height reduction keeps full-size UI and scrolls the focused field into view", async ({
  page,
}) => {
  await fixture(page, false);
  await page.goto("/");
  const input = page.getByLabel("Passwort", { exact: true });
  await input.focus();
  const before = await input.boundingBox();
  await page.setViewportSize({ width: 1920, height: 620 });
  await expect(page.locator("#app")).toHaveClass(/keyboard-open/);
  await expect
    .poll(async () => {
      const r = await input.boundingBox();
      return r!.y + r!.height;
    })
    .toBeLessThanOrEqual(620);
  const after = await input.boundingBox();
  expect(after!.width).toBeCloseTo(before!.width, 0);
  expect(after!.height).toBeCloseTo(before!.height, 0);
  expect(
    await page
      .locator("#app")
      .evaluate((el) => el.getBoundingClientRect().width),
  ).toBe(1920);
  await page.screenshot({ path: "test-results/tizen-keyboard-layout.png" });
});

test("bundled test video decodes in HTML preview and controls hide then return with OK", async ({
  page,
}) => {
  await fixture(page, true, false);
  await page.goto("/");
  const h264 = await page.evaluate(
    () =>
      !!document
        .createElement("video")
        .canPlayType('video/mp4; codecs="avc1.42E01E, mp4a.40.2"'),
  );
  test.skip(
    !h264,
    "This Chromium distribution has no H.264/AAC decoder; run with DADO_BROWSER_CHANNEL=msedge or validate on the Samsung TV.",
  );
  await page
    .getByRole("button", { name: "⚙ Einstellungen", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Integriertes Testvideo abspielen" })
    .click();
  await expect
    .poll(() =>
      page
        .locator("#video-host video")
        .evaluate((el: HTMLVideoElement) => el.currentTime),
    )
    .toBeGreaterThan(0);
  const decoded = await page
    .locator("#video-host video")
    .evaluate((el: HTMLVideoElement) => ({
      width: el.videoWidth,
      height: el.videoHeight,
    }));
  expect(decoded.width).toBeGreaterThan(0);
  expect(decoded.height).toBeGreaterThan(0);
  await page.screenshot({ path: "test-results/tizen-html-playback.png" });
  await expect(page.locator(".fullscreen")).toHaveClass(/controls-hidden/, {
    timeout: 10000,
  });
  await page.keyboard.press("Enter");
  await expect(page.locator(".fullscreen")).not.toHaveClass(/controls-hidden/);
});

test("inputs stay read-only while navigating and activate only with OK", async ({
  page,
}) => {
  await fixture(page, false);
  await page.goto("/");
  const field = page.getByLabel("Server-Adresse");
  await field.focus();
  await expect(field).toHaveJSProperty("readOnly", true);
  await page.keyboard.press("ArrowDown");
  await expect(page.getByLabel("Benutzername")).toBeFocused();
  await field.focus();
  await page.keyboard.press("Enter");
  await expect(field).toHaveJSProperty("readOnly", false);
  await field.fill("https://provider.invalid");
  await page.keyboard.press("Enter");
  await expect(field).toHaveJSProperty("readOnly", true);
});

test("returning to the first form field restores the designed top spacing without moving the canvas", async ({
  page,
}) => {
  await fixture(page, false);
  await page.goto("/");
  await page.getByLabel("Passwort", { exact: true }).click();
  await page.setViewportSize({ width: 1920, height: 620 });
  await page.getByLabel("Profilname", { exact: true }).focus();
  await expect
    .poll(() => page.locator(".welcome-panel").evaluate((el) => el.scrollTop))
    .toBe(0);
  await expect
    .poll(() => page.locator("#app").evaluate((el) => el.scrollTop))
    .toBe(0);
  await expect
    .poll(() =>
      page
        .locator(".welcome-panel")
        .evaluate((el) => el.getBoundingClientRect().top),
    )
    .toBeGreaterThanOrEqual(39);
});

test("catalogues keep seasons, favorites and independent section positions; live preview stays reserved", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await expect(page.locator(".preview-panel")).toContainText(
    "Sender auswählen",
  );
  await page.locator(".channel-list").evaluate((e) => {
    e.scrollTop = 1200;
    e.dispatchEvent(new Event("scroll"));
  });
  await page.getByRole("button", { name: "▤ Serien", exact: true }).click();
  await expect(
    page.locator(".catalog-browser .category-list button").first(),
  ).toHaveText("Favoriten");
  await page.getByRole("button", { name: "Demo Serie", exact: true }).click();
  await expect(page.locator(".episode-list button")).toHaveCount(2);
  await expect(page.locator(".episode-list")).not.toContainText("Der Anfang");
  await page.getByRole("button", { name: "Staffel 2", exact: true }).click();
  await expect(page.locator(".episode-list button")).toHaveCount(1);
  await expect(page.locator(".episode-list button")).toHaveText("Folge 1▷");
  await page.getByRole("button", { name: "▷ Filme", exact: true }).click();
  await expect(
    page.locator(".catalog-browser .category-list button").first(),
  ).toHaveText("Favoriten");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await expect
    .poll(() => page.locator(".channel-list").evaluate((e) => e.scrollTop))
    .toBe(1200);
});

test("live remote up/down zaps and zero toggles the last two channels", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  await page.locator(".channel").first().click();
  await page.locator(".channel").first().click();
  await page.keyboard.press("ArrowUp");
  await expect(page.locator(".player-top")).toContainText("Dado Kanal 0002");
  await page.keyboard.press("0");
  await expect(page.locator(".player-top")).toContainText("Dado Kanal 0001");
  await page.keyboard.press("0");
  await expect(page.locator(".player-top")).toContainText("Dado Kanal 0002");
  await expect(page.locator(".fullscreen")).toHaveClass(/controls-hidden/, {
    timeout: 3500,
  });
  await page.keyboard.press("ArrowDown");
  await expect(page.locator(".player-top")).toContainText("Dado Kanal 0001");
});

test("all home rails scroll horizontally with remote focus and both menus split by kind", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  for (let i = 1; i <= 8; i++) {
    await page.locator(`.channel[data-channel="${i}"]`).click();
  }
  await page.getByRole("button", { name: "⌂ Startseite", exact: true }).click();
  const rail = page
    .locator(".home-section")
    .filter({
      has: page.getByRole("heading", {
        name: "Zuletzt gesehene Sender",
        exact: true,
      }),
    })
    .locator(".card-rail");
  await rail.locator("button").first().focus();
  for (let i = 0; i < 7; i++) await page.keyboard.press("ArrowRight");
  await expect(rail.locator("button").nth(7)).toBeFocused();
  expect(await rail.evaluate((e) => e.scrollLeft)).toBeGreaterThan(0);
  const last = await rail.locator("button").nth(7).boundingBox(),
    box = await rail.boundingBox();
  expect(last!.x + last!.width).toBeLessThanOrEqual(box!.x + box!.width + 1);
  for (let i = 0; i < 7; i++) await page.keyboard.press("ArrowLeft");
  await expect.poll(() => rail.evaluate((e) => e.scrollLeft)).toBe(0);
  await expect(page.locator(".home-hero")).toBeVisible();
  await expect(page.locator(".home-favorites")).toContainText(
    "Das läuft auf deinen Favoriten",
  );
  await page.screenshot({ path: "test-results/tizen-home-014.png" });
  await page.getByRole("button", { name: "☆ Favoriten", exact: true }).click();
  await expect(page.getByRole("tab")).toHaveText([
    "Live-TV",
    "Serien",
    "Filme",
  ]);
  await page.getByRole("tab", { name: "Serien", exact: true }).click();
  await page.getByRole("button", { name: "↻ Fortsetzen", exact: true }).click();
  await expect(page.getByRole("tab")).toHaveText([
    "Live-TV",
    "Serien",
    "Filme",
  ]);
  await page.getByRole("tab", { name: "Filme", exact: true }).click();
  await expect(page.locator(".history-list>div")).toHaveCount(0);
  await page.getByRole("button", { name: "☆ Favoriten", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Serien", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.screenshot({ path: "test-results/tizen-favorites-014.png" });
});

test("column return restores semantic category/channel focus and home favorites is reachable", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await page.getByRole("button", { name: "▣ Live TV", exact: true }).click();
  const category = page
    .locator(".category-list button")
    .filter({ hasText: /^Sport 1$/ });
  await category.click();
  const channel = page.locator('.channel[data-channel="13"]');
  await channel.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(category).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect(channel).toBeFocused();
  await channel.click();
  await page.getByRole("button", { name: "Sender als Favorit" }).click();
  await page.getByRole("button", { name: "⌂ Startseite", exact: true }).click();
  const play = page.locator(".home-hero button");
  await play.focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator(".home-favorites button").first()).toBeFocused();
  await page.keyboard.press("ArrowLeft");
  await expect(play).toBeFocused();
});

test("home episode artwork resolves fresh series cover when the saved episode has none", async ({
  page,
}) => {
  await fixture(page, true, true);
  await page.goto("/");
  await expect(page.locator(".home-hero")).toBeVisible();
  await page.evaluate(() => {
    const s = JSON.parse(localStorage.getItem("dado.tv.v1")!);
    s.history.qa = [
      {
        item: {
          id: "10",
          kind: "episode",
          seriesId: "2",
          season: 1,
          episode: 1,
          name: "Staffel 1 · Folge 1",
          category: "1",
          logo: "",
          url: "https://provider.invalid/series/demo/demo/10.mp4",
        },
        position: 200,
        duration: 1800,
        at: Date.now(),
      },
    ];
    localStorage.setItem("dado.tv.v1", JSON.stringify(s));
  });
  await page.reload();
  await expect(page.locator(".home-hero-art img")).toHaveAttribute(
    "src",
    "https://logos.invalid/wide.svg",
  );
  const rail = page.locator(".home-section").filter({
    has: page.getByRole("heading", {
      name: "Weiter ansehen · Serien",
      exact: true,
    }),
  });
  await expect(rail.locator("img")).toHaveAttribute(
    "src",
    "https://logos.invalid/wide.svg",
  );
});

async function bigCatalogue(page: Page) {
  await fixture(page, true, true);
  await page.route(
    "https://provider.invalid/player_api.php**",
    async (route) => {
      const action = new URL(route.request().url()).searchParams.get("action");
      if (action === "get_vod_streams")
        return route.fulfill({
          headers: { "Access-Control-Allow-Origin": "*" },
          json: Array.from({ length: 20000 }, (_, n) => ({
            stream_id: n + 1,
            name: `Film ${String(n + 1).padStart(5, "0")}`,
            category_id: "1",
            stream_icon: "https://logos.invalid/tall.svg",
            container_extension: "mp4",
          })),
        });
      if (action === "get_series")
        return route.fulfill({
          headers: { "Access-Control-Allow-Origin": "*" },
          json: [
            {
              series_id: 2,
              name: "Modern Family",
              category_id: "1",
              cover: "https://logos.invalid/tall.svg",
              plot: "Eine ausführliche Beschreibung für die ganze Familie. ".repeat(
                30,
              ),
            },
          ],
        });
      if (action === "get_series_info")
        return route.fulfill({
          headers: { "Access-Control-Allow-Origin": "*" },
          json: {
            episodes: Object.fromEntries(
              Array.from({ length: 12 }, (_, s) => [
                String(s + 1),
                Array.from({ length: 20 }, (_, n) => ({
                  id: (s + 1) * 100 + n + 1,
                  season: s + 1,
                  episode_num: n + 1,
                  title: "avx.provider.filename",
                  container_extension: "mp4",
                })),
              ]),
            ),
          },
        });
      return route.fallback();
    },
  );
}

test("large poster catalogue stays bounded and rail expands only on navigation focus", async ({
  page,
}) => {
  await bigCatalogue(page);
  await page.goto("/");
  await page.getByRole("button", { name: "▷ Filme", exact: true }).click();
  await expect(page.locator(".catalog-poster").first()).toBeVisible();
  expect(await page.locator(".catalog-poster").count()).toBeLessThan(80);
  const category = page
    .locator(".catalog-browser .category-list button")
    .nth(2);
  await category.focus();
  await expect(page.locator(".shell")).toHaveClass(/nav-collapsed/);
  await expect
    .poll(() =>
      page
        .locator(".navigation")
        .evaluate((e) => e.getBoundingClientRect().width),
    )
    .toBe(80);
  expect(
    await page
      .locator(".catalog-browser .categories")
      .evaluate((e) => e.getBoundingClientRect().width),
  ).toBe(320);
  await page.keyboard.press("ArrowLeft");
  await expect(
    page.getByRole("button", { name: "▷ Filme", exact: true }),
  ).toBeFocused();
  await expect(page.locator(".shell")).not.toHaveClass(/nav-collapsed/);
  await category.focus();
  await page.keyboard.press("ArrowRight");
  await page.locator(".catalog-poster").first().focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator('[data-grid-index="1"]')).toBeFocused();
  await page.locator(".catalog-title-list").evaluate((e) => {
    e.scrollTop = 100000;
    e.dispatchEvent(new Event("scroll"));
  });
  await expect
    .poll(() =>
      page.locator(".catalog-poster").first().getAttribute("data-grid-index"),
    )
    .not.toBe("0");
  expect(await page.locator(".catalog-poster").count()).toBeLessThan(80);
  await page.screenshot({ path: "test-results/tizen-poster-grid.png" });
});

test("series details restore season five and episode six, keep episodes visible, and reuse cached details", async ({
  page,
}) => {
  await bigCatalogue(page);
  await page.goto("/");
  await expect(page.locator(".home-hero")).toBeVisible();
  await page.evaluate(() => {
    const s = JSON.parse(localStorage.getItem("dado.tv.v1")!);
    s.history.qa = [
      {
        item: {
          id: "506",
          kind: "episode",
          seriesId: "2",
          season: 5,
          episode: 6,
          name: "Staffel 5 · Folge 6",
          category: "1",
          logo: "",
          url: "https://provider.invalid/series/demo/demo/506.mp4",
        },
        position: 300,
        duration: 1800,
        at: Date.now(),
      },
    ];
    localStorage.setItem("dado.tv.v1", JSON.stringify(s));
  });
  let infoRequests = 0;
  page.on("request", (r) => {
    if (r.url().includes("action=get_series_info")) infoRequests++;
  });
  await page.reload();
  await page.getByRole("button", { name: "▤ Serien", exact: true }).click();
  await page
    .getByRole("button", { name: "Modern Family", exact: true })
    .click();
  await expect(page.locator(".season-buttons .selected")).toHaveText(
    "Staffel 5",
  );
  await expect(page.locator(".episode-list [aria-current=true]")).toContainText(
    "Folge 6",
  );
  expect(
    await page.locator(".season-buttons").evaluate((e) => e.scrollLeft),
  ).toBeGreaterThan(0);
  const box = await page.locator(".episode-list").boundingBox(),
    episode = await page
      .locator(".episode-list [aria-current=true]")
      .boundingBox();
  expect(episode!.y).toBeGreaterThanOrEqual(box!.y - 1);
  expect(episode!.y + episode!.height).toBeLessThanOrEqual(
    box!.y + box!.height + 1,
  );
  const tabs = await page.locator(".season-buttons button").all();
  const y = (await tabs[0].boundingBox())!.y;
  for (const tab of tabs)
    expect((await tab.boundingBox())!.y).toBeCloseTo(y, 0);
  await page.getByRole("button", { name: "Mehr lesen", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Beschreibung" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.screenshot({ path: "test-results/tizen-series-season5.png" });
  await page.getByRole("button", { name: "▷ Filme", exact: true }).click();
  await page.getByRole("button", { name: "▤ Serien", exact: true }).click();
  await page
    .getByRole("button", { name: "Modern Family", exact: true })
    .click();
  await expect(page.locator(".season-buttons .selected")).toHaveText(
    "Staffel 5",
  );
  await page
    .getByRole("button", { name: "Staffel 1", exact: true })
    .evaluate((e) => (e as HTMLButtonElement).click());
  await expect(page.locator(".season-buttons .selected")).toHaveText(
    "Staffel 1",
  );
  await page
    .getByRole("button", { name: "Modern Family", exact: true })
    .click();
  await expect(page.locator(".season-buttons .selected")).toHaveText(
    "Staffel 5",
  );
  expect(infoRequests).toBe(1);
});

test('home continue advances across seasons at one minute without a second resume prompt', async ({page}) => {
  await fixture(page, true, true);
  await page.goto('/');
  await expect(page.locator('.home-hero')).toBeVisible();
  await page.evaluate(() => {
    const saved = JSON.parse(localStorage.getItem('dado.tv.v1')!);
    saved.history.qa = [{item: {
      id:'11', kind:'episode', seriesId:'2', season:1, episode:2,
      name:'Staffel 1 · Folge 2', category:'1', logo:'',
      url:'https://provider.invalid/series/demo/demo/11.mp4',
    }, position:1740, duration:1800, at:Date.now()}];
    localStorage.setItem('dado.tv.v1', JSON.stringify(saved));
  });
  await page.reload();
  await page.locator('.home-hero').getByRole('button', {name:'▶ Fortsetzen', exact:true}).click();
  await expect.poll(() => page.evaluate(() => (window as any).__avLog
    .filter((entry:any[]) => entry[0]==='open').at(-1)?.[1])).toContain('/12.mp4');
  await expect.poll(() => page.evaluate(() => (window as any).__avLog
    .filter((entry:any[]) => entry[0]==='play').length)).toBeGreaterThan(0);
});
