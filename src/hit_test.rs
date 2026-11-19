use crate::ListState;

/// Result of a hit-test within the list's inner area.
#[derive(Debug, PartialEq)]
pub enum Hit {
    /// Area hit, but no item
    Area,
    /// Specific item hit at the given index
    Item(usize),
}

impl ListState {
    /// Hit-test using last rendered view self. Used for mouse click handling.
    ///
    /// Returns `Some(index)` if a visible item was hit, otherwise `None`.
    #[must_use]
    pub fn hit_test(&self, mouse_x: u16, mouse_y: u16) -> Option<Hit> {
        let sizes = self.visible_main_axis_sizes();

        if sizes.is_empty() {
            return None;
        }

        let inner_area = self.inner_area();
        let scroll_axis = self.last_scroll_axis();

        let point_in_rect = |rect: ratatui_core::layout::Rect, x: u16, y: u16| {
            x >= rect.left() && x < rect.right() && y >= rect.top() && y < rect.bottom()
        };

        if !point_in_rect(inner_area, mouse_x, mouse_y) {
            return None;
        }

        let (_, cross_axis_size) = scroll_axis.sizes(inner_area);
        let gap_size = self.gap_size();

        let (scroll_axis_pos, cross_axis_pos) = scroll_axis.origin(inner_area);
        let mut scroll_axis_pos = scroll_axis_pos.saturating_add(self.main_axis_offset());

        let start_index = self.scroll_offset_index();
        let mut index = start_index;
        let mut previous_visible = false;

        while let Some(visible_main_axis_size) = sizes.get(&index).copied() {
            // Skip over the gap that is rendered between two visible items.
            if previous_visible && visible_main_axis_size > 0 {
                scroll_axis_pos = scroll_axis_pos.saturating_add(gap_size);
            }
            previous_visible = visible_main_axis_size > 0;

            let rect = scroll_axis.rect(
                scroll_axis_pos,
                cross_axis_pos,
                visible_main_axis_size,
                cross_axis_size,
            );

            if point_in_rect(rect, mouse_x, mouse_y) {
                return Some(Hit::Item(index));
            }

            scroll_axis_pos = scroll_axis_pos.saturating_add(visible_main_axis_size);
            index += 1;
        }

        Some(Hit::Area)
    }
}

#[cfg(test)]
mod tests {
    use crate::hit_test::Hit;
    use crate::{ListBuilder, ListState, ListView, ScrollAxis, ScrollDirection};
    use ratatui::buffer::Buffer;
    use ratatui::prelude::{Rect, StatefulWidget, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    fn build_list(
        item_count: usize,
    ) -> (
        Rect,
        Buffer,
        ListView<'static, Paragraph<'static>>,
        ListState,
    ) {
        // Build a list with items of height 3 lines, vertical scrolling
        let builder = ListBuilder::new(|context| {
            let text = format!("Item {0}", context.index);
            let mut item = Line::from(text);

            if context.index % 2 == 0 {
                item.style = Style::default();
            } else {
                item.style = Style::default();
            };

            if context.is_selected {
                let mut spans = item.spans;
                spans.insert(0, Span::from(">>"));
                item = Line::from(spans);
            };

            let style = item.style;
            let lines = vec![item, Line::from(""), Line::from("")];
            let paragraph = Paragraph::new(lines).style(style);
            (paragraph, 3)
        });

        let area = Rect::new(0, 0, 5, (item_count as u16) * 3);
        let buf = Buffer::empty(area);
        let list = ListView::new(builder, item_count).scroll_axis(ScrollAxis::Vertical);
        let state = ListState::default();
        (area, buf, list, state)
    }

    #[test]
    fn hit_test_points_in_each_visible_item() {
        // given: 3 items, height 3 each
        let (area, mut buf, list, mut state) = build_list(3);
        list.render(area, &mut buf, &mut state);

        let sizes = state.visible_main_axis_sizes().clone();
        let mut scroll_pos = state.inner_area().top();
        let cross_pos = state.inner_area().left();
        let cross_size = state.inner_area().width;

        let mut expected_index = state.scroll_offset_index();
        while let Some(visible) = sizes.get(&expected_index) {
            // middle point within the item's rect
            let mid_y = scroll_pos.saturating_add(visible / 2);
            let mid_x = cross_pos.saturating_add(cross_size / 2);
            assert_eq!(
                state.hit_test(mid_x, mid_y),
                Some(Hit::Item(expected_index))
            );
            scroll_pos = scroll_pos.saturating_add(*visible);
            expected_index += 1;
        }
    }

    #[test]
    fn hit_test_respects_inner_area_offset() {
        // given: render not at origin
        let (_, mut buf, list, mut state) = build_list(3);
        let area = Rect::new(10, 5, 5, 9);
        list.render(area, &mut buf, &mut state);

        let inner = state.inner_area();
        let sizes = state.visible_main_axis_sizes().clone();

        let first_visible = sizes
            .get(&state.scroll_offset_index())
            .copied()
            .unwrap_or(0);
        let mid_y = inner.top() + first_visible / 2;
        let mid_x = inner.left() + inner.width / 2;
        assert_eq!(
            state.hit_test(mid_x, mid_y),
            Some(Hit::Item(state.scroll_offset_index()))
        );
    }

    #[test]
    fn hit_test_with_truncated_first_item() {
        // given: area height 8, select last element so the first visible item is truncated
        let (area, mut buf, list, mut state) = build_list(3);
        state.select(Some(2));
        list.render(
            Rect::new(area.left(), area.top(), area.width, 8),
            &mut buf,
            &mut state,
        );

        let inner = state.inner_area();
        let sizes = state.visible_main_axis_sizes().clone();

        let mut scroll_pos = inner.top();
        let mut index = state.scroll_offset_index();
        while let Some(visible) = sizes.get(&index) {
            let mid_y = scroll_pos.saturating_add(visible / 2);
            let mid_x = inner.left() + inner.width / 2;
            assert_eq!(state.hit_test(mid_x, mid_y), Some(Hit::Item(index)));
            scroll_pos = scroll_pos.saturating_add(*visible);
            index += 1;
        }
    }

    #[test]
    fn hit_test_returns_area_inside_gap_between_items() {
        // given: 3 items of 2 rows with a gap of 1 row on a viewport of 6 rows
        let builder = ListBuilder::new(|_| (Paragraph::new(""), 2));
        let area = Rect::new(0, 0, 5, 6);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(builder, 3).gap_size(1);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then: rows 0-1 contain item 0, row 2 the gap, rows 3-4 item 1
        assert_eq!(state.hit_test(2, 0), Some(Hit::Item(0)));
        assert_eq!(state.hit_test(2, 2), Some(Hit::Area));
        assert_eq!(state.hit_test(2, 3), Some(Hit::Item(1)));
        assert_eq!(state.hit_test(2, 5), Some(Hit::Area));
    }

    #[test]
    fn hit_test_with_gaps_in_backward_direction() {
        // given: 2 items of 3 rows with a gap of 1 row, aligned to the bottom
        let builder = ListBuilder::new(|_| (Paragraph::new(""), 3));
        let area = Rect::new(0, 0, 5, 9);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(builder, 2)
            .gap_size(1)
            .scroll_direction(ScrollDirection::Backward);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then: rows 2-4 contain item 0, row 5 the gap, rows 6-8 item 1
        assert_eq!(state.hit_test(2, 1), Some(Hit::Area));
        assert_eq!(state.hit_test(2, 2), Some(Hit::Item(0)));
        assert_eq!(state.hit_test(2, 5), Some(Hit::Area));
        assert_eq!(state.hit_test(2, 6), Some(Hit::Item(1)));
    }
}
