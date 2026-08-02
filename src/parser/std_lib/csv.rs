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
