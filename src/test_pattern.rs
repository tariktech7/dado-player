//! Runtime generation of the local RGB24 test-pattern AVI used by
//! "Ohne Anbieter testen" (`App::play_test_stream`).
//!
//! Rationale (must survive refactors): the shipped Android `libmpv` links a
//! `libavdevice.so` with **no input devices at all**, so `av://lavfi:testsrc`
//! is rejected ("Unknown lavf format lavfi"). Its `loadfile` options argument
//! also rejects the rawvideo geometry string ("option must be an integer").
//! An **uncompressed RGB24 AVI** needs neither: `libavformat`'s AVI demuxer
//! reads width/height/pixel-format straight from the `strf` BITMAPINFOHEADER
//! in the container header — no network, no per-file options, no demuxer
//! guesses.
//!
//! The byte layout is a direct port of `probe/video_test/gen_avi.py`, which
//! produces a file verified to play on this Android libmpv build. Keep it
//! byte-for-byte identical (apart from the W/H/FRAMES constants) so the
//! demuxer keeps accepting it. A per-frame blue tint acts as a frame counter:
//! two captures a second or two apart prove *live* rendering rather than a
//! frozen frame.
//!
//! W*3 = 288 is a multiple of 4, so BI_RGB rows need no DWORD padding. The
//! `chunk()` even-byte pad rule is nevertheless kept for every chunk.

use std::io::{self, BufWriter, Write};

const W: usize = 96;
const H: usize = 72;
const FPS: u32 = 10;
// 30 s clip, matching gen_avi.py:300. On the emulator a one-time libass/
// fontconfig stall delays the first real video render to ~22.6 s and playback
// only reaches EOF at ~40 s. A 10 s clip would already be over before a live
// frame ever reaches the surface, so QA captures come back black. mpv is built
// with keep-open=no and no loop, so clip length is the only in-scope lever here.
const FRAMES: usize = 300;
const FRAME_LEN: usize = W * H * 3; // 20736 bytes (BI_RGB, top-down)

/// Bytes a `chunk()` occupies: 4 (fourcc) + 4 (size) + data + even-byte pad.
/// The size field holds the *unpadded* data length (Python behaviour).
fn chunk_size(data_len: usize) -> u64 {
    8 + data_len as u64 + if !data_len.is_multiple_of(2) { 1 } else { 0 }
}

/// Bytes a `LIST` occupies: 4 ("LIST") + 4 (size) + 4 (list type) + body.
fn list_size(body_len: usize) -> u64 {
    12 + body_len as u64
}

/// Exact byte length of the generated AVI (6_228_232 bytes). Computed from
/// constants only, so the fast-path existence check never builds the file in
/// memory.
pub fn expected_size() -> u64 {
    let strf_chunk = chunk_size(40); // strf / BITMAPINFOHEADER (40 bytes)
    let strh_chunk = chunk_size(56); // strh / AVISTREAMHEADER (56 bytes)
    let strl = list_size((strh_chunk + strf_chunk) as usize);
    let avih_chunk = chunk_size(14 * 4); // avih = 14 DWORDs (56 bytes)
    let hdrl = list_size((avih_chunk + strl) as usize);
    let movi = list_size((chunk_size(FRAME_LEN) * FRAMES as u64) as usize);
    let idx1 = chunk_size(FRAMES * 16); // FRAMES * <4sIII> (16 bytes each) = 1600
    let body = hdrl + movi + idx1;
    12 + body // "RIFF" + u32(size) + "AVI " + body
}

/// Build a `chunk`: fourcc + u32(unpadded data len) + data + even-byte pad.
fn chunk(fourcc: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + data.len() + 1);
    v.extend_from_slice(fourcc);
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        v.push(0);
    }
    v
}

/// Build a `LIST`: "LIST" + u32(body len + 4 for the list type) + type + body.
fn list_chunk(ltype: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(12 + body.len());
    v.extend_from_slice(b"LIST");
    v.extend_from_slice(&((body.len() as u32) + 4).to_le_bytes());
    v.extend_from_slice(ltype);
    v.extend_from_slice(body);
    v
}

/// BITMAPINFOHEADER packed as `<IiiHHIIiiII` (40 bytes), top-down BI_RGB.
fn bitmap_info_header() -> Vec<u8> {
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&40u32.to_le_bytes()); // biSize
    v.extend_from_slice(&(W as i32).to_le_bytes()); // biWidth
    v.extend_from_slice(&(-(H as i32)).to_le_bytes()); // biHeight < 0 => top-down
    v.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    v.extend_from_slice(&24u16.to_le_bytes()); // biBitCount
    v.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
    v.extend_from_slice(&(FRAME_LEN as u32).to_le_bytes()); // biSizeImage
    v.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
    v.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
    v.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    v.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant
    v
}

/// AVIMAINHEADER: 14 little-endian DWORDs (56 bytes).
fn avi_main_header() -> Vec<u8> {
    let dwords = [
        1_000_000 / FPS,  // dwMicroSecPerFrame
        0,                // dwMaxBytesPerSec
        0,                // dwPaddingGranularity
        0x10,             // dwFlags (AVIF_HASINDEX)
        FRAMES as u32,    // dwTotalFrames
        0,                // dwInitialFrames
        1,                // dwStreams
        FRAME_LEN as u32, // dwSuggestedBufferSize
        W as u32,         // dwWidth
        H as u32,         // dwHeight
        0,
        0,
        0,
        0, // dwReserved[4]
    ];
    let mut v = Vec::with_capacity(56);
    for d in dwords {
        v.extend_from_slice(&d.to_le_bytes());
    }
    v
}

/// AVISTREAMHEADER packed as `<4s4sIIIIIIIIIIHHHH` (56 bytes).
fn stream_header() -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    v.extend_from_slice(b"vids"); // fccType
    v.extend_from_slice(b"DIB "); // fccHandler
    let dwords = [
        0u32,             // dwFlags
        0,                // dwPriority
        0,                // dwInitialFrames
        1,                // dwScale
        FPS,              // dwRate
        0,                // dwStart
        FRAMES as u32,    // dwLength
        FRAME_LEN as u32, // dwSuggestedBufferSize
        u32::MAX,         // dwQuality (-1)
        0,                // dwSampleSize
    ];
    for d in dwords {
        v.extend_from_slice(&d.to_le_bytes());
    }
    for _ in 0..4 {
        v.extend_from_slice(&0u16.to_le_bytes()); // rcFrame (left, top, right, bottom)
    }
    v
}

/// Fill one RGB24 frame: horizontal red ramp, vertical green ramp, blue tint
/// that changes with the frame index. Matches `gen_avi.py::make_frame`.
fn fill_frame(buf: &mut [u8], i: usize) {
    let tint = ((i * 4) % 256) as u8;
    for y in 0..H {
        let g = (y * 255 / (H - 1)) as u8;
        for x in 0..W {
            let r = (x * 255 / (W - 1)) as u8;
            let o = (y * W + x) * 3;
            buf[o] = r;
            buf[o + 1] = g;
            buf[o + 2] = tint;
        }
    }
}

/// Write one `00dc` frame chunk to `w`, returning the bytes written.
fn write_frame_chunk<W: Write>(w: &mut W, frame: &[u8]) -> io::Result<u64> {
    w.write_all(b"00dc")?;
    w.write_all(&(frame.len() as u32).to_le_bytes())?;
    w.write_all(frame)?;
    if !frame.len().is_multiple_of(2) {
        w.write_all(&[0u8])?;
    }
    Ok(chunk_size(frame.len()))
}

/// Generate the test AVI at `path`.
///
/// If `path` already exists with exactly [`expected_size`] bytes, nothing is
/// written. Parent directories are created on demand. The body is streamed
/// through one reusable frame buffer, so the ~6 MB file is never held in
/// memory in full.
pub fn write(path: &std::path::Path) -> io::Result<()> {
    let expected = expected_size();
    if std::fs::metadata(path)
        .map(|m| m.len() == expected)
        .unwrap_or(false)
    {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(path)?;
    let mut w = BufWriter::new(file);

    // ---- Sizes, computed from constants, consistent with expected_size() ----
    let movi_body_len = chunk_size(FRAME_LEN) * FRAMES as u64;
    let movi_total = list_size(movi_body_len as usize);
    let idx1_total = chunk_size(FRAMES * 16);

    // ---- hdrl (small, built once): avih + LIST strl { strh, strf } ----
    let strf = chunk(b"strf", &bitmap_info_header());
    let strh = chunk(b"strh", &stream_header());
    let mut strl_body = strh;
    strl_body.extend_from_slice(&strf);
    let strl = list_chunk(b"strl", &strl_body);
    let mut hdrl_body = chunk(b"avih", &avi_main_header());
    hdrl_body.extend_from_slice(&strl);
    let hdrl = list_chunk(b"hdrl", &hdrl_body);

    // Outer RIFF header. The size field is `len(body)` (Python's `body`),
    // i.e. total - 12; the "AVI " form type is NOT counted. ffmpeg's AVI
    // demuxer does not validate this field, which is why the proven Python
    // file keeps this slightly undercounted value — we reproduce it verbatim.
    let body_len = hdrl.len() as u64 + movi_total + idx1_total;
    debug_assert_eq!(body_len, expected - 12);
    w.write_all(b"RIFF")?;
    w.write_all(&(body_len as u32).to_le_bytes())?;
    w.write_all(b"AVI ")?;
    w.write_all(&hdrl)?;

    // ---- movi LIST: stream FRAMES frame chunks ----
    w.write_all(b"LIST")?;
    w.write_all(&((movi_body_len as u32) + 4).to_le_bytes())?;
    w.write_all(b"movi")?;
    let mut frame = vec![0u8; FRAME_LEN];
    for i in 0..FRAMES {
        fill_frame(&mut frame, i);
        let written = write_frame_chunk(&mut w, &frame)?;
        debug_assert_eq!(written, chunk_size(FRAME_LEN));
    }

    // ---- idx1 ----
    // Offsets are relative to the "movi" list-type fourcc: entry i points at
    // 4 + i*chunk_size (the `+4` is the "movi" tag). This reproduces Python's
    // `offset + 4` exactly. Size is the *frame data* length, not the chunk.
    let mut idx = Vec::with_capacity(FRAMES * 16);
    let mut offset: u64 = 0;
    for _ in 0..FRAMES {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&0x10u32.to_le_bytes()); // AVIIF_KEYFRAME
        idx.extend_from_slice(&((offset + 4) as u32).to_le_bytes());
        idx.extend_from_slice(&(FRAME_LEN as u32).to_le_bytes());
        offset += chunk_size(FRAME_LEN);
    }
    w.write_all(b"idx1")?;
    w.write_all(&(idx.len() as u32).to_le_bytes())?;
    w.write_all(&idx)?;
    if idx.len() % 2 != 0 {
        w.write_all(&[0u8])?;
    }

    w.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_avi_header_and_is_idempotent() {
        let path = std::env::temp_dir().join("dado-player-test-pattern-unit.avi");
        let _ = std::fs::remove_file(&path);

        write(&path).expect("first write");
        let bytes = std::fs::read(&path).expect("reopen");

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"AVI ");
        assert_eq!(bytes.len() as u64, expected_size());
        // gen_avi.py writes u32(len(body)) where `body` EXCLUDES the "AVI "
        // form type, so the RIFF size equals total - 12 (NOT the standard
        // total - 8). Reproduce that proven layout verbatim.
        let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(riff_size as u64, bytes.len() as u64 - 12);

        write(&path).expect("second write");
        let len2 = std::fs::metadata(&path).expect("metadata").len();
        assert_eq!(len2, bytes.len() as u64);

        let _ = std::fs::remove_file(&path);
    }
}
