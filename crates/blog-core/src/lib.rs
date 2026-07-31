//! Domain model and content pipeline for the Rust rewrite of myBlog.

pub const WORDS_PER_MINUTE: usize = 240;

/// Returns a minimum reading time of one minute for non-empty content.
#[must_use]
pub fn reading_minutes(word_count: usize) -> usize {
    if word_count == 0 {
        0
    } else {
        word_count.div_ceil(WORDS_PER_MINUTE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_time_rounds_up() {
        assert_eq!(reading_minutes(0), 0);
        assert_eq!(reading_minutes(1), 1);
        assert_eq!(reading_minutes(240), 1);
        assert_eq!(reading_minutes(241), 2);
    }
}
