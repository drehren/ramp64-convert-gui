use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct ErrorList<C>
where
  C: Category,
{
  errors: HashMap<C, Vec<Box<dyn std::error::Error>>>,
}

impl<C> Default for ErrorList<C>
where
  C: Category,
{
  fn default() -> Self {
    Self {
      errors: Default::default(),
    }
  }
}

pub(crate) trait Category
where
  Self: PartialEq + Eq + std::hash::Hash,
{
  fn name(&self) -> &str;
  fn description(&self) -> &str;
}

impl<C> ErrorList<C>
where
  C: Category,
{
  pub fn add(&mut self, category: C, error: impl Into<Box<dyn std::error::Error>>) {
    self.errors.entry(category).or_default().push(error.into())
  }

  pub fn has_errors(&self) -> bool {
    !self.errors.is_empty()
  }

  pub fn clear(&mut self) {
    self.errors.clear()
  }

  fn show_all(&mut self, ui: &mut egui::Ui) {
    for (category, errors) in &self.errors {
      egui::CollapsingHeader::new(egui::RichText::from(category.name()).heading())
        .default_open(true)
        .show(ui, |ui| {
          for error in errors {
            ui.label(error.to_string());
          }
        })
        .header_response
        .on_hover_text(category.description());
    }
  }

  fn show_one(&mut self, ui: &mut egui::Ui) {
    for (_, errors) in &self.errors {
      for error in errors {
        ui.label(error.to_string());
      }
    }
  }
}

impl<C> ErrorList<C>
where
  C: Category,
{
  pub fn show(&mut self, ui: &mut egui::Ui) {
    if self.errors.len() != 1 {
      self.show_all(ui);
    } else {
      self.show_one(ui);
    }
  }
}
