use std::collections::HashMap;

use ratatui_core::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Style, Styled},
    widgets::{StatefulWidget, Widget},
};
use ratatui_widgets::block::Block;
use ratatui_widgets::block::BlockExt;
use ratatui_widgets::scrollbar::Scrollbar;

use crate::{
    utils::{compute_viewport_layout, ViewportElement},
    ListState,
};

/// A struct representing a list view.
/// The widget displays a scrollable list of items.
#[allow(clippy::module_name_repetitions)]
pub struct ListView<'a, T> {
    /// The total number of items in the list
    pub item_count: usize,

    ///  A `ListBuilder<T>` responsible for constructing the items in the list.
    pub builder: ListBuilder<'a, T>,

    /// Specifies the scroll axis. Either `Vertical` or `Horizontal`.
    pub scroll_axis: ScrollAxis,

    /// Specifies the scroll direction. Either `Forward` or `Backward`.
    pub scroll_direction: ScrollDirection,

    /// The base style of the list view.
    pub style: Style,

    /// The base block surrounding the widget list.
    pub block: Option<Block<'a>>,

    /// The gap size between items. The gap is rendered as empty space only
    /// between items that are visible on the viewport, so there is never a gap
    /// before the first or after the last visible item.
    pub gap_size: u16,

    /// The scrollbar.
    pub scrollbar: Option<Scrollbar<'a>>,

    /// The scroll padding.
    pub(crate) scroll_padding: u16,

    /// Whether infinite scrolling is enabled or not.
    /// Disabled by default.
    pub(crate) infinite_scrolling: bool,
}

impl<'a, T> ListView<'a, T> {
    /// Creates a new `ListView` with a builder an item count.
    #[must_use]
    pub fn new(builder: ListBuilder<'a, T>, item_count: usize) -> Self {
        Self {
            builder,
            item_count,
            scroll_axis: ScrollAxis::Vertical,
            scroll_direction: ScrollDirection::Forward,
            style: Style::default(),
            block: None,
            gap_size: 0,
            scrollbar: None,
            scroll_padding: 0,
            infinite_scrolling: true,
        }
    }

    /// Checks whether the widget list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.item_count == 0
    }

    /// Returns the length of the widget list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.item_count
    }

    /// Sets the block style that surrounds the whole List.
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the gap size between items. The gap is rendered as empty space only
    /// between items that are visible on the viewport.
    #[must_use]
    pub fn gap_size(mut self, gap_size: u16) -> Self {
        self.gap_size = gap_size;
        self
    }

    /// Sets the scrollbar of the List.
    #[must_use]
    pub fn scrollbar(mut self, scrollbar: Scrollbar<'a>) -> Self {
        self.scrollbar = Some(scrollbar);
        self
    }

    /// Set the base style of the List.
    #[must_use]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    /// Set the scroll axis of the list.
    #[must_use]
    pub fn scroll_axis(mut self, scroll_axis: ScrollAxis) -> Self {
        self.scroll_axis = scroll_axis;
        self
    }

    #[must_use]
    pub fn scroll_direction(mut self, scroll_direction: ScrollDirection) -> Self {
        self.scroll_direction = scroll_direction;
        self
    }

    /// Set the scroll padding of the list.
    #[must_use]
    pub fn scroll_padding(mut self, scroll_padding: u16) -> Self {
        self.scroll_padding = scroll_padding;
        self
    }

    /// Specify whether infinite scrolling should be enabled or not.
    #[must_use]
    pub fn infinite_scrolling(mut self, infinite_scrolling: bool) -> Self {
        self.infinite_scrolling = infinite_scrolling;
        self
    }
}

impl<T> Styled for ListView<'_, T> {
    type Item = Self;

    fn style(&self) -> Style {
        self.style
    }

    fn set_style<S: Into<Style>>(mut self, style: S) -> Self::Item {
        self.style = style.into();
        self
    }
}

impl<'a, T: Copy + 'a> From<Vec<T>> for ListView<'a, T> {
    fn from(value: Vec<T>) -> Self {
        let item_count = value.len();
        let builder = ListBuilder::new(move |context| (value[context.index], 1));

        ListView::new(builder, item_count)
    }
}

/// This structure holds information about the item's position, selection
/// status, scrolling behavior, and size along the cross axis.
pub struct ListBuildContext {
    /// The position of the item in the list.
    pub index: usize,

    /// A boolean flag indicating whether the item is currently selected.
    pub is_selected: bool,

    /// Defines the axis along which the list can be scrolled.
    pub scroll_axis: ScrollAxis,

    /// The size of the item along the cross axis.
    pub cross_axis_size: u16,
}

/// A type alias for the closure.
type ListBuilderClosure<'a, T> = dyn Fn(&ListBuildContext) -> (T, u16) + 'a;

/// The builder for constructing list elements in a `ListView<T>`
pub struct ListBuilder<'a, T> {
    closure: Box<ListBuilderClosure<'a, T>>,
}

impl<'a, T> ListBuilder<'a, T> {
    /// Creates a new `ListBuilder` taking a closure as a parameter
    ///
    /// # Example
    /// ```
    /// use ratatui::text::Line;
    /// use tui_widget_list::ListBuilder;
    ///
    /// let builder = ListBuilder::new(|context| {
    ///     let mut item = Line::from(format!("Item {:0}", context.index));
    ///
    ///     // Return the size of the widget along the main axis.
    ///     let main_axis_size = 1;
    ///
    ///     (item, main_axis_size)
    /// });
    /// ```
    pub fn new<F>(closure: F) -> Self
    where
        F: Fn(&ListBuildContext) -> (T, u16) + 'a,
    {
        ListBuilder {
            closure: Box::new(closure),
        }
    }

    /// Method to call the stored closure.
    pub(crate) fn call_closure(&self, context: &ListBuildContext) -> (T, u16) {
        (self.closure)(context)
    }
}

/// Represents the scroll axis of a list.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    /// Indicates vertical scrolling. This is the default.
    #[default]
    Vertical,

    /// Indicates horizontal scrolling.
    Horizontal,
}

impl ScrollAxis {
    /// Returns `(main_axis_size, cross_axis_size)` for the given area.
    pub(crate) fn sizes(self, area: Rect) -> (u16, u16) {
        match self {
            Self::Vertical => (area.height, area.width),
            Self::Horizontal => (area.width, area.height),
        }
    }

    /// Returns `(scroll_axis_pos, cross_axis_pos)` for the given area.
    pub(crate) fn origin(self, area: Rect) -> (u16, u16) {
        match self {
            Self::Vertical => (area.top(), area.left()),
            Self::Horizontal => (area.left(), area.top()),
        }
    }

    /// Builds a `Rect` from axis-agnostic positions and sizes.
    pub(crate) fn rect(
        self,
        scroll_axis_pos: u16,
        cross_axis_pos: u16,
        main_axis_size: u16,
        cross_axis_size: u16,
    ) -> Rect {
        match self {
            Self::Vertical => Rect::new(
                cross_axis_pos,
                scroll_axis_pos,
                cross_axis_size,
                main_axis_size,
            ),
            Self::Horizontal => Rect::new(
                scroll_axis_pos,
                cross_axis_pos,
                main_axis_size,
                cross_axis_size,
            ),
        }
    }
}

/// Represents the scroll direction of a list.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Indicates forward scrolling (top to bottom or left to right). This is the default.
    #[default]
    Forward,

    /// Indicates backward scrolling (bottom to top or right to left).
    Backward,
}

impl<T: Widget> StatefulWidget for ListView<'_, T> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render base style and optional surrounding block
        buf.set_style(area, self.style);
        if let Some(ref block) = self.block {
            block.render(area, buf);
        }
        let inner_area = self.block.inner_if_some(area);

        // Store layout information for post-render queries (e.g. hit testing)
        state.set_num_elements(self.item_count);
        state.set_infinite_scrolling(self.infinite_scrolling);
        state.set_inner_area(inner_area);
        state.set_scroll_axis(self.scroll_axis);
        state.set_gap_size(self.gap_size);

        if self.item_count == 0 {
            return;
        }

        // Resolve which items are visible and how they fit on the viewport
        let (main_axis_size, cross_axis_size) = self.scroll_axis.sizes(inner_area);
        state.set_last_main_axis_size(main_axis_size);
        let (mut scroll_axis_pos, cross_axis_pos) = self.scroll_axis.origin(inner_area);

        let mut viewport = resolve_viewport(
            state,
            &self.builder,
            self.item_count,
            main_axis_size,
            cross_axis_size,
            self.scroll_axis,
            self.scroll_padding,
            self.gap_size,
        );

        let (start, end) = (
            state.view_state.offset,
            viewport.len() + state.view_state.offset,
        );

        // If the whole list fits into the viewport there is nothing to scroll,
        // so the content always stays anchored to the end and does not jump to
        // the start when the first item is selected.
        if start == 0 && end == self.item_count {
            state.set_anchor_to_end(true);
        }

        // Backward direction: align items to the end of the axis if the content
        // is anchored at the end, i.e. after the selection moved below the
        // viewport while scrolling down. Otherwise it sticks to the start of
        // the axis and the unused space stays at the end.
        let mut main_axis_offset = 0;
        if self.scroll_direction == ScrollDirection::Backward && state.anchor_to_end() {
            let mut total_visible: u16 = 0;
            let mut visible_count: u16 = 0;
            for i in start..end {
                if let Some(element) = viewport.get(&i) {
                    let size = element.visible_size();
                    if size > 0 {
                        total_visible = total_visible.saturating_add(size);
                        visible_count = visible_count.saturating_add(1);
                    }
                }
            }
            let total_gap = self
                .gap_size
                .saturating_mul(visible_count.saturating_sub(1));
            main_axis_offset =
                main_axis_size.saturating_sub(total_visible.saturating_add(total_gap));
        }
        scroll_axis_pos += main_axis_offset;
        state.set_main_axis_offset(main_axis_offset);

        // Render each visible item and cache sizes for hit testing
        let mut cached_sizes: HashMap<usize, u16> = HashMap::new();
        let mut cached_total_sizes: HashMap<usize, u16> = HashMap::new();
        let viewport_end = scroll_axis_pos + main_axis_size;
        let mut is_first_visible = true;

        for i in start..end {
            let Some(element) = viewport.remove(&i) else {
                break;
            };

            // Render the gap only between items that are visible. Items at the
            // edge of the viewport are never preceded by a gap.
            if !is_first_visible {
                scroll_axis_pos = scroll_axis_pos.saturating_add(self.gap_size);
            }
            is_first_visible = false;

            cached_total_sizes.insert(i, element.main_axis_size);

            // If the selected item overflows the viewport, apply item_scroll as truncation
            let truncation = if Some(i) == state.selected
                && state.item_scroll() > 0
                && element.main_axis_size > main_axis_size
            {
                Truncation::Top(state.item_scroll())
            } else {
                element.truncation
            };

            let visible_size = element
                .main_axis_size
                .saturating_sub(truncation.value())
                .min(viewport_end.saturating_sub(scroll_axis_pos));

            cached_sizes.insert(i, visible_size);

            render_clipped(
                element.widget,
                self.scroll_axis.rect(
                    scroll_axis_pos,
                    cross_axis_pos,
                    visible_size,
                    cross_axis_size,
                ),
                buf,
                element.main_axis_size,
                &truncation,
                self.style,
                self.scroll_axis,
            );

            scroll_axis_pos += visible_size;
        }

        state.set_visible_main_axis_sizes(cached_sizes);
        state.set_total_main_axis_sizes(cached_total_sizes);

        if let Some(scrollbar) = self.scrollbar {
            scrollbar.render(area, buf, &mut state.scrollbar_state);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_viewport<T>(
    state: &mut ListState,
    builder: &ListBuilder<T>,
    item_count: usize,
    main_axis_size: u16,
    cross_axis_size: u16,
    scroll_axis: ScrollAxis,
    scroll_padding: u16,
    gap_size: u16,
) -> HashMap<usize, ViewportElement<T>> {
    let viewport = compute_viewport_layout(
        state,
        builder,
        item_count,
        main_axis_size,
        cross_axis_size,
        scroll_axis,
        scroll_padding,
        gap_size,
    );
    state.update_scrollbar_state(
        builder,
        item_count,
        main_axis_size,
        cross_axis_size,
        scroll_axis,
        gap_size,
    );
    viewport
}

/// Renders a widget into `buf`, clipping it if partially visible.
fn render_clipped<T: Widget>(
    item: T,
    available_area: Rect,
    buf: &mut Buffer,
    untruncated_size: u16,
    truncation: &Truncation,
    base_style: Style,
    scroll_axis: ScrollAxis,
) {
    if truncation.value() == 0 {
        item.render(available_area, buf);
        return;
    }

    let (width, height) = match scroll_axis {
        ScrollAxis::Vertical => (available_area.width, untruncated_size),
        ScrollAxis::Horizontal => (untruncated_size, available_area.height),
    };
    let mut hidden_buffer = Buffer::empty(Rect {
        x: available_area.left(),
        y: available_area.top(),
        width,
        height,
    });
    hidden_buffer.set_style(hidden_buffer.area, base_style);
    item.render(hidden_buffer.area, &mut hidden_buffer);

    // Copy the visible part from the hidden buffer to the main buffer
    match scroll_axis {
        ScrollAxis::Vertical => {
            let offset = match truncation {
                Truncation::Top(value) => *value,
                _ => 0,
            };
            for y in available_area.top()..available_area.bottom() {
                let y_off = y + offset;
                for x in available_area.left()..available_area.right() {
                    if let Some(to) = buf.cell_mut(Position::new(x, y)) {
                        if let Some(from) = hidden_buffer.cell(Position::new(x, y_off)) {
                            *to = from.clone();
                        }
                    }
                }
            }
        }
        ScrollAxis::Horizontal => {
            let offset = match truncation {
                Truncation::Top(value) => *value,
                _ => 0,
            };
            for x in available_area.left()..available_area.right() {
                let x_off = x + offset;
                for y in available_area.top()..available_area.bottom() {
                    if let Some(to) = buf.cell_mut(Position::new(x, y)) {
                        if let Some(from) = hidden_buffer.cell(Position::new(x_off, y)) {
                            *to = from.clone();
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) enum Truncation {
    #[default]
    None,
    Top(u16),
    Bot(u16),
}

impl Truncation {
    pub(crate) fn value(&self) -> u16 {
        match self {
            Self::Top(value) | Self::Bot(value) => *value,
            Self::None => 0,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ListBuilder;
    use ratatui::widgets::Block;

    use super::*;
    use ratatui::widgets::Borders;

    struct TestItem {}
    impl Widget for TestItem {
        fn render(self, area: Rect, buf: &mut Buffer)
        where
            Self: Sized,
        {
            Block::default().borders(Borders::ALL).render(area, buf);
        }
    }

    fn test_data(total_height: u16) -> (Rect, Buffer, ListView<'static, TestItem>, ListState) {
        let area = Rect::new(0, 0, 5, total_height);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 3);
        (area, Buffer::empty(area), list, ListState::default())
    }

    #[test]
    fn not_truncated() {
        // given
        let (area, mut buf, list, mut state) = test_data(9);

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
            ]),
        )
    }

    #[test]
    fn empty_list() {
        // given
        let area = Rect::new(0, 0, 5, 2);
        let mut buf = Buffer::empty(area);
        let mut state = ListState::default();
        let builder = ListBuilder::new(|_| (TestItem {}, 0));
        let list = ListView::new(builder, 0);

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(buf, Buffer::with_lines(vec!["     ", "     "]))
    }

    #[test]
    fn zero_size() {
        // given
        let (area, mut buf, list, mut state) = test_data(0);

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(buf, Buffer::empty(area))
    }

    #[test]
    fn truncated_bot() {
        // given
        let (area, mut buf, list, mut state) = test_data(8);

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
            ]),
        )
    }

    #[test]
    fn truncated_top() {
        // given
        let (area, mut buf, list, mut state) = test_data(8);
        state.select(Some(2));

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
            ]),
        )
    }

    #[test]
    fn scroll_up() {
        let (area, mut buf, list, mut state) = test_data(8);
        // Select last element and render
        state.select(Some(2));
        list.render(area, &mut buf, &mut state);
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
            ]),
        );

        // Select first element and render
        let (_, mut buf, list, _) = test_data(8);
        state.select(Some(1));
        list.render(area, &mut buf, &mut state);
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
                "┌───┐",
                "│   │",
                "└───┘",
            ]),
        )
    }

    #[test]
    fn scroll_within_large_item() {
        let area = Rect::new(0, 0, 5, 7);
        let builder = ListBuilder::new(|ctx| {
            let size = if ctx.index == 0 { 8 } else { 3 };
            (TestItem {}, size)
        });
        let list = ListView::new(builder, 2);
        let mut state = ListState::default();
        state.select(Some(0));

        // Render: shows top 7 rows
        let mut buf = Buffer::empty(area);
        list.render(area, &mut buf, &mut state);
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "┌───┐",
                "│   │",
                "│   │",
                "│   │",
                "│   │",
                "│   │",
                "│   │",
            ]),
        );

        // next() scrolls within and content_offset becomes 1
        state.next();
        assert_eq!(state.selected, Some(0));
        assert_eq!(state.item_scroll, 1);

        // Render at offset 1: shows rows 1-7
        let mut buf = Buffer::empty(area);
        let builder = ListBuilder::new(|ctx| {
            let size = if ctx.index == 0 { 8 } else { 3 };
            (TestItem {}, size)
        });
        let list = ListView::new(builder, 2);
        list.render(area, &mut buf, &mut state);
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "│   │",
                "│   │",
                "│   │",
                "│   │",
                "│   │",
                "│   │",
                "└───┘",
            ]),
        );

        // next at max offset jumps to item 1
        state.next();
        assert_eq!(state.selected, Some(1));
        assert_eq!(state.item_scroll, 0);
    }

    #[test]
    fn refill_viewport_after_resize() {
        // given: 4 items of 3 rows, the last one is selected (like a chat list)
        let large = Rect::new(0, 0, 5, 9);
        let small = Rect::new(0, 0, 5, 4);
        let full_viewport = Buffer::with_lines(vec![
            "┌───┐",
            "│   │",
            "└───┘",
            "┌───┐",
            "│   │",
            "└───┘",
            "┌───┐",
            "│   │",
            "└───┘",
        ]);

        for direction in [ScrollDirection::Forward, ScrollDirection::Backward] {
            let make_list = || {
                ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 4).scroll_direction(direction)
            };
            let mut state = ListState::default();
            state.select(Some(3));

            // Render with a large viewport
            let mut buf = Buffer::empty(large);
            make_list().render(large, &mut buf, &mut state);
            assert_buffer_eq(buf, full_viewport.clone());

            // Shrink the terminal: only the last two items are visible now
            let mut buf = Buffer::empty(small);
            make_list().render(small, &mut buf, &mut state);
            assert_buffer_eq(
                buf,
                Buffer::with_lines(vec!["└───┘", "┌───┐", "│   │", "└───┘"]),
            );

            // Enlarge the terminal again: the viewport must be filled completely
            let mut buf = Buffer::empty(large);
            make_list().render(large, &mut buf, &mut state);
            assert_buffer_eq(buf, full_viewport.clone());
        }
    }

    #[test]
    fn gap_between_items() {
        // given: 3 items of 3 rows each and a gap of 1 row in between
        let area = Rect::new(0, 0, 5, 11);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 3).gap_size(1);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then: the gap is rendered only between the items, not at the edges
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "┌───┐",
                "│   │",
                "└───┘",
                "     ",
                "┌───┐",
                "│   │",
                "└───┘",
                "     ",
                "┌───┐",
                "│   │",
                "└───┘",
            ]),
        );
    }

    #[test]
    fn no_gap_without_visible_neighbour() {
        // given: viewport of 5 rows, items of 3 rows and a gap of 2 rows.
        // The second item does not fit, so it is not visible at all.
        let area = Rect::new(0, 0, 5, 5);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 3).gap_size(2);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then: the gap is not rendered because the next item is not visible
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec!["┌───┐", "│   │", "└───┘", "     ", "     "]),
        );
    }

    #[test]
    fn gap_before_partially_visible_item() {
        // given: viewport of 6 rows, items of 3 rows and a gap of 2 rows.
        // Only the first row of the second item fits below the gap.
        let area = Rect::new(0, 0, 5, 6);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 3).gap_size(2);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec!["┌───┐", "│   │", "└───┘", "     ", "     ", "┌───┐"]),
        );
    }

    #[test]
    fn no_gap_above_truncated_first_item() {
        // given: viewport of 6 rows, items of 3 rows, gap of 1 row and the last
        // item selected. The first visible item is truncated at the top, so no
        // gap may be rendered above it.
        let area = Rect::new(0, 0, 5, 6);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 3).gap_size(1);
        let mut state = ListState::default();
        state.select(Some(2));

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec!["│   │", "└───┘", "     ", "┌───┐", "│   │", "└───┘"]),
        );
    }

    #[test]
    fn horizontal_gap_between_items() {
        // given: 3 items of 3 columns each and a gap of 1 column
        let area = Rect::new(0, 0, 11, 3);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 3)
            .scroll_axis(ScrollAxis::Horizontal)
            .gap_size(1);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec!["┌─┐ ┌─┐ ┌─┐", "│ │ │ │ │ │", "└─┘ └─┘ └─┘"]),
        );
    }

    #[test]
    fn backward_direction_aligns_items_with_gaps_to_the_end() {
        // given: 2 items of 3 rows and a gap of 1 row on a viewport of 9 rows
        let area = Rect::new(0, 0, 5, 9);
        let mut buf = Buffer::empty(area);
        let list = ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 2)
            .gap_size(1)
            .scroll_direction(ScrollDirection::Backward);
        let mut state = ListState::default();

        // when
        list.render(area, &mut buf, &mut state);

        // then: the items and the gap are aligned to the bottom
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "     ",
                "     ",
                "┌───┐",
                "│   │",
                "└───┘",
                "     ",
                "┌───┐",
                "│   │",
                "└───┘",
            ]),
        );
    }

    #[test]
    fn backward_direction_anchors_only_when_the_selection_leaves_the_viewport() {
        // given: viewport of 9 rows, 5 items of 2 rows and a gap of 1 row. The
        // last row stays unused because the next gap does not fit.
        let area = Rect::new(0, 0, 5, 9);
        let make_list = || {
            ListView::new(ListBuilder::new(|_| (TestItem {}, 2)), 5)
                .gap_size(1)
                .scroll_direction(ScrollDirection::Backward)
        };
        let bottom_aligned = Buffer::with_lines(vec![
            "     ",
            "┌───┐",
            "└───┘",
            "     ",
            "┌───┐",
            "└───┘",
            "     ",
            "┌───┐",
            "└───┘",
        ]);
        let top_aligned = Buffer::with_lines(vec![
            "┌───┐",
            "└───┘",
            "     ",
            "┌───┐",
            "└───┘",
            "     ",
            "┌───┐",
            "└───┘",
            "     ",
        ]);

        let mut state = ListState::default();
        state.select(Some(2));

        // the list is anchored to the bottom by default
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, bottom_aligned.clone());
        assert_eq!(state.main_axis_offset(), 1);

        // scrolling up to a fully visible item keeps the anchor
        state.previous();
        assert_eq!(state.selected, Some(1));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, bottom_aligned.clone());

        // scrolling up to the first item anchors to the top
        state.previous();
        assert_eq!(state.selected, Some(0));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, top_aligned.clone());
        assert_eq!(state.main_axis_offset(), 0);

        // scrolling down to a fully visible item keeps the anchor
        state.next();
        assert_eq!(state.selected, Some(1));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, top_aligned.clone());

        // the bottom item still fits into the viewport, so the anchor is kept
        state.next();
        assert_eq!(state.selected, Some(2));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, top_aligned);

        // only the next selection leaves the viewport and anchors to the bottom
        state.next();
        assert_eq!(state.selected, Some(3));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, bottom_aligned);
        assert_eq!(state.main_axis_offset(), 1);
    }

    #[test]
    fn backward_direction_does_not_jump_when_the_whole_list_fits() {
        // given: viewport of 10 rows, 2 items of 2 rows and a gap of 1 row.
        // All items fit into the viewport, so the content stays anchored to the
        // bottom and does not jump to the top when items are selected.
        let area = Rect::new(0, 0, 5, 10);
        let mut state = ListState::default();
        state.select(Some(0));
        let make_list = || {
            ListView::new(ListBuilder::new(|_| (TestItem {}, 2)), 2)
                .gap_size(1)
                .scroll_direction(ScrollDirection::Backward)
        };
        let bottom_aligned = Buffer::with_lines(vec![
            "     ",
            "     ",
            "     ",
            "     ",
            "     ",
            "┌───┐",
            "└───┘",
            "     ",
            "┌───┐",
            "└───┘",
        ]);

        // when rendering with the first item selected
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, bottom_aligned.clone());

        // when scrolling down to the last item
        state.next();
        assert_eq!(state.selected, Some(1));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, bottom_aligned.clone());

        // when scrolling up to the first item again
        state.previous();
        assert_eq!(state.selected, Some(0));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_buffer_eq(buf, bottom_aligned);
    }

    #[test]
    fn backward_direction_sticks_to_top_when_first_item_is_selected() {
        // given: the last item is selected, so the content is anchored to the
        // bottom
        let area = Rect::new(0, 0, 5, 10);
        let mut state = ListState::default();
        state.select(Some(4));
        let make_list = || {
            ListView::new(ListBuilder::new(|_| (TestItem {}, 3)), 5)
                .gap_size(2)
                .scroll_direction(ScrollDirection::Backward)
        };
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);

        // when scrolling up to the first item
        state.select(Some(0));
        let mut buf = Buffer::empty(area);
        make_list().render(area, &mut buf, &mut state);
        assert_eq!(state.scroll_offset_index(), 0);

        // then: the content sticks to the top, the unused space stays below
        assert_buffer_eq(
            buf,
            Buffer::with_lines(vec![
                "┌───┐",
                "│   │",
                "└───┘",
                "     ",
                "     ",
                "┌───┐",
                "│   │",
                "└───┘",
                "     ",
                "     ",
            ]),
        );
    }

    fn assert_buffer_eq(actual: Buffer, expected: Buffer) {
        if actual.area != expected.area {
            panic!(
                "buffer areas not equal expected: {:?} actual: {:?}",
                expected, actual
            );
        }
        let diff = expected.diff(&actual);
        if !diff.is_empty() {
            panic!(
                "buffer contents not equal\nexpected: {:?}\nactual: {:?}",
                expected, actual,
            );
        }
        assert_eq!(actual, expected, "buffers not equal");
    }
}
