import {
  readdir,
  readFile,
  writeFile,
  mkdir,
  copyFile,
} from "node:fs/promises";
import { zipSync } from "fflate";
import { spawnSync } from "node:child_process";
import path from "node:path";
const version = JSON.parse(await readFile("package.json", "utf8")).version;
const profileIndex = process.argv.indexOf("--profile"),
  profile =
    profileIndex >= 0
      ? process.argv[profileIndex + 1]
      : process.env.TIZEN_SIGNING_PROFILE;
const release = path.resolve(process.env.DADO_PACKAGE_DIR || "../../Releases");
await mkdir(release, { recursive: true });
const entries = {};
async function walk(dir, prefix = "") {
  for (const ent of await readdir(dir, { withFileTypes: true })) {
    const key = prefix + ent.name;
    if (ent.isDirectory()) await walk(path.join(dir, ent.name), key + "/");
    else if (
      !/\.wgt$/.test(key) &&
      !key.startsWith(".") &&
      key !== "build-meta.json" &&
      key !== "author-signature.xml" &&
      !/^signature\d+\.xml$/.test(key)
    )
      entries[key] = new Uint8Array(await readFile(path.join(dir, ent.name)));
  }
}
await walk("dist");
if (!entries["config.xml"] || !entries["index.html"] || !entries["app.js"])
  throw new Error("Run npm run build first.");
let file;
if (profile) {
  const cli = process.env.TIZEN_CLI || "tizen";
  const args = [
    "package",
    "-t",
    "wgt",
    "-s",
    profile,
    "--",
    path.resolve("dist"),
  ];
  if ([cli, ...args].some((a) => /["%\r\n!^&|<>]/.test(a)))
    throw new Error("Unsupported shell characters in SDK path/profile.");
  const command =
    process.platform === "win32"
      ? `"${cli}" ${args.map((a) => '"' + a + '"').join(" ")}`
      : cli;
  const result = spawnSync(command, process.platform === "win32" ? [] : args, {
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  if (result.status !== 0) throw new Error("Tizen signing failed.");
  const packages = (await readdir("dist")).filter((n) => n.endsWith(".wgt"));
  if (packages.length !== 1)
    throw new Error("Expected exactly one signed widget.");
  file = path.join(release, `Dado-Player-Tizen-${version}.wgt`);
  await copyFile(path.join("dist", packages[0]), file);
} else {
  file = path.join(release, `Dado-Player-Tizen-${version}-unsigned.wgt`);
  await writeFile(file, zipSync(entries, { level: 9 }));
}
console.log(file);
console.log(
  profile
    ? "Signed with configured certificate profile."
    : "UNSIGNED: build artifact only. Sign with your Samsung TV certificate before installation.",
);
