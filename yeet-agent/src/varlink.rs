use std::{
    os::unix::fs::{PermissionsExt, lchown},
    path::Path,
};

use api::AgentAction;
use httpsig_hyper::prelude::SecretKey;
use nix::unistd::Group;
use rootcause::{Report, compat::ReportAsError, prelude::ResultExt};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, remove_file};
use url::Url;
use yeet::server;
use zlink::{
    Call, Connection, ReplyError, Service, connection::Socket, proxy, service::MethodReply, unix,
};

shadow_rs::shadow!(build);

use crate::{
    cli_args::{self, AgentConfig},
    version,
};

const SOCKET_PATH: &str = "/run/yeet/agent.varlink";

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "parameters")]
pub enum YeetMethod {
    #[serde(rename = "ch.yeetme.yeet.Status")]
    Status,
    #[serde(rename = "ch.yeetme.yeet.Config")]
    Config,
    #[serde(rename = "ch.yeetme.yeet.Detach")]
    Detach { version: api::StorePath },
}

#[proxy("ch.yeetme.yeet")]
pub trait YeetProxy {
    async fn status(&mut self) -> zlink::Result<Result<DaemonStatus, YeetDaemonError>>;
    async fn config(&mut self) -> zlink::Result<Result<AgentConfig, YeetDaemonError>>;
    async fn detach(
        &mut self,
        version: api::StorePath,
    ) -> zlink::Result<Result<(), YeetDaemonError>>;
}

pub async fn client() -> Result<Connection<zlink::unix::Stream>, Error> {
    log::debug!("Connecting to {SOCKET_PATH}");
    Ok(unix::connect(SOCKET_PATH)
        .await
        .context("Trying to set up the varlink connection\nMake sure you are in the `yeet` group and the daemon is running")
        .map_err(ReportAsError::from)?)
}

pub async fn status() -> Result<DaemonStatus, Error> {
    let mut client = client().await?;
    client
        .status()
        .await
        .context("Could not communicate with the varlink daemon")
        .map_err(ReportAsError::from)?
        .map_err(|e| Error::DaemonError(e))
}

pub async fn config() -> Result<AgentConfig, Error> {
    let mut client = client().await?;
    client
        .config()
        .await
        .context("Could not communicate with the varlink daemon")
        .map_err(ReportAsError::from)?
        .map_err(|e| Error::DaemonError(e))
}

pub async fn detach(version: api::StorePath) -> Result<(), Error> {
    let mut client = client().await?;
    client
        .detach(version)
        .await
        .context("Could not communicate with the varlink daemon")
        .map_err(ReportAsError::from)?
        .map_err(|e| Error::DaemonError(e))
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Report(#[from] ReportAsError<&'static str>),
    #[error("Defined error from Daemon:\n{0:?}")]
    DaemonError(YeetDaemonError),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum YeetReply {
    Status(DaemonStatus),
    Config(AgentConfig),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub up_to_date: UpToDate,
    pub server: Url,
    pub mode: DaemonMode,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UpToDate {
    Yes,
    No,
    Detached,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonMode {
    Provisioned,
    Unverified,
    Detached,
    NetworkError,
}

#[derive(Debug, ReplyError)]
#[zlink(interface = "ch.yeetme.yeet")]
pub enum YeetDaemonError {
    NoCurrentSystem,
    CredentialError,
}

pub struct YeetVarlinkService {
    config: cli_args::AgentConfig,
    key: SecretKey,
}

impl Service for YeetVarlinkService {
    type MethodCall<'de> = YeetMethod;

    type ReplyParams<'ser> = YeetReply;

    type ReplyStreamParams = ();
    type ReplyStream = futures_util::stream::Empty<zlink::Reply<()>>;

    type ReplyError<'ser> = YeetDaemonError;

    async fn handle<'ser, Sock: Socket>(
        &'ser mut self,
        method: &'ser Call<Self::MethodCall<'_>>,
        _conn: &mut Connection<Sock>,
    ) -> MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>>
// where
    //     <Sock as zlink::connection::Socket>::ReadHalf: zlink::connection::socket::UnixSocket,
    {
        match method.method() {
            YeetMethod::Status => {
                log::debug!("Varlink: Daemon status requested");

                //TODO unwrap
                let verified =
                    match server::system::is_host_verified(&self.config.server, &self.key).await {
                        Ok(verified) => Some(verified.is_success()),
                        Err(_) => None,
                    };

                let system_check = {
                    let Ok(store_path) = version::get_active_version() else {
                        return MethodReply::Error(YeetDaemonError::NoCurrentSystem);
                    };

                    server::system::check(
                        &self.config.server,
                        &self.key,
                        &api::VersionRequest { store_path },
                    )
                    .await
                };

                let up_to_date = match system_check {
                    Ok(AgentAction::Nothing) => UpToDate::Yes,
                    Ok(AgentAction::Detach) => UpToDate::Detached,
                    Ok(AgentAction::SwitchTo(_)) | Err(_) => UpToDate::No,
                };

                let mode = 'b: {
                    if let Some(verified) = verified
                        && !verified
                    {
                        break 'b DaemonMode::Unverified;
                    }

                    match system_check {
                        Ok(AgentAction::Nothing) | Ok(AgentAction::SwitchTo(_)) => {
                            DaemonMode::Provisioned
                        }
                        Ok(AgentAction::Detach) => DaemonMode::Detached,
                        Err(_) => DaemonMode::NetworkError,
                    }
                };

                MethodReply::Single(Some(YeetReply::Status(DaemonStatus {
                    up_to_date,
                    server: self.config.server.clone(),
                    mode,
                    version: String::from(build::PKG_VERSION),
                })))
            }
            YeetMethod::Config => MethodReply::Single(Some(YeetReply::Config(self.config.clone()))),
            YeetMethod::Detach { version } => {
                // polkit::detach(pid, uid);
                todo!()
            }
        }
    }
}

impl YeetVarlinkService {
    pub async fn start(config: cli_args::AgentConfig, key: SecretKey) -> Result<(), Report> {
        let listener = {
            let _ = remove_file(SOCKET_PATH).await;
            fs::create_dir_all(Path::new(SOCKET_PATH).parent().unwrap())
                .await
                .context("Ensuring the Socket dir is available")?;
            let mut listener =
                zlink::unix::bind(SOCKET_PATH).attach(format!("SOCKET_PATH: {SOCKET_PATH}"))?;

            setup_socket_permissions(SOCKET_PATH, "yeet").await?;

            listener
        };

        log::debug!("Socket created at {SOCKET_PATH}");
        let server = zlink::Server::new(listener, Self { config, key });
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
