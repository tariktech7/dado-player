import { build } from "esbuild";
import { cp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
const testConfig = process.env.DADO_LICENSE_TEST_CONFIG;
const root = fileURLToPath(new URL("..", import.meta.url));
process.chdir(root);
const out = path.resolve("dist");
if (out !== path.join(root, "dist")) throw new Error("Unexpected output path");
await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
await build({
  entryPoints: ["src/app.tsx"],
  bundle: true,
  outfile: "dist/app.js",
  format: "iife",
  target: ["chrome85"],
  jsx: "automatic",
  jsxImportSource: "preact",
  minify: true,
  external: ["./fonts/*"],
  legalComments: "eof",
  plugins: testConfig ? [{name:'test-license-key',setup(b){b.onLoad({filter:/licensing[\\/]public-config\.json$/},async()=>({contents:await readFile(testConfig,'utf8'),loader:'json'}));}}] : [],
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
await cp("../../ios/Resources/Playback.mp4", "dist/test-video.mp4");
await cp("../../assets/icon.png", "dist/icon.png");
await cp("../../assets/fonts/LICENSE.md", "dist/FONT-LICENSE.md");
await cp("node_modules/preact/LICENSE", "dist/PREACT-LICENSE.txt");
const pkg = JSON.parse(await readFile("package.json", "utf8"));
await writeFile(
  "dist/config.xml",
  (await readFile("tizen/config.xml", "utf8")).replace(
    "__VERSION__",
    pkg.version,
  ),
);
await writeFile(
  "dist/index.html",
  `<!doctype html><html lang="de"><head><meta charset="utf-8"><meta name="viewport" content="width=1920,initial-scale=1"><meta name="application-name" content="Dado Player"><title>Dado Player</title><link rel="stylesheet" href="app.css"></head><body><div id="app"></div><script>(function(){if(typeof tizen!=='undefined'){var s=document.createElement('script');s.src='$WEBAPIS/webapis/webapis.js';s.onload=s.onerror=function(){var a=document.createElement('script');a.src='app.js';document.body.appendChild(a)};document.head.appendChild(s)}else{var a=document.createElement('script');a.src='app.js';document.body.appendChild(a)}})();</script></body></html>`,
);
await writeFile(
  "dist/version.json",
  JSON.stringify(
    {
      name: "Dado Player",
      version: pkg.version,
      platform: "Samsung Tizen",
      minimumTizen: "6.5",
      licenseTestBuild: !!testConfig,
    },
    null,
    2,
  ),
);
console.log(`Dado Player Tizen ${pkg.version}: ${out}`);
