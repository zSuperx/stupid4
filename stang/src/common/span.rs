use crate::translation_unit::source;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}

#[allow(unused)]
impl<T> Spanned<T> {
    /// Creates a new spanned object
    pub fn new(inner: T, span: Span) -> Self {
        Self { inner, span }
    }

    /// Maps the inner `T` to a `U` using function `F(&T) -> U` and
    /// returns a new `Spanned<U>` that inherits the `span` from `self`.
    pub fn map<U, F>(&self, mut f: F) -> Spanned<U>
    where
        F: FnMut(&T) -> U,
    {
        Spanned::new(f(&self.inner), self.span)
    }

    pub fn source_string(&self) -> &'static str {
        str::from_utf8(&source()[self.span.lo..self.span.hi]).unwrap()
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file: &'static str,
    pub lo: usize,
    pub hi: usize,
    pub row: usize,
    pub col: usize,
}

#[allow(unused)]
impl Span {
    pub fn new(file: &'static str, lo: usize, hi: usize, row: usize, col: usize) -> Self {
        Self {
            file,
            lo,
            hi,
            row,
            col,
        }
    }

    pub fn merge(self, other: Self) -> Self {
        assert_eq!(self.file, other.file);
        let smaller = if self.lo <= other.lo { &self } else { &other };
        Self {
            file: self.file,
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            row: smaller.row,
            col: smaller.col,
        }
    }

    pub fn wrap<T>(self, inner: T) -> Spanned<T> {
        Spanned::new(inner, self)
    }

    // TODO: This is partially AI slop! Rewrite it, it can be WAY better integrated with the
    // compiler API
    pub fn content(&self) -> String {
        let source = source();
        let line_start = source[..self.lo]
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let line_end = source[self.lo..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|pos| self.lo + pos)
            .unwrap_or(source.len());

        let line_text =
            std::str::from_utf8(&source[line_start..line_end]).unwrap_or("<invalid utf8>");

        let before_span_count = self.lo.saturating_sub(line_start);

        let is_multiline = self.hi > line_end;
        let effective_hi = if is_multiline { line_end } else { self.hi };

        let caret_count = effective_hi.saturating_sub(self.lo).max(1);

        let spaces = " ".repeat(before_span_count);
        let mut carets = "^".repeat(caret_count);

        if is_multiline {
            carets.push_str("...");
        }

        format!(
            "\n\n{}:{}:{}:\n{}\n{}{}\n",
            self.file,
            self.row + 1,
            self.col,
            line_text,
            spaces,
            carets
        )
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.content())
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = self.source_string();
        f.write_fmt(format_args!("`{raw}`: {}", self.span))
    }
}
