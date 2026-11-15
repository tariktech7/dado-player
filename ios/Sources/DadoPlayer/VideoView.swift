//
//  VideoView.swift
//  Dado Player — AVPlayer and embedded VLCKit video surfaces.
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
import CoreMedia
import MobileVLCKit

final class VideoView: UIView, VLCMediaPlayerDelegate {
    private let vlcSurface = UIView()
    private var vlc: VLCMediaPlayer?
    private var sourceURL: URL?
    private var triedVLC = false
    private var requestedPause = false
    private var volume = 100
    private var generation = 0

    override func layoutSubviews() {
        super.layoutSubviews()
        vlcSurface.frame = bounds
    }

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
    private var pendingSeek: Double?
    private var playbackOptions: [String: Any] = [:]
    private var startupTimeout: DispatchWorkItem?

    /// Set by AVPlayerItemDidPlayToEndTime, cleared on load/seek/stop.
    private(set) var eof: Bool = false

    /// Decoded video size in points (already scaled by the item's
    /// presentationSize). Zero until the item is ready.
    private(set) var videoSize: CGSize = .zero

    /// Last failure code, exposed through dado_host_error_code after fallback.
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
        vlcSurface.frame = bounds
        vlcSurface.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        vlcSurface.isHidden = true
        addSubview(vlcSurface)
        player.actionAtItemEnd = .pause
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .moviePlayback)
            try AVAudioSession.sharedInstance().setActive(true)
        } catch { NSLog("Dado: audio session unavailable") }
    }

    // MARK: - Control

    func configure(options: [String: Any]) { playbackOptions = options }

    func load(url: URL) {
        stop()
        sourceURL = url
        triedVLC = false
        requestedPause = false
        let native = VideoHost.shared.nativePlayer
        let engine = playbackOptions["engine"] as? String ?? "auto"
        // AVFoundation is excellent for HLS. File streams need a demuxer that
        // understands providers' real containers (MKV/AVI/TS), not renamed URLs.
        if PlaybackPolicy.initial(url: url, preference: engine, nativeControls: native) == .vlc {
            startVLC(url: url)
            return
        }
        loadApple(url: url)
    }

    private func loadApple(url: URL) {
        clearItemState()

        var assetOptions: [String: Any] = [:]
        if #available(iOS 16.0, *), let agent = playbackOptions["userAgent"] as? String, !agent.isEmpty {
            assetOptions[AVURLAssetHTTPUserAgentKey] = agent
        }
        let item = AVPlayerItem(asset: AVURLAsset(url: url, options: assetOptions))
        item.preferredForwardBufferDuration = min(600, max(5, (playbackOptions["buffer"] as? Double) ?? 45))
        let size = min(120, max(20, (playbackOptions["subtitleSize"] as? Double) ?? 55))
        item.textStyleRules = [AVTextStyleRule(textMarkupAttributes: [kCMTextMarkupAttribute_RelativeFontSize as String: size * 100 / 55])].compactMap { $0 }
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
        let timeout = DispatchWorkItem { [weak self, weak item] in
            guard let self, let item, self.player.currentItem === item, item.status == .unknown else { return }
            self.failOrFallback(-1001)
        }
        startupTimeout = timeout
        DispatchQueue.main.asyncAfter(deadline: .now() + min(120, max(5, (playbackOptions["timeout"] as? Double) ?? 20)), execute: timeout)
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
        generation += 1
        sourceURL = nil
        vlc?.delegate = nil
        vlc?.stop()
        vlc?.drawable = nil
        vlc = nil
        vlcSurface.isHidden = true
        playerLayer.player = player
        player.pause()
        // Detach observers before dropping the item they observe.
        clearItemState()
        player.replaceCurrentItem(with: nil)
    }

    func setPaused(_ paused: Bool) {
        requestedPause = paused
        if let vlc { if paused { vlc.pause() } else { vlc.play() }; return }
        if paused {
            player.pause()
        } else {
            player.play()
        }
    }

    func seek(to seconds: Double) {
        guard seconds.isFinite else { return }
        let clamped = max(0, seconds)
        if let vlc {
            eof = false
            if vlc.isSeekable { vlc.time = VLCTime(number: NSNumber(value: clamped * 1000)) }
            else { pendingSeek = clamped }
            return
        }
        guard player.currentItem?.status == .readyToPlay else {pendingSeek=clamped;return}
        eof = false
        player.seek(to: CMTime(seconds: clamped, preferredTimescale: 600))
    }

    /// 0..100, matching mpv's volume scale.
    func setVolume(_ volume: Int) {
        let clamped = min(100, max(0, volume))
        self.volume = clamped
        vlc?.audio?.volume = Int32(clamped)
        player.volume = Float(clamped) / 100.0
    }

    // MARK: - State (read-only, safe to call from the ABI layer)

    var position: Double {
        if let vlc { return max(0, (vlc.time.value?.doubleValue ?? 0) / 1000) }
        let time = player.currentTime()
        guard time.isNumeric else { return 0 }
        let seconds = time.seconds
        return seconds.isFinite ? seconds : 0
    }

    var duration: Double {
        if let vlc { return max(0, (vlc.media?.length.value?.doubleValue ?? 0) / 1000) }
        guard let item = player.currentItem else { return 0 }
        let time = item.duration
        guard time.isNumeric else { return 0 }
        let seconds = time.seconds
        return (seconds.isFinite && seconds > 0) ? seconds : 0
    }

    var cacheTime: Double {
        if vlc != nil { return 0 } // VLCKit does not expose seconds buffered.
        guard bufferedEnd > 0 else { return 0 }
        return max(0, bufferedEnd - position)
    }

    var paused: Bool {
        if vlc != nil { return requestedPause }
        return player.rate == 0
    }

    /// Real frame rate from the active video track. 0 when AVFoundation does
    /// not report one (e.g. before playback starts) — never a guess.
    var fps: Double {
        if vlc != nil { return 0 } // Never invent a frame rate.
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
                    guard self?.player.currentItem === item else { return }
                    self?.videoSize = size
                }
            },
            item.observe(\.loadedTimeRanges, options: [.new]) { [weak self] item, _ in
                let end = item.loadedTimeRanges.last?.timeRangeValue.end.seconds ?? 0
                DispatchQueue.main.async {
                    guard self?.player.currentItem === item else { return }
                    self?.bufferedEnd = end.isFinite ? end : 0
                }
            }
        ]
    }

    private func apply(status: AVPlayerItem.Status, error: Error?) {
        switch status {
        case .readyToPlay:
            startupTimeout?.cancel()
            errorCode = nil
            if let seconds=pendingSeek {pendingSeek=nil;seek(to:seconds)}
            if let item = player.currentItem {
                videoSize = item.presentationSize
            }
            NSLog("%@", "Dado: video item loaded")
        case .failed:
            startupTimeout?.cancel()
            // Never log the URL — only the code.
            let code: Int
            if let error = error {
                code = (error as NSError).code
            } else {
                code = -1
            }
            failOrFallback(code)
            NSLog("%@", "Dado: video item failed (code \(code))")
        default:
            break
        }
    }


    private func failOrFallback(_ code: Int) {
        let engine = playbackOptions["engine"] as? String ?? "auto"
        if PlaybackPolicy.canFallback(preference: engine, nativeControls: VideoHost.shared.nativePlayer, triedVLC: triedVLC), let url = sourceURL {
            let resume = pendingSeek ?? position
            player.pause()
            clearItemState()
            player.replaceCurrentItem(with: nil)
            pendingSeek = resume > 0 ? resume : nil
            startVLC(url: url)
            return
        }
        startupTimeout?.cancel()
        errorCode = code
        player.pause()
        VideoHost.shared.showError(code)
    }

    private func startVLC(url: URL) {
        triedVLC = true
        player.pause()
        playerLayer.player = nil
        vlcSurface.isHidden = false
        // Disable library logging: URLs contain provider credentials.
        let decoder = VLCMediaPlayer(options: ["--quiet", "--no-video-title-show"])
        vlc = decoder
        decoder.delegate = self
        decoder.drawable = vlcSurface
        let media = VLCMedia(url: url)
        let cache = Int(min(600, max(5, (playbackOptions["buffer"] as? Double) ?? 45)) * 1000)
        let seconds = min(120, max(5, (playbackOptions["timeout"] as? Double) ?? 20))
        media.addOptions(["network-caching": cache, "file-caching": 1000,
                          "http-reconnect": true, "tcp-timeout": Int(seconds * 1000000)])
        if let agent = playbackOptions["userAgent"] as? String, !agent.isEmpty {
            media.addOption(":http-user-agent=\(agent)")
        }
        let size = min(120, max(20, (playbackOptions["subtitleSize"] as? Double) ?? 55))
        media.addOption(":freetype-rel-fontsize=\(Int(size * 16 / 55))")
        decoder.media = media
        decoder.audio?.volume = Int32(volume)
        decoder.play()
        let currentGeneration = generation
        let timeout = DispatchWorkItem { [weak self, weak decoder] in
            guard let self, self.generation == currentGeneration,
                  let decoder, self.vlc === decoder else { return }
            self.errorCode = -1001
            decoder.stop()
        }
        startupTimeout = timeout
        DispatchQueue.main.asyncAfter(deadline: .now() + seconds, execute: timeout)
    }

    func mediaPlayerStateChanged(_ notification: Notification) { refreshVLC(notification) }
    func mediaPlayerTimeChanged(_ notification: Notification) { refreshVLC(notification) }
    private func refreshVLC(_ notification: Notification) {
        DispatchQueue.main.async { [weak self] in
            guard let self, let decoder = self.vlc,
                  let sender = notification.object as? VLCMediaPlayer, sender === decoder else { return }
            switch decoder.state {
            case .playing, .paused:
                self.startupTimeout?.cancel()
                self.errorCode = nil
                self.videoSize = decoder.videoSize
                #if targetEnvironment(simulator)
                if ProcessInfo.processInfo.environment["DADO_VIDEO_SMOKE"] == "1",
                   decoder.hasVideoOut, self.position > 0.5 {
                    FileManager.default.createFile(atPath: NSHomeDirectory()+"/Documents/video-ready", contents: Data())
                }
                #endif
                if let seek = self.pendingSeek, decoder.isSeekable {
                    self.pendingSeek = nil
                    self.seek(to: seek)
                }
                if self.requestedPause && decoder.isPlaying { decoder.pause() }
            case .ended:
                self.eof = true
                self.startupTimeout?.cancel()
            case .error:
                self.errorCode = -2
                self.startupTimeout?.cancel()
            default: break
            }
        }
    }

    private func mediaGroup(_ kind: Int32) -> AVMediaSelectionGroup? {
        player.currentItem?.asset.mediaSelectionGroup(forMediaCharacteristic: kind == 0 ? .audible : .legible)
    }
    var tracks: [[String: Any]] {
        var result: [[String: Any]] = []
        for kind in [Int32(0), Int32(1)] {
            if let vlc {
                let ids = (kind == 0 ? vlc.audioTrackIndexes : vlc.videoSubTitlesIndexes) as? [NSNumber] ?? []
                let names = (kind == 0 ? vlc.audioTrackNames : vlc.videoSubTitlesNames) as? [String] ?? []
                let metadata = vlc.media?.tracksInformation as? [[String: Any]] ?? []
                for (offset, id) in ids.enumerated() where id.intValue >= 0 {
                    let type = kind == 0 ? VLCMediaTracksInformationTypeAudio : VLCMediaTracksInformationTypeText
                    let info = metadata.first { ($0[VLCMediaTracksInformationId] as? NSNumber)?.intValue == id.intValue && ($0[VLCMediaTracksInformationType] as? String) == type }
                    // Shared UI reserves <=0 for disabled, VLC IDs start at 0.
                    result.append(["id": id.intValue + 1, "type": kind == 0 ? "audio" : "sub",
                                   "lang": info?[VLCMediaTracksInformationLanguage] as? String ?? "", "title": offset < names.count ? names[offset] : "Spur \(offset + 1)"])
                }
            } else if let group = mediaGroup(kind) {
                for (index, option) in group.options.enumerated() {
                    result.append(["id": index + 1, "type": kind == 0 ? "audio" : "sub",
                                   "lang": option.extendedLanguageTag ?? "", "title": option.displayName])
                }
            }
        }
        return result
    }
    func currentTrack(_ kind: Int32) -> Int64 {
        if let vlc {
            let native = kind == 0 ? vlc.currentAudioTrackIndex : vlc.currentVideoSubTitleIndex
            return native < 0 ? -1 : Int64(native) + 1
        }
        guard let item = player.currentItem, let group = mediaGroup(kind),
              let selected = item.currentMediaSelection.selectedMediaOption(in: group),
              let index = group.options.firstIndex(of: selected) else { return -1 }
        return Int64(index + 1)
    }
    func selectTrack(_ kind: Int32, _ id: Int64) {
        if let vlc {
            let native: Int32 = id <= 0 ? -1 : Int32(clamping: id - 1)
            if kind == 0 { vlc.currentAudioTrackIndex = native } else { vlc.currentVideoSubTitleIndex = native }
            return
        }
        guard let item = player.currentItem, let group = mediaGroup(kind) else { return }
        if id <= 0 { if group.allowsEmptySelection { item.select(nil, in: group) }; return }
        guard id <= Int64(group.options.count) else { return }
        item.select(group.options[Int(id - 1)], in: group)
    }

    private func clearItemState() {
        startupTimeout?.cancel()
        startupTimeout = nil
        for observer in itemObservers {
            observer.invalidate()
        }
        itemObservers.removeAll()

        if let endObserver = endObserver {
            NotificationCenter.default.removeObserver(endObserver)
            self.endObserver = nil
        }

        eof = false
        pendingSeek=nil
        bufferedEnd = 0
        videoSize = .zero
        errorCode = nil
    }
}
