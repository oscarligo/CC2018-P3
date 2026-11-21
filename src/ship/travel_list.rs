//! Display order only: destination indices still refer to the world's original list.
use crate::scene::world::TravelTarget;

const FEATURED: [&str; 4] = ["Kashyyyk", "Coruscant", "Ureski", "Alfa Riddertal"];
const OTHER_ROWS: usize = 4;

pub(super) struct TravelList {
    pub order: Vec<usize>,
    pub featured: usize,
}

impl TravelList {
    pub fn new(targets: &[TravelTarget]) -> Self {
        let mut order = Vec::with_capacity(targets.len());
        for name in FEATURED {
            if let Some(index) = targets.iter().position(|target| target.name == name) {
                order.push(index);
            }
        }
        let featured = order.len();
        for index in 0..targets.len() {
            if !order[..featured].contains(&index) {
                order.push(index);
            }
        }
        Self { order, featured }
    }

    pub fn page(&self, selected: usize) -> usize {
        selected.saturating_sub(self.featured) / OTHER_ROWS
    }

    pub fn pages(&self) -> usize {
        (self.order.len() - self.featured)
            .div_ceil(OTHER_ROWS)
            .max(1)
    }

    /// (selection index, original destination index, logical screen Y).
    pub fn rows(&self, selected: usize) -> Vec<(usize, usize, i32)> {
        let mut rows = self
            .order
            .iter()
            .take(self.featured)
            .enumerate()
            .map(|(index, &original)| (index, original, 146 + index as i32 * 31))
            .collect::<Vec<_>>();
        let first = self.featured + self.page(selected) * OTHER_ROWS;
        rows.extend(
            self.order
                .iter()
                .enumerate()
                .skip(first)
                .take(OTHER_ROWS)
                .map(|(index, &original)| (index, original, 310 + (index - first) as i32 * 31)),
        );
        rows
    }

    pub fn hit(&self, x: f32, y: f32, selected: usize) -> Option<usize> {
        if !(128.0..672.0).contains(&x) {
            return None;
        }
        self.rows(selected)
            .into_iter()
            .find_map(|(index, _, top)| (y >= top as f32 && y < (top + 28) as f32).then_some(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn featured_worlds_come_first_without_changing_destination_indices() {
        let targets = crate::scene::build_solar_system().travel_targets();
        let list = TravelList::new(&targets);
        assert_eq!(list.featured, 4);
        assert_eq!(
            list.order[..4]
                .iter()
                .map(|&i| targets[i].name)
                .collect::<Vec<_>>(),
            FEATURED
        );
        let mut indices = list.order.clone();
        indices.sort_unstable();
        assert_eq!(indices, (0..targets.len()).collect::<Vec<_>>());
        for selected in 0..targets.len() {
            let rows = list.rows(selected);
            assert!(rows.iter().any(|&(i, _, _)| i == selected));
            assert!(
                rows.iter()
                    .take(4)
                    .map(|&(_, i, _)| targets[i].name)
                    .eq(FEATURED)
            );
            for (index, original, top) in rows {
                assert_eq!(list.hit(200., top as f32 + 14., selected), Some(index));
                assert_eq!(list.order[index], original);
            }
        }
        assert_eq!(list.hit(200., 285., 0), None); // Category headings are not selectable.
    }

    #[test]
    fn missing_featured_worlds_and_empty_lists_remain_safe() {
        let mut targets = crate::scene::build_solar_system().travel_targets();
        targets.retain(|t| !matches!(t.name, "Kashyyyk" | "Ureski"));
        let list = TravelList::new(&targets);
        assert_eq!(list.featured, 2);
        assert_eq!(targets[list.order[0]].name, "Coruscant");
        assert_eq!(targets[list.order[1]].name, "Alfa Riddertal");
        let empty = TravelList::new(&[]);
        assert!(empty.rows(0).is_empty());
        assert_eq!(empty.pages(), 1);
    }
}
