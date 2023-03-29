mod actions;
mod error_list;
mod file_groups;
mod options;
mod work;

use std::collections::VecDeque;

use crate::widgets::browser::{Browse, UiBrowser};

use self::{actions::Actions, file_groups::FileGroups, options::Options};

use self::{
  error_list::ErrorList,
  work::{start_worker_thread, ScanDirResult, Work, WorkResult},
};

pub(crate) mod shortcuts {
  use egui::{Key::*, KeyboardShortcut, Modifiers};

  pub const QUIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Q);
  pub const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, A);
  pub const HELP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, F1);
  pub const REMOVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Delete);

  pub(crate) trait UiButtonShortcut {
    #[must_use]
    fn button_shortcut(
      &mut self,
      text: impl Into<egui::WidgetText>,
      ks: &KeyboardShortcut,
    ) -> egui::Response;
  }

  impl UiButtonShortcut for egui::Ui {
    #[must_use]
    fn button_shortcut(
      &mut self,
      text: impl Into<egui::WidgetText>,
      ks: &KeyboardShortcut,
    ) -> egui::Response {
      self.add(egui::Button::new(text).shortcut_text(self.ctx().format_shortcut(ks)))
    }
  }
}

enum Windows {
  Options,
  Error,
  ValidationMessage,
  ConversionEndMessage,
}

impl From<&Windows> for egui::WidgetText {
  fn from(value: &Windows) -> Self {
    match value {
      Windows::Options => Self::from(egui::RichText::from("Conversion Options")),
      Windows::Error => Self::from(egui::RichText::from("Could Not Complete")),
      Windows::ValidationMessage => Self::from(egui::RichText::from("Validation Failed")),
      Windows::ConversionEndMessage => Self::from(egui::RichText::from("Conversion Successful")),
    }
  }
}

pub struct RaMp64<'a> {
  errors: ErrorList<ErrorCategory>,
  actions: Actions<'a>,
  options: Options,
  file_groups: FileGroups,
  worker: std::sync::mpsc::Sender<Work>,
  result_receiver: std::sync::mpsc::Receiver<WorkResult>,
  validated: bool,
  window_show_queue: VecDeque<Windows>,
}

impl<'a> RaMp64<'a> {
  pub fn new(_cc: &eframe::CreationContext) -> Self {
    let (worker, receiver) = std::sync::mpsc::channel();
    let (result_sender, result_receiver) = std::sync::mpsc::channel();

    start_worker_thread(receiver, result_sender);

    Self {
      errors: ErrorList::default(),
      actions: Actions::default(),
      options: Options::default(),
      file_groups: FileGroups::default(),
      worker,
      result_receiver,
      validated: false,
      window_show_queue: VecDeque::new(),
    }
  }

  fn check_work_done(&mut self) {
    if let Ok(result) = self.result_receiver.try_recv() {
      match result {
        WorkResult::ScanDirectory(scan_result) => self.check_scan_result(scan_result),
      }
    }
  }

  fn check_scan_result(&mut self, scan_result: ScanDirResult) {
    match scan_result {
      Ok(files) => {
        self.validated = false;
        self.file_groups.add_files(files);
      }
      Err(error) => {
        if !self.errors.has_errors() {
          self.window_show_queue.push_back(Windows::Error);
        }
        self.errors.add(ErrorCategory::AddFile, error)
      }
    }
  }

  fn enabled(&self) -> bool {
    self.window_show_queue.is_empty()
  }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum ErrorCategory {
  AddFile,
  Conversion,
}

impl error_list::Category for ErrorCategory {
  fn name(&self) -> &str {
    match self {
      ErrorCategory::AddFile => "Add File",
      ErrorCategory::Conversion => "Conversion",
    }
  }

  fn description(&self) -> &str {
    match self {
      ErrorCategory::AddFile => "All files which could not be added",
      ErrorCategory::Conversion => "All groups which could not be converted",
    }
  }
}

fn check_can_add_file(path: &std::path::Path) -> bool {
  use std::ffi::OsStr;
  [
    Some(OsStr::new("SRM")),
    Some(OsStr::new("EEP")),
    Some(OsStr::new("SRA")),
    Some(OsStr::new("FLA")),
    Some(OsStr::new("MPK")),
    Some(OsStr::new("MPK1")),
    Some(OsStr::new("MPK2")),
    Some(OsStr::new("MPK3")),
    Some(OsStr::new("MPK4")),
  ]
  .contains(&path.extension().map(OsStr::to_ascii_uppercase).as_deref())
}

impl<'a> eframe::App for RaMp64<'a> {
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    self.check_work_done();

    let enabled = self.enabled();

    egui::TopBottomPanel::top("actions").show(ctx, |ui| {
      ui.set_enabled(enabled);
      self
        .actions
        .set_entries_action_enabled(!self.file_groups.is_empty());

      self.actions.show(ui, |action| {
        if action.is_remove_selected() {
          self.file_groups.has_selection()
        } else {
          true
        }
      });
    });

    // get action
    if let Some(action) = self.actions.get_last_action(ctx) {
      use actions::Action::*;
      match action {
        OpenOptions => self.window_show_queue.push_back(Windows::Options),
        AddFile(selected_file) => {
          if check_can_add_file(&selected_file) {
            self.validated = false;
            self.file_groups.add_file(selected_file);
          }
        }
        AddDir(selected_dir) => {
          let _ = self.worker.send(Work::ScanDirectory(selected_dir));
        }
        SelectAll => self.file_groups.select_all(),
        RemoveAll => self.file_groups.clear(),
        RemoveSelected => self.file_groups.remove_selected(),
        Quit => frame.close(),
        Help => {}
      }
    }

    egui::CentralPanel::default().show(ctx, |ui| {
      ui.set_enabled(enabled);

      // prepare area before end buttons
      let items_max_rect = {
        let spacing = ui.spacing();
        egui::Rect::from_min_size(
          ui.cursor().min,
          ui.available_size()
            - egui::vec2(
              0.0,
              2.0 * (spacing.interact_size.y + spacing.item_spacing.y),
            ),
        )
      };

      let mut items_ui = ui.child_ui(items_max_rect, *ui.layout());
      ui.allocate_rect(items_max_rect, egui::Sense::hover());

      let item_updated = self.file_groups.show(&mut items_ui);
      if item_updated && self.validated {
        self.validated = false;
      }

      ui.horizontal(|ui| {
        ui.label("Output Folder");
        ui.centered_and_justified(|ui| {
          ui.browse(&mut self.options.output_dir, Browse::pick_directory());
        });
      });

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.button("Close").clicked() {
          frame.close();
        }
        ui.add_enabled_ui(self.validated, |ui| {
          if ui.button("Convert").clicked() {
            let group_count = self.file_groups.len();
            let had_errors = self.errors.has_errors();
            self.file_groups.convert(&self.options, &mut self.errors);
            if !had_errors && self.errors.has_errors() {
              self.window_show_queue.push_back(Windows::Error)
            }
            if group_count != self.file_groups.len() {
              self
                .window_show_queue
                .push_back(Windows::ConversionEndMessage);
            }
          }
        });
        ui.add_enabled_ui(!self.validated && !self.file_groups.is_empty(), |ui| {
          if ui.button("Validate").clicked() {
            self.validated = self.file_groups.validate();
            if !self.validated {
              self.window_show_queue.push_back(Windows::ValidationMessage);
            }
          }
        })
      });
    });

    if !self.window_show_queue.is_empty() {
      let mut showing = true;
      let window = self.window_show_queue.front().unwrap();
      egui::Window::new(window)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -30.0])
        .constrain(true)
        .collapsible(false)
        .resizable(false)
        .open(&mut showing)
        .show(ctx, |ui| match window {
          Windows::Options => self.options.show(ui),
          Windows::Error => self.errors.show(ui),
          Windows::ValidationMessage => {
            ui.label("The following entries have validation errors:");
            ui.add_space(1.0);

            self
              .file_groups
              .show_filtered(ui, |item: &file_groups::GroupItem| !item.is_valid());
          }
          Windows::ConversionEndMessage => {
            ui.vertical_centered(|ui| {
              ui.label(format!(
                "{} files where converted successfully!",
                if self.file_groups.is_empty() {
                  "All"
                } else {
                  "Some"
                }
              ));
            });
            if let Some(path) = &self.options.output_dir {
              ui.horizontal(|ui| {
                ui.label("Files where saved to:");
                if ui.link(format!("{}", path.display())).clicked() {
                  open::that_in_background(path);
                }
              });
            }
          }
        });
      if !showing {
        if matches!(window, Windows::Error) {
          self.errors.clear();
        }
        self.window_show_queue.pop_front();
      }
    }
  }
}
