use rootcause::Report;
use zbus::Connection;
use zbus_polkit::policykit1::*;

use crate::cli_args::Config;

pub async fn detach(config: &Config, version: Option<api::StorePath>) -> Result<(), Report> {
    let connection = Connection::system().await?;
    let proxy = AuthorityProxy::new(&connection).await?;
    let subject = Subject::new_for_owner(std::process::id(), None, None)?;
    let result = proxy
        .check_authorization(
            &subject,
            "ch.yeetme.yeet.Detach",
            &std::collections::HashMap::new(),
            CheckAuthorizationFlags::AllowUserInteraction.into(),
            "",
        )
        .await?;
    todo!()
}
