use std::fmt::Write;

use dathren_core::Observation;
use serde::Serialize;

const WIDTH: f64 = 720.0;
const HEIGHT: f64 = 280.0;
const LEFT: f64 = 64.0;
const RIGHT: f64 = 16.0;
const TOP: f64 = 16.0;
const BOTTOM: f64 = 40.0;
const MAX_TICKS: f64 = 6.0;

#[derive(Serialize)]
pub struct LineChart {
    pub svg: String,
    pub points: Vec<Point>,
}

#[derive(Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub period: String,
    pub value: String,
    pub reported: Option<String>,
}

impl LineChart {
    pub fn new(observations: &[&Observation]) -> Option<Self> {
        let plotted: Vec<&Observation> = observations
            .iter()
            .copied()
            .filter(|o| o.best().is_some())
            .collect();
        let data: Vec<(String, f64)> = plotted
            .iter()
            .filter_map(|o| Some((o.period.to_string(), o.best()?)))
            .collect();
        if data.len() < 2 {
            return None;
        }

        let max = data.iter().map(|(_, v)| *v).fold(0.0, f64::max);
        let (step, y_max) = nice_scale(max);
        let plot_w = WIDTH - LEFT - RIGHT;
        let plot_h = HEIGHT - TOP - BOTTOM;
        let x_at = |i: usize| LEFT + plot_w * i as f64 / (data.len() - 1) as f64;
        let y_at = |v: f64| TOP + plot_h * (1.0 - v / y_max);

        let mut svg = String::new();
        let _ = write!(
            svg,
            r#"<svg class="chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Line chart; see the table below for values">"#
        );

        let ticks = (y_max / step).round() as usize;
        for t in 0..=ticks {
            let v = step * t as f64;
            let y = y_at(v);
            let class = if t == 0 { "baseline" } else { "grid" };
            let label = if t % 2 == 1 { "tick minor" } else { "tick" };
            let _ = write!(
                svg,
                r#"<line class="{class}" x1="{LEFT}" x2="{}" y1="{y:.1}" y2="{y:.1}"/><text class="{label}" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
                WIDTH - RIGHT,
                LEFT - 8.0,
                y + 4.0,
                compact(v),
            );
        }

        // Label every January, thinning out so labels never collide.
        let januaries: Vec<usize> = (0..data.len())
            .filter(|&i| data[i].0.ends_with("-01"))
            .collect();
        let every = januaries.len().div_ceil(8).max(1);
        for &i in januaries.iter().step_by(every) {
            let _ = write!(
                svg,
                r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
                x_at(i),
                HEIGHT - 10.0,
                &data[i].0[..4],
            );
        }

        let mut path = String::new();
        for (i, (_, v)) in data.iter().enumerate() {
            let _ = write!(
                path,
                "{}{:.1},{:.1}",
                if i == 0 { "M" } else { "L" },
                x_at(i),
                y_at(*v)
            );
        }
        let _ = write!(svg, r#"<path class="series" d="{path}"/>"#);

        let last = data.len() - 1;
        let (lx, ly) = (x_at(last), y_at(data[last].1));
        let _ = write!(
            svg,
            r#"<circle class="marker" cx="{lx:.1}" cy="{ly:.1}" r="4"/><text class="direct-label" x="{lx:.1}" y="{:.1}" text-anchor="end">{}</text>"#,
            ly - 10.0,
            thousands(data[last].1),
        );
        let _ = write!(
            svg,
            r#"<g class="hover" hidden><line class="crosshair" y1="{TOP}" y2="{}"/><circle class="marker" r="4"/></g></svg>"#,
            HEIGHT - BOTTOM,
        );

        let points = data
            .iter()
            .zip(&plotted)
            .enumerate()
            .map(|(i, ((period, v), o))| Point {
                x: x_at(i),
                y: y_at(*v),
                period: period.clone(),
                value: thousands(*v),
                reported: o.value.map(thousands),
            })
            .collect();
        Some(Self { svg, points })
    }
}

/// Picks the step from 1, 2, 2.5, or 5 times a power of ten that gives the
/// tightest axis top with at most `MAX_TICKS` gridlines.
fn nice_scale(max: f64) -> (f64, f64) {
    if max <= 0.0 {
        return (1.0, 1.0);
    }
    let magnitude = 10f64.powf((max / MAX_TICKS).log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0, 20.0]
        .into_iter()
        .map(|n| n * magnitude)
        .map(|step| (step, step * (max / step).ceil()))
        .find(|(step, top)| top / step <= MAX_TICKS)
        .unwrap()
}

pub fn thousands(v: f64) -> String {
    let digits = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0.0 { format!("-{out}") } else { out }
}

fn compact(v: f64) -> String {
    if v >= 1e6 {
        format!("{}M", trim(v / 1e6))
    } else if v >= 1e3 {
        format!("{}k", trim(v / 1e3))
    } else {
        trim(v)
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").unwrap_or(&s).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers() {
        assert_eq!(thousands(66478.0), "66,478");
        assert_eq!(thousands(999.0), "999");
        assert_eq!(compact(25000.0), "25k");
        assert_eq!(compact(2500.0), "2.5k");
    }

    #[test]
    fn scales_are_tight_and_round() {
        assert_eq!(nice_scale(112_000.0), (20_000.0, 120_000.0));
        assert_eq!(nice_scale(66_478.0), (20_000.0, 80_000.0));
        assert_eq!(nice_scale(0.7), (0.2, 0.8));
        assert_eq!(nice_scale(5.0), (1.0, 5.0));
    }
}
