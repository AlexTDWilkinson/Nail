use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;
use nail::std_lib::http::HTTP_Response;
use nail::std_lib::http::HTTP_Route;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() {
    let port: i64 = 8080;
    let site_title: String = "Nail - Write Code That Can't Go Wrong".to_string();
    let site_description: String = "Stop debugging. Start shipping. Nail eliminates entire categories of bugs by design.".to_string();
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct NavItem {
        name: String,
        path: String,
    }
    let nav_items: Vec<NavItem> = vec! [NavItem { name: "Home".to_string(),  path: "#home".to_string() }, NavItem { name: "Philosophy".to_string(),  path: "#philosophy".to_string() }, NavItem { name: "Features".to_string(),  path: "#features".to_string() }, NavItem { name: "Examples".to_string(),  path: "#examples".to_string() }, NavItem { name: "Documentation".to_string(),  path: "#docs".to_string() }, NavItem { name: "Getting Started".to_string(),  path: "#start".to_string() }];
    let error_example: String = std_lib::fs::read_file("examples/website_examples/simple_error.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let concurrent_example: String = std_lib::fs::read_file("examples/website_examples/simple_concurrent.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let parallel_example: String = std_lib::fs::read_file("examples/website_examples/simple_parallel.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let immutable_example: String = std_lib::fs::read_file("examples/website_examples/immutable_safety.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let basics_example: String = std_lib::fs::read_file("examples/website_examples/binding_values.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let greet_test: String = std_lib::fs::read_file("tests/test_website_greet_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let collections_test: String = std_lib::fs::read_file("tests/test_website_collections_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let factorial_test: String = std_lib::fs::read_file("tests/test_website_factorial_example.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    async fn factorial(num: i64) -> i64 {
        if num <= 1 {
            return 1;
        } else {
            return num * Box::pin(factorial(num - 1)).await;
        }
    }
    async fn is_prime(num: i64) -> bool {
        if num < 2 {
            return false;
        } else {
            let divisors: Vec<i64> = std_lib::array::array_range(2, num).await;
;
                        let has_divisor: bool = {
                let mut __any_match = false;
                for (_idx, div) in divisors.into_iter().enumerate() {
                    let condition_result = {
(num % div) == 0                    };
                    if condition_result {
                        __any_match = true;
                        break;
                    }
                }
                __any_match
            };
;
                        return !has_divisor;
        }
    }
    async fn count_primes_below(limit: i64) -> i64 {
        let candidates: Vec<i64> = std_lib::array::array_range(2, limit).await;
;
                let primes: Vec<i64> = {
            let __futures: Vec<_> = candidates.into_par_iter().enumerate().map(|(_idx, num)| {
                async move {
                    let condition_result = {
is_prime(num).await                    };
                    if condition_result {
                        Some(num.clone())
                    } else {
                        None
                    }
                }
            }).collect();
            let __results = future::join_all(__futures).await;
            let __result: Vec<_> = __results.into_iter().filter_map(|x| x).collect();
            __result
        };
;
                return std_lib::array::len(primes).await;
    }
    async fn divide(numerator: i64, denominator: i64) -> Result<i64, String> {
        if denominator == 0 {
            return Err(format!("[divide] {}", "Cannot divide by zero!".to_string()));
        } else {
            return Ok(numerator / denominator);
        }
    }
    async fn zero_fallback(err: String) -> i64 {
        return 0;
    }
    let (fact_12, sum_to_million, prime_count) = {
        let handle0 = std::thread::spawn({ let __rt_handle = tokio::runtime::Handle::current(); move || { __rt_handle.block_on(async move { factorial(12).await }) } });
        let handle1 = std::thread::spawn({ let __rt_handle = tokio::runtime::Handle::current(); move || { __rt_handle.block_on(async move { std_lib::array::sum(std_lib::array::array_range_inclusive(1, 1000000).await).await }) } });
        let handle2 = std::thread::spawn({ let __rt_handle = tokio::runtime::Handle::current(); move || { __rt_handle.block_on(async move { count_primes_below(10000).await }) } });
        (handle0.join().unwrap(), handle1.join().unwrap(), handle2.join().unwrap())
    };
    let (spec_text, readme_text, website_text) = tokio::join!(
        async { std_lib::fs::read_file("nail_language_spec.md".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error)) },
        async { std_lib::fs::read_file("README.md".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error)) },
        async { std_lib::fs::read_file("examples/nail_website.nail".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error)) }
    );
    let div_ok: i64 = divide(10, 2).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let div_fallback: i64 = match divide(10, 0).await { Ok(v) => v, Err(e) => (zero_fallback.clone())(e).await };
    let concurrent_example_html: String = std_lib::code::highlight_html(concurrent_example.clone()).await;
    let parallel_example_html: String = std_lib::code::highlight_html(parallel_example.clone()).await;
    let error_example_html: String = std_lib::code::highlight_html(error_example.clone()).await;
    let concurrent_rust_html: String = std_lib::code::escape_html(std_lib::code::transpile_to_rust(concurrent_example).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error))).await;
    let parallel_rust_html: String = std_lib::code::escape_html(std_lib::code::transpile_to_rust(parallel_example).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error))).await;
    let error_rust_html: String = std_lib::code::escape_html(std_lib::code::transpile_to_rust(error_example).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error))).await;
    let nav_links: Vec<String> = {
        let __futures: Vec<_> = nav_items.into_par_iter().enumerate().map(|(_idx, item)| {
            async move {
std_lib::array::join(vec! [r#"<a href=""#.to_string(), item.path.clone(), r#"" class="nav-link" hx-boost="true">"#.to_string(), item.name.clone(), "</a>".to_string()], "".to_string()).await
            }
        }).collect();
        let __result = future::join_all(__futures).await;
        __result
    };
    let nav_html: String = std_lib::array::join(nav_links, "".to_string()).await;
    async fn generate_css() -> String {
        let base_styles: String = r#"
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
    }"#.to_string();
;
                let navbar_styles: String = r#"
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
    }
    
    .nav-link:hover {
        color: var(--primary);
    }"#.to_string();
;
                let hero_styles: String = r#"
    .hero {
        max-width: 1200px;
        margin: 0 auto;
        padding: 6rem 2rem 4rem;
        text-align: center;
    }
    
    .hero-title {
        font-size: 4rem;
        font-weight: 800;
        margin-bottom: 1rem;
        background: linear-gradient(135deg, var(--primary) 0%, var(--primary-dark) 100%);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        animation: fadeIn 0.6s ease-out;
    }
    
    .hero-tagline {
        font-size: 1.5rem;
        color: var(--text-secondary);
        margin-bottom: 2rem;
        animation: fadeIn 0.8s ease-out;
    }
    
    .hero-description {
        font-size: 1.1rem;
        color: var(--text-secondary);
        max-width: 800px;
        margin: 0 auto 2rem;
        line-height: 1.8;
        animation: fadeIn 1s ease-out;
    }
    
    .hero-actions {
        display: flex;
        gap: 1rem;
        justify-content: center;
        animation: fadeIn 1.2s ease-out;
    }"#.to_string();
;
                let button_styles: String = r#"
    .btn {
        padding: 0.75rem 2rem;
        border-radius: 0.5rem;
        font-weight: 600;
        text-decoration: none;
        transition: all 0.2s;
        display: inline-block;
        border: 2px solid transparent;
    }
    
    .btn-primary {
        background: var(--primary);
        color: white;
    }
    
    .btn-primary:hover {
        background: var(--primary-dark);
        transform: translateY(-2px);
        box-shadow: 0 10px 20px rgba(37, 99, 235, 0.2);
    }
    
    .btn-secondary {
        background: white;
        color: var(--primary);
        border-color: var(--primary);
    }
    
    .btn-secondary:hover {
        background: var(--bg-secondary);
        transform: translateY(-2px);
    }"#.to_string();
;
                let section_styles: String = r#"
    section {
        max-width: 1200px;
        margin: 0 auto;
        padding: 4rem 2rem;
    }
    
    .section-header {
        text-align: center;
        margin-bottom: 3rem;
    }
    
    .section-header h2 {
        font-size: 2.5rem;
        font-weight: 700;
        color: var(--text-primary);
        margin-bottom: 1rem;
    }
    
    .section-header p {
        font-size: 1.2rem;
        color: var(--text-secondary);
    }"#.to_string();
;
                let feature_styles: String = r#"
    .features-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
        gap: 2rem;
        margin-top: 3rem;
    }
    
    .feature-card {
        background: var(--bg-primary);
        padding: 2rem;
        border-radius: 1rem;
        box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        transition: all 0.3s;
    }
    
    .feature-card:hover {
        transform: translateY(-4px);
        box-shadow: 0 10px 30px rgba(0,0,0,0.1);
    }
    
    .feature-icon {
        font-size: 2.5rem;
        margin-bottom: 1rem;
    }
    
    .feature-card h3 {
        font-size: 1.3rem;
        margin-bottom: 0.5rem;
        color: var(--text-primary);
    }
    
    .feature-card p {
        color: var(--text-secondary);
        line-height: 1.6;
    }"#.to_string();
;
                let code_styles: String = r#"
    .code-example {
        background: #1e293b;
        color: #e2e8f0;
        padding: 2rem;
        border-radius: 0.75rem;
        overflow-x: auto;
        margin: 2rem 0;
        position: relative;
    }
    
    .code-example pre {
        margin: 0;
        font-family: 'Cascadia Code', 'Fira Code', monospace;
        font-size: 0.95rem;
        line-height: 1.6;
    }
    
    .code-label {
        position: absolute;
        top: 0.5rem;
        right: 0.5rem;
        background: rgba(255,255,255,0.1);
        color: #94a3b8;
        padding: 0.25rem 0.75rem;
        border-radius: 0.25rem;
        font-size: 0.85rem;
    }

    /* Token colors emitted by code_highlight_html (Nail's own lexer) */
    .tok-kw { color: #c084fc; }
    .tok-cb { color: #f472b6; font-weight: 600; }
    .tok-fn { color: #60a5fa; }
    .tok-ty { color: #2dd4bf; }
    .tok-str { color: #86efac; }
    .tok-num { color: #fbbf24; }
    .tok-com { color: #64748b; font-style: italic; }
    .tok-op { color: #cbd5e1; }
    .tok-err { color: #ef4444; text-decoration: underline wavy; }

    .rust-details summary {
        cursor: pointer;
        color: var(--primary);
        font-weight: 600;
        margin-top: 1rem;
    }

    .rust-details .code-example {
        margin-top: 0.5rem;
        max-height: 420px;
        overflow-y: auto;
    }"#.to_string();
;
                let philosophy_styles: String = r#"
    .philosophy-content {
        max-width: 800px;
        margin: 0 auto;
        font-size: 1.1rem;
        line-height: 1.8;
        color: var(--text-secondary);
    }
    
    .philosophy-quote {
        background: var(--bg-primary);
        border-left: 4px solid var(--primary);
        padding: 1.5rem 2rem;
        margin: 2rem 0;
        font-style: italic;
        font-size: 1.2rem;
        color: var(--text-primary);
    }"#.to_string();
;
                let animation_styles: String = r#"
    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(-10px); }
        to { opacity: 1; transform: translateY(0); }
    }"#.to_string();
;
                return std_lib::array::join(vec! [base_styles, navbar_styles, hero_styles, button_styles, section_styles, feature_styles, code_styles, philosophy_styles, animation_styles], "".to_string()).await;
    }
    async fn generate_head(site_title: String, site_description: String) -> String {
        let head_html: String = std_lib::array::join(vec! [r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content=""#.to_string(), site_description, r#"">
    <title>"#.to_string(), site_title, r#"</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <script src="https://unpkg.com/htmx.org/dist/ext/ws.js"></script>
    <style>"#.to_string(), generate_css().await, r#"</style>
</head>"#.to_string()], "".to_string()).await;
;
                return head_html;
    }
    async fn generate_navbar(nav_html: String) -> String {
        let navbar_html: String = std_lib::array::join(vec! [r##"<nav class="navbar">
    <div class="nav-container">
        <a href="#home" class="nav-brand">
            <span style="font-size: 1.5rem;">🔨</span>
            <span>Nail</span>
        </a>
        <div class="nav-links">"##.to_string(), nav_html, r#"</div>
    </div>
</nav>"#.to_string()], "".to_string()).await;
;
                return navbar_html;
    }
    async fn generate_hero() -> String {
        return r##"<section id="home" class="hero">
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
</section>"##.to_string();
    }
    async fn generate_philosophy() -> String {
        return r#"<section id="philosophy" class="philosophy">
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
</section>"#.to_string();
    }
    async fn generate_problem_section() -> String {
        return r#"<section class="problems" style="padding: 4rem 2rem; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
    <div style="max-width: 1200px; margin: 0 auto; color: white;">
        <h2 style="font-size: 2.5rem; margin-bottom: 3rem; text-align: center;">The Problem With Modern Languages</h2>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 2rem; margin-bottom: 3rem;">
            <div style="text-align: center;">
                <div style="font-size: 3rem; margin-bottom: 1rem;">NULL</div>
                <p>The "billion dollar mistake" - still causing crashes today</p>
            </div>
            <div style="text-align: center;">
                <div style="font-size: 3rem; margin-bottom: 1rem;">LOOPS</div>
                <p>Off-by-one errors, iterator invalidation, infinite loops</p>
            </div>
            <div style="text-align: center;">
                <div style="font-size: 3rem; margin-bottom: 1rem;">RACES</div>
                <p>Data races, deadlocks, and synchronization nightmares</p>
            </div>
        </div>
        <div style="text-align: center; padding: 2rem; background: rgba(255,255,255,0.1); border-radius: 1rem;">
            <h3 style="font-size: 1.8rem; margin-bottom: 1rem;">Nail's Solution</h3>
            <p style="font-size: 1.2rem; max-width: 800px; margin: 0 auto;">
                We don't add features to work around problems. We remove the features that cause problems.
                No null. Functional iteration with map/filter/reduce. Immutable by default. Simple.
            </p>
        </div>
    </div>
</section>"#.to_string();
    }
    async fn generate_features() -> String {
        let features_data: Vec<String> = vec! [r#"<div class="feature-card">
            <div class="feature-icon">🔒</div>
            <h3>Immutable by Default</h3>
            <p>All values are constants. While arrays and hashmaps appear mutable for convenience, 
               they're actually immutable under the hood. This eliminates race conditions and 
               unexpected state changes.</p>
        </div>"#.to_string(), r#"<div class="feature-card">
            <div class="feature-icon">🔄</div>
            <h3>Functional Collections</h3>
            <p>map, filter, reduce, each, find, all and any are language keywords, not library
               methods. Iteration reads as intent — transform, keep, combine — instead of index
               bookkeeping, eliminating off-by-one bugs. The syntax is designed for readability,
               not terseness.</p>
        </div>"#.to_string(), r#"<div class="feature-card">
            <div class="feature-icon">⚡</div>
            <h3>Concurrent & Parallel Blocks</h3>
            <p>Use c.../c for concurrent I/O operations (async with tokio::join!) or p.../p for 
               CPU-intensive parallel work (OS threads). No locks, no race conditions, just simple 
               concurrent and parallel programming.</p>
        </div>"#.to_string(), r#"<div class="feature-card">
            <div class="feature-icon">🔀</div>
            <h3>No Silent Failures</h3>
            <p>Every error must be handled explicitly. Use safe() with a fallback or danger() 
               to acknowledge risk. Nail won't let you ignore errors—eliminating an entire class of production bugs.</p>
        </div>"#.to_string(), r#"<div class="feature-card">
            <div class="feature-icon">🛡️</div>
            <h3>Zero Overhead</h3>
            <p>Simple doesn't mean slow. Nail compiles to optimized Rust that rivals C++ performance. 
               Automatic parallelization and zero-cost abstractions mean your code is both simple AND fast.</p>
        </div>"#.to_string(), r#"<div class="feature-card">
            <div class="feature-icon">🦀</div>
            <h3>Production Ready</h3>
            <p>Built on Rust's proven foundation. Deploy anywhere Rust runs—from embedded systems 
               to web servers. This website? It's running on Nail-generated code right now.</p>
        </div>"#.to_string()];
;
                let features_html: String = std_lib::array::join(features_data, "".to_string()).await;
;
                let section_html: String = std_lib::array::join(vec! [r#"<section id="features" class="features">
    <div class="section-header">
        <h2>Key Features</h2>
        <p>Every feature in Nail is designed to eliminate entire categories of bugs</p>
    </div>
    <div class="features-grid">"#.to_string(), features_html, r#"
    </div>
</section>"#.to_string()], "".to_string()).await;
;
                return section_html;
    }
    async fn generate_examples(concurrent_example: String, parallel_example: String, error_example: String, concurrent_rust: String, parallel_rust: String, error_rust: String) -> String {
        let examples_html: String = std_lib::array::join(vec! [r#"<section id="examples" class="examples">
    <div class="section-header">
        <h2>Code Examples</h2>
        <p>See how Nail makes complex tasks simple and safe. Every ▶ Run result below is real —
           this server (itself a Nail program) executed these examples and serves what they produced.</p>
        <p style="font-size: 1rem; margin-top: 0.75rem;">
           Two more things this page does with Nail itself: the syntax highlighting is not a JavaScript
           library — the server runs <strong>Nail's own lexer</strong> over each example and emits colored
           HTML. And each "View the generated Rust" panel is produced by the <strong>actual Nail
           compiler</strong> transpiling the example when the server boots, so what you see is exactly
           what your code becomes.</p>
    </div>
    
    <div style="display: grid; gap: 2rem;">
        <div>
            <h3 style="margin-bottom: 1rem;">Concurrent I/O Operations (c.../c)</h3>
            <div class="code-example" style="position: relative;">
                <span class="code-label">Nail</span>
                <button onclick="fetch('/run-example?name=concurrent').then(r=>r.json()).then(d=>{let el=document.getElementById('concurrent-output');el.style.display='block';el.textContent=d.output})" style="position: absolute; bottom: 1rem; right: 1rem; background: #10b981; color: white; border: none; padding: 0.5rem 1rem; border-radius: 0.25rem; cursor: pointer; font-weight: 600;">▶ Run</button>
                <pre>"#.to_string(), concurrent_example, r#"</pre>
            </div>
            <details class="rust-details">
                <summary>🦀 View the generated Rust — transpiled live by this server's compiler</summary>
                <div class="code-example"><span class="code-label">Generated Rust</span><pre>"#.to_string(), concurrent_rust, r#"</pre></div>
            </details>
            <pre id="concurrent-output" style="display: none; background: #1a1a1a; color: #10b981; padding: 1rem; border-radius: 0.5rem; margin-top: 1rem; font-family: monospace;"></pre>
            <p style="color: var(--text-secondary); margin-top: 1rem;">
                <strong>How it works:</strong> every statement inside c.../c starts at the same time,
                and the program continues only when all of them have finished. The compiler turns the
                block into Rust's tokio::join! — real async I/O with zero setup.
                <strong>Why it matters:</strong> in JavaScript this is a hand-built Promise.all; in Go
                it's goroutines plus a WaitGroup; in Nail, putting the statements inside the block
                <em>is</em> the whole job. After /c the results are ordinary immutable values — no
                callbacks, no forgotten awaits, nothing left to race.
            </p>
        </div>
        
        <div>
            <h3 style="margin-bottom: 1rem;">Parallel CPU Work (p.../p)</h3>
            <div class="code-example" style="position: relative;">
                <span class="code-label">Nail</span>
                <button onclick="fetch('/run-example?name=parallel').then(r=>r.json()).then(d=>{let el=document.getElementById('parallel-output');el.style.display='block';el.textContent=d.output})" style="position: absolute; bottom: 1rem; right: 1rem; background: #10b981; color: white; border: none; padding: 0.5rem 1rem; border-radius: 0.25rem; cursor: pointer; font-weight: 600;">▶ Run</button>
                <pre>"#.to_string(), parallel_example, r#"</pre>
            </div>
            <details class="rust-details">
                <summary>🦀 View the generated Rust — see the std::thread::spawn and join barrier</summary>
                <div class="code-example"><span class="code-label">Generated Rust</span><pre>"#.to_string(), parallel_rust, r#"</pre></div>
            </details>
            <pre id="parallel-output" style="display: none; background: #1a1a1a; color: #10b981; padding: 1rem; border-radius: 0.5rem; margin-top: 1rem; font-family: monospace;"></pre>
            <p style="color: var(--text-secondary); margin-top: 1rem;">
                <strong>How it works:</strong> every statement inside p.../p runs on its own OS thread
                (std::thread::spawn) and all threads are joined at /p — true multi-core execution for
                CPU-heavy work, with every value guaranteed ready after the block.
                <strong>Why it matters:</strong> Nail needs no locks or mutexes because all values are
                immutable — threads simply cannot write over each other's data. Rule of thumb: use
                c blocks to wait on the outside world (files, network, databases) and p blocks to
                burn CPU (math, parsing, data crunching).
            </p>
        </div>
        
        <div>
            <h3 style="margin-bottom: 1rem;">Error Handling Done Right</h3>
            <div class="code-example" style="position: relative;">
                <span class="code-label">Nail</span>
                <button onclick="fetch('/run-example?name=error').then(r=>r.json()).then(d=>{let el=document.getElementById('error-output');el.style.display='block';el.textContent=d.output})" style="position: absolute; bottom: 1rem; right: 1rem; background: #10b981; color: white; border: none; padding: 0.5rem 1rem; border-radius: 0.25rem; cursor: pointer; font-weight: 600;">▶ Run</button>
                <pre>"#.to_string(), error_example, r#"</pre>
            </div>
            <details class="rust-details">
                <summary>🦀 View the generated Rust — Result types and explicit handling</summary>
                <div class="code-example"><span class="code-label">Generated Rust</span><pre>"#.to_string(), error_rust, r#"</pre></div>
            </details>
            <pre id="error-output" style="display: none; background: #1a1a1a; color: #10b981; padding: 1rem; border-radius: 0.5rem; margin-top: 1rem; font-family: monospace;"></pre>
            <p style="color: var(--text-secondary); margin-top: 1rem;">
                <strong>How it works:</strong> divide returns i!e — "an integer or an error". Nail
                will not compile code that ignores the error half: you either handle it with safe()
                and a fallback function, or accept a crash explicitly with danger().
                <strong>Why it matters:</strong> there is no null, no unchecked exception, no silently
                swallowed failure — every error path is visible in the code and enforced by the
                compiler, so a whole class of production bugs can't exist.
            </p>
        </div>
    </div>
</section>"#.to_string()], "".to_string()).await;
;
                return examples_html;
    }
    async fn generate_footer() -> String {
        return r#"<footer style="background: var(--text-primary); color: white; padding: 3rem 2rem; margin-top: 4rem;">
    <div style="max-width: 1200px; margin: 0 auto; text-align: center;">
        <p style="font-size: 1.1rem; margin-bottom: 1rem;">
            Made with 🔨 by developers who care about simplicity
        </p>
        <div style="display: flex; gap: 2rem; justify-content: center; margin-top: 1.5rem;">
            <a href="https://github.com/AlexTDWilkinson/Nail" target="_blank" style="color: white; text-decoration: none;">
                GitHub
            </a>
            <a href="https://github.com/AlexTDWilkinson/Nail/blob/main/nail_language_spec.md" target="_blank" style="color: white; text-decoration: none;">
                Documentation
            </a>
        </div>
    </div>
</footer>"#.to_string();
    }
    let website_html: String = std_lib::array::join(vec! [generate_head(site_title, site_description).await, r#"<body hx-boost="true">"#.to_string(), generate_navbar(nav_html).await, generate_hero().await, generate_philosophy().await, generate_problem_section().await, generate_features().await, generate_examples(concurrent_example_html, parallel_example_html, error_example_html, concurrent_rust_html, parallel_rust_html, error_rust_html).await, generate_footer().await, r#"</body>
</html>"#.to_string()], "".to_string()).await;
    let routes: DashMap<String, HTTP_Route> = std_lib::hashmap::new().await;
    let main_route: HTTP_Route = nail::std_lib::http::HTTP_Route { path: "/".to_string(),  content: website_html,  content_type: "text/html; charset=utf-8".to_string(),  status_code: 200 };
    std_lib::hashmap::insert(&routes, "/".to_string(), main_route).await;
    let run_note: String = r#"\n\n(Real output — computed by this server, itself a Nail program, by running the code above.)"#.to_string();
    let concurrent_output: String = std_lib::array::join(vec! [r#"{"output": "Language spec chars: "#.to_string(), std_lib::string::from(std_lib::string::len(spec_text.clone()).await).await, r#"\nREADME chars: "#.to_string(), std_lib::string::from(std_lib::string::len(readme_text.clone()).await).await, r#"\nWebsite source chars: "#.to_string(), std_lib::string::from(std_lib::string::len(website_text.clone()).await).await, r#"\nAll three files loaded concurrently!"#.to_string(), run_note.clone(), r#""}"#.to_string()], "".to_string()).await;
    let concurrent_route: HTTP_Route = nail::std_lib::http::HTTP_Route { path: "/run-example?name=concurrent".to_string(),  content: concurrent_output,  content_type: "application/json".to_string(),  status_code: 200 };
    std_lib::hashmap::insert(&routes, "/run-example?name=concurrent".to_string(), concurrent_route).await;
    let parallel_output: String = std_lib::array::join(vec! [r#"{"output": "12! = "#.to_string(), std_lib::string::from(fact_12.clone()).await, r#"\nSum of 1 to 1,000,000 = "#.to_string(), std_lib::string::from(sum_to_million.clone()).await, r#"\nPrimes below 10,000 = "#.to_string(), std_lib::string::from(prime_count.clone()).await, run_note.clone(), r#""}"#.to_string()], "".to_string()).await;
    let parallel_route: HTTP_Route = nail::std_lib::http::HTTP_Route { path: "/run-example?name=parallel".to_string(),  content: parallel_output,  content_type: "application/json".to_string(),  status_code: 200 };
    std_lib::hashmap::insert(&routes, "/run-example?name=parallel".to_string(), parallel_route).await;
    let error_output: String = std_lib::array::join(vec! [r#"{"output": "10 / 2 = "#.to_string(), std_lib::string::from(div_ok).await, r#"\nError occurred: Cannot divide by zero!\nResult with error handling: "#.to_string(), std_lib::string::from(div_fallback).await, run_note, r#""}"#.to_string()], "".to_string()).await;
    let error_route: HTTP_Route = nail::std_lib::http::HTTP_Route { path: "/run-example?name=error".to_string(),  content: error_output,  content_type: "application/json".to_string(),  status_code: 200 };
    std_lib::hashmap::insert(&routes, "/run-example?name=error".to_string(), error_route).await;
    tokio::spawn(async move {
        std_lib::time::sleep(30.0).await;
;
                loop {
            let ping_response: HTTP_Response = std_lib::http::http_request("GET".to_string(), "https://nail-idtq.onrender.com".to_string(), std_lib::hashmap::new().await, "".to_string()).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
;
                        print_macro!("[Self-ping] Successfully pinged server to keep it warm, status: ".to_string(), ping_response.status.clone());
;
                        std_lib::time::sleep(870.0).await;
        }
    });
    std_lib::http::http_server(port, routes.clone()).await;
    print_macro!("Server running on http://localhost:".to_string(), port);
}
