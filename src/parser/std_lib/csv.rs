use dashmap::DashMap;
use lazy_static::lazy_static;
use std::io::BufRead;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Which whitespace the reader trims.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CSV_Trim {
    None,
    Headers,
    Fields,
    All,
}

/// How to read a CSV document. An empty string means "not set" for the
/// character options, and 0 means "no limit" for the counts.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CSV_Options {
    /// Field separator, default `,`. Use a tab for TSV.
    pub delimiter: String,
    /// Quote character, default `"`.
    pub quote: String,
    /// Escape character. Unset means quotes are doubled ("") instead.
    pub escape: String,
    /// Whether "" inside a quoted field means one quote. Default true.
    pub double_quote: bool,
    /// Lines starting with this character are skipped.
    pub comment: String,
    /// Whether the first row names the columns. When false, keys are column
    /// numbers from 0.
    pub has_headers: bool,
    /// Whether rows may have a different field count than the header.
    pub flexible: bool,
    /// Which whitespace to trim.
    pub trim: CSV_Trim,
    /// Lines dropped before the header, for exports with a title banner.
    pub skip_rows: i64,
    /// Stop after this many data rows. 0 reads all of them.
    pub n_rows: i64,
    /// Field texts to read as empty, e.g. NA or NULL.
    pub null_values: Vec<String>,
    /// Whether a malformed row is skipped rather than failing the parse.
    pub ignore_errors: bool,
    /// Line terminator. Unset accepts any of CR, LF and CRLF.
    pub eol_char: String,
}

/// The defaults, so a caller who wants ordinary comma-separated text with a
/// header row does not have to spell out every field.
pub fn default_options() -> CSV_Options {
    CSV_Options {
        delimiter: ",".to_string(),
        quote: "\"".to_string(),
        escape: String::new(),
        double_quote: true,
        comment: String::new(),
        has_headers: true,
        // Ragged rows are kept by default: a sheet export often has short rows,
        // and dropping the whole parse over one is rarely what the caller wants.
        flexible: true,
        trim: CSV_Trim::None,
        skip_rows: 0,
        n_rows: 0,
        null_values: Vec::new(),
        ignore_errors: false,
        eol_char: String::new(),
    }
}

/// Reads a single-character option, since the csv reader configures its
/// delimiter, quote and escape as bytes rather than strings.
fn single_byte(value: &str, name: &str) -> Result<Option<u8>, String> {
    let mut bytes = value.bytes();
    match (bytes.next(), bytes.next()) {
        (Some(byte), None) => Ok(Some(byte)),
        (None, _) => Ok(None),
        _ => Err(format!("csv_parse: '{}' must be a single character, but it is '{}'", name, value)),
    }
}

fn row_count(value: i64, name: &str) -> Result<usize, String> {
    match usize::try_from(value) {
        Ok(count) => Ok(count),
        Err(_) => Err(format!("csv_parse: '{}' cannot be negative, but it is {}", name, value)),
    }
}

/// The reader configuration shared by csv_parse and csv_open, so both honour
/// the same options in the same way.
fn build_reader(options: &CSV_Options) -> Result<csv::ReaderBuilder, String> {
    let mut builder = csv::ReaderBuilder::new();

    if let Some(delimiter) = single_byte(&options.delimiter, "delimiter")? {
        builder.delimiter(delimiter);
    }
    if let Some(quote) = single_byte(&options.quote, "quote")? {
        builder.quote(quote);
    }
    if let Some(escape) = single_byte(&options.escape, "escape")? {
        builder.escape(Some(escape));
    }
    if let Some(comment) = single_byte(&options.comment, "comment")? {
        builder.comment(Some(comment));
    }
    if let Some(terminator) = single_byte(&options.eol_char, "eol_char")? {
        builder.terminator(csv::Terminator::Any(terminator));
    }

    builder.has_headers(options.has_headers);
    builder.double_quote(options.double_quote);
    builder.flexible(options.flexible);
    builder.trim(match options.trim {
        CSV_Trim::None => csv::Trim::None,
        CSV_Trim::Headers => csv::Trim::Headers,
        CSV_Trim::Fields => csv::Trim::Fields,
        CSV_Trim::All => csv::Trim::All,
    });

    Ok(builder)
}

/// Parses CSV text into one hashmap per row, keyed by the header row.
///
/// Quote-aware, so a field containing the delimiter or a newline survives
/// intact - splitting on the delimiter by hand corrupts every column after
/// such a field.
pub fn parse(text: String, options: CSV_Options) -> Result<Vec<DashMap<String, String>>, String> {
    // Dropped before the reader sees the text, because the header itself may be
    // preceded by title rows that would otherwise be taken as the header.
    let skip_rows = row_count(options.skip_rows, "skip_rows")?;
    let text = if skip_rows > 0 { text.split_inclusive('\n').skip(skip_rows).collect::<String>() } else { text };

    let row_limit = row_count(options.n_rows, "n_rows")?;

    let mut reader = build_reader(&options)?.from_reader(text.as_bytes());

    // Without a header row the columns are addressed by number, so that a
    // headerless file is still readable through the same hashmap shape.
    let keys: Vec<String> = if options.has_headers {
        reader.headers().map_err(|e| format!("csv_parse: could not read the header row: {}", e))?.iter().map(|header| header.to_string()).collect()
    } else {
        Vec::new()
    };

    let mut rows = Vec::new();
    for result in reader.records() {
        if row_limit > 0 && rows.len() >= row_limit {
            break;
        }

        let record = match result {
            Ok(record) => record,
            Err(_) if options.ignore_errors => continue,
            Err(e) => return Err(format!("csv_parse: could not read a row: {}", e)),
        };

        let row = DashMap::new();
        for (index, field) in record.iter().enumerate() {
            let key = match keys.get(index) {
                Some(header) => header.clone(),
                None => index.to_string(),
            };
            // A configured null text becomes the empty string, so callers test
            // for absence one way instead of per-export sentinel values.
            let value = if options.null_values.iter().any(|null_value| null_value == field) { String::new() } else { field.to_string() };
            row.insert(key, value);
        }
        rows.push(row);
    }

    Ok(rows)
}

/// A quote-aware reader over CSV text with the plain comma defaults, plus the
/// header row it starts with. Shared by the text-level helpers that take no
/// options, so they all read quoting the same way csv_parse does.
fn open_text<'text>(text: &'text str, function: &str) -> Result<(csv::Reader<&'text [u8]>, csv::StringRecord), String> {
    if text.trim().is_empty() {
        return Err(format!("{}: the text is empty, so there is no header row", function));
    }
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(text.as_bytes());
    let header_row = reader.headers().map_err(|e| format!("{}: could not read the header row: {}", function, e))?.clone();
    return Ok((reader, header_row));
}

/// The position of a named column, or an error naming the missing column and
/// listing the ones the text has.
fn header_index(header_row: &csv::StringRecord, wanted: &str, function: &str) -> Result<usize, String> {
    if let Some(index) = header_row.iter().position(|field| field == wanted) {
        return Ok(index);
    }
    let known: Vec<String> = header_row.iter().map(|field| format!("'{}'", field)).collect();
    return Err(format!("{}: there is no column named '{}', the columns are {}", function, wanted, known.join(", ")));
}

/// The first row's fields, which name the columns. Read by the same
/// quote-aware reader as csv_parse, so a header holding a comma inside quotes
/// stays one field.
pub fn headers(text: String) -> Result<Vec<String>, String> {
    let (_reader, header_row) = open_text(&text, "csv_headers")?;
    return Ok(header_row.iter().map(|field| field.to_string()).collect());
}

/// How many data rows the text has, not counting the header row. A newline
/// inside a quoted field does not add a row, which is exactly the count that
/// counting the text's lines gets wrong.
pub fn data_row_count(text: String) -> Result<i64, String> {
    let (mut reader, _header_row) = open_text(&text, "csv_row_count")?;
    let mut count: i64 = 0;
    for result in reader.records() {
        result.map_err(|e| format!("csv_row_count: could not read a row: {}", e))?;
        count += 1;
    }
    return Ok(count);
}

/// One column's values as strings, found by header name. A row too short to
/// reach the column contributes an empty string, matching how csv_parse treats
/// ragged rows.
pub fn column(text: String, header: String) -> Result<Vec<String>, String> {
    let (mut reader, header_row) = open_text(&text, "csv_column")?;
    let index = header_index(&header_row, &header, "csv_column")?;
    let mut values = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("csv_column: could not read a row: {}", e))?;
        values.push(record.get(index).unwrap_or("").to_string());
    }
    return Ok(values);
}

/// A single value by header name and zero-based data row index, so row 0 is
/// the first row after the header.
pub fn cell(text: String, header: String, row: i64) -> Result<String, String> {
    let (mut reader, header_row) = open_text(&text, "csv_cell")?;
    let index = header_index(&header_row, &header, "csv_cell")?;
    let wanted = usize::try_from(row).map_err(|_| format!("csv_cell: the row cannot be negative, but it is {}", row))?;
    let mut seen = 0usize;
    for result in reader.records() {
        let record = result.map_err(|e| format!("csv_cell: could not read a row: {}", e))?;
        if seen == wanted {
            return Ok(record.get(index).unwrap_or("").to_string());
        }
        seen += 1;
    }
    return Err(format!("csv_cell: there is no data row {}, the text has {} of them", wanted, seen));
}

/// A new CSV keeping only the named columns, in the order given. Read and
/// written by the csv crate, so quoting is undone and redone properly rather
/// than carried across by text surgery.
pub fn select_columns(text: String, headers: Vec<String>) -> Result<String, String> {
    if headers.is_empty() {
        return Err("csv_select_columns: no column names were given, so there is nothing to keep".to_string());
    }
    let (mut reader, header_row) = open_text(&text, "csv_select_columns")?;
    let indexes: Vec<usize> = headers.iter().map(|header| header_index(&header_row, header, "csv_select_columns")).collect::<Result<Vec<usize>, String>>()?;

    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(&headers).map_err(|e| format!("csv_select_columns: could not write the header row: {}", e))?;
    for result in reader.records() {
        let record = result.map_err(|e| format!("csv_select_columns: could not read a row: {}", e))?;
        let fields: Vec<&str> = indexes.iter().map(|index| record.get(*index).unwrap_or("")).collect();
        writer.write_record(&fields).map_err(|e| format!("csv_select_columns: could not write a row: {}", e))?;
    }

    let bytes = writer.into_inner().map_err(|e| format!("csv_select_columns: could not finish writing: {}", e))?;
    return String::from_utf8(bytes).map_err(|e| format!("csv_select_columns: the written text is not valid UTF-8: {}", e));
}


/// A cursor over a CSV file, so a file larger than memory can be walked in
/// batches instead of being read into a string first.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CSV_Reader {
    pub handle: String,
    pub path: String,
}

struct OpenReader {
    reader: csv::Reader<Box<dyn std::io::Read + Send>>,
    keys: Vec<String>,
    null_values: Vec<String>,
    ignore_errors: bool,
    /// Rows still allowed by n_rows across all batches; None means no limit.
    remaining: Option<usize>,
}

lazy_static! {
    static ref OPEN_READERS: DashMap<String, Mutex<OpenReader>> = DashMap::new();
    static ref NEXT_HANDLE: AtomicU64 = AtomicU64::new(0);
}

/// Opens a CSV file for batch reading. The file stays open until csv_close,
/// so a reader that is never closed holds its descriptor for the process's life.
pub fn open(path: String, options: CSV_Options) -> Result<CSV_Reader, String> {
    let file = std::fs::File::open(&path).map_err(|e| format!("csv_open: could not open '{}': {}", path, e))?;

    // Banner rows are consumed before the parser sees the file, so the real
    // header is not mistaken for data.
    let skip_rows = row_count(options.skip_rows, "skip_rows")?;
    let mut buffered = std::io::BufReader::new(file);
    for _ in 0..skip_rows {
        let mut discarded = String::new();
        if buffered.read_line(&mut discarded).map_err(|e| format!("csv_open: could not read '{}': {}", path, e))? == 0 {
            break;
        }
    }

    let mut reader = build_reader(&options)?.from_reader(Box::new(buffered) as Box<dyn std::io::Read + Send>);

    let keys: Vec<String> = if options.has_headers {
        reader.headers().map_err(|e| format!("csv_open: could not read the header row of '{}': {}", path, e))?.iter().map(|header| header.to_string()).collect()
    } else {
        Vec::new()
    };

    let n_rows = row_count(options.n_rows, "n_rows")?;
    let handle = format!("csv-{}", NEXT_HANDLE.fetch_add(1, Ordering::Relaxed));
    OPEN_READERS.insert(
        handle.clone(),
        Mutex::new(OpenReader {
            reader,
            keys,
            null_values: options.null_values.clone(),
            ignore_errors: options.ignore_errors,
            remaining: if n_rows > 0 { Some(n_rows) } else { None },
        }),
    );

    Ok(CSV_Reader { handle, path })
}

/// Reads up to `count` more rows. A batch shorter than `count` means the file
/// is exhausted, so a caller loops until it gets one.
pub fn next_rows(reader: &CSV_Reader, count: i64) -> Result<Vec<DashMap<String, String>>, String> {
    let wanted = row_count(count, "count")?;

    let entry = OPEN_READERS
        .get(&reader.handle)
        .ok_or_else(|| format!("csv_next_rows: reader for '{}' is closed", reader.path))?;
    let mut open_reader = entry.lock().map_err(|_| format!("csv_next_rows: reader for '{}' was poisoned by an earlier panic", reader.path))?;

    let mut rows = Vec::new();
    while rows.len() < wanted {
        if open_reader.remaining == Some(0) {
            break;
        }

        let mut record = csv::StringRecord::new();
        let has_row = loop {
            match open_reader.reader.read_record(&mut record) {
                Ok(has_row) => break has_row,
                Err(_) if open_reader.ignore_errors => continue,
                Err(e) => return Err(format!("csv_next_rows: could not read a row of '{}': {}", reader.path, e)),
            }
        };
        if !has_row {
            break;
        }

        let row = DashMap::new();
        for (index, field) in record.iter().enumerate() {
            let key = match open_reader.keys.get(index) {
                Some(header) => header.clone(),
                None => index.to_string(),
            };
            let value = if open_reader.null_values.iter().any(|null_value| null_value == field) { String::new() } else { field.to_string() };
            row.insert(key, value);
        }
        rows.push(row);

        if let Some(remaining) = open_reader.remaining.as_mut() {
            *remaining -= 1;
        }
    }

    Ok(rows)
}

/// Closes the reader and releases its file descriptor.
pub fn close(reader: &CSV_Reader) -> Result<(), String> {
    match OPEN_READERS.remove(&reader.handle) {
        Some(_) => Ok(()),
        None => Err(format!("csv_close: reader for '{}' is already closed", reader.path)),
    }
}

/// Writes rows out as CSV text, with the columns named and in the order given.
/// A field that holds the delimiter, a quote or a newline is quoted, and a
/// quote inside it doubled - the escaping every hand-rolled CSV writer gets
/// wrong the first time a customer has a comma in their name. A row missing a
/// column is written as an empty field rather than refused, since a hashmap
/// row that never had the key is the normal case.
pub fn serialize(headers: Vec<String>, rows: Vec<DashMap<String, String>>, options: CSV_Options) -> Result<String, String> {
    if headers.is_empty() {
        return Err("csv_serialize: no column names were given, so there is nothing to write".to_string());
    }

    let delimiter = match options.delimiter.chars().next() {
        Some(character) => character,
        None => ',',
    };
    let quote = match options.quote.chars().next() {
        Some(character) => character,
        None => '"',
    };

    let mut written = String::new();
    if options.has_headers {
        written.push_str(&join_row(&headers, delimiter, quote));
        written.push('\n');
    }

    for row in rows.iter() {
        let fields: Vec<String> = headers
            .iter()
            .map(|column| match row.get(column) {
                Some(found) => found.value().clone(),
                None => String::new(),
            })
            .collect();
        written.push_str(&join_row(&fields, delimiter, quote));
        written.push('\n');
    }
    return Ok(written);
}

/// One row of CSV, with each field quoted only when it has to be.
fn join_row(fields: &[String], delimiter: char, quote: char) -> String {
    let written: Vec<String> = fields
        .iter()
        .map(|field| {
            let needs_quoting = field.contains(delimiter) || field.contains(quote) || field.contains('\n') || field.contains('\r');
            if !needs_quoting {
                return field.clone();
            }
            let escaped = field.replace(quote, &format!("{}{}", quote, quote));
            return format!("{}{}{}", quote, escaped, quote);
        })
        .collect();
    return written.join(&delimiter.to_string());
}

/// Writes rows straight to a file as CSV. The same escaping as csv_serialize,
/// and the file is put in place by a rename, so a reader never catches it half
/// written.
#[cfg(not(target_arch = "wasm32"))]
pub async fn write(path: String, headers: Vec<String>, rows: Vec<DashMap<String, String>>, options: CSV_Options) -> Result<(), String> {
    let text = serialize(headers, rows, options)?;
    return crate::parser::std_lib::fs::write_atomic(path, text).await;
}

#[cfg(test)]
mod writing_tests {
    use super::*;

    fn row(pairs: &[(&str, &str)]) -> DashMap<String, String> {
        let built = DashMap::new();
        for (key, value) in pairs.iter() {
            built.insert(key.to_string(), value.to_string());
        }
        return built;
    }

    #[test]
    fn columns_are_written_in_the_order_they_were_named() {
        let rows = vec![row(&[("name", "Ada"), ("city", "London")]), row(&[("city", "Calgary"), ("name", "Bob")])];
        let text = serialize(vec!["name".to_string(), "city".to_string()], rows, default_options()).expect("columns");
        assert_eq!(text, "name,city\nAda,London\nBob,Calgary\n");
    }

    #[test]
    fn a_field_that_would_break_the_format_is_quoted() {
        let rows = vec![row(&[("name", "Doe, Jane"), ("note", "she said \"hi\""), ("address", "one\ntwo")])];
        let text = serialize(vec!["name".to_string(), "note".to_string(), "address".to_string()], rows, default_options()).expect("columns");
        assert_eq!(text, "name,note,address\n\"Doe, Jane\",\"she said \"\"hi\"\"\",\"one\ntwo\"\n");
    }

    #[test]
    fn what_is_written_can_be_read_back() {
        let rows = vec![row(&[("name", "Doe, Jane"), ("note", "she said \"hi\"")])];
        let text = serialize(vec!["name".to_string(), "note".to_string()], rows, default_options()).expect("columns");
        let read_back = parse(text, default_options()).expect("what we just wrote");
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].get("name").expect("the column").value().clone(), "Doe, Jane");
        assert_eq!(read_back[0].get("note").expect("the column").value().clone(), "she said \"hi\"");
    }

    #[test]
    fn a_missing_column_is_written_as_an_empty_field() {
        let rows = vec![row(&[("name", "Ada")])];
        let text = serialize(vec!["name".to_string(), "city".to_string()], rows, default_options()).expect("columns");
        assert_eq!(text, "name,city\nAda,\n");
    }

    #[test]
    fn the_delimiter_and_the_header_row_follow_the_options() {
        let mut options = default_options();
        options.delimiter = "\t".to_string();
        options.has_headers = false;
        let rows = vec![row(&[("name", "Ada"), ("city", "London")])];
        let text = serialize(vec!["name".to_string(), "city".to_string()], rows, options).expect("columns");
        assert_eq!(text, "Ada\tLondon\n");
    }

    #[test]
    fn writing_no_columns_is_an_error() {
        assert!(serialize(vec![], vec![], default_options()).unwrap_err().contains("no column names"));
    }

    #[tokio::test]
    async fn a_written_file_reads_back_as_the_same_rows() {
        let path = crate::parser::std_lib::fs::temp_file("nail_csv_".to_string(), "csv".to_string()).await.expect("a writable temporary directory");
        let rows = vec![row(&[("name", "Ada"), ("city", "London")])];
        write(path.clone(), vec!["name".to_string(), "city".to_string()], rows, default_options()).await.expect("a writable path");

        let text = crate::parser::std_lib::fs::read_file(path.clone()).await.expect("the file we wrote");
        assert_eq!(text, "name,city\nAda,London\n");
        tokio::fs::remove_file(&path).await.expect("a removable file");
    }
}

#[cfg(test)]
mod text_helper_tests {
    use super::*;

    /// A document with every layer of quoting trouble: a quoted header, fields
    /// holding commas, a doubled quote and a newline inside a field.
    fn tricky_text() -> String {
        return "name,\"city, region\",note\n\"Doe, Jane\",\"Calgary, AB\",\"said \"\"hi\"\"\"\nBob,\"London, UK\",\"one\ntwo\"\n".to_string();
    }

    #[test]
    fn the_header_row_survives_quoted_commas() {
        let found = headers(tricky_text()).expect("a header row");
        assert_eq!(found, vec!["name".to_string(), "city, region".to_string(), "note".to_string()]);
    }

    #[test]
    fn empty_text_has_no_header_row() {
        assert!(headers(String::new()).unwrap_err().contains("empty"));
        assert!(headers("  \n ".to_string()).unwrap_err().contains("empty"));
        assert!(data_row_count(String::new()).unwrap_err().contains("empty"));
    }

    #[test]
    fn rows_are_counted_without_the_header() {
        assert_eq!(data_row_count(tricky_text()).expect("countable rows"), 2);
        assert_eq!(data_row_count("name,city\n".to_string()).expect("countable rows"), 0);
    }

    #[test]
    fn a_newline_inside_quotes_does_not_add_a_row() {
        assert_eq!(data_row_count("a,b\n\"one\ntwo\",x\n".to_string()).expect("countable rows"), 1);
    }

    #[test]
    fn a_column_is_read_by_its_header() {
        let cities = column(tricky_text(), "city, region".to_string()).expect("a named column");
        assert_eq!(cities, vec!["Calgary, AB".to_string(), "London, UK".to_string()]);
    }

    #[test]
    fn a_missing_column_error_lists_the_real_ones() {
        let failure = column(tricky_text(), "country".to_string()).unwrap_err();
        assert!(failure.contains("'country'"), "got: {}", failure);
        assert!(failure.contains("'name'"), "got: {}", failure);
        assert!(failure.contains("'city, region'"), "got: {}", failure);
        assert!(failure.contains("'note'"), "got: {}", failure);
    }

    #[test]
    fn a_cell_is_read_by_header_and_row() {
        assert_eq!(cell(tricky_text(), "name".to_string(), 0).expect("a cell"), "Doe, Jane");
        assert_eq!(cell(tricky_text(), "note".to_string(), 1).expect("a cell"), "one\ntwo");
    }

    #[test]
    fn a_cell_outside_the_rows_says_how_many_there_are() {
        let failure = cell(tricky_text(), "name".to_string(), 5).unwrap_err();
        assert!(failure.contains("no data row 5"), "got: {}", failure);
        assert!(failure.contains("2 of them"), "got: {}", failure);
        assert!(cell(tricky_text(), "name".to_string(), -1).unwrap_err().contains("negative"));
        assert!(cell(tricky_text(), "nope".to_string(), 0).unwrap_err().contains("'nope'"));
    }

    #[test]
    fn selected_columns_come_back_in_the_order_named() {
        let text = select_columns("name,city\n\"Doe, Jane\",Calgary\n".to_string(), vec!["city".to_string(), "name".to_string()]).expect("named columns");
        assert_eq!(text, "city,name\nCalgary,\"Doe, Jane\"\n");
    }

    #[test]
    fn a_selection_round_trips_through_the_reader() {
        let text = select_columns(tricky_text(), vec!["note".to_string(), "name".to_string()]).expect("named columns");
        let rows = parse(text, default_options()).expect("what was just written");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("note").expect("the column").value().clone(), "said \"hi\"");
        assert_eq!(rows[0].get("name").expect("the column").value().clone(), "Doe, Jane");
        assert_eq!(rows[1].get("note").expect("the column").value().clone(), "one\ntwo");
        assert_eq!(rows[1].get("name").expect("the column").value().clone(), "Bob");
    }

    #[test]
    fn selecting_a_missing_column_is_an_error_naming_it() {
        let failure = select_columns(tricky_text(), vec!["nope".to_string()]).unwrap_err();
        assert!(failure.contains("'nope'"), "got: {}", failure);
        assert!(failure.contains("'name'"), "got: {}", failure);
    }

    #[test]
    fn selecting_no_columns_is_an_error() {
        assert!(select_columns(tricky_text(), vec![]).unwrap_err().contains("no column names"));
    }
}
