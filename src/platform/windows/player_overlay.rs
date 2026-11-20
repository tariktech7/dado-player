//! Per-pixel player chrome over the existing native video window.
use egui::epaint::{ClippedShape, Primitive, Vertex, WHITE_UV};
use egui::{Context, LayerId, Pos2, Rect};
use std::ptr::{null, null_mut};
use std::sync::{Mutex, Once, OnceLock};
use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
pub struct Overlay {
    panels: Vec<Panel>,
}

impl Overlay {
    pub fn hide(&mut self) {
        for panel in &mut self.panels {
            panel.hide();
        }
    }

    pub fn paint(&mut self, ctx: &Context, video: isize, layers: &[(LayerId, Rect)]) -> bool {
        let _ = REPAINT.set(ctx.clone());
        let parent = unsafe { GetParent(video as HWND) };
        if parent.is_null() {
            return false;
        }
        let scale = ctx.pixels_per_point();
        for (index, &(layer, rect)) in layers.iter().enumerate() {
            if index == self.panels.len() {
                let Some(panel) = Panel::create(parent) else {
                    self.hide();
                    return false;
                };
                self.panels.push(panel);
            }
            let shapes = ctx
                .graphics(|layers| {
                    layers
                        .get(layer)
                        .map(|list| list.all_entries().cloned().collect())
                })
                .unwrap_or_default();
            if !self.panels[index].paint(ctx, rect, scale, shapes) {
                self.hide();
                return false;
            }
        }
        for panel in &mut self.panels[layers.len()..] {
            panel.hide();
        }
        true
    }
}

enum PointerEvent {
    Move(i32, i32),
    Button(i32, i32, bool),
    Scroll(f32, f32),
    Gone,
}

const OVERLAY_MOUSE_LEAVE: u32 = 0x02a3;
static MOUSE_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static INPUT: Mutex<Vec<PointerEvent>> = Mutex::new(Vec::new());
static REPAINT: OnceLock<Context> = OnceLock::new();

fn pointer_event(event: PointerEvent) {
    if let Ok(mut input) = INPUT.lock() {
        if matches!(event, PointerEvent::Move(..))
            && matches!(input.last(), Some(PointerEvent::Move(..)))
        {
            input.pop();
        }
        input.push(event);
    }
    if let Some(ctx) = REPAINT.get() {
        ctx.request_repaint();
    }
}

/// Child HWNDs are not Winit windows. Feed the overlay pointer into the same
/// egui input pass as the host's keyboard and viewport events.
pub fn read_input(ctx: &Context, raw: &mut egui::RawInput, active: bool) {
    let Ok(mut input) = INPUT.lock() else { return };
    let scale = ctx.pixels_per_point();
    let modifiers = raw.modifiers;
    let point = |x, y| egui::pos2(x as f32 / scale, y as f32 / scale);
    for event in input.drain(..) {
        if !active {
            continue;
        }
        match event {
            PointerEvent::Move(x, y) => raw.events.push(egui::Event::PointerMoved(point(x, y))),
            PointerEvent::Gone => raw.events.push(egui::Event::PointerGone),
            PointerEvent::Scroll(x, y) => raw.events.push(egui::Event::Scroll(egui::vec2(x, y))),
            PointerEvent::Button(x, y, pressed) => {
                raw.events.push(egui::Event::PointerMoved(point(x, y)));
                raw.events.push(egui::Event::PointerButton {
                    pos: point(x, y),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                });
            }
        }
    }
}

struct Panel {
    hwnd: HWND,
    visible: bool,
    geometry: [i32; 4],
    shapes: Vec<ClippedShape>,
    scale: f32,
}

impl Panel {
    fn create(parent: HWND) -> Option<Self> {
        static REGISTER: Once = Once::new();
        let class: Vec<u16> = "DadoPlayerOverlay\0".encode_utf16().collect();
        let hwnd = unsafe {
            let instance = GetModuleHandleW(null());
            REGISTER.call_once(|| {
                let mut wc: WNDCLASSW = std::mem::zeroed();
                wc.lpfnWndProc = Some(window_proc);
                wc.hInstance = instance;
                wc.lpszClassName = class.as_ptr();
                wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
                RegisterClassW(&wc);
            });
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_NOACTIVATE,
                class.as_ptr(),
                null(),
                WS_CHILD,
                0,
                0,
                1,
                1,
                parent,
                null_mut(),
                instance,
                null(),
            )
        };
        (!hwnd.is_null()).then_some(Self {
            hwnd,
            visible: false,
            geometry: [0; 4],
            shapes: Vec::new(),
            scale: 0.0,
        })
    }

    fn hide(&mut self) {
        if self.visible {
            unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            self.visible = false;
        }
    }

    fn paint(&mut self, ctx: &Context, rect: Rect, scale: f32, shapes: Vec<ClippedShape>) -> bool {
        let [x, y, right, bottom] = [rect.left(), rect.top(), rect.right(), rect.bottom()]
            .map(|point| (point * scale).round() as i32);
        let (width, height) = (right - x, bottom - y);
        let geometry = [x, y, width, height];
        if self.visible && self.geometry == geometry && self.scale == scale && self.shapes == shapes
        {
            return true;
        }
        if width <= 0 || height <= 0 {
            self.hide();
            return true;
        }
        let primitives = ctx.tessellate(shapes.clone(), scale);
        let atlas = ctx.fonts(|fonts| fonts.image());
        let coverage: Vec<u8> = atlas.srgba_pixels(None).map(|pixel| pixel.a()).collect();
        let mut pixels = vec![[0; 4]; width as usize * height as usize];
        for primitive in primitives {
            if let Primitive::Mesh(mesh) = primitive.primitive {
                for triangle in mesh.indices.chunks_exact(3) {
                    let v = [triangle[0], triangle[1], triangle[2]]
                        .map(|index| mesh.vertices[index as usize]);
                    rasterize(
                        &mut pixels,
                        geometry,
                        primitive.clip_rect,
                        scale,
                        v,
                        &coverage,
                        atlas.size,
                    );
                }
            }
        }
        if !upload(self.hwnd, geometry, &pixels) {
            return false;
        }
        self.geometry = geometry;
        self.shapes = shapes;
        self.scale = scale;
        self.visible = true;
        true
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

fn upload(hwnd: HWND, [x, y, width, height]: [i32; 4], pixels: &[[u8; 4]]) -> bool {
    unsafe {
        let dc = CreateCompatibleDC(null_mut());
        if dc.is_null() {
            return false;
        }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        let mut bits = null_mut();
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bitmap.is_null() || bits.is_null() {
            DeleteDC(dc);
            return false;
        }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast(), pixels.len());
        let old = SelectObject(dc, bitmap);
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let source = POINT { x: 0, y: 0 };
        let updated = UpdateLayeredWindow(
            hwnd,
            null_mut(),
            null(),
            &size,
            dc,
            &source,
            0,
            &blend,
            ULW_ALPHA,
        ) != 0;
        SelectObject(dc, old);
        DeleteObject(bitmap);
        DeleteDC(dc);
        updated
            && SetWindowPos(
                hwnd,
                HWND_TOP,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            ) != 0
    }
}

fn edge(a: Pos2, b: Pos2, p: Pos2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

fn rasterize(
    pixels: &mut [[u8; 4]],
    [x, y, width, height]: [i32; 4],
    clip: Rect,
    scale: f32,
    mut v: [Vertex; 3],
    atlas: &[u8],
    [atlas_w, atlas_h]: [usize; 2],
) {
    let origin = egui::vec2(x as f32, y as f32);
    let transform = |p: Pos2| p * scale - origin;
    let mut p = v.map(|vertex| transform(vertex.pos));
    let mut area = edge(p[0], p[1], p[2]);
    if area.abs() < f32::EPSILON {
        return;
    }
    if area < 0.0 {
        p.swap(1, 2);
        v.swap(1, 2);
        area = -area;
    }
    let bounds = p
        .iter()
        .fold(Rect::NOTHING, |rect, &point| {
            rect.union(Rect::from_min_max(point, point))
        })
        .intersect(Rect::from_min_max(transform(clip.min), transform(clip.max)));
    let left = (bounds.min.x.floor() as i32).clamp(0, width);
    let top = (bounds.min.y.floor() as i32).clamp(0, height);
    let right = (bounds.max.x.ceil() as i32).clamp(0, width);
    let bottom = (bounds.max.y.ceil() as i32).clamp(0, height);
    let edges = [(p[1], p[2]), (p[2], p[0]), (p[0], p[1])];
    let inclusive = edges.map(|(a, b)| b.y < a.y || (b.y == a.y && b.x > a.x));
    let rgba = v.map(|vertex| vertex.color.to_array());
    let textured = v.iter().any(|vertex| vertex.uv != WHITE_UV);
    let flat = !textured && rgba[0] == rgba[1] && rgba[0] == rgba[2];
    let inv_area = area.recip();
    for row in top..bottom {
        for col in left..right {
            let point = egui::pos2(col as f32 + 0.5, row as f32 + 0.5);
            let e = edges.map(|(a, b)| edge(a, b, point));
            if e.iter()
                .zip(inclusive)
                .any(|(&value, include)| value < 0.0 || (value == 0.0 && !include))
            {
                continue;
            }
            let sample = if flat {
                rgba[0]
            } else {
                let [a, b, c] = e.map(|value| value * inv_area);
                let coverage = if textured {
                    let u = a * v[0].uv.x + b * v[1].uv.x + c * v[2].uv.x;
                    let t = a * v[0].uv.y + b * v[1].uv.y + c * v[2].uv.y;
                    let tx = ((u * atlas_w as f32) as usize).min(atlas_w.saturating_sub(1));
                    let ty = ((t * atlas_h as f32) as usize).min(atlas_h.saturating_sub(1));
                    f32::from(atlas.get(ty * atlas_w + tx).copied().unwrap_or(0)) / 255.0
                } else {
                    1.0
                };
                std::array::from_fn(|i| {
                    ((a * f32::from(rgba[0][i])
                        + b * f32::from(rgba[1][i])
                        + c * f32::from(rgba[2][i]))
                        * coverage)
                        .round() as u8
                })
            };
            let dst = &mut pixels[(row * width + col) as usize];
            let inv = u16::from(255 - sample[3]);
            for (target, source) in dst
                .iter_mut()
                .zip([sample[2], sample[1], sample[0], sample[3]])
            {
                *target = source.saturating_add(((u16::from(*target) * inv + 127) / 255) as u8);
            }
        }
    }
}

pub fn cursor_inside(video: isize, rect: Rect, scale: f32) -> bool {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        if GetCursorPos(&mut pt) == 0 {
            return false;
        }
        MapWindowPoints(null_mut(), GetParent(video as HWND), &mut pt, 1);
    }
    rect.contains(egui::pos2(pt.x as f32 / scale, pt.y as f32 / scale))
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match message {
        WM_MOUSEACTIVATE => return MA_NOACTIVATE as isize,
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP => {
            let parent = GetParent(hwnd);
            let mut point = POINT {
                x: (lparam as i16) as i32,
                y: ((lparam >> 16) as i16) as i32,
            };
            MapWindowPoints(hwnd, parent, &mut point, 1);
            if message == WM_MOUSEMOVE {
                TrackMouseEvent(&mut TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                });
                pointer_event(PointerEvent::Move(point.x, point.y));
            } else {
                let pressed = message == WM_LBUTTONDOWN;
                MOUSE_HELD.store(pressed, std::sync::atomic::Ordering::Relaxed);
                if pressed {
                    SetFocus(parent);
                    SetCapture(hwnd);
                } else if GetCapture() == hwnd {
                    ReleaseCapture();
                }
                pointer_event(PointerEvent::Button(point.x, point.y, pressed));
            }
            return 0;
        }
        WM_CAPTURECHANGED => {
            if MOUSE_HELD.swap(false, std::sync::atomic::Ordering::Relaxed) {
                pointer_event(PointerEvent::Button(-1, -1, false));
                pointer_event(PointerEvent::Gone);
            }
        }
        OVERLAY_MOUSE_LEAVE => {
            if GetCapture() != hwnd {
                pointer_event(PointerEvent::Gone);
            }
            return 0;
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            let delta = f32::from((wparam >> 16) as i16) * (48.0 / 120.0);
            pointer_event(if message == WM_MOUSEWHEEL {
                PointerEvent::Scroll(0.0, delta)
            } else {
                PointerEvent::Scroll(-delta, 0.0)
            });
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}
