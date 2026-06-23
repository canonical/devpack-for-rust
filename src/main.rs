use clap::Parser;
use console::Key;
use eyre::bail;
use treeversal::console_driver::{ConsoleDriver, Palette, TakeInput};

use crate::wizard::InstallWizard;

mod recipe;
mod selection_tree;
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
}

fn main() -> eyre::Result<()> {
  // Parse settings before the root check so `--help` and friends works
  let settings = Cli::parse();

  let privileges = sudo::check();
  if privileges != sudo::RunningAs::Root {
    // the `sudo` crate supports restarting the program as root,
    // but I would rather be explicit than implicit and require the user manually invoke sudo
    bail!("devpack-for-rust must be run as root.");
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

  let mut install_driver = InstallWizard::new(settings);
  for recipe in selected_recipes.iter() {
    'steps: for step in recipe.steps.iter() {
      let res = step.execute(&mut install_driver);
      if let Err(oh_no) = res {
        if install_driver.continue_after_failure {
          eprintln!("[devpack-for-rust] A recipe failed with the following error:");
          eprintln!("{:?}", oh_no);
          // the further steps of this recipe don't make sense,
          // but nevertheless try to continue with the user's other demands
          eprintln!("[devpack-for-rust] Continuing with further recipes as requested.");
          break 'steps;
        } else {
          // quit!
          return Err(oh_no);
        }
      }
    }
  }

  Ok(())
}
