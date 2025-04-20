use crate::app_downloader;
use std::fmt::Display;
use octocrab::models::repos::Release;
use url::Url;
use crate::app_downloader::{Target, TargetDeployment};

const GITHUB_API: &str = "https://api.github.com";
enum Repo {
    Github(String),
    Url(String),
    Python(String),
}

impl Repo {
    fn github(repo: &str) -> Self {
        Self::Github(format!("{}/repos/{}", GITHUB_API, repo))
    }

    fn url(url: &str) -> Self {
        Self::Url(url.to_string())
    }

    fn python(app: &str) -> Self {
        Self::Python(format!("https://pypi.org/simple/{}", app))
    }

    fn get_url(&self) -> Url {
        match self {
            Repo::Github(repo) => Url::parse(repo).unwrap(),
            Repo::Url(url) => Url::parse(url).unwrap(),
            Repo::Python(url) => Url::parse(url).unwrap(),
        }
    }

    fn get_github_repo(&self) -> String {
        self.get_url().path().trim_start_matches("/repos/").to_string()
    }

    // fn get_github_api_url(&self) -> Url {
    //     match self {
    //         Repo::Github(repo) => {},
    //         Repo::Url(url) => panic!("{} is not a github repo", url),
    //         Repo::Python(url) => panic!("{} is not a github repo", url),
    //     }
    // }
}

struct AppInfo {
    binary_name: String,
    repo: Repo,
}

impl AppInfo {
    fn new(binary_name: &str, repo: Repo) -> Self {
        Self {
            binary_name: binary_name.to_string(),
            repo,
        }
    }
}

#[derive(Debug)]
struct DownloadInfo {
    name: String,
    label: String,
    url: Url,
    content_type: String,
    size: u64,
    target: app_downloader::Target,
}

impl DownloadInfo {
    fn from_asset(asset: &octocrab::models::repos::Asset) -> Self {
        Self {
            name: asset.name.clone(),
            label: asset.label.to_owned().unwrap_or("".to_string()),
            url: asset.browser_download_url.clone(),
            content_type: asset.content_type.clone(),
            size: asset.size as u64,
            target: Target::identify(&asset.name),
        }
    }
}

impl Display for DownloadInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]({}) for {} as a {}",
            self.name, self.url, self.target.deployment, self.target.filetype
        )
    }
}

pub async fn get_link(app: &Repo, target: &TargetDeployment, version: Option<&str>) -> Release {
    let version: &str = version.unwrap_or("latest");
    let octocrab = octocrab::instance();
    let repo = app.get_github_repo();
    let (owner, repo) = repo.split_once('/').unwrap();
    let repo = octocrab.repos(owner, repo);
    let releases = repo.releases();
    let release: Release;
    match version {
        "latest" => {
            release = releases.get_latest().await.unwrap();
        }
        _ => {
            release = releases.get_by_tag(version).await.unwrap();
        }
    }
    release
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn base_test() {
        let deployment = TargetDeployment::default();
        for repo in [
            Repo::github("adam-huganir/yutc"),
            Repo::github("cli/cli"),
            Repo::github("google/go-jsonnet"),
            Repo::github("jqlang/jq"),
            Repo::github("koalaman/shellcheck"),
            Repo::github("mikefarah/yq"),
            Repo::github("mvdan/sh"),
        ] {
            let release = get_link(&repo, &deployment, None).await;
            let mut matched = vec![];
            for asset in release.assets {
                let info = DownloadInfo::from_asset(&asset);
                if info.target.deployment == deployment {
                    matched.push(info);
                }
            }
            assert!(matched.len() > 0);
            for info in matched {
                println!("{}", info);
            }
        }
    }
}
