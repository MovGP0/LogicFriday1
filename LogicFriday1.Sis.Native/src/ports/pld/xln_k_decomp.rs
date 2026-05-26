//! Native Rust model for `LogicSynthesis/sis/pld/xln_k_decomp.c`.
//!
//! The C file implements Karp decomposition for SIS `node_t`/`network_t`
//! objects. This port keeps the deterministic decomposition machinery in owned
//! Rust data: ternary cubes, lambda minterm expansion, incompatibility graph
//! construction, compatibility-class unioning, alpha encoding, G-term planning,
//! lambda-index checks, and the C helper semantics. Direct mutation of SIS
//! nodes/networks is deliberately gated by explicit dependency errors.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    pub const fn c_value(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::DontCare => 2,
        }
    }

    pub const fn from_bit(bit: bool) -> Self {
        if bit { Self::One } else { Self::Zero }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    pub literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn len(&self) -> usize {
        self.literals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SopCover {
    pub fanins: Vec<String>,
    pub cubes: Vec<Cube>,
}

impl SopCover {
    pub fn new(fanins: Vec<String>, cubes: Vec<Cube>) -> Result<Self, XlnKDecompError> {
        for cube in &cubes {
            if cube.len() != fanins.len() {
                return Err(XlnKDecompError::CubeArityMismatch {
                    expected: fanins.len(),
                    actual: cube.len(),
                });
            }
        }
        Ok(Self { fanins, cubes })
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlphaFunction {
    pub alpha_index: usize,
    pub one_minterms: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GTerm {
    pub cube_index: usize,
    pub lambda_minterm: usize,
    pub alpha_phases: Vec<Literal>,
    pub u_literals: Vec<(usize, Literal)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KarpDecomposition {
    pub lambda_indices: Vec<usize>,
    pub incompatible: Vec<Vec<bool>>,
    pub classes: Vec<usize>,
    pub class_count: usize,
    pub alpha_count: usize,
    pub alphas: Vec<AlphaFunction>,
    pub g_terms: Vec<GTerm>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecompositionOutcome {
    Decomposed(KarpDecomposition),
    NotDecomposable {
        class_count: usize,
        alpha_count: usize,
        reason: NotDecomposableReason,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotDecomposableReason {
    AlphaCountEqualsSupport,
    AlphaCountExceedsBound,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnKDecompError {
    InvalidSupport {
        support: usize,
    },
    TooManySupportMinterms {
        support: usize,
    },
    InvalidLogInput {
        value: usize,
    },
    InvalidMinterm {
        value: usize,
        length: usize,
    },
    CubeArityMismatch {
        expected: usize,
        actual: usize,
    },
    LambdaIndexOutOfRange {
        index: usize,
        fanin_count: usize,
    },
    ComplementFaninMismatch {
        node_fanins: usize,
        complement_fanins: usize,
    },
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for XlnKDecompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSupport { support } => {
                write!(f, "lambda support must be positive, got {support}")
            }
            Self::TooManySupportMinterms { support } => {
                write!(
                    f,
                    "lambda support {support} does not fit in usize minterm space"
                )
            }
            Self::InvalidLogInput { value } => write!(f, "cannot take ceil log2 of {value}"),
            Self::InvalidMinterm { value, length } => {
                write!(f, "minterm {value} cannot be encoded in {length} bits")
            }
            Self::CubeArityMismatch { expected, actual } => {
                write!(f, "cube has {actual} literals, expected {expected}")
            }
            Self::LambdaIndexOutOfRange { index, fanin_count } => {
                write!(f, "lambda index {index} is outside {fanin_count} fanins")
            }
            Self::ComplementFaninMismatch {
                node_fanins,
                complement_fanins,
            } => write!(
                f,
                "node/complement fanin mismatch: {node_fanins} vs {complement_fanins}"
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnKDecompError {}

pub fn karp_decomp_network_blocked() -> Result<(), XlnKDecompError> {
    missing_native_ports("karp_decomp_network")
}

pub fn xln_k_decomp_node_with_network_blocked() -> Result<(), XlnKDecompError> {
    missing_native_ports("xln_k_decomp_node_with_network")
}

pub fn xln_k_decomp_node_with_array_blocked() -> Result<(), XlnKDecompError> {
    missing_native_ports("xln_k_decomp_node_with_array")
}

fn missing_native_ports(operation: &'static str) -> Result<(), XlnKDecompError> {
    Err(XlnKDecompError::MissingNativePorts { operation })
}

pub fn literal_intersection(left: Literal, right: Literal) -> Option<Literal> {
    match (left, right) {
        (Literal::Zero, Literal::One) | (Literal::One, Literal::Zero) => None,
        (Literal::One, _) | (_, Literal::One) => Some(Literal::One),
        (Literal::Zero, _) | (_, Literal::Zero) => Some(Literal::Zero),
        (Literal::DontCare, Literal::DontCare) => Some(Literal::DontCare),
    }
}

pub fn init_intersection_values() -> [[i8; 3]; 3] {
    [[0, -1, 0], [-1, 1, 1], [0, 1, 2]]
}

pub fn get_lambda_cube(cube: &Cube, lambda_indices: &[usize]) -> Result<Cube, XlnKDecompError> {
    let mut literals = Vec::with_capacity(lambda_indices.len());
    for &index in lambda_indices {
        literals.push(
            *cube
                .literals
                .get(index)
                .ok_or(XlnKDecompError::LambdaIndexOutOfRange {
                    index,
                    fanin_count: cube.len(),
                })?,
        );
    }
    Ok(Cube::new(literals))
}

pub fn generate_all_minterms(cube: &Cube) -> Result<Vec<usize>, XlnKDecompError> {
    if cube.is_empty() {
        return Err(XlnKDecompError::InvalidSupport { support: 0 });
    }
    if cube.len() >= usize::BITS as usize {
        return Err(XlnKDecompError::TooManySupportMinterms {
            support: cube.len(),
        });
    }

    let mut minterms = Vec::new();
    generate_minterm_with_index(&cube.literals, 0, 0, &mut minterms);
    Ok(minterms)
}

fn generate_minterm_with_index(
    literals: &[Literal],
    index: usize,
    value: usize,
    minterms: &mut Vec<usize>,
) {
    if index == literals.len() {
        minterms.push(value);
        return;
    }

    match literals[index] {
        Literal::Zero => generate_minterm_with_index(literals, index + 1, 2 * value, minterms),
        Literal::One => generate_minterm_with_index(literals, index + 1, 2 * value + 1, minterms),
        Literal::DontCare => {
            generate_minterm_with_index(literals, index + 1, 2 * value, minterms);
            generate_minterm_with_index(literals, index + 1, 2 * value + 1, minterms);
        }
    }
}

pub fn make_incompatible(
    left: &Cube,
    right: &Cube,
    support: usize,
) -> Result<Vec<Vec<bool>>, XlnKDecompError> {
    let num_nodes = checked_num_minterms(support)?;
    let mut incompatible = vec![vec![false; num_nodes]; num_nodes];
    mark_incompatible(left, right, &mut incompatible)?;
    Ok(incompatible)
}

fn mark_incompatible(
    left: &Cube,
    right: &Cube,
    incompatible: &mut [Vec<bool>],
) -> Result<(), XlnKDecompError> {
    let left_minterms = generate_all_minterms(left)?;
    let right_minterms = generate_all_minterms(right)?;

    for left_minterm in left_minterms {
        for right_minterm in &right_minterms {
            incompatible[left_minterm][*right_minterm] = true;
            incompatible[*right_minterm][left_minterm] = true;
        }
    }
    Ok(())
}

pub fn form_incompatibility_graph(
    node: &SopCover,
    complement: &SopCover,
    lambda_indices: &[usize],
) -> Result<Vec<Vec<bool>>, XlnKDecompError> {
    validate_lambda_indices(node.fanin_count(), lambda_indices)?;
    if node.fanin_count() != complement.fanin_count() {
        return Err(XlnKDecompError::ComplementFaninMismatch {
            node_fanins: node.fanin_count(),
            complement_fanins: complement.fanin_count(),
        });
    }

    let num_nodes = checked_num_minterms(lambda_indices.len())?;
    let lambda_set: HashSet<usize> = lambda_indices.iter().copied().collect();
    let mut incompatible = vec![vec![false; num_nodes]; num_nodes];

    for cube in &node.cubes {
        for complement_cube in &complement.cubes {
            if u_parts_intersect(cube, complement_cube, &lambda_set)? {
                let lambda_cube = get_lambda_cube(cube, lambda_indices)?;
                let complement_lambda_cube = get_lambda_cube(complement_cube, lambda_indices)?;
                mark_incompatible(&lambda_cube, &complement_lambda_cube, &mut incompatible)?;
            }
        }
    }

    Ok(incompatible)
}

fn u_parts_intersect(
    left: &Cube,
    right: &Cube,
    lambda_set: &HashSet<usize>,
) -> Result<bool, XlnKDecompError> {
    if left.len() != right.len() {
        return Err(XlnKDecompError::CubeArityMismatch {
            expected: left.len(),
            actual: right.len(),
        });
    }

    Ok(left
        .literals
        .iter()
        .zip(&right.literals)
        .enumerate()
        .filter(|(index, _)| !lambda_set.contains(index))
        .all(|(_, (left, right))| literal_intersection(*left, *right).is_some()))
}

pub fn form_compatibility_classes(
    incompatible: &[Vec<bool>],
) -> Result<(Vec<usize>, usize), XlnKDecompError> {
    let num_nodes = incompatible.len();
    let mut forest = UnionFind::new(num_nodes);

    for i in 0..num_nodes {
        if incompatible[i].len() != num_nodes {
            return Err(XlnKDecompError::CubeArityMismatch {
                expected: num_nodes,
                actual: incompatible[i].len(),
            });
        }
        for j in (i + 1)..num_nodes {
            if !incompatible[i][j] {
                forest.union(i, j);
            }
        }
    }

    Ok(forest.assign_class_numbers())
}

pub fn analyze_karp_decomposition(
    node: &SopCover,
    complement: &SopCover,
    lambda_indices: &[usize],
    bound_alphas: Option<usize>,
) -> Result<DecompositionOutcome, XlnKDecompError> {
    let incompatible = form_incompatibility_graph(node, complement, lambda_indices)?;
    let (classes, class_count) = form_compatibility_classes(&incompatible)?;
    let alpha_count = intlog2(class_count)?;

    if alpha_count == lambda_indices.len() {
        return Ok(DecompositionOutcome::NotDecomposable {
            class_count,
            alpha_count,
            reason: NotDecomposableReason::AlphaCountEqualsSupport,
        });
    }
    if bound_alphas.is_some_and(|bound| alpha_count > bound) {
        return Ok(DecompositionOutcome::NotDecomposable {
            class_count,
            alpha_count,
            reason: NotDecomposableReason::AlphaCountExceedsBound,
        });
    }

    Ok(DecompositionOutcome::Decomposed(KarpDecomposition {
        lambda_indices: lambda_indices.to_vec(),
        incompatible,
        alphas: build_alpha_functions(&classes, alpha_count)?,
        g_terms: build_g_terms(node, lambda_indices, &classes, alpha_count)?,
        classes,
        class_count,
        alpha_count,
    }))
}

pub fn build_alpha_functions(
    classes: &[usize],
    alpha_count: usize,
) -> Result<Vec<AlphaFunction>, XlnKDecompError> {
    let mut alphas = (0..alpha_count)
        .map(|alpha_index| AlphaFunction {
            alpha_index,
            one_minterms: Vec::new(),
        })
        .collect::<Vec<_>>();

    for (minterm, class_num) in classes.iter().copied().enumerate() {
        let encoded = xl_binary1(class_num, alpha_count)?;
        for (alpha_index, bit) in encoded.iter().copied().enumerate() {
            if bit {
                alphas[alpha_index].one_minterms.push(minterm);
            }
        }
    }

    Ok(alphas)
}

pub fn build_g_terms(
    node: &SopCover,
    lambda_indices: &[usize],
    classes: &[usize],
    alpha_count: usize,
) -> Result<Vec<GTerm>, XlnKDecompError> {
    validate_lambda_indices(node.fanin_count(), lambda_indices)?;
    let lambda_set: HashSet<usize> = lambda_indices.iter().copied().collect();
    let mut terms = Vec::new();

    for (cube_index, cube) in node.cubes.iter().enumerate() {
        let lambda_cube = get_lambda_cube(cube, lambda_indices)?;
        let lambda_minterms = generate_all_minterms(&lambda_cube)?;
        let u_literals = cube
            .literals
            .iter()
            .copied()
            .enumerate()
            .filter(|(index, literal)| !lambda_set.contains(index) && *literal != Literal::DontCare)
            .collect::<Vec<_>>();

        for lambda_minterm in lambda_minterms {
            let class_num = classes[lambda_minterm];
            let alpha_phases = xl_binary1(class_num, alpha_count)?
                .into_iter()
                .map(Literal::from_bit)
                .collect();
            terms.push(GTerm {
                cube_index,
                lambda_minterm,
                alpha_phases,
                u_literals: u_literals.clone(),
            });
        }
    }

    Ok(terms)
}

pub fn get_combination(num_fanin: usize, comb_num: usize, support: usize) -> Vec<usize> {
    (0..support)
        .map(|index| (comb_num + index) % num_fanin)
        .collect()
}

pub fn xln_checking_lambda_indices_create(
    node: &SopCover,
    lambda_indices: &[usize],
) -> Result<Vec<String>, XlnKDecompError> {
    validate_lambda_indices(node.fanin_count(), lambda_indices)?;
    Ok(lambda_indices
        .iter()
        .map(|index| node.fanins[*index].clone())
        .collect())
}

pub fn xln_checking_lambda_indices(
    node: &SopCover,
    lambda_indices: &[usize],
    check_vec: &[String],
) -> Result<bool, XlnKDecompError> {
    validate_lambda_indices(node.fanin_count(), lambda_indices)?;
    Ok(check_vec.iter().enumerate().all(|(index, fanin)| {
        node.fanins.iter().position(|candidate| candidate == fanin) == Some(lambda_indices[index])
    }))
}

pub fn xln_modify_lambda_indices(
    original_fanins: &[String],
    simplified_fanins: &[String],
    lambda_indices: &[usize],
) -> Result<Vec<usize>, XlnKDecompError> {
    validate_lambda_indices(original_fanins.len(), lambda_indices)?;
    Ok(lambda_indices
        .iter()
        .copied()
        .filter(|index| simplified_fanins.contains(&original_fanins[*index]))
        .collect())
}

pub fn intlog2(value: usize) -> Result<usize, XlnKDecompError> {
    if value == 0 {
        return Err(XlnKDecompError::InvalidLogInput { value });
    }

    let floor = usize::BITS as usize - 1 - value.leading_zeros() as usize;
    let floor_power = 1usize << floor;
    Ok(if value > floor_power {
        floor + 1
    } else {
        floor
    })
}

pub fn xl_binary1(value: usize, length: usize) -> Result<Vec<bool>, XlnKDecompError> {
    if length < usize::BITS as usize && value >= (1usize << length) {
        return Err(XlnKDecompError::InvalidMinterm { value, length });
    }

    Ok((0..length)
        .rev()
        .map(|bit| ((value >> bit) & 1) == 1)
        .collect())
}

pub fn checked_num_minterms(support: usize) -> Result<usize, XlnKDecompError> {
    if support == 0 {
        return Err(XlnKDecompError::InvalidSupport { support });
    }
    1usize
        .checked_shl(support as u32)
        .ok_or(XlnKDecompError::TooManySupportMinterms { support })
}

fn validate_lambda_indices(
    fanin_count: usize,
    lambda_indices: &[usize],
) -> Result<(), XlnKDecompError> {
    if lambda_indices.is_empty() {
        return Err(XlnKDecompError::InvalidSupport { support: 0 });
    }
    for &index in lambda_indices {
        if index >= fanin_count {
            return Err(XlnKDecompError::LambdaIndexOutOfRange { index, fanin_count });
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UnionFind {
    parent: Vec<usize>,
    num_child: Vec<usize>,
}

impl UnionFind {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
            num_child: vec![0; count],
        }
    }

    fn find(&mut self, index: usize) -> usize {
        let parent = self.parent[index];
        if parent == index {
            index
        } else {
            let root = self.find(parent);
            self.parent[index] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }

        if self.num_child[left_root] < self.num_child[right_root] {
            self.parent[left_root] = right_root;
            self.num_child[right_root] += self.num_child[left_root];
        } else {
            self.parent[right_root] = left_root;
            self.num_child[left_root] += self.num_child[right_root];
        }
    }

    fn assign_class_numbers(&mut self) -> (Vec<usize>, usize) {
        let mut roots = Vec::<(usize, usize)>::new();
        let mut classes = Vec::with_capacity(self.parent.len());

        for index in 0..self.parent.len() {
            let root = self.find(index);
            let class_num = match roots.iter().find(|(known_root, _)| *known_root == root) {
                Some((_, class_num)) => *class_num,
                None => {
                    let class_num = roots.len();
                    roots.push((root, class_num));
                    class_num
                }
            };
            classes.push(class_num);
        }

        (classes, roots.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(literals: &[Literal]) -> Cube {
        Cube::new(literals.to_vec())
    }

    fn cover(cubes: Vec<Cube>) -> SopCover {
        SopCover::new(vec!["a".to_owned(), "b".to_owned(), "u".to_owned()], cubes).unwrap()
    }

    #[test]
    fn literal_values_and_intersection_match_c_table() {
        assert_eq!(Literal::Zero.c_value(), 0);
        assert_eq!(Literal::One.c_value(), 1);
        assert_eq!(Literal::DontCare.c_value(), 2);
        assert_eq!(
            init_intersection_values(),
            [[0, -1, 0], [-1, 1, 1], [0, 1, 2]]
        );
        assert_eq!(literal_intersection(Literal::Zero, Literal::One), None);
        assert_eq!(
            literal_intersection(Literal::DontCare, Literal::One),
            Some(Literal::One)
        );
    }

    #[test]
    fn minterm_generation_preserves_c_binary_order() {
        assert_eq!(
            generate_all_minterms(&cube(&[Literal::DontCare, Literal::Zero])).unwrap(),
            vec![0, 2]
        );
        assert_eq!(
            generate_all_minterms(&cube(&[Literal::One, Literal::DontCare, Literal::Zero]))
                .unwrap(),
            vec![4, 6]
        );
    }

    #[test]
    fn make_incompatible_marks_cross_product_symmetrically() {
        let matrix = make_incompatible(
            &cube(&[Literal::DontCare, Literal::Zero]),
            &cube(&[Literal::One, Literal::DontCare]),
            2,
        )
        .unwrap();

        assert!(matrix[0][2]);
        assert!(matrix[0][3]);
        assert!(matrix[2][0]);
        assert!(matrix[2][2]);
        assert!(!matrix[1][3]);
    }

    #[test]
    fn incompatibility_graph_ignores_lambda_fanins_and_filters_empty_u_intersections() {
        let node = cover(vec![
            cube(&[Literal::Zero, Literal::DontCare, Literal::One]),
            cube(&[Literal::One, Literal::DontCare, Literal::Zero]),
        ]);
        let complement = cover(vec![
            cube(&[Literal::One, Literal::DontCare, Literal::One]),
            cube(&[Literal::Zero, Literal::One, Literal::One]),
        ]);

        let matrix = form_incompatibility_graph(&node, &complement, &[0, 1]).unwrap();

        assert!(matrix[0][2]);
        assert!(matrix[0][3]);
        assert!(matrix[2][0]);
        assert!(!matrix[2][3]);
    }

    #[test]
    fn compatibility_classes_merge_all_non_incompatible_pairs() {
        let incompatible = vec![
            vec![false, false, true, true],
            vec![false, false, true, true],
            vec![true, true, false, false],
            vec![true, true, false, false],
        ];

        let (classes, class_count) = form_compatibility_classes(&incompatible).unwrap();

        assert_eq!(class_count, 2);
        assert_eq!(classes, vec![0, 0, 1, 1]);
    }

    #[test]
    fn intlog2_and_binary_encoding_match_xln_aux_helpers() {
        assert_eq!(intlog2(1), Ok(0));
        assert_eq!(intlog2(2), Ok(1));
        assert_eq!(intlog2(3), Ok(2));
        assert_eq!(intlog2(4), Ok(2));
        assert_eq!(intlog2(5), Ok(3));
        assert_eq!(xl_binary1(3, 4), Ok(vec![false, false, true, true]));
        assert_eq!(
            xl_binary1(8, 3),
            Err(XlnKDecompError::InvalidMinterm {
                value: 8,
                length: 3
            })
        );
    }

    #[test]
    fn karp_analysis_returns_alpha_functions_and_g_terms() {
        let node = cover(vec![cube(&[
            Literal::Zero,
            Literal::DontCare,
            Literal::One,
        ])]);
        let complement = cover(vec![cube(&[Literal::One, Literal::DontCare, Literal::One])]);

        let outcome = analyze_karp_decomposition(&node, &complement, &[0, 1], Some(2)).unwrap();

        let DecompositionOutcome::Decomposed(decomposition) = outcome else {
            panic!("expected decomposed outcome");
        };
        assert_eq!(decomposition.class_count, 2);
        assert_eq!(decomposition.alpha_count, 1);
        assert_eq!(
            decomposition.alphas,
            vec![AlphaFunction {
                alpha_index: 0,
                one_minterms: vec![2, 3],
            }]
        );
        assert_eq!(decomposition.g_terms.len(), 2);
        assert!(decomposition.g_terms.iter().any(|term| {
            term.cube_index == 0
                && term.lambda_minterm == 0
                && term.alpha_phases == vec![Literal::Zero]
                && term.u_literals == vec![(2, Literal::One)]
        }));
    }

    #[test]
    fn karp_analysis_reports_c_not_decomposable_conditions() {
        let node = cover(vec![cube(&[
            Literal::Zero,
            Literal::DontCare,
            Literal::DontCare,
        ])]);
        let complement = cover(vec![cube(&[
            Literal::One,
            Literal::DontCare,
            Literal::DontCare,
        ])]);

        assert_eq!(
            analyze_karp_decomposition(&node, &complement, &[0, 1, 2], Some(0)).unwrap(),
            DecompositionOutcome::NotDecomposable {
                class_count: 2,
                alpha_count: 1,
                reason: NotDecomposableReason::AlphaCountExceedsBound,
            }
        );

        let incompatible = vec![
            vec![false, true, true, true],
            vec![true, false, true, true],
            vec![true, true, false, true],
            vec![true, true, true, false],
        ];
        let (classes, class_count) = form_compatibility_classes(&incompatible).unwrap();
        assert_eq!(classes, vec![0, 1, 2, 3]);
        assert_eq!(intlog2(class_count), Ok(2));
    }

    #[test]
    fn lambda_helpers_follow_c_index_behavior() {
        let original = SopCover::new(
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            vec![cube(&[Literal::One, Literal::Zero, Literal::DontCare])],
        )
        .unwrap();
        let simplified_fanins = vec!["a".to_owned(), "c".to_owned()];
        let check_vec = xln_checking_lambda_indices_create(&original, &[0, 2]).unwrap();

        assert_eq!(check_vec, vec!["a", "c"]);
        assert!(xln_checking_lambda_indices(&original, &[0, 2], &check_vec).unwrap());
        assert_eq!(
            xln_modify_lambda_indices(&original.fanins, &simplified_fanins, &[0, 1, 2]).unwrap(),
            vec![0, 2]
        );
        assert_eq!(get_combination(5, 3, 4), vec![3, 4, 0, 1]);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_k_decomp.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
