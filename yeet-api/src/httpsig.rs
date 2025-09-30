use httpsig_hyper::{
    ContentDigest as _, MessageSignatureReq as _, RequestContentDigest as _,
    prelude::{HttpSignatureParams, SigningKey},
};
use reqwest::RequestBuilder;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SignatureError {
    #[error(transparent)]
    HyperDigestError(#[from] httpsig_hyper::HyperDigestError),
    #[error(transparent)]
    HyperSigError(#[from] httpsig_hyper::HyperSigError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

pub trait ReqwestSig {
    fn sign<T: SigningKey + Sync>(
        self,
        signature_params: &HttpSignatureParams,
        signing_key: &T,
    ) -> impl Future<Output = Result<RequestBuilder, SignatureError>> + Send;
}

impl ReqwestSig for RequestBuilder {
    async fn sign<T: SigningKey + Sync>(
        self,
        signature_params: &HttpSignatureParams,
        signing_key: &T,
    ) -> Result<RequestBuilder, SignatureError> {
        let (client, request) = self.build_split();
        let req: http::Request<_> = request?.try_into()?;
        let mut req = req
            .set_content_digest(&httpsig_hyper::ContentDigestType::Sha256)
            .await?;
        req.set_message_signature(signature_params, signing_key, None)
            .await?;
        let (parts, body) = req.into_parts();
        let body: reqwest::Body = body.into_bytes().await?.into();
        let request = http::Request::from_parts(parts, body).try_into()?;
        Ok(RequestBuilder::from_parts(client, request))
    }
}

#[cfg(test)]
mod test_ureq_sign {
    use std::sync::LazyLock;

    use httpsig_hyper::prelude::*;
    use reqwest::Client;

    use crate::httpsig::ReqwestSig;

    static COMPONENTS: LazyLock<Vec<message_component::HttpMessageComponentId>> =
        LazyLock::new(|| {
            ["date", "@target-uri", "@method", "content-digest"]
                .iter()
                .map(|v| message_component::HttpMessageComponentId::try_from(*v))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        });

    const EDDSA_SECRET_KEY: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIDx2kNPzVZ7AmTCEY99KU4gw3DoCc9Unq+YCmVLAychJ
-----END PRIVATE KEY-----
";

    #[tokio::test]
    async fn test_ureq() {
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_pem(EDDSA_SECRET_KEY).unwrap();
        signature_params.set_key_info(&signing_key);

        let req = Client::new()
            .get("https://example.com")
            .body("Hi")
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .build()
            .unwrap();

        assert!(req.headers().contains_key("signature-input"));
        assert!(req.headers().contains_key("signature"));
        assert!(req.headers().contains_key("content-digest"));
    }
}
