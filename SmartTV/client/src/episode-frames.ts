
import { useEffect, useRef, useState } from "preact/hooks";
import { Item } from "./types";
import policy from "../../../shared/product-policy.json" with { type: "json" };

function readFrame(url: string, done: (image: string) => void): () => void {
  const video = document.createElement("video");
  video.crossOrigin = "anonymous";
  video.muted = true;
  video.preload = "auto";
  video.setAttribute("aria-hidden", "true");
  video.style.cssText = "position:fixed;left:-2px;top:0;width:1px;height:1px;pointer-events:none";
  document.body.appendChild(video);
  let finished = false;
  const finish = (image?: string) => {
    if (finished) return;
    finished = true;
    window.clearTimeout(timer);
    video.onloadedmetadata = video.onloadeddata = video.onseeked = video.onerror = null;
    video.removeAttribute("src");
    video.load();
    video.remove();
    if (image !== undefined) done(image);
  };
  let positioned = false;
  let candidates: number[] = [];
  const seekNext = () => {
    const position = candidates.shift();
    if (position === undefined) return finish("");
    positioned = false;
    try {
      video.currentTime = position;
    } catch {
      finish("");
    }
  };
  const capture = () => {
    if (video.readyState < 2 || !video.videoWidth || video.seeking) return;
    try {
      const canvas = document.createElement("canvas");
      canvas.width = 320;
      canvas.height = 180;
      const ctx = canvas.getContext("2d");
      if (!ctx) return finish("");
      const scale = Math.min(canvas.width / video.videoWidth, canvas.height / video.videoHeight);
      const width = video.videoWidth * scale, height = video.videoHeight * scale;
      ctx.fillStyle = "black";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(video, (canvas.width - width) / 2, (canvas.height - height) / 2, width, height);
      const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      for (let offset = 0; offset < pixels.length; offset += 148) {
        if (Math.max(pixels[offset], pixels[offset + 1], pixels[offset + 2]) > 28) {
          return finish(canvas.toDataURL("image/jpeg", 0.75));
        }
      }
      seekNext();
    } catch {
      finish("");
    }
  };
  video.onloadedmetadata = () => {
    const duration = video.duration;
    candidates = Number.isFinite(duration) && duration > 0
      ? policy.episodePreviewSeconds.filter(seconds => seconds < duration - 0.2)
      : [...policy.episodePreviewSeconds];
    if (!candidates.length) candidates.push(duration / 2);
    seekNext();
  };
  video.onloadeddata = () => { if (positioned) capture(); };
  video.onseeked = () => { positioned = true; capture(); };
  video.onerror = () => finish("");
  const timer = window.setTimeout(() => finish(""), 5500);
  video.src = url;
  return () => finish();
}

const images = new Map<string, string>();
let epoch = 0;
let database: Promise<IDBDatabase | undefined> | undefined;
const STORE = "frames";

function previewDatabase() {
  database ??= new Promise(resolve => {
    try {
      const request = indexedDB.open("dado-episode-previews", 1);
      request.onupgradeneeded = () => request.result.createObjectStore(STORE);
      request.onsuccess = () => resolve(request.result);
      request.onerror = request.onblocked = () => resolve(undefined);
    } catch {
      resolve(undefined);
    }
  });
  return database;
}

async function previewStorage<T>(mode: IDBTransactionMode, action: (store: IDBObjectStore) => IDBRequest<T>) {
  const db = await previewDatabase();
  if (!db) return undefined;
  return new Promise<T | undefined>(resolve => {
    try {
      const transaction = db.transaction(STORE, mode);
      const request = action(transaction.objectStore(STORE));
      transaction.oncomplete = () => resolve(request.result);
      transaction.onabort = transaction.onerror = () => resolve(undefined);
    } catch {
      resolve(undefined);
    }
  });
}

export async function clearEpisodeFrames() {
  epoch++;
  images.clear();
  await previewStorage("readwrite", store => store.clear());
}

export function useEpisodeFrames(
  rows: Item[], rail: {current: HTMLDivElement | null}, enabled: boolean, profileId: string,
) {
  const stop = useRef<() => void>(() => {});
  const [, repaint] = useState(0);
  const key = (episode: Item) => JSON.stringify([profileId, episode.seriesId, episode.id]);

  useEffect(() => {
    const root = rail.current;
    if (!root || !enabled || typeof IntersectionObserver === "undefined") return;
    const visible = new Set<number>();
    const failed = new Set<string>();
    const generation = epoch;
    let cancelled = false;
    let active = false;
    let abort: (() => void) | undefined;
    const current = () => !cancelled && generation === epoch;

    function complete(id: string, result: string) {
      if (!current()) return;
      if (result) {
        if (images.size >= 64) {
          const oldest = images.keys().next().value;
          if (oldest !== undefined) images.delete(oldest);
        }
        images.set(id, result);
        repaint(revision => revision + 1);
      } else {
        failed.add(id);
      }
      active = false;
      pump();
    }

    function pump() {
      if (active || !current()) return;
      const next = [...visible].find(index => {
        const episode = rows[index];
        if (!episode?.url) return false;
        const id = key(episode);
        return !images.has(id) && !failed.has(id);
      });
      if (next === undefined) return;
      const episode = rows[next];
      const id = key(episode);
      active = true;
      void previewStorage<string>("readonly", store => store.get(id)).then(cached => {
        if (!current()) return;
        if (cached) {
          complete(id, cached);
          return;
        }
        abort = readFrame(episode.url, result => {
          if (!current()) return;
          if (result) {
            void previewStorage("readwrite", store => current() ? store.add(result, id) : store.get(id)).then(() => complete(id, result));
          } else {
            complete(id, "");
          }
        });
      });
    }

    const observer = new IntersectionObserver(entries => {
      for (const entry of entries) {
        const index = Number((entry.target as HTMLElement).dataset.episodeIndex);
        if (entry.isIntersecting) visible.add(index);
        else visible.delete(index);
      }
      pump();
    }, {root, threshold: 0.35});
    root.querySelectorAll("[data-episode-index]").forEach(button => observer.observe(button));
    function cancel() {
      cancelled = true;
      observer.disconnect();
      abort?.();
    }
    stop.current = cancel;
    return cancel;
  }, [rows, rail, enabled, profileId]);
  return { image: (episode: Item) => images.get(key(episode)) || "", stop: () => stop.current() };
}
