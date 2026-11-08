use std::time::Duration;

const REPORT_EVERY_FRAMES: usize = 120;

#[derive(Clone, Copy, Default)]
pub struct FrameMetrics {
    pub frame: Duration,
    pub update: Duration,
    pub vertex: Duration,
    pub rasterization: Duration,
    pub surface_update: Duration,
    pub triangles: usize,
    pub culled_triangles: usize,
    pub near_clipped_triangles: usize,
    pub near_rejected_triangles: usize,
    pub covered_fragments: usize,
    pub depth_rejected: usize,
    pub shaded_fragments: usize,
    pub objects: usize,
    pub culled_objects: usize,
    pub surface_updates: usize,
    pub lod_switches: usize,
}

#[derive(Default)]
pub struct PerformanceMonitor {
    samples: Vec<FrameMetrics>,
}

impl PerformanceMonitor {
    pub fn record(&mut self, sample: FrameMetrics) {
        self.samples.push(sample);
        if self.samples.len() < REPORT_EVERY_FRAMES {
            return;
        }

        let count = self.samples.len() as f64;
        let average = |value: fn(&FrameMetrics) -> Duration| {
            self.samples
                .iter()
                .map(value)
                .sum::<Duration>()
                .as_secs_f64()
                * 1000.0
                / count
        };
        let average_count = |value: fn(&FrameMetrics) -> usize| {
            self.samples.iter().map(value).sum::<usize>() as f64 / count
        };
        let mut frame_times: Vec<_> = self.samples.iter().map(|sample| sample.frame).collect();
        frame_times.sort_unstable();
        let p95 = percentile_95(&frame_times).as_secs_f64() * 1000.0;
        let frame = average(|sample| sample.frame);

        eprintln!(
            "perf: {:.1} CPU FPS | frame avg {:.2} ms p95 {:.2} ms | update {:.2} ms | surface {:.2} ms | vertex {:.2} ms | raster+shade {:.2} ms | objects {:.0} | objects culled {:.0} | surfaces updated {:.1} | LOD switches {:.1} | triangles {:.0} | culled {:.0} | near clipped {:.0} | near rejected {:.0} | covered {:.0} | depth rejected {:.0} | shaded {:.0}",
            1000.0 / frame.max(f64::EPSILON),
            frame,
            p95,
            average(|sample| sample.update),
            average(|sample| sample.surface_update),
            average(|sample| sample.vertex),
            average(|sample| sample.rasterization),
            average_count(|sample| sample.objects),
            average_count(|sample| sample.culled_objects),
            average_count(|sample| sample.surface_updates),
            average_count(|sample| sample.lod_switches),
            average_count(|sample| sample.triangles),
            average_count(|sample| sample.culled_triangles),
            average_count(|sample| sample.near_clipped_triangles),
            average_count(|sample| sample.near_rejected_triangles),
            average_count(|sample| sample.covered_fragments),
            average_count(|sample| sample.depth_rejected),
            average_count(|sample| sample.shaded_fragments),
        );
        self.samples.clear();
    }
}

fn percentile_95(sorted: &[Duration]) -> Duration {
    sorted[((sorted.len() as f32 * 0.95).ceil() as usize).saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_the_nearest_rank() {
        let samples: Vec<_> = (1..=100).map(Duration::from_millis).collect();
        assert_eq!(percentile_95(&samples), Duration::from_millis(95));
    }
}
