//! Column layout for the short format (`-C`, `-x`, `-m`).
//!
//! Pure functions over cell widths so the arithmetic can be tested without a
//! terminal. A "cell" is everything printed for one entry (inode, name,
//! indicator), measured in display columns.

/// Gap between columns
const GAP: usize = 2;

/// Result of fitting cells into columns
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub rows: usize,
    /// Width of each used column, without the gap
    pub col_widths: Vec<usize>,
    across: bool,
    count: usize,
}

impl Plan {
    /// Index of the cell at (row, col), if there is one
    pub fn index(&self, row: usize, col: usize) -> Option<usize> {
        if row >= self.rows || col >= self.col_widths.len() {
            return None;
        }
        let i = if self.across {
            row * self.col_widths.len() + col
        } else {
            col * self.rows + row
        };
        (i < self.count).then_some(i)
    }

    pub fn cols(&self) -> usize {
        self.col_widths.len()
    }
}

/// Fits `widths` into as many columns as `line` allows (0 means unlimited).
/// `across` fills rows first (`-x`); otherwise columns first (`-C`).
pub fn plan(widths: &[usize], line: usize, across: bool) -> Plan {
    let n = widths.len();
    if n == 0 {
        return Plan {
            rows: 0,
            col_widths: Vec::new(),
            across,
            count: 0,
        };
    }
    // Unlimited width: everything on one row
    if line == 0 {
        return build(widths, n, across);
    }
    // Every column but the last needs at least 1 char plus the gap
    let max_cols = ((line + GAP) / (GAP + 1)).clamp(1, n);
    for cols in (1..=max_cols).rev() {
        let p = build(widths, cols, across);
        if p.total() <= line {
            return p;
        }
    }
    build(widths, 1, across)
}

fn build(widths: &[usize], cols: usize, across: bool) -> Plan {
    let n = widths.len();
    let rows = n.div_ceil(cols);
    // Column-major layouts can need fewer columns than requested
    let used = if across {
        cols.min(n)
    } else {
        n.div_ceil(rows)
    };
    let mut col_widths = vec![0; used];
    for (i, &w) in widths.iter().enumerate() {
        let c = if across { i % used } else { i / rows };
        col_widths[c] = col_widths[c].max(w);
    }
    Plan {
        rows,
        col_widths,
        across,
        count: n,
    }
}

impl Plan {
    fn total(&self) -> usize {
        self.col_widths.iter().sum::<usize>() + GAP * self.col_widths.len().saturating_sub(1)
    }
}

/// Whether entry `i` (width `w`) goes on the current line of a comma list
/// whose line currently holds `pos` columns. GNU ls keeps a strict margin.
pub fn comma_fits(pos: usize, w: usize, line: usize) -> bool {
    line == 0 || pos + w + GAP < line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(p: &Plan, n: usize) -> Vec<Vec<Option<usize>>> {
        let _ = n;
        (0..p.rows)
            .map(|r| (0..p.cols()).map(|c| p.index(r, c)).collect())
            .collect()
    }

    #[test]
    fn test_empty() {
        let p = plan(&[], 80, false);
        assert_eq!((p.rows, p.cols()), (0, 0));
    }

    #[test]
    fn test_all_fit_in_one_row() {
        // 3 cells of width 3 + 2 gaps of 2 = 13
        let p = plan(&[3, 3, 3], 80, false);
        assert_eq!((p.rows, p.cols()), (1, 3));
    }

    #[test]
    fn test_column_major_order() {
        let p = plan(&[1; 5], 5, false); // 1+2+1 = 4 fits 2 cols; 3 cols = 7 does not
        assert_eq!(p.cols(), 2);
        assert_eq!(
            grid(&p, 5),
            vec![
                vec![Some(0), Some(3)],
                vec![Some(1), Some(4)],
                vec![Some(2), None]
            ]
        );
    }

    #[test]
    fn test_across_order() {
        let p = plan(&[1; 5], 5, true);
        assert_eq!(p.cols(), 2);
        assert_eq!(
            grid(&p, 5),
            vec![
                vec![Some(0), Some(1)],
                vec![Some(2), Some(3)],
                vec![Some(4), None]
            ]
        );
    }

    #[test]
    fn test_column_widths_are_per_column() {
        // column-major, 2 cols: [10, 1, 1] [1, 1, 1]
        let p = plan(&[10, 1, 1, 1, 1, 1], 14, false);
        assert_eq!(p.col_widths, vec![10, 1]);
        assert_eq!(p.rows, 3);
    }

    #[test]
    fn test_too_wide_falls_back_to_single_column() {
        let p = plan(&[50, 50], 40, false);
        assert_eq!((p.rows, p.cols()), (2, 1));
    }

    #[test]
    fn test_unlimited_width_is_one_row() {
        let p = plan(&[5; 100], 0, false);
        assert_eq!((p.rows, p.cols()), (1, 100));
    }

    #[test]
    fn test_column_major_never_reports_empty_columns() {
        // 5 cells in 4 requested columns needs rows=2, so only 3 columns are used
        let p = build(&[1; 5], 4, false);
        assert_eq!((p.rows, p.cols()), (2, 3));
        assert_eq!(p.index(1, 2), None);
    }

    #[test]
    fn test_index_out_of_range_is_none() {
        let p = plan(&[1; 5], 12, true); // 4 columns, 2 rows
        assert_eq!(p.cols(), 4);
        assert_eq!(p.index(0, 3), Some(3));
        assert_eq!(p.index(0, 4), None); // would wrap into the next row without the bounds check
        assert_eq!(p.index(2, 0), None);
    }

    #[test]
    fn test_comma_fits() {
        assert!(comma_fits(0, 5, 80));
        assert!(!comma_fits(70, 8, 80));
        assert!(comma_fits(70, 100, 0));
    }
}
