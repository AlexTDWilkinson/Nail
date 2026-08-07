//! Xlsx module stdlib registry entries.
//!
//! The same shape as CSV on purpose: rows keyed by the header row.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Xlsx:
        "xlsx_sheets" [Calamine, Tokio] => "std_lib::xlsx::sheets", (path: s) -> ([s]!e),
            "The sheet names in a workbook, in the order the file keeps them.",
            "names:a:s = danger(xlsx_sheets(`report.xlsx`));";
        "xlsx_read" [Calamine, Tokio, DashMap] => "std_lib::xlsx::read", (path: s, sheet: s) -> ([(h s s)]!e),
            "One sheet as rows keyed by its header row, like csv_parse. Every cell arrives as text. int_from and float_from take it from there.",
            "rows:a:h<s,s> = danger(xlsx_read(`report.xlsx`, `Sheet1`));";
        "xlsx_write" [RustXlsxWriter, Tokio, DashMap] => "std_lib::xlsx::write", (path: s, sheet: s, headers: [s], rows: [(h s s)]) -> (v!e),
            "Writes one sheet from headers and rows, like csv_write. Every cell is written as text.",
            "rows:a:h<s,s> = danger(csv_parse(`name,city\\nAda,London`, csv_default_options()));\ndanger(xlsx_write(`export.xlsx`, `people`, [`name`, `city`], rows));";
    }
}
