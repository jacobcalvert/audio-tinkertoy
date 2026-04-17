use eframe::egui;

/// Represents a single waveform with sample points.
/// The sample points define amplitude values at evenly spaced time intervals.
#[derive(Clone, Debug)]
pub struct WaveformData {
    /// Number of sample points (2 to 1024).
    pub sample_count: usize,
    /// Amplitude values at each sample point, normalized to [-1.0, 1.0].
    pub samples: Vec<f32>,
}

impl WaveformData {
    /// Create a new waveform with the given sample count, initialized to zero.
    pub fn new(sample_count: usize) -> Self {
        Self {
            sample_count,
            samples: vec![0.0_f32; sample_count],
        }
    }

    /// Create a sine wave with the given sample count and number of full cycles.
    pub fn sine_wave(sample_count: usize, num_cycles: usize) -> Self {
        let mut samples = Vec::with_capacity(sample_count);
        for i in 0..sample_count {
            let t = i as f32 / sample_count as f32;
            samples.push((2.0 * std::f32::consts::PI * num_cycles as f32 * t).sin());
        }
        Self {
            sample_count,
            samples,
        }
    }

    /// Interpolate a smooth waveform from the sample points using cubic Hermite interpolation.
    /// Returns a full-resolution buffer suitable for audio synthesis.
    pub fn interpolate_to(&self, resolution: usize) -> Vec<f32> {
        if self.samples.is_empty() {
            return vec![0.0; resolution];
        }

        let mut result = vec![0.0_f32; resolution];
        let n = self.samples.len();

        if n == 1 {
            result.fill(self.samples[0]);
            return result;
        }

        if n == 2 {
            for i in 0..resolution {
                let t = i as f32 / resolution as f32;
                result[i] = self.samples[0] * (1.0 - t) + self.samples[1] * t;
            }
            return result;
        }

        // Compute tangents using a simple central difference approach
        let mut tangents = vec![0.0_f32; n];
        tangents[0] = self.samples[1] - self.samples[0];
        tangents[n - 1] = self.samples[n - 1] - self.samples[n - 2];
        for i in 1..n - 1 {
            tangents[i] = (self.samples[i + 1] - self.samples[i - 1]) * 0.5;
        }

        // Cubic Hermite interpolation
        for i in 0..resolution {
            let t = i as f32 / resolution as f32;
            let pos = t * (n - 1) as f32;
            let idx = pos.floor() as usize;
            let frac = pos - idx as f32;

            if idx >= n - 1 {
                result[i] = self.samples[n - 1];
            } else {
                let a = self.samples[idx];
                let b = self.samples[idx + 1];
                let t0 = tangents[idx];
                let t1 = tangents[idx + 1];

                // Cubic Hermite blend
                let h00 = (1.0 + 2.0 * frac) * (1.0 - frac) * (1.0 - frac);
                let h10 = frac * (1.0 - frac) * (1.0 - frac);
                let h01 = frac * frac * (3.0 - 2.0 * frac);
                let h11 = frac * frac * (frac - 1.0);

                result[i] = a * h00 + t0 * h10 + b * h01 + t1 * h11;
            }
        }

        result
    }
}

/// The waveform editor UI component.
/// Handles drawing the waveform canvas and managing sample point interactions.
pub struct WaveformEditor {
    /// Current waveform data.
    pub waveform: WaveformData,
    /// Default waveform (sine wave) for reset functionality.
    default_waveform: WaveformData,
    /// Number of sample points (2 to 1024).
    pub sample_count: usize,
    /// Playback frequency in Hz.
    pub playback_frequency: f32,
    /// Master volume (0.0 to 1.0).
    pub volume: f32,
    /// Index of the currently selected/edited sample point.
    selected_point: Option<usize>,
    /// Whether the mouse is currently dragging a sample point.
    is_dragging: bool,
}

impl WaveformEditor {
    /// Create a new WaveformEditor initialized with a sine wave.
    pub fn new() -> Self {
        let initial_samples = 256;
        let waveform = WaveformData::sine_wave(initial_samples, 1);
        Self {
            waveform: waveform.clone(),
            default_waveform: waveform,
            sample_count: initial_samples,
            playback_frequency: 440.0, // A4 note
            volume: 0.5,
            selected_point: None,
            is_dragging: false,
        }
    }

    /// Reset the waveform to a default sine wave with current sample count.
    pub fn reset_to_sine(&mut self) {
        self.waveform = WaveformData::sine_wave(self.sample_count, 1);
        self.default_waveform = self.waveform.clone();
        self.selected_point = None;
        self.is_dragging = false;
    }

    /// Update the waveform when sample count changes.
    pub fn update_sample_count(&mut self) {
        // When sample count changes, reinitialize the waveform
        self.waveform = WaveformData::new(self.sample_count);
        // Preserve the shape by scaling the previous samples
        if self.sample_count > 2 {
            let old_samples = &self.default_waveform.samples;
            let old_count = self.default_waveform.sample_count;
            for i in 0..self.sample_count {
                let src_idx = (i as f32 * old_count as f32 / self.sample_count as f32) as usize;
                self.waveform.samples[i] = old_samples[src_idx.min(old_count - 1)];
            }
            self.default_waveform = self.waveform.clone();
        }
        self.selected_point = None;
    }

    /// Draw the waveform canvas UI and return the response for hit detection.
    fn draw_waveform_canvas(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let ctx = ui.ctx().clone();
        let available = ui.available_rect_before_wrap();
        let desired_size = egui::vec2(available.width(), available.height());
        let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::click_and_drag());

        let canvas = response.rect;
        let canvas_width = canvas.width();

        let canvas_height = canvas.height();
        let center_y = canvas.top() + canvas_height / 2.0;

        // Draw zero line
        let zero_line_color = egui::Color32::from_white_alpha(60);
        painter.line(
            vec![egui::pos2(canvas.left(), center_y), egui::pos2(canvas.right(), center_y)],
            egui::Stroke::new(1.0, zero_line_color),
        );

        // Amplitude labels
        let font = egui::FontId::monospace(10.0);
        let label_color = egui::Color32::from_white_alpha(150);
        painter.text(
            egui::pos2(canvas.left() + 4.0, canvas.top() + 10.0),
            egui::Align2::LEFT_TOP,
            "+1.0",
            font.clone(),
            label_color,
        );
        painter.text(
            egui::pos2(canvas.left() + 4.0, center_y - 4.0),
            egui::Align2::LEFT_BOTTOM,
            " 0",
            font.clone(),
            label_color,
        );
        painter.text(
            egui::pos2(canvas.left() + 4.0, canvas.bottom() - 5.0),
            egui::Align2::LEFT_BOTTOM,
            "-1.0",
            font.clone(),
            label_color,
        );

        // Time labels
        let time_font = egui::FontId::monospace(9.0);
        let time_color = egui::Color32::from_white_alpha(100);
        painter.text(
            egui::pos2(canvas.left() + 4.0, canvas.bottom() - 16.0),
            egui::Align2::LEFT_BOTTOM,
            "0",
            time_font.clone(),
            time_color,
        );
        painter.text(
            egui::pos2(canvas.right() - 4.0, canvas.bottom() - 16.0),
            egui::Align2::RIGHT_BOTTOM,
            "T",
            time_font,
            time_color,
        );

        // Draw sample points and waveform line
        let samples = &self.waveform.samples;
        let n = samples.len();

        if n >= 2 {
            // Build the waveform path for smooth rendering
            let mut points = Vec::with_capacity(n);
            for i in 0..n {
                let t = i as f32 / (n - 1) as f32;
                let x = canvas.left() + t * canvas_width;
                let y = center_y - samples[i] * (canvas_height / 2.0 - 14.0);
                points.push(egui::pos2(x, y));
            }

            // Draw the waveform line with a glow effect
            // Outer glow (wider, more transparent)
            painter.line(points.clone(), egui::Stroke::new(6.0, egui::Color32::from_rgba_premultiplied(0, 150, 255, 60)));
            // Main line
            painter.line(points, egui::Stroke::new(2.0, egui::Color32::from_rgba_premultiplied(0, 220, 255, 240)));

            // Draw sample point handles
            for i in 0..n {
                let t = i as f32 / (n - 1) as f32;
                let x = canvas.left() + t * canvas_width;
                let y = center_y - samples[i] * (canvas_height / 2.0 - 14.0);

                let is_selected = self.selected_point == Some(i);
                let radius = if is_selected { 7.0 } else { 4.0 };
                let color = if is_selected {
                    egui::Color32::from_rgba_premultiplied(255, 255, 0, 220)
                } else {
                    egui::Color32::from_rgba_premultiplied(0, 200, 255, 180)
                };
                let stroke = if is_selected {
                    egui::Stroke::new(2.0, egui::Color32::YELLOW)
                } else {
                    egui::Stroke::new(1.0, egui::Color32::WHITE)
                };

                // Draw outline circle
                painter.circle_stroke(egui::pos2(x, y), radius, stroke);
                // Fill inner circle
                painter.circle_filled(egui::pos2(x, y), radius - 1.0, color);
            }
        }

        // Now handle mouse interaction using egui's drag detection API
        self.handle_mouse_interaction(&response, &ctx);

        response
    }

    /// Handle mouse interaction on the canvas.
    ///
    /// Uses egui's built-in drag detection (response.drag_started(), response.dragged(),
    /// response.drag_stopped()) rather than manual pointer state checks. This is critical
    /// because `response.hovered()` returns false when egui is handling a drag internally,
    /// and `response.clicked()` only fires on press+release, not during an ongoing drag.
    fn handle_mouse_interaction(&mut self, response: &egui::Response, ctx: &egui::Context) {
        let canvas = response.rect;
        let canvas_width = canvas.width();

        // Set cursor to pointing hand when pointer is over the canvas
        // Use contains_pointer() instead of hovered() since hovered() returns
        // false when egui is handling interaction internally.
        if response.contains_pointer() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Use egui's built-in drag detection API instead of manual
        // pointer state checks. This works correctly even when egui
        // has taken over the interaction.
        if response.drag_started() {
            // A drag just started — determine which sample point was clicked
            if let Some(pos) = response.interact_pointer_pos() {
                let t = (pos.x - canvas.left()) / canvas_width;
                let sample_idx = (t * (self.sample_count - 1) as f32).round() as usize;
                let sample_idx = sample_idx.min(self.sample_count - 1);
                self.selected_point = Some(sample_idx);
                self.is_dragging = true;
            }
        }

        if self.is_dragging && response.dragged() {
            // We're actively dragging — update the sample at the current pointer position
            if let Some(pos) = response.interact_pointer_pos() {
                if pos.x >= canvas.left() && pos.x <= canvas.right() {
                    let t = (pos.x - canvas.left()) / canvas_width;
                    let sample_idx = (t * (self.sample_count - 1) as f32).round() as usize;
                    let sample_idx = sample_idx.min(self.sample_count - 1);
                    let amplitude_t = (pos.y - canvas.top()) / canvas.height();
                    let amplitude = (1.0 - amplitude_t * 2.0).clamp(-1.0, 1.0);
                    self.waveform.samples[sample_idx] = amplitude;
                    ctx.request_repaint();
                }
            }
        }

        if self.is_dragging && response.drag_stopped() {
            // Drag ended — clean up
            self.is_dragging = false;
            self.selected_point = None;
        }
    }
}

impl egui::Widget for &mut WaveformEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.heading("Waveform Editor");
        ui.label("Click and drag sample points to reshape the waveform.");
        ui.separator();

        self.draw_waveform_canvas(ui)
    }
}
