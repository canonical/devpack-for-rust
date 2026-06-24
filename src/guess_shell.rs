use eyre::{Context, bail};
use os_str_bytes::OsStrBytesExt;
use procfs::process::Process;

#[derive(Clone, Copy, Debug)]
pub enum ShellType {
  /// POSIX-compatible shell, like bash, sh, csh, etc
  Posix,
  /// The Friendly Interactive Shell
  Fish,
  /// Zshell
  Zsh,
  /// There was some problem when trying to guess the shell type;
  /// don't make aliases.
  /// This is never returned by the functions that guess the shell
  /// type from the process.
  Err,
}

impl ShellType {
  /// Guess the shell the user is using by recursively scanning the
  /// process's parents until it finds a command it recognizes as a shell.
  pub fn guess_shell() -> ShellType {
    match ShellType::guess_shell_inner_because_we_still_dont_have_try_blocks() {
      Ok(it) => it,
      Err(oh_no) => {
        eprintln!("[devpack -for-rust] failed to guess user shell: {}", oh_no);
        ShellType::Err
      }
    }
  }

  fn guess_shell_inner_because_we_still_dont_have_try_blocks() -> eyre::Result<ShellType> {
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
    let proc_cli = process.exe()?;

    // > ... if the pathname has been unlinked,
    // > the symbolic link will contain the string “ (deleted)“
    // > appended to the original pathname.
    // From procfs docs.
    if proc_cli.as_os_str().ends_with(" (deleted)") {
      bail!("parent process was terminated")
    }

    let Some(filename) = proc_cli.file_name() else {
      bail!("parent process somehow did not have a file name")
    };

    // unfortunately OsStr does not like `match`
    // please don't use esoteric but posix-compliant shells
    let ty = if filename == "bash" || filename == "sh" || filename == "ksh" || filename == "csh" {
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
}
