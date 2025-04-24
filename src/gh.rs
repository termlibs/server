use crate::app_downloader::TargetDeployment;
use crate::supported_apps::{DownloadInfo, Repo};
use octocrab::models::repos::Release;
use std::fmt::Display;

pub async fn get_github_download_links(
    repo: &Repo,
    target_deployment: &TargetDeployment,
    version: &str,
) -> Vec<DownloadInfo> {
    let octocrab = octocrab::instance();
    let repo = repo.get_github_repo();
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
    let mut matched = vec![];
    let skippable_extensions = [
        ".asc", ".md5", ".sha1", ".sha256", ".sha512", ".sig", ".txt",
    ];
    for asset in release.assets {
        let download_info = DownloadInfo::from_asset(&asset);
        if &download_info.target.deployment == target_deployment
            && !skippable_extensions
                .iter()
                .any(|ext| download_info.name.ends_with(ext)) // TODO: fixme
        {
            matched.push(download_info);
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supported_apps::Repo;

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
            // let release = get_link(&repo, &deployment, None).await;
            // let mut matched = vec![];
            // for asset in release.assets {
            //     let info = DownloadInfo::from_asset(&asset);
            //     if info.target.deployment == deployment {
            //         matched.push(info);
            //     }
            // }
            // assert!(matched.len() > 0);
            // for info in matched {
            //     println!("{}", info);
            // }
        }
    }
}
