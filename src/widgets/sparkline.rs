use cosmic::iced::widget::canvas::{Cache, Frame, Geometry, Path, Stroke};
use cosmic::iced::{Color, Point, Rectangle, Size};
use cosmic::widget::canvas::Program;
use cosmic::{Renderer, Theme};

/// How sample values are normalized into the sparkline's vertical range.
#[derive(Clone, Copy)]
pub enum Scale {
    /// 0..=100 (percentages).
    Percent,
    /// Auto-scales to the largest value in the visible window.
    AutoMax,
}

/// Renders a filled-area sparkline.
///
/// Values are rendered chronologically left → right; the most recent value is on the right edge.
/// `tint` is `None` to fall back to the theme accent.
pub struct Sparkline<'a> {
    samples: Vec<f32>,
    capacity: usize,
    cache: &'a Cache,
    tint: Option<Color>,
    scale: Scale,
}

impl<'a> Sparkline<'a> {
    pub fn new(samples: Vec<f32>, capacity: usize, cache: &'a Cache) -> Self {
        Self {
            samples,
            capacity,
            cache,
            tint: None,
            scale: Scale::Percent,
        }
    }

    pub fn tint(mut self, color: Color) -> Self {
        self.tint = Some(color);
        self
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }
}

impl<'a, Message> Program<Message, Theme, Renderer> for Sparkline<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: cosmic::iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            draw_sparkline(
                frame,
                bounds.size(),
                &self.samples,
                self.capacity,
                self.tint,
                self.scale,
                theme,
            );
        });
        vec![geometry]
    }
}

fn draw_sparkline(
    frame: &mut Frame,
    size: Size,
    samples: &[f32],
    capacity: usize,
    tint: Option<Color>,
    scale: Scale,
    theme: &Theme,
) {
    if samples.is_empty() || size.width <= 0.0 || size.height <= 0.0 {
        return;
    }

    let stroke_color = tint.unwrap_or_else(|| {
        let accent = theme.cosmic().accent_color();
        Color {
            r: accent.red,
            g: accent.green,
            b: accent.blue,
            a: 1.0,
        }
    });
    let fill_color = Color {
        a: 0.35,
        ..stroke_color
    };

    let n = samples.len().min(capacity);
    let denom = (capacity.saturating_sub(1)).max(1) as f32;

    // Right-align the samples: the latest is on the right edge.
    let offset = (capacity - n) as f32;

    // 1.0 floor on AutoMax prevents division-by-zero when all samples are zero.
    let max = match scale {
        Scale::Percent => 100.0,
        Scale::AutoMax => samples.iter().cloned().fold(1.0f32, f32::max),
    };

    let to_point = |i: usize, v: f32| -> Point {
        let x = ((i as f32 + offset) / denom) * size.width;
        let normalized = v.max(0.0).min(max) / max;
        let y = size.height - normalized * size.height;
        Point::new(x, y)
    };

    // Filled area path.
    let fill_path = Path::new(|p| {
        let first = to_point(0, samples[0]);
        p.move_to(Point::new(first.x, size.height));
        for (i, v) in samples.iter().enumerate() {
            let pt = to_point(i, *v);
            p.line_to(pt);
        }
        let last_x = to_point(n - 1, samples[n - 1]).x;
        p.line_to(Point::new(last_x, size.height));
        p.close();
    });
    frame.fill(&fill_path, fill_color);

    // Stroked top edge.
    let stroke_path = Path::new(|p| {
        let first = to_point(0, samples[0]);
        p.move_to(first);
        for (i, v) in samples.iter().enumerate().skip(1) {
            p.line_to(to_point(i, *v));
        }
    });
    frame.stroke(
        &stroke_path,
        Stroke::default().with_color(stroke_color).with_width(1.5),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkline_accepts_auto_max_scale() {
        let cache = Cache::default();
        let _sparkline = Sparkline::new(vec![1.0, 2.0, 3.0], 60, &cache).scale(Scale::AutoMax);
    }

    #[test]
    fn sparkline_defaults_to_percent_scale() {
        let cache = Cache::default();
        let _sparkline = Sparkline::new(vec![10.0, 50.0, 100.0], 60, &cache);
    }
}
