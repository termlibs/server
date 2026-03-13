use comrak::{markdown_to_html, Options};
use std::collections::BTreeMap;
use std::sync::LazyLock;

const STATIC_FILES: [(&str, &str); 2] = [
  ("index.html", include_str!("../static/index.md")),
  ("404.html", include_str!("../static/404.md")),
];
static STATIC_TREE: LazyLock<BTreeMap<&'static str, &'static str>> =
  LazyLock::new(|| BTreeMap::from(STATIC_FILES));
const HEAD: &str = concat!(
  "<head><title>termlibs</title><style>",
  include_str!("../static/style.css"),
  "</style></head><body>"
);

pub(crate) fn wrap_body(html: &str) -> String {
  format!("{}{}</body>", HEAD, html)
}

pub(crate) fn load_static(key: &str) -> Option<String> {
  match STATIC_TREE.get(key) {
    Some(md) => {
      let html = markdown_to_html(md, &Options::default());
      let out = wrap_body(html.as_str());
      Some(out)
    }
    None => None,
  }
}
