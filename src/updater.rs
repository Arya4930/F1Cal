use self_update::{cargo_crate_version, version::bump_is_greater};

const OWNER: &str = "Arya4930";
const REPO: &str = "F1Cal";
const BIN_NAME: &str = "F1Cal";

pub enum UpdateMsg {
    Available(String),
    UpToDate,
    Installed(String),
    Failed(String),
}

#[derive(Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    Available(String),
    Downloading,
    Done(String),
    Failed(String),
}

pub fn check_for_update() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPO)
        .bin_name(BIN_NAME)
        .current_version(cargo_crate_version!())
        .build()?;

    match updater.is_update_available()? {
        Some(release) => Ok(Some(release.version().to_string())),
        None => Ok(None),
    }
}

pub fn install_update() -> Result<String, Box<dyn std::error::Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPO)
        .bin_name(BIN_NAME)
        .current_version(cargo_crate_version!())
        .show_download_progress(false) // no console in a GUI app
        .no_confirm(true)
        .build()?
        .update()?;

    Ok(status.version().to_string())
}