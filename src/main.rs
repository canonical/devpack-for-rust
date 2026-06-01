use console::Key;
use treeversal::console_driver::{ConsoleDriver, Palette, TakeInput};

mod make_tree;
mod recipe;

fn main() {
    let tree = make_tree::make_tree();
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
    println!("{:?}", &selected_recipes);
}
