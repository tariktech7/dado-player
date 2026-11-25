//! Windows entitlement: one verified local certificate, never a playback heartbeat.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Deserialize)]
struct Config {
    enabled: bool,
    #[serde(rename = "apiUrl")]
    api_url: String,
    keys: HashMap<String, Jwk>,
}
#[derive(Deserialize)]
struct Jwk {
    x: String,
    y: String,
}
fn config() -> &'static Config {
    static C: OnceLock<Config> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../licensing/public-config.json"))
            .expect("license configuration")
    })
}
pub fn enabled() -> bool {
    config().enabled && !cfg!(test)
}
#[derive(Clone, Deserialize)]
pub struct Claims {
    v: u8,
    product: String,
    subject: String,
    platform: String,
    pub kind: String,
    issued_at: i64,
    started_at: i64,
    pub expires_at: Option<i64>,
    license_id: String,
}
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    token: String,
    high: i64,
    last_check: i64,
}
fn claims_allow_access(claims: Option<&Claims>, now: i64) -> bool {
    claims.is_some_and(|c| c.kind == "permanent" || c.expires_at.is_some_and(|end| now < end))
}
struct State {
    subject: String,
    code: String,
    macs: Vec<String>,
    claims: Option<Claims>,
    saved: Saved,
    error: String,
    busy: bool,
    auto_started: bool,
    expiry_checked: bool,
    origin: i64,
    tick: Instant,
}
fn utc() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn fingerprint(kind: &str, id: &str) -> String {
    ring::digest::digest(
        &ring::digest::SHA256,
        format!("dado-device-v1:{kind}:{id}").as_bytes(),
    )
    .as_ref()
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect()
}
fn verify(token: &str, subject: &str) -> Result<Claims, String> {
    verify_with_keys(token, subject, &config().keys)
}
fn verify_with_keys(
    token: &str,
    subject: &str,
    keys: &HashMap<String, Jwk>,
) -> Result<Claims, String> {
    let check = || -> Option<Claims> {
        if token.len() > 8192 {
            return None;
        }
        let p = token.split('.').collect::<Vec<_>>();
        if p.len() != 3 {
            return None;
        }
        let h: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(p[0]).ok()?).ok()?;
        if h["alg"] != "ES256" || h["typ"] != "DADO-LICENSE" {
            return None;
        }
        let key = keys.get(h["kid"].as_str()?)?;
        let x = URL_SAFE_NO_PAD.decode(&key.x).ok()?;
        let y = URL_SAFE_NO_PAD.decode(&key.y).ok()?;
        if x.len() != 32 || y.len() != 32 {
            return None;
        }
        let mut point = vec![4];
        point.extend(x);
        point.extend(y);
        let signature = URL_SAFE_NO_PAD.decode(p[2]).ok()?;
        ring::signature::UnparsedPublicKey::new(&ring::signature::ECDSA_P256_SHA256_FIXED, point)
            .verify(format!("{}.{}", p[0], p[1]).as_bytes(), &signature)
            .ok()?;
        let c: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(p[1]).ok()?).ok()?;
        if c.v != 1
            || c.product != "dado-player"
            || c.platform != "windows"
            || c.subject != subject
            || c.license_id.is_empty()
            || c.issued_at < 0
        {
            return None;
        }
        match c.kind.as_str() {
            "permanent" if c.expires_at.is_none() => Some(c),
            "trial" if c.started_at >= 0 && c.started_at.checked_add(604800) == c.expires_at => {
                Some(c)
            }
            _ => None,
        }
    };
    check().ok_or_else(|| "Lizenz ungültig oder für ein anderes Gerät ausgestellt.".into())
}
impl State {
    fn now(&self) -> i64 {
        utc()
            .max(self.saved.high)
            .max(self.origin + self.tick.elapsed().as_secs() as i64)
    }
    fn allowed(&self) -> bool {
        !enabled() || claims_allow_access(self.claims.as_ref(), self.now())
    }
    fn text(&self) -> String {
        match &self.claims {
            Some(c) if c.kind == "permanent" => "Unlimited".into(),
            Some(c) => {
                let left = (c.expires_at.unwrap_or(0) - self.now()).max(0);
                if left == 0 {
                    "Testzeit abgelaufen".into()
                } else {
                    let days = (left + 86399) / 86400;
                    format!(
                        "Noch {days} {} Testzeit",
                        if days == 1 { "Tag" } else { "Tage" }
                    )
                }
            }
            None => "Testzeit noch nicht gestartet".into(),
        }
    }
    fn save(&mut self) -> Result<(), String> {
        self.saved.high = self.now();
        let path = crate::store::license_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| "Lizenzspeicher nicht verfügbar")?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec(&self.saved).map_err(|_| "Lizenzspeicher nicht verfügbar")?,
        )
        .and_then(|_| std::fs::rename(tmp, path))
        .map_err(|_| "Lizenz konnte nicht dauerhaft gespeichert werden.".into())
    }
}
fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        let id = if cfg!(test) {
            Some("unit-test-device".into())
        } else {
            crate::platform::windows::identity::machine_id()
        };
        let subject = id
            .as_ref()
            .map(|id| fingerprint("windows", &id.to_lowercase()))
            .unwrap_or_default();
        let code = if subject.len() == 64 {
            format!(
                "D7-{}",
                subject.as_bytes()[..24]
                    .chunks(4)
                    .map(|c| std::str::from_utf8(c).unwrap().to_uppercase())
                    .collect::<Vec<_>>()
                    .join("-")
            )
        } else {
            "Nicht verfügbar".into()
        };
        let saved: Saved = std::fs::read(crate::store::license_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let claims = verify(&saved.token, &subject).ok();
        let origin = utc()
            .max(saved.high)
            .max(claims.as_ref().map(|c| c.issued_at).unwrap_or(0));
        Mutex::new(State {
            subject,
            code,
            macs: if cfg!(test) {
                Vec::new()
            } else {
                crate::platform::windows::identity::mac_addresses()
            },
            claims,
            saved,
            error: if id.is_none() {
                "Windows-Gerätekennung konnte nicht gelesen werden.".into()
            } else {
                String::new()
            },
            busy: false,
            auto_started: false,
            expiry_checked: false,
            origin,
            tick: Instant::now(),
        })
    })
}
pub fn allowed() -> bool {
    state().lock().unwrap().allowed()
}
pub fn tick(ctx: &Context) {
    let should_request = {
        let mut s = state().lock().unwrap();
        if !enabled() {
            return;
        }
        let first = !s.auto_started;
        s.auto_started = true;
        let expired = s
            .claims
            .as_ref()
            .is_some_and(|c| c.kind == "trial" && c.expires_at.is_some_and(|t| s.now() >= t));
        let expiry = expired && !s.expiry_checked;
        if expiry {
            s.expiry_checked = true;
        }
        if let Some(c) = &s.claims {
            if c.kind == "trial" && c.expires_at.is_some_and(|t| t > s.now()) {
                ctx.request_repaint_after(Duration::from_secs(
                    (((c.expires_at.unwrap_or(0) - s.now() - 1) % 86400) + 1).clamp(1, 3600) as u64,
                ));
            }
        }
        (first
            && s.claims.as_ref().is_some_and(|c| c.kind == "trial")
            && s.now() - s.saved.last_check >= 21600)
            || expiry
    };
    if should_request {
        request(ctx, RequestMode::Status);
    }
}
pub fn refresh(ctx: &Context) {
    request(ctx, RequestMode::Activation);
}
#[derive(Clone, Copy, PartialEq)]
enum RequestMode {
    Trial,
    Activation,
    Status,
}
impl RequestMode {
    fn path(self) -> &'static str {
        match self {
            Self::Trial => "/v1/trial/start",
            Self::Activation => "/v1/license/activation",
            Self::Status => "/v1/license",
        }
    }
    fn accepts(self, claims: &Claims) -> bool {
        self != Self::Activation || claims.kind == "permanent"
    }
}
fn request(ctx: &Context, mode: RequestMode) {
    let (subject, mac) = {
        let mut s = state().lock().unwrap();
        if s.busy || s.subject.is_empty() || !enabled() {
            return;
        }
        s.busy = true;
        s.error.clear();
        (s.subject.clone(), s.macs.first().cloned())
    };
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<(String, Claims), String> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "Verbindung konnte nicht gestartet werden.")?;
            let token=rt.block_on(async{
            let mac_hash=mac.map(|m|fingerprint("mac-alias",&m));
            let client=reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|_|"Verbindung fehlgeschlagen")?;
            let r=client.post(format!("{}{}",config().api_url,mode.path())).json(&serde_json::json!({"subject":subject,"platform":"windows","mac_hash":mac_hash})).send().await.map_err(|_|"Lizenzdienst nicht erreichbar. Erneut versuchen.")?;
            let success=r.status().is_success();
            if r.content_length().is_some_and(|n|n>16384){return Err("Lizenzantwort zu groß.");}
            let value:serde_json::Value=r.json().await.map_err(|_|"Ungültige Lizenzantwort")?;
            if !success {return Err(match value["code"].as_str(){Some("NOT_APPROVED")=>"Noch keine Freischaltung vorhanden.",Some("TRIAL_NOT_STARTED")=>"Testversion noch nicht gestartet.",_=>"Lizenzdienst derzeit nicht verfügbar."});}
            value["license"].as_str().map(str::to_owned).ok_or("Ungültige Lizenzantwort")
        })?;
            let c = verify(&token, &subject)?;
            if !mode.accepts(&c) {
                return Err("Noch keine Freischaltung vorhanden.".into());
            }
            Ok((token, c))
        })();
        let mut s = state().lock().unwrap();
        s.busy = false;
        match result {
            Ok((token, c)) => {
                s.saved.high = s.now().max(c.issued_at);
                s.origin = s.saved.high;
                s.tick = Instant::now();
                s.saved.last_check = s.saved.high;
                s.saved.token = token;
                match s.save() {
                    Ok(()) => s.claims = Some(c),
                    Err(e) => s.error = e,
                }
            }
            Err(e) => s.error = e,
        }
        ctx.request_repaint();
    });
}
pub fn settings(ui: &mut Ui) {
    license_controls(ui);
}

pub fn trial_badge(ui: &mut Ui, compact: bool) {
    if !enabled() {
        return;
    }
    let s = state().lock().unwrap();
    if let Some(c) = &s.claims {
        if c.kind == "trial" {
            let days = ((c.expires_at.unwrap_or(0) - s.now()).max(0) + 86399) / 86400;
            let full = format!(
                "Testversion · noch {days} {}",
                if days == 1 { "Tag" } else { "Tage" }
            );
            ui.label(
                egui::RichText::new(if compact {
                    format!("{days} T")
                } else {
                    full.clone()
                })
                .size(11.0)
                .color(crate::theme::accent(ui)),
            )
            .on_hover_text(full);
        }
    }
}

fn license_controls(ui: &mut Ui) {
    let (text, code, macs, error, busy, started, permanent) = {
        let s = state().lock().unwrap();
        (
            s.text(),
            s.code.clone(),
            s.macs.clone(),
            s.error.clone(),
            s.busy,
            s.claims.is_some(),
            s.claims.as_ref().is_some_and(|c| c.kind == "permanent"),
        )
    };
    ui.label(
        egui::RichText::new(text)
            .color(crate::theme::accent(ui))
            .strong(),
    );
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.label("Geräte-ID");
        ui.label(egui::RichText::new(&code).monospace());
        let copy = ui.small_button("ID kopieren");
        crate::ui::settings::focus_option(ui, &copy);
        if copy.clicked() {
            ui.output_mut(|o| o.copied_text = code.clone());
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("MAC-Adresse");
        if let Some(mac) = macs.first() {
            ui.label(egui::RichText::new(mac).monospace());
            if ui.small_button("MAC kopieren").clicked() {
                ui.output_mut(|o| o.copied_text = mac.clone());
            }
        } else {
            ui.label("Nicht verfügbar – bitte Geräte-ID verwenden");
        }
    });
    ui.add_space(12.0);
    if !error.is_empty() {
        ui.colored_label(crate::theme::DANGER, error);
    }
    if !enabled() {
        ui.label("Lizenzdienst wird eingerichtet. Noch keine Testzeit gestartet.");
    }
    if ui
        .add_enabled(
            enabled() && !busy,
            egui::Button::new(
                egui::RichText::new(if busy {
                    "Wird abgerufen …"
                } else {
                    if permanent {
                        "Lizenz wiederherstellen"
                    } else if started {
                        "Freischaltung abrufen"
                    } else {
                        "7 Tage kostenlos testen"
                    }
                })
                .color(egui::Color32::WHITE),
            )
            .min_size(egui::vec2(0.0, 42.0))
            .fill(crate::theme::accent_fill(ui)),
        )
        .clicked()
    {
        request(
            ui.ctx(),
            if started {
                RequestMode::Activation
            } else {
                RequestMode::Trial
            },
        );
    }
    if !started
        && ui
            .add_enabled(
                !busy && enabled(),
                egui::Button::new("Bereits freigeschaltet? Lizenz abrufen"),
            )
            .clicked()
    {
        refresh(ui.ctx());
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Nach der Freigabe einmal abrufen. Danach Unlimited – ohne automatische Lizenz-Serverabfragen.").small().color(crate::theme::muted(ui)));
    ui.add_space(12.0);
}

/// Runs before login, navigation or player rendering, including Windows TV mode.
pub fn blocked_screen(ctx: &Context) {
    let expired = state()
        .lock()
        .unwrap()
        .claims
        .as_ref()
        .is_some_and(|c| c.kind == "trial");
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let available = ui.available_size();
            let padding = if available.x < 480.0 { 16.0 } else { 32.0 };
            let content_width = (available.x - padding * 2.0 - 32.0).clamp(120.0, 540.0);
            ui.allocate_ui_with_layout(available, egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(((available.y - 510.0) * 0.5).clamp(12.0, 180.0));
                egui::Frame::none()
                    .fill(ui.visuals().window_fill)
                    .stroke(egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color))
                    .rounding(24.0)
                    .inner_margin(egui::Margin::same(padding))
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.horizontal(|ui| {
                                crate::ui::brand_mark(ui, 48.0);
                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Dado Player").size(23.0).strong());
                                    ui.label(egui::RichText::new("WINDOWS · LIZENZ & GERÄT").size(11.0).color(crate::theme::muted(ui)));
                                });
                            });
                            ui.add_space(22.0);
                            ui.label(egui::RichText::new(if expired { "Ihre Testversion ist abgelaufen" } else { "Dein Player. 7 Tage kostenlos." }).size(27.0).strong());
                            ui.add_space(8.0);
                            ui.label(if expired { "Deine Testzeit ist vorbei. Lass dein Gerät manuell freischalten und rufe die Freigabe hier ab." } else { "Starte deine Testversion einmal online. Danach kannst du Dado Player 7 Tage nutzen – auch offline." });
                            ui.add_space(20.0);
                            ui.separator();
                            ui.add_space(12.0);
                            license_controls(ui);
                            ui.label(egui::RichText::new("Für die manuelle Freigabe genügt deine Geräte-ID oder die angezeigte MAC-Adresse. Profile und Favoriten bleiben erhalten.").small().color(crate::theme::muted(ui)));
                        });
                    });
                ui.add_space(16.0);
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;
    fn certificate(subject: &str, kind: &str, end: Option<i64>) -> (String, HashMap<String, Jwk>) {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &rng,
        )
        .unwrap();
        let pair = ring::signature::EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();
        let point = pair.public_key().as_ref();
        let mut keys = HashMap::new();
        keys.insert(
            "test".into(),
            Jwk {
                x: URL_SAFE_NO_PAD.encode(&point[1..33]),
                y: URL_SAFE_NO_PAD.encode(&point[33..]),
            },
        );
        let header =
            URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256","typ":"DADO-LICENSE","kid":"test"}"#);
        let claims = serde_json::json!({"v":1,"product":"dado-player","subject":subject,"platform":"windows","kind":kind,"issued_at":1000,"started_at":1000,"expires_at":end,"license_id":"test-only"});
        let input = format!(
            "{header}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        );
        let signature = pair.sign(&rng, input.as_bytes()).unwrap();
        (
            format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref())),
            keys,
        )
    }
    #[test]
    fn signed_license_is_device_bound_and_cannot_be_edited() {
        let subject = "a".repeat(64);
        let (token, keys) = certificate(&subject, "permanent", None);
        assert_eq!(
            verify_with_keys(&token, &subject, &keys).unwrap().kind,
            "permanent"
        );
        assert!(verify_with_keys(&token, &"b".repeat(64), &keys).is_err());
        let mut parts = token.split('.').map(str::to_owned).collect::<Vec<_>>();
        parts[1].push('A');
        assert!(verify_with_keys(&parts.join("."), &subject, &keys).is_err());
        assert!(verify_with_keys(&token, &subject, &HashMap::new()).is_err());
    }
    #[test]
    fn only_exact_seven_day_trial_or_unlimited_license_is_accepted() {
        let subject = "a".repeat(64);
        for (kind, end, valid) in [
            ("trial", Some(605800), true),
            ("trial", Some(605801), false),
            ("permanent", Some(605800), false),
            ("anything", None, false),
        ] {
            let (token, keys) = certificate(&subject, kind, end);
            assert_eq!(verify_with_keys(&token, &subject, &keys).is_ok(), valid);
        }
    }
    #[test]
    fn trial_blocks_at_exact_expiry_and_permanent_license_has_no_deadline() {
        let subject = "a".repeat(64);
        let (token, keys) = certificate(&subject, "trial", Some(605800));
        let trial = verify_with_keys(&token, &subject, &keys).unwrap();
        assert!(!claims_allow_access(None, 1000));
        assert!(claims_allow_access(Some(&trial), 605799));
        assert!(!claims_allow_access(Some(&trial), 605800));
        assert!(!claims_allow_access(Some(&trial), 999999));
        let (token, keys) = certificate(&subject, "permanent", None);
        let paid = verify_with_keys(&token, &subject, &keys).unwrap();
        assert!(claims_allow_access(Some(&paid), i64::MAX));
    }

    #[test]
    fn activation_cannot_accept_a_valid_signed_trial() {
        let subject = "a".repeat(64);
        let (token, keys) = certificate(&subject, "trial", Some(605800));
        let trial = verify_with_keys(&token, &subject, &keys).unwrap();
        assert!(!RequestMode::Activation.accepts(&trial));
        assert!(RequestMode::Trial.accepts(&trial));
        assert!(RequestMode::Status.accepts(&trial));
        let (token, keys) = certificate(&subject, "permanent", None);
        assert!(
            RequestMode::Activation.accepts(&verify_with_keys(&token, &subject, &keys).unwrap())
        );
        assert_ne!(RequestMode::Trial.path(), RequestMode::Activation.path());
        assert_ne!(RequestMode::Trial.path(), RequestMode::Status.path());
    }
}
