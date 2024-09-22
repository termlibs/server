use std::collections::BTreeMap;
use comrak::{markdown_to_html, parse_document, Arena, Options};

const STATIC_FILES: [(&str, &str); 1] = [
  ("index.html", include_str!("../static/index.md")),
];
const HEAD: &str = concat!(
   "<head><title>termlibs</title><style>",
   include_str!("../static/style.css"),
  "</style></head><body>"
);

pub(crate) fn wrap_body(html: &str) -> String {
  format!("{}{}</body>", HEAD , html)
}

pub(crate) fn load_static(_path: &str) -> Option<String> {
  let tree = BTreeMap::from(STATIC_FILES);
  let md = tree.get("index.html");
  match md {
    Some(md) => {
      let html = markdown_to_html(md, &Options::default());
      let out = wrap_body(html.as_str());
      Some(out)
    }
    None => None,
  }
}
