mod group_item;

use self::group_item::Way;

use super::{error_list::ErrorList, options::Options, ErrorCategory};

use crate::widgets::{
  browser::{Browse, UiBrowser},
  item_list::{ItemList, SelectionRange},
  trim_label::UiTrimLabel,
};

pub(crate) use group_item::GroupItem;

use paste::paste;

use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct FileGroups {
  groups: BTreeMap<String, GroupItem>,
  selection: Option<SelectionRange>,
}

#[derive(Debug, Default)]
struct DisplayPath(std::path::PathBuf);
impl std::fmt::Display for DisplayPath {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.0.display().fmt(f)
  }
}

fn get_group(path: &std::path::Path) -> String {
  use std::ffi::OsStr;
  path
    .file_stem()
    .and_then(OsStr::to_str)
    .unwrap()
    .to_string()
}

impl FileGroups {
  pub fn add_file(&mut self, selected_file: std::path::PathBuf) {
    let group = get_group(&selected_file);
    self
      .groups
      .entry(group)
      .and_modify(|g| g.set(selected_file.clone()))
      .or_insert_with(|| GroupItem::from(selected_file));
  }

  pub fn add_files(&mut self, files: Vec<std::path::PathBuf>) {
    for file in files {
      self.add_file(file)
    }
  }

  pub fn select_all(&mut self) {
    self.selection = Some((0..self.groups.len()).into())
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.groups.is_empty()
  }

  pub(crate) fn has_selection(&self) -> bool {
    self.selection.as_ref().map_or(false, |r| !r.is_empty())
  }

  pub(crate) fn clear(&mut self) {
    self.groups.clear();
    self.selection = None
  }

  pub(crate) fn remove_selected(&mut self) {
    if let Some(selection) = self.selection.take() {
      let mut ranges = selection.into_ranges();
      ranges.reverse();
      for range in ranges {
        let mut i = 0;
        self.groups.retain(|_, _| {
          let x = !range.contains(&i);
          i += 1;
          x
        });
      }
    }
  }

  pub(crate) fn validate(&mut self) -> bool {
    self
      .groups
      .values_mut()
      .map(|g| g.is_valid())
      .reduce(|a, b| a && b)
      .unwrap()
  }

  pub(crate) fn convert(&mut self, options: &Options, errors: &mut ErrorList<ErrorCategory>) {
    self.selection = None;
    for (key, group) in std::mem::take(&mut self.groups) {
      if let Err((error, group)) = group.convert(options) {
        self.groups.insert(key.clone(), group);
        errors.add(
          ErrorCategory::Conversion,
          ItemConversionError { group: key, error },
        );
      }
    }
  }

  pub(crate) fn len(&self) -> usize {
    self.groups.len()
  }
}

#[derive(Debug)]
struct ItemConversionError {
  group: String,
  error: Box<dyn std::error::Error>,
}
impl std::fmt::Display for ItemConversionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{}: {}", self.group, self.error))
  }
}
impl std::error::Error for ItemConversionError {}

macro_rules! pick_file {
  ($ui:expr, $paths:expr, $label:ident) => {{
    pick_file!($ui, $paths, $label, paste! {stringify!([<$label:upper>])})
  }};
  ($ui:expr, $paths:expr, $file:ident, $label:expr) => {{
    $ui.label($label);
    let enabled = paste! { $paths.[<is_ $file _enabled>]() };
    let valid = paste! { $paths.[<is_ $file _valid>]() };
    let mut path = $paths.$file().clone();
    if pick_file!($ui, enabled, valid, &mut path, $label, &[stringify!($file)]) {
      if let Some(path) = path {
        $paths.set(path);
      }
      true
    } else {
      false
    }
  }};
  ($ui:expr, $enabled:expr, $valid:expr, $file_mut:expr, $name:expr, $ext:expr) => {{
    let changed = $ui.with_layout($ui.layout().with_main_justify(true), |ui| {
      ui.set_enabled($enabled);
      if !$valid {
        ui.style_mut().visuals.override_text_color = Some(egui::Color32::RED);
      }
      ui.browse(
        $file_mut,
        Browse::pick_file(&[$crate::widgets::browser::FileFilter {
          name: $name,
          extensions: $ext,
        }])
        .set_show_only_file_name(true),
      )
      .changed()
    });
    $ui.end_row();
    changed.inner
  }};
}

impl FileGroups {
  pub fn show_filtered<F>(&mut self, ui: &mut egui::Ui, filter: F)
  where
    F: Fn(&GroupItem) -> bool,
  {
    let items = self
      .groups
      .iter()
      .filter_map(|(k, g)| filter(g).then_some(k))
      .collect::<Vec<_>>();

    ItemList::new(items.iter().copied(), "filtered_entries")
      .auto_shrink([true; 2])
      .show_border(false)
      .selectable(false)
      .with_tooltips(false)
      .show(&mut None, ui);
  }

  pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
    if self.is_empty() {
      return false;
    }

    let mut item_updated = false;

    egui::SidePanel::new(egui::panel::Side::Right, "item options")
      .min_width(150.0)
      .show_animated_inside(
        ui,
        self.selection.as_ref().map_or(false, |s| s.len() == 1),
        |ui| {
          let (group_name, entry) = self
            .groups
            .iter_mut()
            .nth(self.selection.as_ref().unwrap().start().unwrap())
            .unwrap();
          ui.horizontal(|ui| {
            ui.small("Group");
            ui.trim_label(group_name, false);
          });
          ui.vertical(|ui| ui.add_space(3.0));
          egui::Grid::new("group_file_main")
            .num_columns(2)
            .show(ui, |ui| {
              ui.label("Mode");
              ui.with_layout(ui.layout().with_main_justify(true), |ui| {
                let mut index = Way::ITEMS.iter().position(|v| v == &entry.way).unwrap();
                if egui::ComboBox::from_id_source("group_mode")
                  .wrap(false)
                  .show_index(ui, &mut index, Way::ITEMS.len(), |i| {
                    Way::ITEMS[i].to_string()
                  })
                  .changed()
                {
                  item_updated = true;
                  entry.way = Way::ITEMS[index];
                }
              });
              ui.end_row();
              item_updated |= pick_file!(ui, entry, srm);
            });

          ui.vertical(|ui| ui.add_space(3.0));
          ui.small("Battery File (Only One)");
          egui::Grid::new("group_file_battery")
            .num_columns(2)
            .show(ui, |ui| {
              item_updated |= pick_file!(ui, entry, eep);
              item_updated |= pick_file!(ui, entry, sra);
              item_updated |= pick_file!(ui, entry, fla);
            });

          ui.vertical(|ui| ui.add_space(3.0));
          ui.small("Controller Packs (Mupen or Players)");
          egui::Grid::new("group_file_cp")
            .num_columns(2)
            .show(ui, |ui| {
              item_updated |= pick_file!(ui, entry, mpk, "Mupen");
              item_updated |= pick_file!(ui, entry, mpk1, "Player 1");
              item_updated |= pick_file!(ui, entry, mpk2, "Player 2");
              item_updated |= pick_file!(ui, entry, mpk3, "Player 3");
              item_updated |= pick_file!(ui, entry, mpk4, "Player 4");
            });
        },
      );

    ItemList::new(self.groups.keys(), "entries")
      .with_validation(|key: &&String| self.groups[*key].is_valid())
      .show(&mut self.selection, ui);
    item_updated
  }
}
