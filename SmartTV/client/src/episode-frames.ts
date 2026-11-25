
import { useEffect, useRef, useState } from "preact/hooks";
import { Item } from "./types";

function readFrame(url: string, done: (image: string) => void): () => void {
  const video = document.createElement("video");
  video.crossOrigin = "anonymous";
  video.muted = true;
  video.preload = "auto";
  let finished = false;
  const finish = (image?: string) => {
    if (finished) return;
    finished = true;
    window.clearTimeout(timer);
    video.onloadedmetadata = video.onloadeddata = video.onseeked = video.onerror = null;
    video.removeAttribute("src");
    video.load();
    if (image !== undefined) done(image);
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
      finish(canvas.toDataURL("image/jpeg", 0.75));
    } catch {
      finish("");
    }
  };
  let positioned = false;
  video.onloadedmetadata = () => {
    const position = Number.isFinite(video.duration) ? Math.min(5, video.duration / 2) : 0;
    if (position > 0) {
      try { video.currentTime = position; } catch { finish(""); }
    } else {
      positioned = true;
      capture();
    }
  };
  video.onloadeddata = () => { if (positioned) capture(); };
  video.onseeked = () => { positioned = true; capture(); };
  video.onerror = () => finish("");
  const timer = window.setTimeout(() => finish(""), 5500);
  video.src = url;
  return () => finish();
}

export function useEpisodeFrames(rows: Item[], rail: {current: HTMLDivElement | null}, enabled: boolean) {
  const cache = useRef(new Map<string, string>());
  const stop = useRef<() => void>(() => {});
  const [, repaint] = useState(0);
  useEffect(() => {
    const root = rail.current;
    if (!root || !enabled || typeof IntersectionObserver === "undefined") return;
    const visible = new Set<number>();
    let cancelled = false;
    let active = false;
    let abort: (() => void) | undefined;
    function pump() {
      if (active || cancelled) return;
      const next = [...visible].find(index => {
        const episode = rows[index];
        return episode?.url && !cache.current.has(episode.id);
      });
      if (next === undefined) return;
      const episode = rows[next];
      active = true;
      abort = readFrame(episode.url, (result) => {
        if (cancelled) return;
        if (cache.current.size >= 48) {
          const oldest = cache.current.keys().next().value;
          if (oldest !== undefined) cache.current.delete(oldest);
        }
        cache.current.set(episode.id, result);
        active = false;
        repaint(revision => revision + 1);
        pump();
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
  }, [rows, rail, enabled]);
  return { image: (id: string) => cache.current.get(id) || "", stop: () => stop.current() };
}
