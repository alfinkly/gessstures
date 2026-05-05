use std::ops::{Add, AddAssign, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            *self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    pub fn normalize_or_zero(&self) -> Self {
        let len = self.length();
        if len > 1e-10 {
            *self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self { x: self.x * scalar, y: self.y * scalar, z: self.z * scalar }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        Vec3 { x: self * v.x, y: self * v.y, z: self * v.z }
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, scalar: f32) -> Self {
        Self { x: self.x / scalar, y: self.y / scalar, z: self.z / scalar }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_length_is_zero() {
        assert_eq!(Vec3::ZERO.length(), 0.0);
    }

    #[test]
    fn normalize_unit() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn normalize_or_zero_zero_vector() {
        assert_eq!(Vec3::ZERO.normalize_or_zero(), Vec3::ZERO);
    }

    #[test]
    fn add_commutative() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn scalar_mul() {
        let v = Vec3::new(2.0, 3.0, 4.0);
        assert_eq!(v * 2.0, Vec3::new(4.0, 6.0, 8.0));
        assert_eq!(3.0 * v, Vec3::new(6.0, 9.0, 12.0));
    }
}
