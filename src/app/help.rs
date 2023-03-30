use egui::text::LayoutJob;
use egui::{Style, TextFormat, TextStyle, WidgetText};

struct TextRun<'style> {
  job: LayoutJob,
  style: &'style Style,
}

impl<'style> TextRun<'style> {
  fn new(style: &Style) -> TextRun<'_> {
    TextRun {
      job: Default::default(),
      style,
    }
  }

  fn text_fmt(mut self, text: &str, fmt: impl FnOnce(TextFormat) -> TextFormat) -> Self {
    let font_id = TextStyle::Body.resolve(self.style);
    self.job.append(
      text,
      0.0,
      fmt(TextFormat {
        font_id,
        ..Default::default()
      }),
    );
    self
  }

  fn text(self, text: &str) -> Self {
    self.text_fmt(text, |fmt| fmt)
  }
}

impl<'style> From<TextRun<'style>> for WidgetText {
  fn from(value: TextRun) -> Self {
    WidgetText::LayoutJob(value.job)
  }
}

#[derive(Debug, Default)]
pub(crate) struct Usage {}
impl Usage {
  pub fn show(self, ui: &mut egui::Ui) {
    ui.label(WidgetText::from(
      TextRun::new(ui.style())
        .text(" • ")
        .text_fmt("File > Add File", |f| TextFormat {
          color: ui.visuals().strong_text_color(),
          ..f
        })
        .text(" to add a single save file to convert."),
    ));
    ui.label(WidgetText::from(
      TextRun::new(ui.style())
        .text(" • ")
        .text_fmt("File > Add Directory", |f| TextFormat {
          color: ui.visuals().strong_text_color(),
          ..f
        })
        .text(" to add save files from a directory."),
    ));

    ui.label(WidgetText::from(
      TextRun::new(ui.style())
        .text(" • Specify Output Folder by clicking the ")
        .text_fmt("Select Directory...", |f| TextFormat {
          color: ui.visuals().strong_text_color(),
          ..f
        })
        .text(" button ."),
    ));

    ui.label(WidgetText::from(
      TextRun::new(ui.style())
        .text(" • Press the ")
        .text_fmt("Convert", |f| TextFormat {
          color: ui.visuals().strong_text_color(),
          ..f
        })
        .text(" button to proceed with the conversion."),
    ));

    ui.add_space(30.0);

    ui.horizontal(|ui| {
      ui.spacing_mut().item_spacing.x = 0.0;
      ui.label("See more in the ");
      ui.hyperlink_to(
        "wiki",
        "https://github.com/drehren/ramp64-convert-gui/wiki/Help",
      );
      ui.label(".");
    });
  }
}
