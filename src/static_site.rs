use comrak::{markdown_to_html, Options};
use std::collections::BTreeMap;

const STATIC_FILES: [(&str, &str); 2] = [
  ("index.html", include_str!("../static/index.md")),
  ("404.html", include_str!("../static/404.md")),
];
const HEAD: &str = concat!(
"<head><title>termlibs</title><style>",
include_str!("../static/style.css"),
"</style></head><body>"
);

pub(crate) fn wrap_body(html: &str) -> String {
  format!("{}{}</body>", HEAD, html)
}

pub(crate) fn load_static(key: &str) -> Option<String> {
  let tree = BTreeMap::from(STATIC_FILES);
  match tree.get(key) {
    Some(md) => {
      let html = markdown_to_html(md, &Options::default());
      let out = wrap_body(html.as_str());
      Some(out)
    }
    None => None,
  }
}
