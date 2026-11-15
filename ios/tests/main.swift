import Foundation

let fixtures: [(String, PlaybackEngine)] = [
    ("https://provider.invalid/live/u/p/1.m3u8?token=opaque", .apple),
    ("https://provider.invalid/live/u/p/1.M3U8", .apple),
    ("https://provider.invalid/movie/u/p/1.mkv", .vlc),
    ("https://provider.invalid/series/u/p/1.avi", .vlc),
    ("https://provider.invalid/movie/u/p/1.mp4", .vlc),
    ("https://provider.invalid/live/u/p/1.ts", .vlc),
    ("https://provider.invalid/opaque", .vlc),
]
for (value, expected) in fixtures {
    let url = URL(string: value)!
    precondition(PlaybackPolicy.initial(url: url, preference: "auto", nativeControls: false) == expected)
    precondition(PlaybackPolicy.initial(url: url, preference: "vlc", nativeControls: false) == .vlc)
    precondition(PlaybackPolicy.initial(url: url, preference: "auto", nativeControls: true) == .apple)
    precondition(PlaybackPolicy.initial(url: url, preference: "apple", nativeControls: false) == .apple)
}
precondition(PlaybackPolicy.canFallback(preference: "auto", nativeControls: false, triedVLC: false))
precondition(!PlaybackPolicy.canFallback(preference: "auto", nativeControls: false, triedVLC: true))
precondition(!PlaybackPolicy.canFallback(preference: "auto", nativeControls: true, triedVLC: false))
precondition(!PlaybackPolicy.canFallback(preference: "apple", nativeControls: false, triedVLC: false))
print("Playback policy: HLS/files/opaque URLs, expert choices and bounded fallback passed")
