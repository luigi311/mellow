#![macro_use]

/// Serializes specified values into a `String` which
/// can be used with `deserialize!()` to retreive the
/// values
///
/// # Example:
/// ```rust
/// use mellow::serialize;
///
/// let number = 5;
/// let text = "hello";
///
/// let serialized = serialize!(
///     number => "number",
///     text => "text",
/// );
///
/// assert_eq!(
///     serialized,
///     "\
/// number: 5
/// text: hello
/// "
/// );
/// ```
#[macro_export]
macro_rules! serialize {
    ($($value:expr => $field: tt,)+) => {
        [$(($field.to_owned() + ": " + &$value.to_string()) + "\n",)+].concat()
    };
}

/// Takes serialized `data` and deserializes it into the
/// into specified fields
///
/// # Errors:
/// This function causes the caller to propagate an `Err`
/// value of type `String` in the event of an error
///
/// # Example:
/// ```rust
/// use mellow::deserialize;
///
/// let mut number = 0u32;
/// let mut text = String::new();
///
/// deserialize!(
///     "\
/// number: 5
/// text: hello
/// ",
///     "number"<"u32"> => number,
///     "text"<"String"> => text,
/// );
///
/// assert_eq!(number, 5);
/// assert_eq!(text, "hello".to_string());
///
/// Ok::<(), String>(())
/// ```
#[macro_export]
macro_rules! deserialize {
    ($data: tt, $($field: tt<$type:tt> => $target:expr,)+) => {
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

    (@to_value "u32", $value:expr, $field:expr) => {
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
