pub(crate) trait QueryOptions {
  fn to_args(&self) -> String;
}

#[derive(Debug, PartialEq, FromForm)]
pub struct InstallQueryOptions {
  version: Option<String>,
  prefix: Option<String>,
  app: Option<String>,
}

impl QueryOptions for InstallQueryOptions {
  fn to_args(&self) -> String {
    format!("--version=\"{}\" --prefix=\"{}\" \"{}\"", self.version(), self.prefix(), self.app())
  }
}

impl InstallQueryOptions {
  pub(crate) fn version(&self) -> String {
    self.version.clone().unwrap_or("latest".to_string())
  }

  pub(crate) fn prefix(&self) -> String {
    self.prefix.clone().unwrap_or("$HOME/.local".to_string())
  }

  pub(crate) fn app(&self) -> String {
    self.app.clone().unwrap()
  }

  pub(crate) fn is_none(&self) -> bool {
    self.prefix.clone().is_none() && self.version.clone().is_none()
  }

  pub(crate) fn is_some(&self) -> bool {
    !self.is_none().clone()
  }

  pub(crate) fn set_app(&mut self, app: String) {
    self.app = Some(app);
  }
}
