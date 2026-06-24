use eyre::{Context, bail};

use crate::wizard::InstallWizard;

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
  }, // Command(Vec<String>),
}

impl InstallStep {
  pub fn execute(&self, driver: &mut InstallWizard) -> eyre::Result<()> {
    match self {
      InstallStep::NoOp => {
        // that was easy
        Ok(())
      }
      InstallStep::RustChannel(channel) => {
        let rustup_path = match which::which("rustup") {
          Ok(it) => it,
          Err(_) => {
            if driver.dry_run {
              println!(
                "[devpack-for-rust] did not find rustup, but we are dry-running, so it's okay"
              );
              return Ok(());
            } else {
              eprintln!("[devpack-for-rust] did not find rustup, installing it with snap ...");
              driver.install_snap("rustup", true)?;
              which::which("rustup")
                .wrap_err("even after `snap install`-ing rustup, could not find it")?
            }
          }
        };
        println!(
          "[devpack-for-rust] Using rustup to install rust channel {:?} ...",
          &channel
        );
        let rustup_status = driver.maybe_dry_run_command(rustup_path, &["default", channel])?;
        if !rustup_status.success() {
          bail!("rustup invocation failed with error code {}", rustup_status);
        }
        Ok(())
      }
      InstallStep::Apt(pkg_name) => driver.install_apt(pkg_name),
      InstallStep::Snap {
        package_name,
        classic_confinement,
      } => driver.install_snap(package_name, *classic_confinement),
      InstallStep::MakeAlias { name, command } => {
        println!("[devpack-for-rust] TEMP: aliasing {}={}", name, command);
        Ok(())
      }
    }
  }
}
