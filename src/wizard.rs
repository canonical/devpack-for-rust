use std::{
  ffi::{OsStr, OsString},
  fs::{File, OpenOptions},
  io::Write,
  process::ExitStatus,
};

use eyre::{Context, OptionExt, bail};
use log::{info, trace};

use crate::{Cli, recipe::InstallStep, shell_type::ShellType};

pub struct InstallWizard {
  pub dry_run: bool,
  pub continue_after_failure: bool,
  pub shell_type: Result<ShellType, ()>,
  pub real_user: String,

  config_file_handle: Option<File>,
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
        let rustup_status_1 =
          self.maybe_dry_run_command("snap", &["run", "rustup", "default", channel], true)?;
        if rustup_status_1.code() == Some(1) {
          // this is the code snap returns if it can't find rustup
          if self.dry_run {
            info!("did not find rustup, but we are dry-running, so it's okay");
            return Ok(());
          }
          info!("did not find rustup, installing it with snap ...");
          self.install_snap("rustup", true)?;
          let rustup_status_2 =
            self.maybe_dry_run_command("snap", &["run", "rustup", "default", channel], true)?;
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
  /// This should be the only place in the entire program that calls `Command::new` (to make sure that we don't accidentally run things when dry-running)
  fn maybe_dry_run_command(
    &self,
    cmd: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
    drop_to_user: bool,
  ) -> eyre::Result<ExitStatus> {
    use std::process::Command;

    let (cmd, args) = if drop_to_user {
      let mut sudo_args = vec![
        OsString::from("--user"),
        OsString::from(&self.real_user),
        cmd.as_ref().to_os_string(),
      ];
      sudo_args.extend(
        args
          .into_iter()
          .map(|s| s.as_ref().to_os_string())
          .collect::<Vec<_>>(),
      );
      (OsString::from("sudo"), sudo_args)
    } else {
      (
        cmd.as_ref().to_os_string(),
        args
          .into_iter()
          .map(|s| s.as_ref().to_os_string())
          .collect::<Vec<_>>(),
      )
    };

    let run_verb = if self.dry_run {
      "dry-\"running\""
    } else {
      "running"
    };
    trace!("{} command: {} {:?}", run_verb, cmd.display(), &args);

    if self.dry_run {
      // default impl is success
      Ok(ExitStatus::default())
    } else {
      let mut command = Command::new(cmd);
      command.args(args);
      command.status().map_err(Into::into)
    }
  }

  /// Install a snap using `sudo snap install`
  fn install_snap(&self, package_name: &str, classic_confinement: bool) -> eyre::Result<()> {
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
