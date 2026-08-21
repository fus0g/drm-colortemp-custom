//! Temperature conversion algorithms for DRM color adjustment.
//!
//! Implements the Tanner Helland color temperature to RGB conversion formula.

use std::f64;

/// Convert color temperature (Kelvin) to RGB multipliers.
///
/// Based on Tanner Helland's color temperature formula:
/// http://www.tannerhelland.com/4435/convert-temperature-rgb-algorithm/
///
/// # Arguments
/// * `temp` - Color temperature in Kelvin (1000-10000)
///
/// # Returns
/// Tuple of (red, green, blue) multipliers in range [0, 1]
pub fn temp_to_rgb(temp: u32) -> (f64, f64, f64) {
    let temp_f = temp as f64 / 100.0;
    let red = calc_red(temp_f);
    let green = calc_green(temp_f);
    let blue = calc_blue(temp_f);
    (red, green, blue)
}

/// Calculate red component from temperature.
fn calc_red(temp: f64) -> f64 {
    if temp <= 66.0 {
        1.0
    } else {
        // Red starts to decrease at higher temperatures
        let red = 329.698727446 * (temp - 60.0).powf(-0.1332047592);
        (red / 255.0).min(1.0)
    }
}

/// Calculate green component from temperature.
fn calc_green(temp: f64) -> f64 {
    let green = if temp <= 66.0 {
        // Green for lower temperatures
        99.4708025861 * temp.ln() - 161.1195681661
    } else {
        // Green for higher temperatures
        288.1221695283 * (temp - 60.0).powf(-0.0755148492)
    };
    (green / 255.0).clamp(0.0, 1.0)
}

/// Calculate blue component from temperature.
fn calc_blue(temp: f64) -> f64 {
    if temp >= 66.0 {
        1.0
    } else if temp <= 19.0 {
        0.0
    } else {
        // Blue for mid-range temperatures
        let blue = 138.5177312231 * (temp - 10.0).ln() - 305.0447927307;
        (blue / 255.0).clamp(0.0, 1.0)
    }
}

/// Generate gamma correction lookup table.
///
/// # Arguments
/// * `gamma_size` - Size of the LUT (typically 256)
/// * `temp` - Color temperature in Kelvin
/// * `brightness` - Brightness multiplier (0.0 to 1.0)
///
/// # Returns
/// Tuple of (red_lut, green_lut, blue_lut) vectors
pub fn generate_gamma_luts(
    gamma_size: usize,
    temp: u32,
    brightness: f64,
) -> (Vec<u16>, Vec<u16>, Vec<u16>) {
    let (r_mult, g_mult, b_mult) = temp_to_rgb(temp);
    let r_mult = r_mult * brightness;
    let g_mult = g_mult * brightness;
    let b_mult = b_mult * brightness;

    let mut red_lut = Vec::with_capacity(gamma_size);
    let mut green_lut = Vec::with_capacity(gamma_size);
    let mut blue_lut = Vec::with_capacity(gamma_size);

    let to_u16 = |v: f64| -> u16 { (v * 65535.0).clamp(0.0, 65535.0) as u16 };

    // Handle edge case: gamma_size < 2
    if gamma_size < 2 {
        red_lut.push(to_u16(r_mult));
        green_lut.push(to_u16(g_mult));
        blue_lut.push(to_u16(b_mult));
        return (red_lut, green_lut, blue_lut);
    }

    for i in 0..gamma_size {
        let value = i as f64 / (gamma_size - 1) as f64;
        red_lut.push(to_u16(value * r_mult));
        green_lut.push(to_u16(value * g_mult));
        blue_lut.push(to_u16(value * b_mult));
    }

    (red_lut, green_lut, blue_lut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_to_rgb_6500k() {
        // 6500K should be close to white (1.0, 1.0, 1.0)
        let (r, g, b) = temp_to_rgb(6500);
        assert!(r > 0.9 && r <= 1.0);
        assert!(g > 0.9 && g <= 1.0);
        assert!(b > 0.9 && b <= 1.0);
    }

    #[test]
    fn test_temp_to_rgb_3500k() {
        // 3500K should be warm (more red, less blue)
        let (r, _g, b) = temp_to_rgb(3500);
        assert!(r > b); // Red should dominate blue
        assert!(r > 0.9);
    }

    #[test]
    fn test_gamma_lut_generation() {
        let (red, green, blue) = generate_gamma_luts(256, 6500, 1.0);
        assert_eq!(red.len(), 256);
        assert_eq!(green.len(), 256);
        assert_eq!(blue.len(), 256);
    }

    #[test]
    fn test_gamma_lut_edge_case_size_1() {
        let (red, green, blue) = generate_gamma_luts(1, 6500, 1.0);
        assert_eq!(red.len(), 1);
        assert_eq!(green.len(), 1);
        assert_eq!(blue.len(), 1);
    }

    #[test]
    fn test_temp_to_rgb_cool_temperatures() {
        // High color temperatures (cool white, e.g. 8000K, 8500K)
        // Red should decrease, blue should stay 1.0
        let (r, g, b) = temp_to_rgb(8200);
        assert!(r < 1.0, "Red multiplier should be less than 1.0 at 8200K");
        assert_eq!(b, 1.0, "Blue multiplier should be 1.0 at 8200K");
        assert!(g > 0.8 && g <= 1.0);
    }

    #[test]
    fn test_gamma_lut_monotonic_increasing() {
        let (red, green, blue) = generate_gamma_luts(256, 8000, 1.0);
        for i in 1..256 {
            assert!(
                red[i] >= red[i - 1],
                "Red LUT is not monotonic at index {i}"
            );
            assert!(
                green[i] >= green[i - 1],
                "Green LUT is not monotonic at index {i}"
            );
            assert!(
                blue[i] >= blue[i - 1],
                "Blue LUT is not monotonic at index {i}"
            );
        }
        assert_eq!(red[0], 0);
        assert_eq!(green[0], 0);
        assert_eq!(blue[0], 0);
    }
}
