import { Item, PlayerStatus, Settings, Track } from "./types";
import { screensaver } from "./platform";
import { licenseClient } from "./licensing";
import {languageKey} from './languages';
export function resolvePackagedMedia(
  url: string,
  filesystem: any = (typeof window !== "undefined" ? (window as any).tizen : undefined)?.filesystem,
): Promise<string> {
  if (
    !url.startsWith("file:") ||
    !url.split("?")[0].endsWith("/test-video.mp4")
  )
    return Promise.resolve(url);
  if (!filesystem?.resolve)
    return Promise.reject(
      new Error("Der lokale Test benötigt Samsungs Dateisystem-API."),
    );
  return new Promise((resolve, reject) =>
    filesystem.resolve(
      "wgt-package/test-video.mp4",
      (file: any) => resolve(file.toURI()),
      () =>
        reject(
          new Error(
            "Das mitgelieferte Testvideo wurde im App-Paket nicht gefunden.",
          ),
        ),
      "r",
    ),
  );
}
export class Player {
  status: PlayerStatus = {
    state: "idle",
    time: 0,
    duration: 0,
    buffer: 0,
    error: "",
    tracks: [],
  };
  subtitle = "";
  current: Item | null = null;
  private av: any = null;
  private video: HTMLVideoElement | null = null;
  private generation = 0;
  private pending: (() => void) | null = null;
  private host: HTMLElement;
  private subtitleTimer: ReturnType<typeof setTimeout> | null = null;
  private timer: ReturnType<typeof setInterval>;
  private pendingSeekDelta = 0;
  private lastNativeRect = "";
  private lastNativeAspect = "";
  private lastCssRect = "";
  private seekTarget: number | null = null;
  private seekBusy = false;
  private seekTimer: ReturnType<typeof setTimeout> | null = null;
  private hiddenPlaying = false;
  private suspended = false;
  settings: Settings;
  constructor(
    settings: Settings,
    private changed: () => void,
    private surface: () => HTMLElement | null,
  ) {
    this.settings = settings;
    this.host = document.createElement("div");
    this.host.id = "video-host";
    this.host.hidden = true;
    (document.getElementById("app") || document.body).append(this.host);
    this.timer = setInterval(() => {
      if (this.status.state === "playing" || this.status.state === "paused")
        this.tick();
    }, 1000);
  }
  private emit(): void {
    this.changed();
  }
  private replaceSurface(node?: HTMLElement): void {
    while (this.host.firstChild) this.host.removeChild(this.host.firstChild);
    if (node) this.host.appendChild(node);
  }
  private tick(): void {
    try {
      this.status.time = this.av
        ? this.av.getCurrentTime() / 1000
        : this.video?.currentTime || 0;
      this.status.duration = this.av
        ? this.av.getDuration() / 1000
        : this.video?.duration || 0;
      if (!Number.isFinite(this.status.duration)) this.status.duration = 0;
    } catch {
      /* transient AVPlay state */
    }
    this.emit();
  }
  position(): void {
    const surface = this.surface();
    if (!this.current || !surface) return;
    const rect = surface.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return;

    const canvas = document.getElementById("app")?.getBoundingClientRect();
    const scale = canvas ? canvas.width / 1920 : 1;
    const cssRect = [
      (rect.left - (canvas?.left || 0)) / scale,
      (rect.top - (canvas?.top || 0)) / scale,
      rect.width / scale,
      rect.height / scale,
    ];
    const cssKey = cssRect.join(",");
    if (cssKey !== this.lastCssRect) {
      const [left, top, width, height] = cssRect.map((value) => `${value}px`);
      Object.assign(this.host.style, { left, top, width, height });
      const object = this.host.querySelector("object");
      if (object) Object.assign(object.style, { width, height });
      this.lastCssRect = cssKey;
    }

    // The preview always fits the entire frame. Fullscreen keeps the user's setting.
    const aspect = surface.classList.contains("preview-surface") ? "fit" : this.settings.aspect;
    if (this.video) {
      this.video.style.objectFit =
        aspect === "stretch" ? "fill" : aspect === "fill" ? "cover" : "contain";
    }
    if (!this.av || this.seekBusy) return;

    try {
      if (!["IDLE", "READY", "PLAYING", "PAUSED"].includes(this.av.getState())) return;
      if (aspect !== this.lastNativeAspect) {
        try {
          this.av.setDisplayMethod({
            fit: "PLAYER_DISPLAY_MODE_LETTER_BOX",
            fill: "PLAYER_DISPLAY_MODE_AUTO_ASPECT_RATIO",
            stretch: "PLAYER_DISPLAY_MODE_FULL_SCREEN",
          }[aspect]);
          this.lastNativeAspect = aspect;
          this.lastNativeRect = "";
        } catch {
          // Still update the video bounds if this firmware rejects a mode change.
        }
      }
      // AVPlay uses a 1920x1080 screen coordinate system. Scale both axes
      // by width, just like the TV canvas, even if the viewport height changes.
      const units = 1920 / document.documentElement.clientWidth;
      const x = Math.round(rect.left * units);
      const y = Math.round(rect.top * units);
      const width = Math.round(rect.right * units) - x;
      const height = Math.round(rect.bottom * units) - y;
      const nativeKey = [x, y, width, height].join(",");
      if (nativeKey !== this.lastNativeRect) {
        this.av.setDisplayRect(x, y, width, height);
        this.lastNativeRect = nativeKey;
      }
    } catch {
      // Some states reject changes temporarily. Retry on ready or next layout.
    }
  }
  async open(item: Item, resume = 0, htmlFallback = false): Promise<void> {
    if (!licenseClient.state.allowed) throw new Error('Testzeit abgelaufen oder noch nicht gestartet. Freischaltung in den Einstellungen abrufen.');
    this.stop();
    const gen = ++this.generation;
    this.current = item;
    this.status = {
      state: "loading",
      time: 0,
      duration: 0,
      buffer: 0,
      error: "",
      tracks: [],
    };
    this.host.hidden = false;
    this.emit();
    const automatic = !this.settings.expertMode || this.settings.player === "auto";
    const useNative = !htmlFallback && (automatic || this.settings.player === "avplay");
    const failed = (message: string) => {
      if (gen !== this.generation) return;
      if (automatic && useNative && this.status.state === "loading" && (window as any).webapis?.avplay) {
        // One alternate engine, same original URL and resume point. Never rename
        // a provider's file extension or loop through failing engines forever.
        void this.open(item, Math.max(resume, this.status.time), true);
      } else this.fail(message);
    };
    try {
      const native = (window as any).webapis?.avplay;
      if (useNative && native) {
        this.av = native;
        const object = document.createElement("object");
        object.type = "application/avplayer";
        this.replaceSurface(object);
        const source = await resolvePackagedMedia(item.url);
        if (gen !== this.generation) return;
        native.open(source);
        this.position();
        native.setListener({
          onbufferingstart: () => {
            if (gen === this.generation) {
              this.status.buffer = 0;
              this.emit();
            }
          },
          onbufferingprogress: (n: number) => {
            if (gen === this.generation) {
              this.status.buffer = n;
              this.emit();
            }
          },
          onbufferingcomplete: () => {
            if (gen === this.generation) {
              this.status.buffer = 100;
              this.emit();
            }
          },
          oncurrentplaytime: () => {
            if (gen === this.generation) this.tick();
          },
          onstreamcompleted: () => {
            if (gen === this.generation) {
              this.status.state = "ended";
              screensaver(true);
              this.emit();
            }
          },
          onerror: (code: unknown) => {
            const safeCode = /^(PLAYER_ERROR_|AVPLAY_)[A-Z0-9_]+$/.test(
              String(code),
            )
              ? String(code)
              : "UNBEKANNTER_FEHLER";
            if (gen === this.generation)
              failed(
                `Samsung AVPlay: ${safeCode}. ${item.id === "dado-diagnostic" ? "Lokales Testvideo konnte nicht geöffnet werden." : "Dateiformat, Codec und Anbieter prüfen."}`,
              );
          },
          onsubtitlechange: (duration: number, text: string) => {
            if (gen === this.generation) {
              if (this.subtitleTimer) clearTimeout(this.subtitleTimer);
              this.subtitle = this.settings.subtitles
                ? text.replace(/<[^>]*>/g, "")
                : "";
              this.emit();
              this.subtitleTimer = setTimeout(
                () => {
                  this.subtitle = "";
                  this.emit();
                },
                Math.max(0, duration),
              );
            }
          },
        });
        try {
          native.setBufferingParam(
            "PLAYER_BUFFER_FOR_PLAY",
            "PLAYER_BUFFER_SIZE_IN_SECOND",
            this.settings.bufferSeconds,
          );
        } catch {
          /* some firmwares use defaults */
        }
        if (this.settings.userAgent)
          try {
            native.setStreamingProperty("USER_AGENT", this.settings.userAgent);
          } catch {
            /* optional provider requirement */
          }
        await new Promise<void>((resolve, reject) => {
          let done = false;
          const finish = (error?: Error) => {
            if (done) return;
            done = true;
            clearTimeout(timeout);
            this.pending = null;
            error ? reject(error) : resolve();
          };
          const timeout = setTimeout(
            () => finish(new Error("Der Stream antwortet nicht rechtzeitig.")),
            this.settings.timeoutSeconds * 1000,
          );
          this.pending = () => finish(new Error("Abgebrochen"));
          native.prepareAsync(
            () => finish(),
            () =>
              finish(new Error("Samsung konnte den Stream nicht vorbereiten.")),
          );
        });
        if (gen !== this.generation) return;
        this.lastNativeRect = "";
        this.lastNativeAspect = "";
        this.position();
        native.play();
        this.status.state = "playing";
        this.status.buffer = 100;
        if (resume > 0 && item.kind !== "live") {
          this.status.time=0; this.seek(resume);
        }
        this.refreshTracks();
        this.applyLanguages();
      } else {
        if (!automatic && useNative && (window as any).tizen)
          throw new Error(
            "Samsung AVPlay ist auf diesem Gerät nicht verfügbar. In Einstellungen HTML-Video versuchen.",
          );
        const video = document.createElement("video");
        video.playsInline = true;
        video.preload = "auto";
        this.video = video;
        this.replaceSurface(video);
        this.position();
        video.addEventListener("error", () => {
          if (gen === this.generation)
            this.fail(
              "Video kann nicht abgespielt werden. Codec, Format oder Netzwerkzugriff prüfen.",
            );
        });
        video.addEventListener("ended", () => {
          if (gen === this.generation) {
            this.status.state = "ended";
            screensaver(true);
            this.emit();
          }
        });
        video.addEventListener("loadedmetadata", () => {
          if (gen === this.generation && resume > 0 && item.kind !== "live")
            video.currentTime = resume;
        });
        video.src = item.url;
        await video.play();
        if (gen !== this.generation) return;
        this.status.state = "playing";
        this.status.buffer = 100;
        this.refreshTracks();
      }
      screensaver(false);
      this.tick();
      if (this.pendingSeekDelta) { const delta=this.pendingSeekDelta; this.pendingSeekDelta=0; this.seek(delta); }
      this.emit();
    } catch (error) {
      if (gen === this.generation)
        failed(
          (error as Error).message.includes("http")
            ? "Wiedergabe fehlgeschlagen."
            : (error as Error).message,
        );
    }
  }
  private releaseDecoder(): void {
    // Each cleanup is independent: stop can throw in a broken AVPlay state,
    // but close must still run so there is never an orphan audio decoder.
    const av = this.av, video = this.video;
    this.av = null; this.video = null;
    try { av?.setListener({}); } catch {}
    try { av?.pause(); } catch {}
    try { av?.stop(); } catch {}
    try { av?.close(); } catch {}
    if(video) {
      try { video.pause(); } catch {}
      try { video.removeAttribute("src"); video.load(); } catch {}
    }
    this.replaceSurface();
    this.host.hidden = true;
  }
  private fail(message: string): void {
    this.generation++;
    this.pending?.(); this.pending=null;
    this.clearSeek();
    this.releaseDecoder();
    this.status.state = "error";
    this.status.error = message;
    this.status.notice = "";
    this.status.tracks = [];
    if(this.subtitleTimer)clearTimeout(this.subtitleTimer);
    this.subtitleTimer=null;
    this.subtitle = "";
    screensaver(true);
    this.emit();
  }
  private clearSeek(): void {
    if(this.seekTimer) clearTimeout(this.seekTimer);
    this.seekTimer=null; this.seekBusy=false; this.seekTarget=null;
    this.pendingSeekDelta=0;
  }
  refreshTracks(): void {
    try {
      this.status.tracks = this.av
        ? this.av
            .getTotalTrackInfo()
            .filter((t: any) => t.type === "AUDIO" || t.type === "TEXT")
            .map((t: any) => {
              let extra: any = {};
              try {
                extra = JSON.parse(t.extra_info || "{}");
              } catch {
                /* malformed provider metadata */
              }
              return {
                type: t.type,
                index: t.index,
                language: String(extra.language || extra.track_lang || ''),
                label: `${extra.language || extra.track_lang || "Spur " + (t.index + 1)}${extra.codec ? ` · ${extra.codec}` : ""}`,
              };
            })
        : Array.from(this.video?.textTracks || []).map((t, i) => ({
            type: "TEXT" as const,
            index: i,
            language: t.language,
            label: t.label || t.language || `Untertitel ${i + 1}`,
          }));
    } catch {
      this.status.tracks = [];
    }
    this.emit();
  }
  private applyLanguages(): void {
    const wantSubtitles = this.settings.subtitles;
    const audio = this.status.tracks.find(
      (t) =>
        t.type === "AUDIO" &&
        this.settings.audioLanguage &&
        languageKey(t.language || t.label.split(' ')[0]) === languageKey(this.settings.audioLanguage),
    );
    if (audio) this.selectTrack(audio);
    const sub = this.status.tracks.find(
      (t) =>
        t.type === "TEXT" &&
        (!this.settings.subtitleLanguage ||
          languageKey(t.language || t.label.split(' ')[0]) === languageKey(this.settings.subtitleLanguage)),
    ) || this.status.tracks.find(t=>t.type==='TEXT');
    this.subtitleOff();
    if (wantSubtitles && sub) this.selectTrack(sub);
  }
  selectTrack(track: Track): void {
    try {
      if (this.av) {
        this.av.setSelectTrack(track.type, track.index);
        if (track.type === "TEXT") this.av.setSilentSubtitle(true);
      } else if (track.type === "TEXT" && this.video) {
        Array.from(this.video.textTracks).forEach(
          (t, i) => (t.mode = i === track.index ? "showing" : "disabled"),
        );
      }
    } catch {
      throw new Error(
        "Diese Spur lässt sich bei diesem Stream nicht umschalten.",
      );
    }
    if (track.type === "TEXT") {
      this.settings.subtitles = true;
      this.subtitle = "";
    }
  }
  subtitleOff(): void {
    try {
      if (this.av) this.av.setSilentSubtitle(true);
      if (this.video)
        Array.from(this.video.textTracks).forEach((t) => (t.mode = "disabled"));
    } catch {
      /* no text tracks */
    }
    this.settings.subtitles = false;
    this.subtitle = "";
    this.emit();
  }
  toggle(): void {
    if (!["playing", "paused"].includes(this.status.state)) return;
    try {
      if (this.status.state === "playing") {
        this.av ? this.av.pause() : this.video?.pause();
        this.status.state = "paused";
        screensaver(true);
      } else {
        if (this.av) this.av.play();
        else
          void this.video
            ?.play()
            .catch(() =>
              this.fail("Wiedergabe konnte nicht fortgesetzt werden."),
            );
        this.status.state = "playing";
        screensaver(false);
      }
      this.emit();
    } catch {
      this.fail("Player reagiert nicht. Sender erneut öffnen.");
    }
  }
  seek(delta: number): void {
    if(!Number.isFinite(delta) || !this.current)return;
    if(this.status.state==='loading'){this.pendingSeekDelta+=delta;return;}
    if(!['playing','paused'].includes(this.status.state))return;
    this.seekTo((this.seekTarget ?? this.status.time) + delta);
  }
  seekTo(seconds: number): void {
    if (!Number.isFinite(seconds) || !this.current) return;
    if (!["playing", "paused"].includes(this.status.state)) return;
    this.seekTarget = this.current.kind === "live" ? seconds
      : Math.max(0, Math.min(seconds, this.status.duration > 0 ? Math.max(0, this.status.duration - 1) : Infinity));
    if(!this.seekBusy)this.flushSeek();
  }
  private liveSeekRange(): [number, number] | null {
    if (this.av) {
      try {
        const value = this.av.getStreamingProperty("GET_LIVE_DURATION") as string;
        const [start, end] = value.split("|").map(ms => Number(ms) / 1000);
        return Number.isFinite(start) && Number.isFinite(end) && end > start ? [start, end] : null;
      } catch {
        return null;
      }
    }
    const ranges = this.video?.seekable;
    if (!ranges?.length) return null;
    for (let index = 0; index < ranges.length; index++) {
      const start = ranges.start(index), end = ranges.end(index);
      if (this.status.time >= start && this.status.time <= end) return [start, end];
    }
    const last = ranges.length - 1;
    return [ranges.start(last), ranges.end(last)];
  }
  private flushSeek(): void {
    if(this.seekTarget===null || this.seekBusy)return;
    const requested = this.seekTarget;
    const live = this.current?.kind === "live";
    const range = live ? this.liveSeekRange() : null;
    if (live && !range) {
      this.seekTarget = null;
      this.status.notice = "Dieser Live-Stream unterstützt kein Spulen.";
      this.emit();
      return;
    }
    const target = range ? Math.max(range[0], Math.min(requested, range[1])) : requested;
    const gen = this.generation, av = this.av;
    this.seekBusy=true;
    let finished = false;
    const finish=(ok:boolean, timedOut = false)=>{
      if(finished || gen!==this.generation)return;
      finished = true;
      if(this.seekTimer)clearTimeout(this.seekTimer);
      this.seekTimer=null;this.seekBusy=false;
      if(!ok){
        const queued = this.seekTarget !== requested;
        if (queued && !timedOut) { this.flushSeek(); return; }
        this.seekTarget=null;
        this.status.notice='Diese Position ist nicht erreichbar. Die Wiedergabe läuft weiter.';
        this.emit();return;
      }
      this.status.notice = '';
      this.tick();
      this.position();
      // New presses update a single absolute destination while a seek is busy.
      if(this.seekTarget!==requested){this.flushSeek();return;}
      this.seekTarget=null;
    };
    this.seekTimer=setTimeout(()=>finish(false, true),this.settings.timeoutSeconds*1000);
    try {
      if(av) av.seekTo(Math.floor(target*1000),()=>finish(true),()=>finish(false));
      else if(this.video){this.video.currentTime=target;finish(true);}
      else finish(false);
    }catch{finish(false);}
  }
  setAspect(aspect: Settings["aspect"]): void {
    this.settings.aspect = aspect;
    this.position();
  }
  visibility(hidden: boolean): void {
    if (hidden) {
      this.hiddenPlaying = this.status.state === "playing";
      if (this.av && ["playing", "paused"].includes(this.status.state)) {
        try {
          this.av.suspend();
          this.suspended = true;
        } catch {
          this.av.pause();
        }
      } else this.video?.pause();
      screensaver(true);
    } else if (this.current) {
      if (this.av && this.suspended) {
        try {
          this.av.restoreAsync(
            "",
            0,
            false,
            () => {
              this.suspended = false;
              this.lastNativeRect = "";
              this.lastNativeAspect = "";
              this.position();
              if (!this.hiddenPlaying) this.av.pause();
              this.status.state = this.hiddenPlaying ? "playing" : "paused";
              screensaver(!this.hiddenPlaying);
              this.emit();
            },
            () =>
              this.fail("Wiederaufnahme fehlgeschlagen. Sender erneut öffnen."),
          );
        } catch {
          this.fail("Wiederaufnahme fehlgeschlagen.");
        }
      } else if (this.hiddenPlaying && this.video) {
        void this.video
          .play()
          .then(() => screensaver(false))
          .catch(() => this.fail("Wiedergabe erneut starten."));
      }
    }
  }
  stop(): void {
    this.lastNativeRect = "";
    this.lastNativeAspect = "";
    this.lastCssRect = "";
    this.clearSeek();
    if (this.subtitleTimer) clearTimeout(this.subtitleTimer);
    this.subtitleTimer = null;
    this.generation++;
    this.pending?.(); this.pending=null;
    this.releaseDecoder();
    this.current = null;
    this.subtitle = "";
    this.replaceSurface();
    this.host.hidden = true;
    this.status = {
      state: "idle",
      time: 0,
      duration: 0,
      buffer: 0,
      error: "",
      tracks: [],
    };
    screensaver(true);
  }
  dispose(): void {
    this.stop();
    clearInterval(this.timer);
    this.host.remove();
  }
}
