//! Plain-text output shared by several commands (the `--json` output is built per command).

use gecko_core::metrics::Dir;
use gecko_core::report::Breakdown;
use gecko_core::{Geometry, LayerMode, Layout};

pub fn fmt_val(v: f64) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

pub fn print_breakdown(b: &Breakdown) {
    println!("{:<5} {:<18} {:>9} {:>7} {:>9}", "id", "metric", "value", "weight", "score");
    for m in &b.metrics {
        if m.weight == 0.0 && gecko_core::metrics::def(m.id).is_some_and(|d| d.retired) {
            continue;
        }
        let unit = if m.unit == "%" { "%" } else { "" };
        let dir = match m.dir {
            Dir::Min => "",
            Dir::Max => " +",
            Dir::Info => " i",
        };
        println!(
            "{:<5} {:<18} {:>8}{:1} {:>7} {:>9}",
            m.id,
            format!("{}{dir}", m.name),
            fmt_val(m.value),
            unit,
            if m.weight == 0.0 { "-".into() } else { format!("{}", m.weight) },
            if m.weighted == 0.0 { "-".into() } else { format!("{:.3}", m.weighted) }
        );
    }
    println!("{:<5} {:<18} {:>36.3}", "", "score (lower=better)", b.score);
}

/// A layout's rows (then each layer and magic key), indented for printing under a title.
pub fn layout_rows(layout: &Layout, g: &Geometry) -> String {
    let rows =
        |rs: &[String]| rs.iter().map(|r| r.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n  ");
    let mut out = rows(&layout.rows(g));
    let modes = layout.holders().unwrap_or_default();
    for (li, (ly, rs)) in layout.layers.iter().zip(layout.layer_rows(g)).enumerate() {
        let mode = modes.get(li).filter(|h| h.1 != LayerMode::Hold).map_or(String::new(), |h| format!(", {}", h.1.name()));
        out += &format!("\n  [{}{mode}]\n  {}", ly.name, rows(&rs));
    }
    for m in &layout.magic {
        out += &format!("\n  {} {}{}", m.key, gecko_core::magic::format_rules(&m.rules), if m.repeat { " (else repeat)" } else { "" });
    }
    if !layout.shifted.is_empty() {
        out += &format!("\n  ⇧ {}", layout.shifted.join(" "));
    }
    out
}

/// Slot-ordered symbols split into the geometry's rows on one line (symbols may repeat).
pub fn chars_rows(chars: &str, g: &Geometry) -> String {
    let mut it = chars.chars();
    g.row_lengths().iter().map(|&n| it.by_ref().take(n).collect::<String>()).collect::<Vec<_>>().join(" ")
}
