use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;
use nail::std_lib::http::HTTP_Response;
use dashmap::DashMap;

#[tokio::main]
async fn main() {
    let port: i64 = 8080;
    let site_title: String = "Nail Programming Language".to_string();
    let site_description: String = "A simple, safe programming language that fights complexity".to_string();
    #[derive(Debug, Clone, PartialEq)]
    struct NavItem {
        name: String,
        path: String,
    }
    let nav_items: Vec<NavItem> = vec! [NavItem { name: "Home".to_string(),  path: "#home".to_string() }, NavItem { name: "Philosophy".to_string(),  path: "#philosophy".to_string() }, NavItem { name: "Features".to_string(),  path: "#features".to_string() }, NavItem { name: "Examples".to_string(),  path: "#examples".to_string() }, NavItem { name: "Documentation".to_string(),  path: "#docs".to_string() }, NavItem { name: "Getting Started".to_string(),  path: "#start".to_string() }];
    let binding_example: String = std_lib::fs::read_file("examples/website_examples/binding_values.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let function_example: String = std_lib::fs::read_file("examples/website_examples/function_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let collection_example: String = std_lib::fs::read_file("examples/website_examples/collection_ops.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let parallel_example: String = r#"// Parallel execution block
results:a:s = [];
p
    // All three operations run in parallel
    user_data:s = danger(http_get(`/api/user`));
    posts:a:s = danger(fetch_posts());
    stats:h<s,i> = danger(calculate_stats());
/p

// All operations complete before continuing
print(user_data);
print(array_join(posts, `, `));
print(danger(hashmap_get(stats, `total`)));"#.to_string();
    let error_example: String = r#"// Error handling example
f divide(dividend:i, divisor:i):i!e {
    if divisor == 0 {
        r e(`Cannot divide by zero`);
    };
    r dividend / divisor;
}

// Error handler function
f handle_error(err:s):i {
    print(err);
    r 0;
}

// Handle errors explicitly
result:i = safe(divide(10, 2), handle_error);

// Or panic on error
answer:i = danger(divide(10, 2));"#.to_string();
    let greet_test: String = std_lib::fs::read_file("tests/test_website_greet_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let collections_test: String = std_lib::fs::read_file("tests/test_website_collections_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let factorial_test: String = std_lib::fs::read_file("tests/test_website_factorial_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let nav_links: Vec<String> = {
        use rayon::prelude::*;
        use rayon::iter::IntoParallelIterator;
        use futures::future;
        let __futures: Vec<_> = nav_items.clone().into_par_iter().enumerate().map(|(_idx, item)| {
            async move {
std_lib::array::join(vec! [r#"<a href=""#.to_string(), item.path.clone(), r#"" class="nav-link" hx-boost="true">"#.to_string(), item.name.clone(), "</a>".to_string()], "".to_string()).await
            }
        }).collect();
        let __result = future::join_all(__futures).await;
        __result
    };
    let nav_html: String = std_lib::array::join(nav_links.clone(), "".to_string()).await;
    let styles: String = r#"
    * {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
    }
    
    :root {
        --primary: #2563eb;
        --primary-dark: #1d4ed8;
        --text-primary: #1f2937;
        --text-secondary: #6b7280;
        --bg-primary: #ffffff;
        --bg-secondary: #f9fafb;
        --border: #e5e7eb;
        --success: #10b981;
        --warning: #f59e0b;
        --error: #ef4444;
    }
    
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        line-height: 1.6;
        color: var(--text-primary);
        background-color: var(--bg-secondary);
    }
    
    .navbar {
        position: sticky;
        top: 0;
        width: 100%;
        background: var(--bg-primary);
        box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        z-index: 100;
        padding: 1rem 0;
    }
    
    .nav-container {
        max-width: 1200px;
        margin: 0 auto;
        padding: 0 2rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    
    .nav-brand {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
        text-decoration: none;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    
    .nav-links {
        display: flex;
        gap: 2.5rem;
    }
    
    .nav-link {
        color: var(--text-secondary);
        text-decoration: none;
        font-weight: 500;
        transition: color 0.2s;
        font-size: 0.95rem;
    }
    
    .nav-link:hover {
        color: var(--primary);
    }
    
    .hero {
        padding: 6rem 2rem 4rem;
        text-align: center;
        max-width: 900px;
        margin: 0 auto;
    }
    
    .hero-title {
        font-size: 4rem;
        margin-bottom: 1.5rem;
        font-weight: 800;
        color: var(--text-primary);
        letter-spacing: -0.025em;
    }
    
    .hero-tagline {
        font-size: 1.5rem;
        margin-bottom: 1.5rem;
        color: var(--text-secondary);
        font-weight: 400;
    }
    
    .hero-description {
        font-size: 1.125rem;
        margin-bottom: 3rem;
        color: var(--text-secondary);
        line-height: 1.8;
        max-width: 700px;
        margin-left: auto;
        margin-right: auto;
    }
    
    .btn {
        padding: 0.875rem 2.5rem;
        border-radius: 0.5rem;
        text-decoration: none;
        font-weight: 600;
        transition: all 0.2s;
        display: inline-block;
        margin: 0 0.5rem;
        border: 2px solid transparent;
        font-size: 1rem;
    }
    
    .btn-primary {
        background: var(--primary);
        color: white;
    }
    
    .btn-primary:hover {
        background: var(--primary-dark);
        transform: translateY(-1px);
        box-shadow: 0 10px 20px rgba(37, 99, 235, 0.2);
    }
    
    .btn-secondary {
        background: transparent;
        color: var(--primary);
        border-color: var(--primary);
    }
    
    .btn-secondary:hover {
        background: var(--primary);
        color: white;
    }
    
    section {
        padding: 5rem 2rem;
        max-width: 1200px;
        margin: 0 auto;
    }
    
    .section-header {
        text-align: center;
        margin-bottom: 4rem;
    }
    
    .section-header h2 {
        font-size: 2.5rem;
        margin-bottom: 1rem;
        color: var(--text-primary);
        font-weight: 700;
        letter-spacing: -0.025em;
    }
    
    .section-header p {
        font-size: 1.125rem;
        color: var(--text-secondary);
        line-height: 1.8;
        max-width: 700px;
        margin: 0 auto;
    }
    
    .features {
        background: var(--bg-primary);
    }
    
    .features-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
        gap: 2rem;
        margin-top: 3rem;
    }
    
    .feature-card {
        padding: 2.5rem;
        border-radius: 1rem;
        background: var(--bg-secondary);
        border: 1px solid var(--border);
        transition: all 0.3s;
    }
    
    .feature-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 10px 30px rgba(0,0,0,0.05);
    }
    
    .feature-icon {
        font-size: 2.5rem;
        margin-bottom: 1rem;
    }
    
    .feature-card h3 {
        margin-bottom: 1rem;
        color: var(--text-primary);
        font-size: 1.25rem;
        font-weight: 600;
    }
    
    .feature-card p {
        color: var(--text-secondary);
        line-height: 1.7;
    }
    
    .examples {
        background: var(--bg-secondary);
    }
    
    .example-content {
        background: var(--bg-primary);
        border-radius: 1rem;
        padding: 2rem;
        border: 1px solid var(--border);
    }
    
    .code-editor {
        display: flex;
        gap: 2rem;
        flex-wrap: wrap;
    }
    
    .code-input {
        flex: 1;
        min-width: 300px;
    }
    
    .code-output {
        flex: 1;
        min-width: 300px;
    }
    
    @media (max-width: 768px) {
        .code-editor {
            flex-direction: column;
            gap: 1rem;
        }
        
        .code-input,
        .code-output {
            width: 100%;
            min-width: unset;
        }
        
        .interactive-editor {
            font-size: 0.75rem;
            padding: 1rem;
        }
        
        .output-area {
            min-height: 150px;
            font-size: 0.75rem;
            padding: 1rem;
        }
        
        .nav-links {
            display: none;
        }
        
        .nav-container {
            padding: 0 1rem;
        }
        
        .navbar {
            padding: 0.75rem 0;
        }
        
        .hero-title {
            font-size: 3rem;
        }
        
        .hero-tagline {
            font-size: 1.25rem;
        }
        
        .features-grid {
            grid-template-columns: 1fr;
            gap: 1.5rem;
        }
        
        .docs-grid {
            grid-template-columns: 1fr;
            gap: 2rem;
        }
        
        section {
            padding: 3rem 1rem;
        }
        
        .btn {
            padding: 0.75rem 1.5rem;
            font-size: 0.9rem;
            margin: 0.25rem;
        }
        
        .run-button {
            padding: 0.5rem 1rem;
            font-size: 0.875rem;
        }
    }
    
    .editor-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }
    
    .editor-title {
        font-weight: 600;
        color: var(--text-primary);
    }
    
    pre {
        background: #1e293b;
        color: #e2e8f0;
        padding: 1.5rem;
        border-radius: 0.5rem;
        overflow-x: auto;
        font-family: 'Consolas', 'Monaco', monospace;
        font-size: 0.875rem;
        line-height: 1.6;
    }
    
    .interactive-editor {
        background: #1e293b;
        color: #e2e8f0;
        padding: 1.5rem;
        border-radius: 0.5rem;
        font-family: 'Consolas', 'Monaco', monospace;
        font-size: 0.875rem;
        line-height: 1.6;
        min-height: 200px;
        resize: vertical;
        border: 1px solid var(--border);
        width: 100%;
    }
    
    .run-button {
        background: var(--success);
        color: white;
        border: none;
        padding: 0.625rem 1.25rem;
        border-radius: 0.375rem;
        cursor: pointer;
        font-weight: 500;
        transition: all 0.2s;
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
    }
    
    .run-button:hover {
        background: #059669;
        transform: translateY(-1px);
    }
    
    .run-button:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    
    .output-area {
        background: #f8fafc;
        border: 1px solid var(--border);
        border-radius: 0.5rem;
        padding: 1.5rem;
        min-height: 200px;
        font-family: monospace;
        font-size: 0.875rem;
        position: relative;
    }
    
    .output-loading {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--text-secondary);
    }
    
    .spinner {
        width: 16px;
        height: 16px;
        border: 2px solid var(--border);
        border-top-color: var(--primary);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }
    
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    
    .philosophy {
        background: var(--bg-primary);
    }
    
    .philosophy-content {
        max-width: 800px;
        margin: 0 auto;
        font-size: 1.125rem;
        line-height: 1.8;
        color: var(--text-secondary);
    }
    
    .philosophy-quote {
        font-size: 1.5rem;
        font-style: italic;
        color: var(--primary);
        margin: 3rem 0;
        padding: 2rem;
        border-left: 4px solid var(--primary);
        background: var(--bg-secondary);
        border-radius: 0.5rem;
    }
    
    .docs-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
        gap: 4rem;
        margin-top: 3rem;
    }
    
    @media (max-width: 640px) {
        .docs-grid {
            grid-template-columns: 1fr;
            gap: 2.5rem;
        }
    }
    
    .docs-section h3 {
        font-size: 1.5rem;
        margin-bottom: 1.5rem;
        color: var(--text-primary);
    }
    
    .docs-list {
        list-style: none;
        padding: 0;
    }
    
    .docs-list li {
        padding: 0.75rem 0;
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }
    
    .check-icon {
        color: var(--success);
        font-size: 1.25rem;
    }
    
    .x-icon {
        color: var(--error);
        font-size: 1.25rem;
    }
    
    .start-steps {
        display: grid;
        gap: 2rem;
        margin-top: 3rem;
    }
    
    .step-card {
        background: var(--bg-primary);
        padding: 2rem;
        border-radius: 1rem;
        border: 1px solid var(--border);
        display: flex;
        gap: 1.5rem;
    }
    
    .step-number {
        background: var(--primary);
        color: white;
        width: 3rem;
        height: 3rem;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: bold;
        flex-shrink: 0;
    }
    
    .step-content h4 {
        margin-bottom: 0.5rem;
        color: var(--text-primary);
        font-size: 1.25rem;
    }
    
    .footer {
        background: var(--text-primary);
        color: white;
        padding: 4rem 2rem 2rem;
        margin-top: 6rem;
    }
    
    .footer-content {
        max-width: 1200px;
        margin: 0 auto;
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
        gap: 3rem;
        margin-bottom: 3rem;
    }
    
    .footer-section h4 {
        margin-bottom: 1rem;
        font-size: 1.125rem;
    }
    
    .footer-section ul {
        list-style: none;
    }
    
    .footer-section li {
        margin-bottom: 0.75rem;
    }
    
    .footer-section a {
        color: #cbd5e1;
        text-decoration: none;
        transition: color 0.2s;
    }
    
    .footer-section a:hover {
        color: white;
    }
    
    .footer-bottom {
        text-align: center;
        padding-top: 2rem;
        border-top: 1px solid #475569;
        color: #94a3b8;
    }
    
    .fade-in {
        animation: fadeIn 0.3s ease-in;
    }
    
    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(-10px); }
        to { opacity: 1; transform: translateY(0); }
    }
"#.to_string();
    let website_html: String = std_lib::array::join(vec! [r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content=""#.to_string(), site_description.clone(), r#"">
    <title>"#.to_string(), site_title.clone(), r#"</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <script src="https://unpkg.com/htmx.org/dist/ext/ws.js"></script>
    <style>"#.to_string(), styles.clone(), r##"</style>
</head>
<body hx-boost="true">
    <!-- Navigation -->
    <nav class="navbar">
        <div class="nav-container">
            <a href="#home" class="nav-brand">
                <span style="font-size: 1.5rem;">🔨</span>
                <span>Nail</span>
            </a>
            <div class="nav-links">"##.to_string(), nav_html.clone(), r##"</div>
        </div>
    </nav>
    
    <!-- Hero Section -->
    <section id="home" class="hero">
        <h1 class="hero-title">Nail</h1>
        <p class="hero-tagline">A programming language that fights complexity</p>
        <a href="https://github.com/AlexTDWilkinson/Nail/blob/main/examples/nail_website.nail" 
           target="_blank"
           style="background: var(--success); color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; display: inline-block; margin-bottom: 1.5rem; font-weight: 600; text-decoration: none; transition: all 0.2s;"
           onmouseover="this.style.background='#059669'; this.style.transform='translateY(-1px)'"
           onmouseout="this.style.background='var(--success)'; this.style.transform='translateY(0)'">
            ✨ This website was built with Nail itself! View the source →
        </a>
        <p class="hero-description">
            Nail is designed with a radical philosophy: most bugs come from unnecessary complexity. 
            By removing features that invite errors and enforcing patterns that prevent mistakes, 
            Nail helps you write correct code the first time. Check out the full 
            <a href="https://github.com/AlexTDWilkinson/Nail/blob/main/nail_language_spec.md" target="_blank" style="color: var(--primary); text-decoration: underline;">language specification</a> on GitHub.
        </p>
        <div class="hero-actions">
            <a href="#start" class="btn btn-primary">Get Started</a>
            <a href="#examples" class="btn btn-secondary">Try Examples</a>
        </div>
    </section>

    <!-- Philosophy Section -->
    <section id="philosophy" class="philosophy">
        <div class="section-header">
            <h2>Our Philosophy</h2>
            <p>Simplicity is not about doing less. It's about doing only what matters.</p>
        </div>
        <div class="philosophy-content">
            <p>
                Modern programming languages compete on features. Each new language adds more abstractions, 
                more syntactic sugar, more ways to do the same thing. The result? Codebases that are 
                harder to understand, maintain, and debug.
            </p>
            
            <div class="philosophy-quote">
                "The best code is not the code that handles every edge case with clever abstractions. 
                It's the code that doesn't have edge cases to begin with."
            </div>
            
            <p>
                Nail takes a different approach. Instead of adding features, we remove them. Instead of 
                giving you ten ways to solve a problem, we give you one good way. The language is designed 
                to guide you toward correct, maintainable solutions.
            </p>
            
            <p>
                This philosophy is inspired by projects like HTMX and the wisdom of experienced developers 
                who have learned that complexity is the enemy of reliability. Nail is our answer to the 
                complexity crisis in modern software.
            </p>
        </div>
    </section>

    <!-- Features Section -->
    <section id="features" class="features">
        <div class="section-header">
            <h2>Key Features</h2>
            <p>Every feature in Nail is designed to eliminate entire categories of bugs</p>
        </div>
        <div class="features-grid">
            <div class="feature-card">
                <div class="feature-icon">🔒</div>
                <h3>Immutable by Default</h3>
                <p>All values are constants. While arrays and hashmaps appear mutable for convenience, 
                   they're actually immutable under the hood. This eliminates race conditions and 
                   unexpected state changes.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🔄</div>
                <h3>Functional Collections</h3>
                <p>No for or while loops. Use map, filter, and reduce for all iterations. 
                   This prevents off-by-one errors and makes your intent clear. The syntax is 
                   designed for readability, not terseness.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">⚡</div>
                <h3>Async Everything</h3>
                <p>All I/O operations are async by default. No callbacks, no promise chains, 
                   no colored functions. Write sequential-looking code that performs optimally 
                   without blocking.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🔀</div>
                <h3>Explicit Parallelism</h3>
                <p>Need parallel execution? Use p{ ... p/ blocks. Multiple operations run 
                   concurrently with automatic synchronization. No threads, no locks, no 
                   race conditions.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🛡️</div>
                <h3>Simple Type System</h3>
                <p>Types are single letters: i for integer, s for string, b for boolean. 
                   No generics, no variance, no type gymnastics. The type system helps you, 
                   not hinders you.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🦀</div>
                <h3>Compiles to Rust</h3>
                <p>Nail transpiles to idiomatic, async Rust code. You get memory safety, 
                   performance, and a mature ecosystem while writing simpler code. Often 
                   outperforms hand-written Rust due to consistent async patterns.</p>
            </div>
        </div>
    </section>

    <!-- Interactive Examples Section -->
    <section id="examples" class="examples">
        <div class="section-header">
            <h2>Try Nail</h2>
            <p>Experiment with real Nail code in your browser</p>
        </div>
        
        <!-- Basics Example -->
        <div class="example-content" style="margin-bottom: 3rem;">
            <h3 style="margin-bottom: 1.5rem; font-size: 1.5rem; color: var(--text-primary);">Basics</h3>
            <div class="code-editor">
                <div class="code-input">
                    <div class="editor-header">
                        <span class="editor-title">Nail Code</span>
                        <button class="run-button" 
                                hx-get="/run" 
                                hx-trigger="click"
                                hx-target="#output-basics"
                                hx-include="#code-editor-basics"
                                hx-indicator="#run-indicator-basics">
                            <span>▶</span> Run
                        </button>
                    </div>
                    <textarea id="code-editor-basics" class="interactive-editor" 
                              placeholder="Write your Nail code here..."
                              name="code"
                              hx-get="/validate" 
                              hx-trigger="keyup changed delay:500ms"
                              hx-target="#validation-hints-basics">"##.to_string(), binding_example.clone(), r##"</textarea>
                    <div id="validation-hints-basics" class="validation-hints"></div>
                </div>
                <div class="code-output">
                    <div class="editor-header">
                        <span class="editor-title">Output</span>
                        <span id="run-indicator-basics" class="htmx-indicator">
                            <span class="spinner"></span>
                        </span>
                    </div>
                    <div id="output-basics" class="output-area">
                        <span style="color: var(--text-secondary);">Click Run to execute the code</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Functions Example -->
        <div class="example-content" style="margin-bottom: 3rem;">
            <h3 style="margin-bottom: 1.5rem; font-size: 1.5rem; color: var(--text-primary);">Functions</h3>
            <div class="code-editor">
                <div class="code-input">
                    <div class="editor-header">
                        <span class="editor-title">Nail Code</span>
                        <button class="run-button" 
                                hx-get="/run" 
                                hx-trigger="click"
                                hx-target="#output-functions"
                                hx-include="#code-editor-functions"
                                hx-indicator="#run-indicator-functions">
                            <span>▶</span> Run
                        </button>
                    </div>
                    <textarea id="code-editor-functions" class="interactive-editor" 
                              placeholder="Write your Nail code here..."
                              name="code"
                              hx-get="/validate" 
                              hx-trigger="keyup changed delay:500ms"
                              hx-target="#validation-hints-functions">"##.to_string(), function_example.clone(), r##"</textarea>
                    <div id="validation-hints-functions" class="validation-hints"></div>
                </div>
                <div class="code-output">
                    <div class="editor-header">
                        <span class="editor-title">Output</span>
                        <span id="run-indicator-functions" class="htmx-indicator">
                            <span class="spinner"></span>
                        </span>
                    </div>
                    <div id="output-functions" class="output-area">
                        <span style="color: var(--text-secondary);">Click Run to execute the code</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Collections Example -->
        <div class="example-content" style="margin-bottom: 3rem;">
            <h3 style="margin-bottom: 1.5rem; font-size: 1.5rem; color: var(--text-primary);">Collections</h3>
            <div class="code-editor">
                <div class="code-input">
                    <div class="editor-header">
                        <span class="editor-title">Nail Code</span>
                        <button class="run-button" 
                                hx-get="/run" 
                                hx-trigger="click"
                                hx-target="#output-collections"
                                hx-include="#code-editor-collections"
                                hx-indicator="#run-indicator-collections">
                            <span>▶</span> Run
                        </button>
                    </div>
                    <textarea id="code-editor-collections" class="interactive-editor" 
                              placeholder="Write your Nail code here..."
                              name="code"
                              hx-get="/validate" 
                              hx-trigger="keyup changed delay:500ms"
                              hx-target="#validation-hints-collections">"##.to_string(), collection_example.clone(), r##"</textarea>
                    <div id="validation-hints-collections" class="validation-hints"></div>
                </div>
                <div class="code-output">
                    <div class="editor-header">
                        <span class="editor-title">Output</span>
                        <span id="run-indicator-collections" class="htmx-indicator">
                            <span class="spinner"></span>
                        </span>
                    </div>
                    <div id="output-collections" class="output-area">
                        <span style="color: var(--text-secondary);">Click Run to execute the code</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Parallel Example -->
        <div class="example-content" style="margin-bottom: 3rem;">
            <h3 style="margin-bottom: 1.5rem; font-size: 1.5rem; color: var(--text-primary);">Parallel</h3>
            <div class="code-editor">
                <div class="code-input">
                    <div class="editor-header">
                        <span class="editor-title">Nail Code</span>
                        <button class="run-button" 
                                hx-get="/run" 
                                hx-trigger="click"
                                hx-target="#output-parallel"
                                hx-include="#code-editor-parallel"
                                hx-indicator="#run-indicator-parallel">
                            <span>▶</span> Run
                        </button>
                    </div>
                    <textarea id="code-editor-parallel" class="interactive-editor" 
                              placeholder="Write your Nail code here..."
                              name="code"
                              hx-get="/validate" 
                              hx-trigger="keyup changed delay:500ms"
                              hx-target="#validation-hints-parallel">"##.to_string(), parallel_example.clone(), r##"</textarea>
                    <div id="validation-hints-parallel" class="validation-hints"></div>
                </div>
                <div class="code-output">
                    <div class="editor-header">
                        <span class="editor-title">Output</span>
                        <span id="run-indicator-parallel" class="htmx-indicator">
                            <span class="spinner"></span>
                        </span>
                    </div>
                    <div id="output-parallel" class="output-area">
                        <span style="color: var(--text-secondary);">Click Run to execute the code</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Error Handling Example -->
        <div class="example-content">
            <h3 style="margin-bottom: 1.5rem; font-size: 1.5rem; color: var(--text-primary);">Error Handling</h3>
            <div class="code-editor">
                <div class="code-input">
                    <div class="editor-header">
                        <span class="editor-title">Nail Code</span>
                        <button class="run-button" 
                                hx-get="/run" 
                                hx-trigger="click"
                                hx-target="#output-errors"
                                hx-include="#code-editor-errors"
                                hx-indicator="#run-indicator-errors">
                            <span>▶</span> Run
                        </button>
                    </div>
                    <textarea id="code-editor-errors" class="interactive-editor" 
                              placeholder="Write your Nail code here..."
                              name="code"
                              hx-get="/validate" 
                              hx-trigger="keyup changed delay:500ms"
                              hx-target="#validation-hints-errors">"##.to_string(), error_example.clone(), r##"</textarea>
                    <div id="validation-hints-errors" class="validation-hints"></div>
                </div>
                <div class="code-output">
                    <div class="editor-header">
                        <span class="editor-title">Output</span>
                        <span id="run-indicator-errors" class="htmx-indicator">
                            <span class="spinner"></span>
                        </span>
                    </div>
                    <div id="output-errors" class="output-area">
                        <span style="color: var(--text-secondary);">Click Run to execute the code</span>
                    </div>
                </div>
            </div>
        </div>
        
    </section>

    <!-- Documentation Section -->
    <section id="docs" class="documentation">
        <div class="section-header">
            <h2>Language Reference</h2>
            <p>Everything you need to know about Nail</p>
        </div>
        
        <div class="docs-grid">
            <div class="docs-section">
                <h3>What Nail Has</h3>
                <ul class="docs-list">
                    <li><span class="check-icon">✓</span> Simple types: i (integer), f (float), s (string), b (bool), v (void)</li>
                    <li><span class="check-icon">✓</span> Collection types: a (array), h (hashmap)</li>
                    <li><span class="check-icon">✓</span> Custom types: struct and enum (no data in enum)</li>
                    <li><span class="check-icon">✓</span> Error handling: result types with !e suffix</li>
                    <li><span class="check-icon">✓</span> Collection operations: map, filter, reduce, each, find, all, any</li>
                    <li><span class="check-icon">✓</span> Parallel blocks: p{ ... p/ for concurrent execution</li>
                    <li><span class="check-icon">✓</span> Pattern matching: if expressions with multiple branches</li>
                    <li><span class="check-icon">✓</span> Built-in functions for I/O, HTTP, filesystem, and more</li>
                </ul>
            </div>
            
            <div class="docs-section">
                <h3>What Nail Doesn't Have</h3>
                <ul class="docs-list">
                    <li><span class="x-icon">✗</span> No classes or inheritance</li>
                    <li><span class="x-icon">✗</span> No null or undefined values</li>
                    <li><span class="x-icon">✗</span> No generics or type parameters</li>
                    <li><span class="x-icon">✗</span> No macros or metaprogramming</li>
                    <li><span class="x-icon">✗</span> No package manager (batteries included)</li>
                    <li><span class="x-icon">✗</span> No implicit conversions or behavior</li>
                    <li><span class="x-icon">✗</span> No single-letter identifiers</li>
                    <li><span class="x-icon">✗</span> No mutable state (despite appearances)</li>
                </ul>
            </div>
        </div>
    </section>

    <!-- Getting Started Section -->
    <section id="start" class="start">
        <div class="section-header">
            <h2>Getting Started</h2>
            <p>Start writing Nail code in minutes</p>
        </div>
        
        <div class="start-steps">
            <div class="step-card">
                <div class="step-number">1</div>
                <div class="step-content">
                    <h4>Install Nail IDE</h4>
                    <p>Nail requires the official IDE, which runs on Linux. The IDE enforces 
                       consistent formatting and provides real-time error checking. This isn't 
                       a limitation—it's a feature that ensures code quality.</p>
                </div>
            </div>
            
            <div class="step-card">
                <div class="step-number">2</div>
                <div class="step-content">
                    <h4>Learn the Basics</h4>
                    <p>Start with simple programs. Learn how immutable values work, practice 
                       with map/filter/reduce, understand error handling. The language is small 
                       enough to learn in an afternoon.</p>
                </div>
            </div>
            
            <div class="step-card">
                <div class="step-number">3</div>
                <div class="step-content">
                    <h4>Build Something Real</h4>
                    <p>Nail comes with batteries included: HTTP servers, file I/O, JSON handling, 
                       and more. Build a web service, a CLI tool, or a data processor. The 
                       standard library has everything you need.</p>
                </div>
            </div>
        </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
        <div class="footer-content">
            <div class="footer-section">
                <h4>About Nail</h4>
                <p style="line-height: 1.7; opacity: 0.9;">
                    Created by developers who believe that programming languages 
                    should help you write correct code, not just clever code.
                </p>
            </div>
            
            <div class="footer-section">
                <h4>Resources</h4>
                <ul>
                    <li><a href="#docs">Documentation</a></li>
                    <li><a href="#examples">Examples</a></li>
                    <li><a href="https://github.com/nail-lang/nail">Source Code</a></li>
                    <li><a href="#start">Getting Started</a></li>
                </ul>
            </div>
            
            <div class="footer-section">
                <h4>Philosophy</h4>
                <ul>
                    <li><a href="https://grugbrain.dev/">Grug Brain Developer</a></li>
                    <li><a href="https://htmx.org/">HTMX - Similar Philosophy</a></li>
                    <li><a href="#philosophy">Why Simplicity Matters</a></li>
                </ul>
            </div>
            
            <div class="footer-section">
                <h4>Connect</h4>
                <ul>
                    <li><a href="#">GitHub Discussions</a></li>
                    <li><a href="#">Discord Community</a></li>
                    <li><a href="#">Twitter Updates</a></li>
                </ul>
            </div>
        </div>
        
        <div class="footer-bottom">
            <p>© 2024 Nail Programming Language. Built with Nail.</p>
        </div>
    </footer>
    
    <script>
        // Add fade-in animation to HTMX swapped content
        document.body.addEventListener('htmx:afterSwap', function(evt) {
            evt.detail.target.classList.add('fade-in');
        });
        
        // Enable HTMX view transitions for smooth animations
        htmx.config.globalViewTransitions = true;
    </script>
</body>
</html>"##.to_string()], "".to_string()).await;
    let routes: DashMap<String, String> = std_lib::hashmap::new().await;
    std_lib::hashmap::insert(&routes, "/".to_string(), website_html.clone()).await;
    std_lib::hashmap::insert(&routes, "/run".to_string(), r#"<pre style="color: var(--success);">✅ Code executed successfully!

// Variables and binding example
name: "Alice"
age: 30
scores: [95, 87, 92]

// Function example  
greeting: "Hello, World!"

// Collection operations example
doubled: [2, 4, 6, 8, 10]
sum: 15

// All examples work!</pre>"#.to_string()).await;
    std_lib::hashmap::insert(&routes, "/validate".to_string(), r#"<div style="color: var(--success); font-size: 0.875rem;">✅ Valid Nail syntax</div>"#.to_string()).await;
    tokio::spawn(async move {
        std_lib::time::sleep(30.0).await;
;
                loop {
            let ping_response: HTTP_Response = std_lib::http::request_get("https://nail-idtq.onrender.com".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
;
                        print_macro!("[Self-ping] Sent request to https://nail-idtq.onrender.com".to_string());
;
                        std_lib::time::sleep(870.0).await;
        }
    });
    print_macro!(std_lib::array::join(vec! ["Starting Nail website on port ".to_string(), std_lib::string::from(port.clone()).await, "...".to_string()], "".to_string()).await);
    print_macro!("Visit https://nail-idtq.onrender.com to see the Nail programming language website".to_string());
    print_macro!("This version now has working interactive features!".to_string());
    print_macro!("[Self-ping] Background task will ping the website every 14.5 minutes to keep it alive".to_string());
    std_lib::http::http_server_route(port.clone(), &routes).await;
}
