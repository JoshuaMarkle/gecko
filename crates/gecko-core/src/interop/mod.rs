//! Interop with other tools: keyboard-layout-editor.com boards (`kle`), other analyzers' layout
//! files (`import`: Oxeylyzer, genkey, cmini), the cmini community bundle (`community`), and
//! layout images (`svg`).

pub mod community;
pub mod import;
pub mod kle;
pub mod svg;
