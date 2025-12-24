/// Fuzzy query result scoring function, which returns a score number
/// between `0` and `1`, where `1` is a complete match, `0` a non-match,
/// and partial matches anywhere in-between. Note that this function has
/// no preference between query/result length; the query "test" will
/// match results "test" and "testing" with the same score of `1.0`.
///
/// # Example:
/// ```rust
/// use mellow::library::search::query_score;
///
/// assert_eq!(query_score("test", "test"), 1.0);
/// assert_eq!(query_score("test", "TEST"), 1.0);
/// assert_eq!(query_score("test", "testing"), 1.0);
/// assert_eq!(query_score("testing", "test"), 0.5714285714285714);
/// assert_eq!(query_score("testang", "testing"), 0.8571428571428571);
/// assert_eq!(query_score("itesting", "testing"), 0.7777777777777778);
/// assert_eq!(query_score("ttesting", "testing"), 0.8888888888888888);
/// assert_eq!(query_score("testingg", "testing"), 0.875);
/// assert_eq!(query_score("fever", "forever"), 0.2857142857142857);
/// assert_eq!(query_score("apple", "pineapple"), 0.25);
/// assert_eq!(query_score("banana", "pineapple"), 0.0);
/// ```
#[must_use]
pub fn query_score(query: &str, item: &str) -> f64 {
    // dbg!(&query);
    // dbg!(&item);
    let query: Vec<u8> = query.to_lowercase().bytes().collect();
    let item: Vec<u8> = item.to_lowercase().bytes().collect();
    let (mut start, mut end) = (0, 0);
    let mut offset = 0;
    let mut match_len = 0.0;
    for q in 0..query.len() {
        if q - offset >= item.len() {
            break;
        }
        if query[q] == item[q - offset] {
            end += 1;
        } else {
            if start == end {
                start = q;
                end = q;
                offset += 1;
                continue;
            }
            if q > 0 && query[q] == query[q - 1] && query[q] == item[q - offset - 1] {
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
    match_len / (query.len() + offset) as f64
}
