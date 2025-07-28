use pulldown_cmark::{Parser, Options, html};

pub async fn to_html(markdown: String) -> String {
    let parser = Parser::new(&markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub async fn to_html_with_options(markdown: String, enable_tables: bool, enable_footnotes: bool, enable_strikethrough: bool) -> String {
    let mut options = Options::empty();
    if enable_tables {
        options.insert(Options::ENABLE_TABLES);
    }
    if enable_footnotes {
        options.insert(Options::ENABLE_FOOTNOTES);
    }
    if enable_strikethrough {
        options.insert(Options::ENABLE_STRIKETHROUGH);
    }
    
    let parser = Parser::new_ext(&markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}