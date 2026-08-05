//! Pdf module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Pdf:
        "pdf_text" [PdfExtract, Tokio] => "std_lib::pdf::text", (path: s) -> (s!e),
            "The text of a PDF, in reading order as far as the file allows. A scanned PDF is photographs and gives back nothing - that needs OCR, which this is not.",
            "contents:s = danger(pdf_text(`statement.pdf`));";
        "pdf_from_text" [PrintPdf, Tokio] => "std_lib::pdf::from_text", (path: s, title: s, body: s) -> (v!e),
            "Writes a paginated A4 PDF of a title and plain text body, wrapping lines and flowing onto as many pages as needed. For tables and letterheads, use real typesetting instead.",
            "danger(pdf_from_text(`invoice.pdf`, `Invoice INV-7`, body));";
    }
}
