#![cfg(target_os = "linux")]

use clap::Parser;
use console::Key;
use eyre::{ContextCompat, bail, eyre};
use log::warn;
use treeversal::console_driver::{ConsoleDriver, Palette, TakeInput};

use crate::{shell_type::ShellType, wizard::InstallWizard};

mod recipe;
mod selection_tree;
mod shell_type;
mod wizard;

/// Devpack for Rust -- an easy installer for Rustup, IDEs, and Rusty accessories.
#[derive(Default, Parser)]
#[command(version, about)]
pub struct Cli {
  /// If one recipe fails, choose whether to try and execute the other recipes.
  #[arg(short = 'C', long)]
  pub continue_after_failure: bool,
  #[arg(short = 'd', long)]
  /// Print what will be done, but don't actually execute any commands.
  pub dry_run: bool,
  /// Override the automatic shell detection and use the given shell type.
  /// This is used to figure out how to make aliases for your shell.
  #[arg(short = 'S', long)]
  pub override_shell_type: Option<ShellType>,
  /// Override the automatic user detection and install the programs under the given user.
  ///
  /// Devpack-for-rust must be run as root, but some installers like `rustup` need to be run as you.
  /// Normally the crate guesses this by checking `$SUDO_USER`, but you can override it here.
  #[arg(short = 'U', long)]
  pub override_user: Option<String>,

  #[command(flatten)]
  pub verbosity: clap_verbosity_flag::Verbosity,
}

fn main() -> eyre::Result<()> {
  let settings = Cli::parse();
  env_logger::Builder::new()
    .filter_level(settings.verbosity.into())
    .init();

  if sudo::check() == sudo::RunningAs::User {
    bail!("Please run devpack-for-rust through sudo.");
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
