//! Human-like input behavior for mouse and keyboard.
//!
//! Generates bezier curve paths for mouse movement and variable
//! typing delays based on character frequency (common letters = faster).

use std::time::Duration;

/// Generate a bezier-based mouse path from (x1,y1) to (x2,y2).
///
/// Returns a vector of intermediate points plus the destination.
/// Each step represents ~8ms of movement for realistic cursor
/// velocity (~500px/s average).
#[must_use]
pub fn bezier_path(
    from: (f64, f64),
    to: (f64, f64),
) -> Vec<(f64, f64)> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 5.0 {
        return vec![to];
    }

    // Control points: random offset perpendicular to the line
    let angle = dy.atan2(dx);
    let perp = angle + std::f64::consts::FRAC_PI_2;

    // Random offset magnitude proportional to distance
    let offset = dist * 0.15 * fast_random();
    let wobble_x = offset * perp.cos();
    let wobble_y = offset * perp.sin();

    let cp1 = (from.0 + dx * 0.33 + wobble_x, from.1 + dy * 0.33 + wobble_y);
    let cp2 = (from.0 + dx * 0.66 - wobble_x, from.1 + dy * 0.66 - wobble_y);

    // Number of steps based on distance (realistic: ~500px/s, ~8ms/step)
    let steps = (dist / 4.0) as usize;
    let steps = steps.clamp(8, 120);

    let mut path = Vec::with_capacity(steps + 1);
    for i in 0..steps {
        let t = (i + 1) as f64 / steps as f64;
        // Cubic bezier: B(t) = (1-t)^3*P0 + 3(1-t)^2*t*P1 + 3(1-t)*t^2*P2 + t^3*P3
        let mt = 1.0 - t;
        let x = mt * mt * mt * from.0
            + 3.0 * mt * mt * t * cp1.0
            + 3.0 * mt * t * t * cp2.0
            + t * t * t * to.0;
        let y = mt * mt * mt * from.1
            + 3.0 * mt * mt * t * cp1.1
            + 3.0 * mt * t * t * cp2.1
            + t * t * t * to.1;

        // Add micro-jitter: human hand tremor (sub-pixel)
        let jitter_x = (fast_random() - 0.5) * 0.8;
        let jitter_y = (fast_random() - 0.5) * 0.8;

        path.push((x + jitter_x, y + jitter_y));
    }

    path
}

/// Human-like typing delay for a given character.
///
/// Common letters (e, t, a, o, i, n, s, h, r) get 40-80ms.
/// Rare letters get 60-120ms.
/// Space and punctuation get 30-60ms.
/// Uppercase adds shift-key overhead.
#[must_use]
pub fn typing_delay(c: char, _total_chars_typed: usize) -> Duration {
    let base_ms = match c {
        ' ' | '.' | ',' | ';' => fast_random_range(30.0, 60.0),
        'e' | 't' | 'a' | 'o' | 'i' | 'n' | 's' | 'h' | 'r' => {
            fast_random_range(40.0, 80.0)
        }
        _ if c.is_uppercase() => fast_random_range(70.0, 140.0), // shift overhead
        _ => fast_random_range(60.0, 120.0),
    };
    Duration::from_millis(base_ms as u64)
}

/// Random delay between actions (think time).
#[must_use]
pub fn think_delay() -> Duration {
    Duration::from_millis(fast_random_range(200.0, 800.0) as u64)
}

/// Random delay after a click (reaction time).
#[must_use]
pub fn click_linger() -> Duration {
    Duration::from_millis(fast_random_range(150.0, 400.0) as u64)
}

/// Sub-pixel position jitter for click coordinates.
/// Prevents clicking the exact center of elements every time.
#[must_use]
pub fn position_jitter() -> f64 {
    (fast_random() - 0.5) * 4.0
}

// ── Internal fast RNG (no crypto needed for humanization) ──────────

fn fast_random() -> f64 {
    // Simple xorshift — fast, no allocation, good enough for jitter
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(0xDEAD_BEEF_CAFE_BABE);
    }
    STATE.with(|state| {
        let mut x = state.get();
        if x == 0 { x = 0xDEAD_BEEF_CAFE_BABE; }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        state.set(x);
        (x as f64) / (u64::MAX as f64)
    })
}

fn fast_random_range(min: f64, max: f64) -> f64 {
    min + fast_random() * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_short_path() {
        let path = bezier_path((100.0, 100.0), (102.0, 102.0));
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], (102.0, 102.0));
    }

    #[test]
    fn test_bezier_long_path() {
        let path = bezier_path((0.0, 0.0), (500.0, 300.0));
        // Should have many intermediate points
        assert!(path.len() > 10);
        // Last point should be very close to destination
        let last = path.last().expect("bezier_path always returns at least one point");
        assert!((last.0 - 500.0).abs() < 3.0);
        assert!((last.1 - 300.0).abs() < 3.0);
    }

    #[test]
    fn test_bezier_points_are_monotonic_ish() {
        let path = bezier_path((0.0, 0.0), (100.0, 100.0));
        // Path should generally progress toward destination
        for i in 1..path.len() {
            let prev = path[i - 1];
            let curr = path[i];
            let dist_before = (prev.0 * prev.0 + prev.1 * prev.1).sqrt();
            let dist_after = (curr.0 * curr.0 + curr.1 * curr.1).sqrt();
            // Points get further from origin (generally)
            assert!(
                dist_after > dist_before - 2.0,
                "point {i} ({curr:?}) moved backwards from {prev:?}"
            );
        }
    }

    #[test]
    fn test_typing_delay_reasonable() {
        for c in ['e', 'A', ' ', 'q'] {
            let delay = typing_delay(c, 0);
            assert!(delay.as_millis() >= 30, "delay for '{c}' too short");
            assert!(delay.as_millis() <= 150, "delay for '{c}' too long");
        }
    }

    #[test]
    fn test_think_delay_reasonable() {
        for _ in 0..10 {
            let d = think_delay();
            assert!(d.as_millis() >= 200);
            assert!(d.as_millis() <= 800);
        }
    }
}
