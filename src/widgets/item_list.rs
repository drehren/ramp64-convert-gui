pub(crate) trait Validator<T> {
  fn validate(&self, item: &T) -> bool;
}

impl<T> Validator<T> for () {
  fn validate(&self, _: &T) -> bool {
    true
  }
}

impl<T, U: Fn(&T) -> bool> Validator<T> for U {
  fn validate(&self, item: &T) -> bool {
    self(item)
  }
}

pub(crate) struct ItemList<T, V = ()>
where
  T: Iterator + ExactSizeIterator,
  <T as Iterator>::Item: Copy,
  egui::WidgetText: From<<T as Iterator>::Item>,
  V: Validator<<T as Iterator>::Item>,
{
  items: T,
  id_source: egui::Id,
  validation: Option<V>,
  shrink: [bool; 2],
  border: bool,
  selectable: bool,
  show_tooltips: bool,
}

impl<T> ItemList<T, ()>
where
  T: Iterator + ExactSizeIterator,
  <T as Iterator>::Item: Copy,
  egui::WidgetText: From<<T as Iterator>::Item>,
{
  pub fn new(items: T, id: impl Into<egui::Id>) -> Self {
    Self {
      items,
      id_source: id.into(),
      validation: None,
      shrink: [false; 2],
      border: true,
      selectable: true,
      show_tooltips: true,
    }
  }
}

impl<T, V> ItemList<T, V>
where
  T: Iterator + ExactSizeIterator,
  <T as Iterator>::Item: Copy,
  egui::WidgetText: From<<T as Iterator>::Item>,
  V: Validator<<T as Iterator>::Item>,
{
  pub fn with_validation<F>(self, validation: F) -> ItemList<T, F>
  where
    F: Validator<<T as Iterator>::Item>,
  {
    ItemList {
      validation: Some(validation),
      items: self.items,
      id_source: self.id_source,
      shrink: self.shrink,
      border: self.border,
      selectable: self.selectable,
      show_tooltips: self.show_tooltips,
    }
  }

  pub fn auto_shrink(self, shrink: [bool; 2]) -> Self {
    Self { shrink, ..self }
  }

  pub fn show_border(self, show_border: bool) -> Self {
    Self {
      border: show_border,
      ..self
    }
  }

  pub fn selectable(self, selectable: bool) -> Self {
    Self { selectable, ..self }
  }

  pub fn with_tooltips(self, show_tooltips: bool) -> Self {
    Self {
      show_tooltips,
      ..self
    }
  }

  pub fn show(self, selection: &mut Option<SelectionRange>, ui: &mut egui::Ui) {
    ui.push_id(self.id_source, |child_ui| {
      if self.border {
        egui::Frame::group(child_ui.style()).show(child_ui, |ui| self.show_contents(selection, ui));
      } else {
        self.show_contents(selection, child_ui);
      }
    });
  }

  fn show_contents(self, selection: &mut Option<SelectionRange>, ui: &mut egui::Ui) {
    let Self {
      items,
      id_source,
      validation,
      shrink,
      selectable,
      show_tooltips,
      border: _,
    } = self;

    let height = if selectable {
      ui.text_style_height(&egui::TextStyle::Button)
        .max(ui.spacing().interact_size.y)
    } else {
      ui.text_style_height(&egui::TextStyle::Body)
    };

    egui::ScrollArea::new([false, true])
      .id_source(id_source.with("_scroll_area"))
      .auto_shrink(shrink)
      .show_rows(ui, height, items.len(), |ui, range| {
        ui.skip_ahead_auto_ids(range.start);

        ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
          for (value, i) in items.skip(range.start).zip(range.into_iter()) {
            ui.scope(|ui| {
              if !validation.as_ref().map_or(true, |v| v.validate(&value)) {
                ui.style_mut().visuals.override_text_color = Some(egui::Color32::RED);
              }

              let mut response = if selectable {
                ui.selectable_label(selection.as_ref().map_or(false, |r| r.contains(&i)), value)
              } else {
                ui.label(value)
              };
              if show_tooltips {
                response = response.on_hover_text(value);
              }

              if response.clicked() {
                if ui.input(|i| i.modifiers.shift_only()) {
                  let r = selection.get_or_insert((i..i + 1).into());
                  if !r.contains(&i) {
                    let mut new_r = r.start().unwrap()..r.end().unwrap();
                    if new_r.start > i {
                      new_r.start = i;
                    } else if new_r.end <= i {
                      new_r.end = i + 1;
                    }
                    r.add(new_r);
                  }
                } else if ui.input(|i| i.modifiers.command_only()) {
                  selection.get_or_insert(Default::default()).add(i..i + 1);
                } else {
                  *selection = Some((i..i + 1).into());
                }
              }
            });
          }
        })
      });
  }
}

#[derive(Default, Clone, PartialEq)]
pub struct SelectionRange {
  ranges: Vec<usize>,
}

impl std::fmt::Debug for SelectionRange {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SelectionRange")
      .field("ranges", &self.ranges.chunks(2).map(|c| c[0]..c[1]))
      .finish()
  }
}

impl From<std::ops::Range<usize>> for SelectionRange {
  fn from(value: std::ops::Range<usize>) -> Self {
    Self::new(value)
  }
}

impl FromIterator<std::ops::Range<usize>> for SelectionRange {
  fn from_iter<T: IntoIterator<Item = std::ops::Range<usize>>>(iter: T) -> Self {
    let mut me = Self::default();
    for range in iter {
      me.add(range);
    }
    me
  }
}

impl SelectionRange {
  pub(crate) fn new(range: std::ops::Range<usize>) -> Self {
    let mut me = Self::default();
    me.add(range);
    me
  }

  pub(crate) fn start(&self) -> Option<usize> {
    self.ranges.first().copied()
  }

  pub(crate) fn end(&self) -> Option<usize> {
    self.ranges.last().copied()
  }

  pub(crate) fn add(&mut self, range: std::ops::Range<usize>) {
    if range.is_empty() {
      return;
    }

    let std::ops::Range { start, end } = range;

    match (
      self.ranges.binary_search(&start),
      self.ranges.binary_search(&end),
    ) {
      (Ok(start_i), Ok(end_i)) => {
        let add_start = if start_i % 2 == 0 { 1 } else { 0 };
        let add_end = if end_i % 2 == 0 { 1 } else { 0 };
        self.drain_ranges((start_i + add_start)..(end_i + add_end));
      }
      (Ok(start_i), Err(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => {
          self.ranges[end_i - 1] = end;
          self.drain_ranges((start_i + 1)..(end_i - 1));
        }
        (true, false) => {
          self.drain_ranges((start_i + 1)..end_i);
        }
        (false, true) => {
          self.ranges[end_i - 1] = end;
          self.drain_ranges(start_i..(end_i - 1));
        }
        (false, false) => {
          self.drain_ranges(start_i..end_i);
        }
      },
      (Err(start_i), Ok(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => {
          self.ranges[start_i] = start;
          self.drain_ranges((start_i + 1)..(end_i + 1));
        }
        (true, false) => {
          self.ranges[start_i] = start;
          self.drain_ranges((start_i + 1)..end_i);
        }
        (false, true) => {
          self.drain_ranges(start_i..(end_i + 1));
        }
        (false, false) => {
          self.drain_ranges(start_i..end_i);
        }
      },
      (Err(start_i), Err(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => {
          self.ranges.insert(end_i, end);
          self.ranges.insert(start_i, start);
          self.drain_ranges((start_i + 1)..end_i);
        }
        (true, false) => {
          self.ranges[start_i] = start;
          self.drain_ranges((start_i + 1)..end_i);
        }
        (false, true) => {
          self.ranges[end_i - 1] = end;
          self.drain_ranges(start_i..(end_i - 1));
        }
        (false, false) => {
          self.drain_ranges(start_i..end_i);
        }
      },
    }
  }

  fn drain_ranges(&mut self, range: std::ops::Range<usize>) -> bool {
    if !range.is_empty() {
      self.ranges.drain(range).any(|_| true)
    } else {
      false
    }
  }

  pub(crate) fn len(&self) -> usize {
    self.ranges.chunks(2).map(|c| c[1] - c[0]).sum::<usize>()
  }

  #[allow(dead_code)]
  pub(crate) fn remove(&mut self, range: &std::ops::Range<usize>) -> bool {
    if self.ranges.is_empty() {
      return false;
    }
    let std::ops::Range { start, end } = range;

    match (
      self.ranges.binary_search(start),
      self.ranges.binary_search(end),
    ) {
      (Ok(start_i), Ok(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => self.drain_ranges(start_i..end_i),
        (true, false) => self.drain_ranges(start_i..(end_i + 1)),
        (false, true) => self.drain_ranges((start_i + 1)..end_i),
        (false, false) => self.drain_ranges((start_i + 1)..(end_i + 1)),
      },
      (Ok(start_i), Err(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => self.drain_ranges(start_i..end_i),
        (true, false) => {
          self.ranges[end_i - 1] = *end;
          self.drain_ranges(start_i..(end_i - 1));
          true
        }
        (false, true) => self.drain_ranges((start_i + 1)..end_i),
        (false, false) => {
          self.ranges[end_i - 1] = *end;
          self.drain_ranges((start_i + 1)..(end_i - 1));
          true
        }
      },
      (Err(start_i), Ok(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => self.drain_ranges(start_i..end_i),
        (true, false) => self.drain_ranges(start_i..(end_i + 1)),
        (false, true) => {
          self.ranges[start_i] = *start;
          self.drain_ranges((start_i + 1)..end_i);
          true
        }
        (false, false) => {
          self.ranges[start_i] = *start;
          self.drain_ranges((start_i + 1)..(end_i + 1));
          true
        }
      },
      (Err(start_i), Err(end_i)) => match (start_i % 2 == 0, end_i % 2 == 0) {
        (true, true) => self.drain_ranges(start_i..end_i),
        (true, false) => {
          self.ranges[end_i - 1] = *end;
          self.drain_ranges(start_i..(end_i - 1));
          true
        }
        (false, true) => {
          self.ranges[start_i] = *start;
          self.drain_ranges((start_i + 1)..end_i);
          true
        }
        (false, false) => {
          // hack?
          if start_i == end_i {
            self.ranges.extend([*start, *end]);
            self.ranges.sort();
            true
          } else {
            self.ranges[start_i] = *start;
            self.ranges[end_i - 1] = *end;
            self.drain_ranges((start_i + 1)..(end_i - 1));
            true
          }
        }
      },
    }
  }

  pub(crate) fn contains(&self, value: &usize) -> bool {
    if self.ranges.is_empty() {
      return false;
    }
    match self.ranges.binary_search(value) {
      Ok(idx) => idx % 2 == 0,
      Err(idx) => idx % 2 == 1,
    }
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.ranges.is_empty()
  }

  pub(crate) fn into_ranges(self) -> Vec<std::ops::Range<usize>> {
    self.ranges.chunks(2).map(|c| c[0]..c[1]).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::SelectionRange;

  #[test]
  fn selection_range_test() {
    let mut range = SelectionRange::default();

    assert!(range.is_empty());
    assert_eq!(range.len(), 0);

    range.add(1..3);
    // internally
    assert_eq!(range.ranges.len(), 2);

    assert!(!range.is_empty());
    assert_eq!(range.len(), 2);
    assert!(!range.contains(&0));
    assert!(range.contains(&1));
    assert!(range.contains(&2));
    assert!(!range.contains(&3));

    range.add(4..7);
    // internally
    assert_eq!(range.ranges.len(), 4);

    assert_eq!(range.len(), 5);
    assert!(!range.contains(&3));
    assert!(range.contains(&4));

    range.add(3..4);
    // internally
    assert_eq!(range.ranges.len(), 2);

    assert_eq!(range.len(), 6);
    assert!(range.contains(&3));
  }

  #[test]
  fn selection_range_add_edge_tests_ok_ok() {
    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 6);

    range.add(10..14);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    range.add(10..15);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    range.add(11..14);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    range.add(11..15);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));
  }

  #[test]
  fn selection_range_add_edge_tests_ok_err() {
    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 2 * 2);

    range.add(10..13);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    range.add(10..15);
    assert_eq!(range.len(), 6);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    range.add(11..13);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    range.add(11..15);
    assert_eq!(range.len(), 6);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));
  }

  #[test]
  fn selection_range_add_edge_tests_err_ok() {
    let range = SelectionRange::from_iter([10..12, 14..15]);
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 2 * 2);

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    range.add(13..14);
    assert_eq!(range.len(), 4);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    range.add(13..15);
    assert_eq!(range.len(), 4);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    range.add(11..14);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    range.add(11..15);
    assert_eq!(range.len(), 5);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));
  }

  #[test]
  fn selection_range_add_edge_tests_err_err() {
    let range = SelectionRange::default();
    assert_eq!(range.len(), 0);
    assert!(range.is_empty());

    let mut range = SelectionRange::from(13..16);
    range.add(10..15);
    assert_eq!(range.len(), 6);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from(10..13);
    range.add(12..16);
    assert_eq!(range.len(), 6);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..12, 14..16]);
    range.add(11..15);
    assert_eq!(range.len(), 6);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));
  }

  #[test]
  fn selection_range_remove_edge_tests_ok_ok() {
    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 3 * 2);

    assert!(range.remove(&(10..14)));
    assert_eq!(range.len(), 1);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(14));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    assert!(range.remove(&(10..15)));
    assert_eq!(range.len(), 0);
    assert_eq!(range.ranges.len(), 0 * 2);
    assert_eq!(range.start(), None);
    assert_eq!(range.end(), None);

    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    assert!(range.remove(&(11..14)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..11, 12..13, 14..15]);
    assert!(range.remove(&(11..15)));
    assert_eq!(range.len(), 1);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(11));
  }

  #[test]
  fn selection_range_remove_edge_tests_ok_err() {
    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 2 * 2);

    assert!(range.remove(&(10..13)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(14));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    assert!(range.remove(&(10..15)));
    assert_eq!(range.len(), 1);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(15));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    assert!(!range.remove(&(11..13)));
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..11, 14..16]);
    assert!(range.remove(&(11..15)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));
  }

  #[test]
  fn selection_range_remove_edge_tests_err_ok() {
    let range = SelectionRange::from_iter([10..12, 14..15]);
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 2 * 2);

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    assert!(range.remove(&(9..14)));
    assert_eq!(range.len(), 1);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(14));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    assert!(range.remove(&(9..15)));
    assert_eq!(range.len(), 0);
    assert_eq!(range.ranges.len(), 0 * 2);
    assert_eq!(range.start(), None);
    assert_eq!(range.end(), None);

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    assert!(range.remove(&(11..14)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(15));

    let mut range = SelectionRange::from_iter([10..12, 14..15]);
    assert!(range.remove(&(11..15)));
    assert_eq!(range.len(), 1);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(11));
  }

  #[test]
  fn selection_range_remove_edge_tests_err_err() {
    let mut range = SelectionRange::from(13..16);
    assert!(!range.remove(&(10..12)));
    assert_eq!(range.len(), 3);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(13));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from_iter([10..12, 14..16, 18..20]);
    assert!(range.remove(&(13..17)));
    assert_eq!(range.len(), 4);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(20));

    let mut range = SelectionRange::from(13..16);
    assert!(range.remove(&(10..15)));
    assert_eq!(range.len(), 1);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(15));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from(10..13);
    assert!(range.remove(&(12..16)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 1 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(12));

    let mut range = SelectionRange::from_iter([10..12, 14..16]);
    assert!(range.remove(&(11..15)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));

    let mut range = SelectionRange::from(10..16);
    assert!(range.remove(&(11..15)));
    assert_eq!(range.len(), 2);
    assert_eq!(range.ranges.len(), 2 * 2);
    assert_eq!(range.start(), Some(10));
    assert_eq!(range.end(), Some(16));
  }
}
