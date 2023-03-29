use crate::app::options::Options;

use paste::paste;

use ramp64_srm_convert_lib::{BatteryPath, ControllerPackPaths, Converter};

#[derive(Debug)]
pub(crate) struct GroupItem {
  pub(super) way: Way,
  paths: Box<Paths>,
}

macro_rules! can_edit {
  (create srm) => {
    false
  };
  (create $_:ident) => {
    true
  };
  (split srm) => {
    true
  };
  (split $_:ident) => {
    false
  };
}

macro_rules! is_valid {
  (create $self:expr, srm) => {
    true
  };
  (create $self:expr, $_:ident) => {
    is_valid!(create $self, eep sra fla mpk mpk1 mpk2 mpk3 mpk4)
  };
  (create $self:expr, $($name:ident)+ ) => {
    [$($self.paths.$name.is_some(),)+].iter().any(|b|*b)
  };
  (split $self:expr, srm) => {
    $self.paths.srm.is_some()
  };
  (split $self:expr, $_:ident) => {
    true
  }
}

macro_rules! get_mod_set {
  ($name:ident) => {
    #[allow(dead_code)]
    pub fn $name(&self) -> &Option<std::path::PathBuf> {
      &self.paths.$name
    }
    paste! {
      pub fn [<is_ $name _valid>](&self) -> bool {
        match &self.way {
          Way::Create => is_valid!(create self, $name),
          Way::Split => is_valid!(split self, $name),
        }
      }

      pub fn [<is_ $name _enabled>](&self) -> bool {
        match &self.way {
          Way::Create => can_edit!(create $name),
          Way::Split => can_edit!(split $name),
        }
      }
    }
  };
}

impl GroupItem {
  get_mod_set!(srm);
  get_mod_set!(eep);
  get_mod_set!(sra);
  get_mod_set!(fla);
  get_mod_set!(mpk);
  get_mod_set!(mpk1);
  get_mod_set!(mpk2);
  get_mod_set!(mpk3);
  get_mod_set!(mpk4);

  fn create(paths: Box<Paths>) -> Self {
    Self {
      way: Way::Create,
      paths,
    }
  }

  fn split(paths: Box<Paths>) -> Self {
    Self {
      way: Way::Split,
      paths,
    }
  }

  pub(crate) fn is_valid(&self) -> bool {
    self.is_srm_valid()
      && self.is_eep_valid()
      && self.is_sra_valid()
      && self.is_fla_valid()
      && self.is_mpk_valid()
      && self.is_mpk1_valid()
      && self.is_mpk2_valid()
      && self.is_mpk3_valid()
      && self.is_mpk4_valid()
  }

  pub(crate) fn set(&mut self, path: std::path::PathBuf) {
    self.paths.set(path)
  }

  pub(crate) fn convert(self, options: &Options) -> Result<(), (Box<dyn std::error::Error>, Self)> {
    match self.way {
      Way::Create => create_conversion(self.paths, options),
      Way::Split => split_conversion(self.paths, options),
    }
  }
}

impl From<std::path::PathBuf> for GroupItem {
  fn from(path: std::path::PathBuf) -> Self {
    let way = match tag_path(&path) {
      Some(Tag::Srm) => Way::Split,
      Some(_) => Way::Create,
      None => Way::Create,
    };
    let mut paths = Box::from(Paths::default());
    paths.set(path);
    Self { way, paths }
  }
}

#[derive(Debug)]
pub(crate) struct InvalidGroupError<V>
where
  V: std::fmt::Display + std::fmt::Debug,
{
  validation: Option<V>,
}

impl<V> std::fmt::Display for InvalidGroupError<V>
where
  V: std::fmt::Display + std::fmt::Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.validation {
      Some(validation) => f.write_fmt(format_args!("{validation}")),
      None => f.write_str("An invalid group was being converted!"),
    }
  }
}

impl<V> std::error::Error for InvalidGroupError<V> where V: std::fmt::Display + std::fmt::Debug {}

fn to_battery(path: std::path::PathBuf) -> Option<BatteryPath> {
  ramp64_srm_convert_lib::to_battery(path).ok()
}

fn to_controller_pack(path: std::path::PathBuf) -> Option<ControllerPackPaths> {
  ramp64_srm_convert_lib::to_controller_pack(path).ok()
}

fn create_conversion(
  paths: Box<Paths>,
  options: &Options,
) -> Result<(), (Box<dyn std::error::Error>, GroupItem)> {
  use ramp64_srm_convert_lib::create::Params;

  let Paths {
    srm: _,
    eep,
    sra,
    fla,
    mpk,
    mpk1,
    mpk2,
    mpk3,
    mpk4,
  } = *paths.clone();

  let mut params = Params::default()
    .set_out_dir(options.output_dir.clone());
  if let Some(battery) = eep.or(sra).or(fla).and_then(to_battery){
    params.as_mut().set_battery(battery);
  }
  if let Some(cp) = mpk.or(mpk1).and_then(to_controller_pack) {
    params.as_mut().set_controller_pack(cp);
  }
  if let Some(cp2) = mpk2.and_then(to_controller_pack){
    params.as_mut().set_controller_pack(cp2);
  }
  if let Some(cp3) = mpk3.and_then(to_controller_pack){
    params.as_mut().set_controller_pack(cp3);
  }
  if let Some(cp4) = mpk4.and_then(to_controller_pack){
    params.as_mut().set_controller_pack(cp4);
  }

  let validation = params.validate();
  if !validation.is_ok() {
    return Err((
      Box::new(InvalidGroupError {
        validation: Some(validation),
      }),
      GroupItem::create(paths),
    ));
  }

  params
    .convert(&options.user_params)
    .map_err(|e| (Box::from(e), GroupItem::create(paths)))
}

#[derive(Debug)]
struct InvalidSrmError {}
impl std::fmt::Display for InvalidSrmError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Invalid SRM for Split in group")
  }
}
impl std::error::Error for InvalidSrmError {}

fn split_conversion(
  paths: Box<Paths>,
  options: &Options,
) -> Result<(), (Box<dyn std::error::Error>, GroupItem)> {
  use ramp64_srm_convert_lib::split::{can_be_srm, Params};

  let srm = paths.srm.clone();
  let Some(srm_path) = srm else {
    return Err((Box::from(InvalidGroupError {validation:None::<&str>}), GroupItem::split(paths)))
  };

  let srm_path = match can_be_srm(srm_path) {
    Ok(path) => path,
    Err((_, err)) => return Err((Box::from(err), GroupItem::split(paths))),
  };

  let params = Params::new(srm_path)
    .set_out_dir(options.output_dir.clone())
    .set_output_mupen_pack(options.output_mupen);

  let validation = params.validate();
  if !validation.is_ok() {
    return Err((
      Box::from(InvalidGroupError {
        validation: Some(validation),
      }),
      GroupItem::split(paths),
    ));
  }

  params
    .convert(&options.user_params)
    .map_err(|e| (Box::from(e), GroupItem::split(paths)))
}

impl Paths {
  pub(crate) fn set(&mut self, path: std::path::PathBuf) {
    use Tag::*;
    match tag_path(&path) {
      Some(Srm) => self.srm = Some(path),
      Some(Eep) => {
        (self.sra, self.fla) = (None, None);
        self.eep = Some(path)
      }
      Some(Sra) => {
        (self.eep, self.fla) = (None, None);
        self.sra = Some(path)
      }
      Some(Fla) => {
        (self.eep, self.sra) = (None, None);
        self.fla = Some(path)
      }
      Some(Mpk) => {
        (self.mpk1, self.mpk2, self.mpk3, self.mpk4) = (None, None, None, None);
        self.mpk = Some(path)
      }
      Some(Mpk1) => {
        self.mpk = None;
        self.mpk1 = Some(path)
      }
      Some(Mpk2) => {
        self.mpk = None;
        self.mpk2 = Some(path)
      }
      Some(Mpk3) => {
        self.mpk = None;
        self.mpk3 = Some(path)
      }
      Some(Mpk4) => {
        self.mpk = None;
        self.mpk4 = Some(path)
      }
      _ => {}
    }
  }
}

#[derive(Clone, Debug, Default)]
struct Paths {
  srm: Option<std::path::PathBuf>,
  eep: Option<std::path::PathBuf>,
  sra: Option<std::path::PathBuf>,
  fla: Option<std::path::PathBuf>,
  mpk: Option<std::path::PathBuf>,
  mpk1: Option<std::path::PathBuf>,
  mpk2: Option<std::path::PathBuf>,
  mpk3: Option<std::path::PathBuf>,
  mpk4: Option<std::path::PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Way {
  Create,
  Split,
}

impl Way {
  pub(super) const ITEMS: [Way; 2] = [Way::Create, Way::Split];
}

impl ToString for Way {
  fn to_string(&self) -> String {
    String::from(match self {
      Way::Create => "Create",
      Way::Split => "Split",
    })
  }
}

enum Tag {
  Srm,
  Eep,
  Sra,
  Fla,
  Mpk,
  Mpk1,
  Mpk2,
  Mpk3,
  Mpk4,
}

fn tag_path(path: &std::path::Path) -> Option<Tag> {
  use std::ffi::OsStr;
  use Tag::*;
  match path
    .extension()
    .map(OsStr::to_ascii_uppercase)
    .as_deref()
    .and_then(OsStr::to_str)
  {
    Some("SRM") => Some(Srm),
    Some("EEP") => Some(Eep),
    Some("SRA") => Some(Sra),
    Some("FLA") => Some(Fla),
    Some("MPK") => {
      if let Ok(metadata) = path.metadata() {
        if metadata.len() == 0x8000 {
          return Some(Mpk1);
        }
      }
      Some(Mpk)
    }
    Some("MPK1") => Some(Mpk1),
    Some("MPK2") => Some(Mpk2),
    Some("MPK3") => Some(Mpk3),
    Some("MPK4") => Some(Mpk4),
    _ => None,
  }
}
