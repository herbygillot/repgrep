use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use tui::style::{Color, Style};
use tui::widgets::Text;

use crate::rg::de::{ArbitraryData, RgMessageType, SubMatch};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ItemKind {
  Begin,
  Context,
  Match,
  End,
  Summary,
}

// TODO: tests for Base64 decoding on separate platforms

/// Convert Base64 encoded data to an OsString on Unix platforms.
/// https://doc.rust-lang.org/std/ffi/index.html#on-unix
#[cfg(not(target_os = "windows"))]
fn base64_to_os_string(bytes: Vec<u8>) -> Result<OsString> {
  use std::os::unix::ffi::OsStringExt;
  Ok(OsString::from_vec(bytes))
}

/// Convert Base64 encoded data to an OsString on Windows platforms.
/// https://doc.rust-lang.org/std/ffi/index.html#on-windows
#[cfg(target_os = "windows")]
fn base64_to_os_string(bytes: Vec<u8>) -> Result<OsString> {
  use safe_transmute::{transmute_many, try_copy, PedanticGuard};
  use std::os::windows::ffi::OsStringExt;

  // Transmute decoded Base64 bytes as UTF-16 since that's what underlying paths are on Windows.
  let bytes_u16 = try_copy!(transmute_many::<u16, PedanticGuard>(&bytes))?;
  OsString::from_wide(&bytes_u16)
}

#[derive(Debug, Clone)]
pub struct Item {
  rg_message_type: RgMessageType,

  pub kind: ItemKind,

  pub should_replace: bool,
}

impl Item {
  pub fn new(rg_message_type: RgMessageType) -> Item {
    let kind = match rg_message_type {
      RgMessageType::Begin { .. } => ItemKind::Begin,
      RgMessageType::Match { .. } => ItemKind::Match,
      RgMessageType::Context { .. } => ItemKind::Context,
      RgMessageType::End { .. } => ItemKind::End,
      RgMessageType::Summary { .. } => ItemKind::Summary,
    };

    Item {
      rg_message_type,
      kind,
      should_replace: true,
    }
  }

  pub fn is_selectable(&self) -> bool {
    matches!(self.kind, ItemKind::Begin | ItemKind::Match)
  }

  pub fn match_count(&self) -> usize {
    self
      .matches()
      .map(|submatches| submatches.len())
      .unwrap_or(0)
  }

  pub fn matches(&self) -> Option<&[SubMatch]> {
    match &self.rg_message_type {
      RgMessageType::Match { submatches, .. } => Some(submatches),
      _ => None,
    }
  }

  pub fn path(&self) -> PathBuf {
    let path_data = match &self.rg_message_type {
      RgMessageType::Begin { path, .. } => path,
      RgMessageType::Match { path, .. } => path,
      RgMessageType::Context { path, .. } => path,
      RgMessageType::End { path, .. } => path,
      unexpected_type => panic!(
        "Unexpected enum variant, got {:?} and expected all except Summary!",
        unexpected_type
      ),
    };

    match path_data {
      ArbitraryData::Text { text } => PathBuf::from(text),
      ArbitraryData::Base64 { bytes } => {
        // Decode the Base64 into u8 bytes.
        let data = match base64::decode(bytes) {
          Ok(data) => data,
          Err(e) => panic!("Error deserialising Base64 data: {}", e),
        };

        // Convert the bytes into an OsString.
        let os_string = match base64_to_os_string(data) {
          Ok(os_string) => os_string,
          Err(e) => panic!("Error transmuting Base64 data to OsString: {}", e),
        };

        PathBuf::from(os_string)
      }
    }
  }

  pub fn to_text(&self, replacement: Option<&String>) -> Text {
    // TODO: handle non-UTF-8 text
    match &self.rg_message_type {
      RgMessageType::Begin { path, .. } => {
        Text::styled(path.lossy_utf8(), Style::default().fg(Color::Magenta))
      }
      RgMessageType::Context {
        lines, line_number, ..
      } => {
        let mut text = lines.lossy_utf8();
        if let Some(number) = line_number {
          text = format!("{}:{}", number, text);
        }

        Text::styled(text, Style::default().fg(Color::DarkGray))
      }
      RgMessageType::Match {
        lines,
        line_number,
        submatches,
        ..
      } => {
        // TODO: highlight matches (red) on line and replacements (green). Currently not possible.
        // See: https://github.com/fdehau/tui-rs/issues/315
        let mut style = Style::default();
        if !self.should_replace {
          style = style.fg(Color::Red);
        }

        let mut text = lines.lossy_utf8();
        // TODO: when we can highlight mid-text, don't replace the match, colour the match (submatch.text.lossy_utf8())
        // and add the replacement after.
        if self.should_replace {
          if let Some(replacement) = replacement {
            for submatch in submatches.iter().rev() {
              text.replace_range(submatch.range.clone(), replacement);
            }
          }
        }

        if let Some(number) = line_number {
          text = format!("{}:{}", number, text);
        }

        Text::styled(text, style)
      }
      RgMessageType::End { .. } => Text::raw(""),
      unexpected_type => panic!(
        "Unexpected enum variant, got {:?} and expected only Context or Match!",
        unexpected_type
      ),
    }
  }
}