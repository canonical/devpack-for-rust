use std::{io, ops::Range};

use crossterm::{
  event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
  terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
  Frame, Terminal,
  backend::CrosstermBackend,
  layout::Rect,
  style::{Modifier, Style},
  text::Line,
  widgets::Paragraph,
};
use treeversal::{
  EditPickedType, NodeDefinitionType, TreeDefinition, TreeInteraction, TreeInteractor,
  TreeInteractorNode, TreeNodeDefinition,
  console_driver::{Palette, StyledMsgAndData},
};

use crate::recipe::InstallRecipe;

const MAX_WIDTH: u16 = 100;
type Picker = TreeInteractor<StyledMsgAndData<InstallRecipe>>;

struct RawMode;

impl Drop for RawMode {
  fn drop(&mut self) {
    let _ = disable_raw_mode();
  }
}

pub fn pick(tree: TreeDefinition<StyledMsgAndData<InstallRecipe>>) -> io::Result<Option<Picker>> {
  enable_raw_mode()?;
  let _raw_mode = RawMode;
  let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
  terminal.clear()?;
  let mut picker = Picker::new(tree);
  let mut viewport = TreeViewport::default();
  let result = (|| {
    loop {
      terminal.draw(|frame| viewport.draw(frame, &picker))?;
      let Event::Key(key) = event::read()? else {
        continue;
      };
      if key.kind == KeyEventKind::Release {
        continue;
      }
      if matches!(key.code, KeyCode::Char('q' | 'Q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
      {
        return Ok(None);
      }
      match key.code {
        KeyCode::PageDown => viewport.scroll(true),
        KeyCode::PageUp => viewport.scroll(false),
        KeyCode::Esc | KeyCode::Home => viewport.go_to_top(&mut picker),
        KeyCode::End => {
          viewport.offset = usize::MAX;
          viewport.follow_cursor = false;
        }
        code => {
          let action = match code {
            KeyCode::Char(' ') | KeyCode::Enter => {
              TreeInteraction::EditPicked(EditPickedType::Toggle)
            }
            KeyCode::Char('y' | 'Y') => TreeInteraction::EditPicked(EditPickedType::Select),
            KeyCode::Char('n' | 'N') => TreeInteraction::EditPicked(EditPickedType::Deselect),
            KeyCode::Char('h') | KeyCode::Left => TreeInteraction::ExitNode,
            KeyCode::Char('j') | KeyCode::Down => TreeInteraction::SeekSibling { next: true },
            KeyCode::Char('k') | KeyCode::Up => TreeInteraction::SeekSibling { next: false },
            KeyCode::Char('l') | KeyCode::Right => TreeInteraction::EnterNode,
            _ => continue,
          };
          viewport.follow_cursor = true;
          if picker.interact(action).unwrap_or(false) {
            return Ok(Some(picker));
          }
        }
      }
    }
  })();
  terminal.clear()?;
  terminal.show_cursor()?;
  result
}

struct TreeViewport {
  offset: usize,
  page_size: usize,
  follow_cursor: bool,
}

impl Default for TreeViewport {
  fn default() -> Self {
    Self {
      offset: 0,
      page_size: 1,
      follow_cursor: true,
    }
  }
}

impl TreeViewport {
  fn go_to_top(&mut self, picker: &mut Picker) {
    while picker.cursor_path().len() > 1 {
      picker
        .interact(TreeInteraction::ExitNode)
        .expect("nested cursor has a parent");
    }
    while picker.cursor_path()[0] != 0 {
      picker
        .interact(TreeInteraction::SeekSibling { next: false })
        .expect("top-level cursor has siblings");
    }
    self.offset = 0;
    self.follow_cursor = true;
  }

  fn scroll(&mut self, down: bool) {
    self.offset = if down {
      self.offset.saturating_add(self.page_size)
    } else {
      self.offset.saturating_sub(self.page_size)
    };
    self.follow_cursor = false;
  }

  fn draw(&mut self, frame: &mut Frame, picker: &Picker) {
    let area = Rect {
      width: frame.area().width.min(MAX_WIDTH),
      ..frame.area()
    };
    if area.is_empty() {
      return;
    }
    let (lines, focused) = tree_lines(picker, area.width as usize);
    let capacity = if lines.len() > area.height as usize {
      (area.height as usize).saturating_sub(1).max(1)
    } else {
      area.height as usize
    };
    self.page_size = capacity;
    self.offset = self.offset.min(lines.len().saturating_sub(capacity));
    if self.follow_cursor {
      if focused.start < self.offset {
        self.offset = focused.start;
      } else if focused.end > self.offset + capacity {
        self.offset = focused.end.saturating_sub(capacity).min(focused.start);
      }
    }
    let end = (self.offset + capacity).min(lines.len());
    let visible = lines[self.offset..end].to_vec();
    frame.render_widget(
      Paragraph::new(visible),
      Rect {
        height: capacity as u16,
        ..area
      },
    );
    if capacity < area.height as usize {
      let remaining = lines.len() - end;
      let status = match (self.offset, remaining) {
        (0, 0) => String::new(),
        (0, below) => format!("({below} more lines)"),
        (above, 0) => format!("({above} lines above)"),
        (above, below) => format!("({above} lines above, {below} more lines)"),
      };
      frame.render_widget(
        Paragraph::new(status).style(Style::default().add_modifier(Modifier::DIM)),
        Rect {
          y: area.y + capacity as u16,
          height: 1,
          ..area
        },
      );
    }
  }
}

fn tree_lines(picker: &Picker, width: usize) -> (Vec<Line<'static>>, Range<usize>) {
  let mut lines = Vec::new();
  let mut focused = 0..1;
  append_node(
    &picker.tree().root,
    picker.root(),
    &mut Vec::new(),
    &[],
    picker.cursor_path(),
    width,
    &mut lines,
    &mut focused,
  );
  (lines, focused)
}

fn marker(
  node: &TreeNodeDefinition<StyledMsgAndData<InstallRecipe>>,
  state: &TreeInteractorNode,
) -> &'static str {
  match node.ty {
    NodeDefinitionType::PickMany => {
      if state.picked() == Some(true) {
        "[X] "
      } else {
        "[ ] "
      }
    }
    NodeDefinitionType::PickExactlyOne | NodeDefinitionType::PickUpToOne => {
      if state.picked() == Some(true) {
        "(o) "
      } else {
        "( ) "
      }
    }
    NodeDefinitionType::AllDone => "{?} ",
    NodeDefinitionType::Text => {
      let children = node
        .children
        .iter()
        .zip(state.children())
        .filter(|(child, _)| child.ty == NodeDefinitionType::PickMany)
        .map(|(_, child)| child.picked() == Some(true))
        .collect::<Vec<_>>();
      if children.is_empty() {
        ""
      } else if children.iter().all(|picked| *picked) {
        "[X] "
      } else if children.iter().any(|picked| *picked) {
        "[/] "
      } else {
        "[ ] "
      }
    }
  }
}

fn append_node(
  node: &TreeNodeDefinition<StyledMsgAndData<InstallRecipe>>,
  state: &TreeInteractorNode,
  path: &mut Vec<usize>,
  scaffolding: &[bool],
  cursor: &[usize],
  width: usize,
  lines: &mut Vec<Line<'static>>,
  focused: &mut Range<usize>,
) {
  let palette = Palette::fancy();
  let selected = path.as_slice() == cursor;
  let start = lines.len();
  let ancestors = scaffolding
    .iter()
    .take(scaffolding.len().saturating_sub(1))
    .map(|pipe| format!("{}  ", if *pipe { palette.pipe_vert } else { ' ' }))
    .collect::<String>();
  let branch = if let Some(pipe) = scaffolding.last() {
    format!(
      "{}{}{}",
      ancestors,
      if *pipe {
        palette.pipe_branch
      } else {
        palette.pipe_corner
      },
      if selected {
        &palette.selected_connector
      } else {
        &palette.unselected_connector
      }
    )
  } else {
    String::new()
  };
  let marker = marker(node, state);
  let prefix = format!("{branch}{marker}");
  let continuation = if let Some(pipe) = scaffolding.last() {
    format!(
      "{}{} {}",
      ancestors,
      if *pipe || !node.children.is_empty() {
        palette.pipe_vert
      } else {
        ' '
      },
      " ".repeat(marker.len())
    )
  } else {
    String::new()
  };
  let message = node.data.message.to_string();
  let message = if let Some((description, url)) = message.split_once(" (https://") {
    format!("{description}\nhttps://{}", url.trim_end_matches(')'))
  } else {
    message
  };
  let prefix_width = console::measure_text_width(&prefix);
  let (prefix, continuation) = if prefix_width >= width {
    (String::new(), String::new())
  } else {
    (prefix, continuation)
  };
  let content_width = width
    .saturating_sub(console::measure_text_width(&prefix))
    .max(1);
  let style = if selected {
    Style::default().add_modifier(Modifier::BOLD)
  } else {
    Style::default()
  };
  let mut first = true;
  for paragraph in message.lines() {
    for wrapped in textwrap::wrap(paragraph, content_width) {
      lines.push(Line::styled(
        format!("{}{wrapped}", if first { &prefix } else { &continuation }),
        style,
      ));
      first = false;
    }
  }
  if selected {
    *focused = start..lines.len();
  }
  for (index, (child, child_state)) in node.children.iter().zip(state.children()).enumerate() {
    path.push(index);
    let mut child_scaffolding = scaffolding.to_vec();
    child_scaffolding.push(index + 1 < node.children.len());
    append_node(
      child,
      child_state,
      path,
      &child_scaffolding,
      cursor,
      width,
      lines,
      focused,
    );
    path.pop();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ratatui::backend::TestBackend;

  #[test]
  fn returning_to_top_resets_focus_and_scroll_without_changing_selections() {
    let mut picker = Picker::new(crate::selection_tree::make_tree());
    picker
      .interact(TreeInteraction::SeekSibling { next: false })
      .unwrap();
    picker
      .interact(TreeInteraction::SeekSibling { next: false })
      .unwrap();
    picker.interact(TreeInteraction::EnterNode).unwrap();
    picker.interact(TreeInteraction::EnterNode).unwrap();
    picker
      .interact(TreeInteraction::EditPicked(EditPickedType::Select))
      .unwrap();
    let selections = picker.get_all_selected_paths();
    let mut viewport = TreeViewport {
      offset: 20,
      follow_cursor: false,
      ..TreeViewport::default()
    };
    viewport.go_to_top(&mut picker);
    assert_eq!(picker.cursor_path(), &[0]);
    assert_eq!(viewport.offset, 0);
    assert!(viewport.follow_cursor);
    assert_eq!(picker.get_all_selected_paths(), selections);
  }

  #[test]
  fn layout_separates_urls_and_preserves_tree_markers() {
    let picker = Picker::new(crate::selection_tree::make_tree());
    let (lines, _) = tree_lines(&picker, 100);
    let text = lines.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(text[0], "Customize your devpack-for-rust.");
    assert_eq!(text[1], "├>Pick a rust channel");
    assert_eq!(text[2], "│  ├─(o) stable");
    assert_eq!(text[6], "│  ├─( ) Helix: A post-modern text editor");
    assert_eq!(text[7], "│  │     https://helix-editor.com/");
    assert!(text.contains(&"├─[ ] Developer tools".to_string()));
  }

  #[test]
  fn narrow_layout_wraps_every_line() {
    let picker = Picker::new(crate::selection_tree::make_tree());
    for width in [1, 8, 20, 40, 80] {
      let (lines, _) = tree_lines(&picker, width);
      assert!(
        lines
          .iter()
          .all(|line| console::measure_text_width(&line.to_string()) <= width)
      );
    }
  }

  #[test]
  fn viewport_scrolls_to_focus_and_counts_hidden_lines() {
    let mut picker = Picker::new(crate::selection_tree::make_tree());
    let mut viewport = TreeViewport::default();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal
      .draw(|frame| viewport.draw(frame, &picker))
      .unwrap();
    assert_eq!(viewport.offset, 0);
    let (lines, _) = tree_lines(&picker, 80);
    let status = terminal
      .backend()
      .buffer()
      .content
      .iter()
      .skip(80 * 11)
      .map(|cell| cell.symbol())
      .collect::<String>();
    assert!(status.starts_with(&format!("({} more lines)", lines.len() - 11)));
    picker
      .interact(TreeInteraction::SeekSibling { next: false })
      .unwrap();
    terminal
      .draw(|frame| viewport.draw(frame, &picker))
      .unwrap();
    assert!(viewport.offset > 0);
    let (_, focused) = tree_lines(&picker, 80);
    assert!(focused.start >= viewport.offset);
    assert!(focused.end <= viewport.offset + viewport.page_size);
    viewport.scroll(false);
    terminal
      .draw(|frame| viewport.draw(frame, &picker))
      .unwrap();
    assert!(!viewport.follow_cursor);
  }

  #[test]
  fn resize_keeps_focus_visible_and_uses_available_height() {
    let mut picker = Picker::new(crate::selection_tree::make_tree());
    picker
      .interact(TreeInteraction::SeekSibling { next: false })
      .unwrap();
    let mut viewport = TreeViewport::default();
    let mut terminal = Terminal::new(TestBackend::new(160, 60)).unwrap();
    for (width, height) in [(160, 60), (80, 30), (40, 8), (20, 4), (1, 1), (80, 24)] {
      terminal.backend_mut().resize(width, height);
      terminal
        .draw(|frame| viewport.draw(frame, &picker))
        .unwrap();
      let (lines, focused) = tree_lines(&picker, width.min(MAX_WIDTH) as usize);
      let expected_capacity = if lines.len() > height as usize {
        (height as usize).saturating_sub(1).max(1)
      } else {
        height as usize
      };
      assert_eq!(viewport.page_size, expected_capacity);
      assert!(focused.start >= viewport.offset);
      assert!(focused.start < viewport.offset + viewport.page_size);
      for row in 0..height {
        for column in 0..width {
          if column >= MAX_WIDTH {
            assert_eq!(terminal.backend().buffer()[(column, row)].symbol(), " ");
          }
        }
      }
    }
  }

  #[test]
  fn selection_rules_and_install_recipes_are_preserved() {
    let mut picker = Picker::new(crate::selection_tree::make_tree());
    picker.interact(TreeInteraction::EnterNode).unwrap();
    assert!(
      picker
        .interact(TreeInteraction::EditPicked(EditPickedType::Deselect))
        .is_err()
    );
    picker
      .interact(TreeInteraction::SeekSibling { next: true })
      .unwrap();
    picker
      .interact(TreeInteraction::EditPicked(EditPickedType::Select))
      .unwrap();
    let selected = picker.get_all_selected_data();
    assert_eq!(selected.len(), 1);
    assert!(
      matches!(&selected[0].data.steps[0], crate::recipe::InstallStep::RustChannel(channel) if channel == "beta")
    );

    picker.interact(TreeInteraction::ExitNode).unwrap();
    picker
      .interact(TreeInteraction::SeekSibling { next: true })
      .unwrap();
    picker
      .interact(TreeInteraction::SeekSibling { next: true })
      .unwrap();
    picker
      .interact(TreeInteraction::EditPicked(EditPickedType::Toggle))
      .unwrap();
    assert_eq!(
      marker(picker.selected_node(), picker.selected_interactor_node()),
      "[X] "
    );
    assert_eq!(picker.get_all_selected_data().len(), 4);

    picker
      .interact(TreeInteraction::SeekSibling { next: true })
      .unwrap();
    picker.interact(TreeInteraction::EnterNode).unwrap();
    picker.interact(TreeInteraction::EnterNode).unwrap();
    picker
      .interact(TreeInteraction::EditPicked(EditPickedType::Select))
      .unwrap();
    assert_eq!(
      picker
        .select_interactor_node_via_path([3, 0].into_iter())
        .unwrap()
        .picked(),
      Some(true)
    );
    picker.interact(TreeInteraction::ExitNode).unwrap();
    picker
      .interact(TreeInteraction::EditPicked(EditPickedType::Deselect))
      .unwrap();
    assert_eq!(
      picker
        .select_interactor_node_via_path([3, 0, 0].into_iter())
        .unwrap()
        .picked(),
      Some(false)
    );
  }
}
