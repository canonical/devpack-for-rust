use std::{
  ffi::OsStr,
  fs::{File, OpenOptions},
  io::Write,
  os::unix::process::CommandExt,
  process::ExitStatus,
};

use eyre::{Context, ContextCompat, OptionExt, bail};
use log::{info, trace};
use users::os::unix::UserExt;

use crate::{Cli, recipe::InstallStep, shell_type::ShellType};

pub struct InstallWizard {
  pub dry_run: bool,
  pub continue_after_failure: bool,
  pub shell_type: Result<ShellType, ()>,
  pub real_user: String,

  config_file_handle: Option<File>,
  cached_real_user: Option<users::User>,
}

impl InstallWizard {
  /// Initialize the driver from the CLI args.
  pub fn new(settings: Cli, shell_type: Result<ShellType, ()>, real_user: String) -> Self {
    Self {
      dry_run: settings.dry_run,
      continue_after_failure: settings.continue_after_failure,
      shell_type,
      real_user,

      config_file_handle: None,
      cached_real_user: None,
    }
  }

  pub fn execute_step(&mut self, step: &InstallStep) -> eyre::Result<()> {
    match step {
      InstallStep::NoOp => {
        // that was easy
        Ok(())
      }
      InstallStep::RustChannel(channel) => {
        info!("Using rustup to install rust channel {:?} ...", &channel);
        let rustup_status_1 = self.maybe_dry_run_command("rustup", &["default", channel], true);
        let rustup_status_1_ok = match rustup_status_1 {
          // This probably means it could not execute the command
          Err(_) => false,
          Ok(code) if code.success() => true,
          Ok(code) => bail!("bad status code {} when invoking rustup", code),
        };
        if !rustup_status_1_ok {
          // this is the code snap returns if it can't find rustup
          if self.dry_run {
            info!("did not find rustup, but we are dry-running, so it's okay");
            return Ok(());
          }
          info!("did not find rustup, installing it with snap ...");
          self.install_snap("rustup", true)?;
          let rustup_status_2 =
            self.maybe_dry_run_command("rustup", &["default", channel], true)?;
          if !rustup_status_2.success() {
            bail!("could not snap run rustup even after snap installing it");
          }
        }
        Ok(())
      }
      InstallStep::CargoInstall(pkg_name) => {
        info!("Using cargo to install {:?} ...", pkg_name);
        // --locked makes sure it uses *exactly* the versions listed
        // in the crate's Cargo.lock,
        // ie exactly what the dev tested with
        let cargo_status =
          self.maybe_dry_run_command("cargo", &["install", "--locked", pkg_name], true)?;
        if !cargo_status.success() {
          bail!(
            "cargo install invocation failed with error code {}",
            cargo_status
          );
        }
        Ok(())
      }
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

        let home_dir = homedir::home(&self.real_user)?
          .ok_or_eyre("the given user did not have a home directory?")?;
        let alias = shell.format_alias(name, command);
        let cfg_file_loc = home_dir.join(shell.config_file_location());
        info!(
          "writing following alias in file {}:\n{}",
          &cfg_file_loc.display(),
          &alias,
        );

        if self.dry_run {
          return Ok(());
        }

        let file = match self.config_file_handle {
          Some(ref mut it) => it,
          None => {
            let mut file = OpenOptions::new()
              .create(true)
              .append(true)
              .open(&cfg_file_loc)
              .context(format!(
                "trying to open config file at {}",
                cfg_file_loc.display()
              ))?;
            writeln!(
              &mut file,
              "\n\n{}devpack-for-rust aliases",
              shell.comment_sigil()
            )?;
            self.config_file_handle.insert(file)
          }
        };

        writeln!(file, "{}", alias)?;
        Ok(())
      }
    }
  }

  /// Run a command, block, and return the exit code.
  ///
  /// If `dry_run` is set, print the command but don't actually do it.
  ///
  /// This should be the only place in the entire program that calls `Command::new`
  /// (to make sure that we don't accidentally run things when dry-running)
  fn maybe_dry_run_command(
    &mut self,
    cmd: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
    drop_to_user: bool,
  ) -> eyre::Result<ExitStatus> {
    use std::process::Command;

    let cmd = cmd.as_ref().to_os_string();
    let args = args
      .into_iter()
      .map(|s| s.as_ref().to_os_string())
      .collect::<Vec<_>>();

    trace!(
      "{} command{}: {} {:?}",
      if self.dry_run {
        "dry-\"running\""
      } else {
        "running"
      },
      if drop_to_user {
        " (dropping privileges)"
      } else {
        ""
      },
      &cmd.display(),
      &args
    );

    if self.dry_run {
      return Ok(ExitStatus::default());
    }

    let mut process = Command::new(cmd);
    process.args(args);
    if drop_to_user {
      let real_user_info = match self.cached_real_user {
        Some(ref it) => {
          trace!("above command run as uid {}", it.uid());
          it
        }
        None => {
          let real_user = users::get_user_by_name(&self.real_user).context(format!(
            "no user with name '{}' could be found",
            &self.real_user
          ))?;
          &*self.cached_real_user.insert(real_user)
        }
      };

      process.uid(real_user_info.uid());
      process.env("USER", real_user_info.name());
      process.env("HOME", real_user_info.home_dir());
    }

    let status = process.status().context("while running command")?;
    Ok(status)
  }

  /// Install a snap using `sudo snap install`
  fn install_snap(&mut self, package_name: &str, classic_confinement: bool) -> eyre::Result<()> {
    info!(
      "using snap to install {:?}{} ...",
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

    let status = self.maybe_dry_run_command("snap", &args, false)?;
    if !status.success() {
      bail!("snap invocation failed with error code {}", status);
    }
    Ok(())
  }
}
