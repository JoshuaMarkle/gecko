//! A layout on its physical board as a standalone SVG image (`gecko layouts svg`). A port of the
//! web's `boardSvg` (`web/src/export.ts`) with the same sizes and colors, so the CLI and the
//! Export tab draw the same picture. Keys are drawn from [`Geometry::flat2d`].

use std::fmt::Write;

use crate::geometry::{Finger, Geometry, LayerMode};
use crate::layout::{self, Layout, EMPTY};
use crate::report::Analysis;
use crate::{Error, Result};

/// How keys are filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyColor {
    Plain,
    /// The finger palette (`web/src/colors.ts`), mirrored per hand.
    Fingers,
    /// White → ink by keystroke share, relative to the busiest key.
    Heat,
}

impl KeyColor {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "plain" => Ok(KeyColor::Plain),
            "fingers" => Ok(KeyColor::Fingers),
            "heat" => Ok(KeyColor::Heat),
            _ => Err(Error::invalid("svg", format!("unknown color '{s}' (plain, fingers or heat)"))),
        }
    }
}

/// What the image shows besides the base layer.
#[derive(Clone, Debug)]
pub struct SvgOptions {
    pub color: KeyColor,
    /// One board per layer (two per row) instead of the base layer only.
    pub layers: bool,
    /// Title line (layout name) and subtitle (board, corpus, profile, score).
    pub title: Option<(String, String)>,
}

const U: f64 = 56.0;
const PAD: f64 = 28.0;
const GAP: f64 = 36.0;
const CAP: f64 = 22.0;
const INK: &str = "#141414";
const MUTED: &str = "#707070";
const FONT: &str = "Inter, -apple-system, 'Helvetica Neue', Arial, sans-serif";

/// Legend for a token glyph (`TOKEN_LEGEND` in the web); other symbols are their own legend.
fn legend(c: char) -> String {
    let named = match c {
        layout::SPACE => "Space",
        layout::ENTER => "Enter",
        layout::BACKSPACE => "Bksp",
        layout::TAB => "Tab",
        layout::ESC => "Esc",
        layout::SHIFT => "Shift",
        EMPTY => "",
        _ => {
            return match layout::layer_key_info(c) {
                Some((l, LayerMode::Hold)) => format!("L{l}"),
                Some((l, LayerMode::Oneshot)) => format!("OS{l}"),
                Some((l, LayerMode::Toggle)) => format!("TG{l}"),
                None => c.to_string(),
            }
        }
    };
    named.to_string()
}

fn finger_color(f: Finger) -> &'static str {
    match f {
        Finger::LP | Finger::RP => "#a98ae0",
        Finger::LR | Finger::RR => "#72aeea",
        Finger::LM | Finger::RM => "#6fcf94",
        Finger::LI | Finger::RI => "#f5bf55",
        Finger::LT | Finger::RT => "#d6d6d6",
    }
}

/// `t` in [0, 1] → white..ink (`heatColor` in the web).
fn heat_color(t: f64) -> String {
    let v = (255.0 - (255.0 - 20.0) * t.clamp(0.0, 1.0).powf(0.8)).round() as u8;
    format!("#{v:02x}{v:02x}{v:02x}")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Rounded to 4 decimals and printed shortest, like the web's `r4`.
fn r4(v: f64) -> f64 {
    (v * 1e4).round() / 1e4
}

/// Keystroke share per (layer, physical key) from an analysis of `layout` on `g`: the scorer's
/// slots are `g`'s keys, then each layer's usable keys ([`Geometry::layered`]).
pub fn layer_heat(g: &Geometry, layout: &Layout, a: &Analysis) -> Result<Vec<Vec<f64>>> {
    let (vg, _) = layout.flat(g)?;
    let mut out = vec![vec![0.0; g.len()]; layout.layers.len() + 1];
    for (slot, k) in vg.key.iter().enumerate() {
        let key = k.phys.map_or(slot, |p| p as usize);
        if let (Some(row), Some(&h)) = (out.get_mut(k.layer as usize), a.heat.get(slot)) {
            row[key] = h;
        }
    }
    Ok(out)
}

/// The SVG. `heat` is [`layer_heat`] (needed only for [`KeyColor::Heat`]; missing keys are 0).
pub fn board_svg(g: &Geometry, layout: &Layout, heat: &[Vec<f64>], o: &SvgOptions) -> String {
    let flat = g.flat2d();
    // Drawn keys: top-left x, y, size w, h, rotation (degrees about the center).
    let ks: Vec<[f64; 5]> = flat
        .iter()
        .zip(&g.key)
        .map(|(f, k)| [f.pos[0] as f64, f.pos[1] as f64, k.size[0] as f64, k.size[1] as f64, f.rot as f64])
        .collect();
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for &[x, y, w, h, rot] in &ks {
        let (s, c) = rot.to_radians().sin_cos();
        let (hw, hh) = ((w * c.abs() + h * s.abs()) / 2.0, (w * s.abs() + h * c.abs()) / 2.0);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        (x0, x1, y0, y1) = (x0.min(cx - hw), x1.max(cx + hw), y0.min(cy - hh), y1.max(cy + hh));
    }
    let (bw, bh) = ((x1 - x0) * U, (y1 - y0) * U);
    let mut layers: Vec<&[char]> = vec![&layout.chars];
    if o.layers {
        layers.extend(layout.layers.iter().map(|l| l.chars.as_slice()));
    }
    let max = heat.iter().flatten().copied().fold(1e-9, f64::max);
    let captions = layers.len() > 1;

    let mut body = String::new();
    let mut y = PAD;
    if let Some((title, sub)) = &o.title {
        let _ = write!(body, r#"<text x="{PAD}" y="{}" font-size="20" font-weight="650" fill="{INK}">{}</text>"#, y + 18.0, esc(title));
        let _ = write!(body, r#"<text x="{PAD}" y="{}" font-size="13" fill="{MUTED}">{}</text>"#, y + 40.0, esc(sub));
        y += 64.0;
    }
    // Layers in a grid, two per row, so a layered layout stays close to the board's aspect.
    let cols = if captions { 2 } else { 1 };
    let cell = bh + if captions { CAP } else { 0.0 };
    for (l, keys) in layers.iter().enumerate() {
        let ox = PAD + (l % cols) as f64 * (bw + GAP);
        let mut oy = y + (l / cols) as f64 * (cell + GAP);
        if captions {
            let cap = if l == 0 {
                "Base".to_string()
            } else {
                let lk = LayerMode::ALL.into_iter().map(|m| layout::layer_key(l, m)).find(|c| layout.chars.contains(c));
                match lk {
                    Some(c) => {
                        let mode = match layout::layer_key_info(c).map(|i| i.1) {
                            Some(LayerMode::Oneshot) => "one-shot",
                            Some(LayerMode::Toggle) => "toggle",
                            _ => "hold",
                        };
                        format!("Layer {l} · {mode} {}", legend(c))
                    }
                    None => format!("Layer {l}"),
                }
            };
            let _ = write!(
                body,
                r#"<text x="{}" y="{}" font-size="13" font-weight="600" fill="{INK}">{}</text>"#,
                r4(ox),
                r4(oy + 14.0),
                esc(&cap)
            );
            oy += CAP;
        }
        for (i, &[kx, ky, kw, kh, rot]) in ks.iter().enumerate() {
            let sym = keys.get(i).copied().unwrap_or(EMPTY);
            let off = sym == EMPTY;
            let t = heat.get(l).and_then(|r| r.get(i)).copied().unwrap_or(0.0) / max;
            let fill = match o.color {
                _ if off => "#f3f3f3".to_string(),
                KeyColor::Fingers => finger_color(g.key[i].finger).to_string(),
                KeyColor::Heat => heat_color(t),
                KeyColor::Plain => "#ffffff".to_string(),
            };
            let ink = if o.color == KeyColor::Heat && !off && t > 0.5 { "#ffffff" } else { INK };
            let (x, yy) = (ox + (kx - x0) * U, oy + (ky - y0) * U);
            let (w, h) = (kw * U - 4.0, kh * U - 4.0);
            let (cx, cy) = (x + 2.0 + w / 2.0, yy + 2.0 + h / 2.0);
            let text = legend(sym);
            let size = if text.chars().count() > 2 { 11.0 } else { 18.0 };
            body.push_str("<g");
            if rot != 0.0 {
                let _ = write!(body, r#" transform="rotate({} {} {})""#, r4(rot), r4(cx), r4(cy));
            }
            let stroke = if off { "#e0e0e0" } else { "#c6c6c6" };
            let _ = write!(
                body,
                r#"><rect x="{}" y="{}" width="{}" height="{}" rx="6" fill="{fill}" stroke="{stroke}"/>"#,
                r4(x + 2.0),
                r4(yy + 2.0),
                r4(w),
                r4(h)
            );
            if !text.is_empty() {
                let _ = write!(
                    body,
                    r#"<text x="{}" y="{}" text-anchor="middle" font-size="{size}" font-weight="600" fill="{ink}">{}</text>"#,
                    r4(cx),
                    r4(cy + size * 0.36),
                    esc(&text)
                );
            }
            body.push_str("</g>");
        }
    }
    let rows = layers.len().div_ceil(cols);
    y += rows as f64 * cell + (rows - 1) as f64 * GAP;
    let min_w = if o.title.is_some() { 420.0 } else { 0.0 };
    let w = (cols as f64 * bw + (cols - 1) as f64 * GAP + 2.0 * PAD).max(min_w).ceil();
    let h = (y + PAD).ceil();
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" font-family="{FONT}"><rect width="{w}" height="{h}" fill="#ffffff"/>{body}</svg>"##
    )
}
