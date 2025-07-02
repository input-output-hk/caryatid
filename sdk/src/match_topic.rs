//! Helper function to pattern match topics
use std::iter::Peekable;
use std::str::Split;

/// Match a dotted topic against a pattern - implements as RabbitMQ:
///   * - match one word
///   # - match zero or more words
pub fn match_topic(pattern: &str, topic: &str) -> bool {
    let pattern_split = pattern.split('.').peekable();
    let topic_split = topic.split('.').peekable();

    match_topic_split(pattern_split, topic_split)
}

fn match_topic_split(mut pattern: Peekable<Split<char>>, mut topic: Peekable<Split<char>>) -> bool {
    while let Some(pattern_part) = pattern.next() {
        match pattern_part {
            // * matches exactly one word
            "*" => {
                topic.next();
            }

            // # matches zero or more words
            "#" => {
                // Eat any extra #
                while pattern.peek() == Some("#").as_ref() {
                    pattern.next();
                }
                if pattern.peek() == None {
                    // All the rest
                    return true;
                }

                // Try to match the next part of the pattern to any
                // subsequent part of the topic
                while let Some(_) = topic.peek() {
                    if match_topic_split(pattern.clone(), topic.clone()) {
                        return true;
                    }
                    topic.next();
                }

                // No match found for the rest of the topic after #
                return false;
            }

            // Direct match for a part?
            pattern_part if Some(pattern_part) == topic.peek().map(|v| &**v) => {
                topic.next();
            }

            // No match
            _ => return false,
        }
    }

    // If we reached the end of both the pattern and topic, it's a match
    topic.peek() == None
}

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_matches() {
        assert!(match_topic("foo", "foo"));
        assert!(!match_topic("foo", "bar"));
        assert!(match_topic("foo.bar", "foo.bar"));
    }

    #[test]
    fn star_matches_one_word() {
        assert!(match_topic("*", "foo"));
        assert!(!match_topic("*", "foo.bar"));
        assert!(match_topic("foo.*", "foo.bar"));
        assert!(match_topic("foo.*.baz", "foo.bar.baz"));
    }

    #[test]
    fn hash_matches_multiple_words() {
        assert!(match_topic("#", "foo"));
        assert!(match_topic("#", "foo.bar.baz"));
        assert!(match_topic("foo.#", "foo.bar.baz"));
        assert!(match_topic("foo.#.#", "foo.bar.baz"));
        assert!(match_topic("foo.#", "foo"));
        assert!(match_topic("foo.#.#", "foo"));
        assert!(match_topic("foo.bar.#", "foo.bar.baz"));
        assert!(match_topic("foo.#.baz", "foo.bar.baz"));
        assert!(match_topic("foo.#.baz", "foo.baz"));
        assert!(match_topic("foo.#.#.baz", "foo.baz"));
        assert!(match_topic("foo.#.baz", "foo.bar.something.more.baz"));
        assert!(match_topic("#.baz", "baz"));
        assert!(match_topic("#.#.baz", "baz"));
        assert!(match_topic("#.baz", "foo.bar.baz"));
        assert!(!match_topic("#.splat", "foo.bar.baz"));
        assert!(!match_topic("foo.bar.baz#", "foo.bar.baz"));
    }

    #[test]
    fn mixed_patterns() {
        assert!(match_topic("foo.*", "foo.bar"));
        assert!(match_topic("foo.#", "foo.bar.baz"));
        assert!(!match_topic("foo.*", "foo.bar.baz"));
        assert!(match_topic("foo.*.#", "foo.bar.baz"));
        assert!(match_topic("foo.*.#.baz", "foo.bar.baz"));
        assert!(match_topic("foo.*.#.*", "foo.bar.baz"));
        assert!(match_topic("foo.#.bar.#.*", "foo.bar.baz"));
        assert!(!match_topic("foo.*.#.*.baz", "foo.bar.baz"));
    }
}
