/// Returns a value between `left` and `right` at point `mid`
/// The `mid` point maps values between 0 and 1 such that 0 is `left`
/// and 1 is `right`. Values outside the 0 to 1 range are also allowed
///
/// # Example
/// ```rust
/// use mellow::util::lerp;
///
/// assert_eq!(lerp(5.0, 10.0, 0.0), 5.0);
/// assert_eq!(lerp(5.0, 10.0, 1.0), 10.0);
/// assert_eq!(lerp(5.0, 10.0, 0.5), 7.5);
/// assert_eq!(lerp(5.0, 10.0, 2.0), 15.0);
/// assert_eq!(lerp(5.0, 10.0, -1.0), 0.0);
/// ```
#[must_use]
pub fn lerp(left: f64, right: f64, mid: f64) -> f64 {
    (right - left).mul_add(mid, left)
}

/// Checks if two float numbers are similar
///
/// # Example
/// ```rust
/// use mellow::util::approx_eq;
///
/// assert!(approx_eq(0.9995, 1.0));
/// assert!(approx_eq(1.0005, 1.0));
/// assert!(!approx_eq(0.9994, 1.0));
/// ```
#[inline]
#[must_use]
pub fn approx_eq(left: f64, right: f64) -> bool {
    const TOLERANCE: f64 = 0.0005;
    (left - right).abs() < TOLERANCE
}
