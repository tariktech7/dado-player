//! Android VLC video sink. VLC writes aligned BGRA frames on its decoder
//! thread, and egui uploads only the most recent complete frame.
use super::{c_char, c_uint, Handle, Library};
use std::{ptr, sync::Mutex};

#[derive(Clone, Copy, Default)]
struct Layout {
    width: usize,
    height: usize,
    pitch: usize,
    lines: usize,
}

#[repr(align(32))]
#[derive(Clone)]
struct Block {
    _bytes: [u8; 32],
}

struct Picture {
    layout: Layout,
    bytes: Vec<Block>,
}

#[derive(Default)]
struct State {
    layout: Layout,
    spare: Option<Box<Picture>>,
    ready: Option<egui::ColorImage>,
}

#[derive(Default)]
pub(super) struct Frames(Mutex<State>);

type Setup = unsafe extern "C" fn(
    *mut Handle,
    *mut c_char,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
) -> c_uint;
type Lock = unsafe extern "C" fn(Handle, *mut Handle) -> Handle;
type Unlock = unsafe extern "C" fn(Handle, Handle, *const Handle);
type Callbacks = unsafe extern "C" fn(
    Handle,
    Lock,
    Unlock,
    Option<unsafe extern "C" fn(Handle, Handle)>,
    Handle,
);
type Format = unsafe extern "C" fn(Handle, Setup, Option<unsafe extern "C" fn(Handle)>);

impl Frames {
    pub(super) fn attach(&self, library: &Library, player: Handle) -> Result<(), String> {
        unsafe {
            let callbacks = library
                .get::<Callbacks>(b"libvlc_video_set_callbacks\0")
                .map_err(|e| e.to_string())?;
            let format = library
                .get::<Format>(b"libvlc_video_set_format_callbacks\0")
                .map_err(|e| e.to_string())?;
            callbacks(
                player,
                lock,
                unlock,
                None,
                (self as *const Self).cast_mut().cast(),
            );
            format(player, setup, None);
        }
        Ok(())
    }

    pub(super) fn clear(&self) {
        self.0.lock().unwrap().ready = None;
    }

    pub(super) fn take(&self) -> Option<egui::ColorImage> {
        self.0.lock().unwrap().ready.take()
    }
}

unsafe extern "C" fn setup(
    opaque: *mut Handle,
    chroma: *mut c_char,
    width: *mut c_uint,
    height: *mut c_uint,
    pitch: *mut c_uint,
    lines: *mut c_uint,
) -> c_uint {
    if *width == 0 || *height == 0 {
        return 0;
    }
    let frames = &*((*opaque).cast::<Frames>());
    let scale = (960.0 / f64::from((*width).max(*height))).min(1.0);
    *width = (f64::from(*width) * scale).round().max(1.0) as c_uint;
    *height = (f64::from(*height) * scale).round().max(1.0) as c_uint;
    *pitch = (*width * 4).next_multiple_of(32);
    *lines = (*height).next_multiple_of(32);
    ptr::copy_nonoverlapping(b"RV32".as_ptr().cast(), chroma, 4);
    let mut state = frames.0.lock().unwrap();
    state.layout = Layout {
        width: *width as usize,
        height: *height as usize,
        pitch: *pitch as usize,
        lines: *lines as usize,
    };
    state.spare = None;
    1
}

unsafe extern "C" fn lock(opaque: Handle, planes: *mut Handle) -> Handle {
    let frames = &*opaque.cast::<Frames>();
    let mut state = frames.0.lock().unwrap();
    let layout = state.layout;
    let mut picture = state.spare.take().unwrap_or_else(|| {
        Box::new(Picture {
            layout,
            bytes: vec![Block { _bytes: [0; 32] }; layout.lines * layout.pitch / 32],
        })
    });
    *planes = picture.bytes.as_mut_ptr().cast();
    Box::into_raw(picture).cast()
}

unsafe extern "C" fn unlock(opaque: Handle, picture: Handle, _planes: *const Handle) {
    let frames = &*opaque.cast::<Frames>();
    let picture = Box::from_raw(picture.cast::<Picture>());
    let layout = picture.layout;
    let mut image = egui::ColorImage::new([layout.width, layout.height], egui::Color32::BLACK);
    let bytes = std::slice::from_raw_parts(
        picture.bytes.as_ptr().cast::<u8>(),
        picture.bytes.len() * 32,
    );
    for (row, output) in bytes
        .chunks_exact(layout.pitch)
        .zip(image.pixels.chunks_exact_mut(layout.width))
    {
        for (bgra, pixel) in row.chunks_exact(4).zip(output) {
            *pixel = egui::Color32::from_rgb(bgra[2], bgra[1], bgra[0]);
        }
    }
    let mut state = frames.0.lock().unwrap();
    if state.layout.width == layout.width && state.layout.height == layout.height {
        state.ready = Some(image);
        state.spare = Some(picture);
        drop(state);
        crate::platform::android::jni_bridge::wake_ui();
    }
}
