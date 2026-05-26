//! Native Rust port of `sis/map/pwl.c`.
//!
//! SIS used this package to model piecewise-linear scalar functions for
//! delay-oriented mapping. The C representation stored sorted breakpoints and
//! an opaque `char *data` payload for the solution selected on each segment.
//! This port keeps the same breakpoint semantics with owned Rust values.

#[derive(Clone, Debug, PartialEq)]
pub struct PiecewisePoint<T> {
    pub x: f64,
    pub y: f64,
    pub slope: f64,
    pub data: Option<T>,
}

impl<T> PiecewisePoint<T> {
    pub fn new(x: f64, y: f64, slope: f64, data: Option<T>) -> Self {
        Self { x, y, slope, data }
    }

    pub fn eval(&self, x: f64) -> f64 {
        self.y + self.slope * (x - self.x)
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PiecewiseLinear<T> {
    points: Vec<PiecewisePoint<T>>,
}

impl<T> PiecewiseLinear<T> {
    pub fn empty() -> Self {
        Self { points: Vec::new() }
    }

    pub fn extract(points: Vec<PiecewisePoint<T>>) -> Self {
        Self { points }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn points(&self) -> &[PiecewisePoint<T>] {
        &self.points
    }

    pub fn lookup(&self, x: f64) -> Option<&PiecewisePoint<T>> {
        match self.points.binary_search_by(|point| point.x.total_cmp(&x)) {
            Ok(index) => self.points.get(index),
            Err(0) => self.points.first(),
            Err(index) if index >= self.points.len() => self.points.last(),
            Err(index) => self.points.get(index - 1),
        }
    }

    pub fn eval(&self, x: f64) -> Option<f64> {
        self.lookup(x).map(|point| point.eval(x))
    }

    pub fn select(&self, x: f64) -> Option<&T> {
        self.lookup(x).and_then(PiecewisePoint::data)
    }
}

impl<T> PiecewiseLinear<T>
where
    T: Clone,
{
    pub fn create(mut points: Vec<PiecewisePoint<T>>) -> Self {
        points.sort_by(|left, right| left.x.total_cmp(&right.x));
        Self { points }
    }

    pub fn set_data(&mut self, data: Option<T>) {
        for point in &mut self.points {
            point.data = data.clone();
        }
    }

    pub fn shift(&self, shift: f64) -> Self {
        if self.points.is_empty() || shift == 0.0 {
            return self.clone();
        }
        assert!(shift > 0.0, "PWL shift must be non-negative");

        let Some(start) = self.lookup(shift) else {
            return Self::empty();
        };
        let mut points = Vec::new();
        points.push(PiecewisePoint {
            x: 0.0,
            y: start.eval(shift),
            slope: start.slope,
            data: start.data.clone(),
        });
        points.extend(
            self.points
                .iter()
                .filter(|point| point.x > shift)
                .cloned()
                .map(|mut point| {
                    point.x -= shift;
                    point
                }),
        );

        Self::extract(points)
    }
}

impl<T> PiecewiseLinear<T>
where
    T: Clone + PartialEq,
{
    pub fn min(&self, other: &Self) -> Self {
        if self.points.is_empty() {
            return other.clone();
        }
        if other.points.is_empty() {
            return self.clone();
        }

        compute_min(self, other)
    }

    pub fn max(&self, other: &Self) -> Self {
        self.negated().min(&other.negated()).negated()
    }

    pub fn sum(&self, other: &Self) -> Self {
        if self.points.is_empty() {
            return other.clone();
        }
        if other.points.is_empty() {
            return self.clone();
        }
        assert_eq!(
            self.points[0].x, other.points[0].x,
            "PWL sum requires matching initial x values"
        );

        let mut breakpoints: Vec<f64> = self
            .points
            .iter()
            .chain(other.points.iter())
            .map(|point| point.x)
            .collect();
        breakpoints.sort_by(f64::total_cmp);
        breakpoints.dedup_by(|left, right| *left == *right);
        let mut points = Vec::new();

        for x in breakpoints {
            let left = self.lookup(x).expect("sum inputs must be non-empty");
            let right = other.lookup(x).expect("sum inputs must be non-empty");
            let data = if self.points.iter().any(|point| point.x == x) {
                left.data.clone()
            } else {
                right.data.clone()
            };
            points.push(PiecewisePoint {
                x,
                y: left.eval(x) + right.eval(x),
                slope: left.slope + right.slope,
                data,
            });
        }

        Self::extract(points)
    }

    pub fn linear_max(lines: &[PiecewisePoint<T>]) -> Self {
        if lines.is_empty() {
            return Self::empty();
        }

        let mut valid = vec![true; lines.len()];
        let mut x = 0.0;
        let max_slope = lines
            .iter()
            .map(|point| point.slope)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut points = Vec::new();

        loop {
            let selected = argmax_y(x, lines, &valid);
            let selected_line = &lines[selected];
            let point = PiecewisePoint {
                x,
                y: selected_line.y + selected_line.slope * (x - selected_line.x),
                slope: selected_line.slope,
                data: selected_line.data.clone(),
            };
            valid[selected] = false;
            let reached_max_slope = point.slope == max_slope;
            points.push(point.clone());

            if reached_max_slope {
                break;
            }

            let mut next_x = f64::INFINITY;
            for (index, line) in lines.iter().enumerate() {
                if !valid[index] {
                    continue;
                }
                if point.slope >= line.slope {
                    valid[index] = false;
                    continue;
                }

                let intersect = compute_intersect(&point, line);
                if intersect <= x {
                    continue;
                }
                next_x = next_x.min(intersect);
            }

            if next_x >= f64::INFINITY {
                break;
            }
            x = next_x;
        }

        Self::extract(points)
    }

    fn negated(&self) -> Self {
        Self {
            points: self
                .points
                .iter()
                .cloned()
                .map(|mut point| {
                    point.y = -point.y;
                    point.slope = -point.slope;
                    point
                })
                .collect(),
        }
    }
}

fn compute_min<T>(left: &PiecewiseLinear<T>, right: &PiecewiseLinear<T>) -> PiecewiseLinear<T>
where
    T: Clone + PartialEq,
{
    let fns = [left, right];
    let mut indices = [0usize, 0usize];
    let mut x = 0.0;
    let mut select = first_selected(left, right);
    let mut intersect_flag = false;
    let mut points = Vec::new();
    let mut last: Option<PiecewisePoint<T>> = None;

    loop {
        let best = &fns[select].points[indices[select]];
        if !last
            .as_ref()
            .is_some_and(|point| point.data == best.data && point.slope == best.slope)
        {
            let point = PiecewisePoint {
                x,
                y: best.eval(x),
                slope: best.slope,
                data: best.data.clone(),
            };
            last = Some(point.clone());
            points.push(point);
        }

        let Some(next) = next_min_point(fns, indices, x, intersect_flag) else {
            break;
        };
        indices = next.indices;
        x = next.x;
        select = next.select;
        intersect_flag = next.intersect_flag;
    }

    PiecewiseLinear::extract(points)
}

fn first_selected<T>(left: &PiecewiseLinear<T>, right: &PiecewiseLinear<T>) -> usize {
    let mut diff = left.points[0].y - right.points[0].y;
    if diff == 0.0 {
        diff = left.points[0].slope - right.points[0].slope;
    }
    if diff <= 0.0 { 0 } else { 1 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NextPoint {
    indices: [usize; 2],
    x: f64,
    select: usize,
    intersect_flag: bool,
}

fn next_min_point<T>(
    fns: [&PiecewiseLinear<T>; 2],
    mut indices: [usize; 2],
    x: f64,
    intersect_flag: bool,
) -> Option<NextPoint> {
    let mins = [
        next_breakpoint(fns[0], indices[0]),
        next_breakpoint(fns[1], indices[1]),
    ];
    let min = mins[0].min(mins[1]);
    let intersect = if intersect_flag {
        f64::NEG_INFINITY
    } else {
        compute_intersect(&fns[0].points[indices[0]], &fns[1].points[indices[1]])
    };

    if intersect > x && intersect < min {
        let diff = fns[0].points[indices[0]].slope - fns[1].points[indices[1]].slope;
        assert_ne!(diff, 0.0);
        return Some(NextPoint {
            indices,
            x: intersect,
            select: if diff < 0.0 { 0 } else { 1 },
            intersect_flag: true,
        });
    }

    if mins[0] == f64::INFINITY && mins[1] == f64::INFINITY {
        return None;
    }

    if min >= mins[0] {
        indices[0] += 1;
    }
    if min >= mins[1] {
        indices[1] += 1;
    }

    let seg = [&fns[0].points[indices[0]], &fns[1].points[indices[1]]];
    let mut diff = seg[0].eval(min) - seg[1].eval(min);
    if diff == 0.0 {
        diff = seg[0].slope - seg[1].slope;
    }

    Some(NextPoint {
        indices,
        x: min,
        select: if diff <= 0.0 { 0 } else { 1 },
        intersect_flag: false,
    })
}

fn next_breakpoint<T>(pwl: &PiecewiseLinear<T>, index: usize) -> f64 {
    if index + 1 == pwl.points.len() {
        f64::INFINITY
    } else {
        pwl.points[index + 1].x
    }
}

fn compute_intersect<T>(left: &PiecewisePoint<T>, right: &PiecewisePoint<T>) -> f64 {
    if left.slope == right.slope {
        return f64::NEG_INFINITY;
    }

    ((right.y - right.slope * right.x) - (left.y - left.slope * left.x))
        / (left.slope - right.slope)
}

fn argmax_y<T>(x: f64, points: &[PiecewisePoint<T>], valid: &[bool]) -> usize {
    let mut y = f64::NEG_INFINITY;
    let mut slope = f64::NEG_INFINITY;
    let mut argmax = None;

    for (index, point) in points.iter().enumerate() {
        if !valid[index] {
            continue;
        }

        let point_y = point.eval(x);
        if y < point_y || (y == point_y && slope < point.slope) {
            argmax = Some(index);
            y = point_y;
            slope = point.slope;
        }
    }

    argmax.expect("linear_max requires at least one valid point")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64, slope: f64, data: &'static str) -> PiecewisePoint<&'static str> {
        PiecewisePoint::new(x, y, slope, Some(data))
    }

    fn approx_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "actual {actual} expected {expected}"
        );
    }

    #[test]
    fn create_sorts_points_and_lookup_selects_owning_segment() {
        let pwl =
            PiecewiseLinear::create(vec![point(5.0, 11.0, 3.0, "b"), point(0.0, 1.0, 2.0, "a")]);

        assert_eq!(pwl.points()[0].x, 0.0);
        assert_eq!(pwl.select(-1.0), Some(&"a"));
        assert_eq!(pwl.select(4.0), Some(&"a"));
        assert_eq!(pwl.select(5.0), Some(&"b"));
        assert_eq!(pwl.select(99.0), Some(&"b"));
        approx_eq(pwl.eval(3.0).unwrap(), 7.0);
    }

    #[test]
    fn min_splits_at_line_intersection_and_keeps_lower_segment() {
        let falling = PiecewiseLinear::extract(vec![point(0.0, 10.0, -1.0, "falling")]);
        let rising = PiecewiseLinear::extract(vec![point(0.0, 0.0, 1.0, "rising")]);

        let result = falling.min(&rising);

        assert_eq!(result.points().len(), 2);
        assert_eq!(result.points()[0].data, Some("rising"));
        approx_eq(result.points()[0].x, 0.0);
        assert_eq!(result.points()[1].data, Some("falling"));
        approx_eq(result.points()[1].x, 5.0);
        approx_eq(result.eval(2.0).unwrap(), 2.0);
        approx_eq(result.eval(8.0).unwrap(), 2.0);
    }

    #[test]
    fn max_is_dual_of_min() {
        let falling = PiecewiseLinear::extract(vec![point(0.0, 10.0, -1.0, "falling")]);
        let rising = PiecewiseLinear::extract(vec![point(0.0, 0.0, 1.0, "rising")]);

        let result = falling.max(&rising);

        assert_eq!(result.points().len(), 2);
        assert_eq!(result.points()[0].data, Some("falling"));
        assert_eq!(result.points()[1].data, Some("rising"));
        approx_eq(result.points()[1].x, 5.0);
        approx_eq(result.eval(2.0).unwrap(), 8.0);
        approx_eq(result.eval(8.0).unwrap(), 8.0);
    }

    #[test]
    fn sum_merges_breakpoints_from_both_inputs() {
        let left =
            PiecewiseLinear::extract(vec![point(0.0, 1.0, 2.0, "l0"), point(3.0, 7.0, 1.0, "l1")]);
        let right = PiecewiseLinear::extract(vec![
            point(0.0, 4.0, -1.0, "r0"),
            point(5.0, -1.0, 4.0, "r1"),
        ]);

        let result = left.sum(&right);

        assert_eq!(result.points().len(), 3);
        assert_eq!(result.points()[0].data, Some("l0"));
        assert_eq!(result.points()[1].data, Some("l1"));
        assert_eq!(result.points()[2].data, Some("r1"));
        approx_eq(result.points()[1].x, 3.0);
        approx_eq(result.points()[1].y, 8.0);
        approx_eq(result.points()[2].x, 5.0);
        approx_eq(result.points()[2].y, 8.0);
        approx_eq(result.eval(6.0).unwrap(), 13.0);
    }

    #[test]
    fn shift_reanchors_at_lookup_segment() {
        let pwl =
            PiecewiseLinear::extract(vec![point(0.0, 0.0, 2.0, "a"), point(4.0, 8.0, 3.0, "b")]);

        let result = pwl.shift(2.0);

        assert_eq!(result.points().len(), 2);
        assert_eq!(result.points()[0].data, Some("a"));
        approx_eq(result.points()[0].x, 0.0);
        approx_eq(result.points()[0].y, 4.0);
        approx_eq(result.points()[1].x, 2.0);
        approx_eq(result.eval(3.0).unwrap(), 11.0);
    }

    #[test]
    fn linear_max_constructs_upper_envelope() {
        let result = PiecewiseLinear::linear_max(&[
            point(0.0, 6.0, 0.0, "flat"),
            point(0.0, 0.0, 2.0, "rising"),
            point(0.0, 3.0, 1.0, "middle"),
        ]);

        assert_eq!(result.points().len(), 2);
        assert_eq!(result.points()[0].data, Some("flat"));
        assert_eq!(result.points()[1].data, Some("rising"));
        approx_eq(result.points()[1].x, 3.0);
        approx_eq(result.eval(2.0).unwrap(), 6.0);
        approx_eq(result.eval(4.0).unwrap(), 8.0);
    }

    #[test]
    fn set_data_assigns_payload_to_all_segments() {
        let mut pwl = PiecewiseLinear::extract(vec![point(0.0, 0.0, 1.0, "old")]);

        pwl.set_data(Some("new"));

        assert_eq!(pwl.select(10.0), Some(&"new"));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("pwl.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
