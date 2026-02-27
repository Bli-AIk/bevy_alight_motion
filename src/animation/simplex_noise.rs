//! # simplex_noise.rs
//!
//! 3D Simplex noise implementation matching Alight Motion's `SimplexNoiseKt.java`.
//! Used by the jitter effect for deterministic pseudo-random position displacement.
//!
//! 3D Simplex 噪声实现，匹配 Alight Motion 的 `SimplexNoiseKt.java`。
//! 用于 jitter 效果的确定性伪随机位移。

/// Standard Perlin permutation table (256 entries).
const P: [usize; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

/// 3D gradient vectors (12 directions).
const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// 512-entry permutation table matching AM's `SimplexNoiseKt.java` static init.
/// AM uses `P[i % 255]` (NOT `P[i % 256]`), which shifts entries starting at index 255.
const PERM: [usize; 512] = {
    let mut arr = [0usize; 512];
    let mut i = 0;
    while i < 512 {
        arr[i] = P[i % 255];
        i += 1;
    }
    arr
};

/// Pre-computed `PERM[i] % 12` for gradient index lookup.
const PERM_MOD12: [usize; 512] = {
    let mut arr = [0usize; 512];
    let mut i = 0;
    while i < 512 {
        arr[i] = PERM[i] % 12;
        i += 1;
    }
    arr
};

const F3: f64 = 1.0 / 3.0;
const G3: f64 = 1.0 / 6.0;

#[inline]
fn fastfloor(x: f64) -> i32 {
    let xi = x as i32;
    if x < xi as f64 { xi - 1 } else { xi }
}

#[inline]
fn dot3(g: &[f64; 3], x: f64, y: f64, z: f64) -> f64 {
    g[0] * x + g[1] * y + g[2] * z
}

/// Compute 3D simplex noise, matching AM's `SimplexNoiseKt.simplexNoise(x, y, z)`.
/// Returns value in approximately [-1, 1].
///
/// 计算 3D simplex 噪声，匹配 AM 的实现。
pub fn simplex_noise_3d(xin: f64, yin: f64, zin: f64) -> f64 {
    // Build perm and permMod12 (same as AM's static init)
    // Using inline lookup: perm[i] = P[i & 255], permMod12[i] = perm[i] % 12

    // Skew input space
    let s = (xin + yin + zin) * F3;
    let i = fastfloor(xin + s);
    let j = fastfloor(yin + s);
    let k = fastfloor(zin + s);

    let t = (i + j + k) as f64 * G3;
    let x0 = xin - (i as f64 - t);
    let y0 = yin - (j as f64 - t);
    let z0 = zin - (k as f64 - t);

    // Determine simplex
    let (i1, j1, k1, i2, j2, k2);
    if x0 >= y0 {
        if y0 >= z0 {
            i1 = 1;
            j1 = 0;
            k1 = 0;
            i2 = 1;
            j2 = 1;
            k2 = 0;
        } else if x0 >= z0 {
            i1 = 1;
            j1 = 0;
            k1 = 0;
            i2 = 1;
            j2 = 0;
            k2 = 1;
        } else {
            i1 = 0;
            j1 = 0;
            k1 = 1;
            i2 = 1;
            j2 = 0;
            k2 = 1;
        }
    } else if y0 < z0 {
        i1 = 0;
        j1 = 0;
        k1 = 1;
        i2 = 0;
        j2 = 1;
        k2 = 1;
    } else if x0 < z0 {
        i1 = 0;
        j1 = 1;
        k1 = 0;
        i2 = 0;
        j2 = 1;
        k2 = 1;
    } else {
        i1 = 0;
        j1 = 1;
        k1 = 0;
        i2 = 1;
        j2 = 1;
        k2 = 0;
    }

    let x1 = x0 - i1 as f64 + G3;
    let y1 = y0 - j1 as f64 + G3;
    let z1 = z0 - k1 as f64 + G3;
    let x2 = x0 - i2 as f64 + 2.0 * G3;
    let y2 = y0 - j2 as f64 + 2.0 * G3;
    let z2 = z0 - k2 as f64 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    // Hash coordinates (AM uses & KotlinVersion.MAX_COMPONENT_VALUE which is & 255)
    let ii = (i & 255) as usize;
    let jj = (j & 255) as usize;
    let kk = (k & 255) as usize;

    // AM: permMod12[perm[perm[kk] + jj] + ii]
    // Uses 512-entry PERM table (built with % 255) so sums up to 510 are valid indices.
    let gi0 = PERM_MOD12[PERM[PERM[kk] + jj] + ii];
    let gi1 = PERM_MOD12[PERM[PERM[kk + k1] + jj + j1] + ii + i1];
    let gi2 = PERM_MOD12[PERM[PERM[kk + k2] + jj + j2] + ii + i2];
    let gi3 = PERM_MOD12[PERM[PERM[kk + 1] + jj + 1] + ii + 1];

    // Contribution from four corners
    let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    let n0 = if t0 < 0.0 {
        0.0
    } else {
        let t0 = t0 * t0;
        t0 * t0 * dot3(&GRAD3[gi0], x0, y0, z0)
    };

    let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    let n1 = if t1 < 0.0 {
        0.0
    } else {
        let t1 = t1 * t1;
        t1 * t1 * dot3(&GRAD3[gi1], x1, y1, z1)
    };

    let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    let n2 = if t2 < 0.0 {
        0.0
    } else {
        let t2 = t2 * t2;
        t2 * t2 * dot3(&GRAD3[gi2], x2, y2, z2)
    };

    let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    let n3 = if t3 < 0.0 {
        0.0
    } else {
        let t3 = t3 * t3;
        t3 * t3 * dot3(&GRAD3[gi3], x3, y3, z3)
    };

    // Scale to [-1, 1] (AM uses 32.0 for 3D)
    (n0 + n1 + n2 + n3) * 32.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplex_noise_deterministic() {
        let v1 = simplex_noise_3d(1.0, 2.0, 3.0);
        let v2 = simplex_noise_3d(1.0, 2.0, 3.0);
        assert!((v1 - v2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_simplex_noise_range() {
        // Check many values are in reasonable range
        for i in 0..100 {
            let x = i as f64 * 0.1;
            let v = simplex_noise_3d(x * 637.729, 0.0, x * 394.417);
            assert!(
                v >= -1.5 && v <= 1.5,
                "noise value {v} out of range at x={x}"
            );
        }
    }

    #[test]
    fn test_simplex_noise_varies() {
        let v1 = simplex_noise_3d(0.0, 0.0, 0.0);
        let v2 = simplex_noise_3d(1.0, 0.0, 0.0);
        assert!((v1 - v2).abs() > 0.001, "noise should vary");
    }
}
