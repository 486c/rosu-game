use rosu_map::util::Pos;

use crate::hit_objects::{Object, ObjectKind};

/// Maximum distance between two objects (in osu!pixels) for them to be
/// considered stacked. Taken from osu!stable's stacking implementation.
pub const STACK_LENIENCE: f32 = 3.0;

/// One hit object reduced down to the data the stacking algorithm needs.
///
/// Positions here are always the *original* (unstacked) ones. Stack counts are
/// computed on these entries first and only afterwards applied to the real
/// objects, so distance checks are never polluted by offsets.
struct StackEntry {
    start_time: f64,
    end_time: f64,
    pos: Pos,
    /// End position; differs from `pos` for sliders only.
    end_pos: Pos,
    is_slider: bool,
    stack_count: i32,
}

impl StackEntry {
    fn from_object(object: &Object) -> Self {
        match &object.kind {
            ObjectKind::Circle(circle) => Self {
                start_time: object.start_time,
                end_time: object.end_time(),
                pos: circle.pos,
                end_pos: circle.pos,
                is_slider: false,
                stack_count: object.stack_count,
            },
            ObjectKind::Slider(slider) => Self {
                start_time: object.start_time,
                end_time: slider.end_time(),
                pos: slider.pos,
                end_pos: slider.pos + slider.curve.position_at(1.0),
                is_slider: true,
                stack_count: object.stack_count,
            },
        }
    }
}


fn compute_stack_counts(objects: &mut [StackEntry], preempt: f64, stack_leniency: f32) {
    let stack_window = preempt * f64::from(stack_leniency);

    // Reverse pass for the stack calculation.
    for i in (1..objects.len()).rev() {
        /* We should check every note which has not yet got a stack.
         * Consider the case we have two interwound stacks and this will make
         * sense:
         *
         *  o <-1      o <-2
         *   o <-3      o <-4
         *
         * We first process starting from 4 and handle 2, then we come
         * backwards on the i loop iteration until we reach 3 and handle 1.
         * 2 and 1 will be ignored in the i loop because they already have a
         * stack value.
         */
        if objects[i].stack_count != 0 {
            continue;
        }

        let mut n = i;

        if objects[i].is_slider {
            /* We have hit the first slider in a possible stack.
             * From this point on, we ALWAYS stack positive regardless.
             */
            let mut oi_start = objects[i].start_time;
            let mut oi_pos = objects[i].pos;
            let mut oi_stack = objects[i].stack_count;

            while n > 0 {
                n -= 1;

                if oi_start - stack_window > objects[n].start_time {
                    // We are no longer within stacking range of the previous object.
                    break;
                }

                let pos_n = if objects[n].is_slider {
                    objects[n].end_pos
                } else {
                    objects[n].pos
                };

                if pos_n.distance(oi_pos) < STACK_LENIENCE {
                    objects[n].stack_count = oi_stack + 1;
                    oi_start = objects[n].start_time;
                    oi_pos = objects[n].pos;
                    oi_stack = objects[n].stack_count;
                }
            }
        } else {
            /* If this object is a hitcircle, then we enter this "special" case.
             * It either ends with a stack of hitcircles only, or a stack of
             * hitcircles that are underneath a slider.
             * Any other case is handled by the "is Slider" code below this.
             */
            let mut oi_start = objects[i].start_time;
            let mut oi_pos = objects[i].pos;
            let mut oi_stack = objects[i].stack_count;

            while n > 0 {
                n -= 1;

                if oi_start - stack_window > objects[n].end_time {
                    // We are no longer within stacking range of the previous object.
                    break;
                }

                /* This is a special case where hitcircles are moved DOWN and
                 * RIGHT (negative stacking) if they are under the *last*
                 * slider in a stacked pattern.
                 *
                 *      o==o <- slider is at original location
                 *          o <- hitCircle has stack of -1
                 *           o <- hitCircle has stack of -2
                 */
                if objects[n].is_slider && objects[n].end_pos.distance(oi_pos) < STACK_LENIENCE {
                    let slider_end = objects[n].end_pos;
                    let offset = oi_stack - objects[n].stack_count + 1;

                    // For each object which was declared under this slider, we
                    // will offset it to appear *below* the slider end (rather
                    // than above).
                    for object_j in &mut objects[n + 1..=i] {
                        if slider_end.distance(object_j.pos) < STACK_LENIENCE {
                            object_j.stack_count -= offset;
                        }
                    }

                    // We have hit a slider. We should restart calculation using
                    // this as the new base. Breaking here will mean that the
                    // slider still has a stack_count of 0, so it will be handled
                    // in the i-outer-loop.
                    break;
                }

                if objects[n].pos.distance(oi_pos) < STACK_LENIENCE {
                    // Keep processing as if there are no sliders. If we come
                    // across a slider, this gets cancelled out.
                    // NOTE: Sliders with start positions stacking are a special
                    // case that is also handled here.
                    objects[n].stack_count = oi_stack + 1;
                    oi_start = objects[n].start_time;
                    oi_pos = objects[n].pos;
                    oi_stack = objects[n].stack_count;
                }
            }
        }
    }
}

/// Runs the stacking algorithm on all objects and applies the resulting
/// offsets: an object with stack count `n` is moved by
/// `(-stack_offset, -stack_offset) * n`.
///
/// Note the negative sign: in osu!, higher stack counts move objects up-left
/// (osu!lazer applies `StackHeight * -6.4f`, and peppy's reference comments
/// that negative stacks - found under a slider's end - are moved "DOWN and
/// RIGHT").
pub fn apply_stacking(
    objects: &mut [Object], 
    preempt: f64, 
    stack_leniency: f32, 
    stack_offset: f32
) {
    let mut entries: Vec<StackEntry> = objects.iter().map(StackEntry::from_object).collect();

    compute_stack_counts(&mut entries, preempt, stack_leniency);

    for (object, entry) in objects.iter_mut().zip(entries) {
        object.stack_count = entry.stack_count;

        if entry.stack_count == 0 {
            continue;
        }

        let dist = stack_offset * entry.stack_count as f32;
        let offset = Pos::new(-dist, -dist);

        match &mut object.kind {
            ObjectKind::Circle(circle) => circle.pos += offset,
            ObjectKind::Slider(slider) => slider.pos += offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::case;

    const PREEMPT: f64 = 750.0; // AR9
    const LENIENCY: f32 = 0.7;

    fn circle(start_time: f64, pos: (f32, f32)) -> StackEntry {
        let pos = Pos::new(pos.0, pos.1);

        StackEntry {
            start_time,
            end_time: start_time,
            pos,
            end_pos: pos,
            is_slider: false,
            stack_count: 0,
        }
    }

    fn slider(start_time: f64, duration: f64, pos: (f32, f32), end_pos: (f32, f32)) -> StackEntry {
        StackEntry {
            start_time,
            end_time: start_time + duration,
            pos: Pos::new(pos.0, pos.1),
            end_pos: Pos::new(end_pos.0, end_pos.1),
            is_slider: true,
            stack_count: 0,
        }
    }

    fn stacks(objects: &mut [StackEntry]) -> Vec<i32> {
        compute_stack_counts(objects, PREEMPT, LENIENCY);
        objects.iter().map(|o| o.stack_count).collect()
    }
    
    #[case(
        &mut vec![
            circle(0.0, (256.0, 192.0)),
            circle(172.0, (256.0, 192.0)),
            circle(344.0, (256.0, 192.0)),
        ],
        &vec![2, 1, 0];
        "three circles in same spot"
    )]
    #[case(
        &mut vec![
            circle(0.0, (256.0, 192.0)),
            circle(526.0, (256.0, 192.0)),
        ],
        &vec![0, 0];
        "no stacking"
    )]
    #[case(
        &mut vec![
            circle(0.0, (256.0, 192.0)),
            circle(500.0, (256.0, 192.0)),
        ],
        &vec![1, 0];
        "within window edge"
    )]
    #[case(
        &mut vec![
            circle(0.0, (100.0, 100.0)),
            circle(172.0, (300.0, 300.0)),
            circle(344.0, (500.0, 500.0)),
        ],
        &vec![0, 0, 0];
        "distant objects no stacking applied"
    )]
    #[case(
        &mut vec![
            slider(0.0, 1000.0, (100.0, 100.0), (100.0, 100.0)),
            circle(1100.0, (100.0, 100.0)),
            circle(1200.0, (100.0, 100.0)),
        ],
        &vec![0, -1, -2];
        "syntetic negative stacking under slider end"
    )]
    #[case(
        &mut vec![
            slider(0.0, 1000.0, (100.0, 100.0), (400.0, 300.0)),
            circle(1100.0, (100.0, 100.0)),
            circle(1200.0, (100.0, 100.0)),
        ],
        &vec![2, 1, 0];
        "slider head stacking"
    )]
    #[case(
        &mut vec![
            circle(0.0, (100.0, 100.0)),
            circle(172.0, (100.0, 100.0)),
            slider(344.0, 1000.0, (100.0, 100.0), (400.0, 300.0)),
        ],
        &vec![2, 1, 0];
        "circles stacked on slider start"
    )]
    #[case(
        &mut vec![
            circle(0.0, (100.0, 100.0)),
            circle(86.0, (300.0, 100.0)),
            circle(172.0, (100.0, 100.0)),
            circle(258.0, (300.0, 100.0)),
        ],
        &vec![1, 1, 0, 0];
        "interwound stacks"
    )]
    #[case(
        &mut vec![
            slider(0.0, 2000.0, (100.0, 100.0), (400.0, 100.0)),
            slider(500.0, 2000.0, (100.0, 300.0), (400.0, 300.0)),
        ],
        &vec![0, 0];
        "not intersecting slider bodies"
    )]
    #[case(
        &mut vec![
            slider(0.0, 1000.0, (100.0, 100.0), (300.0, 100.0)),
            slider(400.0, 1000.0, (300.0, 100.0), (500.0, 100.0)),
        ],
        &vec![1, 0];
        "slider end stack where first ends"
    )]
    #[case(
        &mut vec![
            slider(0.0, 1000.0, (100.0, 100.0), (300.0, 100.0)),
            slider(1100.0, 1000.0, (300.0, 100.0), (500.0, 100.0)),
        ],
        &vec![0, 0];
        "slider end not stack where first ends, but too time-wise apart so no stack"
    )]
    fn test_stacking(objects: &mut [StackEntry], expected_stack_count: &[i32]) {
        assert_eq!(stacks(objects), expected_stack_count);
    }

    #[test]
    fn test_stacking_stack_leniency_zero_disables_stacking() {
        let mut objects = vec![
            circle(0.0, (256.0, 192.0)),
            circle(172.0, (256.0, 192.0)),
            circle(344.0, (256.0, 192.0)),
            circle(516.0, (256.0, 192.0)),
            circle(688.0, (256.0, 192.0)),
        ];

        assert_eq!(compute_stack_counts(&mut objects, PREEMPT, 0.0), ());
        assert_eq!(objects.iter().map(|o| o.stack_count).collect::<Vec<_>>(), vec![0, 0, 0, 0, 0]);
    }
}
