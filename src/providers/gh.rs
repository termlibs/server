use crate::domain::platform::TargetDeployment;
use crate::error::AppError;
use crate::supported_apps::{DownloadInfo, Repo};
use log::debug;
use octocrab::models::repos::Release;

const MIN_ASSET_SIZE: u64 = 64 * 1024; // arbitrary, may need to change if we start installing scripts

pub(crate) async fn get_github_download_links(
  repo: &Repo,
  target_deployment: &TargetDeployment,
  version: &str,
) -> Result<Vec<DownloadInfo>, AppError> {
  let octocrab = octocrab::instance();
  let repo_string = repo.get_github_repo()?;
  let (owner, repo_name) = repo_string
    .split_once('/')
    .ok_or_else(|| AppError::InvalidInput(format!("Invalid github repo path: {}", repo_string)))?;
  let repo = octocrab.repos(owner, repo_name);
  let releases = repo.releases();
  let release: Release;

  debug!("checking for release '{}' from {:?}", version, repo_string);
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
  ];
  let download_infos: Vec<DownloadInfo> = release
    .assets
    .iter()
    .map(DownloadInfo::from_asset)
    .collect();

  let (name_width, filetype_width, mime_width, size_width, deployment_width) =
    calc_all_widths(&download_infos);
  let col = |value: String, width: usize| format!("{value:<width$.width$}");

  for download_info in download_infos {
    let is_target = &download_info.target.deployment == target_deployment;
    let extension_skippable = skippable_extensions
      .iter()
      .any(|ext| download_info.name.ends_with(ext));
    let mimetype_skippable = skippable_mimetypes
      .iter()
      .any(|mime| download_info.content_type.essence_str() == mime.essence_str());
    let is_big_enough = download_info.size > MIN_ASSET_SIZE;
    let name_col = col(format!("{:?}", download_info.name), name_width);
    let filetype_col = col(
      format!("{:?}", download_info.target.filetype.to_string()),
      filetype_width,
    );
    let mime_col = col(
      format!("{:?}", download_info.content_type.essence_str()),
      mime_width,
    );
    let size_col = col(
      format!("{:.3}", download_info.size as f64 / (1024f64.powi(2))),
      size_width,
    );
    let deployment_col = col(
      format!("{:?}", download_info.target.deployment.to_string()),
      deployment_width,
    );
    if is_target && !extension_skippable && is_big_enough && !mimetype_skippable {
      debug!(
        "match=true  name={} filetype={} mime={} size_mb={} deployment={}",
        name_col, filetype_col, mime_col, size_col, deployment_col
      );
      matched.push(download_info);
    } else {
      debug!(
        "match=false name={} filetype={} mime={} size_mb={} deployment={} ext_skip={:?} size_ok={:?} mime_skip={:?}",
        name_col,
        filetype_col,
        mime_col,
        size_col,
        deployment_col,
        extension_skippable,
        is_big_enough,
        mimetype_skippable
      );
    }
  }
  Ok(matched)
}

fn calc_all_widths(download_infos: &Vec<DownloadInfo>) -> (usize, usize, usize, usize, usize) {
  let calc_width =
    |values: Vec<String>| -> usize { values.iter().map(String::len).max().unwrap_or(0).min(32) };
  let name_width = calc_width(
    download_infos
      .iter()
      .map(|d| format!("{:?}", d.name))
      .collect(),
  );
  let filetype_width = calc_width(
    download_infos
      .iter()
      .map(|d| format!("{:?}", d.target.filetype.to_string()))
      .collect(),
  );
  let mime_width = calc_width(
    download_infos
      .iter()
      .map(|d| format!("{:?}", d.content_type.essence_str()))
      .collect(),
  );
  let size_width = calc_width(
    download_infos
      .iter()
      .map(|d| format!("{:.3}", d.size as f64 / (1024f64.powi(2))))
      .collect(),
  );
  let deployment_width = calc_width(
    download_infos
      .iter()
      .map(|d| format!("{:?}", d.target.deployment.to_string()))
      .collect(),
  );
  (
    name_width,
    filetype_width,
    mime_width,
    size_width,
    deployment_width,
  )
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
      let _ = get_github_download_links(&repo, &deployment, "latest").await;
    }
  }
}
