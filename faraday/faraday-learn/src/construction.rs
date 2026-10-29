//! A construction (`docs/CRYPTO-LEARN.md` §18): how a format several
//! actions share is built, told as more sections of a Learn page after
//! its own. The type `how.rs`'s `CONSTRUCTIONS` is generated into; the
//! text is the manifest's, its numbers and tables filled from the code.

/// One construction: its title, the Learn page it is drawn after, the
/// actions it belongs to and its sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Construction {
    /// Its id in the manifest.
    pub id: &'static str,
    /// The heading it is drawn under.
    pub title: &'static str,
    /// The Learn page it is drawn after, as an action's `learn` names one
    /// (`faraday:<id>` for one of [`crate::ids`]).
    pub learn: &'static str,
    /// The actions whose format it tells, by their ids.
    pub actions: &'static [&'static str],
    /// Its sections, in order.
    pub sections: &'static [Part],
}

/// A section of a construction: a heading and what follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    /// The section's heading.
    pub heading: &'static str,
    /// Its paragraphs and tables, in order.
    pub items: &'static [Item],
}

/// A paragraph or a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// A paragraph; one about work not built begins "Planned, not built:".
    Para(&'static str),
    /// A table written from the code.
    Table(Table),
}

/// A table: its column heads and its rows, each as many cells as heads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table {
    /// The column heads.
    pub head: &'static [&'static str],
    /// The rows.
    pub rows: &'static [&'static [&'static str]],
}
