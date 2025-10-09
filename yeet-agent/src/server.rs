use api::httpsig::ReqwestSig;
use httpsig_hyper::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::sync::LazyLock;
use url::Url;

use reqwest::{Client, IntoUrl, RequestBuilder, Response};

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

fn sig_param<K: SigningKey + Sync>(key: &K) -> anyhow::Result<HttpSignatureParams> {
    let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS)?;
    signature_params.set_key_info(key);
    Ok(signature_params)
}

trait ErrorForJson {
    async fn error_for_json<T: DeserializeOwned>(self) -> anyhow::Result<T>;
}

impl ErrorForJson for Response {
    async fn error_for_json<T: DeserializeOwned>(self) -> anyhow::Result<T> {
        if self.status().is_success() {
            Ok(self.json::<T>().await?)
        } else {
            Err(anyhow::anyhow!("{}: {}", self.status(), self.text().await?))
        }
    }
}
