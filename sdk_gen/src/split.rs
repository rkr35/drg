pub struct ReverseSplitIterator<'a> {
    source: &'a [u8],
    delimiter: u8,
}

impl<'a> ReverseSplitIterator<'a> {
    pub fn new(source: &[u8], delimiter: u8) -> ReverseSplitIterator {
        ReverseSplitIterator { source, delimiter }
    }
}

impl<'a> Iterator for ReverseSplitIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        #[allow(clippy::int_plus_one)]
        // Applying this lint yields `i < self.source.len()`, which doesn't elide the panic branch.
        if let Some(split) = self
            .source
            .iter()
            .rposition(|c| *c == self.delimiter)
            .filter(|i| i + 1 <= self.source.len())
        {
            // Return everything after the delimiter.
            let ret = &self.source[split + 1..];

            // Shrink the input up to and excluding the delimiter.
            self.source = &self.source[..split];

            Some(ret)
        } else if self.source.is_empty() {
            // We've exhausted the input, and there's nothing else to return.
            None
        } else {
            // Return the remaining piece.
            let ret = self.source;

            // Signal that we exhausted the input.
            self.source = &[];

            Some(ret)
        }
    }
}
