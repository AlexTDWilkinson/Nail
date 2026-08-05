//! PDFs: reading the text out of one, and writing a simple one.
//!
//! A PDF is a page-description format, not a document format - reading one back
//! is excavation, and writing a rich one is typesetting. This module does the
//! two things a program actually needs: pull the text out of a PDF somebody
//! sent, and produce a clean, paginated text report - an invoice, a statement,
//! a summary - without a browser or an office suite anywhere near the server.
//!
//! Anything fancier than text - tables, images, letterheads - wants real
//! typesetting, and the honest tool for that is HTML printed to PDF by
//! something built for it.

use printpdf::{BuiltinFont, Mm, PdfDocument};

/// The text of a PDF, in reading order as far as the file allows. A scanned
/// PDF is photographs of text, and gives back nothing - that needs OCR, which
/// this is not.
pub async fn text(path: String) -> Result<String, String> {
    let path_for_task = path.clone();
    // Extraction is CPU work on a whole file, so it runs on the blocking pool.
    let extracted = tokio::task::spawn_blocking(move || pdf_extract::extract_text(&path_for_task))
        .await
        .map_err(|failure| format!("pdf_text: the extraction task failed: {}", failure))?
        .map_err(|failure| format!("pdf_text: could not read text out of '{}': {}", path, failure))?;
    // The extractor leaves runs of blank lines where the layout had space.
    let tidied = extracted.lines().map(|line| line.trim_end()).collect::<Vec<&str>>().join("\n");
    return Ok(tidied.trim().to_string());
}

/// A4, in millimetres, with the margins a printed report wants.
const PAGE_WIDTH: f32 = 210.0;
const PAGE_HEIGHT: f32 = 297.0;
const MARGIN: f32 = 20.0;
const BODY_POINTS: f32 = 11.0;
const LINE_HEIGHT: f32 = 5.0;
/// Helvetica at 11pt runs out of A4 width around here.
const CHARACTERS_PER_LINE: usize = 88;

/// Writes a paginated PDF of a title and body text. Lines wrap at the page's
/// width and long documents flow onto as many pages as they need. The body is
/// treated as plain text: markup is printed, not interpreted.
pub async fn from_text(path: String, title: String, body: String) -> Result<(), String> {
    let (document, first_page, first_layer) = PdfDocument::new(&title, Mm(PAGE_WIDTH), Mm(PAGE_HEIGHT), "text");
    let font = document.add_builtin_font(BuiltinFont::Helvetica).map_err(|failure| format!("pdf_from_text: could not load the built-in font: {}", failure))?;
    let title_font = document.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|failure| format!("pdf_from_text: could not load the built-in font: {}", failure))?;

    // Wrap each paragraph line to the page width, keeping blank lines.
    let mut lines: Vec<String> = Vec::new();
    for paragraph in body.lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let wrapped = crate::parser::std_lib::string::word_wrap(paragraph.to_string(), CHARACTERS_PER_LINE as i64);
        lines.extend(wrapped.lines().map(|line| line.to_string()));
    }

    let mut layer = document.get_page(first_page).get_layer(first_layer);
    let mut cursor = PAGE_HEIGHT - MARGIN;

    if !title.is_empty() {
        layer.use_text(title.clone(), 16.0, Mm(MARGIN), Mm(cursor), &title_font);
        cursor -= LINE_HEIGHT * 2.0;
    }

    for line in lines {
        if cursor < MARGIN {
            let (page, new_layer) = document.add_page(Mm(PAGE_WIDTH), Mm(PAGE_HEIGHT), "text");
            layer = document.get_page(page).get_layer(new_layer);
            cursor = PAGE_HEIGHT - MARGIN;
        }
        if !line.is_empty() {
            layer.use_text(line, BODY_POINTS, Mm(MARGIN), Mm(cursor), &font);
        }
        cursor -= LINE_HEIGHT;
    }

    let file = std::fs::File::create(&path).map_err(|failure| format!("pdf_from_text: could not write '{}': {}", path, failure))?;
    document.save(&mut std::io::BufWriter::new(file)).map_err(|failure| format!("pdf_from_text: could not write '{}': {}", path, failure))?;
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beside(name: &str) -> String {
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_file(&path);
        return path.to_string_lossy().to_string();
    }

    #[tokio::test]
    async fn a_written_pdf_reads_back_as_its_own_text() {
        let path = beside("nail_pdf_round_trip.pdf");
        from_text(path.clone(), "August Statement".to_string(), "Total due: $12.34\nThank you for your business.".to_string()).await.expect("a writable report");

        let read_back = text(path.clone()).await.expect("our own file");
        assert!(read_back.contains("August Statement"), "got: {}", read_back);
        assert!(read_back.contains("Total due: $12.34"), "got: {}", read_back);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_long_body_flows_onto_more_pages() {
        let path = beside("nail_pdf_long.pdf");
        let body: Vec<String> = (1..=200).map(|line_number| format!("line {}", line_number)).collect();
        from_text(path.clone(), "Long".to_string(), body.join("\n")).await.expect("a writable report");

        let read_back = text(path.clone()).await.expect("our own file");
        assert!(read_back.contains("line 1"), "the first page is there");
        assert!(read_back.contains("line 200"), "and so is the last");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn something_that_is_not_a_pdf_says_so() {
        let path = beside("nail_pdf_not_a_pdf.pdf");
        std::fs::write(&path, "just text").expect("a writable file");
        let failure = text(path.clone()).await.unwrap_err();
        assert!(failure.contains("pdf_text"), "got: {}", failure);
        let _ = std::fs::remove_file(&path);
    }
}
