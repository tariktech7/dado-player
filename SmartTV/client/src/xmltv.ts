import { Epg } from "./types";
export function xmltvTime(value: string): number {
  const m =
    /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})?(?:\s*([+-])(\d{2})(\d{2})|\s*Z)?$/.exec(
      value.trim(),
    );
  if (!m) return NaN;
  const utc = Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +(m[6] || 0));
  const offset =
    (+(m[8] || 0) * 60 + +(m[9] || 0)) * 60 * (m[7] === "+" ? 1 : -1);
  return utc / 1000 - offset;
}
export function parseXmltv(xml: string): Map<string, Epg[]> {
  if (xml.length > 30_000_000)
    throw new Error("XMLTV-Datei zu groß (maximal 30 MB).");
  const doc = new DOMParser().parseFromString(xml, "text/xml");
  if (doc.querySelector("parsererror") || !doc.querySelector("tv"))
    throw new Error("XMLTV-Antwort ungültig.");
  const result = new Map<string, Epg[]>(),
    now = Date.now() / 1000;
  for (const node of doc.querySelectorAll("programme")) {
    const start = xmltvTime(node.getAttribute("start") || ""),
      end = xmltvTime(node.getAttribute("stop") || ""),
      id = node.getAttribute("channel") || "";
    if (
      !id ||
      !Number.isFinite(start) ||
      !Number.isFinite(end) ||
      end <= now ||
      start > now + 172800
    )
      continue;
    const rows = result.get(id) || [];
    if (rows.length < 100) {
      rows.push({
        start,
        end,
        title: node.querySelector("title")?.textContent || "Ohne Titel",
        description: node.querySelector("desc")?.textContent || "",
      });
      result.set(id, rows);
    }
  }
  for (const rows of result.values()) rows.sort((a, b) => a.start - b.start);
  return result;
}
