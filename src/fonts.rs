//! Platform UI font.
//!
//! egui ships Ubuntu-Light, which looks foreign on both Windows and Android.
//! This loads a system face once and registers two extra families with the
//! medium and bold cuts, because egui has no font weights — `RichText::strong()`
//! only recolours text.
//!
//! Android note (verified on the API 34 image, do not "simplify" this):
//! Roboto ships as a **variable** font — `/system/fonts/Roboto-Regular.ttf` is
//! 2.4 MB with a weight axis, and there is no `Roboto-Bold.ttf`. `ab_glyph`
//! (what epaint uses) cannot select an axis, so asking Roboto for bold silently
//! yields regular weight. `DroidSans-Bold.ttf` looks like the escape hatch but
//! it is a symlink:
//!
//! ```text
//! DroidSans-Bold.ttf -> Roboto-Regular.ttf
//! ```
//!
//! So every "bold" rendered at weight 400. AOSP also ships Source Sans Pro with
//! genuine static cuts, which is what we use: three real weights, one coherent
//! family, zero APK bytes.

use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Context, FontData, FontDefinitions, FontFamily};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Set when `set_fonts` was called and the new families are not live yet.
static PENDING: AtomicBool = AtomicBool::new(false);

/// `(font_data key, file)` pairs, front to back.
type Faces = (
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
    &'static [&'static str],
);

fn candidates() -> Faces {
    #[cfg(target_os = "android")]
    {
        // Roboto stays in the chain *behind* Source Sans as a per-glyph
        // fallback, not as a conditional alternative: IPTV line-ups are full of
        // Cyrillic and Greek channel names, which Source Sans Pro does not
        // cover. epaint walks the family list per glyph, so mixing is safe.
        (
            &[
                ("ui-regular", "/system/fonts/SourceSansPro-Regular.ttf"),
                ("ui-regular-script", "/system/fonts/Roboto-Regular.ttf"),
            ],
            &["/system/fonts/SourceSansPro-SemiBold.ttf"],
            &["/system/fonts/SourceSansPro-Bold.ttf"],
        )
    }
    #[cfg(not(target_os = "android"))]
    {
        (
            &[("ui-regular", "C:/Windows/Fonts/segoeui.ttf")],
            &["C:/Windows/Fonts/segoeuisb.ttf"],
            &["C:/Windows/Fonts/segoeuib.ttf"],
        )
    }
}

/// True once the registered families are live. `Context::set_fonts` only takes
/// effect at the next `begin_frame`, so the first frame after `install()` must
/// not draw text with `ui-bold` / `ui-medium` (epaint panics with
/// "FontFamily::Name(...) is not bound to any fonts").
pub fn ready(_ctx: &Context) -> bool {
    !PENDING.swap(false, Ordering::Relaxed)
}

/// Install the fonts. Safe to call every frame; only the first call does work.
pub fn install(ctx: &Context) {
    if INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    let (regular, medium, bold) = candidates();
    let mut defs = FontDefinitions::default();

    // Loaded faces go to the front of Proportional, in order, so every
    // proportional string uses the system face; the bundled fonts stay behind
    // them as the last resort (symbols, emoji).
    let mut front: Vec<String> = Vec::new();
    for (key, file) in regular {
        if let Ok(bytes) = std::fs::read(file) {
            defs.font_data
                .insert((*key).to_owned(), FontData::from_owned(bytes));
            front.push((*key).to_owned());
        }
    }
    if let Some(fam) = defs.families.get_mut(&FontFamily::Proportional) {
        for (i, key) in front.iter().enumerate() {
            fam.insert(i, key.clone());
        }
    }

    // A missing face must never leave the family unbound: epaint panics with
    // "FontFamily::Name(..) is not bound to any fonts" when text is drawn.
    // So each named family always ends up non-empty — worst case it is just a
    // copy of Proportional (regular weight), which degrades but does not crash.
    for (name, files) in [("ui-bold", bold), ("ui-medium", medium)] {
        let mut fam = Vec::new();
        for file in files {
            if let Ok(bytes) = std::fs::read(file) {
                defs.font_data
                    .insert(name.to_owned(), FontData::from_owned(bytes));
                fam.push(name.to_owned());
                break;
            }
        }
        if let Some(defaults) = defs.families.get(&FontFamily::Proportional) {
            fam.extend(defaults.iter().cloned());
        }
        defs.families.insert(FontFamily::Name(name.into()), fam);
    }

    PENDING.store(true, Ordering::Relaxed);
    ctx.set_fonts(defs);
}
