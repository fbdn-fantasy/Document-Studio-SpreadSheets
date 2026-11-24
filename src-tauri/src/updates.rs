use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};
use tauri::{AppHandle, Manager};

const REPO: &str = "fbdn-fantasy/Document-Studio-SpreadSheets";
const PRODUCT: &str = "Document-Studio-Spreadsheets";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    state: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
}

#[derive(Clone, Copy)]
enum Installer {
    #[cfg(windows)]
    Windows,
    #[cfg(target_os = "linux")]
    Apt,
    #[cfg(target_os = "linux")]
    Dnf,
    #[cfg(target_os = "linux")]
    Zypper,
}

impl Installer {
    fn filename(self, version: &Version) -> String {
        match self {
            #[cfg(windows)]
            Self::Windows => format!("{PRODUCT}-V{version}-Setup.exe"),
            #[cfg(target_os = "linux")]
            Self::Apt => format!("{PRODUCT}-V{version}.deb"),
            #[cfg(target_os = "linux")]
            Self::Dnf | Self::Zypper => {
                format!("document-studio-spreadsheets-{version}-1.x86_64.rpm")
            }
        }
    }

    fn for_platform() -> Result<Self, String> {
        if !cfg!(target_arch = "x86_64") {
            return Err("Updates are available for x86_64 installations only.".into());
        }
        #[cfg(windows)]
        return Ok(Self::Windows);
        #[cfg(target_os = "linux")]
        {
            // Match the native package even on systems with more than one package manager.
            let os = fs::read_to_string("/etc/os-release")
                .or_else(|_| fs::read_to_string("/usr/lib/os-release"))
                .unwrap_or_default();
            let mut families = Vec::new();
            for key in ["ID=", "ID_LIKE="] {
                if let Some(value) = os.lines().find_map(|line| line.strip_prefix(key)) {
                    families.extend(value.trim_matches(&['"', '\''][..]).split_whitespace());
                }
            }
            for family in families {
                match family {
                    "debian" | "ubuntu" => return Ok(Self::Apt),
                    "fedora" | "rhel" | "centos" => return Ok(Self::Dnf),
                    s if s == "suse" || s.starts_with("opensuse") => return Ok(Self::Zypper),
                    _ => (),
                }
            }
            for (path, manager) in [
                ("/usr/bin/apt-get", Self::Apt),
                ("/usr/bin/dnf", Self::Dnf),
                ("/usr/bin/zypper", Self::Zypper),
            ] {
                if Path::new(path).is_file() {
                    return Ok(manager);
                }
            }
            Err("This installation requires APT, DNF or Zypper to install an update.".into())
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        return Err("Updates are available on Windows and Linux.".into());
    }
}

struct Pending {
    version: Version,
    installer: Installer,
    asset: Asset,
}

#[derive(Default)]
pub(crate) struct UpdateState(Mutex<Option<Pending>>);

fn stable_version(release: &Release) -> Option<Version> {
    if release.draft || release.prerelease {
        return None;
    }
    let tag = release
        .tag_name
        .strip_prefix(['V', 'v'])
        .unwrap_or(&release.tag_name);
    let version = Version::parse(tag).ok()?;
    (version.pre.is_empty() && version.build.is_empty()).then_some(version)
}

fn curl() -> Result<Command, String> {
    #[cfg(windows)]
    let path = PathBuf::from(
        std::env::var_os("SystemRoot").ok_or("Windows system directory is unavailable.")?,
    )
    .join("System32")
    .join("curl.exe");
    #[cfg(not(windows))]
    let path = PathBuf::from("/usr/bin/curl");
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.args([
        "-q",
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--max-redirs",
        "5",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "15",
        "--user-agent",
        "Document-Studio-Spreadsheets-Updater",
    ])
    .stdin(Stdio::null());
    Ok(cmd)
}

fn fetch(cmd: &mut Command) -> Result<Vec<u8>, String> {
    let output = cmd
        .output()
        .map_err(|_| "Could not start the system HTTPS client (curl).")?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let error = String::from_utf8_lossy(&output.stderr);
    if error.contains("403") || error.contains("429") {
        return Err(
            "GitHub declined the request or its request limit was reached. Try again later.".into(),
        );
    }
    Err("Could not download from GitHub. Check your connection and try again.".into())
}

fn newest_release(current: &Version) -> Result<Option<(Version, Release)>, String> {
    let mut newest = None;
    for page in 1_u32.. {
        let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=100&page={page}");
        let bytes = fetch(curl()?.args([
            "--max-time",
            "45",
            "--max-filesize",
            "33554432",
            "--header",
            "Accept: application/vnd.github+json",
            &url,
        ]))?;
        let releases: Vec<Release> = serde_json::from_slice(&bytes)
            .map_err(|_| "GitHub returned an unreadable list of releases.")?;
        let more = releases.len() == 100;
        newest = releases
            .into_iter()
            .filter_map(|release| {
                let version = stable_version(&release)?;
                (&version > current).then_some((version, release))
            })
            .chain(newest)
            .max_by(|a, b| a.0.cmp(&b.0));
        if !more {
            break;
        }
    }
    Ok(newest)
}

fn select_asset(
    version: Version,
    release: Release,
    installer: Installer,
) -> Result<Pending, String> {
    let name = installer.filename(&version);
    let asset = release.assets.into_iter().find(|a| a.name == name && a.state == "uploaded")
        .ok_or_else(|| format!("V{version} is available, but its installer for this system has not been published."))?;
    let official_url = format!(
        "https://github.com/{REPO}/releases/download/{}/{name}",
        release.tag_name
    );
    if asset.browser_download_url != official_url
        || asset.size == 0
        || !asset
            .digest
            .as_deref()
            .and_then(|s| s.strip_prefix("sha256:"))
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(format!(
            "GitHub has not supplied verifiable installer details for V{version}."
        ));
    }
    Ok(Pending {
        version,
        installer,
        asset,
    })
}

#[tauri::command]
pub(crate) async fn check_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<UpdateState>();
        let mut pending = state
            .0
            .try_lock()
            .map_err(|_| "An update is already in progress.")?;
        *pending = None;
        let current = &app.package_info().version;
        #[cfg(windows)]
        if let Ok(dir) = update_cache(&app) {
            // An installer handed off by the previous version can be removed after that upgrade.
            let _ = fs::remove_file(dir.join(Installer::Windows.filename(current)));
        }
        let Some((version, release)) = newest_release(current)? else {
            return Ok(None);
        };
        let label = version.to_string();
        *pending = Some(select_asset(version, release, Installer::for_platform()?)?);
        Ok(Some(label))
    })
    .await
    .map_err(|_| "The update check was interrupted.")?
}

fn verify(path: &Path, asset: &Asset) -> Result<(), String> {
    let mut file = File::open(path).map_err(|_| "Could not open the downloaded installer.")?;
    let mut hash = Sha256::new();
    let size =
        io::copy(&mut file, &mut hash).map_err(|_| "Could not read the downloaded installer.")?;
    let actual = format!("sha256:{:x}", hash.finalize());
    if size != asset.size
        || !asset
            .digest
            .as_deref()
            .is_some_and(|d| d.eq_ignore_ascii_case(&actual))
    {
        return Err("The installer did not match GitHub’s size and SHA-256 digest. Try downloading it again.".into());
    }
    Ok(())
}

fn update_cache(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|dir| dir.join("updates"))
        .map_err(|_| "The update download folder is unavailable.".into())
}

fn install(app: &AppHandle, update: &Pending) -> Result<&'static str, String> {
    let dir = update_cache(app)?;
    fs::create_dir_all(&dir).map_err(|_| "Could not create the update download folder.")?;
    let staging =
        tempfile::tempdir_in(&dir).map_err(|_| "Could not prepare the installer download.")?;
    let path = staging.path().join(&update.asset.name);
    fetch(
        curl()?
            .args([
                "--max-time",
                "1800",
                "--max-filesize",
                &update.asset.size.to_string(),
                "--output",
            ])
            .arg(&path)
            .arg(&update.asset.browser_download_url),
    )?;
    verify(&path, &update.asset)?;
    match update.installer {
        #[cfg(windows)]
        Installer::Windows => {
            // Keep the verified installer available after this process closes for the upgrade.
            let target = dir.join(&update.asset.name);
            fs::rename(path, &target)
                .map_err(|_| "Could not prepare the verified Windows installer.")?;
            Command::new(target)
                .args(["/UPDATE", "/P", "/R"])
                .arg(format!("/DSS-WAIT-PID={}", std::process::id()))
                .spawn()
                .map_err(|_| "Could not start the Windows update installer.")?;
            Ok("installer-started")
        }
        #[cfg(target_os = "linux")]
        manager => {
            let args: &[&str] = match manager {
                Installer::Apt => &["/usr/bin/apt-get", "-y", "install"],
                Installer::Dnf => &["/usr/bin/dnf", "-y", "install"],
                Installer::Zypper => &["/usr/bin/zypper", "--non-interactive", "install"],
            };
            let status = Command::new("/usr/bin/pkexec")
                .arg("--disable-internal-agent")
                .args(args)
                .arg(path)
                .stdin(Stdio::null())
                .status()
                .map_err(|_| {
                    "Could not start the system authorization dialog. Install polkit and try again."
                })?;
            match status.code() {
                Some(0) => Ok("installed"),
                Some(126 | 127) => Err("The update was cancelled or administrator authorization was unavailable.".into()),
                _ => Err("The system package manager could not install the update. Check its availability and try again.".into()),
            }
        }
    }
}

#[tauri::command]
pub(crate) async fn install_update(
    app: AppHandle,
    version: String,
) -> Result<&'static str, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<UpdateState>();
        let mut pending = state
            .0
            .try_lock()
            .map_err(|_| "An update is already in progress.")?;
        let update = pending
            .as_ref()
            .filter(|u| u.version.to_string() == version)
            .ok_or("Check for updates again before installing.")?;
        let result = install(&app, update)?;
        *pending = None;
        Ok(result)
    })
    .await
    .map_err(|_| "The update installation was interrupted.")?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: vec![],
        }
    }

    #[test]
    fn accepts_stable_three_part_versions_and_ignores_legacy_and_previews() {
        assert_eq!(
            stable_version(&release("V1.0.10")).unwrap(),
            Version::new(1, 0, 10)
        );
        assert!(stable_version(&release("v1.0.10")) > stable_version(&release("V1.0.9")));
        for tag in [
            "V10",
            "V1",
            "V1.1",
            "V01.0.0",
            "nightly",
            "V1.0.1-rc.1",
            "V1.0.1+build",
            "VV1.0.1",
        ] {
            assert!(stable_version(&release(tag)).is_none(), "{tag}");
        }
        let mut candidate = release("V1.0.1");
        candidate.draft = true;
        assert!(stable_version(&candidate).is_none());
        candidate.draft = false;
        candidate.prerelease = true;
        assert!(stable_version(&candidate).is_none());
    }

    #[test]
    fn accepts_only_the_uploaded_native_package_in_the_official_release() {
        let version = Version::new(1, 0, 1);
        let installer = Installer::for_platform().unwrap();
        let name = installer.filename(&version);
        let url = format!("https://github.com/{REPO}/releases/download/V1.0.1/{name}");
        let fixture = |asset_name: &str, download: &str, hash: Option<String>| {
            let mut rel = release("V1.0.1");
            rel.assets.push(Asset {
                name: asset_name.into(),
                browser_download_url: download.into(),
                digest: hash,
                state: "uploaded".into(),
                size: 3,
            });
            select_asset(version.clone(), rel, installer)
        };
        let digest = Some(format!("sha256:{}", "ab".repeat(32)));
        assert!(fixture(&name, &url, digest.clone()).is_ok());
        assert!(fixture("source.zip", &url, digest.clone()).is_err());
        assert!(fixture(&name, "https://example.invalid/installer", digest).is_err());
        assert!(fixture(&name, &url, None).is_err());
        assert!(fixture(&name, &url, Some("sha256:invalid".into())).is_err());
    }

    #[test]
    fn rejects_a_modified_or_incomplete_download() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("installer");
        let mut asset = Asset {
            name: String::new(),
            state: "uploaded".into(),
            size: 3,
            digest: Some(
                "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            ),
            browser_download_url: String::new(),
        };
        fs::write(&path, b"abc").unwrap();
        assert!(verify(&path, &asset).is_ok());
        fs::write(&path, b"abd").unwrap();
        assert!(verify(&path, &asset).is_err());
        fs::write(&path, b"abc").unwrap();
        asset.size = 4;
        assert!(verify(&path, &asset).is_err());
    }
}
