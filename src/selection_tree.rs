use treeversal::{
  NodeDefinitionType, TreeDefinition, TreeNodeDefinition, console_driver::StyledMsgAndData,
};

use crate::recipe::{InstallRecipe, InstallStep};

pub fn make_tree() -> TreeDefinition<StyledMsgAndData<InstallRecipe>> {
  let rust_version = TreeNodeDefinition::new(
    NodeDefinitionType::PickOneChild { mandatory: true },
    StyledMsgAndData {
      message: console::style("Pick a Rust channel".to_string()),
      data: InstallRecipe::noop(),
    },
  )
  .with_child(rust_channel("stable"))
  .with_child(rust_channel("beta"))
  .with_child(rust_channel("nightly"))
  .with_child(TreeNodeDefinition::new(
    NodeDefinitionType::Text,
    StyledMsgAndData::unstyled("DEBUG don't try to install rust", InstallRecipe::noop()),
  ));

  let ide = TreeNodeDefinition::new(
    NodeDefinitionType::PickOneChild { mandatory: true },
    StyledMsgAndData {
      message: console::style("Pick an IDE?".to_string()),
      data: InstallRecipe::noop(),
    },
  )
  .with_child(snap(
    "Helix: A post-modern text editor (https://helix-editor.com/)",
    "helix",
    true,
  ))
  .with_child(snap(
    "VSCode: Your home for multi-agent development (https://code.visualstudio.com/)",
    "code",
    true,
  ))
  .with_child(snap(
    "RustRover: JetBrains' powerful IDE for Rust (https://www.jetbrains.com/rust/)",
    "rustrover",
    true,
  ));
  // - rustrover
  // - zed
  // - can auto download vscode rust extension?

  let extras = TreeNodeDefinition::new(
    NodeDefinitionType::PickManyChildren,
    StyledMsgAndData {
      message: console::style("Oxidize your tooling?".to_string()),
      data: InstallRecipe::noop(),
    },
  )
  .with_child(apt(
    "du-dust: a more intuitive version of du (https://github.com/bootandy/dust)",
    "du-dust",
  ))
  .with_child(apt_with_alias(
    "fd-find: simple, fast and user-friendly alternative to 'find' (https://github.com/sharkdp/fd)",
    "fd-find",
    "fd",
    "fdfind",
  ))
  .with_child(apt_with_alias(
    "ripgrep: recursively search directories (https://github.com/BurntSushi/ripgrep)",
    "ripgrep",
    "rg",
    "ripgrep",
  ))
  .with_child(apt(
    "sd: intuitive find & replace cli (https://github.com/chmln/sd)",
    "sd",
  ));

  TreeDefinition::new(
    text("Customize your devpack-for-rust.")
      .with_child(rust_version)
      .with_child(ide)
      .with_child(extras)
      .with_child(TreeNodeDefinition::new(
        NodeDefinitionType::AllDone,
        StyledMsgAndData {
          message: console::style("All done?".to_string()),
          data: InstallRecipe::noop(),
        },
      )),
  )
}

fn text(msg: impl AsRef<str>) -> TreeNodeDefinition<StyledMsgAndData<InstallRecipe>> {
  TreeNodeDefinition::new(
    NodeDefinitionType::Text,
    StyledMsgAndData {
      message: console::style(msg.as_ref().to_owned()),
      data: InstallRecipe::noop(),
    },
  )
}

fn rust_channel(channel: impl AsRef<str>) -> TreeNodeDefinition<StyledMsgAndData<InstallRecipe>> {
  let channel = channel.as_ref();
  // TODO: custom channel?
  // TODO: is "channel" the right word for this
  TreeNodeDefinition::new(
    NodeDefinitionType::Text,
    StyledMsgAndData {
      data: InstallRecipe::onestep(InstallStep::RustChannel(channel.to_string())),
      message: console::style(channel.to_string()),
    },
  )
}

fn snap(
  blurb: &str,
  program: &str,
  classic_confinement: bool,
) -> TreeNodeDefinition<StyledMsgAndData<InstallRecipe>> {
  TreeNodeDefinition::new(
    NodeDefinitionType::Text,
    StyledMsgAndData {
      message: console::style(blurb.to_string()),
      data: InstallRecipe::onestep(InstallStep::Snap {
        package_name: program.to_string(),
        classic_confinement,
      }),
    },
  )
}

fn apt(blurb: &str, package_name: &str) -> TreeNodeDefinition<StyledMsgAndData<InstallRecipe>> {
  TreeNodeDefinition::new(
    NodeDefinitionType::Text,
    StyledMsgAndData {
      message: console::style(blurb.to_string()),
      data: InstallRecipe::onestep(InstallStep::Apt(package_name.to_string())),
    },
  )
}

fn apt_with_alias(
  blurb: &str,
  package_name: &str,
  shortcut: &str,
  cmd_name: &str,
) -> TreeNodeDefinition<StyledMsgAndData<InstallRecipe>> {
  TreeNodeDefinition::new(
    NodeDefinitionType::PickOneChild { mandatory: false },
    StyledMsgAndData {
      message: console::style(blurb.to_string()),
      data: InstallRecipe::onestep(InstallStep::Apt(package_name.to_string())),
    },
  )
  .with_child(TreeNodeDefinition::new(
    NodeDefinitionType::Text,
    StyledMsgAndData {
      message: console::style(format!("alias {}={}?", shortcut, cmd_name)),
      data: InstallRecipe::onestep(InstallStep::MakeAlias {
        name: shortcut.to_string(),
        command: cmd_name.to_string(),
      }),
    },
  ))
}
