use std::{
  ffi::OsStr,
  fs::{File, OpenOptions},
  io::Write,
  process::ExitStatus,
};

use eyre::{Context, bail};

use crate::{Cli, recipe::InstallStep, shell_type::ShellType};

pub struct InstallWizard {
  pub dry_run: bool,
  pub continue_after_failure: bool,
  pub shell_type: Result<ShellType, ()>,

  already_ran_apt_update: bool,
  config_file_handle: Option<File>,
}

impl InstallWizard {
  /// Initialize the driver from the CLI args.
  pub fn new(settings: Cli) -> Self {
    let shell_type = match settings.override_shell_type {
      Some(it) => Ok(it),
      None => match ShellType::guess_shell() {
        Ok(it) => {
          println!("[devpack-for-rust] autodetected shell type as {:?}", it);
          Ok(it)
        }
        Err(oh_no) => {
          eprintln!(
            "[devpack-for-rust] failed to detect shell type! {:?}",
            oh_no
          );
          Err(())
        }
      },
    };

    Self {
      dry_run: settings.dry_run,
      continue_after_failure: settings.continue_after_failure,
      shell_type,

      already_ran_apt_update: false,
      config_file_handle: None,
    }
  }

  pub fn execute_step(&mut self, step: &InstallStep) -> eyre::Result<()> {
    match step {
      InstallStep::NoOp => {
        // that was easy
        Ok(())
      }
      InstallStep::RustChannel(channel) => {
        let rustup_path = match which::which("rustup") {
          Ok(it) => it,
          Err(_) => {
            if self.dry_run {
              println!(
                "[devpack-for-rust] did not find rustup, but we are dry-running, so it's okay"
              );
              return Ok(());
            } else {
              eprintln!("[devpack-for-rust] did not find rustup, installing it with snap ...");
              self.install_snap("rustup", true)?;
              which::which("rustup")
                .wrap_err("even after `snap install`-ing rustup, could not find it")?
            }
          }
        };
        println!(
          "[devpack-for-rust] Using rustup to install rust channel {:?} ...",
          &channel
        );
        let rustup_status = self.maybe_dry_run_command(rustup_path, &["default", channel])?;
        if !rustup_status.success() {
          bail!("rustup invocation failed with error code {}", rustup_status);
        }
        Ok(())
      }
      InstallStep::Apt(pkg_name) => self.install_apt(pkg_name),
      InstallStep::Snap {
        package_name,
        classic_confinement,
      } => self.install_snap(package_name, *classic_confinement),
      InstallStep::MakeAlias { name, command } => {
        let Ok(shell) = self.shell_type else {
          bail!(
            "couldn't make alias {}={} because the shell type was not known",
            &name,
            &command
          )
        };

        let alias = shell.format_alias(name, command);
        if self.dry_run {
          println!(
            "[devpack-for-rust] skipping writing following alias in file {} due to dry run:\n{}",
            shell.config_file_location().display(),
            &alias,
          );
          return Ok(());
        }

        let file = match self.config_file_handle {
          Some(ref mut it) => it,
          None => {
            let path = shell.config_file_location();
            let mut file = OpenOptions::new()
              .create(true)
              .append(true)
              .open(&path)
              .context(format!("trying to open config file at {}", path.display()))?;
            writeln!(
              &mut file,
              "\n\n{}devpack-for-rust aliases",
              shell.comment_sigil()
            )?;
            self.config_file_handle.insert(file)
          }
        };

        file.write_all(alias.as_bytes())?;
        Ok(())
      }
    }
  }

  /// Run a command, block, and return the exit code.
  ///
  /// If `dry_run` is set, print the command but don't actually do it.
  ///
  /// This should be the only place in the entire program that calls `Command::new` (to make sure that we don't accidentally run things when dry-running)
  fn maybe_dry_run_command(
    &self,
    cmd: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
  ) -> eyre::Result<ExitStatus> {
    use std::process::Command;

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

  /// Install a package using `sudo apt install`.
  ///
  /// Also run `sudo apt update` the first time this function is called
  fn install_apt(&mut self, pkg_name: &str) -> eyre::Result<()> {
    println!("[devpack-for-rust] Using apt to install {:?} ...", pkg_name);
    if !self.already_ran_apt_update {
      let apt_status = self.maybe_dry_run_command("sudo", &["apt", "update"])?;
      if !apt_status.success() {
        bail!(
          "apt update invocation failed with error code {}",
          apt_status
        );
      }
      self.already_ran_apt_update = true;
    }

    let apt_status = self.maybe_dry_run_command("sudo", &["apt", "install", pkg_name])?;
    if !apt_status.success() {
      bail!(
        "apt install invocation failed with error code {}",
        apt_status
      );
    }
    Ok(())
  }

  /// Install a snap using `sudo snap install`
  fn install_snap(&self, package_name: &str, classic_confinement: bool) -> eyre::Result<()> {
    println!(
      "[devpack-for-rust] using snap to install {:?}{} ...",
      package_name,
      if classic_confinement {
        " with --classic confinement"
      } else {
        ""
      }
    );
    // need to bind the cmd to a variable to appease borrowck
    let mut args = vec!["snap", "install", package_name];
    if classic_confinement {
      args.push("--classic");
    }

    let status = self.maybe_dry_run_command("sudo", &args)?;
    if !status.success() {
      bail!("snap invocation failed with error code {}", status);
    }
    Ok(())
  }
}
