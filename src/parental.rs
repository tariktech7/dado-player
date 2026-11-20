//! Device-local parental controls. No plaintext PIN or permanent playback grants.
use ring::pbkdf2;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    num::NonZeroU32,
    time::{Duration, Instant},
};

const ITERATIONS: u32 = 100_000;
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Policy {
    pub salt: Vec<u8>,
    pub hash: Vec<u8>,
    pub blocked: HashSet<String>,
}
impl Policy {
    pub fn enabled(&self) -> bool {
        !self.hash.is_empty()
    }
    pub fn set_pin(&mut self, pin: &str) -> anyhow::Result<()> {
        anyhow::ensure!(valid_pin(pin), "Bitte genau vier Ziffern eingeben.");
        let mut salt = vec![0; 16];
        getrandom::getrandom(&mut salt)
            .map_err(|_| anyhow::anyhow!("Zufallsquelle nicht verfügbar"))?;
        let mut hash = vec![0; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(ITERATIONS).unwrap(),
            &salt,
            pin.as_bytes(),
            &mut hash,
        );
        self.salt = salt;
        self.hash = hash;
        Ok(())
    }
    pub fn verify(&self, pin: &str) -> bool {
        self.enabled()
            && valid_pin(pin)
            && pbkdf2::verify(
                pbkdf2::PBKDF2_HMAC_SHA256,
                NonZeroU32::new(ITERATIONS).unwrap(),
                &self.salt,
                pin.as_bytes(),
                &self.hash,
            )
            .is_ok()
    }
    pub fn locked(&self, profile: &str, key: &str) -> bool {
        self.enabled()
            && (self.blocked.contains("*") || self.blocked.contains(&scoped_key(profile, key)))
    }
}
pub fn valid_pin(pin: &str) -> bool {
    pin.len() == 4 && pin.bytes().all(|c| c.is_ascii_digit())
}
pub fn content_key(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("ep:") {
        return format!("series:{}", rest.split(':').next().unwrap_or(rest));
    }
    key.to_owned()
}
pub fn scoped_key(profile: &str, key: &str) -> String {
    format!("{}:{}:{}", profile.len(), profile, content_key(key))
}

#[derive(Default)]
pub struct Controls {
    pub policy: Policy,
    pub open: bool,
    pub authenticated: bool,
    pub pending: Option<(crate::types::PlayItem, bool)>,
    pub grant: Option<String>,
    pub pin: String,
    pub first_pin: String,
    pub changing: bool,
    pub query: String,
    pub kind: usize,
    pub page: usize,
    pub catalog_cache: Option<(String, Vec<(String, String)>)>,
    pub error: String,
    attempts: u8,
    cooldown: Option<Instant>,
}
impl Controls {
    pub fn load() -> Self {
        Self {
            policy: crate::store::load_parental(),
            ..Self::default()
        }
    }
    pub fn close(&mut self) {
        self.catalog_cache = None;
        self.page = 0;
        self.open = false;
        self.authenticated = false;
        self.pending = None;
        self.grant = None;
        self.pin.clear();
        self.first_pin.clear();
        self.changing = false;
        self.error.clear();
    }
    pub fn check(&mut self) -> bool {
        if self.cooldown.is_some_and(|t| t > Instant::now()) {
            self.error = "Zu viele Versuche. Bitte 30 Sekunden warten.".into();
            self.pin.clear();
            return false;
        }
        let ok = self.policy.verify(&self.pin);
        self.pin.clear();
        if ok {
            self.attempts = 0;
            self.cooldown = None;
            self.error.clear();
        } else {
            self.attempts += 1;
            self.error = "Falsche PIN.".into();
            if self.attempts >= 5 {
                self.attempts = 0;
                self.cooldown = Some(Instant::now() + Duration::from_secs(30));
                self.error = "Zu viele Versuche. Bitte 30 Sekunden warten.".into();
            }
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pin_and_scope() {
        let mut p = Policy::default();
        assert!(!p.enabled());
        assert!(p.set_pin("123").is_err());
        assert!(p.set_pin("１２３４").is_err());
        p.set_pin("0123").unwrap();
        assert!(p.verify("0123"));
        assert!(!p.verify("1234"));
        p.blocked.insert(scoped_key("a", "series:8"));
        assert!(p.locked("a", "ep:8:9"));
        assert!(p.locked("a", "ep:8:10"));
        assert!(!p.locked("b", "ep:8:9"));
        assert!(!p.locked("a", "vod:8"));
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("0123"));
        assert!(serde_json::from_str::<Policy>(&json)
            .unwrap()
            .verify("0123"));
    }
    #[test]
    fn attempts_survive_cancel() {
        let mut c = Controls::default();
        c.policy.set_pin("1234").unwrap();
        for _ in 0..5 {
            c.pin = "0000".into();
            assert!(!c.check());
            c.close();
        }
        c.pin = "1234".into();
        assert!(!c.check());
    }
}
