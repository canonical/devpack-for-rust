#![cfg(target_os = "linux")]

use clap::Parser;
use console::Key;
use eyre::{ContextCompat, eyre};
use log::warn;
use treeversal::console_driver::{ConsoleDriver, Palette, TakeInput};

use crate::{shell_type::ShellType, wizard::InstallWizard};

mod recipe;
mod selection_tree;
mod shell_type;
mod wizard;

/// an easy installer for rustup, IDEs, and rusty accessories
///
/// devpack-for-rust is an "install wizard" to make it easy to
/// install a whole Rust development environment.
/// You can install a Rust version via rustup, an IDE, and many
/// rust-based command line utilities.
///
/// The wizard presents a tree interface. Navigate it with HJKL
/// or the arrow keys.
/// Use space or enter to (un)pick an option.
#[derive(Default, Parser)]
#[command(version, about)]
#[command(after_long_help = "\
Development happens at https://github.com/canonical/devpack-for-rust

SNAP CAVEATS

The tools that devpack-for-rust installs are not integrated into the snap lifecycle.
Specifically:
- Uninstalling this snap does not uninstall the tools
- Refreshing this snap does not update the tools

TROUBLESHOOTING

> The program bails because it is missing a linker!

Many Linux distributions do not come with a linker by default.
On Ubuntu/Debian, you can get a linker with `sudo apt install build-essential`.
There are too many Linux distributions to document how to get a linker on each of them
in these docs; a web search is your friend.

> Some of the tools were installed, but I cannot find them from the command line!

You need to add `~/.cargo/bin` to your $PATH.
")]
pub struct Cli {
  #[arg(short = 'd', long)]
  /// Print what will be done, but don't actually execute any commands.
  pub dry_run: bool,
  /// devpack-for-rust tries to guess the shell you are using so it
  /// can properly create aliases.
  /// It does this by recursively checking the parent process and
  /// seeing if its `argv[0]` matches a known shell.
  ///
  /// Use this flag to override the shell detection and instead create
  /// aliases in the given shell's format.
  #[arg(short = 'S', long)]
  pub override_shell_type: Option<ShellType>,
  /// Override the automatic user detection and install the programs under the given user.
  ///
  /// Devpack-for-rust must be run as root, but some installers like `rustup` need to be run as you.
  /// Normally the crate guesses this by checking `$SUDO_USER`, but you can override it here.
  #[arg(short = 'U', long)]
  pub override_user: Option<String>,

  /// Skip the automatic check for a linker.
  ///
  /// A linker is required to compile Rust programs;
  /// this includes anything you might write *and* the programs
  /// that devpack-for-rust uses `cargo install` to install.
  ///
  /// devpack-for-rust does a simple check to see if `cc` is in
  /// the path and bails if it cannot find it.
  /// If your particular system provides a linker in some other way,
  /// you can override the check here.
  #[arg(long)]
  pub skip_linker_check: bool,

  /// Usually, if one install step fails the whole program halts.
  /// Use this flag to override this behavior.
  ///
  ///  Note this may cause cascading failures, as some steps depend on other steps.
  #[arg(long)]
  pub continue_after_failure: bool,

  #[command(flatten)]
  pub verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

fn main() -> eyre::Result<()> {
  let settings = Cli::parse();
  env_logger::Builder::new()
    .filter_level(settings.verbosity.into())
    .init();

  if sudo::check() == sudo::RunningAs::User {
    // TODO: using bail! or otherwise returning Err(_) makes the program print the line number
    // We want this for other errors, but it looks kind of messy in this case.
    // std::process::exit is a bit of an antipattern, but I'm not sure how to return
    // a non-zero exit code and also not print line info.
    eprintln!("Please run devpack-for-rust through sudo.");
    std::process::exit(1)
  }

  if !settings.skip_linker_check && which::which("cc").is_err() {
    eprintln!("You do not appear to have a linker installed!");
    eprintln!("Please install a linker before continuing.");
    eprintln!("(You can skip this check with `--skip-cc-check`)");
    std::process::exit(1)
  }

  println!("Welcome to devpack-for-rust! Run with --help for more information.");

  let shell_type = match settings.override_shell_type {
    Some(it) => Ok(it),
    None => ShellType::guess_shell().map_err(|_| ()),
  };
  // it would be more correct to do this as an OsString not a String
  // but the `homedir` crate only accepts strings for some reason
  let real_user = match settings.override_user {
    Some(ref it) => it.clone(),
    None => {
      let sudo_user = std::env::var_os("SUDO_USER")
        .context("$SUDO_USER was not set (are you running this via sudo?)")?;
      sudo_user
        .into_string()
        .map_err(|_| eyre!("could not convert $SUDO_USER to a string"))?
    }
  };

  println!("You appear to be the user {}", &real_user);
  if let Ok(shell_type) = shell_type {
    println!("Your shell has been autodetected as: {:?}", shell_type);
  } else {
    println!("Your shell could not be autodetected.")
  }

  let tree = selection_tree::make_tree();
  let mut console_driver = ConsoleDriver::new_stdout(Palette::fancy(), tree);
  console_driver.print_tree();

  while let Ok(key) = console_driver.term.read_key() {
    if key == Key::CtrlC {
      break;
    }
    let res = console_driver.take_input(key);
    if let Ok(TakeInput::Quit) = res {
      break;
    }
    console_driver.print_tree();
  }

  let selected = console_driver.interactor.get_all_selected_data();
  let selected_recipes = selected.iter().map(|smad| &smad.data).collect::<Vec<_>>();

  let mut install_wizard = InstallWizard::new(settings, shell_type, real_user);
  for recipe in selected_recipes.iter() {
    'steps: for step in recipe.steps.iter() {
      let res = install_wizard.execute_step(step);
      if let Err(oh_no) = res {
        if install_wizard.continue_after_failure {
          warn!("A recipe failed with the following error: {:?}", oh_no);
          // the further steps of this recipe don't make sense,
          // but nevertheless try to continue with the user's other demands
          warn!("Continuing with further recipes as requested.");
          break 'steps;
        } else {
          // quit!
          return Err(oh_no);
        }
      }
    }
  }

  println!("All done! Enjoy your devpack-for-rust!");

  Ok(())
}
