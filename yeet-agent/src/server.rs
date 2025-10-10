use api::httpsig::ReqwestSig as _;
use http::StatusCode;
use httpsig_hyper::prelude::*;
use serde::de::DeserializeOwned;
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

pub async fn status<K: SigningKey + Sync>(url: Url, key: K) -> anyhow::Result<Vec<api::Host>> {
    Client::new()
        .get(url.join("/status")?)
        .sign(&sig_param(&key)?, &key)
        .await?
        .send()
        .await?
        .error_for_json::<Vec<api::Host>>()
        .await
}

pub async fn register<K: SigningKey + Sync>(
    url: Url,
    key: K,
    register_host: api::RegisterHost,
) -> anyhow::Result<StatusCode> {
    Client::new()
        .post(url.join("/system/register")?)
        .json(&register_host)
        .sign(&sig_param(&key)?, &key)
        .await?
        .send()
        .await?
        .error_for_code()
        .await
}

pub async fn update<K: SigningKey + Sync>(
    url: Url,
    key: K,
    host_update_request: api::HostUpdateRequest,
) -> anyhow::Result<StatusCode> {
    Client::new()
        .post(url.join("/system/update")?)
        .json(&host_update_request)
        .sign(&sig_param(&key)?, &key)
        .await?
        .send()
        .await?
        .error_for_code()
        .await
}

fn sig_param<K: SigningKey + Sync>(key: &K) -> anyhow::Result<HttpSignatureParams> {
    let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS)?;
    signature_params.set_key_info(key);
    Ok(signature_params)
}

pub trait ErrorForJson {
    async fn error_for_json<T: DeserializeOwned>(self) -> anyhow::Result<T>;
    async fn error_for_code(self) -> anyhow::Result<StatusCode>;
}

impl ErrorForJson for Response {
    async fn error_for_json<T: DeserializeOwned>(self) -> anyhow::Result<T> {
        if self.status().is_success() {
            Ok(self.json::<T>().await?)
        } else {
            Err(anyhow::anyhow!("{}: {}", self.status(), self.text().await?))
        }
    }

    async fn error_for_code(self) -> anyhow::Result<StatusCode> {
        if self.status().is_success() {
            Ok(self.status())
        } else {
            Err(anyhow::anyhow!("{}: {}", self.status(), self.text().await?))
        }
    }
}
