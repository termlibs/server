use crate::app_downloader::TargetDeployment;
use crate::supported_apps::{DownloadInfo, Repo};
use octocrab::models::repos::Release;
use std::fmt::Display;

const MIN_ASSET_SIZE: u64 = 64 * 1024; // arbitrary, may need to change if we start installing scripts

pub async fn get_github_download_links(
    repo: &Repo,
    target_deployment: &TargetDeployment,
    version: &str,
) -> anyhow::Result<Vec<DownloadInfo>> {
    let octocrab = octocrab::instance();
    let repo_string = repo.get_github_repo();
    let (owner, repo) = repo_string.split_once('/').unwrap();
    let repo = octocrab.repos(owner, repo);
    let releases = repo.releases();
    let release: Release;
    print!("checking for release {} from {:?}\n", version, repo_string);
    match version {
        "latest" => {
            release = releases.get_latest().await?;
        }
        _ => {
            release = releases.get_by_tag(version).await?;
        }
    }
    let mut matched = vec![];
    let skippable_extensions = [
        ".asc", ".md5", ".sha1", ".sha256", ".sha512", ".sig", ".txt",
    ];
    let skippable_mimetypes = [
        mime::TEXT_PLAIN, // will probably break the ability to get scripts, so we may remove this
                          // at a future time
    ];

    for asset in release.assets {
        let download_info = DownloadInfo::from_asset(&asset);
        let is_target = &download_info.target.deployment == target_deployment;
        let extension_skippable = skippable_extensions
            .iter()
            .any(|ext| download_info.name.ends_with(ext));
        let mimetype_skippable = skippable_mimetypes
            .iter()
            .any(|mime| download_info.content_type.essence_str() == mime.essence_str());
        let is_big_enough = download_info.size > MIN_ASSET_SIZE;
        if is_target && !extension_skippable && is_big_enough && !mimetype_skippable {
            debug!(
                "+ matched: {:?} {:?} ({:?}, {}) thought to be {}",
                download_info.name,
                download_info.target.filetype,
                download_info.content_type,
                download_info.size,
                download_info.target.deployment
            );
            matched.push(download_info);
        } else {
            debug!(
                "- skipped: {:?} {:?} ({:?}, {}) thought to be {}. \
            other reasons: extension_skippable: {} is_big_enough: {} mimetype_skippable: {}",
                download_info.name,
                download_info.target.filetype,
                download_info.content_type,
                download_info.size,
                download_info.target.deployment,
                extension_skippable,
                is_big_enough,
                mimetype_skippable
            );
        }
    }
    Ok(matched)
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
