use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yeet_api::Capability;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    #[serde(with = "chrono::serde::ts_seconds")]
    exp: DateTime<Utc>,
    jti: Uuid,
    aud: Vec<String>,
}

impl Claims {
    pub fn encode(&self, secret: &[u8]) -> jsonwebtoken::errors::Result<String> {
        encode(&Header::default(), self, &EncodingKey::from_secret(secret))
    }
    pub fn decode(token: &str, aud: &str, secret: &[u8]) -> jsonwebtoken::errors::Result<Self> {
        let mut val = Validation::default();
        val.set_required_spec_claims(&["aud", "exp", "jti"]);
        val.set_audience(&[aud]);
        Ok(jsonwebtoken::decode::<Self>(token, &DecodingKey::from_secret(secret), &val)?.claims)
    }
    #[inline]
    pub fn new(aud: Vec<Capability>, duration: Duration) -> Self {
        let uuid = Uuid::now_v7();
        Self {
            aud: aud
                .into_iter()
                .flat_map(Into::<Vec<String>>::into)
                .collect(),
            #[allow(clippy::arithmetic_side_effects)]
            exp: Utc::now() + duration,
            jti: uuid,
        }
    }
    pub fn jti(&self) -> Uuid {
        self.jti
    }

    pub fn aud(&self) -> Vec<String> {
        self.aud.clone()
    }

    pub fn exp(&self) -> DateTime<Utc> {
        self.exp
    }
}
