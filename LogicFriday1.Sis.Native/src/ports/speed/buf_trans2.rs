//! Native Rust port scaffold for `sis/speed/buf_trans2.c`.
//!
//! This module ports the balanced two-level buffering partition selection from
//! the C implementation. Applying the transform to real SIS nodes remains
//! blocked on buffer recursion, delay helpers, node patching, and network
//! mutation ports.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }
}

pub fn min_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.min(right.rise), left.fall.min(right.fall))
}

pub fn req_improved(left: DelayTime, right: DelayTime) -> bool {
    (left.rise - right.rise) > 1.0e-6 && (left.fall - right.fall) > 1.0e-6
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BufferChoice {
    pub area: f64,
    pub input_load: f64,
    pub block: DelayTime,
    pub drive: DelayTime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootGateChoice {
    pub area: f64,
    pub load: f64,
    pub block: DelayTime,
    pub drive: DelayTime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutReq {
    pub req: DelayTime,
    pub cumulative_load_start: f64,
    pub cumulative_load_end: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trans2Input {
    pub num_pos: usize,
    pub num_neg: usize,
    pub max_input_load: f64,
    pub auto_route: f64,
    pub original_required: DelayTime,
    pub previous_drive: DelayTime,
    pub original_pin_load: f64,
    pub req_diff: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2Candidate {
    pub root_gate: usize,
    pub neg_partitions: usize,
    pub pos_partitions: usize,
    pub pos_buffer: usize,
    pub neg_buffer: usize,
    pub mid_buffer: usize,
    pub req_at_root: DelayTime,
    pub area: f64,
    pub met_target: bool,
}

pub fn partition_ranges(count: usize, partitions: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for k in (1..=partitions).rev() {
        let end = start + (count - start) / k;
        ranges.push((start, end));
        start = end;
    }
    ranges
}

pub fn subtract_delay(
    mut req: DelayTime,
    block: DelayTime,
    drive: DelayTime,
    load: f64,
) -> DelayTime {
    req.rise -= block.rise + drive.rise * load;
    req.fall -= block.fall + drive.fall * load;
    req
}

pub fn drive_adjustment(mut req: DelayTime, drive: DelayTime, load: f64) -> DelayTime {
    req.rise -= drive.rise * load;
    req.fall -= drive.fall * load;
    req
}

fn grouped_required(
    fanouts: &[FanoutReq],
    partitions: usize,
    buffer: BufferChoice,
    auto_route: f64,
) -> (DelayTime, f64, f64) {
    if fanouts.is_empty() || partitions == 0 {
        return (DelayTime::new(POS_LARGE, POS_LARGE), 0.0, 0.0);
    }

    let mut req = DelayTime::new(POS_LARGE, POS_LARGE);
    for (start, end) in partition_ranges(fanouts.len(), partitions) {
        let load = fanouts[end - 1].cumulative_load_end - fanouts[start].cumulative_load_start;
        let group_req = fanouts[start..end]
            .iter()
            .fold(DelayTime::new(POS_LARGE, POS_LARGE), |acc, fanout| {
                min_delay(acc, fanout.req)
            });
        req = min_delay(
            req,
            subtract_delay(group_req, buffer.block, buffer.drive, load),
        );
    }

    (
        req,
        partitions as f64 * (auto_route + buffer.input_load),
        partitions as f64 * buffer.area,
    )
}

pub fn choose_trans2_candidate(
    input: Trans2Input,
    positive: &[FanoutReq],
    negative: &[FanoutReq],
    root_gates: &[RootGateChoice],
    buffers: &[BufferChoice],
) -> Option<Trans2Candidate> {
    let target = DelayTime::new(
        input.original_required.rise + input.req_diff.rise,
        input.original_required.fall + input.req_diff.fall,
    );
    let adjusted_original = drive_adjustment(
        input.original_required,
        input.previous_drive,
        input.original_pin_load,
    );

    let pos_parts = (input.num_pos / 2).max(usize::from(input.num_pos == 1));
    let neg_parts = (input.num_neg / 2).max(usize::from(input.num_neg == 1));
    if pos_parts + neg_parts == 0 {
        return None;
    }

    let mut best: Option<Trans2Candidate> = None;
    let mut min_area = POS_LARGE;
    for (g, root) in root_gates.iter().enumerate() {
        if root.load > input.max_input_load {
            continue;
        }
        for (g_i, neg_buffer) in buffers.iter().enumerate() {
            let neg_limit = if input.num_neg == 0 {
                0
            } else {
                neg_parts.max(1)
            };
            for i in 0..=neg_limit {
                if input.num_neg > 0 && i == 0 {
                    continue;
                }
                let (req_gi, load_gi, area_gi) =
                    grouped_required(negative, i, *neg_buffer, input.auto_route);
                for (a, pos_buffer) in buffers.iter().enumerate() {
                    let pos_limit = if input.num_pos == 0 {
                        0
                    } else {
                        pos_parts.max(1)
                    };
                    for j in 0..=pos_limit {
                        if input.num_pos > 0 && j == 0 {
                            continue;
                        }
                        let (req_a, load_a, area_a) =
                            grouped_required(positive, j, *pos_buffer, input.auto_route);
                        for (b, mid_buffer) in buffers.iter().enumerate() {
                            let req_b = if input.num_pos > 0 {
                                subtract_delay(req_a, mid_buffer.block, mid_buffer.drive, load_a)
                            } else {
                                req_a
                            };
                            let load_b = if input.num_pos > 0 {
                                mid_buffer.input_load + input.auto_route
                            } else {
                                0.0
                            };
                            let area_b = if input.num_pos > 0 {
                                mid_buffer.area
                            } else {
                                0.0
                            };
                            let req_g = min_delay(req_b, req_gi);
                            let load_op_g = load_b + load_gi;
                            let req_at_root = drive_adjustment(
                                subtract_delay(req_g, root.block, root.drive, load_op_g),
                                input.previous_drive,
                                root.load,
                            );
                            let area = root.area + area_a + area_b + area_gi;
                            let met_target = req_improved(req_at_root, target);
                            let improves_best = best.as_ref().is_none_or(|candidate| {
                                req_improved(req_at_root, candidate.req_at_root)
                            });
                            if (met_target && area < min_area) || (!met_target && improves_best) {
                                if met_target {
                                    min_area = area;
                                }
                                best = Some(Trans2Candidate {
                                    root_gate: g,
                                    neg_partitions: i,
                                    pos_partitions: j,
                                    pos_buffer: a,
                                    neg_buffer: g_i,
                                    mid_buffer: b,
                                    req_at_root,
                                    area,
                                    met_target,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    best.filter(|candidate| req_improved(candidate.req_at_root, adjusted_original))
}

pub fn buf_evaluate_trans2_bound() -> Result<(), BufTrans2Error> {
    Err(BufTrans2Error::MissingDependency(
        "buf_evaluate_trans2 requires native buffer recursion, delay helpers, buffer utilities, node patching, and network mutation ports",
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BufTrans2Error {
    MissingDependency(&'static str),
}

impl fmt::Display for BufTrans2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for BufTrans2Error {}

#[cfg(test)]
mod tests {
    use super::*;

    fn fan(req: f64, start: f64, end: f64) -> FanoutReq {
        FanoutReq {
            req: DelayTime::new(req, req),
            cumulative_load_start: start,
            cumulative_load_end: end,
        }
    }

    fn buf(area: f64, load: f64, block: f64, drive: f64) -> BufferChoice {
        BufferChoice {
            area,
            input_load: load,
            block: DelayTime::new(block, block),
            drive: DelayTime::new(drive, drive),
        }
    }

    #[test]
    fn partition_ranges_match_c_balanced_loop() {
        assert_eq!(partition_ranges(5, 2), vec![(0, 2), (2, 5)]);
        assert_eq!(partition_ranges(6, 3), vec![(0, 2), (2, 4), (4, 6)]);
    }

    #[test]
    fn grouped_required_subtracts_buffer_delay_per_partition() {
        let (req, load, area) = grouped_required(
            &[fan(10.0, 0.0, 2.0), fan(8.0, 2.0, 5.0)],
            2,
            buf(3.0, 1.0, 1.0, 0.5),
            0.25,
        );

        assert_eq!(req, DelayTime::new(5.5, 5.5));
        assert_eq!(load, 2.5);
        assert_eq!(area, 6.0);
    }

    #[test]
    fn choose_candidate_prefers_target_meeting_lowest_area() {
        let input = Trans2Input {
            num_pos: 2,
            num_neg: 0,
            max_input_load: 10.0,
            auto_route: 0.0,
            original_required: DelayTime::new(1.0, 1.0),
            previous_drive: DelayTime::new(0.0, 0.0),
            original_pin_load: 0.0,
            req_diff: DelayTime::new(1.0, 1.0),
        };
        let root = [RootGateChoice {
            area: 1.0,
            load: 1.0,
            block: DelayTime::new(0.0, 0.0),
            drive: DelayTime::new(0.0, 0.0),
        }];
        let buffers = [buf(3.0, 1.0, 0.0, 0.0), buf(1.0, 1.0, 0.0, 0.0)];

        let best = choose_trans2_candidate(
            input,
            &[fan(5.0, 0.0, 1.0), fan(5.0, 1.0, 2.0)],
            &[],
            &root,
            &buffers,
        )
        .unwrap();

        assert!(best.met_target);
        assert_eq!(best.pos_buffer, 1);
    }

    #[test]
    fn bound_entry_reports_missing_dependencies() {
        assert_eq!(
            buf_evaluate_trans2_bound(),
            Err(BufTrans2Error::MissingDependency(
                "buf_evaluate_trans2 requires native buffer recursion, delay helpers, buffer utilities, node patching, and network mutation ports",
            ))
        );
    }
}
