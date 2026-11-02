//! The toolbar's controls, left to right (design, "The toolbar";
//! `item_view/02-editor-and-toolbar.md`): what each is called, its icon or
//! text, its key, and whether Markdown mode dims it.

/// One control of the toolbar. The table controls come last and are drawn
/// only while the caret is in a pipe table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Control {
    Rich,
    Markdown,
    Undo,
    Redo,
    Style,
    Bold,
    Italic,
    Strike,
    Code,
    Link,
    Bullets,
    Numbered,
    Checklist,
    Quote,
    CodeBlock,
    Table,
    RowBelow,
    ColumnRight,
    DeleteRow,
    DeleteColumn,
    AlignLeft,
    AlignCentre,
    AlignRight,
}

/// How many controls there are: one focus handle each.
pub(crate) const COUNT: usize = 23;

/// The controls always drawn, in order.
pub(crate) const MAIN: [Control; 16] = [
    Control::Rich,
    Control::Markdown,
    Control::Undo,
    Control::Redo,
    Control::Style,
    Control::Bold,
    Control::Italic,
    Control::Strike,
    Control::Code,
    Control::Link,
    Control::Bullets,
    Control::Numbered,
    Control::Checklist,
    Control::Quote,
    Control::CodeBlock,
    Control::Table,
];

/// The table controls, drawn after the others inside a table.
pub(crate) const TABLE: [Control; 7] = [
    Control::RowBelow,
    Control::ColumnRight,
    Control::DeleteRow,
    Control::DeleteColumn,
    Control::AlignLeft,
    Control::AlignCentre,
    Control::AlignRight,
];

impl Control {
    /// Its element ID.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Rich => "toolbar-rich",
            Self::Markdown => "toolbar-markdown",
            Self::Undo => "toolbar-undo",
            Self::Redo => "toolbar-redo",
            Self::Style => "toolbar-style",
            Self::Bold => "toolbar-bold",
            Self::Italic => "toolbar-italic",
            Self::Strike => "toolbar-strike",
            Self::Code => "toolbar-code",
            Self::Link => "toolbar-link",
            Self::Bullets => "toolbar-bullets",
            Self::Numbered => "toolbar-numbered",
            Self::Checklist => "toolbar-checklist",
            Self::Quote => "toolbar-quote",
            Self::CodeBlock => "toolbar-code-block",
            Self::Table => "toolbar-table",
            Self::RowBelow => "toolbar-row-below",
            Self::ColumnRight => "toolbar-column-right",
            Self::DeleteRow => "toolbar-delete-row",
            Self::DeleteColumn => "toolbar-delete-column",
            Self::AlignLeft => "toolbar-align-left",
            Self::AlignCentre => "toolbar-align-centre",
            Self::AlignRight => "toolbar-align-right",
        }
    }

    /// Its accessible name, and its tooltip's words before the key.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Rich => "Rich text",
            Self::Markdown => "Markdown",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Style => "Paragraph style",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Strike => "Strikethrough",
            Self::Code => "Inline code",
            Self::Link => "Link",
            Self::Bullets => "Bulleted list",
            Self::Numbered => "Numbered list",
            Self::Checklist => "Checklist",
            Self::Quote => "Quote",
            Self::CodeBlock => "Code block",
            Self::Table => "Table",
            Self::RowBelow => "Insert row below",
            Self::ColumnRight => "Insert column right",
            Self::DeleteRow => "Delete row",
            Self::DeleteColumn => "Delete column",
            Self::AlignLeft => "Align left",
            Self::AlignCentre => "Align centre",
            Self::AlignRight => "Align right",
        }
    }

    /// Its Lucide icon, for an icon button.
    pub(crate) fn icon(self) -> Option<&'static str> {
        Some(match self {
            Self::Undo => "undo-2",
            Self::Redo => "redo-2",
            Self::Style => "pilcrow",
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::Strike => "strikethrough",
            Self::Code => "code",
            Self::Link => "link",
            Self::Bullets => "list",
            Self::Numbered => "list-ordered",
            Self::Checklist => "list-todo",
            Self::Quote => "text-quote",
            Self::CodeBlock => "square-code",
            Self::Table => "table",
            _ => return None,
        })
    }

    /// Its visible text, for a control drawn as words: the mode segments
    /// and the table controls, which have no icon of their own.
    pub(crate) fn text(self) -> Option<&'static str> {
        Some(match self {
            Self::Rich => "Rich text",
            Self::Markdown => "Markdown",
            Self::RowBelow => "+ Row",
            Self::ColumnRight => "+ Column",
            Self::DeleteRow => "− Row",
            Self::DeleteColumn => "− Column",
            Self::AlignLeft => "Left",
            Self::AlignCentre => "Centre",
            Self::AlignRight => "Right",
            _ => return None,
        })
    }

    /// The key that does the same, as GPUI parses it: the editor's own
    /// (fact 72), the link key, or the mode key.
    pub(crate) fn key(self) -> Option<&'static str> {
        Some(match self {
            Self::Rich | Self::Markdown => "secondary-shift-m",
            Self::Undo => "secondary-z",
            Self::Redo => "secondary-shift-z",
            Self::Bold => "secondary-b",
            Self::Italic => "secondary-i",
            Self::Strike => "secondary-shift-x",
            Self::Code => "secondary-e",
            Self::Link => "secondary-k",
            _ => return None,
        })
    }

    /// Whether Markdown mode dims it: every control that formats. The mode
    /// switch, Undo and Redo keep full strength.
    pub(crate) fn formatting(self) -> bool {
        !matches!(self, Self::Rich | Self::Markdown | Self::Undo | Self::Redo)
    }

    /// Whether a separator comes before it: between the groups History,
    /// Block, Inline, Lists and Blocks, the table's rows and columns, and
    /// its alignment.
    pub(crate) fn starts_group(self) -> bool {
        matches!(
            self,
            Self::Style | Self::Bold | Self::Bullets | Self::RowBelow | Self::AlignLeft
        )
    }

    /// Its place among the focus handles.
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}
