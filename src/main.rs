#![cfg(target_os = "linux")]

use clap::Parser;
use console::Key;
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

  #[command(flatten)]
  pub verbosity: clap_verbosity_flag::Verbosity,
}

fn main() -> eyre::Result<()> {
  let settings = Cli::parse();
  env_logger::Builder::new()
    .filter_level(settings.verbosity.into())
    .init();

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

  let mut install_wizard = InstallWizard::new(settings);
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
