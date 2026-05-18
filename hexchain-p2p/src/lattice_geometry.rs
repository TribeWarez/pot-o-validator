use serde::{Deserialize, Serialize};

use crate::types::NEIGHBOR_SLOTS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HCPCoord {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

fn planar_offsets() -> [HCPCoord; 6] {
    [
        HCPCoord { q: 1, r: 0, s: 0 },
        HCPCoord { q: 1, r: -1, s: 0 },
        HCPCoord { q: 0, r: -1, s: 0 },
        HCPCoord { q: -1, r: 0, s: 0 },
        HCPCoord { q: -1, r: 1, s: 0 },
        HCPCoord { q: 0, r: 1, s: 0 },
    ]
}

fn upper_offsets(a_layer: bool) -> [HCPCoord; 3] {
    if a_layer {
        [
            HCPCoord { q: 0, r: 0, s: 1 },
            HCPCoord { q: 1, r: -1, s: 1 },
            HCPCoord { q: -1, r: 0, s: 1 },
        ]
    } else {
        [
            HCPCoord { q: 0, r: 0, s: 1 },
            HCPCoord { q: -1, r: 1, s: 1 },
            HCPCoord { q: 0, r: 1, s: 1 },
        ]
    }
}

fn lower_offsets(a_layer: bool) -> [HCPCoord; 3] {
    if a_layer {
        [
            HCPCoord { q: 0, r: 0, s: -1 },
            HCPCoord { q: 1, r: -1, s: -1 },
            HCPCoord { q: -1, r: 0, s: -1 },
        ]
    } else {
        [
            HCPCoord { q: 0, r: 0, s: -1 },
            HCPCoord { q: -1, r: 1, s: -1 },
            HCPCoord { q: 0, r: -1, s: -1 },
        ]
    }
}

pub fn get_neighbors(c: HCPCoord) -> [HCPCoord; NEIGHBOR_SLOTS] {
    let a_layer = (c.s & 1) == 0;
    let mut out: [HCPCoord; NEIGHBOR_SLOTS] = [HCPCoord { q: 0, r: 0, s: 0 }; NEIGHBOR_SLOTS];
    let mut i = 0;

    for d in planar_offsets() {
        out[i] = HCPCoord {
            q: c.q + d.q,
            r: c.r + d.r,
            s: c.s + d.s,
        };
        i += 1;
    }
    for d in upper_offsets(a_layer) {
        out[i] = HCPCoord {
            q: c.q + d.q,
            r: c.r + d.r,
            s: c.s + d.s,
        };
        i += 1;
    }
    for d in lower_offsets(a_layer) {
        out[i] = HCPCoord {
            q: c.q + d.q,
            r: c.r + d.r,
            s: c.s + d.s,
        };
        i += 1;
    }

    out
}

pub fn neighbor_slot_offset(slot_index: usize) -> HCPCoord {
    get_neighbors(HCPCoord { q: 0, r: 0, s: 0 })[slot_index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_has_12_neighbors() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        assert_eq!(n.len(), 12);
    }

    #[test]
    fn test_all_neighbors_distinct() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        for i in 0..12 {
            for j in (i + 1)..12 {
                assert_ne!(n[i], n[j], "neighbors {} and {} are identical", i, j);
            }
        }
    }

    #[test]
    fn test_six_planar_have_s_zero() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        for i in 0..6 {
            assert_eq!(n[i].s, 0, "planar neighbor {} has nonzero s", i);
        }
    }

    #[test]
    fn test_upper_have_s_positive() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        for i in 6..9 {
            assert_eq!(n[i].s, 1, "upper neighbor {} has s != 1", i);
        }
    }

    #[test]
    fn test_lower_have_s_negative() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        for i in 9..12 {
            assert_eq!(n[i].s, -1, "lower neighbor {} has s != -1", i);
        }
    }

    #[test]
    fn test_b_layer_upper_differs_from_a() {
        let a_n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        let b_n = get_neighbors(HCPCoord { q: 0, r: 0, s: 1 });
        assert_ne!(
            a_n[7..9],
            b_n[7..9],
            "B-layer upper should differ from A-layer"
        );
    }

    #[test]
    fn test_neighbor_slot_offset_consistency() {
        for i in 0..12 {
            assert_eq!(
                neighbor_slot_offset(i),
                get_neighbors(HCPCoord { q: 0, r: 0, s: 0 })[i]
            );
        }
    }

    #[test]
    fn test_known_a_layer_upper() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        assert_eq!(n[6], HCPCoord { q: 0, r: 0, s: 1 });
        assert_eq!(n[7], HCPCoord { q: 1, r: -1, s: 1 });
        assert_eq!(n[8], HCPCoord { q: -1, r: 0, s: 1 });
    }

    #[test]
    fn test_known_a_layer_lower() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        assert_eq!(n[9], HCPCoord { q: 0, r: 0, s: -1 });
        assert_eq!(n[10], HCPCoord { q: 1, r: -1, s: -1 });
        assert_eq!(n[11], HCPCoord { q: -1, r: 0, s: -1 });
    }

    #[test]
    fn test_known_b_layer_upper() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 1 });
        assert_eq!(n[6], HCPCoord { q: 0, r: 0, s: 2 });
        assert_eq!(n[7], HCPCoord { q: -1, r: 1, s: 2 });
        assert_eq!(n[8], HCPCoord { q: 0, r: 1, s: 2 });
    }

    #[test]
    fn test_known_b_layer_lower() {
        let n = get_neighbors(HCPCoord { q: 0, r: 0, s: 1 });
        assert_eq!(n[9], HCPCoord { q: 0, r: 0, s: 0 });
        assert_eq!(n[10], HCPCoord { q: -1, r: 1, s: 0 });
        assert_eq!(n[11], HCPCoord { q: 0, r: -1, s: 0 });
    }

    #[test]
    fn test_nonzero_qr_all_planes() {
        let n = get_neighbors(HCPCoord { q: 7, r: -3, s: 4 });
        assert_eq!(n.len(), 12);
        let origin = HCPCoord { q: 7, r: -3, s: 4 };
        for neighbor in &n {
            let dist_q = (neighbor.q - origin.q).abs();
            let dist_r = (neighbor.r - origin.r).abs();
            let dist_s = (neighbor.s - origin.s).abs();
            assert!(
                dist_q <= 1 && dist_r <= 1 && dist_s <= 1,
                "neighbor {:?} too far from origin {:?}",
                neighbor,
                origin
            );
        }
    }

    #[test]
    fn test_vertical_mutual_symmetry() {
        let origin = HCPCoord { q: 0, r: 0, s: 0 };
        let upper = get_neighbors(origin)[6]; // (0,0,1)
        let nn = get_neighbors(upper);
        let found = nn.iter().any(|&x| x == origin);
        assert!(found, "(0,0,1) should include (0,0,0) via lower slot");
    }

    #[test]
    fn test_inplane_mutual_symmetry() {
        let origin = HCPCoord { q: 3, r: -1, s: 2 };
        let planar_nb = get_neighbors(origin)[0]; // (4,-1,2)
        let nn = get_neighbors(planar_nb);
        let found = nn.iter().any(|&x| x == origin);
        assert!(
            found,
            "({},{},{}) should include ({},{},{})",
            planar_nb.q, planar_nb.r, planar_nb.s, origin.q, origin.r, origin.s
        );
    }
}
