//
//  VideoView.swift
//  Dado Player — AVPlayerLayer-backed video surface.
//
//  This is the iOS equivalent of the Android mpv surface: a plain UIView whose
//  backing layer *is* an AVPlayerLayer, so no extra sublayer bookkeeping is
//  needed when the frame changes.
//
//  Threading: every method here is expected to be called on the main thread
//  (AVFoundation is main-thread-affine for UI-visible playback). HostBridge
//  guarantees that by dispatching; KVO and notification callbacks hop back to
//  main before touching state.
//
//  Logging: the URL carries provider credentials. Nothing in this file ever
//  logs a URL — only "item loaded" and numeric error codes.
//

import UIKit
import AVFoundation

final class VideoView: UIView {

    override class var layerClass: AnyClass { AVPlayerLayer.self }

    private var playerLayer: AVPlayerLayer {
        // Safe: layerClass above guarantees the type.
        return layer as! AVPlayerLayer
    }

    let player = AVPlayer()
    private var itemObservers: [NSKeyValueObservation] = []
    private var endObserver: NSObjectProtocol?

    /// Forward-buffer end, in seconds, refreshed by KVO on `loadedTimeRanges`.
    /// `cacheTime` is this minus the current position, i.e. what mpv calls
    /// `demuxer-cache-time`.
    private var bufferedEnd: Double = 0

    /// Set by AVPlayerItemDidPlayToEndTime, cleared on load/seek/stop.
    private(set) var eof: Bool = false

    /// Decoded video size in points (already scaled by the item's
    /// presentationSize). Zero until the item is ready.
    private(set) var videoSize: CGSize = .zero

    /// Last item failure code, nil when healthy. The ABI has no error channel,
    /// so this stays host-internal; HostBridge could surface it later.
    private(set) var errorCode: Int?

    // MARK: - Init

    override init(frame: CGRect) {
        super.init(frame: frame)
        commonInit()
    }

    required init?(coder: NSCoder) {
        // Built programmatically only; a nib would never provide the right layer.
        fatalError("VideoView is created in code only")
    }

    private func commonInit() {
        backgroundColor = .black
        isOpaque = true
        playerLayer.player = player
        playerLayer.videoGravity = .resizeAspect
        player.actionAtItemEnd = .pause
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .moviePlayback)
            try AVAudioSession.sharedInstance().setActive(true)
        } catch { NSLog("Dado: audio session unavailable") }
    }

    // MARK: - Control

    func load(url: URL) {
        clearItemState()

        let item = AVPlayerItem(url: url)
        observe(item)

        // EOF is per item, so bind the observer to this item instance.
        endObserver = NotificationCenter.default.addObserver(
            forName: .AVPlayerItemDidPlayToEndTime,
            object: item,
            queue: .main
        ) { [weak self] _ in
            self?.eof = true
        }

        player.replaceCurrentItem(with: item)
        player.play()
        #if targetEnvironment(simulator)
        if ProcessInfo.processInfo.environment["DADO_VIDEO_SMOKE"] == "1" {
            DispatchQueue.main.asyncAfter(deadline: .now()+4) { [weak self] in
                guard let self = self, self.player.currentItem?.status == .readyToPlay, self.player.currentTime().seconds > 0.5 else { return }
                FileManager.default.createFile(atPath: NSHomeDirectory()+"/Documents/video-ready", contents: Data())
            }
        }
        #endif
    }

    func stop() {
        player.pause()
        // Detach observers before dropping the item they observe.
        clearItemState()
        player.replaceCurrentItem(with: nil)
    }

    func setPaused(_ paused: Bool) {
        if paused {
            player.pause()
        } else {
            player.play()
        }
    }

    func seek(to seconds: Double) {
        let clamped = max(0, seconds)
        eof = false
        player.seek(to: CMTime(seconds: clamped, preferredTimescale: 600))
    }

    /// 0..100, matching mpv's volume scale.
    func setVolume(_ volume: Int) {
        let clamped = min(100, max(0, volume))
        player.volume = Float(clamped) / 100.0
    }

    // MARK: - State (read-only, safe to call from the ABI layer)

    var position: Double {
        let time = player.currentTime()
        guard time.isNumeric else { return 0 }
        let seconds = time.seconds
        return seconds.isFinite ? seconds : 0
    }

    var duration: Double {
        guard let item = player.currentItem else { return 0 }
        let time = item.duration
        guard time.isNumeric else { return 0 }
        let seconds = time.seconds
        return (seconds.isFinite && seconds > 0) ? seconds : 0
    }

    var cacheTime: Double {
        guard bufferedEnd > 0 else { return 0 }
        return max(0, bufferedEnd - position)
    }

    var paused: Bool {
        return player.rate == 0
    }

    /// Real frame rate from the active video track. 0 when AVFoundation does
    /// not report one (e.g. before playback starts) — never a guess.
    var fps: Double {
        guard let item = player.currentItem else { return 0 }
        for track in item.tracks {
            guard let assetTrack = track.assetTrack, assetTrack.mediaType == .video else { continue }
            let rate = track.currentVideoFrameRate
            if rate.isFinite && rate > 0 {
                return Double(rate)
            }
        }
        return 0
    }

    // MARK: - Observation

    private func observe(_ item: AVPlayerItem) {
        itemObservers = [
            item.observe(\.status, options: [.new, .initial]) { [weak self] item, _ in
                // KVO may fire on a background queue; state lives on main.
                let status = item.status
                let error = item.error
                DispatchQueue.main.async {
                    guard self?.player.currentItem === item else { return }
                    self?.apply(status: status, error: error)
                }
            },
            item.observe(\.presentationSize, options: [.new]) { [weak self] item, _ in
                let size = item.presentationSize
                DispatchQueue.main.async {
                    self?.videoSize = size
                }
            },
            item.observe(\.loadedTimeRanges, options: [.new]) { [weak self] item, _ in
                let end = item.loadedTimeRanges.last?.timeRangeValue.end.seconds ?? 0
                DispatchQueue.main.async {
                    self?.bufferedEnd = end.isFinite ? end : 0
                }
            }
        ]
    }

    private func apply(status: AVPlayerItem.Status, error: Error?) {
        switch status {
        case .readyToPlay:
            errorCode = nil
            if let item = player.currentItem {
                videoSize = item.presentationSize
            }
            NSLog("%@", "Dado: video item loaded")
        case .failed:
            // Never log the URL — only the code.
            let code: Int
            if let error = error {
                code = (error as NSError).code
            } else {
                code = -1
            }
            errorCode = code
            VideoHost.shared.showError(code)
            NSLog("%@", "Dado: video item failed (code \(code))")
        default:
            break
        }
    }

    private func clearItemState() {
        for observer in itemObservers {
            observer.invalidate()
        }
        itemObservers.removeAll()

        if let endObserver = endObserver {
            NotificationCenter.default.removeObserver(endObserver)
            self.endObserver = nil
        }

        eof = false
        bufferedEnd = 0
        videoSize = .zero
        errorCode = nil
    }
}
