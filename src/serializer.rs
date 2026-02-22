#![macro_use]

/// Serializes the given value/field pairs into a `String`,
/// which can be used with `deserialize!()` to retreive the
/// values afterwards
///
/// Note: When serializing `ClockTime`, use `[…].nseconds()`
/// on the left side of the expression to convert it to a
/// format compatible with `deserialize!()`
///
/// # Example
/// ```rust
/// use mellow::{serialize, serializer::serialize_list};
/// use gst::ClockTime;
///
/// let number = 5;
/// let text = "hello";
/// let time = ClockTime::from_nseconds(50000);
/// let list = &[
///     "one".to_string(),
///     "two".to_string(),
///     "three, four".to_string(),
/// ];
/// let numbers = &[1, 2, 3, 4];
///
/// assert_eq!(
///     serialize! {
///         number => "number",
///         text => "text",
///         time.nseconds() => "time",
///         serialize_list(list) => "list",
///         serialize_list(&numbers.map(|n| n.to_string())) => "numbers",
///     },
///     "\
/// number: 5
/// text: hello
/// time: 50000
/// list: one, two, three\\, four, \n\
/// numbers: 1, 2, 3, 4, \n\
/// "
/// );
/// ```
#[macro_export]
macro_rules! serialize {
    {$($value:expr => $field:tt,)+} => {
        [$($field, ": ", &$value.to_string(), "\n",)+].concat()
    };
}

/// Combines a list of `String`s into a single `String` which
/// can be used with the `serialize!()` macro
///
/// # Example
/// ```rust
/// use mellow::serializer::serialize_list;
///
/// assert_eq!(
///     serialize_list(&[
///         "one".to_string(),
///         "two".to_string(),
///         "three, four".to_string(),
///     ]),
///     "one, two, three\\, four, "
/// );
/// ```
#[inline]
#[must_use]
pub fn serialize_list(list: &[String]) -> String {
    list.iter().map(|s| s.replace(',', "\\,") + ", ").collect()
}

/// Retreives serialized `data` field values and assigns them
/// to the variables on the right side of each expression
///
/// Note: Assignment may fail silently for individual fields
/// if they are not present within the provided `data`
///
/// The following types are supported:
/// - `str` for assigning string slices
/// - `String` for assigning owned strings
/// - `?` for types implementing the `FromStr` trait
/// - `ClockTime` for assigning `gst::ClockTime`
/// - All types (except `str`) can be wrapped in square brackets
///   (`[…]`) to parse them as lists (such as `Vec`s)
///
/// # Panics
/// This macro panics when parsing invalid data for types `?`/`[?]`
/// or `ClockTime`/`[ClockTime]`
///
/// # Example
/// ```rust
/// use mellow::{deserialize, unescaped_split};
/// use gst::ClockTime;
///
/// let mut number = 0;
/// let mut text = String::new();
/// let mut text_str = "";
/// let mut time = ClockTime::default();
/// let mut numbers: Vec<usize> = Vec::new();
/// let mut list = Vec::new();
/// let mut times: Vec<ClockTime> = Vec::new();
///
/// let data = "\
/// number: 5
/// text: hello
/// text_str: hi
/// time: 50000
/// numbers: 1, 2, 3, 4
/// list: one, two, three\\, four,
/// times: 12, 34
/// ";
///
/// deserialize! {
///     data => {
///         "number"<?> => number,
///         "text"<String> => text,
///         "text_str"<str> => text_str,
///         "time"<ClockTime> => time,
///         "numbers"<[?]> => numbers,
///         "list"<[String]> => list,
///         "times"<[ClockTime]> => times,
///     }
/// }
///
/// assert_eq!(number, 5);
/// assert_eq!(text, "hello".to_string());
/// assert_eq!(text_str, "hi");
/// assert_eq!(time, ClockTime::from_nseconds(50000));
/// assert_eq!(
///     list,
///     ["one".to_owned(), "two".to_owned(), "three, four".to_owned()],
/// );
/// assert_eq!(
///     times,
///     [ClockTime::from_nseconds(12), ClockTime::from_nseconds(34)],
/// );
/// assert_eq!(numbers, [1, 2, 3, 4]);
/// ```
#[macro_export]
macro_rules! deserialize {
    {$data:tt => {$($field:tt<$type:tt> => $target:expr,)+}} => {
        #[cfg(debug_assertions)]
        if $data.is_empty() {
            panic!("No data provided");
        }

        for line in $data.lines() {
            let Some((field, value)) = line.split_once(": ") else {
                continue;
            };

            match field {
                $($field => {
                    $target = deserialize!(@to_value $type, value, field);
                },)+
                _ => eprintln!("Unknown field: `{field}`"),
            }
        }
    };

    (@to_value ?, $value:expr, $field:expr) => {
        $value.parse().map_err(|e| format!("{} {e}", $field)).unwrap()
    };
    (@to_value [?], $value:expr, $field:expr) => {
        $value.split(',').into_iter().map(|value| value.trim().parse().unwrap()).collect()
    };
    (@to_value str, $value:expr, $field:expr) => {
        $value
    };
    (@to_value String, $value:expr, $field:expr) => {
        $value.to_owned()
    };
    (@to_value [String], $value:expr, $field:expr) => {
        unescaped_split($value, ',')
    };
    (@to_value ClockTime, $value:expr, $field:expr) => {
        ClockTime::from_nseconds($value.parse().unwrap_or_else(|e| {
            panic!("deserialize!: {e} (value was: '{}', field name: {})", $value, $field)
        }))
    };
    (@to_value [ClockTime], $value:expr, $field:expr) => {
        $value.split(',').into_iter().map(|value| {
            ClockTime::from_nseconds(value.trim().parse().unwrap_or_else(|e| {
                panic!("deserialize!: {e} (value was: '{}', field name: {})", value, $field)
            }))
        }).collect()
    };
}
