#![macro_use]

/// Serializes the given value/field pairs into a `String`,
/// which can be used with `deserialize!()` to retreive the
/// values afterwards
///
/// Note: When serializing `ClockTime`, use `[…].nseconds()`
/// on the left side of the expression to convert it to a
/// format compatible with `deserialize!()`
///
/// # Example:
/// ```rust
/// use mellow::serialize;
/// use gst::ClockTime;
///
/// let number = 5;
/// let text = "hello";
/// let time = ClockTime::from_nseconds(50000);
///
/// let serialized = serialize!(
///     number => "number",
///     text => "text",
///     time.nseconds() => "time",
/// );
///
/// assert_eq!(
///     serialized,
///     "\
/// number: 5
/// text: hello
/// time: 50000
/// "
/// );
/// ```
#[macro_export]
macro_rules! serialize {
    ($($value:expr => $field:tt,)+) => {
        [$($field.to_owned() + ": " + &$value.to_string() + "\n",)+].concat()
    };
}

/// Retreives serialized `data` field values and assigns them
/// to the variables on the right side of each expression
///
/// Note: Assignment may fail silently for individual fields
/// if they are not present within the provided `data`
///
/// # Errors:
/// This function causes the caller to propagate an `Err`
/// value of type `String` in the event of an error
///
/// # Example:
/// ```rust
/// use mellow::deserialize;
/// use gst::ClockTime;
///
/// let mut number = 0;
/// let mut text = String::new();
/// let mut time = ClockTime::default();
///
/// let data = "\
/// number: 5
/// text: hello
/// time: 50000
/// ";
///
/// deserialize!(
///     data,
///     "number"<"parse"> => number,
///     "text"<"String"> => text,
///     "time"<"ClockTime"> => time,
/// );
///
/// assert_eq!(number, 5);
/// assert_eq!(text, "hello".to_string());
/// assert_eq!(time, ClockTime::from_nseconds(50000));
///
/// Ok::<(), String>(())
/// ```
#[macro_export]
macro_rules! deserialize {
    ($data:tt, $($field:tt<$type:tt> => $target:expr,)+) => {
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
    (@to_value "ClockTime", $value:expr, $field:expr) => {
        ClockTime::from_nseconds(
            $value.parse().map_err(|e| format!("{} {e}", $field))?
        )
    };
}
