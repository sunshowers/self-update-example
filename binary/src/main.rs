// self-update-example
//
// Written in 2021 by Rain <rain@sunshowers.io>
//
// To the extent possible under law, the author(s) have dedicated all copyright and related and
// neighboring rights to this software to the public domain worldwide. This software is distributed
// without any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication along with this software. If
// not, see <http://creativecommons.org/publicdomain/zero/1.0/>.

//! This is a working example that shows an end-to-end flow using the self_update crate.

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{bail, Report, Result, WrapErr};
use semver::VersionReq;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::env;
use std::fs;
use std::str::FromStr;
use structopt::StructOpt;

/// Default name of config file.
pub static CONFIG_FILE_NAME: &str = "Config.toml";

/// Working example for self-update.
#[derive(Debug, StructOpt)]
pub struct Args {
    /// Location of config file (default: <workspace root>/Config.toml).
    #[structopt(long = "config")]
    config_path: Option<Utf8PathBuf>,

    /// Subcommand to execute
    #[structopt(subcommand)]
    subcommand: Subcommand,
}

impl Args {
    pub fn exec(self) -> Result<()> {
        let config_path = match self.config_path {
            Some(path) => path,
            None => {
                let mut project_root = get_project_root()?;
                project_root.push(CONFIG_FILE_NAME);
                project_root
            }
        };
        let config = Config::read_path(&config_path)?;

        self.subcommand.exec(config)
    }
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// List currently available releases
    ListReleases,

    /// Update to a release
    Update {
        /// Release to update to (default: pinned or latest)
        #[structopt(long, short)]
        version: Option<DownloadVersion>,
    },
}

impl Subcommand {
    pub fn exec(self, config: Config) -> Result<()> {
        match self {
            Subcommand::ListReleases => {
                let releases = self_update::backends::github::ReleaseList::configure()
                    .repo_owner(&config.repo.owner)
                    .repo_name(&config.repo.name)
                    .build()?
                    .fetch()?;

                for release in releases {
                    println!(
                        "- Name: {}\n  Version: {}\n  Date: {}",
                        release.name, release.version, release.date
                    );
                }
            }
            Subcommand::Update { .. } => {
                unimplemented!();
            }
        }

        Ok(())
    }
}

/// Configuration for self-update-example. Read from the downstream repository.
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Config {
    /// The repository to download updates from.
    repo: RepoId,

    /// The prefix for version numbers.
    prefix: String,

    /// The version requirement to download.
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)]
    version: DownloadVersion,
}

impl Config {
    /// Read the config from a given path.
    pub fn read_path(path: &Utf8Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("error reading config from {}", path))?;
        toml::from_str(&contents).with_context(|| format!("error deserializing config"))
    }
}

#[derive(Debug, Deserialize)]
pub struct RepoId {
    owner: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DownloadVersion {
    /// Download the latest version.
    Latest,
    /// Download a version or requirement.
    Pinned(VersionReq),
}

// serde and structopt use the Default::default impl.
impl Default for DownloadVersion {
    fn default() -> Self {
        DownloadVersion::Latest
    }
}

// This impl is used by structopt to convert a value read from the command-line into a
// proper value.
impl FromStr for DownloadVersion {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("latest") {
            Ok(DownloadVersion::Latest)
        } else {
            // Try parsing the version as a semver requirement.
            let version_req = s
                .parse::<VersionReq>()
                .with_context(|| format!("error parsing version '{}'", s))?;
            Ok(DownloadVersion::Pinned(version_req))
        }
    }
}

fn get_project_root() -> Result<Utf8PathBuf> {
    color_eyre::install()?;

    // Use duct to run cargo -- it handles a lot of nasty error cases, saving 20-30 lines of
    // complicated boilerplate.
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let path = duct::cmd!(
        &cargo,
        "locate-project",
        "--message-format",
        "plain",
        "--workspace"
    )
    .stdout_capture()
    .read()
    .with_context(|| format!("error executing '{} locate-project'", cargo))?;

    let mut utf8_path = Utf8PathBuf::from(path);
    // The last component is expected to be Cargo.toml.
    if !utf8_path.ends_with("Cargo.toml") {
        bail!(
            "'cargo locate-project' output does not end with Cargo.toml: {}",
            utf8_path
        )
    };

    // Remove the last component.
    utf8_path.pop();
    Ok(utf8_path)
}

fn main() -> Result<()> {
    let args = Args::from_args();
    args.exec()
}
