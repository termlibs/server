use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection
    , ParseFilter, Runtime, Value, ValueView,
};
use shell_quote::{Bash, Quote};
use std::path::PathBuf;
use std::sync::LazyLock;

const TEMPLATE: &str = include_str!("../static/install-linux.liquid");
const TEMPLATE_PATH: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("./static/install-linux.liquid"));

pub async fn template_install_script(options: &liquid::Object) -> anyhow::Result<String> {
    let parser = liquid::ParserBuilder::with_stdlib()
        .filter(ShellEscape)
        .build()?;
    let mut template_string = TEMPLATE.to_string(); 
    if TEMPLATE_PATH.as_path().exists() {
         let t = tokio::fs::read(TEMPLATE_PATH.as_path()).await?;
         template_string = String::from_utf8(t)?;
    }
    let template = parser.parse(&template_string)?;
    template.render(options).map_err(anyhow::Error::from)
}

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "escape_shell",
    description = "Takes in a string and escapes if for use in shell scripts",
    parsed(ShellEscapeFilter)
)]
pub struct ShellEscape;

#[derive(Debug, Default, Display_filter)]
#[name = "escape_shell"]
pub struct ShellEscapeFilter {}

impl Filter for ShellEscapeFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> liquid_core::Result<Value> {
        let to_escape = input.to_kstr();
        let escaped: Vec<u8> = Bash::quote(to_escape.as_str());
        Ok(Value::scalar(String::from_utf8(escaped).unwrap()))
    }
}

pub fn template_string(template: &str, globals: liquid::Object) -> anyhow::Result<String> {
    let parser = liquid::ParserBuilder::with_stdlib()
        .filter(ShellEscape)
        .build()?;
    let template = parser.parse(template)?;
    template.render(&globals).map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_bash_quote() {
        let template = include_str!("../static/install-linux.liquid");
        let parser = liquid::ParserBuilder::with_stdlib()
            .filter(ShellEscape)
            .build()
            .expect("should succeed without partials");
        let template = parser.parse(template).unwrap();

        let globals = liquid::object!(
            {
                "file_url": "https://github.com/mikefarah/yq/releases/download/v4.30.2/yq_linux_amd64?download=true&version=v4.30.2"
            }
        );
        let output = template.render(&globals).unwrap();
        println!("{:}", output);
    }

    #[test]
    fn test_shell_escape() {
        let template = include_str!("../static/install-linux.liquid");
        let to_quote = "
        ```bash
        sudo apt-get update
        sudo apt-get install -y curl
        ```
        ";
        let out = liquid_core::call_filter!(ShellEscape, to_quote,).unwrap();
        println!("{:?}", out);
        assert_eq!(
            out.to_kstr().as_str(),
            String::from_utf8(Bash::quote(to_quote)).unwrap().as_str()
        );
    }
}
