//! Opt-in cloud profile inbox; receipt follows successful durable profile save.
use crate::{
    store,
    types::{Profile, ProfileKind},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    token: String,
    device_id: String,
    address: String,
    owner: Option<String>,
    pairing_code: Option<String>,
    pairing_expires: u64,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    device_id: String,
    address: String,
    owner: Option<String>,
    pairing_code: Option<String>,
    pairing_expires: Option<u64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transfer {
    name: String,
    kind: String,
    #[serde(default)]
    host: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    m3u_url: String,
}
#[derive(Deserialize)]
struct Message {
    id: String,
    profile: Transfer,
}
#[derive(Deserialize)]
struct Inbox {
    #[serde(flatten)]
    device: Device,
    messages: Vec<Message>,
}
enum Event {
    Device(Device),
    Inbox(Inbox),
    Ack(Result<usize, String>),
    Error(String),
}
pub struct DeviceClient {
    saved: Saved,
    busy: bool,
    message: String,
    confirm_reset: bool,
    expanded: bool,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}
impl Default for DeviceClient {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            saved: serde_json::from_value(store::load_connect_state()).unwrap_or_default(),
            busy: false,
            message: String::new(),
            confirm_reset: false,
            expanded: false,
            tx,
            rx,
        }
    }
}
fn config() -> &'static serde_json::Value {
    static CONFIG: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| {
        serde_json::from_str(include_str!("../connect/public-config.json")).expect("Connect config")
    })
}
fn utc() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn valid_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
    })
}
fn convert(m: &Message) -> Result<Profile, String> {
    let p = &m.profile;
    if m.id.len() != 36
        || !m.id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        || p.name.trim().is_empty()
        || p.name.len() > 320
    {
        return Err("Ungültiges Cloud-Profil.".into());
    }
    let kind = match p.kind.as_str() {
        "Xtream" => ProfileKind::Xtream,
        "M3u" => ProfileKind::M3u,
        _ => return Err("Nicht unterstützte Profilart.".into()),
    };
    if match kind {
        ProfileKind::Xtream => {
            !valid_url(&p.host) || p.username.is_empty() || p.password.is_empty()
        }
        ProfileKind::M3u => !valid_url(&p.m3u_url),
    } {
        return Err("Ungültige Anbieteradresse oder fehlende Zugangsdaten.".into());
    }
    let mut out = Profile::new(&p.name, kind);
    out.id = format!("cloud-{}", m.id);
    out.host = p.host.clone();
    out.username = p.username.clone();
    out.password = p.password.clone();
    out.m3u_url = p.m3u_url.clone();
    Ok(out)
}
async fn post<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    token: &str,
    path: &str,
    input: serde_json::Value,
) -> Result<T, String> {
    let base = config()["apiUrl"].as_str().unwrap_or_default().to_owned();
    let mut response = client
        .post(format!("{base}/v1/device/{path}"))
        .bearer_auth(token)
        .json(&input)
        .send()
        .await
        .map_err(|_| "Cloud-Dienst nicht erreichbar. Verbindung prüfen.".to_owned())?;
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cloud-Antwort unvollständig.".to_owned())?
    {
        if bytes.len() + chunk.len() > 512 * 1024 {
            return Err("Cloud-Antwort zu groß.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        return Err(serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .filter(|v| v.len() < 500)
            .unwrap_or_else(|| "Cloud-Anfrage fehlgeschlagen.".into()));
    }
    serde_json::from_slice(&bytes).map_err(|_| "Cloud-Antwort nicht lesbar.".into())
}
fn platform() -> &'static str {
    if cfg!(target_os = "ios") {
        "ios"
    } else if cfg!(target_os = "android") {
        if crate::platform::tv_device() {
            "android-tv"
        } else {
            "android"
        }
    } else {
        "windows"
    }
}
fn hardware_mac() -> Option<String> {
    #[cfg(target_os = "windows")]
    return crate::platform::windows::identity::mac_addresses()
        .into_iter()
        .next();
    #[cfg(not(target_os = "windows"))]
    None
}
impl DeviceClient {
    pub fn expanded(&self) -> bool {
        self.expanded
    }
    fn persist(&self) -> Result<(), String> {
        store::save_connect_state(&self.saved)
            .map_err(|_| "Geräteverbindung konnte nicht gespeichert werden.".into())
    }
    fn apply(&mut self, d: Device) -> Result<(), String> {
        self.saved.device_id = d.device_id;
        self.saved.address = d.address;
        self.saved.owner = d.owner;
        if self.saved.owner.is_some() {
            self.saved.pairing_code = None;
            self.saved.pairing_expires = 0;
        } else if let Some(code) = d.pairing_code {
            self.saved.pairing_code = Some(code);
            self.saved.pairing_expires = d.pairing_expires.unwrap_or(0);
        }
        self.persist()
    }
    fn start(&mut self, ctx: &egui::Context, rt: &tokio::runtime::Handle, pair: bool, reset: bool) {
        if self.busy {
            return;
        }
        if self.saved.token.is_empty() {
            let mut secret = [0u8; 32];
            if getrandom::getrandom(&mut secret).is_err() {
                self.message = "Sichere Gerätekennung konnte nicht erzeugt werden.".into();
                return;
            }
            self.saved.token = URL_SAFE_NO_PAD.encode(secret);
            if let Err(e) = self.persist() {
                self.saved.token.clear();
                self.message = e;
                return;
            }
        }
        self.busy = true;
        self.message = "Cloud-Verbindung wird hergestellt …".into();
        self.confirm_reset = false;
        let token = self.saved.token.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let mac = hardware_mac();
        rt.spawn(async move {
            let result = async {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(25))
                    .build()
                    .map_err(|_| "Cloud-Verbindung nicht verfügbar.".to_owned())?;
                let _: Device = post(
                    &client,
                    &token,
                    "register",
                    serde_json::json!({"platform":platform(),"mac":mac}),
                )
                .await?;
                if pair {
                    Ok(Event::Device(
                        post(
                            &client,
                            &token,
                            "pairing",
                            serde_json::json!({"reset":reset}),
                        )
                        .await?,
                    ))
                } else {
                    Ok(Event::Inbox(
                        post(&client, &token, "pull", serde_json::json!({})).await?,
                    ))
                }
            }
            .await;
            let _ = tx.send(result.unwrap_or_else(Event::Error));
            ctx.request_repaint();
        });
    }
    fn poll(
        &mut self,
        ctx: &egui::Context,
        rt: &tokio::runtime::Handle,
        profiles: &mut Vec<Profile>,
    ) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Device(d) => {
                    self.busy = false;
                    self.message=match self.apply(d){Ok(())=>"Im Portal Adresse und Kopplungscode eintragen. Code ist zehn Minuten gültig.".into(),Err(e)=>e};
                }
                Event::Error(e) => {
                    self.busy = false;
                    self.message = e;
                }
                Event::Ack(result) => {
                    self.busy = false;
                    self.message=match result{Ok(n)=>format!("{n} neue Profile gespeichert. Unter Profile auswählen und anmelden."),Err(e)=>format!("Profile gespeichert. Empfangsbestätigung fehlt: {e} Bitte Aktualisieren erneut drücken.")};
                }
                Event::Inbox(inbox) => {
                    if let Err(e) = self.apply(inbox.device) {
                        self.busy = false;
                        self.message = e;
                        continue;
                    }
                    let mut next = profiles.clone();
                    let mut ids = Vec::new();
                    let mut added = 0;
                    let mut error = None;
                    for m in inbox.messages {
                        match convert(&m) {
                            Ok(p) => {
                                if !next.iter().any(|v| v.id == p.id) {
                                    next.push(p);
                                    added += 1;
                                }
                                ids.push(m.id);
                            }
                            Err(e) => {
                                error = Some(e);
                                break;
                            }
                        }
                    }
                    if let Some(e) = error {
                        self.busy = false;
                        self.message = e;
                        continue;
                    }
                    if ids.is_empty() {
                        self.busy = false;
                        self.message = if self.saved.owner.is_some() {
                            "Keine neuen Profile vorhanden.".into()
                        } else {
                            "Noch nicht gekoppelt. Zuerst Gerät verbinden und im Portal zuordnen."
                                .into()
                        };
                        continue;
                    }
                    if store::save_profiles(&next).is_err() {
                        self.busy = false;
                        self.message="Profile konnten nicht gespeichert werden. Sendungen bleiben in der Cloud abrufbar.".into();
                        continue;
                    }
                    *profiles = next;
                    let tx = self.tx.clone();
                    let token = self.saved.token.clone();
                    let ctx = ctx.clone();
                    rt.spawn(async move {
                        let result = async {
                            let client = reqwest::Client::builder()
                                .timeout(std::time::Duration::from_secs(25))
                                .build()
                                .map_err(|_| "Verbindung fehlgeschlagen.".to_owned())?;
                            let _: serde_json::Value =
                                post(&client, &token, "ack", serde_json::json!({"ids":ids}))
                                    .await?;
                            Ok(added)
                        }
                        .await;
                        let _ = tx.send(Event::Ack(result));
                        ctx.request_repaint();
                    });
                }
            }
        }
    }
}
fn clicked(ui: &mut egui::Ui, label: &str, enabled: bool, tv: bool) -> bool {
    let r = ui.add_enabled(enabled, egui::Button::new(label));
    if !enabled {
        return false;
    }
    if tv {
        crate::ui::mobile::focus::clicked(ui, &r)
    } else {
        r.clicked()
    }
}
pub fn show(
    ui: &mut egui::Ui,
    state: &mut DeviceClient,
    rt: &tokio::runtime::Handle,
    profiles: &mut Vec<Profile>,
    tv: bool,
    compact: bool,
) {
    state.poll(ui.ctx(), rt, profiles);
    let ctx = ui.ctx().clone();
    if compact && !state.expanded {
        if clicked(ui, "Profile vom Handy/PC", true, tv) {
            state.expanded = true;
        }
        return;
    }
    if compact && clicked(ui, "Zurück zur Anmeldung", true, tv) {
        state.expanded = false;
    }
    ui.group(|ui|{
  ui.strong("Profile vom Handy/PC");ui.small("Dado Connect · Konto im Portal anlegen und vom Betreiber freigeben lassen.");ui.hyperlink(config()["portalUrl"].as_str().unwrap_or_default());
  if !state.saved.address.is_empty(){ui.label(format!("Geräteadresse: {}",state.saved.address));ui.small("Private Geräteadresse im MAC-Format; auf iOS/Android keine Hardware-MAC.");}
  if let Some(owner)=&state.saved.owner{ui.label(format!("Verbunden mit: {owner}"));}
  if let Some(code)=&state.saved.pairing_code{if state.saved.pairing_expires>utc(){ui.label(egui::RichText::new(format!("Kopplungscode: {code}")).strong());ui.small("Zehn Minuten gültig. Nur an die Person geben, die Profile senden darf.");}else{ui.small("Kopplungscode abgelaufen. Bitte neu erzeugen.");}}
  ui.horizontal_wrapped(|ui|{if state.saved.owner.is_none()&&clicked(ui,"Gerät verbinden",!state.busy,tv){state.start(&ctx,rt,true,false);}
if clicked(ui,if state.busy{"Wird abgerufen …"}else{"Aktualisieren"},!state.busy,tv){state.start(&ctx,rt,false,false);}
if state.saved.owner.is_some()&&clicked(ui,"Verbindung trennen",!state.busy,tv){state.confirm_reset=true;}});
  if state.confirm_reset{ui.small("Cloud-Zuordnung und ungelesene Sendungen entfernen? Gespeicherte Profile bleiben erhalten.");if clicked(ui,"Trennen bestätigen",!state.busy,tv){state.start(&ctx,rt,true,true);}
if clicked(ui,"Abbrechen",true,tv){state.confirm_reset=false;}}
  if !state.message.is_empty(){ui.label(&state.message);}ui.small("Abruf nur auf Knopfdruck. Keine automatische Anmeldung oder Wiedergabe.");
 });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cloud_panel_fits_a_narrow_phone_without_horizontal_overflow() {
        let mut state = DeviceClient::default();
        state.saved.address = "02:11:22:33:44:55".into();
        state.saved.owner = Some("family-with-a-long-email@example.invalid".into());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = egui::Context::default();
        let mut profiles = Vec::new();
        for frame in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(320.0, 800.0),
                    )),
                    time: Some(f64::from(frame) / 60.0),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let right = ui.max_rect().right();
                        show(ui, &mut state, rt.handle(), &mut profiles, false, false);
                        assert!(
                            ui.min_rect().right() <= right + 1.0,
                            "Cloud panel overflowed a phone viewport"
                        );
                    });
                },
            );
        }
    }
    #[test]
    fn failed_storage_keeps_existing_profiles_and_never_acknowledges_delivery() {
        let mut state = DeviceClient::default();
        let path = store::license_path().with_file_name("profiles.json");
        std::fs::create_dir_all(&path).unwrap();
        state.busy = true;
        state
            .tx
            .send(Event::Inbox(Inbox {
                device: Device {
                    device_id: "fixture".into(),
                    address: "02:11:22:33:44:55".into(),
                    owner: Some("father@example.invalid".into()),
                    ..Default::default()
                },
                messages: vec![Message {
                    id: "11111111-1111-4111-8111-111111111111".into(),
                    profile: Transfer {
                        name: "Remote".into(),
                        kind: "M3u".into(),
                        host: String::new(),
                        username: String::new(),
                        password: String::new(),
                        m3u_url: "https://provider.invalid/list.m3u".into(),
                    },
                }],
            }))
            .unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut profiles = vec![Profile::new("Existing", ProfileKind::M3u)];
        state.poll(&egui::Context::default(), rt.handle(), &mut profiles);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Existing");
        assert!(!state.busy);
        assert!(state.message.contains("nicht gespeichert"));
        std::fs::remove_dir(&path).unwrap();
    }
    #[test]
    fn cloud_profiles_are_stable_and_reject_unsafe_urls() {
        let m = Message {
            id: "11111111-1111-4111-8111-111111111111".into(),
            profile: Transfer {
                name: "Papa".into(),
                kind: "Xtream".into(),
                host: "https://provider.invalid".into(),
                username: "user".into(),
                password: "secret".into(),
                m3u_url: String::new(),
            },
        };
        let p = convert(&m).unwrap();
        assert_eq!(p.id, convert(&m).unwrap().id);
        assert_eq!(p.password, "secret");
        assert!(!valid_url("file:///private"));
        assert!(!valid_url("javascript:alert(1)"));
        assert!(!valid_url("https://user:pass@provider.invalid"));
        assert!(valid_url("http://provider.invalid/list.m3u"));
    }
}
