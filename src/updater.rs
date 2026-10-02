use self_update::cargo_crate_version;

pub fn check_for_update() -> Result<bool, Box<dyn std::error::Error>> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner("Arya4930")
        .repo_name("F1Cal")
        .bin_name("F1Cal")
        .current_version(cargo_crate_version!())
        .build()?;

    match updater.is_update_available()? {
        Some(release) => {
            println!("Update available: {}", release.version());
            Ok(true)
        }

        None => {
            println!("Already up to date.");
            Ok(false)
        }
    }
}