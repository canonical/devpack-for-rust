use console::Key;

use eyre::bail;
use treeversal::console_driver::{ConsoleDriver, Palette, TakeInput};

mod recipe;
mod selection_tree;

/// Passed around for install parameters.
#[derive(Default)]
struct DevpackSettings {
  /// If one recipe fails, choose whether to try and execute the other recipes.
  pub continue_after_failure: bool,
}

fn main() -> eyre::Result<()> {
  let privileges = sudo::check();
  if privileges != sudo::RunningAs::Root {
    // the `sudo` crate supports restarting the program as root,
    // but I would rather be explicit than implicit and require the user manually invoke sudo
    bail!("devpack-for-rust must be run as root.");
  }

  let settings = DevpackSettings::default();

  let tree = selection_tree::make_tree();
  let mut driver = ConsoleDriver::new_stdout(Palette::fancy(), tree);
  driver.print_tree();

  while let Ok(key) = driver.term.read_key() {
    if key == Key::CtrlC {
      break;
    }
    let res = driver.take_input(key);
    if let Ok(TakeInput::Quit) = res {
      break;
    }
    driver.print_tree();
  }

  let selected = driver.interactor.get_all_selected_data();
  let selected_recipes = selected.iter().map(|smad| &smad.data).collect::<Vec<_>>();

  for recipe in selected_recipes.iter() {
    'steps: for step in recipe.steps.iter() {
      let res = step.execute(&settings);
      if let Err(oh_no) = res {
        if settings.continue_after_failure {
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
