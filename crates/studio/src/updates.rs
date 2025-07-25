use std::fmt::Display;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Release {
    tag_name: String,
    html_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UpdateInfo {
    pub version: [u8; 3],
    pub url: String,
}

fn parse_version(version: &str) -> Option<[u8; 3]> {
    version
        .trim_start_matches("lutgen-studio-v")
        .split('.')
        .map(|v| v.parse().ok())
        .collect::<Option<Vec<u8>>>()?
        .try_into()
        .ok()
}

impl TryFrom<Release> for UpdateInfo {
    type Error = ();

    fn try_from(rel: Release) -> Result<Self, Self::Error> {
        if !rel.tag_name.starts_with("lutgen-studio-v") {
            return Err(());
        }
        let version = parse_version(&rel.tag_name).ok_or(())?;
        Ok(UpdateInfo {
            version,
            url: rel.html_url,
        })
    }
}

fn check_for_updates_inner(current: [u8; 3]) -> Result<Option<UpdateInfo>, String> {
    fn update_err(e: impl Display) -> String {
        format!("Failed to fetch latest github releases: {e}")
    }

    let body = ureq::get("https://api.github.com/repos/ozwaldorf/lutgen-rs/releases")
        .call()
        .map_err(update_err)?
        .into_body();

    let releases: Vec<Release> = serde_json::from_reader(body.into_reader()).map_err(update_err)?;

    let Some(latest) = releases
        .into_iter()
        .filter_map(|rel| UpdateInfo::try_from(rel).ok())
        .max()
    else {
        return Ok(None);
    };

    if latest.version > current {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}

/// Check for latest updates from github releases api.
///
/// In debug mode, should always return the latest version.
/// In release mode, only returns a version greater than the package version.
pub fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    #[cfg(debug_assertions)]
    let current = [0, 0, 0];
    #[cfg(not(debug_assertions))]
    let current = parse_version(env!("CARGO_PKG_VERSION")).unwrap();
    check_for_updates_inner(current)
}
