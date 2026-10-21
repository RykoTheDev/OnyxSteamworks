use std::time::Instant;

const MAX_SAMPLES: usize = 60;

pub struct SpeedTracker {
    samples: Vec<f32>,
    last: Option<(f32, f32)>,
    fraction: f32,
    rate: f32,
}

impl SpeedTracker {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            last: None,
            fraction: 0.0,
            rate: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.last = None;
        self.fraction = 0.0;
        self.rate = 0.0;
    }

    pub fn record(&mut self, fraction: f32, started: Option<Instant>) {
        let Some(started) = started else {
            return;
        };
        let now = started.elapsed().as_secs_f32();
        self.fraction = fraction;
        if let Some((prev_t, prev_f)) = self.last
            && now > prev_t
            && fraction >= prev_f
        {
            let rate = (fraction - prev_f) / (now - prev_t);
            self.rate = self.rate * 0.6 + rate * 0.4;
            self.samples.push(self.rate);
            if self.samples.len() > MAX_SAMPLES {
                self.samples.remove(0);
            }
        }
        self.last = Some((now, fraction));
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn has_data(&self) -> bool {
        self.samples.len() >= 2 && self.rate > 1e-5
    }

    pub fn eta_secs(&self) -> Option<u64> {
        if self.rate <= 1e-5 {
            return None;
        }
        let remaining = (1.0 - self.fraction).max(0.0);
        Some((remaining / self.rate) as u64)
    }
}
