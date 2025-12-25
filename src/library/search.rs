use std::sync::{Arc, Mutex, MutexGuard};

/// Fuzzy query result scoring function, which returns a score number
/// between `0` and `1`, where `1` is a complete match, `0` or below
/// is a non-match, and anything in-between `0` and `1` is a partial
/// match.
///
/// # Example:
/// ```rust
/// use mellow::library::search::query_score;
///
/// assert_eq!(query_score("world", "Hello world!"), 0.9944444444444445);
/// assert_eq!(query_score("Hello world!", "world"), 0.3787878787878788);
/// assert_eq!(query_score("test", "test"), 1.0);
/// assert_eq!(query_score("test", "TEST"), 1.0);
/// assert_eq!(query_score("test", "testing"), 0.9846938775510204);
/// assert_eq!(query_score("testing", "test"), 0.5714285714285714);
/// assert_eq!(query_score("testang", "testing"), 0.8571428571428571);
/// assert_eq!(query_score("itesting", "testing"), 0.8596491228070176);
/// assert_eq!(query_score("ttesting", "testing"), 0.9824561403508772);
/// assert_eq!(query_score("testingg", "testing"), 0.875);
/// assert_eq!(query_score("fever", "forever"), 0.37065637065637064);
/// assert_eq!(query_score("apple", "pineapple"), 0.36574074074074076);
/// assert_eq!(query_score("apples", "oranges"), -0.002976190476190476);
/// ```
#[must_use]
pub fn query_score(query: &str, item: &str) -> f64 {
    // dbg!(&query);
    // dbg!(&item);
    let query_bytes: Vec<u8> = query.to_lowercase().bytes().collect();
    let item_bytes: Vec<u8> = item.to_lowercase().bytes().collect();
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

    let query_len = query_bytes.len() as f64;
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

/// Returns a filtered `Vec<Arc<Mutex<T>>>`, ordered by the
/// scoring criteria returned by the closure. The highest
/// scoring item is at index 0, and lowest is at the end
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
/// ].into_iter().map(|item| Arc::new(Mutex::new(item))).collect();
///
/// let results = query_items(&items, "sing", |item, query| {
///     query_score(query, &item)
/// });
///
/// let mut results = results.iter();
///
/// assert_eq!(
///     results.next().unwrap().lock().unwrap().to_string(),
///     String::from("Song of the Singing Birds")
/// );
/// assert_eq!(
///     results.next().unwrap().lock().unwrap().to_string(),
///     String::from("Sing the Song")
/// );
/// assert_eq!(
///     results.next().unwrap().lock().unwrap().to_string(),
///     String::from("Hit Single")
/// );
/// assert_eq!(
///     results.next().unwrap().lock().unwrap().to_string(),
///     String::from("Song 4"),
/// );
/// assert!(results.next().is_none());
/// ```
pub fn query_items<T, S>(items: &Vec<Arc<Mutex<T>>>, query: &str, score: S) -> Vec<Arc<Mutex<T>>>
where
    S: Fn(MutexGuard<T>, &str) -> f64,
{
    let mut matches = Vec::<(Arc<Mutex<T>>, f64)>::new();
    for item in items {
        let score = score(item.lock().unwrap(), query);
        if score < 0.5 {
            continue;
        }
        let index = matches.binary_search_by(|item| score.total_cmp(&item.1));
        matches.insert(
            match index {
                Err(index) | Ok(index) => index,
            },
            (Arc::clone(item), score),
        );
    }
    matches.drain(..).map(|song| song.0).collect()
}
