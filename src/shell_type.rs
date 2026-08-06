use std::{os::unix::ffi::OsStrExt, path::PathBuf};

use clap::ValueEnum;
use eyre::{Context, bail, eyre};
use log::trace;
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
    trace!("guessing if this is a shell? {:?}", &proc_cli);
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

    // https://superuser.com/questions/278859/dash-in-front-of-bash/278865#278865
    // > A login shell is one whose first character of argument zero is a -,
    // > or one started with the --login option.
    // in other words, `-coolshellname` == `coolshellname`
    let filename_bytes = filename.as_bytes();
    let filename_trimmed = match filename_bytes.strip_prefix(&[b'-']) {
      Some(it) => it,
      None => &filename_bytes,
    };

    let ty = match filename_trimmed {
      b"bash" => ShellType::Bash,
      b"dash" | b"sh" | b"ksh" | b"csh" => ShellType::Posix,
      b"fish" => ShellType::Fish,
      b"zsh" => ShellType::Zsh,
      _ => return Ok(None),
    };
    Ok(Some(ty))
  }

  /// Append this to the home directory.
  pub fn config_file_location(&self) -> PathBuf {
    match self {
      ShellType::Bash => PathBuf::from(".bash_aliases"),
      // TODO: is this actually-actually portable?
      // who uses sh anyways
      ShellType::Posix => PathBuf::from(".profile"),
      ShellType::Zsh => PathBuf::from(".zshrc"),
      // fish is special
      ShellType::Fish => {
        // it is technically a bad assumption to use ~/.config
        // as the fish config dir.
        // however, getting the xdg dirs of another user is EXTREMELY
        // difficult as it turns out
        PathBuf::from(".config/fish/functions/devpack-for-rust.fish")
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
