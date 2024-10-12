use crate::types::QueryOptions;
use crate::TERMLIBS_ROOT;
use std::error::Error;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::PathBuf;
use std::string::ToString;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const APP_FILES: [(&str, &str); 2] = [
    ("install", "install.sh/scripts/main.sh"),
    ("json", "json.sh/bin/json.sh"),
];

pub(crate) async fn create_install_script<T: QueryOptions + Debug>(
    args: Option<T>,
) -> Result<String, Box<dyn Error>> {
    let script_path = APP_FILES.iter().find(|(x, _)| x == &"install").unwrap().1;
    let filepath: PathBuf = PathBuf::from(&TERMLIBS_ROOT.deref()).join(PathBuf::from(script_path));
    let mut script = File::open(filepath).await?;
    let mut data: String = String::new();
    let _ = script.read_to_string(&mut data).await?;
    let mut lines: Vec<String> = vec![];
    info!("Opening script {:?}", TERMLIBS_ROOT);

    match args {
        Some(a) => {
            info!("Adding args {:?}", a.to_args());
            let mut inserted = false;
            for line in data.split_terminator("\n") {
                if !inserted {
                    if line.starts_with("#") {
                        lines.push(line.to_string());
                        continue;
                    }
                    if line.starts_with("eval") {
                        lines.push(format!("eval set -- {} \"$@\"", a.to_args()));
                    } else {
                        lines.push(format!("eval set -- {} \"$@\"", a.to_args()));
                        lines.push(line.to_string());
                    }
                    inserted = true;
                } else {
                    lines.push(line.to_string());
                }
            }
            Ok(lines.join("\n"))
        }
        None => Ok(data),
    }
}
