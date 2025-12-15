use api::httpsig::ReqwestSig as _;
use http::StatusCode;
use httpsig_hyper::prelude::*;
use rootcause::{
    IntoReport, Report, compat::boxed_error::IntoBoxedError, prelude::ResultExt, report,
};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::sync::LazyLock;
use url::Url;

use reqwest::{Client, Response};

#[expect(clippy::expect_used, reason = "Is there another way?")]
static COMPONENTS: LazyLock<Vec<message_component::HttpMessageComponentId>> = LazyLock::new(|| {
    ["date", "@path", "@method", "content-digest"]
        .iter()
        .map(|component| message_component::HttpMessageComponentId::try_from(*component))
        .collect::<Result<Vec<_>, _>>()
        .expect("Could not create HTTP Signature components")
});

pub async fn status<K: SigningKey + Sync>(url: &Url, key: &K) -> Result<Vec<api::Host>, Report> {
    Client::new()
        .get(url.join("/status")?)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_json::<Vec<api::Host>>()
        .await
}

pub async fn register<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
    register_host: &api::RegisterHost,
) -> Result<StatusCode, Report> {
    Client::new()
        .post(url.join("/system/register")?)
        .json(register_host)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_code()
        .await
}

pub async fn update<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
    host_update_request: &api::HostUpdateRequest,
) -> Result<StatusCode, Report> {
    Client::new()
        .post(url.join("/system/update")?)
        .json(host_update_request)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_code()
        .await
}

pub async fn is_host_verified<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
) -> Result<StatusCode, Report> {
    Ok(Client::new()
        .get(url.join("/system/verify")?)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .status())
}

pub async fn add_verification_attempt(
    url: &Url,
    attempt: &api::VerificationAttempt,
) -> Result<u32, Report> {
    Client::new()
        .post(url.join("/system/verify")?)
        .json(attempt)
        .send()
        .await?
        .error_for_json()
        .await
}

pub async fn verify_attempt<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
    acceptance: &api::VerificationAcceptance,
) -> Result<api::VerificationArtifacts, Report> {
    Client::new()
        .post(url.join("/system/verify/accept")?)
        .json(acceptance)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_json()
        .await
}

pub async fn system_check<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
    version: &api::VersionRequest,
) -> Result<api::AgentAction, Report> {
    Client::new()
        .post(url.join("/system/check")?)
        .json(version)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_json()
        .await
}

pub async fn add_key<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
    add_key: &api::AddKey,
) -> Result<StatusCode, Report> {
    Client::new()
        .post(url.join("/key/add")?)
        .json(add_key)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_code()
        .await
}

pub async fn remove_key<K: SigningKey + Sync>(
    url: &Url,
    key: &K,
    remove_key: &ed25519_dalek::VerifyingKey,
) -> Result<StatusCode, Report> {
    Client::new()
        .post(url.join("/key/remove")?)
        .json(remove_key)
        .sign(&sig_param(key)?, key)
        .await?
        .send()
        .await?
        .error_for_code()
        .await
}

fn sig_param<K: SigningKey + Sync>(key: &K) -> Result<HttpSignatureParams, Report> {
    let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS)?;
    signature_params.set_key_info(key);
    Ok(signature_params)
}

pub trait ErrorForJson {
    async fn error_for_json<T: DeserializeOwned>(self) -> Result<T, Report>;
    async fn error_for_code(self) -> Result<StatusCode, Report>;
}

impl ErrorForJson for Response {
    async fn error_for_json<T: DeserializeOwned>(self) -> Result<T, Report> {
        if self.status().is_success() {
            Ok(self.json::<T>().await?)
        } else {
            Err(report!("{}: {}", self.status(), self.text().await?))
        }
    }

    async fn error_for_code(self) -> Result<StatusCode, Report> {
        if self.status().is_success() {
            Ok(self.status())
        } else {
            Err(report!("{}: {}", self.status(), self.text().await?))
        }
    }
}
