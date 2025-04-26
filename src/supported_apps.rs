use crate::app_downloader::{Target, TargetDeployment};
use crate::gh::{get_github_download_links};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;
use mime::Mime;
use url::Url;

const GITHUB_API: &str = "https://api.github.com";

pub fn get_app(name: &str) -> Option<SupportedApp> {
    SUPPORTED_APPS.get(name).cloned()
}

#[derive(Debug, Clone)]
pub struct SupportedApp {
    pub shortname: String,
    pub repo: Repo,
    pub source: String,
}
impl SupportedApp {
    pub fn new(shortname: &str, repo: Repo, source: &str) -> Self {
        Self {
            shortname: shortname.to_string(),
            repo,
            source: source.to_string(),
        }
    }
}

pub static SUPPORTED_APPS: LazyLock<HashMap<&str, SupportedApp>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for (app, github_url) in [
        ("yq", "mikefarah/yq"),
        ("jq", "jqlang/jq"),
        ("gh", "cli/cli"),
        ("jsonnet", "google/go-jsonnet"),
        ("shellcheck", "koalaman/shellcheck"),
        ("shfmt", "mvdan/sh"),
        ("yutc", "adam-huganir/yutc"),
        ("kubectl", "kubernetes/kubectl"),
        ("helm", "helm/helm"),
        ("uv", "astral-sh/uv")
    ] {
        let _ = map.insert(
            app,
            SupportedApp::new(app, Repo::github(github_url), "github"),
        );
    }
    map
});

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Repo {
    Github(String),
    Url(String),
    Python(String),
}


impl Repo {
    pub(crate) fn github(repo: &str) -> Self {
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

    pub(crate) fn get_github_repo(&self) -> String {
        self.get_url().path().trim_start_matches("/repos/").to_string()
    }

    pub async fn  get_download_link(&self, version: &str, target_deployment: &TargetDeployment) -> Vec<DownloadInfo> {
        match self {
            Repo::Github(_) => get_github_download_links(&self, target_deployment, version).await.unwrap(),
            Repo::Url(url) => panic!("{} is not a github repo", url),
            Repo::Python(url) => panic!("{} is not a github repo", url),
        }
    }

}

#[derive(Debug)]
pub struct DownloadInfo {
    pub name: String,
    pub label: String,
    pub url: Url,
    pub content_type: Mime,
    pub size: u64,
    pub(crate) target: Target,
}

impl DownloadInfo {
    pub(crate) fn from_asset(asset: &octocrab::models::repos::Asset) -> Self {
        let mime = asset.content_type.parse::<Mime>().unwrap();
        Self {
            name: asset.name.clone(),
            label: asset.label.to_owned().unwrap_or("".to_string()),
            url: asset.browser_download_url.clone(),
            content_type: mime.to_owned(),
            size: asset.size as u64,
            target: Target::identify(
                &asset.name,
                Some(&mime),
                
            ),
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