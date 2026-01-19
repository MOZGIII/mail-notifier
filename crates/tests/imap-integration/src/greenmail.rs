//! GreenMail container helpers.

use testcontainers::{
    GenericImage, ImageExt as _, core::IntoContainerPort as _, runners::AsyncRunner as _,
};

/// Starts a GreenMail container configured with the provided credentials.
pub async fn start_greenmail(
    user: &str,
    password: &str,
) -> Result<testcontainers::ContainerAsync<GenericImage>, testcontainers::TestcontainersError> {
    let container = testcontainers::GenericImage::new("greenmail/standalone", "latest")
        .with_exposed_port(crate::IMAP_PORT.tcp())
        .with_wait_for(testcontainers::core::WaitFor::message_on_stdout(
            "Starting GreenMail API server at",
        ))
        .with_env_var("GREENMAIL_USERS", format!("{user}:{password}"))
        .start()
        .await?;

    Ok(container)
}
