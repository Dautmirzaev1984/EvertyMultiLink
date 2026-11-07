use everty_ui::LogicalRect;
#[derive(Clone, Copy)]
pub struct ScrollBar {
    pub track: LogicalRect,
    pub thumb: LogicalRect,
    pub max: usize,
}
impl ScrollBar {
    pub fn new(track: LogicalRect, total: usize, visible: usize, offset: usize) -> Option<Self> {
        let visible = visible.max(1);
        if total <= visible {
            return None;
        }
        let max = total - visible;
        let height = (track.size.height.0 * visible as f64 / total as f64)
            .max(28.0)
            .min(track.size.height.0);
        let top =
            track.top() + (track.size.height.0 - height) * offset.min(max) as f64 / max as f64;
        Some(Self {
            track,
            thumb: LogicalRect::from_xywh(track.left(), top, track.size.width.0, height),
            max,
        })
    }
    pub fn offset_at(&self, top: f64) -> usize {
        let travel = self.track.size.height.0 - self.thumb.size.height.0;
        if travel <= 0.0 {
            0
        } else {
            (((top - self.track.top()) / travel).clamp(0.0, 1.0) * self.max as f64).round() as usize
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrolling_clamps_and_reaches_last_full_page() {
        let r = LogicalRect::from_xywh(0.0, 100.0, 8.0, 300.0);
        assert!(ScrollBar::new(r, 5, 5, 0).is_none());
        let b = ScrollBar::new(r, 100, 10, 90).unwrap();
        assert_eq!(b.offset_at(999.0), 90);
        assert_eq!(b.offset_at(-99.0), 0);
        assert!((b.thumb.bottom() - r.bottom()).abs() < 0.01);
    }
}
