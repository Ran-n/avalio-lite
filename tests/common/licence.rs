// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.164721
//+ Revised: 	2026/11/11 08:54:25.176949
// -----------------------------------------------------------------------------

//! A Clavren licence server in memory, for the tests and the desktop test driver: it answers the
//! public API as Clavren does, with tokens signed by a test key, and can be told to refuse, fail or
//! go offline. Ported from the Clavren repository's own fake (crates/clavren-client/tests).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use avalio_lite::licence::PRODUCT;
use chrono::{DateTime, Utc};
use clavren_client::clock::{Clock, ManualClock};
use clavren_client::store::{MemoryStore, SecretStore, StoreError};
use clavren_client::transport::{Answer, Transport};
use clavren_client::{Client, Config};
use clavren_protocol::check::{check_message, device_hash};
use clavren_protocol::encoding;
use clavren_protocol::token::{self, Claims, TrustedKeys};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde_json::{Value, json};
use uuid::Uuid;

/// A well-formed key; the fake accepts any.
pub const KEY: &str = "BRN-EYZE-C1AE-CR2G-MTJK";

/// What the server does with the next requests.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    /// Answers every request with this error code and HTTP status, as Clavren refuses:
    /// `invalid_key` 404, `already_activated` 409, `licence_revoked` 403, `licence_expired` 403,
    /// `product_unavailable` 403, `rate_limited` 429.
    Refuse(&'static str, u16),
    /// No answer at all (no network).
    Offline,
}

pub struct FakeClavren {
    pub clock: Arc<ManualClock>,
    pub mode: Mutex<Mode>,
    pub calls: AtomicUsize,
    pub licence_expires_at: Mutex<Option<DateTime<Utc>>>,
    signing: SigningKey,
    device: Mutex<Option<[u8; 32]>>,
    nonces: Mutex<Vec<(Uuid, [u8; 32])>>,
}

impl FakeClavren {
    pub fn new(clock: Arc<ManualClock>) -> Arc<Self> {
        Arc::new(Self {
            clock,
            mode: Mutex::new(Mode::Normal),
            calls: AtomicUsize::new(0),
            licence_expires_at: Mutex::new(None),
            signing: SigningKey::from_bytes(&[42; 32]),
            device: Mutex::new(None),
            nonces: Mutex::new(Vec::new()),
        })
    }

    pub fn trusted_keys() -> TrustedKeys {
        TrustedKeys::from([("v1".to_string(), SigningKey::from_bytes(&[42; 32]).verifying_key())])
    }

    pub fn set_mode(&self, mode: Mode) {
        *self.mode.lock().unwrap() = mode;
    }

    fn token(&self, device: &[u8; 32]) -> Value {
        let now = self.clock.now();
        let licence_end = *self.licence_expires_at.lock().unwrap();
        let end = licence_end.map_or(now + chrono::Duration::days(30), |at| at.min(now + chrono::Duration::days(30)));
        let claims = Claims {
            iss: "https://licences.example.com".into(),
            aud: PRODUCT.into(),
            sub: Uuid::from_u128(1),
            act: Uuid::from_u128(2),
            dev: device_hash(device),
            lic_exp: licence_end.map(|at| at.timestamp()),
            iat: now.timestamp(),
            exp: end.timestamp(),
            jti: Uuid::from_u128(3),
        };
        json!({
            "activation": Uuid::from_u128(2),
            "token": token::sign(&claims, "v1", &self.signing),
            "token_expires_at": end,
            "licence_expires_at": licence_end,
            "server_time": now,
        })
    }
}

fn answer(status: u16, body: Value) -> Answer {
    Answer { status, body, retry_after: if status == 429 { Some(120) } else { None } }
}

impl Transport for FakeClavren {
    fn post(&self, path: &str, body: &Value) -> Result<Answer, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let now = self.clock.now();
        match self.mode.lock().unwrap().clone() {
            Mode::Offline => return Err("connection refused".into()),
            Mode::Refuse(code, status) => {
                return Ok(answer(status, json!({ "error": code, "message": "no", "server_time": now })));
            }
            Mode::Normal => {}
        }
        if path == "/api/v1/activations" {
            assert_eq!(body["product"], PRODUCT);
            let device = encoding::decode_array::<32>(body["device_public_key"].as_str().unwrap()).unwrap();
            *self.device.lock().unwrap() = Some(device);
            return Ok(answer(201, self.token(&device)));
        }
        if path.ends_with("/challenges") {
            let (id, nonce) = (Uuid::from_u128(1000 + self.calls.load(Ordering::SeqCst) as u128), [7u8; 32]);
            self.nonces.lock().unwrap().push((id, nonce));
            return Ok(answer(
                200,
                json!({ "challenge": id, "nonce": encoding::encode(&nonce), "expires_at": now, "server_time": now }),
            ));
        }
        // a check: the signature must come from the activated device
        let challenge = Uuid::parse_str(body["challenge"].as_str().unwrap()).unwrap();
        let nonce =
            self.nonces.lock().unwrap().iter().find(|(id, _)| *id == challenge).map(|(_, nonce)| *nonce).unwrap();
        let device = self.device.lock().unwrap().unwrap();
        let signature = encoding::decode_array::<64>(body["signature"].as_str().unwrap()).unwrap();
        let message = check_message(Uuid::from_u128(2), challenge, &nonce);
        VerifyingKey::from_bytes(&device)
            .unwrap()
            .verify_strict(message.as_bytes(), &Signature::from_bytes(&signature))
            .unwrap();
        Ok(answer(200, self.token(&device)))
    }
}

/// A store that can't save: the device key of an activation can't be kept.
pub struct FailingStore;

impl SecretStore for FailingStore {
    fn load(&self) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(None)
    }

    fn save(&self, _plain: &[u8]) -> Result<(), StoreError> {
        Err(StoreError::Io { path: "licence.dat".into(), source: std::io::Error::other("disk full") })
    }
}

/// One computer's licence: the fake server, its clock and the saved state.
pub struct FakeLicence {
    pub server: Arc<FakeClavren>,
    pub clock: Arc<ManualClock>,
    pub store: Arc<MemoryStore>,
}

impl FakeLicence {
    /// A computer that was never activated, its clock at a fixed time.
    pub fn new() -> Self {
        Self::at(DateTime::from_timestamp(1_800_000_000, 0).unwrap())
    }

    /// A computer that was never activated, its clock at `start`.
    pub fn at(start: DateTime<Utc>) -> Self {
        let clock = Arc::new(ManualClock::new(start));
        Self { server: FakeClavren::new(clock.clone()), clock, store: Arc::new(MemoryStore::default()) }
    }

    /// A computer whose key was activated.
    pub fn activated() -> Self {
        Self::new().activate()
    }

    /// Activates this computer's key (the server answering normally).
    pub fn activate(self) -> Self {
        assert!(self.client().activate(KEY).unwrap().is_licensed());
        self
    }

    /// Moves this computer's clock on, e.g. past the 30 days a token lasts offline.
    pub fn advance(&self, by: chrono::TimeDelta) {
        self.clock.set(self.clock.now() + by);
    }

    /// From now on, the licence ends at `at` (`None`: permanent).
    pub fn ends_at(&self, at: Option<DateTime<Utc>>) {
        *self.server.licence_expires_at.lock().unwrap() = at;
    }

    /// The app starting: a client on this computer's saved state.
    pub fn client(&self) -> Arc<Client> {
        self.client_with(Config::new(PRODUCT, FakeClavren::trusted_keys(), "1.0.0"))
    }

    /// Like `client`, checking online once `interval` has passed instead of daily.
    pub fn client_checking_every(&self, interval: std::time::Duration) -> Arc<Client> {
        let mut config = Config::new(PRODUCT, FakeClavren::trusted_keys(), "1.0.0");
        config.check_interval = interval;
        self.client_with(config)
    }

    fn client_with(&self, config: Config) -> Arc<Client> {
        Arc::new(Client::new(
            config,
            Box::new(self.store.clone()),
            Box::new(self.server.clone()),
            Box::new(self.clock.clone()),
        ))
    }

    /// A client that can't save anything.
    pub fn failing_client(&self) -> Arc<Client> {
        let config = Config::new(PRODUCT, FakeClavren::trusted_keys(), "1.0.0");
        Arc::new(Client::new(
            config,
            Box::new(FailingStore),
            Box::new(self.server.clone()),
            Box::new(self.clock.clone()),
        ))
    }
}
