import { build } from "esbuild";
import { cp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { createHash } from 'node:crypto';
const testConfig = process.env.DADO_LICENSE_TEST_CONFIG;
const legacy = process.env.DADO_TIZEN_PROFILE === '2019';
const root = fileURLToPath(new URL("..", import.meta.url));
process.chdir(root);
const release = JSON.parse(await readFile('../../release.json', 'utf8'));
const pkg = JSON.parse(await readFile("package.json", "utf8"));
const publicVersion = release.revision === undefined ? release.version : `${release.version}.${release.revision}`;
if (pkg.version !== release.version || pkg.releaseVersion !== publicVersion) {
  throw new Error("Run python scripts/release_version.py before building");
}
const out = path.resolve("dist");
const testKeyPlugin = testConfig ? [{ name: 'test-update-key', setup(b) {
  b.onLoad({ filter: /tizen[\\/]update-key\.json$/ }, async () => ({
    contents: JSON.stringify(JSON.parse(await readFile(testConfig, 'utf8')).keys['ui-test']), loader: 'json',
  }));
}}] : [];
if (out !== path.join(root, "dist")) throw new Error("Unexpected output path");
await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
await build({
  // Both packages run exactly the same application and target the oldest supported TV.
  entryPoints: ["src/app.tsx"],
  bundle: true,
  outfile: "dist/app.js",
  format: "iife",
  target: ["chrome63"],
  jsx: "automatic",
  jsxImportSource: "preact",
  minify: true,
  external: ["./fonts/*"],
  legalComments: "eof",
  plugins: testConfig ? [...testKeyPlugin, {name:'test-license-key',setup(b){b.onLoad({filter:/licensing[\\/]public-config\.json$/},async()=>({contents:await readFile(testConfig,'utf8'),loader:'json'}));}}] : [],
  metafile: true,
}).then(async (result) => {
  await writeFile(
    "dist/build-meta.json",
    JSON.stringify(result.metafile, null, 2),
  );
});
await mkdir("dist/fonts", { recursive: true });
for (const weight of ["Regular", "Semibold", "Bold"])
  await cp(
    `../../assets/fonts/SourceSans3-${weight}.ttf`,
    `dist/fonts/SourceSans3-${weight}.ttf`,
  );
await cp("../../assets/d7.svg", "dist/d7.svg");
await cp("../../assets/d7-dark.svg", "dist/d7-dark.svg");
await cp("../../ios/Resources/Playback.mp4", "dist/test-video.mp4");
await cp("../../assets/icon.png", "dist/icon.png");
await cp("../../assets/fonts/LICENSE.md", "dist/FONT-LICENSE.md");
await cp("node_modules/preact/LICENSE", "dist/PREACT-LICENSE.txt");
await writeFile(
  "dist/config.xml",
  (await readFile("tizen/config.xml", "utf8")).replace('required_version="6.5"',legacy?'required_version="5.0"':'required_version="6.5"').replace(
    "__VERSION__",
    pkg.version,
  ),
);
const html = '<!doctype html><html lang="de"><head><meta charset="utf-8"><meta name="viewport" content="width=1920,initial-scale=1"><meta name="application-name" content="Dado Player"><title>Dado Player</title><link id="packaged-css" rel="stylesheet" href="app.css"></head><body><div id="app"></div><script src="boot.js"></script></body></html>';
await writeFile('dist/index.html', html);
const fingerprint = createHash('sha256').update(html).update(legacy ? 'legacy' : 'modern');
for (const name of ['src/boot.ts','src/update-format.ts','tizen/update-key.json','tizen/config.xml',
  'dist/d7.svg','dist/d7-dark.svg','dist/test-video.mp4','dist/icon.png',
  ...['Regular', 'Semibold', 'Bold'].map(weight => `dist/fonts/SourceSans3-${weight}.ttf`)]) {
  const content = await readFile(name);
  const stable = /\.(ts|json|xml)$/.test(name) ? content.toString('utf8').replace(/\r\n/g, '\n') : content;
  fingerprint.update(name).update(createHash('sha256').update(stable).digest());
}
const installed = { version: release.version, build: release.tizenBuild,
  variant: legacy ? 'legacy' : 'modern', compatibility: fingerprint.digest('hex') };
await build({entryPoints:['src/boot.ts'], bundle:true, outfile:'dist/boot.js', format:'iife',
  target:["chrome63"], minify:true, plugins:testKeyPlugin, define:{INSTALLED:JSON.stringify(installed)}});
await writeFile(
  "dist/version.json",
  JSON.stringify(
    {
      name: "Dado Player",
      version: pkg.version,
      releaseVersion: pkg.releaseVersion,
      platform: "Samsung Tizen",
      minimumTizen: legacy ? "5.0" : "6.5",
      browserTarget: "chrome63",
      licenseTestBuild: !!testConfig,
      update: installed,
    },
    null,
    2,
  ),
);
console.log(`Dado Player Tizen ${pkg.releaseVersion}: ${out}`);
