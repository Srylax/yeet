use std::{
    os::unix::fs::{PermissionsExt, lchown},
    path::Path,
};

use nix::unistd::Group;
use rootcause::{Report, prelude::ResultExt};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, remove_file};
use zlink::{
    Call, Connection, ReplyError, Service, connection::Socket, proxy, service::MethodReply, unix,
};

const SOCKET_PATH: &str = "/run/yeet/agent.varlink";

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "parameters")]
pub enum YeetMethod {
    #[serde(rename = "ch.yeetme.yeet.Status")]
    Status,
}

#[proxy("ch.yeetme.yeet")]
pub trait YeetProxy {
    async fn status(&mut self) -> zlink::Result<Result<DaemonStatus, YeetDaemonError>>;
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum YeetReply {
    Status(DaemonStatus),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub up_to_date: UpToDate,
    pub server: String,
    pub mode: YeetDaemonMode,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(dead_code)]
pub enum UpToDate {
    Yes,
    No,
    Detached,
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(dead_code)]
pub enum YeetDaemonMode {
    Provisioned,
    Detached,
    Unknown,
}

pub async fn client() -> Result<Connection<zlink::unix::Stream>, Report> {
    log::debug!("Connecting to {SOCKET_PATH}");
    Ok(unix::connect(SOCKET_PATH)
        .await
        .context("Trying to set up varlink connection. Make sure you are in the `yeet` group")?)
}

#[derive(Debug, ReplyError)]
#[zlink(interface = "ch.yeetme.yeet")]
pub enum YeetDaemonError {}

pub struct YeetVarlinkService;

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
                log::debug!("Varlink: Daemon status requested");
                MethodReply::Single(Some(YeetReply::Status(DaemonStatus {
                    up_to_date: UpToDate::Yes,
                    server: "heloooooo".to_owned(),
                    mode: YeetDaemonMode::Provisioned,
                    version: "Wouldn't you like to know weather boy".to_owned(),
                })))
            }
        }
    }
}

impl YeetVarlinkService {
    pub async fn start() -> Result<(), Report> {
        let listener = {
            let _ = remove_file(SOCKET_PATH).await;
            fs::create_dir_all(Path::new(SOCKET_PATH).parent().unwrap())
                .await
                .context("Ensuring the Socket dir is available")?;
            let listener = unix::bind(SOCKET_PATH).attach(format!("SOCKET_PATH: {SOCKET_PATH}"))?;
            setup_socket_permissions(SOCKET_PATH, "yeet").await?;
            listener
        };

        log::debug!("Socket created at {SOCKET_PATH}");
        let server = zlink::Server::new(listener, Self);
        log::info!("Listening for varlink connections");
        server.run().await.map_err(std::convert::Into::into)
    }
}

async fn setup_socket_permissions(path: &str, group_name: &str) -> Result<(), Report> {
    let group = Group::from_name(group_name)
        .context("Error while trying to look up `yeet` group on the system")?
        .ok_or(rootcause::report!("`yeet` Group does not exist on system"))?;

    lchown(path, None, Some(group.gid.as_raw()))
        .context("Trying to set varlink socket connection")?;

    let mut perms = fs::metadata(path)
        .await
        .context("Trying to fetch existing file permissions")?
        .permissions();
    perms.set_mode(0o660);
    fs::set_permissions(path, perms)
        .await
        .context("Trying to set file permission")?;

    log::debug!("Socket permissions set: Group '{group_name}' can now access {path}");
    Ok(())
}
