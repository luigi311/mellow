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
        [$($field.to_owned() + ": " + &$value.to_string() + "\n",)+].concat()
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
/// # Panics
/// This macro may panic when the `"[parse]"` type is used
/// if the data cannot be parsed into the target type
///
/// # Errors
/// This macro causes the caller to propagate an `Err`
/// value of type `String` if a value cannot be parsed
/// (with the exception of `"[parse]"`; see above)
///
/// # Example
/// ```rust
/// use mellow::deserialize;
/// use mellow::unescaped_split;
/// use gst::ClockTime;
///
/// let mut number = 0;
/// let mut text = String::new();
/// let mut time = ClockTime::default();
/// let mut list = Vec::new();
/// let mut numbers: Vec<usize> = Vec::new();
///
/// let data = "\
/// number: 5
/// text: hello
/// time: 50000
/// list: one, two, three\\, four,
/// numbers: 1, 2, 3, 4
/// ";
///
/// deserialize! {
///     data => {
///         "number"<"parse"> => number,
///         "text"<"String"> => text,
///         "time"<"ClockTime"> => time,
///         "list"<"[String]"> => list,
///         "numbers"<"[parse]"> => numbers,
///     }
/// }
///
/// assert_eq!(number, 5);
/// assert_eq!(text, "hello".to_string());
/// assert_eq!(time, ClockTime::from_nseconds(50000));
/// assert_eq!(
///     list,
///     [
///         "one".to_string(),
///         "two".to_string(),
///         "three, four".to_string(),
///     ],
/// );
/// assert_eq!(numbers, [1, 2, 3, 4]);
///
/// Ok::<(), String>(())
/// ```
#[macro_export]
macro_rules! deserialize {
    {$data:tt => {$($field:tt<$type:tt> => $target:expr,)+}} => {
        #[cfg(debug_assertions)]
        if $data.is_empty() {
            Err("No data provided".to_string())?
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

    (@to_value "parse", $value:expr, $field:expr) => {
        $value.parse().map_err(|e| format!("{} {e}", $field))?
    };
    (@to_value "&str", $value:expr, $field:expr) => {
        $value
    };
    (@to_value "String", $value:expr, $field:expr) => {
        $value.to_owned()
    };
    (@to_value "[parse]", $value:expr, $field:expr) => {
        $value.split(',').into_iter().map(|value| value.trim().parse().unwrap()).collect()
    };
    (@to_value "[String]", $value:expr, $field:expr) => {
        unescaped_split($value, ',')
    };
    (@to_value "ClockTime", $value:expr, $field:expr) => {
        ClockTime::from_nseconds(
            $value.parse().map_err(|e| format!("{} {e}", $field))?
        )
    };
}
