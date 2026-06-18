use std::process::Command;

use eyre::{Context, bail};

use crate::DevpackSettings;

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
  pub fn execute(&self, settings: &DevpackSettings) -> eyre::Result<()> {
    match self {
      InstallStep::NoOp => {
        // that was easy
        Ok(())
      }
      InstallStep::RustChannel(channel) => {
        let rustup_path = match which::which("rustup") {
          Ok(it) => it,
          Err(_) => {
            eprintln!("[devpack-for-rust] rustup was not found in path, installing via snap ...");
            install_snap("rustup", true, settings)?;
            which::which("rustup")
              .wrap_err("even after `snap install`-ing rustup, could not find it in the path")?
          }
        };

        println!(
          "[devpack-for-rust] Using rustup to install rust channel {:?} ...",
          &channel
        );
        let rustup_status = Command::new(rustup_path)
          .arg("default")
          .arg(&channel)
          .status()?;
        if !rustup_status.success() {
          bail!("rustup invocation failed with error code {}", rustup_status);
        }
        Ok(())
      }
      InstallStep::Apt(pkg_name) => install_apt(pkg_name),
      InstallStep::Snap {
        package_name,
        classic_confinement,
      } => install_snap(package_name, *classic_confinement, settings),
      InstallStep::MakeAlias { name, command } => todo!(),
    }
  }
}

fn install_apt(pkg_name: &String) -> Result<(), eyre::Error> {
  println!("[devpack-for-rust] Using apt to install {:?} ...", pkg_name);
  let apt_status = Command::new("apt").arg("update").status()?;
  if !apt_status.success() {
    bail!(
      "apt update invocation failed with error code {}",
      apt_status
    );
  }

  let apt_status = Command::new("apt").arg("install").arg(pkg_name).status()?;
  if !apt_status.success() {
    bail!(
      "apt install invocation failed with error code {}",
      apt_status
    );
  }
  Ok(())
}

fn install_snap(
  package_name: &str,
  classic_confinement: bool,
  settings: &DevpackSettings,
) -> eyre::Result<()> {
  println!(
    "[devpack-for-rust] using snap to install {:?}{}",
    package_name,
    if classic_confinement {
      " with --classic confinement"
    } else {
      ""
    }
  );
  // need to bind the cmd to a variable to appease borrowck
  let mut cmd = Command::new("snap");
  cmd.arg("install").arg(package_name);
  if classic_confinement {
    cmd.arg("--classic");
  }

  let status = cmd.status()?;
  if !status.success() {
    bail!("snap invocation failed with error code {}", status);
  }
  Ok(())
}
