// Fuzzy search idea:
//
// Match the search query using multiple substrings,
// and score them based on certain criteria
//
// For every search candidate:
// - Loop through each character of the query until a non-match is found
// - In the case of a non-matching character, store the matching query substring
// to an array of (start, end), tuples and start over using the next character of
// the query after the last matching character of the result candidate
// - Any character which does not match does not need to be stored
// - Repeat until the query is depleted
// - Next, loop through the array and compare all non-overlapping/sequential
// parts' length sum with the length of the query itself to calculate a score
// - Build a list of results, ordered by descending scores (exclude results whose
// scores are below a certain threshold)
