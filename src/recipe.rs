#[derive(Clone, Debug)]
pub struct InstallRecipe {
  pub steps: Vec<InstallStep>,
}

impl InstallRecipe {
  pub fn new(steps: Vec<InstallStep>) -> Self {
    Self { steps }
  }

  pub fn onestep(step: InstallStep) -> InstallRecipe {
    Self::new(vec![step])
  }

  pub fn noop() -> InstallRecipe {
    InstallRecipe::onestep(InstallStep::NoOp)
  }
}

#[derive(Clone, Debug)]
pub enum InstallStep {
  /// For header nodes, etc
  NoOp,
  RustChannel(String),
  Apt(String),
  Snap {
    package_name: String,
    classic_confinement: bool,
  },
  /// `alias name=command`
  MakeAlias {
    name: String,
    command: String,
  },
  // Command(Vec<String>),
}
