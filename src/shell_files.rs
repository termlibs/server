use std::error::Error;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub(crate) async fn open_file(path: impl AsRef<Path>, args: Option<String>) -> Result<String, Box<dyn Error>> {
  let mut dot_sh = File::open(path).await?;
  let mut data:  String= String::new();
  let _d = dot_sh.read_to_string(&mut data);
  let mut lines: Vec<String> = vec![];

  match args {
    Some(a) => {
      for (line_no, line) in data.split_terminator("\n").enumerate() {
        lines.push(line.to_string());
        if line.starts_with("#!") && line_no == 0 {
          lines.push(
            format!("eval set -- {} \"$@\"", a)
          )
        }
      }
    }
    None => lines = data.split_terminator("\n").map(|s| s.to_string()).collect()
  }
  Ok(lines.join("\n"))
}
