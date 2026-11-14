//! Validated numeric and ARGB entry for button properties.
use super::Placement;

/// Reject incomplete/invalid input instead of replacing geometry with zero.
pub fn apply_number(p: &mut Placement, field: usize, text: &str, viewport: [f32; 2]) -> bool {
    if field >= 7 {
        let Ok(color) = u32::from_str_radix(text.trim().trim_start_matches('#'), 16) else {
            return false;
        };
        if text.trim().trim_start_matches('#').len() != 8 {
            return false;
        }
        if field == 7 {
            p.button_color = color;
        } else {
            p.stroke_color = color;
        }
        return true;
    }
    let Ok(n) = text.trim().parse::<f32>() else {
        return false;
    };
    if !n.is_finite() {
        return false;
    }
    let [w, h] = viewport;
    let rect = p.rect(w / h.max(1.0));
    match field {
        0 | 1 => {
            if n <= 0.0 {
                return false;
            }
            let ratio = (n / (h.max(1.0) * p.size)).clamp(0.4, 3.2);
            if field == 0 {
                p.width_ratio = ratio;
            } else {
                p.height_ratio = ratio;
            }
        }
        2 => {
            p.center[0] = (n / w.max(1.0) + rect[2] / 2.0)
                .clamp(rect[2] / 2.0, (1.0 - rect[2] / 2.0).max(rect[2] / 2.0))
        }
        3 => {
            p.center[1] = (n / h.max(1.0) + rect[3] / 2.0)
                .clamp(rect[3] / 2.0, (1.0 - rect[3] / 2.0).max(rect[3] / 2.0))
        }
        4 => p.opacity = (n / 100.0).clamp(0.0, 1.0),
        5 => p.roundness = n.clamp(0.0, 100.0),
        6 => p.stroke_size = n.clamp(0.0, 8.0),
        _ => return false,
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_position_and_invalid_input() {
        let mut p = Placement::new(0.5, 0.5, 0.1);
        assert!(apply_number(&mut p, 0, "64", [800.0, 400.0]));
        assert!(apply_number(&mut p, 1, "80", [800.0, 400.0]));
        assert!(apply_number(&mut p, 2, "20", [800.0, 400.0]));
        assert!(apply_number(&mut p, 3, "30", [800.0, 400.0]));
        let r = p.rect(2.0);
        for (actual, expected) in r.into_iter().zip([0.025, 0.075, 0.08, 0.2]) {
            assert!((actual - expected).abs() < 0.0001);
        }
        let before = p;
        for bad in ["", "NaN", "inf", "-1", "oops"] {
            assert!(!apply_number(&mut p, 0, bad, [800.0, 400.0]));
        }
        assert_eq!(p, before);
        assert!(apply_number(&mut p, 7, "#8033AABB", [800.0, 400.0]));
        assert_eq!(p.button_color, 0x8033AABB);
    }
}
