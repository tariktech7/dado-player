import Foundation

enum PlaybackEngine: String { case auto, apple, vlc }

enum PlaybackPolicy {
    static func initial(url: URL, preference: String, nativeControls: Bool) -> PlaybackEngine {
        if nativeControls { return .apple }
        switch PlaybackEngine(rawValue: preference) ?? .auto {
        case .apple: return .apple
        case .vlc: return .vlc
        case .auto: return url.pathExtension.lowercased() == "m3u8" ? .apple : .vlc
        }
    }

    static func canFallback(preference: String, nativeControls: Bool, triedVLC: Bool) -> Bool {
        !nativeControls && !triedVLC && (PlaybackEngine(rawValue: preference) ?? .auto) == .auto
    }
}
