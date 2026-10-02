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
