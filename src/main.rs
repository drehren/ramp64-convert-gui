#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use ramp64_convert_gui::RaMp64;

fn main() -> Result<(), eframe::Error> {
  let mut native_options = eframe::NativeOptions::default();
  native_options.min_window_size = Some(egui::vec2(410.0, 370.0));
  native_options.initial_window_size = native_options.min_window_size;
  native_options.drag_and_drop_support = false;
  if cfg!(windows) {
    native_options.centered = true;
  }

  eframe::run_native(
    "RetroArch Mupen64 SRM Converter",
    native_options,
    Box::new(|cc| Box::new(RaMp64::new(cc))),
  )
}
