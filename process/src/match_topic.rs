//! Helper function to pattern match topics

/// Match a dotted topic against a pattern - implements as RabbitMQ:
///   * - match one word
///   # - match zero or more words
pub fn match_topic(pattern: &str, topic: &str) -> bool {

    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let topic_parts: Vec<&str> = topic.split('.').collect();

    let mut i = 0;
    let mut j = 0;

    while i < pattern_parts.len() && j < topic_parts.len() {

        match pattern_parts[i] {
            // * matches exactly one word
            "*" => {
                i += 1;
                j += 1;
            }

            // # matches zero or more words
            "#" => {
                if i == pattern_parts.len() - 1 {
                    // All the rest
                    return true;
                }

                // Try to match the next part of the pattern to any
                // subsequent part of the topic
                while j < topic_parts.len() {
                    if match_topic(&pattern_parts[i + 1..].join("."),
                                   &topic_parts[j..].join(".")) {
                        return true;
                    }
                    j += 1;
                }

                // No match found for the rest of the topic after #
                return false;
            }

            // Direct match for a part?
            part if part == topic_parts[j] => {
                i += 1;
                j += 1;
            }

            // No match
            _ => return false
        }
    }

    // If we reached the end of both the pattern and topic, it's a match
    i == pattern_parts.len() && j == topic_parts.len()
}
