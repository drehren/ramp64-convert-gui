use ramp64_srm_convert_lib::UserParams;

#[derive(Debug, Default)]
pub(crate) struct Options {
  pub user_params: UserParams,
  pub output_mupen: bool,
  pub output_dir: Option<std::path::PathBuf>,
}

impl Options {
  pub fn show(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.user_params.overwrite, "Overwrite Existing Files");
    ui.checkbox(
      &mut self.user_params.swap_bytes,
      "Swap Bytes (EEP/FlashRAM)",
    );
    ui.checkbox(&mut self.output_mupen, "Output Mupen Pack on Split");
  }
}
