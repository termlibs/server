use std::env;
use std::env::join_paths;
use std::error::Error;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::string::ToString;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use crate::TERMLIBS_ROOT;
use crate::types::{QueryOptions, InstallQueryOptions};

const APP_FILES: [(&str, &str); 2] = [
  ("install", "install.sh/scripts/install_all.sh"),
  ("json", "json.sh/bin/json.sh"),
];

pub(crate) async fn create_install_script<T: QueryOptions + Debug>(args: Option<T>) -> Result<String, Box<dyn Error>> {
  let script_path = APP_FILES.iter().find(|(x, _)| x == &"install").unwrap().1;
  let filepath: PathBuf = PathBuf::from(TERMLIBS_ROOT.deref()).join(PathBuf::from(script_path));
  let argstring = args.as_ref().unwrap().to_args();

  info!("Opening {:?} with arguments {:?}", filepath, argstring);
  let mut script = File::open(filepath).await?;
  let mut data: String = String::new();
  let _ = script.read_to_string(&mut data).await?;
  let mut lines: Vec<String> = vec![];

  match args {
    Some(a) => {
      for (line_no, line) in data.split_terminator("\n").enumerate() {
        lines.push(line.to_string());
        if line.starts_with("#!") && line_no == 0 {
          lines.push(
            format!("eval set -- {} \"$@\"", a.to_args())
          )
        }
      }
    }
    None => lines = data.split_terminator("\n").map(|s| s.to_string()).collect()
  }
  Ok(lines.join("\n"))
}
