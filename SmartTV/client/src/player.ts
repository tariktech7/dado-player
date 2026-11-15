import { Item, PlayerStatus, Settings, Track } from "./types";
import { screensaver } from "./platform";
import { licenseClient } from "./licensing";
import {languageKey} from './languages';
export function resolvePackagedMedia(
  url: string,
  filesystem: any = (globalThis as any).tizen?.filesystem,
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
  private hiddenPlaying = false;
  private suspended = false;
  settings: Settings;
  constructor(
    settings: Settings,
    private changed: () => void,
    host?: HTMLElement,
  ) {
    this.settings = settings;
    this.host = host || document.createElement("div");
    this.host.id = "video-host";
    this.host.hidden = true;
    if (!host)
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
  position(rect: DOMRect): void {
    if (!this.current) return;
    const canvas = document.getElementById("app")?.getBoundingClientRect(),
      scale = canvas ? canvas.width / 1920 : 1;
    Object.assign(this.host.style, {
      left: `${(rect.left - (canvas?.left || 0)) / scale}px`,
      top: `${(rect.top - (canvas?.top || 0)) / scale}px`,
      width: `${rect.width / scale}px`,
      height: `${rect.height / scale}px`,
    });
    if (
      this.av &&
      ["IDLE", "READY", "PLAYING", "PAUSED"].includes(this.av.getState())
    ) {
      try {
        this.av.setDisplayRect(
          Math.round((rect.left * 1920) / innerWidth),
          Math.round((rect.top * 1080) / innerHeight),
          Math.round((rect.width * 1920) / innerWidth),
          Math.round((rect.height * 1080) / innerHeight),
        );
      } catch {
        /* retry at ready */
      }
    }
  }
  async open(item: Item, resume = 0): Promise<void> {
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
    try {
      const native = (window as any).webapis?.avplay;
      if (this.settings.player === "avplay" && native) {
        this.av = native;
        const object = document.createElement("object");
        object.type = "application/avplayer";
        this.replaceSurface(object);
        const source = await resolvePackagedMedia(item.url);
        if (gen !== this.generation) return;
        native.open(source);
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
              this.fail(
                `Samsung AVPlay: ${safeCode}. ${item.id === "dado-diagnostic" ? "Lokales Testvideo konnte nicht geöffnet werden." : "Format HLS/TS und Anbieter prüfen."}`,
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
        this.position(this.host.getBoundingClientRect());
        this.setAspect(this.settings.aspect);
        native.play();
        this.status.state = "playing";
        this.status.buffer = 100;
        if (resume > 0 && item.kind !== "live")
          native.seekTo(
            Math.floor(resume * 1000),
            () => {},
            () => {},
          );
        this.refreshTracks();
        this.applyLanguages();
      } else {
        if (this.settings.player === "avplay" && (window as any).tizen)
          throw new Error(
            "Samsung AVPlay ist auf diesem Gerät nicht verfügbar. In Einstellungen HTML-Video versuchen.",
          );
        const video = document.createElement("video");
        video.playsInline = true;
        video.preload = "auto";
        this.video = video;
        this.replaceSurface(video);
        this.setAspect(this.settings.aspect);
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
      this.emit();
    } catch (error) {
      if (gen === this.generation)
        this.fail(
          (error as Error).message.includes("http")
            ? "Wiedergabe fehlgeschlagen."
            : (error as Error).message,
        );
    }
  }
  private fail(message: string): void {
    this.status.state = "error";
    this.status.error = message;
    this.subtitle = "";
    this.host.hidden = true;
    screensaver(true);
    this.emit();
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
    if (
      !this.current ||
      this.current.kind === "live" ||
      !["playing", "paused"].includes(this.status.state)
    )
      return;
    const target = Math.max(
      0,
      Math.min(this.status.time + delta, Math.max(0, this.status.duration - 1)),
    );
    try {
      if (this.av)
        this.av.seekTo(
          Math.floor(target * 1000),
          () => this.tick(),
          () => this.fail("Springen ist bei diesem Stream nicht möglich."),
        );
      else if (this.video) this.video.currentTime = target;
    } catch {
      throw new Error("Springen ist bei diesem Stream nicht möglich.");
    }
  }
  setAspect(aspect: Settings["aspect"]): void {
    this.settings.aspect = aspect;
    try {
      if (this.av)
        this.av.setDisplayMethod(
          {
            fit: "PLAYER_DISPLAY_MODE_LETTER_BOX",
            fill: "PLAYER_DISPLAY_MODE_AUTO_ASPECT_RATIO",
            stretch: "PLAYER_DISPLAY_MODE_FULL_SCREEN",
          }[aspect],
        );
    } catch {
      /* set again after prepare */
    }
    if (this.video)
      this.video.style.objectFit =
        aspect === "stretch" ? "fill" : aspect === "fill" ? "cover" : "contain";
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
    if (this.subtitleTimer) clearTimeout(this.subtitleTimer);
    this.subtitleTimer = null;
    this.generation++;
    this.pending?.();
    this.pending = null;
    try {
      if (this.av) {
        if (["READY", "PLAYING", "PAUSED"].includes(this.av.getState()))
          this.av.stop();
        this.av.close();
      }
    } catch {
      /* close best effort after decoder failure */
    }
    if (this.video) {
      this.video.pause();
      this.video.removeAttribute("src");
      this.video.load();
    }
    this.av = null;
    this.video = null;
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
