use treeversal::{
  NodeDefinitionType, TreeDefinition, TreeNodeDefinition, console_driver::StyledMsgAndData, dsl,
};

use crate::recipe::{InstallRecipe, InstallStep};

pub fn make_tree() -> TreeDefinition<StyledMsgAndData<InstallRecipe>> {
  let rust_version = text("Pick a rust channel")
    .with_child(rust_channel("stable"))
    .with_child(rust_channel("beta"))
    .with_child(rust_channel("nightly"));

  let ide = text("Pick an IDE?")
    .with_child(dsl::pick_up_to_one(snap_step(
      "Helix: A post-modern text editor (https://helix-editor.com/)",
      "helix",
      true,
    )))
    .with_child(dsl::pick_up_to_one(snap_step(
      "VSCode: Your home for multi-agent development (https://code.visualstudio.com/)",
      "code",
      true,
    )))
    .with_child(dsl::pick_up_to_one(snap_step(
      "RustRover: JetBrains' powerful IDE for Rust (https://www.jetbrains.com/rust/)",
      "rustrover",
      true,
    )));
  // - can auto download vscode rust extension?

  // using backslash strings here because
  // rust-fmt has trouble with very long strings
  let dev_tools = text("Developer tools")
    // just is available on both snap and apt,
    // but is more up-to-date on snap
    .with_child(dsl::pick_many(snap_step(
      "just: handy way to save and run project-specific commands \
       (https://github.com/casey/just)",
      "just",
      true,
    )))
    .with_child(dsl::pick_many(apt_step(
      "bacon: background code checker \
       (https://github.com/canop/bacon)",
      "bacon",
    )))
    .with_child(dsl::pick_many(apt_step(
      "hyperfine: command-line benchmarking tool \
       (https://github.com/sharkdp/hyperfine)",
      "hyperfine",
    )));

  let extras = text("Oxidize your command line?")
    .with_child(dsl::pick_many(apt_step(
      "du-dust: a more intuitive version of du \
       (https://github.com/bootandy/dust)",
      "du-dust",
    )))
    .with_child(
      dsl::pick_many(apt_step(
        "fd-find: simple, fast and user-friendly alternative to 'find' \
         (https://github.com/sharkdp/fd)",
        "fd-find",
      ))
      .with_pick_children_needs_self(true)
      .with_child(dsl::pick_up_to_one(alias_step("fd", "fdfind"))),
    )
    .with_child(
      dsl::pick_many(apt_step(
        "ripgrep: recursively search directories \
         (https://github.com/BurntSushi/ripgrep)",
        "ripgrep",
      ))
      .with_pick_children_needs_self(true)
      .with_child(dsl::pick_up_to_one(alias_step("rg", "ripgrep"))),
    )
    .with_child(dsl::pick_many(apt_step(
      "sd: intuitive find & replace cli \
       (https://github.com/chmln/sd)",
      "sd",
    )))
    .with_child(dsl::pick_many(apt_step(
      "xh: friendly and fast tool for sending HTTP requests \
       (https://github.com/ducaale/xh)",
      "xh",
    )));

  TreeDefinition::new(
    text("Customize your devpack-for-rust.")
      .with_child(rust_version)
      .with_child(ide)
      .with_child(dev_tools)
      .with_child(extras)
      .with_child(TreeNodeDefinition::new(
        NodeDefinitionType::AllDone,
        StyledMsgAndData {
          message: console::style("All done?".to_string()),
          data: InstallRecipe::noop(),
        },
        false,
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
    false,
  )
}

fn rust_channel(channel: impl AsRef<str>) -> TreeNodeDefinition<StyledMsgAndData<InstallRecipe>> {
  let channel = channel.as_ref();
  // TODO: custom channel?
  // TODO: is "channel" the right word for this
  TreeNodeDefinition::new(
    NodeDefinitionType::PickExactlyOne,
    StyledMsgAndData {
      data: InstallRecipe::onestep(InstallStep::RustChannel(channel.to_string())),
      message: console::style(channel.to_string()),
    },
    false,
  )
}

fn snap_step(
  blurb: &str,
  program: &str,
  classic_confinement: bool,
) -> StyledMsgAndData<InstallRecipe> {
  StyledMsgAndData {
    message: console::style(blurb.to_string()),
    data: InstallRecipe::onestep(InstallStep::Snap {
      package_name: program.to_string(),
      classic_confinement,
    }),
  }
}

fn apt_step(blurb: &str, package_name: &str) -> StyledMsgAndData<InstallRecipe> {
  StyledMsgAndData {
    message: console::style(blurb.to_string()),
    data: InstallRecipe::onestep(InstallStep::Apt(package_name.to_string())),
  }
}

fn alias_step(shortcut: &str, cmd_name: &str) -> StyledMsgAndData<InstallRecipe> {
  StyledMsgAndData {
    message: console::style(format!("alias {}={}?", shortcut, cmd_name)),
    data: InstallRecipe::onestep(InstallStep::MakeAlias {
      name: shortcut.to_string(),
      command: cmd_name.to_string(),
    }),
  }
}
