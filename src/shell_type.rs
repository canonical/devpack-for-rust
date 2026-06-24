use std::path::PathBuf;

use clap::ValueEnum;
use eyre::{Context, bail, eyre};
use procfs::process::Process;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ShellType {
  /// Write alias to `~/.bash_aliases`
  Bash,
  /// Write alias to `~/.profile`
  Posix,
  /// Write alias to `~/.config/fish/functions/devpack-for-rust.fish`
  Fish,
  /// Write alias to `~/.zshrc`
  Zsh,
}

impl ShellType {
  /// Guess the shell the user is using by recursively scanning the
  /// process's parents until it finds a command it recognizes as a shell.
  pub fn guess_shell() -> eyre::Result<ShellType> {
    let mut the_process = Process::myself().wrap_err("couldn't get my own PID")?;

    loop {
      let the_shell = ShellType::guess_shell_of_pid(&the_process)?;
      if let Some(the_shell) = the_shell {
        return Ok(the_shell);
      }

      let parent_pid = the_process.stat()?.ppid;
      if parent_pid == 1 {
        bail!("reached the root process");
      }
      let parent_process = Process::new(parent_pid)?;
      the_process = parent_process;
    }
  }

  /// Guess the shell the user is using by checking the CLI of the
  /// invoking process.
  fn guess_shell_of_pid(process: &Process) -> eyre::Result<Option<ShellType>> {
    // use `cmdline` instead of `exe` because some shells
    // are polyglots; we care mostly how the USER invoked this.
    let proc_cli = process.cmdline()?;
    let proc_cmd = proc_cli
      .get(0)
      .ok_or(eyre!("process somehow did not have a 0th argument"))?;

    // > ... if the pathname has been unlinked,
    // > the symbolic link will contain the string “ (deleted)“
    // > appended to the original pathname.
    // From procfs docs.
    if proc_cmd.ends_with(" (deleted)") {
      bail!("parent process was terminated")
    }

    let proc_path = PathBuf::from(proc_cmd);
    let Some(filename) = proc_path.file_name() else {
      bail!("process somehow did not have a file name")
    };

    // unfortunately OsStr does not like `match`
    // please don't use esoteric but posix-compliant shells
    let ty = if filename == "bash" {
      ShellType::Bash
    } else if filename == "dash" || filename == "sh" || filename == "ksh" || filename == "csh" {
      ShellType::Posix
    } else if filename == "fish" {
      ShellType::Fish
    } else if filename == "zsh" {
      ShellType::Zsh
    } else {
      return Ok(None);
    };
    Ok(Some(ty))
  }

  pub fn config_file_location(&self) -> PathBuf {
    match self {
      ShellType::Bash => unwrap_home_dir().join(".bash_aliases"),
      // TODO: is this actually-actually portable?
      // who uses sh anyways
      ShellType::Posix => unwrap_home_dir().join(".profile"),
      ShellType::Zsh => unwrap_home_dir().join("~/.zshrc"),
      // fish is special
      ShellType::Fish => {
        let cfg_dir = dirs::config_dir().unwrap_or_else(|| {
          eprintln!("[devpack-for-rust] couldn't find config dir, defaulting to ~/.config");
          unwrap_home_dir().join(".config")
        });
        cfg_dir.join("fish/functions/devpack-for-rust.fish")
      }
    }
  }

  /// Yes, this always returns `# ` right now.
  /// You never know.
  pub fn comment_sigil(&self) -> &'static str {
    match self {
      ShellType::Bash | ShellType::Posix | ShellType::Fish | ShellType::Zsh => "# ",
    }
  }

  pub fn format_alias(&self, name: &str, command: &str) -> String {
    match self {
      ShellType::Bash | ShellType::Posix | ShellType::Zsh => format!("alias {name}=\"{command}\""),
      // This is what fish writes when using the `alias` command
      // https://fishshell.com/docs/current/cmds/alias.html
      ShellType::Fish => format!(
        "function {name} --wraps {command} --description 'alias {name}={command}'\n\
          {command} $argv\n\
        end"
      ),
    }
  }
}

fn unwrap_home_dir() -> PathBuf {
  dirs::home_dir().expect("could not find your home directory somehow")
}
