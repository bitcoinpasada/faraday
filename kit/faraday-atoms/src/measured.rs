//! Drawing to measure: what a body draws into a zero clip, taken back (09 §3.4).

use faraday_draw::Rect;

use crate::Pressed;
use crate::ui::Ui;

impl<A: Pressed> Ui<'_, A> {
    /// Runs `draw` into a zero clip, for what it returns (a height, its buttons), then takes back
    /// everything it recorded: the presses and the Tab order, the typed lines, the regions, the
    /// anchors and layout answers, the tooltip anchors, the cut presses, the focus box a field
    /// typing goes to, the caret and the back link. Nothing it drew shows.
    pub fn measured<T>(&mut self, draw: impl FnOnce(&mut Self) -> T) -> T {
        let marks = self.presses_mark();
        let lens = (
            self.fields.len(),
            self.regions.len(),
            self.anchors.len(),
            self.laid.len(),
            self.tips.len(),
            self.cut.len(),
        );
        let kept = (
            self.focus_box,
            self.focus_action.clone(),
            self.primaries,
            self.caret_drawn,
            self.caret_foot,
            self.fab_at,
            self.back.clone(),
        );
        self.c.push_clip(Rect::new(0, 0, 0, 0));
        let out = draw(self);
        self.c.pop_clip();
        self.presses_rewind(marks);
        self.fields.truncate(lens.0);
        self.regions.truncate(lens.1);
        self.anchors.truncate(lens.2);
        self.laid.truncate(lens.3);
        self.tips.truncate(lens.4);
        self.cut.truncate(lens.5);
        (
            self.focus_box,
            self.focus_action,
            self.primaries,
            self.caret_drawn,
            self.caret_foot,
            self.fab_at,
            self.back,
        ) = kept;
        out
    }
}
