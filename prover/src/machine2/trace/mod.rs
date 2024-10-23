use num_traits::Zero;
use stwo_prover::core::{
    backend::simd::{column::BaseColumn, m31::LOG_N_LANES, SimdBackend},
    fields::m31::BaseField,
    poly::{
        circle::{CanonicCoset, CircleEvaluation},
        BitReversedOrder,
    },
    ColumnVec,
};

use super::column::Column;

pub mod eval;
pub mod utils;

use utils::{bit_reverse, coset_order_to_circle_domain_order};

use nexus_vm::cpu::RegisterFile;

// Program execution step.
pub struct Step {
    /// Machine registers.
    pub(crate) regs: RegisterFile,
    /// Program step.
    pub(crate) step: nexus_vm::trace::Step,
}

pub struct Traces {
    cols: Vec<Vec<BaseField>>,
    log_size: u32,
}

impl Traces {
    /// Returns [`Column::TOTAL_COLUMNS_NUM`] zeroed columns, each one `2.pow(log_size)` in length.
    pub(crate) fn new(log_size: u32) -> Self {
        assert!(log_size >= LOG_N_LANES);
        Self {
            cols: vec![vec![BaseField::zero(); 1 << log_size]; Column::COLUMNS_NUM],
            log_size,
        }
    }

    /// Returns inner representation of columns.
    pub fn into_inner(self) -> Vec<Vec<BaseField>> {
        self.cols
    }

    /// Returns a copy of `N` raw columns in range `[offset..offset + N]` at `row`, where
    /// `N` is assumed to be equal `Column::size` of a `col`.
    #[doc(hidden)]
    pub fn column<const N: usize>(&self, row: usize, col: Column) -> [BaseField; N] {
        assert_eq!(col.size(), N, "column size mismatch");

        let offset = col.offset();
        let mut iter = self.cols[offset..].iter();
        std::array::from_fn(|_idx| iter.next().expect("invalid offset; must be unreachable")[row])
    }

    /// Returns mutable reference to `N` raw columns in range `[offset..offset + N]` at `row`,
    /// where `N` is assumed to be equal `Column::size` of a `col`.
    #[doc(hidden)]
    pub fn column_mut<const N: usize>(&mut self, row: usize, col: Column) -> [&mut BaseField; N] {
        assert_eq!(col.size(), N, "column size mismatch");

        let offset = col.offset();
        let mut iter = self.cols[offset..].iter_mut();
        std::array::from_fn(|_idx| {
            &mut iter.next().expect("invalid offset; must be unreachable")[row]
        })
    }

    /// Converts traces into circle domain evaluations, bit-reversing row indices
    /// according to circle domain ordering.
    pub fn into_circle_evaluation(
        self,
    ) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(self.log_size).circle_domain();
        self.cols
            .into_iter()
            .map(|col| {
                let mut eval = coset_order_to_circle_domain_order(col.as_slice());
                bit_reverse(&mut eval);

                let col = BaseColumn::from_iter(eval);
                CircleEvaluation::<SimdBackend, _, BitReversedOrder>::new(domain, col)
            })
            .collect()
    }
}

/// Returns a copy of `column` values as an array.
///
/// ```ignore
/// let mut traces = Traces::new(6);
/// let row = 0usize;
/// let mut add_row: [BaseField; 1] = trace_column!(traces, row, Column::IsAdd);
///
/// dbg!(add_row[0].is_one());
/// ```
macro_rules! trace_column {
    ($traces:expr, $row:expr, $col:expr) => {{
        $traces.column::<{ Column::size($col) }>($row, $col)
    }};
}

/// Returns a mutable reference to `column` values as an array.
///
/// ```ignore
/// let mut traces = Traces::new(6);
/// let row = 0usize;
/// let mut add_row: [&mut BaseField; 1] = trace_column_mut!(traces, row, Column::IsAdd);
///
/// *add_row[0] = BaseField::one();
/// ```
macro_rules! trace_column_mut {
    ($traces:expr, $row:expr, $col:expr) => {{
        $traces.column_mut::<{ Column::size($col) }>($row, $col)
    }};
}

pub(crate) use trace_column;
pub(crate) use trace_column_mut;
