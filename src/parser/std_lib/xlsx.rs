//! Spreadsheets - the format the data always turns out to be in.
//!
//! The same shape as CSV on purpose: a sheet is read as one hashmap per row,
//! keyed by the header row, and written from headers plus rows. A program that
//! handles CSV handles xlsx by changing the function name, which is the point -
//! the person sending the file chose the format, not the program.
//!
//! Every cell travels as text, the way CSV cells do. Numbers, dates and
//! formula results are read as what the spreadsheet displays; `int_from`,
//! `float_from` and `time_parse` take it from there.

use dashmap::DashMap;

/// The sheet names in a workbook, in the order the file keeps them.
pub async fn sheets(path: String) -> Result<Vec<String>, String> {
    use calamine::Reader;
    let path_for_task = path.clone();
    return tokio::task::spawn_blocking(move || {
        let workbook = calamine::open_workbook_auto(&path_for_task).map_err(|failure| format!("xlsx_sheets: could not open '{}': {}", path_for_task, failure))?;
        return Ok(workbook.sheet_names().to_vec());
    })
    .await
    .map_err(|failure| format!("xlsx_sheets: the reading task failed: {}", failure))?;
}

/// One sheet as rows keyed by its header row, like csv_parse. Cells beyond the
/// header's width are dropped, and missing cells are empty strings.
pub async fn read(path: String, sheet: String) -> Result<Vec<DashMap<String, String>>, String> {
    use calamine::Reader;
    let path_for_task = path.clone();
    return tokio::task::spawn_blocking(move || {
        let mut workbook = calamine::open_workbook_auto(&path_for_task).map_err(|failure| format!("xlsx_read: could not open '{}': {}", path_for_task, failure))?;
        let range = workbook.worksheet_range(&sheet).map_err(|failure| format!("xlsx_read: '{}' has no sheet named '{}': {}", path_for_task, sheet, failure))?;

        let mut rows = range.rows();
        let headers: Vec<String> = match rows.next() {
            Some(header_row) => header_row.iter().map(|cell| cell.to_string()).collect(),
            None => return Ok(Vec::new()),
        };

        let mut out: Vec<DashMap<String, String>> = Vec::new();
        for row in rows {
            let map = DashMap::new();
            for (position, header) in headers.iter().enumerate() {
                if header.is_empty() {
                    continue;
                }
                let value = row.get(position).map(|cell| cell.to_string()).unwrap_or_default();
                map.insert(header.clone(), value);
            }
            out.push(map);
        }
        return Ok(out);
    })
    .await
    .map_err(|failure| format!("xlsx_read: the reading task failed: {}", failure))?;
}

/// Writes one sheet from headers and rows, like csv_write. Every cell is
/// written as text; a spreadsheet that should hold real numbers is better
/// exported as CSV and imported, where the types are the reader's problem.
pub async fn write(path: String, sheet: String, headers: Vec<String>, rows: Vec<DashMap<String, String>>) -> Result<(), String> {
    if headers.is_empty() {
        return Err("xlsx_write: there are no headers, so the columns have no names and no order".to_string());
    }
    return tokio::task::spawn_blocking(move || {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet).map_err(|failure| format!("xlsx_write: `{}` is not a usable sheet name: {}", sheet, failure))?;

        for (column, header) in headers.iter().enumerate() {
            worksheet.write_string(0, column as u16, header).map_err(|failure| format!("xlsx_write: could not write the header row: {}", failure))?;
        }
        for (row_number, row) in rows.iter().enumerate() {
            for (column, header) in headers.iter().enumerate() {
                let value = row.get(header).map(|entry| entry.value().clone()).unwrap_or_default();
                worksheet.write_string(row_number as u32 + 1, column as u16, &value).map_err(|failure| format!("xlsx_write: could not write row {}: {}", row_number + 1, failure))?;
            }
        }
        workbook.save(&path).map_err(|failure| format!("xlsx_write: could not write '{}': {}", path, failure))?;
        return Ok(());
    })
    .await
    .map_err(|failure| format!("xlsx_write: the writing task failed: {}", failure))?;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beside(name: &str) -> String {
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_file(&path);
        return path.to_string_lossy().to_string();
    }

    fn a_row(pairs: &[(&str, &str)]) -> DashMap<String, String> {
        let map = DashMap::new();
        for (key, value) in pairs {
            map.insert(key.to_string(), value.to_string());
        }
        return map;
    }

    #[tokio::test]
    async fn a_sheet_round_trips_through_the_file() {
        let path = beside("nail_xlsx_round_trip.xlsx");
        let headers = vec!["name".to_string(), "city".to_string()];
        let rows = vec![a_row(&[("name", "Alex"), ("city", "Calgary")]), a_row(&[("name", "Sam"), ("city", "Toronto")])];
        write(path.clone(), "people".to_string(), headers, rows).await.expect("a writable workbook");

        assert_eq!(sheets(path.clone()).await.expect("our own file"), vec!["people".to_string()]);
        let read_back = read(path.clone(), "people".to_string()).await.expect("our own sheet");
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].get("name").expect("the name").value(), "Alex");
        assert_eq!(read_back[1].get("city").expect("the city").value(), "Toronto");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_missing_cell_is_an_empty_string() {
        let path = beside("nail_xlsx_sparse.xlsx");
        let headers = vec!["name".to_string(), "note".to_string()];
        let rows = vec![a_row(&[("name", "Alex")])];
        write(path.clone(), "sparse".to_string(), headers, rows).await.expect("a writable workbook");

        let read_back = read(path.clone(), "sparse".to_string()).await.expect("our own sheet");
        assert_eq!(read_back[0].get("note").expect("the empty note").value(), "");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn the_wrong_sheet_name_says_so() {
        let path = beside("nail_xlsx_wrong_sheet.xlsx");
        write(path.clone(), "only".to_string(), vec!["a".to_string()], vec![]).await.expect("a writable workbook");
        let failure = read(path.clone(), "other".to_string()).await.unwrap_err();
        assert!(failure.contains("no sheet named 'other'"), "got: {}", failure);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn something_that_is_not_a_workbook_says_so() {
        let path = beside("nail_xlsx_not_a_workbook.xlsx");
        std::fs::write(&path, "just text").expect("a writable file");
        assert!(sheets(path.clone()).await.is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn writing_with_no_headers_is_refused() {
        assert!(write(beside("nail_xlsx_never.xlsx"), "s".to_string(), vec![], vec![]).await.is_err());
    }
}
