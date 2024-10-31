use crate::error::YeetError;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use uuid::Uuid;
use yeet_api::Capability;
use yeet_server::{AppState, Jti};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    #[serde(with = "chrono::serde::ts_seconds")]
    exp: DateTime<Utc>,
    jti: Uuid,
    cap: Vec<Capability>,
}

impl Claims {
    pub fn require(&self, cap: Capability) -> Result<(), YeetError> {
        if self.cap.contains(&cap) {
            Ok(())
        } else {
            Err(YeetError::MissingCapability(cap))
        }
    }
    pub fn encode(&self, secret: &[u8]) -> Result<String, YeetError> {
        Ok(encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret),
        )?)
    }
    pub fn decode(token: &str, secret: &[u8]) -> jsonwebtoken::errors::Result<Self> {
        let mut val = Validation::default();
        val.set_required_spec_claims(&["exp", "jti"]);
        Ok(jsonwebtoken::decode::<Self>(token, &DecodingKey::from_secret(secret), &val)?.claims)
    }
    #[inline]
    #[must_use]
    pub fn new(cap: Vec<Capability>, duration: Duration) -> Self {
        Self {
            cap,
            exp: Utc::now().add(duration),
            jti: Uuid::now_v7(),
        }
    }

    pub fn rotate(self, state: &mut AppState, hostname: String) -> Result<String, YeetError> {
        // Check if the token has the correct audience
        if !self.cap.contains(&Capability::SystemCheck {
            hostname: hostname.clone(),
        }) {
            return Err(YeetError::InvalidAudience);
        }
        // Check if JTI is blocked
        if state.jti_blacklist.contains(&self.jti()) {
            return Err(YeetError::BlockedJTI);
        }

        let host = state
            .hosts
            .get_mut(&hostname)
            .ok_or(YeetError::HostNotFound(hostname.clone()))?;

        // JWT leaked and either the malicious actor or the agent tried to authenticate with an old JTI
        if host.jti != Jti::Jti(self.jti()) {
            host.jti = Jti::Blocked;
            return Err(YeetError::BlockedJTI);
        }

        let next = Claims::new(
            vec![Capability::SystemCheck { hostname }],
            Duration::days(30),
        );
        host.jti = Jti::Jti(next.jti());
        next.encode(&state.jwt_secret)
    }

    #[must_use]
    pub fn jti(&self) -> Uuid {
        self.jti
    }

    #[must_use]
    pub fn cap(&self) -> &Vec<Capability> {
        &self.cap
    }
}
