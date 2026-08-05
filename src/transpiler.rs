use crate::common::{CodeError, CodeSpan};
use crate::lexer::{NailDataTypeDescriptor, Operation};
use crate::parser::ASTNode;
use crate::stdlib_registry::{self, CrateDependency, StructDerive};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Identifies one binding site (declaration, param, or binder) within a single
/// MoveContext. Shadowing declarations that reuse a name get distinct ids, so
/// move analysis never conflates two bindings that share a name.
type BindingId = usize;

/// Per-function (or per-program) bookkeeping that lets identifier references
/// transpile to moves instead of clones. Nail is fully immutable, so replacing
/// a clone with a move can never change behavior — a wrong decision here fails
/// Rust compilation loudly rather than miscompiling silently.
struct MoveContext {
    /// Remaining syntactic uses of each binding, in emission order. A use that
    /// is emitted outside the Identifier arm (struct field access, assignment
    /// target) is counted but never decremented, which pins the count above
    /// zero and keeps the binding clone-only.
    remaining: HashMap<BindingId, usize>,
    /// Bindings declared in THIS context (declarations and params). Only
    /// activated bindings may be moved.
    activated: HashSet<BindingId>,
    /// Bindings referenced anywhere execution can revisit the reference
    /// (loop bodies, closure bodies, parallel blocks) — never moved.
    never_move: HashSet<BindingId>,
    /// Declared type of each activated binding, used to emit Copy types
    /// (i/f/b) bare with no clone. Exact even when a name is shadowed by a
    /// differently-typed binding.
    binding_types: HashMap<BindingId, NailDataTypeDescriptor>,
    /// Name-keyed types for binders (iterators, indexes) recorded mid-emission,
    /// and the fallback when a reference has no resolved binding. Updated in
    /// emission order, so it tracks the binding currently in scope.
    types: HashMap<String, NailDataTypeDescriptor>,
    /// Maps each identifier reference and declaration site, keyed by
    /// (code span, name), to the binding it names. Built by BindingResolver
    /// before emission starts.
    resolution: HashMap<(CodeSpan, String), BindingId>,
    /// Binding ids of this context's params, which have no declaration node.
    param_bindings: HashMap<String, BindingId>,
}

/// Assigns a distinct BindingId to every binding site inside one move context
/// and resolves every identifier reference to the binding it names. Mirrors
/// the emitter's scoping: blocks and binder constructs open scopes, shadowing
/// declarations get fresh ids, and parallel/concurrent block declarations bind
/// in the enclosing scope (the emitter hoists them into a tuple `let`).
/// A reference that resolves to nothing simply gets no map entry, which the
/// move analysis treats as clone-only — the safe default.
struct BindingResolver {
    scopes: Vec<HashMap<String, BindingId>>,
    next_id: BindingId,
    resolution: HashMap<(CodeSpan, String), BindingId>,
}

impl BindingResolver {
    fn new() -> Self {
        BindingResolver { scopes: vec![HashMap::new()], next_id: 0, resolution: HashMap::new() }
    }

    fn bind(&mut self, name: &str) -> BindingId {
        let id = self.next_id;
        self.next_id += 1;
        self.scopes.last_mut().expect("resolver always has a root scope").insert(name.to_string(), id);
        id
    }

    fn resolve(&self, name: &str) -> Option<BindingId> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name)).copied()
    }

    fn record(&mut self, code_span: &CodeSpan, name: &str) {
        if let Some(id) = self.resolve(name) {
            self.resolution.insert((code_span.clone(), name.to_string()), id);
        }
    }

    fn scoped(&mut self, walk_inner: impl FnOnce(&mut Self)) {
        self.scopes.push(HashMap::new());
        walk_inner(self);
        self.scopes.pop();
    }

    fn walk(&mut self, node: &ASTNode) {
        match node {
            ASTNode::Identifier { name, code_span, .. } => self.record(code_span, name),
            ASTNode::StructFieldAccess { struct_name, code_span, .. } => self.record(code_span, struct_name),
            ASTNode::ConstDeclaration { name, value, code_span, .. } => {
                // The value refers to the previous binding of the name; the new
                // binding exists only after it.
                self.walk(value);
                let id = self.bind(name);
                self.resolution.insert((code_span.clone(), name.clone()), id);
            }
            // Function bodies get their own move context and resolver.
            ASTNode::FunctionDeclaration { .. } => {}
            // Lambda bodies also get their own context, but this walk still
            // resolves their free references so collect_never_move can pin the
            // captured outer bindings. Params shadow outer names.
            ASTNode::LambdaDeclaration { params, body, .. } => {
                self.scoped(|resolver| {
                    for (param_name, _) in params {
                        resolver.bind(param_name);
                    }
                    resolver.walk(body);
                });
            }
            ASTNode::Block { statements, .. } => {
                self.scoped(|resolver| {
                    for statement in statements {
                        resolver.walk(statement);
                    }
                });
            }
            ASTNode::ForLoop { iterator, iterable, initial_value, filter, body, .. } => {
                self.walk(iterable);
                if let Some(initial_value) = initial_value {
                    self.walk(initial_value);
                }
                self.scoped(|resolver| {
                    resolver.bind(iterator);
                    if let Some(filter) = filter {
                        resolver.walk(filter);
                    }
                    resolver.walk(body);
                });
            }
            ASTNode::MapExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::FilterExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::EachExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::FindExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::AllExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::AnyExpression { iterator, index_iterator, iterable, body, .. } => {
                self.walk(iterable);
                self.scoped(|resolver| {
                    resolver.bind(iterator);
                    if let Some(index_iterator) = index_iterator {
                        resolver.bind(index_iterator);
                    }
                    resolver.walk(body);
                });
            }
            ASTNode::ReduceExpression { accumulator, iterator, index_iterator, iterable, initial_value, body, .. } | ASTNode::ScanExpression { accumulator, iterator, index_iterator, iterable, initial_value, body, .. } => {
                self.walk(iterable);
                self.walk(initial_value);
                self.scoped(|resolver| {
                    resolver.bind(accumulator);
                    resolver.bind(iterator);
                    if let Some(index_iterator) = index_iterator {
                        resolver.bind(index_iterator);
                    }
                    resolver.walk(body);
                });
            }
            ASTNode::Loop { index_iterator, body, .. } => {
                self.scoped(|resolver| {
                    if let Some(index_iterator) = index_iterator {
                        resolver.bind(index_iterator);
                    }
                    resolver.walk(body);
                });
            }
            _ => {
                for child in Transpiler::ast_children(node) {
                    self.walk(child);
                }
            }
        }
    }
}

/// A fold step whose result does not depend on how the work is grouped.
///
/// All four are over i64, where they are associative and commutative: no
/// grouping and no ordering of the elements can change the answer, which is
/// exactly what splitting the fold across threads changes.
#[derive(Clone, Copy)]
enum AssociativeStep {
    Add,
    Mul,
    Min,
    Max,
}

impl AssociativeStep {
    /// The value an exhausted fold produces, so that an empty chunk of a
    /// parallel reduction contributes nothing to the result.
    fn identity(self) -> &'static str {
        match self {
            AssociativeStep::Add => "0i64",
            AssociativeStep::Mul => "1i64",
            AssociativeStep::Min => "i64::MAX",
            AssociativeStep::Max => "i64::MIN",
        }
    }

    /// The step itself, applied to two i64 expressions.
    fn combine(self, left: &str, right: &str) -> String {
        match self {
            AssociativeStep::Add => format!("{} + {}", left, right),
            AssociativeStep::Mul => format!("{} * {}", left, right),
            AssociativeStep::Min => format!("std::cmp::min({}, {})", left, right),
            AssociativeStep::Max => format!("std::cmp::max({}, {})", left, right),
        }
    }
}

/// A fold proven safe to evaluate in any grouping: how two values combine, and
/// what each element contributes to the fold.
struct AssociativeFold<'a> {
    step: AssociativeStep,
    /// The whole number an element contributes, when the step folds in
    /// something computed from the element - `acc + num * num` contributes
    /// `num * num`. `None` when the element is folded in as it stands.
    element_value: Option<&'a ASTNode>,
}

#[derive(Clone, Copy, PartialEq)]
enum ParallelIterMode {
    Sync,
    AsyncPure,
    AsyncBlockOn,
}

pub struct Transpiler {
    indent_level: usize,
    scope_level: usize,
    current_function_return_type: Option<NailDataTypeDescriptor>,
    current_function_name: Option<String>,
    used_stdlib_functions: HashSet<String>,
    in_collection_operation: bool,
    stdlib_types: HashMap<String, String>,  // Maps stdlib type names to their full paths
    move_contexts: Vec<MoveContext>,
    // Names that appear as assignment targets anywhere in the program; their
    // declarations need `let mut`. Name-based, so a same-named variable in
    // another scope gets a harmless unused_mut at worst.
    reassigned_variables: HashSet<String>,
    // User functions whose bodies (transitively) perform no async work. These
    // emit as plain sync Rust fns: no .await at call sites, no Box::pin for
    // recursion, and collection operations over them use plain rayon closures.
    pure_functions: HashSet<String>,
    // True while emitting the body of a pure (sync) function.
    in_sync_function: bool,
    // True while emitting inside a rayon closure that is not itself wrapped in
    // block_on. Nothing there may await, so a collection operation nested in
    // one has to emit as a plain sync rayon chain.
    in_parallel_closure: bool,
    // Saved values of in_parallel_closure, one per open parallel operation.
    enclosing_parallel_closures: Vec<bool>,
    // struct name -> field name -> declared type, from struct declarations;
    // used to skip .clone() on Copy-typed field access.
    struct_field_types: HashMap<String, HashMap<String, NailDataTypeDescriptor>>,
    // User function name -> declared return type, from function declarations.
    function_return_types: HashMap<String, NailDataTypeDescriptor>,
    // Whether to emit runtime profiling (a drop guard per user function, an
    // init call and a finish call in main). nailc and the IDE turn this on,
    // making it the default for every build a user runs. It stays off here so
    // library callers like code_transpile emit uninstrumented programs.
    pub profile: bool,
    // Fingerprint of the source this program was built from, embedded in the
    // profile dump so the IDE can dim timings the moment the code on screen
    // no longer matches the run. Set by whoever holds the source text.
    pub profile_source_hash: Option<String>,
    // Whether the program is being built for a browser. Changes the main
    // scaffold (wasm-bindgen start instead of a tokio runtime), forces every
    // collection operation onto the plain sync chain (a browser has one
    // thread), and drives the wasm variant of the generated Cargo.toml.
    pub wasm_target: bool,
    // User function name -> profiling id, its index into the init name list.
    // Source order, assigned once per transpile run.
    profile_fn_ids: HashMap<String, usize>,
    profile_fn_names: Vec<String>,
}

impl Transpiler {
    pub fn new() -> Self {
        Transpiler {
            indent_level: 0,
            scope_level: 0,
            current_function_return_type: None,
            current_function_name: None,
            used_stdlib_functions: HashSet::new(),
            in_collection_operation: false,
            stdlib_types: HashMap::new(),
            move_contexts: Vec::new(),
            reassigned_variables: HashSet::new(),
            pure_functions: HashSet::new(),
            in_sync_function: false,
            in_parallel_closure: false,
            enclosing_parallel_closures: Vec::new(),
            struct_field_types: HashMap::new(),
            function_return_types: HashMap::new(),
            profile: false,
            profile_source_hash: None,
            wasm_target: false,
            profile_fn_ids: HashMap::new(),
            profile_fn_names: Vec::new(),
        }
    }

    /// Collect field types of every struct declaration in the program.
    fn collect_struct_field_types(node: &ASTNode, out: &mut HashMap<String, HashMap<String, NailDataTypeDescriptor>>) {
        if let ASTNode::StructDeclaration { name, fields, .. } = node {
            let mut field_map = HashMap::new();
            for field in fields {
                if let ASTNode::StructDeclarationField { name: field_name, data_type, .. } = field {
                    field_map.insert(field_name.clone(), data_type.clone());
                }
            }
            out.insert(name.clone(), field_map);
        }
        for child in Self::ast_children(node) {
            Self::collect_struct_field_types(child, out);
        }
    }

    /// Whether evaluating this node requires an async context, given the
    /// current optimistic set of pure user functions. Function and lambda
    /// declarations don't propagate: their bodies run in their own context.
    fn node_needs_async(node: &ASTNode, pure: &HashSet<String>) -> bool {
        match node {
            ASTNode::ParallelBlock { .. } | ASTNode::ConcurrentBlock { .. } | ASTNode::SpawnBlock { .. } => true,
            ASTNode::FunctionDeclaration { .. } | ASTNode::LambdaDeclaration { .. } => false,
            ASTNode::FunctionCall { name, args, .. } => {
                let callee_async = if name == "safe" {
                    // safe(expr, handler): the handler is invoked at the call
                    // site — a lambda handler is an async closure, a named
                    // handler follows its own purity.
                    match args.get(1) {
                        Some(ASTNode::Identifier { name: handler, .. }) => !pure.contains(handler),
                        _ => true,
                    }
                } else if let Some(stdlib_fn) = stdlib_registry::get_stdlib_function(name) {
                    // A key function is run before the call, in a loop this
                    // emits; if that function is async, so is the call.
                    let key_is_async = stdlib_registry::precomputes_key_argument(name)
                        && matches!(args.get(1), Some(ASTNode::Identifier { name: key, .. }) if !pure.contains(key));
                    // A file fold is emitted as a loop that reads the file, so it
                    // is always awaited whatever its step function does.
                    let folds_a_file = stdlib_registry::file_fold(name).is_some();
                    key_is_async || folds_a_file || (!stdlib_fn.rust_path.ends_with('!') && stdlib_registry::is_stdlib_fn_async(name))
                } else {
                    !pure.contains(name)
                };
                callee_async || args.iter().any(|arg| Self::node_needs_async(arg, pure))
            }
            _ => Self::ast_children(node).into_iter().any(|child| Self::node_needs_async(child, pure)),
        }
    }

    /// Collect the declared return type of every function declaration.
    fn collect_function_return_types(node: &ASTNode, out: &mut HashMap<String, NailDataTypeDescriptor>) {
        if let ASTNode::FunctionDeclaration { name, data_type, .. } = node {
            out.insert(name.clone(), data_type.clone());
        }
        for child in Self::ast_children(node) {
            Self::collect_function_return_types(child, out);
        }
    }

    /// Collect every function declaration (at any nesting level) as (name, body).
    fn collect_function_declarations<'a>(node: &'a ASTNode, out: &mut Vec<(String, &'a ASTNode)>) {
        if let ASTNode::FunctionDeclaration { name, body, .. } = node {
            out.push((name.clone(), body.as_ref()));
        }
        for child in Self::ast_children(node) {
            Self::collect_function_declarations(child, out);
        }
    }

    /// Fixpoint purity analysis: start assuming every user function is pure,
    /// then repeatedly demote any whose body needs async under the current
    /// assumption, until stable.
    ///
    /// Visible to the checker as well as the emitter: whether a function ends up
    /// async is a property of code generation, and the checker has to know it to
    /// reject a program that passes an async function where a stdlib function
    /// will call it synchronously.
    pub(crate) fn compute_pure_functions(program: &ASTNode) -> HashSet<String> {
        let mut declarations = Vec::new();
        Self::collect_function_declarations(program, &mut declarations);
        // A function a stdlib callback dispatches to is invoked from async glue
        // that always awaits it, so it must be emitted async however pure its
        // body happens to be.
        let mut pure: HashSet<String> = declarations.iter().map(|(name, _)| name.clone()).filter(|name| !stdlib_registry::is_handler_callback_target(name)).collect();
        loop {
            let mut changed = false;
            for (name, body) in &declarations {
                if pure.contains(name) && Self::node_needs_async(body, &pure) {
                    pure.remove(name);
                    changed = true;
                }
            }
            if !changed {
                return pure;
            }
        }
    }

    /// Collects every name used as an assignment target so declarations can
    /// be emitted with `let mut`.
    fn collect_reassigned_variables(node: &ASTNode, out: &mut HashSet<String>) {
        if let ASTNode::Assignment { left, .. } = node {
            if let ASTNode::Identifier { name, .. } = left.as_ref() {
                out.insert(name.clone());
            }
        }
        for child in Self::ast_children(node) {
            Self::collect_reassigned_variables(child, out);
        }
    }

    fn has_return_statements(&self, node: &ASTNode) -> bool {
        match node {
            ASTNode::ReturnDeclaration { .. } => true,
            ASTNode::YieldDeclaration { .. } => true,
            ASTNode::Block { statements, .. } => {
                statements.iter().any(|stmt| self.has_return_statements(stmt))
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                condition_branches.iter().any(|(_, branch)| self.has_return_statements(branch)) ||
                else_branch.as_ref().map_or(false, |branch| self.has_return_statements(branch))
            }
            _ => false,
        }
    }
    

    pub fn get_required_dependencies(&self) -> HashSet<CrateDependency> {
        let mut required_crates = HashSet::new();

        // Check stdlib functions for their dependencies
        for func_name in &self.used_stdlib_functions {
            if let Some(func) = stdlib_registry::get_stdlib_function(func_name) {
                required_crates.extend(func.crate_deps.clone());
            }
        }

        required_crates
    }

    /// Cargo features of the `nail` crate required by the stdlib functions this
    /// program uses (heavy optional dependencies like DuckDB are feature-gated).
    pub fn get_required_nail_features(&self) -> Vec<&'static str> {
        let mut features: Vec<&'static str> = self.get_required_dependencies().iter().filter_map(|dep| dep.nail_feature()).collect();
        features.sort();
        features.dedup();
        features
    }

    /// Generate a complete Cargo.toml for a transpiled program based on the
    /// stdlib functions it actually uses. Crates gated behind a nail feature
    /// are delivered through the nail crate's feature flags rather than as
    /// direct dependencies.
    pub fn generate_cargo_toml(&self, package_name: &str, nail_path: &str) -> String {
        Self::render_cargo_toml(package_name, nail_path, self.get_required_dependencies())
    }

    /// Generate a Cargo.toml requiring every crate the stdlib registry can
    /// ever emit, with all nail features enabled. The bundle build compiles
    /// this once so every real program's dependencies are already cached.
    pub fn generate_cargo_toml_superset(package_name: &str, nail_path: &str) -> String {
        Self::render_cargo_toml(package_name, nail_path, CrateDependency::all().into_iter().collect())
    }

    /// The Cargo.toml for a browser build. A wasm program is a cdylib, not a
    /// binary - the browser loads it, wasm-bindgen glues it to JavaScript,
    /// and tokio shrinks to the executor-free subset wasm supports. The
    /// wasm-bindgen CLI must match the crate version the lockfile resolves,
    /// to the patch - install it with `cargo install wasm-bindgen-cli
    /// --version <resolved>` when they drift.
    pub fn generate_cargo_toml_wasm(&self, package_name: &str, nail_path: &str) -> String {
        use std::collections::BTreeSet;

        let mut dep_lines: BTreeSet<String> = BTreeSet::new();
        dep_lines.insert("rayon = \"1.10.0\"".to_string());
        dep_lines.insert("futures = \"0.3\"".to_string());
        dep_lines.insert(CrateDependency::DashMap.to_cargo_dep().to_string());
        dep_lines.insert(CrateDependency::Serde.to_cargo_dep().to_string());
        dep_lines.insert("tokio = { version = \"1\", features = [\"sync\", \"macros\", \"rt\"] }".to_string());
        dep_lines.insert("wasm-bindgen = \"0.2\"".to_string());
        dep_lines.insert("wasm-bindgen-futures = \"0.4\"".to_string());

        let mut nail_features: BTreeSet<&'static str> = BTreeSet::new();
        for dep in self.get_required_dependencies() {
            match dep.nail_feature() {
                Some(feature) => {
                    nail_features.insert(feature);
                }
                None => {
                    // Tokio's native line asks for the full runtime, which
                    // does not exist on wasm - the fixed line above replaces it.
                    if dep != CrateDependency::Tokio {
                        dep_lines.insert(dep.to_cargo_dep().to_string());
                    }
                }
            }
        }

        let nail_dep = if nail_features.is_empty() {
            format!("nail = {{ path = \"{}\" }}", nail_path)
        } else {
            let features: Vec<String> = nail_features.iter().map(|feature| format!("\"{}\"", feature)).collect();
            format!("nail = {{ path = \"{}\", features = [{}] }}", nail_path, features.join(", "))
        };

        let mut manifest = String::new();
        manifest.push_str("[package]\n");
        manifest.push_str(&format!("name = \"{}\"\n", package_name));
        manifest.push_str("version = \"0.1.0\"\n");
        manifest.push_str("edition = \"2021\"\n");
        manifest.push('\n');
        manifest.push_str("[lib]\n");
        manifest.push_str("crate-type = [\"cdylib\"]\n");
        manifest.push('\n');
        manifest.push_str("[dependencies]\n");
        manifest.push_str(&nail_dep);
        manifest.push('\n');
        for line in dep_lines {
            manifest.push_str(&line);
            manifest.push('\n');
        }
        manifest.push('\n');
        // Browsers download the binary, so release builds trade compile time
        // for size as well as speed.
        manifest.push_str("[profile.release]\n");
        manifest.push_str("lto = \"thin\"\n");
        manifest.push_str("codegen-units = 1\n");
        manifest.push_str("opt-level = \"s\"\n");
        manifest
    }

    /// Every stdlib function this program uses that cannot exist in a
    /// browser, sorted. Empty means the program is fit for `--target=wasm`.
    pub fn wasm_unsupported_functions(&self) -> Vec<String> {
        let mut blockers: Vec<String> = self.used_stdlib_functions.iter().filter(|name| !stdlib_registry::is_stdlib_fn_wasm_safe(name)).cloned().collect();
        blockers.sort();
        return blockers;
    }

    fn render_cargo_toml(package_name: &str, nail_path: &str, required: HashSet<CrateDependency>) -> String {
        use std::collections::BTreeSet;

        // Crates the generated code references unconditionally (see transpile() header)
        let mut dep_lines: BTreeSet<String> = BTreeSet::new();
        dep_lines.insert(CrateDependency::Tokio.to_cargo_dep().to_string());
        dep_lines.insert("rayon = \"1.10.0\"".to_string());
        dep_lines.insert("futures = \"0.3\"".to_string());
        dep_lines.insert(CrateDependency::DashMap.to_cargo_dep().to_string());
        dep_lines.insert(CrateDependency::Serde.to_cargo_dep().to_string());

        let mut nail_features: BTreeSet<&'static str> = BTreeSet::new();
        for dep in required {
            match dep.nail_feature() {
                Some(feature) => {
                    nail_features.insert(feature);
                }
                None => {
                    dep_lines.insert(dep.to_cargo_dep().to_string());
                }
            }
        }

        let nail_dep = if nail_features.is_empty() {
            format!("nail = {{ path = \"{}\" }}", nail_path)
        } else {
            let features: Vec<String> = nail_features.iter().map(|feature| format!("\"{}\"", feature)).collect();
            format!("nail = {{ path = \"{}\", features = [{}] }}", nail_path, features.join(", "))
        };

        let mut manifest = String::new();
        manifest.push_str("[package]\n");
        manifest.push_str(&format!("name = \"{}\"\n", package_name));
        manifest.push_str("version = \"0.1.0\"\n");
        manifest.push_str("edition = \"2021\"\n");
        manifest.push('\n');
        manifest.push_str("[dependencies]\n");
        manifest.push_str(&nail_dep);
        manifest.push('\n');
        for line in dep_lines {
            manifest.push_str(&line);
            manifest.push('\n');
        }
        manifest.push('\n');
        // Thin LTO plus one codegen unit lets LLVM inline across the stdlib
        // boundary and vectorize user loops it otherwise cannot see through.
        // The price is a slower release build, which is the right trade for a
        // binary that ships.
        manifest.push_str("[profile.release]\n");
        manifest.push_str("lto = \"thin\"\n");
        manifest.push_str("codegen-units = 1\n");
        manifest
    }
    
    fn collect_used_functions(&mut self, node: &ASTNode) {
        match node {
            ASTNode::Program { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::FunctionDeclaration { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::FunctionCall { name, args, .. } => {
                if stdlib_registry::get_stdlib_function(name).is_some() {
                    self.used_stdlib_functions.insert(name.clone());
                }
                for arg in args {
                    self.collect_used_functions(arg);
                }
            }
            ASTNode::ConstDeclaration { value, .. } => {
                self.collect_used_functions(value);
            }
            ASTNode::LambdaDeclaration { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::StructInstantiation { fields, .. } => {
                for field in fields {
                    if let ASTNode::StructInstantiationField { value, .. } = field {
                        self.collect_used_functions(value);
                    }
                }
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                for (condition, branch) in condition_branches {
                    self.collect_used_functions(condition);
                    self.collect_used_functions(branch);
                }
                if let Some(else_b) = else_branch {
                    self.collect_used_functions(else_b);
                }
            }
            ASTNode::ForLoop { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::MapExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::FilterExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::ReduceExpression { iterable, initial_value, body, .. } | ASTNode::ScanExpression { iterable, initial_value, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(initial_value);
                self.collect_used_functions(body);
            }
            ASTNode::EachExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::FindExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::AllExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::AnyExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::WhileLoop { condition, max_iterations, body, .. } => {
                self.collect_used_functions(condition);
                if let Some(max) = max_iterations {
                    self.collect_used_functions(max);
                }
                self.collect_used_functions(body);
            }
            ASTNode::Loop { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::SpawnBlock { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::BinaryOperation { left, right, .. } => {
                self.collect_used_functions(left);
                self.collect_used_functions(right);
            }
            ASTNode::ReturnDeclaration { statement, .. } => {
                self.collect_used_functions(statement);
            }
            ASTNode::YieldDeclaration { statement, .. } => {
                self.collect_used_functions(statement);
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                for elem in elements {
                    self.collect_used_functions(elem);
                }
            }
            ASTNode::StructFieldAccess { .. } => {
                // No nested expressions in simple field access
            }
            ASTNode::NestedFieldAccess { object, .. } => {
                self.collect_used_functions(object);
            }
            ASTNode::ParallelBlock { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::ConcurrentBlock { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::UnaryOperation { operand, .. } => {
                self.collect_used_functions(operand);
            }
            // Terminal nodes (literals, identifiers, etc.) that don't contain function calls
            ASTNode::StringLiteral { .. } | 
            ASTNode::NumberLiteral { .. } | 
            ASTNode::BooleanLiteral { .. } | 
            ASTNode::Identifier { .. } |
            ASTNode::BreakStatement { .. } |
            ASTNode::ContinueStatement { .. } |
            ASTNode::StructDeclaration { .. } |
            ASTNode::EnumDeclaration { .. } |
            ASTNode::StructDeclarationField { .. } |
            ASTNode::EnumVariant { .. } => {
                // These nodes don't contain function calls or other expressions
            }
            ASTNode::Assignment { left, right, .. } => {
                // Collect functions from both sides of assignment
                self.collect_used_functions(left);
                self.collect_used_functions(right);
            }
            ASTNode::StructInstantiationField { value, .. } => {
                self.collect_used_functions(value);
            }
        }
    }

    /// Collect free variables referenced in `node` that are not in `bound`.
    /// Used to determine which outer-scope variables a generated closure captures,
    /// so they can be cloned before being moved into an `async move` block.
    fn collect_free_variables(node: &ASTNode, bound: &mut HashSet<String>, free: &mut std::collections::BTreeSet<String>) {
        match node {
            ASTNode::Program { statements, .. } | ASTNode::Block { statements, .. } | ASTNode::ParallelBlock { statements, .. } | ASTNode::ConcurrentBlock { statements, .. } => {
                // Statements form a sequence: declarations bind names for later statements
                let mut inner_bound = bound.clone();
                for stmt in statements {
                    Self::collect_free_variables(stmt, &mut inner_bound, free);
                }
            }
            ASTNode::FunctionDeclaration { name, params, body, .. } => {
                bound.insert(name.clone());
                let mut inner_bound = bound.clone();
                for (param_name, _) in params {
                    inner_bound.insert(param_name.clone());
                }
                Self::collect_free_variables(body, &mut inner_bound, free);
            }
            ASTNode::LambdaDeclaration { params, body, .. } => {
                let mut inner_bound = bound.clone();
                for (param_name, _) in params {
                    inner_bound.insert(param_name.clone());
                }
                Self::collect_free_variables(body, &mut inner_bound, free);
            }
            ASTNode::FunctionCall { args, .. } => {
                for arg in args {
                    Self::collect_free_variables(arg, bound, free);
                }
            }
            ASTNode::ConstDeclaration { name, value, .. } => {
                Self::collect_free_variables(value, bound, free);
                bound.insert(name.clone());
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                for (condition, branch) in condition_branches {
                    Self::collect_free_variables(condition, bound, free);
                    Self::collect_free_variables(branch, &mut bound.clone(), free);
                }
                if let Some(else_b) = else_branch {
                    Self::collect_free_variables(else_b, &mut bound.clone(), free);
                }
            }
            ASTNode::ForLoop { iterator, iterable, initial_value, filter, body, .. } => {
                Self::collect_free_variables(iterable, bound, free);
                if let Some(init) = initial_value {
                    Self::collect_free_variables(init, bound, free);
                }
                let mut inner_bound = bound.clone();
                inner_bound.insert(iterator.clone());
                if let Some(filter_expr) = filter {
                    Self::collect_free_variables(filter_expr, &mut inner_bound, free);
                }
                Self::collect_free_variables(body, &mut inner_bound, free);
            }
            ASTNode::MapExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::FilterExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::EachExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::FindExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::AllExpression { iterator, index_iterator, iterable, body, .. }
            | ASTNode::AnyExpression { iterator, index_iterator, iterable, body, .. } => {
                Self::collect_free_variables(iterable, bound, free);
                let mut inner_bound = bound.clone();
                inner_bound.insert(iterator.clone());
                if let Some(idx) = index_iterator {
                    inner_bound.insert(idx.clone());
                }
                Self::collect_free_variables(body, &mut inner_bound, free);
            }
            ASTNode::ReduceExpression { accumulator, iterator, index_iterator, iterable, initial_value, body, .. } | ASTNode::ScanExpression { accumulator, iterator, index_iterator, iterable, initial_value, body, .. } => {
                Self::collect_free_variables(iterable, bound, free);
                Self::collect_free_variables(initial_value, bound, free);
                let mut inner_bound = bound.clone();
                inner_bound.insert(accumulator.clone());
                inner_bound.insert(iterator.clone());
                if let Some(idx) = index_iterator {
                    inner_bound.insert(idx.clone());
                }
                Self::collect_free_variables(body, &mut inner_bound, free);
            }
            ASTNode::WhileLoop { condition, initial_value, max_iterations, body, .. } => {
                Self::collect_free_variables(condition, bound, free);
                if let Some(init) = initial_value {
                    Self::collect_free_variables(init, bound, free);
                }
                if let Some(max) = max_iterations {
                    Self::collect_free_variables(max, bound, free);
                }
                Self::collect_free_variables(body, &mut bound.clone(), free);
            }
            ASTNode::Loop { index_iterator, body, .. } => {
                let mut inner_bound = bound.clone();
                if let Some(idx) = index_iterator {
                    inner_bound.insert(idx.clone());
                }
                Self::collect_free_variables(body, &mut inner_bound, free);
            }
            ASTNode::SpawnBlock { body, .. } => {
                Self::collect_free_variables(body, &mut bound.clone(), free);
            }
            ASTNode::BinaryOperation { left, right, .. } | ASTNode::Assignment { left, right, .. } => {
                Self::collect_free_variables(left, bound, free);
                Self::collect_free_variables(right, bound, free);
            }
            ASTNode::UnaryOperation { operand, .. } => {
                Self::collect_free_variables(operand, bound, free);
            }
            ASTNode::StructInstantiation { fields, .. } => {
                for field in fields {
                    Self::collect_free_variables(field, bound, free);
                }
            }
            ASTNode::StructInstantiationField { value, .. } => {
                Self::collect_free_variables(value, bound, free);
            }
            ASTNode::StructFieldAccess { struct_name, .. } => {
                if !bound.contains(struct_name) {
                    free.insert(struct_name.clone());
                }
            }
            ASTNode::NestedFieldAccess { object, .. } => {
                Self::collect_free_variables(object, bound, free);
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                for elem in elements {
                    Self::collect_free_variables(elem, bound, free);
                }
            }
            ASTNode::Identifier { name, .. } => {
                if !bound.contains(name) {
                    free.insert(name.clone());
                }
            }
            ASTNode::ReturnDeclaration { statement, .. } | ASTNode::YieldDeclaration { statement, .. } => {
                Self::collect_free_variables(statement, bound, free);
            }
            ASTNode::StringLiteral { .. }
            | ASTNode::NumberLiteral { .. }
            | ASTNode::BooleanLiteral { .. }
            | ASTNode::BreakStatement { .. }
            | ASTNode::ContinueStatement { .. }
            | ASTNode::StructDeclaration { .. }
            | ASTNode::StructDeclarationField { .. }
            | ASTNode::EnumDeclaration { .. }
            | ASTNode::EnumVariant { .. } => {
                // No variable references
            }
        }
    }

    /// Free variables that a collection-operation closure body captures from the
    /// enclosing scope (excluding the iterator and optional index binding).
    fn closure_captured_variables(body: &ASTNode, iterator: &str, index_iterator: &Option<String>) -> std::collections::BTreeSet<String> {
        let mut bound = HashSet::new();
        bound.insert(iterator.to_string());
        if let Some(idx) = index_iterator {
            bound.insert(idx.clone());
        }
        let mut free = std::collections::BTreeSet::new();
        Self::collect_free_variables(body, &mut bound, &mut free);
        free
    }

    /// Every direct child expression/statement node of an AST node, used by the
    /// move-analysis walks below. Binder names (iterators, params) are handled
    /// by the callers, not here. The enumeration itself lives on ASTNode so
    /// other passes (like the checker's sandboxed-code walk) share one definition.
    fn ast_children(node: &ASTNode) -> Vec<&ASTNode> {
        node.children()
    }

    /// Count how many times each binding is referenced, in the same shape the
    /// emitter walks the tree. Nested functions and lambdas get their own
    /// MoveContext, so their bodies are skipped. References that emit outside
    /// the Identifier arm (struct field access bases, assignment targets) are
    /// counted here but never decremented during emission, which permanently
    /// pins those bindings to clone-only.
    fn count_variable_uses(node: &ASTNode, resolution: &HashMap<(CodeSpan, String), BindingId>, counts: &mut HashMap<BindingId, usize>) {
        let count = |code_span: &CodeSpan, name: &str, counts: &mut HashMap<BindingId, usize>| {
            if let Some(id) = resolution.get(&(code_span.clone(), name.to_string())) {
                *counts.entry(*id).or_insert(0) += 1;
            }
        };
        match node {
            ASTNode::Identifier { name, code_span, .. } => count(code_span, name, counts),
            ASTNode::StructFieldAccess { struct_name, code_span, .. } => {
                // Emitted as a raw name outside the Identifier arm: pin it.
                count(code_span, struct_name, counts);
            }
            ASTNode::Assignment { left, right, .. } => {
                // The assignment target is emitted bare and must stay valid: pin it.
                if let ASTNode::Identifier { name, code_span, .. } = left.as_ref() {
                    count(code_span, name, counts);
                } else {
                    Self::count_variable_uses(left, resolution, counts);
                }
                Self::count_variable_uses(right, resolution, counts);
            }
            ASTNode::FunctionDeclaration { .. } | ASTNode::LambdaDeclaration { .. } => {}
            _ => {
                for child in Self::ast_children(node) {
                    Self::count_variable_uses(child, resolution, counts);
                }
            }
        }
    }

    /// Collect every binding referenced in a region that execution can
    /// revisit (loop bodies, closure bodies, parallel/spawn blocks) or that is
    /// captured by a lambda. Those bindings are never moved. Expressions a
    /// construct evaluates exactly once in the enclosing flow (a loop's
    /// iterable, a reduce's initial value) are NOT part of the region.
    /// Binding-keyed, so an inner binder that shadows an outer name does not
    /// pin the outer binding.
    fn collect_never_move(node: &ASTNode, revisitable: bool, resolution: &HashMap<(CodeSpan, String), BindingId>, out: &mut HashSet<BindingId>) {
        let mark_all = |subtrees: &[&ASTNode], out: &mut HashSet<BindingId>| {
            for subtree in subtrees {
                Self::collect_never_move(subtree, true, resolution, out);
            }
        };
        match node {
            ASTNode::Identifier { name, code_span, .. } => {
                if revisitable {
                    if let Some(id) = resolution.get(&(code_span.clone(), name.clone())) {
                        out.insert(*id);
                    }
                }
            }
            ASTNode::StructFieldAccess { struct_name, code_span, .. } => {
                if revisitable {
                    if let Some(id) = resolution.get(&(code_span.clone(), struct_name.clone())) {
                        out.insert(*id);
                    }
                }
            }
            ASTNode::FunctionDeclaration { .. } => {} // own context, cannot capture
            ASTNode::LambdaDeclaration { body, .. } => mark_all(&[body.as_ref()], out),
            ASTNode::MapExpression { iterable, body, .. }
            | ASTNode::FilterExpression { iterable, body, .. }
            | ASTNode::EachExpression { iterable, body, .. }
            | ASTNode::FindExpression { iterable, body, .. }
            | ASTNode::AllExpression { iterable, body, .. }
            | ASTNode::AnyExpression { iterable, body, .. } => {
                Self::collect_never_move(iterable, revisitable, resolution, out);
                mark_all(&[body.as_ref()], out);
            }
            ASTNode::ReduceExpression { iterable, initial_value, body, .. } | ASTNode::ScanExpression { iterable, initial_value, body, .. } => {
                Self::collect_never_move(iterable, revisitable, resolution, out);
                Self::collect_never_move(initial_value, revisitable, resolution, out);
                mark_all(&[body.as_ref()], out);
            }
            ASTNode::ForLoop { iterable, initial_value, filter, body, .. } => {
                Self::collect_never_move(iterable, revisitable, resolution, out);
                if let Some(initial_value) = initial_value {
                    Self::collect_never_move(initial_value, revisitable, resolution, out);
                }
                if let Some(filter) = filter {
                    mark_all(&[filter.as_ref()], out);
                }
                mark_all(&[body.as_ref()], out);
            }
            ASTNode::WhileLoop { condition, initial_value, max_iterations, body, .. } => {
                if let Some(initial_value) = initial_value {
                    Self::collect_never_move(initial_value, revisitable, resolution, out);
                }
                if let Some(max_iterations) = max_iterations {
                    Self::collect_never_move(max_iterations, revisitable, resolution, out);
                }
                mark_all(&[condition.as_ref(), body.as_ref()], out);
            }
            ASTNode::Loop { body, .. } => mark_all(&[body.as_ref()], out),
            ASTNode::ParallelBlock { statements, .. } | ASTNode::ConcurrentBlock { statements, .. } => {
                for statement in statements {
                    Self::collect_never_move(statement, true, resolution, out);
                }
            }
            ASTNode::SpawnBlock { body, .. } => mark_all(&[body.as_ref()], out),
            _ => {
                for child in Self::ast_children(node) {
                    Self::collect_never_move(child, revisitable, resolution, out);
                }
            }
        }
    }

    /// Params have no declaration node in `root`, so they are bound here into
    /// the resolver's root scope before the walk.
    fn push_move_context(&mut self, root: &ASTNode, params: &[String]) {
        let mut resolver = BindingResolver::new();
        let mut param_bindings = HashMap::new();
        for param in params {
            param_bindings.insert(param.clone(), resolver.bind(param));
        }
        resolver.walk(root);
        let mut remaining = HashMap::new();
        Self::count_variable_uses(root, &resolver.resolution, &mut remaining);
        let mut never_move = HashSet::new();
        Self::collect_never_move(root, false, &resolver.resolution, &mut never_move);
        self.move_contexts.push(MoveContext {
            remaining,
            activated: HashSet::new(),
            never_move,
            binding_types: HashMap::new(),
            types: HashMap::new(),
            resolution: resolver.resolution,
            param_bindings,
        });
    }

    fn pop_move_context(&mut self) {
        self.move_contexts.pop();
    }

    /// Record a declaration in the current context as move-eligible, resolved
    /// to its binding via the declaration node's span.
    fn activate_declaration(&mut self, name: &str, data_type: &NailDataTypeDescriptor, code_span: &CodeSpan) {
        if let Some(context) = self.move_contexts.last_mut() {
            if let Some(id) = context.resolution.get(&(code_span.clone(), name.to_string())).copied() {
                context.activated.insert(id);
                context.binding_types.insert(id, data_type.clone());
            }
            context.types.insert(name.to_string(), data_type.clone());
        }
    }

    /// Record a param of the current context as move-eligible.
    fn activate_param(&mut self, name: &str, data_type: &NailDataTypeDescriptor) {
        if let Some(context) = self.move_contexts.last_mut() {
            if let Some(id) = context.param_bindings.get(name).copied() {
                context.activated.insert(id);
                context.binding_types.insert(id, data_type.clone());
            }
            context.types.insert(name.to_string(), data_type.clone());
        }
    }

    /// Record a binder's type (iterator, index) for Copy classification only.
    fn record_variable_type(&mut self, name: &str, data_type: NailDataTypeDescriptor) {
        if let Some(context) = self.move_contexts.last_mut() {
            context.types.insert(name.to_string(), data_type);
        }
    }

    fn lookup_variable_type(&self, name: &str) -> Option<&NailDataTypeDescriptor> {
        self.move_contexts.iter().rev().find_map(|context| context.types.get(name))
    }

    /// Resolve a reference in the CURRENT context to its binding, if the
    /// resolver mapped it.
    fn resolve_binding(&self, name: &str, code_span: &CodeSpan) -> Option<BindingId> {
        self.move_contexts.last().and_then(|context| context.resolution.get(&(code_span.clone(), name.to_string())).copied())
    }

    /// The declared type of a reference: exact per-binding type when resolved,
    /// otherwise the name-keyed fallback (binders, cross-context references).
    fn reference_type(&self, name: &str, code_span: &CodeSpan) -> Option<&NailDataTypeDescriptor> {
        self.resolve_binding(name, code_span)
            .and_then(|id| self.move_contexts.last().and_then(|context| context.binding_types.get(&id)))
            .or_else(|| self.lookup_variable_type(name))
    }

    /// The element type of an iterable, when it is statically known: from a
    /// declared array variable, from the signature of the standard library
    /// function that built the array, or from the literal itself.
    fn iterable_element_type(&self, iterable: &ASTNode) -> Option<NailDataTypeDescriptor> {
        match iterable {
            ASTNode::Identifier { name, code_span, .. } => match self.reference_type(name, code_span) {
                Some(NailDataTypeDescriptor::Array(inner)) => Some((**inner).clone()),
                _ => None,
            },
            ASTNode::FunctionCall { name, .. } => match stdlib_registry::get_stdlib_function(name).map(|function| function.return_type.clone()) {
                Some(NailDataTypeDescriptor::Array(inner)) => Some(*inner),
                _ => None,
            },
            // A literal array is its own declaration. Elements written as
            // anything but literals of one type say nothing certain here.
            ASTNode::ArrayLiteral { elements, .. } => {
                let mut element_types = elements.iter().map(|element| match element {
                    ASTNode::NumberLiteral { data_type, .. } => Some(data_type.clone()),
                    ASTNode::StringLiteral { .. } => Some(NailDataTypeDescriptor::String),
                    ASTNode::BooleanLiteral { .. } => Some(NailDataTypeDescriptor::Boolean),
                    _ => None,
                });
                let first = element_types.next()??;
                element_types.all(|element_type| element_type.as_ref() == Some(&first)).then_some(first)
            }
            _ => None,
        }
    }

    /// How a variable reference is emitted. Copy types (i/f/b) are always bare.
    /// A heap-typed binding moves at its last syntactic use when execution
    /// cannot revisit the reference; otherwise it clones.
    fn identifier_expression(&mut self, name: &str, code_span: &CodeSpan) -> String {
        if matches!(self.reference_type(name, code_span), Some(NailDataTypeDescriptor::Int | NailDataTypeDescriptor::Float | NailDataTypeDescriptor::Boolean)) {
            return name.to_string();
        }
        if let Some(id) = self.resolve_binding(name, code_span) {
            if let Some(context) = self.move_contexts.last_mut() {
                if let Some(remaining) = context.remaining.get_mut(&id) {
                    if *remaining > 0 {
                        *remaining -= 1;
                    }
                    if *remaining == 0 && context.activated.contains(&id) && !context.never_move.contains(&id) {
                        return name.to_string();
                    }
                }
            }
        }
        format!("{}.clone()", name)
    }

    /// How a parallel collection operation (map/filter/all/any/find) is
    /// emitted, decided by context and body purity:
    /// - Sync: inside a pure fn — plain rayon chain, no runtime involved.
    /// - AsyncPure: async context, pure body — rayon inside spawn_blocking so
    ///   tokio core threads aren't blocked, but plain closures (no block_on).
    /// - AsyncBlockOn: async context, body does async work — rayon inside
    ///   spawn_blocking with a per-element block_on.
    fn parallel_iter_mode(&self, body: &ASTNode) -> ParallelIterMode {
        // A browser has one thread and no blocking executor, so every
        // collection operation is a plain chain there - rayon runs those
        // inline when it cannot spawn threads, which on wasm it cannot.
        if self.wasm_target {
            return ParallelIterMode::Sync;
        }
        if self.in_sync_function || self.in_parallel_closure {
            ParallelIterMode::Sync
        } else if !Self::node_needs_async(body, &self.pure_functions) {
            ParallelIterMode::AsyncPure
        } else {
            ParallelIterMode::AsyncBlockOn
        }
    }

    /// Called once the closure header of a parallel operation is written. Only
    /// an AsyncBlockOn body sits inside an async block; every other mode puts
    /// the body in a plain closure, where an inner operation must stay sync.
    fn enter_parallel_closure(&mut self, mode: ParallelIterMode) {
        self.enclosing_parallel_closures.push(self.in_parallel_closure);
        self.in_parallel_closure = mode != ParallelIterMode::AsyncBlockOn;
    }

    /// Called once the closure of a parallel operation is closed.
    fn leave_parallel_closure(&mut self) {
        self.in_parallel_closure = self.enclosing_parallel_closures.pop().unwrap_or(false);
    }

    /// Shared opening of a parallel collection operation (map/filter). The
    /// iterable is evaluated first (it may await in async context), then the
    /// rayon chain runs — wrapped in spawn_blocking in async contexts.
    /// Per-capture clones keep the closures from moving outer variables
    /// (E0507). Callers emit the body then write_parallel_iter_close.
    fn write_parallel_iter_open(&mut self, iterator: &str, index_iterator: &Option<String>, iterable: &ASTNode, body: &ASTNode, rayon_method: &str, mode: ParallelIterMode, output: &mut String) -> Result<(), CodeError> {
        if self.wasm_target && Self::node_needs_async(body, &self.pure_functions) {
            return Err(CodeError {
                help: Some("do the async work before the collection operation, or build natively".to_string()),
                message: "the browser build cannot run async work inside a collection operation".to_string(),
                code_span: body.code_span(),
            });
        }
        if let Some(element_type) = self.iterable_element_type(iterable) {
            self.record_variable_type(iterator, element_type);
        }
        if let Some(idx) = index_iterator {
            self.record_variable_type(idx, NailDataTypeDescriptor::Int);
        }
        if mode == ParallelIterMode::AsyncBlockOn {
            writeln!(output, "{}let __rt_handle = tokio::runtime::Handle::current();", self.indent())?;
        }
        write!(output, "{}let __iter = ", self.indent())?;
        // Lazy iterator forms (ranges) are only IndexedParallelIterator for
        // small integer types in rayon, so enumerate() is unavailable on them;
        // use the lazy form only when no index binding is needed.
        let use_index = index_iterator.is_some();
        self.transpile_iterable(iterable, !use_index, output)?;
        writeln!(output, ";")?;

        let captured = Self::closure_captured_variables(body, iterator, index_iterator);
        for var in &captured {
            writeln!(output, "{}let {} = {}.clone();", self.indent(), var, var)?;
        }

        if mode == ParallelIterMode::Sync {
            write!(output, "{}let __result: Vec<_> = ", self.indent())?;
        } else {
            writeln!(output, "{}let __result: Vec<_> = tokio::task::spawn_blocking(move || {{", self.indent())?;
            self.indent_level += 1;
            write!(output, "{}", self.indent())?;
        }
        if use_index {
            writeln!(output, "__iter.into_par_iter().enumerate().{}(|(_idx, {})| {{", rayon_method, iterator)?;
        } else {
            writeln!(output, "__iter.into_par_iter().{}(|{}| {{", rayon_method, iterator)?;
        }
        self.indent_level += 1;

        for var in &captured {
            writeln!(output, "{}let {} = {}.clone();", self.indent(), var, var)?;
        }

        if mode == ParallelIterMode::AsyncBlockOn {
            writeln!(output, "{}__rt_handle.block_on(async move {{", self.indent())?;
            self.indent_level += 1;
        }

        if let Some(idx) = index_iterator {
            writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
        }
        self.enter_parallel_closure(mode);
        Ok(())
    }

    /// Closes the structure opened by write_parallel_iter_open.
    fn write_parallel_iter_close(&mut self, mode: ParallelIterMode, output: &mut String) -> Result<(), CodeError> {
        self.leave_parallel_closure();
        if mode == ParallelIterMode::AsyncBlockOn {
            self.indent_level -= 1;
            writeln!(output, "{}}})", self.indent())?;
        }
        self.indent_level -= 1;
        match mode {
            ParallelIterMode::Sync => {
                writeln!(output, "{}}}).collect();", self.indent())?;
            }
            _ => {
                writeln!(output, "{}}}).collect()", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}).await.unwrap();", self.indent())?;
            }
        }
        Ok(())
    }

    /// Whether an identifier appears anywhere inside a node.
    fn mentions(node: &ASTNode, name: &str) -> bool {
        if let ASTNode::Identifier { name: identifier, .. } = node {
            if identifier == name {
                return true;
            }
        }
        Self::ast_children(node).into_iter().any(|child| Self::mentions(child, name))
    }

    /// The single expression a block evaluates to, whether it is written as a
    /// yield, a return, or a bare expression - `{ num }` and `{ y num; }` are
    /// the same branch.
    fn block_value(node: &ASTNode) -> Option<&ASTNode> {
        match node {
            ASTNode::Block { statements, .. } => match statements.as_slice() {
                [ASTNode::YieldDeclaration { statement, .. }] | [ASTNode::ReturnDeclaration { statement, .. }] => Some(statement.as_ref()),
                [only] => Some(only),
                _ => None,
            },
            other => Some(other),
        }
    }

    /// The name an expression is, when it is nothing but an identifier.
    fn identifier_name(node: &ASTNode) -> Option<&str> {
        match node {
            ASTNode::Identifier { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    /// Whether an expression provably evaluates to a whole number, with the
    /// element binding taken to have the type its array gives it. Every case
    /// this cannot prove reads as "not provably a whole number", which only
    /// ever keeps a fold sequential.
    fn provably_int(&self, node: &ASTNode, iterator: &str, element_type: &NailDataTypeDescriptor) -> bool {
        let int = NailDataTypeDescriptor::Int;
        match node {
            ASTNode::NumberLiteral { data_type, .. } => data_type == &int,
            ASTNode::Identifier { name, code_span, .. } => {
                if name == iterator {
                    element_type == &int
                } else {
                    self.reference_type(name, code_span) == Some(&int)
                }
            }
            // Nail's arithmetic operators take two values of one type and
            // produce that same type, integer division included.
            ASTNode::BinaryOperation { left, operator, right, .. } => {
                matches!(operator, Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod)
                    && self.provably_int(left, iterator, element_type)
                    && self.provably_int(right, iterator, element_type)
            }
            ASTNode::UnaryOperation { operator: Operation::Sub, operand, .. } => self.provably_int(operand, iterator, element_type),
            ASTNode::FunctionCall { name, .. } => {
                let returns = stdlib_registry::get_stdlib_function(name).map(|function| function.return_type.clone()).or_else(|| self.function_return_types.get(name).cloned());
                returns.as_ref() == Some(&int)
            }
            ASTNode::StructFieldAccess { struct_name, field_name, code_span, .. } => {
                let held = if struct_name == iterator { Some(element_type) } else { self.reference_type(struct_name, code_span) };
                let declared = match held {
                    Some(NailDataTypeDescriptor::Struct(name)) => name.clone(),
                    _ => return false,
                };
                self.struct_field_types.get(&declared).and_then(|fields| fields.get(field_name)) == Some(&int)
            }
            _ => false,
        }
    }

    /// The fold that may be evaluated in any grouping, or None when the fold
    /// must stay sequential.
    ///
    /// Splitting a fold across threads regroups it - `(a+b)+c` becomes
    /// `a+(b+c)` - so only an associative step may be parallelised. That is
    /// proven here from the shape of the code rather than promised by whoever
    /// wrote it. The body must be a single value that either combines the
    /// accumulator with a whole number computed from the element, using `+` or
    /// `*`, or chooses the larger or smaller of the accumulator and the
    /// element. What each element contributes is then a whole number and so is
    /// the accumulator, making the whole fold integer arithmetic - which is
    /// associative and commutative for all four steps.
    ///
    /// Everything else stays sequential, including folds that happen to be
    /// associative but aren't written this plainly, and every float fold -
    /// floating point addition is not associative, so regrouping changes the
    /// rounding and the result.
    fn associative_fold<'a>(&self, accumulator: &str, iterator: &str, index_iterator: &Option<String>, iterable: &ASTNode, body: &'a ASTNode) -> Option<AssociativeFold<'a>> {
        // A step that consults the position of the element is not a step that
        // can be moved to another position. A bound but unused index is no
        // such consultation, and is common enough to be worth allowing.
        if let Some(index) = index_iterator {
            if Self::mentions(body, index) {
                return None;
            }
        }
        let element_type = self.iterable_element_type(iterable)?;
        let statements = match body {
            ASTNode::Block { statements, .. } => statements,
            _ => return None,
        };
        let folded = match statements.as_slice() {
            [ASTNode::YieldDeclaration { statement, .. }] | [ASTNode::ReturnDeclaration { statement, .. }] => statement.as_ref(),
            _ => return None,
        };
        // The two names the step is allowed to combine, in whichever order
        // they were written.
        let pair_of = |left: &ASTNode, right: &ASTNode| -> Option<(bool, bool)> {
            let (left, right) = (Self::identifier_name(left)?, Self::identifier_name(right)?);
            if left == accumulator && right == iterator {
                Some((true, false))
            } else if left == iterator && right == accumulator {
                Some((false, true))
            } else {
                None
            }
        };
        match folded {
            ASTNode::BinaryOperation { left, operator, right, .. } => {
                let step = match operator {
                    Operation::Add => AssociativeStep::Add,
                    Operation::Mul => AssociativeStep::Mul,
                    _ => return None,
                };
                // One side must be the accumulator itself; the other is what
                // the element contributes. That contribution has to be a whole
                // number, may not read the accumulator - which would make the
                // step depend on the order elements arrive in - and may not do
                // async work, since it will be evaluated in a plain closure.
                let contributed = match (Self::identifier_name(left), Self::identifier_name(right)) {
                    (Some(name), _) if name == accumulator => right.as_ref(),
                    (_, Some(name)) if name == accumulator => left.as_ref(),
                    _ => return None,
                };
                if Self::mentions(contributed, accumulator) || !self.provably_int(contributed, iterator, &element_type) || Self::node_needs_async(contributed, &self.pure_functions) {
                    return None;
                }
                let element_value = match Self::identifier_name(contributed) {
                    Some(name) if name == iterator => None,
                    _ => Some(contributed),
                };
                Some(AssociativeFold { step, element_value })
            }
            // `if { num > acc => { num }, else => { acc } }` and its seven
            // siblings are min and max written out by hand.
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                // Here the element is compared and kept as it stands, so it is
                // the element itself that has to be a whole number.
                if element_type != NailDataTypeDescriptor::Int {
                    return None;
                }
                let (condition, taken) = match condition_branches.as_slice() {
                    [(condition, branch)] => (condition.as_ref(), Self::block_value(branch)?),
                    _ => return None,
                };
                let untaken = Self::block_value(else_branch.as_ref()?)?;
                let (left, operator, right) = match condition {
                    ASTNode::BinaryOperation { left, operator, right, .. } => (left.as_ref(), operator, right.as_ref()),
                    _ => return None,
                };
                let (left_is_accumulator, _) = pair_of(left, right)?;
                let taken = Self::identifier_name(taken)?;
                let untaken = Self::identifier_name(untaken)?;
                // The branches must return the two names, not the same one
                // twice - `if { a > b => { a }, else => { a } }` ignores an
                // element and is not this fold at all.
                if taken == untaken || (taken != accumulator && taken != iterator) || (untaken != accumulator && untaken != iterator) {
                    return None;
                }
                // Which side of the comparison is taken decides which end of
                // the range is being kept.
                let takes_left = (taken == accumulator) == left_is_accumulator;
                let step = match (operator, takes_left) {
                    (Operation::Gt | Operation::Gte, true) | (Operation::Lt | Operation::Lte, false) => AssociativeStep::Max,
                    (Operation::Lt | Operation::Lte, true) | (Operation::Gt | Operation::Gte, false) => AssociativeStep::Min,
                    _ => return None,
                };
                Some(AssociativeFold { step, element_value: None })
            }
            _ => None,
        }
    }

    /// A fold run one element at a time, carrying the accumulator forward.
    /// With `collect_steps` the block evaluates to every value the accumulator
    /// took, one per element, which is a `scan`; without it, to the last value
    /// alone, which is a `reduce`.
    #[allow(clippy::too_many_arguments)]
    fn write_sequential_fold(
        &mut self,
        accumulator: &str,
        iterator: &str,
        index_iterator: &Option<String>,
        iterable: &ASTNode,
        initial_value: &ASTNode,
        body: &ASTNode,
        collect_steps: bool,
        add_semicolons: bool,
        output: &mut String,
    ) -> Result<(), CodeError> {
        if add_semicolons {
            write!(output, "{}", self.indent())?;
        }
        writeln!(output, "{{")?;
        self.indent_level += 1;

        // Initialize accumulator; record its type when the initial
        // value makes it obvious so Copy accumulators stay bare
        match initial_value {
            ASTNode::NumberLiteral { data_type, .. } => self.record_variable_type(accumulator, data_type.clone()),
            ASTNode::StringLiteral { .. } => self.record_variable_type(accumulator, NailDataTypeDescriptor::String),
            ASTNode::BooleanLiteral { .. } => self.record_variable_type(accumulator, NailDataTypeDescriptor::Boolean),
            _ => {}
        }
        write!(output, "{}let mut {} = ", self.indent(), accumulator)?;
        self.transpile_node_internal(initial_value, output, false)?;
        writeln!(output, ";")?;
        if collect_steps {
            writeln!(output, "{}let mut __scan_result = Vec::new();", self.indent())?;
        }

        self.write_sequential_for_open(iterator, index_iterator, iterable, output)?;

        // Transpile the body statements
        if let ASTNode::Block { statements, .. } = body {
            for stmt in statements {
                match stmt {
                    // Yield/return statements assign the next accumulator value
                    ASTNode::YieldDeclaration { statement, .. } | ASTNode::ReturnDeclaration { statement, .. } => {
                        write!(output, "{}{} = ", self.indent(), accumulator)?;
                        self.transpile_node_internal(statement, output, false)?;
                        writeln!(output, ";")?;
                    }
                    _ => {
                        self.transpile_node_internal(stmt, output, true)?;
                    }
                }
            }
        }
        // One value per element, recorded after the step that produced it, so
        // a scan is always as long as the array it scanned.
        if collect_steps {
            writeln!(output, "{}__scan_result.push({}.clone());", self.indent(), accumulator)?;
        }

        self.indent_level -= 1;
        writeln!(output, "{}}}", self.indent())?;
        writeln!(output, "{}{}", self.indent(), if collect_steps { "__scan_result" } else { accumulator })?;
        self.indent_level -= 1;
        write!(output, "{}}}", self.indent())?;
        Ok(())
    }

    /// Shared opening of a sequential collection operation (reduce/scan/each/
    /// find/all/any): enumerate for-loop plus the optional index binding. Leaves
    /// indent_level one deeper; callers emit the body and the closing brace.

    fn write_sequential_for_open(&mut self, iterator: &str, index_iterator: &Option<String>, iterable: &ASTNode, output: &mut String) -> Result<(), CodeError> {
        if let Some(element_type) = self.iterable_element_type(iterable) {
            self.record_variable_type(iterator, element_type);
        }
        if let Some(idx) = index_iterator {
            self.record_variable_type(idx, NailDataTypeDescriptor::Int);
        }
        write!(output, "{}for (_idx, {}) in ", self.indent(), iterator)?;
        self.transpile_iterable(iterable, true, output)?;
        writeln!(output, ".into_iter().enumerate() {{")?;
        self.indent_level += 1;

        if let Some(idx) = index_iterator {
            writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
        }
        Ok(())
    }

    /// Emit an iterable expression. When `allow_lazy` is set and the iterable
    /// is a stdlib call the registry knows a native iterator form for (like
    /// array_range -> a..b), emit that form instead of materializing a Vec.
    fn transpile_iterable(&mut self, iterable: &ASTNode, allow_lazy: bool, output: &mut String) -> Result<(), CodeError> {
        if allow_lazy {
            if let ASTNode::FunctionCall { name, args, .. } = iterable {
                if let Some(template) = stdlib_registry::get_iterator_form(name) {
                    let mut rendered = template.to_string();
                    for (i, arg) in args.iter().enumerate() {
                        let mut arg_out = String::new();
                        self.transpile_node_internal(arg, &mut arg_out, false)?;
                        rendered = rendered.replace(&format!("{{{}}}", i), &arg_out);
                    }
                    write!(output, "{}", rendered)?;
                    return Ok(());
                }
            }
        }
        self.transpile_node_internal(iterable, output, false)
    }

    /// Emit `let condition_result = { <body> };` with the collection-operation
    /// context flag set while the body transpiles (filter/find/all/any).
    fn write_condition_result(&mut self, body: &ASTNode, output: &mut String) -> Result<(), CodeError> {
        writeln!(output, "{}let condition_result = {{", self.indent())?;
        self.indent_level += 1;

        let prev_context = self.in_collection_operation;
        self.in_collection_operation = true;
        write!(output, "{}", self.indent())?;
        self.transpile_node_internal(body, output, false)?;
        self.in_collection_operation = prev_context;
        if !output.ends_with('\n') {
            writeln!(output)?;
        }

        self.indent_level -= 1;
        writeln!(output, "{}}};", self.indent())?;
        Ok(())
    }

    /// Shared shape of the short-circuiting search operations (find/all/any):
    /// an init statement, a sequential loop evaluating the body as a condition,
    /// on-match statements ending in break, and a result expression.
    /// Shared shape of the short-circuiting search operations (find/all/any),
    /// emitted as rayon's parallel short-circuiting find_first/all/any. The
    /// same three modes as map/filter apply (sync, async + pure body, async +
    /// block_on). find_first's predicate receives &Item, so `by_ref_param`
    /// clones the binding to an owned value first.
    fn write_parallel_search(
        &mut self,
        iterator: &str,
        index_iterator: &Option<String>,
        iterable: &ASTNode,
        body: &ASTNode,
        output: &mut String,
        add_semicolons: bool,
        rayon_method: &str,
        by_ref_param: bool,
        result_expr: &str,
    ) -> Result<(), CodeError> {
        if add_semicolons {
            write!(output, "{}", self.indent())?;
        }
        write!(output, "{{")?;
        writeln!(output)?;
        self.indent_level += 1;

        if let Some(element_type) = self.iterable_element_type(iterable) {
            self.record_variable_type(iterator, element_type);
        }
        if let Some(idx) = index_iterator {
            self.record_variable_type(idx, NailDataTypeDescriptor::Int);
        }

        let mode = self.parallel_iter_mode(body);
        let use_index = index_iterator.is_some();

        if mode == ParallelIterMode::AsyncBlockOn {
            writeln!(output, "{}let __rt_handle = tokio::runtime::Handle::current();", self.indent())?;
        }
        write!(output, "{}let __iter = ", self.indent())?;
        self.transpile_iterable(iterable, !use_index, output)?;
        writeln!(output, ";")?;

        let captured = Self::closure_captured_variables(body, iterator, index_iterator);
        for var in &captured {
            writeln!(output, "{}let {} = {}.clone();", self.indent(), var, var)?;
        }

        if mode == ParallelIterMode::Sync {
            write!(output, "{}let __search_result = ", self.indent())?;
        } else {
            writeln!(output, "{}let __search_result = tokio::task::spawn_blocking(move || {{", self.indent())?;
            self.indent_level += 1;
            write!(output, "{}", self.indent())?;
        }
        match (use_index, by_ref_param) {
            (false, false) => writeln!(output, "__iter.into_par_iter().{}(|{}| {{", rayon_method, iterator)?,
            (false, true) => writeln!(output, "__iter.into_par_iter().{}(|__item| {{", rayon_method)?,
            (true, false) => writeln!(output, "__iter.into_par_iter().enumerate().{}(|(_idx, {})| {{", rayon_method, iterator)?,
            (true, true) => writeln!(output, "__iter.into_par_iter().enumerate().{}(|__pair| {{", rayon_method)?,
        }
        self.indent_level += 1;
        if by_ref_param {
            if use_index {
                writeln!(output, "{}let (_idx, {}) = __pair.clone();", self.indent(), iterator)?;
            } else {
                writeln!(output, "{}let {} = __item.clone();", self.indent(), iterator)?;
            }
        }

        for var in &captured {
            writeln!(output, "{}let {} = {}.clone();", self.indent(), var, var)?;
        }

        if mode == ParallelIterMode::AsyncBlockOn {
            writeln!(output, "{}__rt_handle.block_on(async move {{", self.indent())?;
            self.indent_level += 1;
        }
        if let Some(idx) = index_iterator {
            writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
        }

        self.enter_parallel_closure(mode);
        self.write_condition_result(body, output)?;
        writeln!(output, "{}condition_result", self.indent())?;
        self.leave_parallel_closure();

        if mode == ParallelIterMode::AsyncBlockOn {
            self.indent_level -= 1;
            writeln!(output, "{}}})", self.indent())?;
        }
        self.indent_level -= 1;
        match mode {
            ParallelIterMode::Sync => writeln!(output, "{}}});", self.indent())?,
            _ => {
                writeln!(output, "{}}})", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}).await.unwrap();", self.indent())?;
            }
        }

        writeln!(output, "{}{}", self.indent(), result_expr)?;
        self.indent_level -= 1;
        write!(output, "{}}}", self.indent())?;
        Ok(())
    }

    pub fn transpile(&mut self, node: &ASTNode) -> Result<String, CodeError> {
        // First pass: collect all used functions by traversing the AST
        self.collect_used_functions(node);
        let mut reassigned = HashSet::new();
        Self::collect_reassigned_variables(node, &mut reassigned);
        self.reassigned_variables = reassigned;
        self.pure_functions = Self::compute_pure_functions(node);
        let mut struct_fields = HashMap::new();
        Self::collect_struct_field_types(node, &mut struct_fields);
        self.struct_field_types = struct_fields;
        let mut return_types = HashMap::new();
        Self::collect_function_return_types(node, &mut return_types);
        self.function_return_types = return_types;
        let mut declarations = Vec::new();
        Self::collect_function_declarations(node, &mut declarations);
        self.profile_fn_names = declarations.iter().map(|(name, _)| name.clone()).collect();
        self.profile_fn_ids = self.profile_fn_names.iter().enumerate().map(|(id, name)| (name.clone(), id)).collect();

        let mut output = String::new();
        writeln!(output, "use tokio;")?;
        writeln!(output, "use nail::std_lib;")?;
        writeln!(output, "use nail::print_macro;")?;
        // Always import Box in case of recursive functions
        writeln!(output, "use std::boxed::Box;")?;
        // Always import rayon and futures since map/filter/reduce are so common
        writeln!(output, "use rayon::prelude::*;")?;
        writeln!(output, "use rayon::iter::IntoParallelIterator;")?;
        writeln!(output, "use futures::future;")?;
        
        // Collect and import all custom types from used stdlib functions
        // (BTreeSet keeps generated import order deterministic)
        let mut custom_type_imports = std::collections::BTreeSet::new();
        for func_name in &self.used_stdlib_functions {
            if let Some(func) = stdlib_registry::get_stdlib_function(func_name) {
                for (type_name, module_path) in &func.custom_type_imports {
                    custom_type_imports.insert((*type_name, *module_path));
                }
            }
        }
        
        // Every stdlib type is reachable by its full path, whether or not a
        // called function imported it. A program can name TUI_Screen in its
        // own signatures without having called tui_run yet, and the type
        // annotations below resolve either way.
        for type_name in stdlib_registry::get_stdlib_type_names() {
            if let Some(full_path) = stdlib_registry::stdlib_type_rust_path(&type_name) {
                self.stdlib_types.insert(type_name, full_path);
            }
        }

        // Generate imports for custom types and populate stdlib_types map
        for (type_name, module_path) in custom_type_imports {
            writeln!(output, "use {}::{};", module_path, type_name)?;
            // Store the mapping for use during struct instantiation
            self.stdlib_types.insert(type_name.to_string(), format!("{}::{}", module_path, type_name));
        }
        
        // Generate imports for required crates
        let required_deps = self.get_required_dependencies();
        if required_deps.contains(&CrateDependency::DashMap) {
            writeln!(output, "use dashmap::DashMap;")?;
        }
        
        // Add serde import if any used functions require serde derives
        let mut needs_serde = false;
        for func_name in &self.used_stdlib_functions {
            if let Some(func) = stdlib_registry::get_stdlib_function(func_name) {
                if func.struct_derives.iter().any(|d| matches!(d, StructDerive::SerdeSerialize | StructDerive::SerdeDeserialize)) {
                    needs_serde = true;
                    break;
                }
            }
        }
        if needs_serde {
            writeln!(output, "use serde::{{Serialize, Deserialize}};")?;
        }
        
        writeln!(output)?;
        if self.wasm_target {
            // A browser program has no main. The browser calls the start
            // function when the wasm module loads, and the program runs as a
            // task on the browser's own event loop.
            writeln!(output, "#[wasm_bindgen::prelude::wasm_bindgen(start)]")?;
            writeln!(output, "pub fn nail_start() {{")?;
            writeln!(output, "    wasm_bindgen_futures::spawn_local(nail_browser_main());")?;
            writeln!(output, "}}")?;
            writeln!(output)?;
            writeln!(output, "async fn nail_browser_main() {{")?;
        } else {
            writeln!(output, "#[tokio::main]")?;
            writeln!(output, "async fn main() {{")?;
        }
        self.indent_level += 1;
        // Browser builds skip instrumentation: no clock, no filesystem, no
        // terminal, so the guards would be dead weight in the wasm binary.
        let emit_profiling = self.profile && !self.wasm_target && !self.profile_fn_names.is_empty();
        if emit_profiling {
            let names: Vec<String> = self.profile_fn_names.iter().map(|name| format!("\"{}\"", name)).collect();
            writeln!(output, "{}nail::prof::init(\"{}\", &[{}]);", self.indent(), self.profile_source_hash.as_deref().unwrap_or(""), names.join(", "))?;
        }
        self.transpile_node(node, &mut output)?;
        if emit_profiling {
            writeln!(output, "{}nail::prof::finish();", self.indent())?;
        }
        self.indent_level -= 1;
        writeln!(output, "}}")?;
        Ok(output)
    }

    fn transpile_node(&mut self, node: &ASTNode, output: &mut String) -> Result<(), CodeError> {
        self.transpile_node_internal(node, output, true)
    }

    /// Transpile an operand of a binary/unary operation, parenthesizing nested
    /// operations so the emitted Rust preserves the AST's grouping instead of
    /// being regrouped by Rust's own operator precedence.
    fn transpile_operand(&mut self, node: &ASTNode, output: &mut String) -> Result<(), CodeError> {
        let needs_parens = matches!(node, ASTNode::BinaryOperation { .. } | ASTNode::UnaryOperation { .. });
        if needs_parens {
            write!(output, "(")?;
        }
        self.transpile_node_internal(node, output, false)?;
        if needs_parens {
            write!(output, ")")?;
        }
        Ok(())
    }

    fn transpile_node_internal(&mut self, node: &ASTNode, output: &mut String, add_semicolons: bool) -> Result<(), CodeError> {
        match node {
            ASTNode::StructDeclarationField { .. } => {
                // This is handled in StructDeclaration
            }
            ASTNode::StructInstantiationField { .. } => {
                // This is handled in StructInstantiation
            }
            ASTNode::Program { statements, .. } => {
                self.push_move_context(node, &[]);
                for stmt in statements {
                    self.transpile_node_internal(stmt, output, add_semicolons)?;
                    if !add_semicolons {
                        writeln!(output)?;
                    }
                }
                self.pop_move_context();
            }
            ASTNode::FunctionDeclaration { name, params, data_type, body, .. } => {
                let is_sync = self.pure_functions.contains(name);
                write!(output, "{}{}fn {}(", self.indent(), if is_sync { "" } else { "async " }, name)?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    write!(output, "{}: {}", param_name, self.rust_type(param_type, name))?;
                }
                writeln!(output, ") -> {} {{", self.rust_type(data_type, name))?;

                // Store the current function's context
                let prev_return_type = self.current_function_return_type.clone();
                let prev_name = self.current_function_name.clone();
                let prev_sync = self.in_sync_function;
                self.current_function_return_type = Some(data_type.clone());
                self.current_function_name = Some(name.clone());
                self.in_sync_function = is_sync;

                let param_names: Vec<String> = params.iter().map(|(param_name, _)| param_name.clone()).collect();
                self.push_move_context(body, &param_names);
                for (param_name, param_type) in params {
                    self.activate_param(param_name, param_type);
                }

                self.indent_level += 1;
                if self.profile {
                    if let Some(id) = self.profile_fn_ids.get(name) {
                        // Bodies emit bare `return`, so measurement rides a
                        // drop guard that fires on every exit path.
                        writeln!(output, "{}let __nail_prof_guard = nail::prof::guard({});", self.indent(), id)?;
                    }
                }
                self.transpile_node_internal(body, output, add_semicolons)?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;

                self.pop_move_context();

                // Restore previous function context
                self.current_function_return_type = prev_return_type;
                self.current_function_name = prev_name;
                self.in_sync_function = prev_sync;
            }
            ASTNode::FunctionCall { name, args, .. } => {
                if add_semicolons {
                    self.transpile_function_call(name, args, output, true)?;
                } else {
                    self.transpile_function_call(name, args, output, false)?;
                }
            }
            ASTNode::ConstDeclaration { name, data_type, value, code_span, .. } => {
                let mutability = if self.reassigned_variables.contains(name) { "mut " } else { "" };
                if add_semicolons {
                    write!(output, "{}let {}{}: {} = ", self.indent(), mutability, name, self.rust_type(data_type, name))?;
                } else {
                    // Inside expression context (like lambdas), don't add indent
                    write!(output, "let {}{}: {} = ", mutability, name, self.rust_type(data_type, name))?;
                }
                self.transpile_node_internal(value, output, false)?;
                self.activate_declaration(name, data_type, code_span);
                if add_semicolons {
                    writeln!(output, ";")?;
                }
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                // Check if this is being used as an expression or statement
                // If add_semicolons is false, we're in expression context
                if !add_semicolons {
                    // Expression context - generate Rust if expression
                    let _num_conditions = condition_branches.len();
                    for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                        if i == 0 {
                            write!(output, "if ")?;
                        } else {
                            write!(output, " else if ")?;
                        }
                        
                        // Always write the condition
                        self.transpile_node_internal(condition, output, false)?;
                        write!(output, " {{ ")?;
                        
                        // For expressions, we need to output all statements and return the last value
                        if let ASTNode::Block { statements, .. } = branch.as_ref() {
                            // Check if any statement in this branch is diverging
                            let mut _found_diverging = false;
                            for (idx, stmt) in statements.iter().enumerate() {
                                if self.statement_contains_diverging_call(stmt) {
                                    // This statement diverges - output it with semicolon and stop
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                    _found_diverging = true;
                                    break;
                                } else if idx < statements.len() - 1 {
                                    // Not the last statement and not diverging - add semicolon
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                } else {
                                    // Last statement and not diverging - it's the return value
                                    if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                                        self.transpile_node_internal(statement, output, false)?;
                                    } else {
                                        self.transpile_node_internal(stmt, output, false)?;
                                    }
                                }
                            }
                        }
                        write!(output, " }}")?;
                    }
                    if let Some(branch) = else_branch {
                        write!(output, " else {{ ")?;
                        if let ASTNode::Block { statements, .. } = branch.as_ref() {
                            // Check if any statement in this branch is diverging
                            let mut _found_diverging = false;
                            for (idx, stmt) in statements.iter().enumerate() {
                                if self.statement_contains_diverging_call(stmt) {
                                    // This statement diverges - output it with semicolon and stop
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                    _found_diverging = true;
                                    break;
                                } else if idx < statements.len() - 1 {
                                    // Not the last statement and not diverging - add semicolon
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                } else {
                                    // Last statement and not diverging - it's the return value
                                    if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                                        self.transpile_node_internal(statement, output, false)?;
                                    } else {
                                        self.transpile_node_internal(stmt, output, false)?;
                                    }
                                }
                            }
                        }
                        write!(output, " }}")?;
                    } else {
                        // No else branch - add unreachable for if expressions
                        write!(output, " else {{ unreachable!(\"Non-exhaustive if expression reached else branch\") }}")?;
                    }
                } else {
                    // Statement context - generate regular if statement
                    let _num_conditions = condition_branches.len();
                    for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                        if i == 0 {
                            write!(output, "{}if ", self.indent())?;
                            self.transpile_node_internal(condition, output, false)?;
                            writeln!(output, " {{")?;
                        } else {
                            write!(output, " else if ")?;
                            self.transpile_node_internal(condition, output, false)?;
                            writeln!(output, " {{")?;
                        }
                        
                        self.indent_level += 1;
                        self.transpile_node_internal(branch, output, add_semicolons)?;
                        self.indent_level -= 1;
                        write!(output, "{}}}", self.indent())?;
                    }
                    if let Some(branch) = else_branch {
                        write!(output, " else {{")?;
                        writeln!(output)?;
                        self.indent_level += 1;
                        self.transpile_node_internal(branch, output, add_semicolons)?;
                        self.indent_level -= 1;
                        write!(output, "{}}}", self.indent())?;
                    }
                    writeln!(output)?;
                }
            }
            ASTNode::Block { statements, .. } => {
                for (i, stmt) in statements.iter().enumerate() {
                    self.transpile_node_internal(stmt, output, add_semicolons)?;
                    // In statement context every statement terminates itself;
                    // a separator is only needed between expression-context statements
                    if !add_semicolons && i < statements.len() - 1 {
                        writeln!(output, ";")?;
                        write!(output, "{}", self.indent())?;
                    }
                }
            }
            ASTNode::ForLoop { iterator, iterable, initial_value, filter, body, .. } => {
                if let Some(element_type) = self.iterable_element_type(iterable) {
                    self.record_variable_type(iterator, element_type);
                }
                // Check if the loop body contains return statements (collecting values)
                let has_returns = self.has_return_statements(body);

                if has_returns {
                    // Generate an imperative collecting loop (since everything is async)
                    if add_semicolons {
                        write!(output, "{}", self.indent())?;
                    }
                    write!(output, "{{")?;
                    writeln!(output)?;
                    self.indent_level += 1;
                    writeln!(output, "{}let mut __result = Vec::new();", self.indent())?;
                    write!(output, "{}for {} in ", self.indent(), iterator)?;
                    self.transpile_iterable(iterable, true, output)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;

                    // The when clause skips elements that don't match
                    if let Some(filter_condition) = filter {
                        write!(output, "{}if !(", self.indent())?;
                        self.transpile_node_internal(filter_condition, output, false)?;
                        writeln!(output, ") {{ continue; }}")?;
                    }

                    // Extract and transpile the yield expression
                    if let ASTNode::Block { statements, .. } = body.as_ref() {
                        for stmt in statements {
                            if let ASTNode::YieldDeclaration { statement, .. } = stmt {
                                write!(output, "{}__result.push(", self.indent())?;
                                self.transpile_node_internal(statement, output, false)?;
                                writeln!(output, ");")?;
                                break;
                            } else if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                                // Legacy return statement support
                                write!(output, "{}__result.push(", self.indent())?;
                                self.transpile_node_internal(statement, output, false)?;
                                writeln!(output, ");")?;
                                break;
                            } else {
                                // Regular statement before yield/return
                                self.transpile_node_internal(stmt, output, true)?;
                            }
                        }
                    }
                    
                    self.indent_level -= 1;
                    writeln!(output, "{}}}", self.indent())?;
                    writeln!(output, "{}__result", self.indent())?;
                    self.indent_level -= 1;
                    write!(output, "{}}}", self.indent())?;
                } else {
                    // Generate a simple for loop for side effects
                    if !add_semicolons {
                        // When used as an expression, wrap in braces to return ()
                        write!(output, "{{")?;
                    }
                    
                    if add_semicolons {
                        write!(output, "{}for {} in ", self.indent(), iterator)?;
                    } else {
                        write!(output, " for {} in ", iterator)?;
                    }
                    self.transpile_iterable(iterable, true, output)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;
                    // The when clause skips elements that don't match
                    if let Some(filter_condition) = filter {
                        write!(output, "{}if !(", self.indent())?;
                        self.transpile_node_internal(filter_condition, output, false)?;
                        writeln!(output, ") {{ continue; }}")?;
                    }
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        write!(output, "}}; () }}")?;
                    }
                }
            }
            ASTNode::MapExpression { iterator, index_iterator, iterable, body, .. } => {
                // Map expressions collect values using Rayon for parallelism
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;

                let mode = self.parallel_iter_mode(body);
                self.write_parallel_iter_open(iterator, index_iterator, iterable, body, "map", mode, output)?;

                // Transpile the body statements and collect result
                if let ASTNode::Block { statements, .. } = body.as_ref() {
                    let num_statements = statements.len();
                    for (i, stmt) in statements.iter().enumerate() {
                        match stmt {
                            // Yield/return statements are the produced value
                            ASTNode::YieldDeclaration { statement, .. } | ASTNode::ReturnDeclaration { statement, .. } => {
                                write!(output, "{}", self.indent())?;
                                self.transpile_node_internal(statement, output, false)?;
                            }
                            _ => {
                                self.transpile_node_internal(stmt, output, true)?;
                            }
                        }
                        if i < num_statements - 1 {
                            writeln!(output)?;
                        }
                    }
                }
                if !output.ends_with('\n') {
                    writeln!(output)?;
                }

                self.write_parallel_iter_close(mode, output)?;
                writeln!(output, "{}__result", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::FilterExpression { iterator, index_iterator, iterable, body, .. } => {
                // Filter expressions collect values that match a condition using Rayon for parallelism
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;

                let mode = self.parallel_iter_mode(body);
                self.write_parallel_iter_open(iterator, index_iterator, iterable, body, "filter_map", mode, output)?;
                self.write_condition_result(body, output)?;

                // Return Some(value) if condition is true, None otherwise
                writeln!(output, "{}if condition_result {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}Some({}.clone())", self.indent(), iterator)?;
                self.indent_level -= 1;
                writeln!(output, "{}}} else {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}None", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;

                self.write_parallel_iter_close(mode, output)?;
                writeln!(output, "{}__result", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::ReduceExpression { iterator, index_iterator, iterable, initial_value, accumulator, body, .. } if self.associative_fold(accumulator, iterator, index_iterator, iterable, body).is_some() => {
                // A fold whose step is provably associative gives the same
                // answer however the work is divided, so it can be split
                // across cores - above the size where threading pays for
                // itself. The initial value is applied once at the end rather
                // than seeded into each chunk, which is why it need not be the
                // operator's identity.
                let fold = self.associative_fold(accumulator, iterator, index_iterator, iterable, body).expect("guard proved this fold associative");
                let step = fold.step;
                self.record_variable_type(accumulator, NailDataTypeDescriptor::Int);

                // What one element contributes, as Rust source, when it is
                // something computed from the element rather than the element
                // itself. It is evaluated inside plain closures, so it renders
                // in a sync context and re-clones whatever it reads from
                // around the fold on each call.
                let mut captured = std::collections::BTreeSet::new();
                let contribution = match fold.element_value {
                    Some(expression) => {
                        if let Some(element_type) = self.iterable_element_type(iterable) {
                            self.record_variable_type(iterator, element_type);
                        }
                        captured = Self::closure_captured_variables(expression, iterator, &None);
                        let mut rendered = String::new();
                        for name in &captured {
                            write!(rendered, "let {} = {}.clone(); ", name, name)?;
                        }
                        self.enter_parallel_closure(ParallelIterMode::Sync);
                        let result = self.transpile_node_internal(expression, &mut rendered, false);
                        self.leave_parallel_closure();
                        result?;
                        Some(rendered)
                    }
                    None => None,
                };

                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                writeln!(output, "{{")?;
                self.indent_level += 1;

                write!(output, "{}let __fold_items = ", self.indent())?;
                self.transpile_iterable(iterable, true, output)?;
                writeln!(output, ".into_iter();")?;
                write!(output, "{}let __fold_init = ", self.indent())?;
                self.transpile_node_internal(initial_value, output, false)?;
                writeln!(output, ";")?;
                for name in &captured {
                    writeln!(output, "{}let {} = {}.clone();", self.indent(), name, name)?;
                }
                // Mapping each element to its contribution is itself work that
                // divides across cores, so it happens inside the parallel
                // iterator rather than before it.
                let (collected, mapped, sequential_step) = match &contribution {
                    Some(rendered) => (
                        "Vec<_>".to_string(),
                        format!(".map(|{}| {{ {} }})", iterator, rendered),
                        format!("|__a, {}| {{ let __b = {{ {} }}; {} }}", iterator, rendered, step.combine("__a", "__b")),
                    ),
                    None => ("Vec<i64>".to_string(), String::new(), format!("|__a, __b| {}", step.combine("__a", "__b"))),
                };
                // An iterator that cannot report its length reports zero here,
                // which keeps the fold sequential - the safe direction.
                writeln!(output, "{}let (__fold_len, _) = __fold_items.size_hint();", self.indent())?;
                writeln!(output, "{}if std_lib::array::worth_parallel_fold(__fold_len) {{", self.indent())?;
                writeln!(output, "{}    let __fold_vec: {} = __fold_items.collect();", self.indent(), collected)?;
                writeln!(output, "{}    let __fold_parallel = __fold_vec.into_par_iter(){}.reduce(|| {}, |__a, __b| {});", self.indent(), mapped, step.identity(), step.combine("__a", "__b"))?;
                writeln!(output, "{}    {}", self.indent(), step.combine("__fold_init", "__fold_parallel"))?;
                writeln!(output, "{}}} else {{", self.indent())?;
                writeln!(output, "{}    __fold_items.fold(__fold_init, {})", self.indent(), sequential_step)?;
                writeln!(output, "{}}}", self.indent())?;

                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::ReduceExpression { iterator, index_iterator, iterable, initial_value, accumulator, body, .. } => {
                // Every other fold stays sequential: an arbitrary step is only
                // safe to regroup if it is associative, and that cannot be
                // proven about code the compiler did not write.
                self.write_sequential_fold(accumulator, iterator, index_iterator, iterable, initial_value, body, false, add_semicolons, output)?;
            }
            ASTNode::ScanExpression { iterator, index_iterator, iterable, initial_value, accumulator, body, .. } => {
                // A scan is the same fold as a reduce, keeping every value the
                // accumulator took instead of only the last. Each of those
                // values depends on every element before it, so a scan is
                // sequential by nature - there is nothing here to parallelise.
                self.write_sequential_fold(accumulator, iterator, index_iterator, iterable, initial_value, body, true, add_semicolons, output)?;
            }
            ASTNode::EachExpression { iterator, index_iterator, iterable, body, .. } => {
                // Each expressions are for side effects only
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;

                self.write_sequential_for_open(iterator, index_iterator, iterable, output)?;

                // Transpile the body statements (no return collection)
                self.transpile_node_internal(body, output, true)?;

                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                writeln!(output, "{}()", self.indent())?; // Each returns unit
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::FindExpression { iterator, index_iterator, iterable, body, .. } => {
                // Find: first matching element (by original order — rayon's
                // find_first) or an error. Parallel with short-circuit.
                let result_expr = if index_iterator.is_some() {
                    "__search_result.map(|__pair| __pair.1).ok_or_else(|| \"find: no element matched the condition\".to_string())"
                } else {
                    "__search_result.ok_or_else(|| \"find: no element matched the condition\".to_string())"
                };
                self.write_parallel_search(iterator, index_iterator, iterable, body, output, add_semicolons, "find_first", true, result_expr)?;
            }
            ASTNode::AllExpression { iterator, index_iterator, iterable, body, .. } => {
                // All: parallel with short-circuit on the first miss
                self.write_parallel_search(iterator, index_iterator, iterable, body, output, add_semicolons, "all", false, "__search_result")?;
            }
            ASTNode::AnyExpression { iterator, index_iterator, iterable, body, .. } => {
                // Any: parallel with short-circuit on the first hit
                self.write_parallel_search(iterator, index_iterator, iterable, body, output, add_semicolons, "any", false, "__search_result")?;
            }
            ASTNode::WhileLoop { condition, max_iterations, body, .. } => {
                if let Some(max_iter) = max_iterations {
                    // Generate a bounded while loop
                    if add_semicolons {
                        writeln!(output, "{}{{", self.indent())?;
                        writeln!(output, "{}    let mut _iterations = 0;", self.indent())?;
                        write!(output, "{}    let _max_iterations = ", self.indent())?;
                        self.transpile_node_internal(max_iter, output, false)?;
                        writeln!(output, ";")?;
                        write!(output, "{}    while ", self.indent())?;
                    } else {
                        writeln!(output, "{{")?;
                        writeln!(output, "    let mut _iterations = 0;")?;
                        write!(output, "    let _max_iterations = ")?;
                        self.transpile_node_internal(max_iter, output, false)?;
                        writeln!(output, ";")?;
                        write!(output, "    while ")?;
                    }
                    self.transpile_node_internal(condition, output, false)?;
                    writeln!(output, " && _iterations < _max_iterations {{")?;
                    self.indent_level += 2;
                    self.transpile_node_internal(body, output, true)?;
                    if add_semicolons {
                        writeln!(output, "{}_iterations += 1;", self.indent())?;
                    } else {
                        writeln!(output, "        _iterations += 1;")?;
                    }
                    self.indent_level -= 2;
                    if add_semicolons {
                        writeln!(output, "{}    }}", self.indent())?;
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        writeln!(output, "    }}")?;
                        write!(output, "}}")?;
                    }
                } else {
                    // Regular unbounded while loop
                    if add_semicolons {
                        write!(output, "{}while ", self.indent())?;
                    } else {
                        write!(output, "while ")?;
                    }
                    self.transpile_node_internal(condition, output, false)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        write!(output, "}}")?;
                    }
                }
            }
            ASTNode::Loop { index_iterator, body, .. } => {
                if let Some(index_name) = index_iterator {
                    self.record_variable_type(index_name, NailDataTypeDescriptor::Int);
                    // Loop with index iterator - needs mutable counter outside loop
                    if add_semicolons {
                        writeln!(output, "{}{{", self.indent())?;
                        self.indent_level += 1;
                        writeln!(output, "{}let mut __loop_index: i64 = 0;", self.indent())?;
                        writeln!(output, "{}loop {{", self.indent())?;
                    } else {
                        writeln!(output, "{{")?;
                        writeln!(output, "let mut __loop_index: i64 = 0;")?;
                        writeln!(output, "loop {{")?;
                    }
                    self.indent_level += 1;
                    // Make index available inside loop as immutable
                    writeln!(output, "{}let {} = __loop_index;", self.indent(), index_name)?;
                    writeln!(output, "{}__loop_index += 1;", self.indent())?;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                        self.indent_level -= 1;
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        writeln!(output, "}}")?;
                        write!(output, "}}")?;
                    }
                } else {
                    // Simple loop without index
                    if add_semicolons {
                        writeln!(output, "{}loop {{", self.indent())?;
                    } else {
                        writeln!(output, "loop {{")?;
                    }
                    self.indent_level += 1;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        write!(output, "}}")?;
                    }
                }
            }
            ASTNode::SpawnBlock { body, .. } => {
                // Spawn a new async task
                if add_semicolons {
                    writeln!(output, "{}tokio::spawn(async move {{", self.indent())?;
                } else {
                    writeln!(output, "tokio::spawn(async move {{")?;
                }
                self.indent_level += 1;
                self.transpile_node_internal(body, output, true)?;
                self.indent_level -= 1;
                if add_semicolons {
                    writeln!(output, "{}}}){};", self.indent(), if add_semicolons { "" } else { "" })?;
                } else {
                    write!(output, "}})")?;
                }
            }
            ASTNode::BreakStatement { .. } => {
                if add_semicolons {
                    writeln!(output, "{}break;", self.indent())?;
                } else {
                    write!(output, "break")?;
                }
            }
            ASTNode::ContinueStatement { .. } => {
                if add_semicolons {
                    writeln!(output, "{}continue;", self.indent())?;
                } else {
                    write!(output, "continue")?;
                }
            }
            ASTNode::ParallelBlock { statements, .. } => {
                self.transpile_parallel_block(statements, output)?;
            }
            ASTNode::ConcurrentBlock { statements, .. } => {
                self.transpile_concurrent_block(statements, output)?;
            }
            ASTNode::BinaryOperation { left, operator, right, .. } => {
                // No string concatenation with + allowed in Nail - use array_join instead
                // Nested operations must be parenthesized: the Nail AST already encodes
                // the intended grouping, and re-emitting flat text would let Rust's
                // operator precedence regroup it (e.g. (2 + 3) * 4 becoming 2 + 3 * 4).
                self.transpile_operand(left, output)?;
                write!(output, " {} ", self.rust_operator(operator))?;
                self.transpile_operand(right, output)?;
            }
            ASTNode::UnaryOperation { operator, operand, .. } => {
                write!(output, "{}", self.rust_operator(operator))?;
                self.transpile_operand(operand, output)?;
            }
            ASTNode::Identifier { name, code_span, .. } => {
                let expression = self.identifier_expression(name, code_span);
                write!(output, "{}", expression)?;
            }
            ASTNode::NumberLiteral { value, data_type, .. } => {
                // Suffix the literal so untyped contexts (like macro arguments)
                // don't fall back to Rust's i32 default and overflow on large values
                match data_type {
                    NailDataTypeDescriptor::Int => write!(output, "{}i64", value)?,
                    NailDataTypeDescriptor::Float => write!(output, "{}f64", value)?,
                    _ => write!(output, "{}", value)?,
                }
            }
            ASTNode::StringLiteral { value, .. } => {
                // For multiline strings or strings with backslashes, use raw strings
                // Quotes don't need escaping in backtick strings and work fine in raw strings
                if value.contains('\n') || value.contains('\t') || value.contains('\\') || value.contains('"') {
                    // Use raw string literal with enough # symbols to avoid conflicts
                    let mut delimiter = String::from("#");
                    while value.contains(&format!("\"{}", delimiter)) || value.contains(&format!("#{}", delimiter)) {
                        delimiter.push('#');
                    }
                    write!(output, "r{0}\"{1}\"{0}.to_string()", delimiter, value)?;
                } else {
                    // Use regular string literal for simple strings
                    write!(output, "\"{}\".to_string()", value)?;
                }
            }
            ASTNode::BooleanLiteral { value, .. } => {
                write!(output, "{}", value)?;
            }
            ASTNode::NestedFieldAccess { object, field_name, .. } => {
                self.transpile_node_internal(object, output, false)?;
                write!(output, ".{}", field_name)?;
            }

            ASTNode::ReturnDeclaration { statement, .. } => {
                // In collection operations, return statements should just be the expression value
                if self.in_collection_operation {
                    self.transpile_node_internal(statement, output, false)?;
                } else {
                    if add_semicolons {
                        write!(output, "{}return ", self.indent())?;
                    } else {
                        write!(output, "return ")?;
                    }

                    // Check if we need to wrap in Ok() for result types
                    let needs_ok_wrap = if let Some(return_type) = &self.current_function_return_type {
                        match return_type {
                            NailDataTypeDescriptor::Any => false,
                            NailDataTypeDescriptor::Result(_) => true,
                            _ => false,
                        }
                    } else {
                        false
                    };

                    // Check if the statement is already an error (e() call)
                    let is_error_call = match statement.as_ref() {
                        ASTNode::FunctionCall { name, .. } => name == "e",
                        _ => false,
                    };

                    if needs_ok_wrap && !is_error_call {
                        write!(output, "Ok(")?;
                        self.transpile_node_internal(statement, output, false)?;
                        write!(output, ")")?;
                    } else {
                        self.transpile_node_internal(statement, output, false)?;
                    }
                    
                    if add_semicolons {
                        writeln!(output, ";")?;
                    }
                }
            }
            ASTNode::YieldDeclaration { statement, .. } => {
                // Yield statements should only be used in collection operations
                if self.in_collection_operation {
                    self.transpile_node_internal(statement, output, false)?;
                } else {
                    // This should be caught by the type checker, but just in case
                    write!(output, "/* ERROR: yield outside collection operation */")?;
                }
            }
            ASTNode::StructDeclaration { name, fields, .. } => {
                // Collect all derives needed for this struct based on used stdlib functions
                let mut derives = vec!["Debug", "Clone", "PartialEq"];
                let mut needs_serde = false;
                
                for func_name in &self.used_stdlib_functions {
                    if let Some(func) = stdlib_registry::get_stdlib_function(func_name) {
                        for derive in &func.struct_derives {
                            match derive {
                                StructDerive::SerdeSerialize | 
                                StructDerive::SerdeDeserialize => {
                                    needs_serde = true;
                                }
                                _ => {} // Other derives handled by default
                            }
                        }
                    }
                }
                
                if needs_serde {
                    derives.push("serde::Serialize");
                    derives.push("serde::Deserialize");
                }
                
                writeln!(output, "{}#[derive({})]", self.indent(), derives.join(", "))?;
                writeln!(output, "{}struct {} {{", self.indent(), name)?;
                self.indent_level += 1;
                for field in fields {
                    match field {
                        ASTNode::StructDeclarationField { name: field_name, data_type, .. } => {
                            writeln!(output, "{}{}: {},", self.indent(), field_name, self.rust_type(data_type, field_name))?;
                        }
                        other => return Err(CodeError { help: None, message: format!("Struct declaration '{}' contains a non-field node", name), code_span: other.code_span() }),
                    }
                }
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::EnumDeclaration { name, variants, .. } => {
                writeln!(output, "{}#[derive(Debug, Clone, PartialEq)]", self.indent())?;
                writeln!(output, "{}enum {} {{", self.indent(), name)?;
                self.indent_level += 1;
                for variant in variants {
                    match variant {
                        ASTNode::EnumVariant { variant, .. } => {
                            writeln!(output, "{}{},", self.indent(), variant)?;
                        }
                        other => return Err(CodeError { help: None, message: format!("Enum declaration '{}' contains a non-variant node", name), code_span: other.code_span() }),
                    }
                }
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::LambdaDeclaration { params, body, .. } => {
                // Generate a regular closure that returns an async block
                write!(output, "move |")?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    write!(output, "{}: {}", param_name, self.rust_type(param_type, ""))?;
                }
                write!(output, "| {{ async move {{ ")?;

                let param_names: Vec<String> = params.iter().map(|(param_name, _)| param_name.clone()).collect();
                self.push_move_context(body, &param_names);
                for (param_name, param_type) in params {
                    self.activate_param(param_name, param_type);
                }

                // Transpile the body inline
                if let ASTNode::Block { statements, .. } = body.as_ref() {
                    for (i, stmt) in statements.iter().enumerate() {
                        if i > 0 {
                            write!(output, "; ")?;
                        }
                        self.transpile_node_internal(stmt, output, false)?;
                    }
                }

                self.pop_move_context();
                write!(output, " }} }}")?;
            }
            ASTNode::StructInstantiation { name, fields, .. } => {
                // Check if this is a stdlib struct and use the full path if so
                let struct_name = if let Some(full_path) = self.stdlib_types.get(name) {
                    full_path.clone()
                } else {
                    name.clone()
                };
                write!(output, "{} {{", struct_name)?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    match field {
                        ASTNode::StructInstantiationField { name: field_name, value, .. } => {
                            write!(output, " {}: ", field_name)?;
                            self.transpile_node_internal(value, output, false)?;
                        }
                        other => return Err(CodeError { help: None, message: "Struct instantiation contains a non-field node".to_string(), code_span: other.code_span() }),
                    }
                }
                write!(output, " }}")?;
            }
            ASTNode::StructFieldAccess { struct_name, field_name, code_span, .. } => {
                // Copy-typed fields (i/f/b) don't need a clone
                let field_is_copy = match self.reference_type(struct_name, code_span) {
                    Some(NailDataTypeDescriptor::Struct(type_name)) => matches!(
                        self.struct_field_types.get(type_name).and_then(|fields| fields.get(field_name)),
                        Some(NailDataTypeDescriptor::Int | NailDataTypeDescriptor::Float | NailDataTypeDescriptor::Boolean)
                    ),
                    _ => false,
                };
                if field_is_copy {
                    write!(output, "{}.{}", struct_name, field_name)?;
                } else {
                    write!(output, "{}.{}.clone()", struct_name, field_name)?;
                }
            }
            ASTNode::EnumVariant { name, variant, .. } => {
                write!(output, "{}{}::{}", self.indent(), name, variant)?;
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                write!(output, "vec! [")?;
                for (i, value) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    self.transpile_node_internal(value, output, false)?;
                }
                write!(output, "]")?;
            }
            ASTNode::Assignment { left, right, .. } => {
                // Transpile assignment: left = right
                // For assignment left-hand side, don't clone - just use the variable name
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                if let ASTNode::Identifier { name, .. } = left.as_ref() {
                    write!(output, "{}", name)?;
                } else {
                    self.transpile_node_internal(left, output, false)?;
                }
                write!(output, " = ")?;
                self.transpile_node_internal(right, output, false)?;
                if add_semicolons {
                    writeln!(output, ";")?;
                }
            }
        }
        Ok(())
    }

    fn rust_type(&self, data_type: &NailDataTypeDescriptor, _name: &str) -> String {
        match data_type {
            NailDataTypeDescriptor::String => "String".to_string(),
            NailDataTypeDescriptor::Int => "i64".to_string(),
            NailDataTypeDescriptor::Float => "f64".to_string(),
            NailDataTypeDescriptor::Boolean => "bool".to_string(),
            // A stdlib type is written by its full path, so a signature
            // mentioning one compiles whether or not anything imported it.
            NailDataTypeDescriptor::Struct(name) => self.stdlib_types.get(name).cloned().unwrap_or_else(|| name.to_string()),
            NailDataTypeDescriptor::Enum(name) => self.stdlib_types.get(name).cloned().unwrap_or_else(|| name.to_string()),
            NailDataTypeDescriptor::Void => "()".to_string(),
            NailDataTypeDescriptor::Never => "!".to_string(),
            NailDataTypeDescriptor::Error => "String".to_string(),
            NailDataTypeDescriptor::Array(inner) => format!("Vec<{}>", self.rust_type(inner, _name)),
            NailDataTypeDescriptor::Any => "Box<dyn std::any::Any>".to_string(),
            NailDataTypeDescriptor::Result(inner_type) => {
                format!("Result<{}, String>", self.rust_type(inner_type, _name))
            }
            NailDataTypeDescriptor::Fn(_, _) => panic!("NailDataTypeDescriptor::Fn data type found during transpilation. This should not happen."),
            NailDataTypeDescriptor::OneOf(_) => panic!("NailDataTypeDescriptor::OneOf found during transpilation. This should not happen."),
            NailDataTypeDescriptor::HashMap(key_type, value_type) => {
                format!("DashMap<{}, {}>", self.rust_type(key_type, _name), self.rust_type(value_type, _name))
            }
            NailDataTypeDescriptor::FailedToResolve => panic!("NailDataTypeDescriptor::FailedToResolve found during transpilation. This should not happen."),
            NailDataTypeDescriptor::TypeVar(name, _) => panic!("NailDataTypeDescriptor::TypeVar({}) found during transpilation. Type variables must be resolved by the checker.", name),
        }
    }

    fn rust_async_return_type(&self, data_type: &NailDataTypeDescriptor, name: &str) -> String {
        format!("{}", self.rust_type(data_type, name))
    }

    fn rust_operator(&self, op: &Operation) -> &'static str {
        match op {
            Operation::Add => "+",
            Operation::Sub => "-",
            Operation::Mul => "*",
            Operation::Div => "/",
            Operation::Mod => "%",
            Operation::Eq => "==",
            Operation::Ne => "!=",
            Operation::Lt => "<",
            Operation::Lte => "<=",
            Operation::Gt => ">",
            Operation::Gte => ">=",
            Operation::Not => "!",
            Operation::Neg => "-",
            Operation::And => "&&",
            Operation::Or => "||",
        }
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }


    fn transpile_function_call(&mut self, name: &str, args: &[ASTNode], output: &mut String, add_indent: bool) -> Result<(), CodeError> {
        // Special handling for error-related functions
        if name == "e" {
            // e(message) - create an error with context
            if args.len() != 1 {
                return Err(CodeError { help: None, message: format!("e expects 1 argument, got {}", args.len()), code_span: args.first().map(|a| a.code_span()).unwrap_or_default() });
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Add function context to error messages, matching the stdlib
            // runtime error convention: "function_name: message"
            write!(output, "Err(format!(\"{}: {{}}\", ", self.current_function_name.as_ref().unwrap_or(&"unknown".to_string()))?;
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, "))")?;

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "safe" {
            // safe(expression, |e|:T { handler })
            if args.len() != 2 {
                return Err(CodeError { help: None, message: format!("safe expects 2 arguments, got {}", args.len()), code_span: args.first().map(|a| a.code_span()).unwrap_or_default() });
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Generate: match expression { Ok(v) => v, Err(e) => (handler)(e) }
            // awaiting the handler only when it is actually async
            let handler_is_sync = matches!(&args[1], ASTNode::Identifier { name: handler, .. } if self.pure_functions.contains(handler));
            write!(output, "match ")?;
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, " {{ Ok(v) => v, Err(e) => (")?;

            // The second argument is a handler function name or lambda
            self.transpile_node_internal(&args[1], output, false)?;
            if handler_is_sync {
                write!(output, ")(e) }}")?;
            } else {
                write!(output, ")(e).await }}")?;
            }

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "danger" || name == "expect" {
            // danger/expect(expression) - unwrap with a custom panic message
            // (semantically identical; they differ only in programmer intent)
            if args.len() != 1 {
                return Err(CodeError { help: None, message: format!("{} expects 1 argument, got {}", name, args.len()), code_span: args.first().map(|a| a.code_span()).unwrap_or_default() });
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Check if the argument is a function call that needs .await
            if let ASTNode::FunctionCall { name: inner_name, args: inner_args, .. } = &args[0] {
                // It's a function call - transpile it properly with .await if needed
                self.transpile_function_call(inner_name, inner_args, output, false)?;
            } else {
                // Not a function call, transpile normally
                self.transpile_node_internal(&args[0], output, false)?;
            }
            write!(output, ".unwrap_or_else(|nail_error| panic!(\"🔨 Nail Error: {{}}\", nail_error))")?;

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        }

        // A stdlib function that folds a file gets a loop rather than a call: the
        // file is read a batch of lines at a time and the program's step function
        // is run over each one, awaited when that function does I/O. The runtime
        // function names come from the registry.
        if let (Some(fold), 3) = (stdlib_registry::file_fold(name), args.len()) {
            if let ASTNode::Identifier { name: step_function, .. } = &args[2] {
                self.used_stdlib_functions.insert(name.to_string());
                let step_is_async = !self.pure_functions.contains(step_function);

                if add_indent {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "async {{ let nail_fold_reader = {}(", fold.open)?;
                self.transpile_node_internal(&args[0], output, false)?;
                write!(output, ").await?; let mut nail_fold_total = ")?;
                self.transpile_node_internal(&args[1], output, false)?;
                write!(output, "; loop {{ let nail_fold_lines = {}(&nail_fold_reader, {}).await?; if nail_fold_lines.is_empty() {{ break; }} for nail_fold_line in nail_fold_lines {{ nail_fold_total = {}(nail_fold_total, nail_fold_line)", fold.next_lines, fold.lines_at_a_time, step_function)?;
                if step_is_async {
                    write!(output, ".await")?;
                }
                write!(output, "; }} }} {}(&nail_fold_reader).await?; Ok::<_, String>(nail_fold_total) }}.await", fold.close)?;
                if add_indent {
                    writeln!(output, ";")?;
                }
                return Ok(());
            }
        }

        // A stdlib function that takes a key function gets the keys rather than
        // the function: they are worked out here, in a loop that can await, so
        // nothing downstream ever calls back into the program. The registry says
        // which functions these are, so no name is baked in here.
        if stdlib_registry::precomputes_key_argument(name) && args.len() == 2 {
            if let (Some(stdlib_fn), ASTNode::Identifier { name: key_function, .. }) = (stdlib_registry::get_stdlib_function(name), &args[1]) {
                self.used_stdlib_functions.insert(name.to_string());
                let key_is_async = !self.pure_functions.contains(key_function);

                if add_indent {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{ let nail_key_items = ")?;
                self.transpile_node_internal(&args[0], output, false)?;
                write!(output, "; let mut nail_keys = Vec::with_capacity(nail_key_items.len()); for nail_key_item in nail_key_items.iter() {{ nail_keys.push({}(nail_key_item.clone())", key_function)?;
                if key_is_async {
                    write!(output, ".await")?;
                }
                write!(output, "); }} {}(nail_key_items, nail_keys) }}", stdlib_fn.rust_path)?;
                if add_indent {
                    writeln!(output, ";")?;
                }
                return Ok(());
            }
        }

        // Check if it's a stdlib function
        if let Some(stdlib_fn) = stdlib_registry::get_stdlib_function(name) {
            // Track that we're using this stdlib function
            self.used_stdlib_functions.insert(name.to_string());
            // All stdlib functions are regular function calls now
            if add_indent {
                write!(output, "{}{}", self.indent(), stdlib_fn.rust_path)?;
            } else {
                write!(output, "{}", stdlib_fn.rust_path)?;
            }

            // Special case for macros (rust_path ending in "!"), e.g. print_macro!
            if stdlib_fn.rust_path.ends_with("!") {
                write!(output, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    self.transpile_node_internal(arg, output, false)?;
                }
                write!(output, ")")?;
            } else {
                // Regular functions
                write!(output, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    
                    // Check if this parameter should be passed by reference
                    let pass_by_ref = if i < stdlib_fn.parameters.len() {
                        stdlib_fn.parameters[i].pass_by_reference
                    } else {
                        false
                    };
                    
                    if pass_by_ref {
                        // Pass by reference
                        if let ASTNode::Identifier { name, .. } = arg {
                            write!(output, "&{}", name)?;
                        } else {
                            write!(output, "&")?;
                            self.transpile_node_internal(arg, output, false)?;
                        }
                    } else {
                        self.transpile_node_internal(arg, output, false)?;
                    }
                }

                // Stdlib functions that dispatch back into the program receive
                // the target function as a trailing argument. The name comes
                // from the registry, so no function name is baked in here.
                for (position, callback) in stdlib_registry::get_handler_callbacks(name).map(|callbacks| callbacks.as_slice()).unwrap_or(&[]).iter().enumerate() {
                    let parameters: Vec<String> = (0..callback.parameter_types.len()).map(|index| format!("callback_argument_{}", index)).collect();
                    let argument_list = parameters.join(", ");
                    if !args.is_empty() || position > 0 {
                        write!(output, ", ")?;
                    }
                    // An optional callback the program left out is filled by a
                    // stand-in that hands back one of its own arguments, which
                    // the registry names.
                    match callback.optional_stand_in {
                        Some(kept) if !self.function_return_types.contains_key(callback.function_name) => {
                            let dropped: Vec<String> = parameters.iter().enumerate().filter(|(index, _)| *index != kept).map(|(_, parameter)| parameter.clone()).collect();
                            write!(output, "move |{}| Box::pin(async move {{ let _ = ({},); callback_argument_{} }})", argument_list, dropped.join(", "), kept)?;
                        }
                        _ => {
                            write!(output, "move |{}| Box::pin({}({}))", argument_list, callback.function_name, argument_list)?;
                        }
                    }
                }

                write!(output, ")")?;

                // Await only the stdlib functions whose Rust implementations
                // are async (I/O); pure computation functions are plain sync
                if !stdlib_fn.rust_path.ends_with('!') && stdlib_registry::is_stdlib_fn_async(name) {
                    write!(output, ".await")?;
                }

                // Note: Result types should be handled by the type checker and
                // explicit error handling functions like danger() or safe().
                // The transpiler should not automatically unwrap Results.
            }
            if add_indent {
                writeln!(output, ";")?;
            }
        } else {
            // User-defined function
            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            let is_pure = self.pure_functions.contains(name);
            if is_pure {
                // Pure functions are plain sync fns: direct call, plain
                // recursion, no future involved.
                write!(output, "{}(", name)?;
            } else {
                // Async user functions: any call cycle (direct or mutual
                // recursion) would make the future type infinitely sized.
                // Boxing every async user-function call breaks all possible
                // cycles uniformly, with no per-function special cases.
                write!(output, "Box::pin({}(", name)?;
            }

            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ")?;
                }
                self.transpile_node_internal(arg, output, false)?;
            }

            if is_pure {
                write!(output, ")")?;
            } else {
                write!(output, ")).await")?;
            }

            if add_indent {
                writeln!(output, ";")?;
            }
        }

        Ok(())
    }

    fn transpile_concurrent_block(&mut self, statements: &[ASTNode], output: &mut String) -> Result<(), CodeError> {
        if statements.is_empty() {
            return Ok(());
        }

        // Extract variable names from const declarations and generate expressions
        let mut var_names = Vec::new();
        let mut expressions = Vec::new();
        let mut declarations = Vec::new();

        for stmt in statements.iter() {
            match stmt {
                ASTNode::ConstDeclaration { name, data_type, value, code_span, .. } => {
                    var_names.push(name.clone());
                    expressions.push(value.as_ref());
                    declarations.push((name.clone(), data_type.clone(), code_span.clone()));
                }
                ASTNode::FunctionCall { .. } => {
                    // Function calls that don't assign to variables get a placeholder name
                    var_names.push("_".to_string());
                    expressions.push(stmt);
                }
                _ => {
                    // Other statements get placeholder names
                    var_names.push("_".to_string());
                    expressions.push(stmt);
                }
            }
        }

        // Generate the destructuring assignment with actual variable names
        write!(output, "{}let (", self.indent())?;
        for (i, var_name) in var_names.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}", var_name)?;
        }
        writeln!(output, ") = tokio::join!(")?;

        // Generate the async blocks that return the computed values
        self.indent_level += 1;
        for (i, expr) in expressions.iter().enumerate() {
            if i > 0 {
                writeln!(output, ",")?;
            }
            write!(output, "{}async {{ ", self.indent())?;

            // For const declarations, return just the value expression
            // For other expressions, return the expression itself
            self.transpile_node_internal(expr, output, false)?;

            write!(output, " }}")?;
        }
        self.indent_level -= 1;
        writeln!(output)?;
        writeln!(output, "{});", self.indent())?;

        // The tuple destructure brings the declared names into scope; register
        // them so later references know their types (Copy types stay bare).
        for (name, data_type, code_span) in &declarations {
            self.activate_declaration(name, data_type, code_span);
        }

        Ok(())
    }

    fn transpile_parallel_block(&mut self, statements: &[ASTNode], output: &mut String) -> Result<(), CodeError> {
        if statements.is_empty() {
            return Ok(());
        }

        // Extract variable names from const declarations and generate expressions
        let mut var_names = Vec::new();
        let mut expressions = Vec::new();
        let mut declarations = Vec::new();

        for stmt in statements.iter() {
            match stmt {
                ASTNode::ConstDeclaration { name, data_type, value, code_span, .. } => {
                    var_names.push(name.clone());
                    expressions.push(value.as_ref());
                    declarations.push((name.clone(), data_type.clone(), code_span.clone()));
                }
                ASTNode::FunctionCall { .. } => {
                    // Function calls that don't assign to variables get a placeholder name
                    var_names.push("_".to_string());
                    expressions.push(stmt);
                }
                _ => {
                    // Other statements get placeholder names
                    var_names.push("_".to_string());
                    expressions.push(stmt);
                }
            }
        }

        // Generate thread spawning and joining
        write!(output, "{}let (", self.indent())?;
        for (i, var_name) in var_names.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}", var_name)?;
        }
        writeln!(output, ") = {{")?;

        self.indent_level += 1;
        
        // Spawn threads for each expression. Statements may call stdlib functions,
        // which are async, so each thread gets a handle to the tokio runtime and
        // drives its expression to completion with block_on. Outer variables are
        // cloned per thread before the move closure so several statements can
        // capture the same variable.
        for (i, expr) in expressions.iter().enumerate() {
            let mut bound = HashSet::new();
            let mut free = std::collections::BTreeSet::new();
            Self::collect_free_variables(expr, &mut bound, &mut free);
            write!(output, "{}let handle{} = std::thread::spawn({{ let __rt_handle = tokio::runtime::Handle::current(); ", self.indent(), i)?;
            for var_name in &free {
                write!(output, "let {0} = {0}.clone(); ", var_name)?;
            }
            write!(output, "move || {{ __rt_handle.block_on(async move {{ ")?;
            self.transpile_node_internal(expr, output, false)?;
            writeln!(output, " }}) }} }});")?;
        }
        
        // Join all threads and collect results
        write!(output, "{}(", self.indent())?;
        for (i, _) in expressions.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "handle{}.join().unwrap()", i)?;
        }
        writeln!(output, ")")?;

        self.indent_level -= 1;
        writeln!(output, "{}}};", self.indent())?;

        // The tuple destructure brings the declared names into scope; register
        // them so later references know their types (Copy types stay bare).
        for (name, data_type, code_span) in &declarations {
            self.activate_declaration(name, data_type, code_span);
        }

        Ok(())
    }

    fn transpile_parallel_assignment(&mut self, assignments: &[(String, NailDataTypeDescriptor, Box<ASTNode>)], output: &mut String) -> Result<(), CodeError> {
        if assignments.is_empty() {
            return Ok(());
        }

        // Generate the destructuring assignment with actual variable names
        write!(output, "{}let (", self.indent())?;
        for (i, (var_name, _, _)) in assignments.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}", var_name)?;
        }
        writeln!(output, ") = tokio::join!(")?;

        // Generate the async blocks that return the computed values
        self.indent_level += 1;
        for (i, (_, _, value)) in assignments.iter().enumerate() {
            if i > 0 {
                writeln!(output, ",")?;
            }
            write!(output, "{}async {{ ", self.indent())?;

            // Return the value expression
            self.transpile_node_internal(value, output, false)?;

            write!(output, " }}")?;
        }
        self.indent_level -= 1;
        writeln!(output)?;
        writeln!(output, "{});", self.indent())?;

        Ok(())
    }

    /// Check if a statement contains a diverging function call (like panic or todo)
    fn statement_contains_diverging_call(&self, stmt: &ASTNode) -> bool {
        match stmt {
            ASTNode::FunctionCall { name, .. } => {
                if let Some(stdlib_fn) = stdlib_registry::get_stdlib_function(name) {
                    stdlib_fn.diverging
                } else {
                    false
                }
            }
            ASTNode::ReturnDeclaration { statement, .. } => {
                self.statement_contains_diverging_call(statement)
            }
            ASTNode::YieldDeclaration { statement, .. } => {
                self.statement_contains_diverging_call(statement)
            }
            // Check inside blocks for the last statement
            ASTNode::Block { statements, .. } => {
                statements.last()
                    .map(|s| self.statement_contains_diverging_call(s))
                    .unwrap_or(false)
            }
            _ => false
        }
    }
    
}

fn insert_semicolons(code: String) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let mut new_line = line.to_string();

        // check if the line ends with an await, .to_string(), or a number with a white space or a \n after it and add a ; in that case or if ends with a number
        if trimmed.ends_with("await") || trimmed.ends_with(".to_string()") || trimmed.chars().last().unwrap_or_default().is_ascii_digit() {
            // or if it ends with a number ||
            let next_line = lines.get(i + 1).unwrap_or(&"");
            if next_line.trim().is_empty() || next_line.trim().starts_with("//") {
                new_line.push(';');
            }
        }

        result.push(new_line);
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::checker;
    use crate::lexer::lexer;
    use crate::parser::parse;

    fn transpile_source(code: &str) -> String {
        let tokens = lexer(code);
        let mut ast = parse(tokens).expect("Parse should succeed");
        checker(&mut ast).expect("Type check should succeed");
        Transpiler::new().transpile(&ast).expect("Transpilation should succeed")
    }

    #[test]
    fn test_import_transpiles_identically_to_import_dangerous() {
        // Sandboxing is a compile-time proof with zero runtime cost: a compliant
        // file must produce byte-identical Rust either way it is included.
        let trusted_side = "greeting:s = shout_greeting(`world`);\nprint(greeting);";
        let via_import_dangerous = format!("import_dangerous(`tests/test_import_helper.nail`)\n{}", trusted_side);
        let via_import = format!("import(`tests/test_import_helper.nail`)\n{}", trusted_side);
        assert_eq!(transpile_source(&via_import_dangerous), transpile_source(&via_import));
    }

    #[test]
    fn test_profile_emits_init_guard_and_finish() {
        let code = "f double_it(value:i):i { r value * 2; }\nresult:i = double_it(21);\nprint(result);";
        let tokens = lexer(code);
        let mut ast = parse(tokens).expect("Parse should succeed");
        checker(&mut ast).expect("Type check should succeed");
        let mut transpiler = Transpiler::new();
        transpiler.profile = true;
        let instrumented = transpiler.transpile(&ast).expect("Transpilation should succeed");
        assert!(instrumented.contains("nail::prof::init(\"\", &[\"double_it\"]);"), "main starts by registering every user function");
        assert!(instrumented.contains("let __nail_prof_guard = nail::prof::guard(0);"), "function bodies open with a drop guard");
        assert!(instrumented.contains("nail::prof::finish();"), "main ends with the report call");
    }

    #[test]
    fn test_profile_off_emits_nothing_and_skips_fnless_programs() {
        let code = "f double_it(value:i):i { r value * 2; }\nresult:i = double_it(21);\nprint(result);";
        assert!(!transpile_source(code).contains("nail::prof"), "library default is uninstrumented");

        let fnless = "print(`hi`);";
        let tokens = lexer(fnless);
        let mut ast = parse(tokens).expect("Parse should succeed");
        checker(&mut ast).expect("Type check should succeed");
        let mut transpiler = Transpiler::new();
        transpiler.profile = true;
        let out = transpiler.transpile(&ast).expect("Transpilation should succeed");
        assert!(!out.contains("nail::prof"), "no user functions means nothing to profile");
    }
}
