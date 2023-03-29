pub(crate) mod browser;
pub(crate) mod item_list;

pub(crate) mod trim_label {
  pub(crate) struct TrimLabel {
    text: egui::WidgetText,
    trim_at_word: bool,
  }

  pub(crate) trait UiTrimLabel {
    fn trim_label(
      &mut self,
      text: impl Into<egui::WidgetText>,
      trim_at_word: bool,
    ) -> egui::Response;
  }

  impl UiTrimLabel for egui::Ui {
    fn trim_label(
      &mut self,
      text: impl Into<egui::WidgetText>,
      trim_at_word: bool,
    ) -> egui::Response {
      self.add(TrimLabel {
        text: text.into(),
        trim_at_word,
      })
    }
  }

  impl TrimLabel {
    fn layout_in_ui(
      self,
      ui: &mut egui::Ui,
    ) -> (
      egui::Pos2,
      egui::widget_text::WidgetTextGalley,
      egui::Response,
    ) {
      let sense = {
        // We only want to focus labels if the screen reader is on.
        if ui.memory(|mem| mem.options.screen_reader) {
          egui::Sense::focusable_noninteractive()
        } else {
          egui::Sense::hover()
        }
      };
      if let egui::WidgetText::Galley(galley) = self.text {
        // If the user said "use this specific galley", then just use it:
        let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
        let pos = match galley.job.halign {
          egui::Align::LEFT => rect.left_top(),
          egui::Align::Center => rect.center_top(),
          egui::Align::RIGHT => rect.right_top(),
        };
        let text_galley = egui::widget_text::WidgetTextGalley {
          galley,
          galley_has_color: true,
        };
        return (pos, text_galley, response);
      }

      let valign = ui.layout().vertical_align();
      let mut text_job = self
        .text
        .into_text_job(ui.style(), egui::FontSelection::Default, valign);
      text_job.job.wrap.max_rows = 1;
      text_job.job.wrap.break_anywhere = !self.trim_at_word;

      text_job.job.wrap.max_width = ui.available_width();
      text_job.job.halign = ui.layout().horizontal_placement();
      text_job.job.justify = ui.layout().horizontal_justify();

      let text_galley = ui.fonts(|f| text_job.into_galley(f));
      let (rect, response) = ui.allocate_exact_size(text_galley.size(), sense);
      let pos = match text_galley.galley.job.halign {
        egui::Align::LEFT => rect.left_top(),
        egui::Align::Center => rect.center_top(),
        egui::Align::RIGHT => rect.right_top(),
      };
      (pos, text_galley, response)
    }
  }

  impl egui::Widget for TrimLabel {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
      let (pos, text_galley, response) = self.layout_in_ui(ui);
      response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, text_galley.text()));

      if ui.is_rect_visible(response.rect) {
        let response_color = ui.style().interact(&response).text_color();

        let underline = if response.has_focus() || response.highlighted() {
          egui::Stroke::new(1.0, response_color)
        } else {
          egui::Stroke::NONE
        };

        let override_text_color = if text_galley.galley_has_color {
          None
        } else {
          Some(response_color)
        };

        ui.painter().add(egui::epaint::TextShape {
          pos,
          galley: text_galley.galley,
          override_text_color,
          underline,
          angle: 0.0,
        });
      }

      response
    }
  }
}
