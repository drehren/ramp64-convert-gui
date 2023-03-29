use crate::{
  app::shortcuts::*,
  widgets::browser::{Browse, FileFilter, UiBrowser},
};

use paste::paste;

pub(crate) enum Action {
  OpenOptions,
  AddFile(std::path::PathBuf),
  AddDir(std::path::PathBuf),
  SelectAll,
  RemoveAll,
  RemoveSelected,
  Quit,
  Help,
}

macro_rules! make_is_match_fn {
  ($n:ident) => {
    paste! {
      pub fn [<is_ $n:snake>](&self) -> bool {
        matches!(self, Self::$n)
      }
    }
  };
  ($n:tt(_)) => {
    paste! {
      pub fn [<is_ $n:snake>](&self) -> bool {
        matches!(self, Self::$n(_))
      }
    }
  };
}

impl Action {
  make_is_match_fn! {RemoveSelected}
}

pub(crate) struct Actions<'f> {
  add_file_opts: Browse<'f>,
  add_dir_opts: Browse<'f>,
  last_action: Option<Action>,
  entries_enabled: bool,
}

impl<'f> Default for Actions<'f> {
  fn default() -> Self {
    const ALL_FILTERS: [FileFilter; 6] = [
      FileFilter::new(
        "All Supported Files",
        &[
          "srm", "eep", "sra", "fla", "mpk", "mpk1", "mpk2", "mpk3", "mpk4",
        ],
      ),
      FileFilter::new("RetroArch Save", &["srm"]),
      FileFilter::new("EEPROM Save", &["eep"]),
      FileFilter::new("SRAM Save", &["sra"]),
      FileFilter::new("FlashRAM Save", &["fla"]),
      FileFilter::new("Controller Pack", &["mpk", "mpk1", "mpk2", "mpk3", "mpk4"]),
    ];

    Self {
      add_file_opts: Browse::pick_file(&ALL_FILTERS).set_default_text("Add File..."),
      add_dir_opts: Browse::pick_directory().set_default_text("Add Directory..."),
      last_action: None,
      entries_enabled: false,
    }
  }
}

impl<'f> Actions<'f> {
  pub fn get_last_action(&mut self, ctx: &egui::Context) -> Option<Action> {
    self.last_action.take().or_else(|| {
      ctx.input_mut(|input| {
        input
          .consume_shortcut(&QUIT)
          .then_some(Action::Quit)
          .or_else(|| {
            input
              .consume_shortcut(&SELECT_ALL)
              .then_some(Action::SelectAll)
          })
          .or_else(|| {
            input
              .consume_shortcut(&REMOVE)
              .then_some(Action::RemoveSelected)
          })
          .or_else(|| input.consume_shortcut(&HELP).then_some(Action::Help))
      })
    })
  }

  fn set_action(&mut self, action: Action, ui: &mut egui::Ui) {
    self.last_action = Some(action);
    ui.close_menu();
  }

  pub fn set_entries_action_enabled(&mut self, entries_enabled: bool) {
    self.entries_enabled = entries_enabled;
  }

  pub fn show(&mut self, ui: &mut egui::Ui, action_enable: impl FnOnce(&Action) -> bool) {
    egui::menu::bar(ui, |ui| {
      ui.menu_button("File", |ui| {
        let mut path = None;

        if ui.browse(&mut path, self.add_file_opts.clone()).clicked() {
          if let Some(path) = path {
            self.last_action = Some(Action::AddFile(path));
          }
          ui.close_menu();
        }

        path = None;
        if ui.browse(&mut path, self.add_dir_opts.clone()).clicked() {
          if let Some(path) = path {
            self.last_action = Some(Action::AddDir(path));
          }
          ui.close_menu();
        }

        ui.separator();

        if ui.button("Options").clicked() {
          self.set_action(Action::OpenOptions, ui);
        }

        ui.separator();

        if ui.button_shortcut("Close", &QUIT).clicked() {
          self.set_action(Action::Quit, ui);
        }
      });

      ui.add_enabled_ui(self.entries_enabled, |ui| {
        ui.menu_button("Entries", |ui| {
          ui.set_min_width(165.0);
          if ui.button_shortcut("Select All", &SELECT_ALL).clicked() {
            self.set_action(Action::SelectAll, ui);
          }

          ui.add_enabled_ui(action_enable(&Action::RemoveSelected), |ui| {
            if ui.button_shortcut("Remove Selected", &REMOVE).clicked() {
              self.set_action(Action::RemoveSelected, ui);
            }
          });

          if ui.button("Remove All").clicked() {
            self.set_action(Action::RemoveAll, ui);
          }
        });
      });

      if ui.button("Help").clicked() {
        self.set_action(Action::Help, ui);
      }
    });
  }
}
