/// The goal is to no longer require the pub key at registration of the host.
/// Rather any unauthenticated client can try an `verification_attempt` and supply his public key.
/// This then generates a six digit number which the admin has to retrieve from the client (not the server!)
/// This ensure that the identity of the host is verified.
/// However the identity model is now flipped. Instead of just identifying the host based on
/// the public key it is now tied to an arbitrary name.
/// We could make it so that the client saves its hostname either by looking at its hostname
/// or via config. An other solution would be that when you run `yeet approve` and input the clients
/// one time pin that you also have to input the hostname that it should be associated with.
///
use axum::http::StatusCode;

use crate::httpsig::HttpSig;

pub async fn is_host_verified(HttpSig(_http_key): HttpSig) -> StatusCode {
    StatusCode::OK
}
