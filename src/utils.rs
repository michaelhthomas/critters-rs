use lightningcss::rules::style::StyleRule;

/// Cantor's pairing function. Generates a unique integer from a pair of two integers.
pub fn cantor(a: u32, b: u32) -> u32 {
    (a + b + 1) * (a + b) / 2 + b
}

pub trait StyleRuleExt {
    /// Generates a unique identifier that can be used to identify the rule in later passes of the AST.
    fn id(&self) -> u32;
}
impl StyleRuleExt for StyleRule<'_> {
    fn id(&self) -> u32 {
        cantor(
            self.loc.source_index,
            cantor(self.loc.line, self.loc.column),
        )
    }
}
