//! Portable transcendental math: std float methods can differ in their last
//! bits between native code and WASM, changing seeded histories. Keep those
//! operations in libm; basic arithmetic and square roots stay native.

#[inline]
pub(crate) fn exp(value: f32) -> f32 {
    libm::expf(value)
}

#[inline]
pub(crate) fn ln(value: f32) -> f32 {
    libm::logf(value)
}

#[inline]
pub(crate) fn pow(base: f32, exponent: f32) -> f32 {
    libm::powf(base, exponent)
}

#[inline]
pub(crate) fn sin(value: f64) -> f64 {
    libm::sin(value)
}

#[inline]
pub(crate) fn cos(value: f64) -> f64 {
    libm::cos(value)
}

#[inline]
pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

#[inline]
pub(crate) fn exp64(value: f64) -> f64 {
    libm::exp(value)
}
