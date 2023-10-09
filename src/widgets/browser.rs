#[derive(Debug, Default, Clone)]
pub(crate) struct FileFilter<'f> {
  pub name: &'f str,
  pub extensions: &'f [&'f str],
}

impl FileFilter<'static> {
  pub const fn new(name: &'static str, extensions: &'static [&'static str]) -> Self {
    Self { name, extensions }
  }
}

#[derive(Clone)]
pub(crate) struct Browse<'o> {
  kind: Kind<'o>,
  default_text: egui::WidgetText,
  only_file_name: bool,
}

impl<'o> std::fmt::Debug for Browse<'o> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Browse")
      .field("kind", &self.kind)
      .field("default_text", &self.default_text.text())
      .finish()
  }
}

#[derive(Debug, Clone)]
enum Kind<'f> {
  PickFile(&'f [FileFilter<'f>]),
  // SaveFile(&'f [FileFilter<'f>], Option<&'f str>),
  PickDir,
}

impl<'o> Browse<'o> {
  fn new<'x: 'o>(default_text: impl Into<egui::WidgetText>, kind: Kind<'x>) -> Self {
    Self {
      kind,
      default_text: default_text.into(),
      only_file_name: false,
    }
  }

  pub fn pick_file<'f>(filters: &'f [FileFilter]) -> Browse<'f> {
    Browse::new("Select File...", Kind::PickFile(filters))
  }

  pub fn pick_directory() -> Self {
    Self::new("Select Directory...", Kind::PickDir)
  }

  // pub fn save_file<'f>(name: Option<&'f str>, filters: &'f [FileFilter]) -> Browse<'f> {
  //   Browse::new("Select File...", Kind::SaveFile(filters, name))
  // }

  pub fn set_default_text(self, default_text: impl Into<egui::WidgetText>) -> Self {
    Self {
      default_text: default_text.into(),
      ..self
    }
  }

  pub fn set_show_only_file_name(self, only_file_name: bool) -> Self {
    Self {
      only_file_name,
      ..self
    }
  }
}

pub(crate) trait UiBrowser {
  fn browse(&mut self, path: &mut Option<std::path::PathBuf>, options: Browse) -> egui::Response;
}

impl UiBrowser for egui::Ui {
  fn browse(&mut self, path: &mut Option<std::path::PathBuf>, options: Browse) -> egui::Response {
    let Browse {
      kind,
      default_text,
      only_file_name,
    } = options;

    let default_text = path
      .as_ref()
      .and_then(|p| {
        if only_file_name {
          p.file_name().and_then(|p| p.to_str())
        } else {
          p.to_str()
        }
      })
      .map_or(default_text, egui::WidgetText::from);

    let mut response = self.add(Browser { text: default_text });
    if let Some(path) = &path {
      response = response.on_hover_text(path.to_string_lossy());
    }
    if response.clicked() {
      let mut dialog = rfd::FileDialog::new();

      // put filters
      if let Kind::PickFile(filters) /*| Kind::SaveFile(filters, _) */ = &kind {
        for FileFilter { name, extensions } in *filters {
          dialog = dialog.add_filter(*name, extensions)
        }
      }

      // put initial dir
      if let Some(path) = &path {
        if path.is_dir() {
          dialog = dialog.set_directory(path);
        } else if path.is_file() {
          if let Some(parent) = path.parent() {
            dialog = dialog.set_directory(parent)
          }
          if let Some(file_name) = path.file_name().and_then(std::ffi::OsStr::to_str) {
            dialog = dialog.set_file_name(file_name)
          }
        }
      }

      if let Some(selected_path) = match kind {
        Kind::PickFile(_) => dialog.pick_file(),
        /*Kind::SaveFile(_, name) => {
          if let Some(name) = name {
            dialog = dialog.set_file_name(name);
          }
          dialog.save_file()
        }*/
        Kind::PickDir => dialog.pick_folder(),
      } {
        path.replace(selected_path);
        response.mark_changed();
      }
    }
    response
  }
}

struct Browser {
  text: egui::WidgetText,
}

impl egui::Widget for Browser {
  fn ui(self, ui: &mut egui::Ui) -> egui::Response {
    ui.scope(|ui| {
      let Browser { text } = self;

      let available_size = ui.available_size().clamp(
        ui.spacing().interact_size,
        egui::vec2(f32::INFINITY, ui.spacing().interact_size.y),
      );

      let mut label_job = text.into_text_job(
        ui.style(),
        egui::TextStyle::Button.into(),
        egui::Align::Center,
      );
      label_job.job.wrap.max_rows = 1;
      label_job.job.wrap.break_anywhere = true;
      label_job.job.wrap.max_width = available_size.x - 2.0 * ui.spacing().button_padding.x;

      let galley = ui.fonts(|fonts| label_job.into_galley(fonts));

      // only up to text size if it did not overflow
      let size = if galley
        .galley()
        .rows
        .last()
        .and_then(|r| r.glyphs.last())
        .map(|g| g.chr)
        != galley.galley().job.wrap.overflow_character.or(Some('…'))
      {
        galley.size() + 2.0 * ui.spacing().button_padding
      } else {
        available_size
      };

      let (rect, response) = ui.allocate_at_least(
        size.max(egui::vec2(0.0, ui.spacing().interact_size.y)),
        egui::Sense::click(),
      );

      response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, galley.text()));

      if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
          rect.expand(visuals.expansion),
          visuals.rounding,
          visuals.weak_bg_fill,
          visuals.bg_stroke,
        );

        let pos = ui
          .layout()
          .align_size_within_rect(galley.size(), rect.shrink2(ui.spacing().button_padding))
          .min;
        galley.paint_with_visuals(ui.painter(), pos, visuals);
      }
      response
    })
    .inner
  }
}
