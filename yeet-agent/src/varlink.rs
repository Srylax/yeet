use rootcause::{IntoReport, Report};
use serde::{Deserialize, Serialize};
use tokio::fs::remove_file;
use zlink::{Call,
            Connection,
            ReplyError,
            Service,
            connection::Socket,
            service::MethodReply,
            unix};

const SOCKET_PATH: &str = "/run/yeet/agent.varlink";

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "parameters")]
pub enum YeetMethod {
    #[serde(rename = "ch.yeetme.yeet.Status")]
    Status,
}

#[derive(Debug, Serialize)]
pub enum YeetReply {
    DaemonStatus(DaemonStatus),
}

#[derive(Debug, Serialize)]
pub struct DaemonStatus {
    pub up_to_date: UpToDate,
    pub server: String,
    pub mode: YeetDaemonMode,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub enum UpToDate {
    Yes,
    No,
    Detached,
}

#[derive(Debug, Serialize)]
pub enum YeetDaemonMode {
    Provisioned,
    Detached,
    Unknown,
}

#[derive(Debug, ReplyError)]
#[zlink(interface = "ch.yeetme.yeet")]
pub enum YeetDaemonError {}

pub struct YeetVarlinkService;

impl YeetVarlinkService {
    pub async fn start() -> Result<(), Report> {
        let service = Self;
        let _ = remove_file(SOCKET_PATH).await;
        let listener = unix::bind(SOCKET_PATH)?;
        let server = zlink::Server::new(listener, service);
        server.run().await.map_err(|e| e.into())
    }
}

impl Service for YeetVarlinkService {
    type MethodCall<'de> = YeetMethod;

    type ReplyParams<'ser> = YeetReply;

    type ReplyStreamParams = ();
    type ReplyStream = futures_util::stream::Empty<zlink::Reply<()>>;

    type ReplyError<'ser> = YeetDaemonError;

    async fn handle<'ser, 'de: 'ser, Sock: Socket>(
        &'ser mut self,
        method: Call<Self::MethodCall<'de>>,
        _conn: &mut Connection<Sock>,
    ) -> MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>> {
        match method.method() {
            YeetMethod::Status => {
                MethodReply::Single(Some(YeetReply::DaemonStatus(DaemonStatus {
                    up_to_date: UpToDate::Yes,
                    server: "heloooooo".to_owned(),
                    mode: YeetDaemonMode::Provisioned,
                    version: "Wouldn't you like to know weather boy".to_owned(),
                })))
            }
        }
    }
}
