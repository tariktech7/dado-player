//
//  HostBridge.swift
//  Dado Player — the Swift half of the C ABI declared in DadoBridge.h.
//
//  Direction convention (see the header): dado_app_* is implemented in Rust and
//  called from Swift; dado_host_* is implemented here and called from Rust. This
//  file contains every dado_host_* function, exported with @_cdecl because Swift
//  cannot expose methods as C symbols — only free functions.
//
//  Threading: AVPlayer is main-thread-affine for anything UI-visible. Rust calls
//  these from the UI thread, which *is* the main thread, but every mutating call
//  goes through VideoHost.performOnMain, which runs inline on main and dispatches
//  asynchronously otherwise. Getters read AVFoundation's cached state and are
//  safe from any thread.
//
//  Nil safety: Rust may call before the video view exists (attach is async, and
//  the first frames of the app race with window creation). Every entry point
//  tolerates a nil view and returns a neutral value (0 / false / no-op).
//
//  The C declarations live in DadoBridge.h and are NOT re-imported here on
//  purpose: these definitions are the other half of that header, and importing
//  it would make every name a shadowed redeclaration.
//

import UIKit
import AVKit
import DadoBridge

/// Owns the single VideoView instance for the process.
final class VideoHost {

    static let shared = VideoHost()

    private(set) var view: VideoView?
    private weak var uiWindow: UIWindow?
    private var videoWindow: UIWindow?

    /// Rust calls `dado_host_attach` on every frame (`acquire_surface`), so a
    /// naive retry would start a new 50-step chain each frame while no window
    /// exists yet. This collapses them into one in-flight attempt.
    private var playbackController: PlaybackController?
    var nativePlayer = false
    private var attachPending = false
    private var pending: [(VideoView) -> Void] = []

    private init() {}

    /// Creates the playback owner. AVKit presents the full-screen player. Idempotent
    /// and safe to call every frame.
    func attach(attempt: Int = 0) {
        performOnMain { [weak self] in
            guard let self = self else { return }
            if self.view != nil { return }
            if attempt == 0 {
                // A fresh chain is only allowed when nothing is in flight.
                if self.attachPending { return }
                self.attachPending = true
            }

            guard let window = VideoHost.keyWindow() else {
                // Normal at launch: Rust can reach the host before UIKit has a
                // window. Retry briefly instead of dropping the only attach call.
                if attempt < 50 {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                        self.attach(attempt: attempt + 1)
                    }
                } else {
                    self.attachPending = false
                }
                return
            }

            let controller = RootViewController.install(in: window)
            let video = VideoView(frame: controller.view.bounds)
            video.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            // Hidden until Rust positions it. The view is created at full-screen
            // size, so showing it immediately would flash a black rectangle over
            // the login screen.
            video.isHidden = true
            // This view owns AVPlayer. AVKit presents its interactive playback surface.
            video.isUserInteractionEnabled = false
            self.uiWindow = window
            // Give the native decoder its own non-key window. EAGL/VLC cannot
            // insert a view or rendering layer above the independent Metal UI.
            let background: UIWindow
            if let scene = window.windowScene { background = UIWindow(windowScene: scene) }
            else { background = UIWindow(frame: window.frame) }
            background.frame = window.frame
            background.windowLevel = UIWindow.Level(rawValue: window.windowLevel.rawValue - 1)
            background.isUserInteractionEnabled = false
            let container = UIViewController()
            container.view.frame = background.bounds
            container.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            container.view.backgroundColor = .black
            background.rootViewController = container
            container.view.addSubview(video)
            self.videoWindow = background
            window.backgroundColor = .clear
            window.isOpaque = false
            if let root = window.rootViewController?.view {
                root.isOpaque = false
                root.backgroundColor = .clear
                root.layer.isOpaque = false
                root.layer.zPosition = 0
                root.layer.sublayers?.forEach { $0.isOpaque = false }
            }
            self.view = video
            self.keepVideoBehindControls()
            let pending = self.pending
            self.pending.removeAll()
            pending.forEach { $0(video) }
            self.attachPending = false
        }
    }

    /// Restore the cross-window invariant without touching UIKit modal layers.
    func keepVideoBehindControls() {
        guard let ui = uiWindow, let background = videoWindow else { return }
        let level = ui.windowLevel.rawValue - 1
        if background.windowLevel.rawValue != level { background.windowLevel = UIWindow.Level(rawValue: level) }
        if background.frame != ui.frame { background.frame = ui.frame }
        // Never call makeKeyAndVisible: input, IME and winit remain on uiWindow.
    }

    /// Runs `body` with the view on the main thread; no-op when the view does
    /// not exist yet; commands are queued until attachment.
    func withView(_ body: @escaping (VideoView) -> Void) {
        performOnMain { [weak self] in
            guard let self = self else { return }
            if let view = self.view { body(view) } else { self.pending.append(body); self.attach() }
        }
    }

    /// Places the video view. Coordinates arrive in points relative to the root
    /// view, which is exactly this view's superview, so no conversion is needed.
    ///
    /// Rust already deduplicates frames, but UIKit is cheap to call and a
    /// redundant assignment would still dirty the layer tree, so skip identical
    /// frames here too.
    func setFrame(x: Double, y: Double, w: Double, h: Double) {
        withView { view in
            self.keepVideoBehindControls()
            // A degenerate rect would make AVPlayerLayer drop its content and
            // can produce a 1 px flicker on rotation. Ignore it.
            guard w > 0.5, h > 0.5 else { return }
            let target = CGRect(x: x, y: y, width: w, height: h)
            if view.frame.equalTo(target) { return }
            view.frame = target
        }
    }

    func setVisible(_ visible: Bool) {
        withView { [weak self] view in
            guard let self=self else {return}
            self.keepVideoBehindControls()
            if self.playbackController != nil {view.isHidden=true;self.videoWindow?.isHidden=true;return}
            if view.isHidden != !visible { view.isHidden = !visible }
            self.videoWindow?.isHidden = !visible
        }
    }

    func presentPlayer() {
        guard playbackController == nil, let window=Self.keyWindow(), let root=window.rootViewController, let video=view else { return }
        let controller=PlaybackController()
        controller.player=video.player
        controller.modalPresentationStyle = .fullScreen
        controller.onClose = { [weak self] in
            self?.playbackController=nil
            dado_app_player_closed()
        }
        playbackController=controller
        video.isHidden=true
        videoWindow?.isHidden=true
        root.present(controller,animated:true)
    }

    func showError(_ code: Int) {
        guard let controller=playbackController, controller.presentedViewController == nil else {return}
        let hint = [-1200,-1202,-1203].contains(code) ? "Der Anbieter meldet ein ungültiges HTTPS-Zertifikat." :
            "Der Stream konnte nicht geladen werden. Prüfe HLS-Format und Anbieteradresse."
        let alert=UIAlertController(title:"Wiedergabe fehlgeschlagen",message:"\(hint) Fehler \(code).",preferredStyle:.alert)
        alert.addAction(UIAlertAction(title:"Zurück",style:.default){[weak self] _ in
            self?.view?.stop()
            self?.dismissPlayer()
            dado_app_player_closed()
        })
        controller.present(alert,animated:true)
    }

    func dismissPlayer() {
        guard let controller=playbackController else {return}
        controller.onClose=nil
        playbackController=nil
        controller.dismiss(animated:true)
    }

    private func performOnMain(_ work: @escaping () -> Void) {
        if Thread.isMainThread {
            work()
        } else {
            DispatchQueue.main.async(execute: work)
        }
    }

    /// The window winit created. Scene-based API only; UIApplication.windows is
    /// deprecated as of the iOS 15 deployment target.
    private static func keyWindow() -> UIWindow? {
        for scene in UIApplication.shared.connectedScenes {
            guard let windowScene = scene as? UIWindowScene else { continue }
            if let key = windowScene.windows.first(where: { $0.isKeyWindow }) {
                return key
            }
            if let first = windowScene.windows.first {
                return first
            }
        }
        return UIApplication.shared.windows.first(where: { $0.isKeyWindow })
    }
}

@_cdecl("dado_host_motion_active")
public func dado_host_motion_active(_ active: Int32) {
    guard Thread.isMainThread else { DispatchQueue.main.async { dado_host_motion_active(active) }; return }
    guard let window = UIApplication.shared.connectedScenes.compactMap({$0 as? UIWindowScene}).flatMap({$0.windows}).first(where: {$0.isKeyWindow}) else { return }
    RootViewController.install(in: window).setMotionActive(active != 0)
}

// MARK: - Lifecycle

/// Creates the AVPlayerLayer-backed view and adds it to the root view, behind
/// any UI overlay. Idempotent.
@_cdecl("dado_host_attach")
public func dado_host_attach() {
    VideoHost.shared.attach()
}

@_cdecl("dado_host_native_player")
public func dado_host_native_player(_ enabled: Int32) {
    VideoHost.shared.nativePlayer = enabled != 0
}

@_cdecl("dado_host_tracks")
public func dado_host_tracks(_ buffer: UnsafeMutablePointer<CChar>?, _ capacity: Int32) -> Int32 {
    let tracks = VideoHost.shared.view?.tracks ?? []
    guard let data = try? JSONSerialization.data(withJSONObject: tracks) else { return 0 }
    if let buffer = buffer, capacity >= data.count + 1 {
        data.withUnsafeBytes { raw in
            if let base = raw.baseAddress { memcpy(buffer, base, data.count) }
        }
        buffer[data.count] = 0
    }
    return Int32(data.count + 1)
}

@_cdecl("dado_host_current_track")
public func dado_host_current_track(_ kind: Int32) -> Int64 {
    return VideoHost.shared.view?.currentTrack(kind) ?? -1
}

@_cdecl("dado_host_select_track")
public func dado_host_select_track(_ kind: Int32, _ id: Int64) {
    VideoHost.shared.withView { $0.selectTrack(kind, id) }
}

/// Places the native video view. Points, relative to the root view.
@_cdecl("dado_host_video_frame")
public func dado_host_video_frame(_ x: Double, _ y: Double, _ w: Double, _ h: Double) {
    VideoHost.shared.setFrame(x: x, y: y, w: w, h: h)
}

/// Shows or hides the native video view.
@_cdecl("dado_host_video_visible")
public func dado_host_video_visible(_ visible: Int32) {
    VideoHost.shared.setVisible(visible != 0)
}

/// Replaces the current item. The URL may carry provider credentials, so it is
/// never logged.
@_cdecl("dado_host_aspect")
public func dado_host_aspect(_ mode: Int32) {
    VideoHost.shared.withView { $0.setAspect(mode) }
}

@_cdecl("dado_host_options")
public func dado_host_options(_ json: UnsafePointer<CChar>?) {
    guard let json, let data = String(cString: json).data(using: .utf8),
          let options = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return }
    VideoHost.shared.withView { $0.configure(options: options) }
}

@_cdecl("dado_host_load")
public func dado_host_load(_ url: UnsafePointer<CChar>?) {
    guard let url = url else { return }
    let string = String(cString: url)
    let source = string == "dado:demo" ? Bundle.main.url(forResource: "Playback", withExtension: "mp4") : URL(string: string)
    guard let parsed = source else {
        NSLog("%@", "Dado: dado_host_load got an unparseable URL")
        return
    }
    VideoHost.shared.withView { view in
        view.load(url: parsed)
        if VideoHost.shared.nativePlayer { VideoHost.shared.presentPlayer() }
    }
}

/// Stops playback and releases the current item.
@_cdecl("dado_host_stop")
public func dado_host_stop() {
    VideoHost.shared.withView { view in
        view.stop()
        VideoHost.shared.setVisible(false)
        VideoHost.shared.dismissPlayer()
    }
}

/// 1 = pause, 0 = play.
@_cdecl("dado_host_pause")
public func dado_host_pause(_ paused: Int32) {
    VideoHost.shared.withView { view in
        view.setPaused(paused != 0)
    }
}

/// Absolute seek in seconds. Negative values are clamped to 0 inside VideoView.
@_cdecl("dado_host_seek")
public func dado_host_seek(_ secs: Double) {
    VideoHost.shared.withView { view in
        view.seek(to: secs)
    }
}

/// 0..100, matching mpv's volume scale.
@_cdecl("dado_host_volume")
public func dado_host_volume(_ vol: Int32) {
    VideoHost.shared.withView { view in
        view.setVolume(Int(vol))
    }
}

@_cdecl("dado_host_brightness")
public func dado_host_brightness(_ requested: Float) -> Float {
    let update = {
        let screen = VideoHost.shared.view?.window?.screen ?? UIScreen.main
        if requested >= 0 { screen.brightness = CGFloat(min(1, requested)) }
        return Float(screen.brightness)
    }
    return Thread.isMainThread ? update() : DispatchQueue.main.sync(execute: update)
}

@_cdecl("dado_host_speed")
public func dado_host_speed(_ speed: Float) {
    VideoHost.shared.withView { $0.setSpeed(speed) }
}

// MARK: - Queries

@_cdecl("dado_host_position")
public func dado_host_position() -> Double {
    return VideoHost.shared.view?.position ?? 0
}

@_cdecl("dado_host_duration")
public func dado_host_duration() -> Double {
    return VideoHost.shared.view?.duration ?? 0
}

@_cdecl("dado_host_cache_time")
public func dado_host_cache_time() -> Double {
    return VideoHost.shared.view?.cacheTime ?? 0
}

@_cdecl("dado_host_paused")
public func dado_host_paused() -> Int32 {
    guard let view = VideoHost.shared.view else { return 0 }
    return view.paused ? 1 : 0
}

@_cdecl("dado_host_eof")
public func dado_host_eof() -> Int32 {
    guard let view = VideoHost.shared.view else { return 0 }
    return view.eof ? 1 : 0
}

@_cdecl("dado_host_width")
public func dado_host_width() -> Int64 {
    return int64Size(VideoHost.shared.view?.videoSize.width)
}

@_cdecl("dado_host_height")
public func dado_host_height() -> Int64 {
    return int64Size(VideoHost.shared.view?.videoSize.height)
}

@_cdecl("dado_host_fps")
public func dado_host_fps() -> Double {
    return VideoHost.shared.view?.fps ?? 0
}

/// CGFloat -> Int64 without trapping on NaN/infinity/negative values.
private func int64Size(_ value: CGFloat?) -> Int64 {
    guard let value = value, value.isFinite, value > 0 else { return 0 }
    return Int64(value)
}

@_cdecl("dado_host_error_code")
public func dadoHostErrorCode() -> Int32 { Int32(clamping: VideoHost.shared.view?.errorCode ?? 0) }

private final class PlaybackController: AVPlayerViewController {
    var onClose: (() -> Void)?
    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)
        if isBeingDismissed || presentingViewController == nil { onClose?(); onClose=nil }
    }
}
