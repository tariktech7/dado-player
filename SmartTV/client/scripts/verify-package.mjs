import { readFile } from "node:fs/promises";
import { unzipSync } from "fflate";
import { createHash } from "node:crypto";
const version = JSON.parse(await readFile("package.json", "utf8")).version;
const file =
  process.argv[2] || `../../Releases/Dado-Player-Tizen-${version}-unsigned.wgt`;
const data = await readFile(file);
if (data.length > 20_000_000) throw new Error("Unexpected widget size");
const entries = unzipSync(data),
  names = Object.keys(entries),
  text = (n) => new TextDecoder().decode(entries[n]);
for (const file of [
  "index.html",
  "config.xml",
  "app.js",
  "app.css",
  "icon.png",
  "d7.svg",
  "fonts/SourceSans3-Regular.ttf",
  "fonts/SourceSans3-Bold.ttf",
  "fonts/SourceSans3-Semibold.ttf",
])
  if (!entries[file]) throw new Error(`Missing ${file}`);
if (
  names.some((n) =>
    /(^|\/)(\.\.|node_modules)(\/|$)|\.p12$|\.pfx$|\.local\.json$|\.ts$|^\.manifest/.test(
      n,
    ),
  )
)
  throw new Error("Unexpected private/source content in widget");
if (
  !text("config.xml").includes('id="DadoPlayTV.Player"') ||
  !text("config.xml").includes(`version="${version}"`)
)
  throw new Error("Identity/version mismatch");
if (!text("index.html").includes("$WEBAPIS/webapis/webapis.js"))
  throw new Error("Missing Samsung API loader");
const signed = !!entries["author-signature.xml"] && !!entries["signature1.xml"];
if (file.includes("-unsigned") && signed)
  throw new Error("Unsigned artifact contains signatures");
console.log(
  JSON.stringify(
    {
      file,
      bytes: data.length,
      sha256: createHash("sha256").update(data).digest("hex"),
      files: names.length,
      signatureFilesPresent: signed,
    },
    null,
    2,
  ),
);
if (signed)
  console.log(
    "Signature presence only: Samsung device installation must validate the certificate chain and DUID.",
  );
