//! What a page or a sheet offers only in Demo (`docs/DEMO.md` §14): a
//! sample, the demo's passphrase, a line saying what Demo puts in place
//! of the person's own. The page hands each item to the frame
//! ([`Ui::demo_offer`], [`Ui::demo_line`]) where it would have drawn it;
//! Faraday Desktop's host takes them into its strip, outside the app
//! (`docs/DESKTOP-HOST.md` §3.1), so nothing demo-only is drawn in a
//! page.

use crate::Ui;

/// What a page hands over for the demo.
#[derive(Debug, Clone, PartialEq)]
pub enum DemoItem<A> {
    /// A button: its label and its press.
    Button(&'static str, A),
    /// A line saying what Demo does in place of the person's own.
    Line(String),
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Hands the frame a demo button, once however often the page
    /// offers it.
    pub fn demo_offer(&mut self, label: &'static str, action: A) {
        let known = self
            .demo
            .iter()
            .any(|i| matches!(i, DemoItem::Button(_, a) if *a == action));
        if !known {
            self.demo.push(DemoItem::Button(label, action));
        }
    }

    /// Hands the frame a demo line.
    pub fn demo_line(&mut self, text: impl Into<String>) {
        let text = text.into();
        if !self.demo.iter().any(|i| *i == DemoItem::Line(text.clone())) {
            self.demo.push(DemoItem::Line(text));
        }
    }

    /// What the page handed over for the demo, taken.
    pub fn take_demo(&mut self) -> Vec<DemoItem<A>> {
        std::mem::take(&mut self.demo)
    }
}
