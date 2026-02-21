pub const SCORE_THRESHOLD: f64 = 0.75;

/// Fuzzy query result scoring function which returns a
/// score number between 0 and 1, depending on how well
/// the `query` matches the `item`
///
/// Note: the comparison is case-sensitive
///
/// # Example:
/// ```rust
/// use mellow::library::search::query_score;
///
/// assert_eq!(query_score("world", "Hello world!"), 0.9944444444444445);
/// assert_eq!(query_score("Hello world!", "world"), 0.2604166666666667);
/// assert_eq!(query_score("test", "test"), 1.0);
/// assert_eq!(query_score("test", "TEST"), 1.0);
/// assert_eq!(query_score("test", "testing"), 0.9846938775510204);
/// assert_eq!(query_score("testing", "test"), 0.5714285714285714);
/// assert_eq!(query_score("testang", "testing"), 0.8571428571428571);
/// assert_eq!(query_score("itesting", "testing"), 0.765625);
/// assert_eq!(query_score("ttesting", "testing"), 0.875);
/// assert_eq!(query_score("testingg", "testing"), 0.875);
/// assert_eq!(query_score("fever", "forever"), 0.27450980392156865);
/// assert_eq!(query_score("apple", "pineapple"), 0.2385185185185185);
/// assert_eq!(query_score("apples", "oranges"), 0.0);
/// assert_eq!(query_score("", "something"), 1.0);
/// assert_eq!(query_score("nothing", ""), 0.0);
/// ```
#[must_use]
pub fn query_score(query: &str, item: &str) -> f64 {
    if query.is_empty() {
        return 1.0;
    }
    if item.is_empty() {
        return 0.0;
    }
    // dbg!(&query);
    // dbg!(&item);
    let query_bytes: Vec<u8> = query.bytes().collect();
    let item_bytes: Vec<u8> = item.bytes().collect();
    let (mut start, mut end) = (0, 0);
    let mut offset = 0;
    let mut match_len = 0.0;
    for q in 0..query_bytes.len() {
        if q - offset >= item_bytes.len() {
            break;
        }
        if query_bytes[q] == item_bytes[q - offset] {
            end += 1;
        } else {
            if start == end {
                start = q;
                end = q;
                offset += 1;
                continue;
            }
            if q - offset > 0
                && query_bytes[q] == query_bytes[q - 1]
                && query_bytes[q] == item_bytes[q - offset - 1]
            {
                offset += 1;
                end += 1;
                continue;
            }
            match_len += (end - start) as f64;
            start = q;
            end = start;
        }
    }
    if start != end {
        match_len += (end - start) as f64;
    }

    let query_len = (query_bytes.len() + offset) as f64;
    let item_len = item_bytes.len() as f64;
    let result = (match_len - ((item_len - query_len).max(0.0) / (item_len * item_len)))
        / (query_len + (offset as f64 / item_len));

    // dbg!(result);
    if item.len() > 2 {
        let Some((_, item)) = item.split_once(' ') else {
            return result;
        };
        return result.max(query_score(query, item));
    }

    result
}

#[inline]
#[must_use]
pub fn query_score_simple(query: &str, item: &str) -> f64 {
    let words = query.split(' ').collect::<Vec<&str>>();
    let mut missed_words = 0.0;
    for word in &words {
        if !item.contains(word) {
            missed_words += 1.0;
        }
    }
    1.0 - missed_words * (1.0 / words.len() as f64)
}

/// Returns a filtered `Vec<T>`, ordered by the scoring
/// criteria returned by the closure. The item with the
/// highest score is at index 0, and lowest is at the end
///
/// # Example:
/// ```rust
/// use mellow::library::search::{query_score, query_items};
/// use std::sync::{Arc, Mutex};
///
/// let items = vec![
///     "Sing the Song",
///     "Hit Single",
///     "Track 3",
///     "Song 4",
///     "Violin Solo",
///     "Song of the Singing Birds",
/// ];
///
/// let results = query_items(&items, "sing", |item, query| {
///     query_score(query, &item)
/// });
/// let mut results = results.iter();
///
/// assert_eq!(results.next(), Some(&"Song of the Singing Birds"));
/// assert_eq!(results.next(), Some(&"Sing the Song"));
/// assert_eq!(results.next(), Some(&"Hit Single"));
/// assert_eq!(results.next(), None);
/// ```
pub fn query_items<T, S>(items: &Vec<T>, query: &str, score: S) -> Vec<T>
where
    S: Fn(&T, &str) -> f64,
    T: Clone,
{
    if query.is_empty() {
        return items.to_owned();
    }
    let mut matches = Vec::<(T, f64)>::new();
    for item in items {
        let score = score(item, query);
        if score < SCORE_THRESHOLD {
            continue;
        }
        let index = matches.binary_search_by(|item| score.total_cmp(&item.1));
        matches.insert(
            match index {
                Err(index) | Ok(index) => index,
            },
            (item.clone(), score),
        );
    }
    matches.into_iter().map(|item| item.0).collect()
}
