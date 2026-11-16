//
//  DadoBridge.h
//  Dado Player — C ABI between the Swift/UIKit host and the Rust core.
//
//  This header is the SINGLE SOURCE OF TRUTH for the boundary. It is included
//  by the Swift host (via the module map) and mirrored by
//  `src/platform/ios/bridge.rs` (Rust `extern "C"` declarations). If you change
//  a signature here, change it there in the same commit — a mismatch is not a
//  compile error on either side, it is a crash at runtime.
//
//  Naming convention encodes the direction of the call:
//
//    dado_app_*   implemented in RUST,  called from SWIFT
//    dado_host_*  implemented in SWIFT, called from RUST
//
//  All strings are NUL-terminated UTF-8. The callee must copy anything it
//  needs; no pointer passed across this boundary stays valid after the call
//  returns. Geometry is in UIKit POINTS (not pixels); Rust converts with the
//  reported `scale`.
//

#ifndef DADO_BRIDGE_H
#define DADO_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Rust API — implemented in Rust, called by Swift
// ---------------------------------------------------------------------------

/// Boots the Rust application. Calls into eframe/winit, which takes over the
/// main thread and runs the UIKit application loop. This function does not
/// return under normal operation.
///
/// Must be called exactly once, from the Swift `main.swift` entry point, after
/// `dado_app_set_storage_dir` has been called. Calling it from a SwiftUI
/// `@main` App would mean two owners of `UIApplicationMain`; don't.
void dado_app_start(void);

/// Absolute path of the writable per-app directory (Application Support).
/// Must be called before `dado_app_start`. Rust points `directories` at it so
/// `profiles.json` / `settings.json` land inside the app sandbox.
void dado_app_set_storage_dir(const char *utf8_path);

/// Pushes the current environment. Called by the host on every layout pass and
/// on appearance changes; cheap and idempotent.
///
/// @param top/bottom/left/right  Safe-area insets in points.
/// @param ime                    Keyboard overlap in points (already included
///                               in `bottom`).
/// @param dark                   1 = dark appearance, 0 = light.
/// @param scale                  UIScreen scale factor (points -> pixels).
/// @param width/height           Root view bounds in points. Rust uses these for
///                               aspect-ratio math; the video rect itself is
///                               pushed back with `dado_host_video_frame`.
void dado_app_set_env(double top,
                      double bottom,
                      double left,
                      double right,
                      double ime,
                      int32_t dark,
                      double scale,
                      double width,
                      double height);

// ---------------------------------------------------------------------------
// Swift host API — implemented in Swift, called by Rust
// ---------------------------------------------------------------------------

/// Creates the AVPlayerLayer-backed view and adds it to the root view, behind
/// any UI overlay. Idempotent.
void dado_host_attach(void);
/// UTF-8 playback options JSON, copied synchronously by the host. Never logged.
void dado_host_options(const char *json);
void dado_host_native_player(int32_t enabled);
int32_t dado_host_tracks(char *buffer, int32_t capacity);
int64_t dado_host_current_track(int32_t kind);
void dado_host_select_track(int32_t kind, int64_t id);

/// Places the native video view. Coordinates are in points, relative to the
/// host's root view. Called every frame while the player is visible; the host
/// is expected to ignore no-op changes (comparing against the current frame).
void dado_host_video_frame(double x, double y, double w, double h);

/// Shows (1) or hides (0) the native video view.
void dado_host_video_visible(int32_t visible);

/// Replaces the current item. `url` may be an HLS playlist, a progressive file
/// or a provider stream URL. Credentials embedded in the URL must never be
/// logged by the host.
void dado_host_load(const char *url);

/// Stops playback and releases the current item.
void dado_host_stop(void);

/// 1 = pause, 0 = play.
void dado_host_pause(int32_t paused);

/// Absolute seek in seconds. Negative values are clamped to 0.
void dado_host_seek(double secs);

/// 0..100, matching mpv's volume scale.
void dado_host_volume(int32_t vol);

double  dado_host_position(void);
double  dado_host_duration(void);
/// Seconds of forward buffer, mirroring mpv's `demuxer-cache-time`.
double  dado_host_cache_time(void);
int32_t dado_host_paused(void);
int32_t dado_host_eof(void);

/// Decoded video dimensions, 0 when unknown.
int64_t dado_host_width(void);
int64_t dado_host_height(void);
double  dado_host_fps(void);
void dado_app_player_closed(void);
void dado_app_edit_finished(uint64_t id, int32_t next);
void dado_app_edited(uint64_t id, const char *value);
void dado_host_edit(uint64_t id, const char *title, const char *value, int32_t kind, double x, double y, double w, double h, int32_t mode, int32_t dark);
int32_t dado_host_error_code(void);
void dado_host_mobile_update(int32_t install);
void dado_host_motion_active(int32_t active);

#ifdef __cplusplus
}
#endif

#endif /* DADO_BRIDGE_H */
