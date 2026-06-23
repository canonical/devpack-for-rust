use std::{
  ffi::OsStr,
  process::{Command, ExitStatus},
};

use eyre::bail;

use crate::Cli;

pub struct InstallWizard {
  pub dry_run: bool,
  pub continue_after_failure: bool,
  already_ran_apt_update: bool,
}

impl InstallWizard {
  /// Initialize the driver from the CLI args.
  pub fn new(settings: Cli) -> Self {
    Self {
      already_ran_apt_update: false,
      dry_run: settings.dry_run,
      continue_after_failure: settings.continue_after_failure,
    }
  }

  /// Run a command, block, and return the exit code.
  ///
  /// If `dry_run` is set, print the command but don't actually do it.
  ///
  /// This should be the only place in the entire program that calls `Command::new` (to make sure that we don't accidentally run things when dry-running)
  pub fn maybe_dry_run_command(
    &self,
    cmd: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
  ) -> eyre::Result<ExitStatus> {
    let cmd = cmd.as_ref();
    let args = args.into_iter().map(AsRef::as_ref).collect::<Vec<_>>();

    if self.dry_run {
      println!(
        "[devpack-for-rust] not running due to --dry-run: {} {:?}",
        cmd.display(),
        &args
      );
      // default impl is success
      Ok(ExitStatus::default())
    } else {
      let mut command = Command::new(cmd);
      command.args(args);
      command.status().map_err(Into::into)
    }
  }

  pub fn install_apt(&mut self, pkg_name: &str) -> eyre::Result<()> {
    println!("[devpack-for-rust] Using apt to install {:?} ...", pkg_name);
    if !self.already_ran_apt_update {
      let apt_status = self.maybe_dry_run_command("apt", &["update"])?;
      if !apt_status.success() {
        bail!(
          "apt update invocation failed with error code {}",
          apt_status
        );
      }
      self.already_ran_apt_update = true;
    }

    let apt_status = self.maybe_dry_run_command("apt", &["install", pkg_name])?;
    if !apt_status.success() {
      bail!(
        "apt install invocation failed with error code {}",
        apt_status
      );
    }
    Ok(())
  }

  pub fn install_snap(&self, package_name: &str, classic_confinement: bool) -> eyre::Result<()> {
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
    let mut args = vec!["install", package_name];
    if classic_confinement {
      args.push("--classic");
    }

    let status = self.maybe_dry_run_command("snap", &args)?;
    if !status.success() {
      bail!("snap invocation failed with error code {}", status);
    }
    Ok(())
  }
}
