use cosmic::iced::widget::canvas::{Cache, Frame, Geometry, Path, Stroke};
use cosmic::iced::{Color, Point, Rectangle, Size};
use cosmic::widget::canvas::Program;
use cosmic::{Renderer, Theme};

/// Renders a filled-area sparkline using values in 0..=100.
///
/// Values are rendered chronologically left → right; the most recent value is on the right edge.
pub struct Sparkline<'a> {
    samples: Vec<f32>,
    capacity: usize,
    cache: &'a Cache,
}

impl<'a> Sparkline<'a> {
    pub fn new(samples: Vec<f32>, capacity: usize, cache: &'a Cache) -> Self {
        Self {
            samples,
            capacity,
            cache,
        }
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
            draw_sparkline(frame, bounds.size(), &self.samples, self.capacity, theme);
        });
        vec![geometry]
    }
}

fn draw_sparkline(
    frame: &mut Frame,
    size: Size,
    samples: &[f32],
    capacity: usize,
    theme: &Theme,
) {
    if samples.is_empty() || size.width <= 0.0 || size.height <= 0.0 {
        return;
    }

    let cosmic = theme.cosmic();
    let accent = cosmic.accent_color();
    let stroke_color = Color {
        r: accent.red,
        g: accent.green,
        b: accent.blue,
        a: 1.0,
    };
    let fill_color = Color {
        r: accent.red,
        g: accent.green,
        b: accent.blue,
        a: 0.35,
    };

    let n = samples.len().min(capacity);
    let denom = (capacity.saturating_sub(1)).max(1) as f32;

    // Right-align the samples: the latest is on the right edge.
    let offset = (capacity - n) as f32;

    let to_point = |i: usize, v: f32| -> Point {
        let x = ((i as f32 + offset) / denom) * size.width;
        let clamped = v.clamp(0.0, 100.0);
        let y = size.height - (clamped / 100.0 * size.height);
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
