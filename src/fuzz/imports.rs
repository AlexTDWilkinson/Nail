//! Multi-file cases for the promise `import` makes.
//!
//! `import` splices another file in sandboxed: that file may only declare
//! functions, structs, enums and constants, and everything reachable from it
//! may only compute. The other engines write one program at a time, so
//! nothing they produce can test a promise about two files. This one writes a
//! helper and a program that imports it, and knows before it compiles which
//! answer the compiler owes.
//!
//! Both answers are generated on purpose. A helper that only computes has to
//! be accepted, and a helper that reaches the world, directly or through a
//! chain of its own functions or through a function of the importing program,
//! has to be refused. Accepted when it should be refused is the serious
//! finding: that is somebody else's code touching this machine. Refused when
//! it should be accepted is the other half of the same promise, because a
//! sandbox that refuses honest code makes `import` useless.
//!
//! What is denied and what is allowed comes from the registry at runtime,
//! never from a list written here, so a function added to the library
//! tomorrow is fuzzed tomorrow.

use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::checker::checker;
use crate::common::NailDataTypeDescriptor;
use crate::fuzz::oracle::{Finding, Property};
use crate::lexer::{collect_lexer_errors, lex_program};
use crate::parser::parse;
use crate::transpiler::Transpiler;

/// Every file a case writes starts with the version line, the same as every
/// file a person writes.
const VERSION_LINE: &str = "nail latest";

// ---------------------------------------------------------------------------
// What a case is
// ---------------------------------------------------------------------------

/// The answer the compiler owes a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    Refused,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Accepted => "accepted",
            Verdict::Refused => "refused",
        }
    }
}

/// What a case is built to exercise. The kind decides which files are written
/// and which answer is owed, so a finding names one of these and a person
/// knows immediately what broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A helper that only computes, brought in with import.
    PureHelper,
    /// A helper whose functions call each other before computing, so the
    /// checker's transitive walk has a chain to follow that ends in nothing.
    PureChain,
    /// A helper that itself imports a second pure file.
    PureNestedImport,
    /// A helper that declares a struct and an enum the program then uses.
    PureStructHelper,
    /// Two helpers importing the same third file, which an import may splice
    /// only once.
    Diamond,
    /// A helper that reaches the world, brought in with import_dangerous,
    /// where reaching the world is exactly what was asked for.
    DangerousHelper,
    /// Sandboxed code calling something the registry denies it.
    DeniedCall,
    /// Sandboxed code reaching the same call through a function of its own.
    DeniedThroughOwnChain,
    /// Sandboxed code laundering the call through a function of the importing
    /// program, which is trusted only until sandboxed code can reach it.
    DeniedThroughTrustedHelper,
    /// The same laundering by naming the function rather than calling it, as
    /// the error handler of a safe().
    DeniedThroughHandlerReference,
    /// A sandboxed constant whose initializer reaches the world when the
    /// program starts.
    DeniedInConstant,
    /// A sandboxed file with a statement at the top level, which would run
    /// when the importing program starts rather than when it chose.
    TopLevelStatement,
    /// A sandboxed file that imports itself.
    SelfImport,
    /// Two sandboxed files that import each other.
    MutualImport,
    /// One file brought in both ways, which would give it two capabilities.
    BothCapabilities,
    /// Sandboxed code using import_dangerous to bring in a third file, which
    /// would be the sandbox letting itself out.
    DangerousInsideSandbox,
    /// The denied call one import further down, in a file the helper imports.
    DeniedInNestedImport,
    /// A sandboxed file declaring a function the importing program has
    /// already declared. Only one of the two can be the one that runs, so if
    /// the program were allowed to keep both, the walk that checks sandboxed
    /// bodies could be given the wrong one.
    ShadowedDeclaration,
}

impl Kind {
    /// Every kind, which is also the order a run walks them in: seed N writes
    /// kind N modulo this length, so any range of seeds covers all of them.
    pub const ALL: &'static [Kind] = &[
        Kind::PureHelper,
        Kind::DeniedCall,
        Kind::PureChain,
        Kind::DeniedThroughOwnChain,
        Kind::PureNestedImport,
        Kind::DeniedThroughTrustedHelper,
        Kind::PureStructHelper,
        Kind::DeniedThroughHandlerReference,
        Kind::Diamond,
        Kind::DeniedInConstant,
        Kind::DangerousHelper,
        Kind::TopLevelStatement,
        Kind::SelfImport,
        Kind::MutualImport,
        Kind::BothCapabilities,
        Kind::DangerousInsideSandbox,
        Kind::DeniedInNestedImport,
        Kind::ShadowedDeclaration,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::PureHelper => "pure-helper",
            Kind::PureChain => "pure-chain",
            Kind::PureNestedImport => "pure-nested-import",
            Kind::PureStructHelper => "pure-struct-helper",
            Kind::Diamond => "diamond",
            Kind::DangerousHelper => "dangerous-helper",
            Kind::DeniedCall => "denied-call",
            Kind::DeniedThroughOwnChain => "denied-through-own-chain",
            Kind::DeniedThroughTrustedHelper => "denied-through-trusted-helper",
            Kind::DeniedThroughHandlerReference => "denied-through-handler-reference",
            Kind::DeniedInConstant => "denied-in-constant",
            Kind::TopLevelStatement => "top-level-statement",
            Kind::SelfImport => "self-import",
            Kind::MutualImport => "mutual-import",
            Kind::BothCapabilities => "both-capabilities",
            Kind::DangerousInsideSandbox => "dangerous-inside-sandbox",
            Kind::DeniedInNestedImport => "denied-in-nested-import",
            Kind::ShadowedDeclaration => "shadowed-declaration",
        }
    }

    pub fn verdict(self) -> Verdict {
        match self {
            Kind::PureHelper | Kind::PureChain | Kind::PureNestedImport | Kind::PureStructHelper | Kind::Diamond | Kind::DangerousHelper => Verdict::Accepted,
            _ => Verdict::Refused,
        }
    }

    /// Why that answer is owed, in one line, for the finding report.
    pub fn reason(self) -> &'static str {
        match self {
            Kind::PureHelper => "the imported file only computes",
            Kind::PureChain => "the imported file only computes, through a chain of its own functions",
            Kind::PureNestedImport => "the imported file imports a second file, and both only compute",
            Kind::PureStructHelper => "the imported file declares a struct and an enum and only computes",
            Kind::Diamond => "two imported files share a third, which an import splices once",
            Kind::DangerousHelper => "the file was brought in with import_dangerous, so reaching the world is what was asked for",
            Kind::DeniedCall => "sandboxed code calls something the registry denies it",
            Kind::DeniedThroughOwnChain => "sandboxed code reaches a denied call through a function of its own",
            Kind::DeniedThroughTrustedHelper => "sandboxed code reaches a denied call through a function of the importing program",
            Kind::DeniedThroughHandlerReference => "sandboxed code names a function of the importing program that reaches a denied call",
            Kind::DeniedInConstant => "a sandboxed constant reaches the world when the program starts",
            Kind::TopLevelStatement => "a sandboxed file runs a statement at its top level",
            Kind::SelfImport => "a sandboxed file imports itself",
            Kind::MutualImport => "two sandboxed files import each other",
            Kind::BothCapabilities => "one file is brought in with both import and import_dangerous",
            Kind::DangerousInsideSandbox => "sandboxed code uses import_dangerous",
            Kind::DeniedInNestedImport => "a file imported by an imported file reaches the world",
            Kind::ShadowedDeclaration => "a sandboxed file declares a function the program has already declared",
        }
    }

    /// A phrase the refusal has to contain. Without it a case refused for an
    /// unrelated reason, a type error the generator wrote by accident, would
    /// be counted as the sandbox working when the sandbox was never asked.
    /// Empty for the kinds that must be accepted.
    pub fn marker(self) -> &'static str {
        match self {
            Kind::DeniedCall | Kind::DeniedThroughOwnChain | Kind::DeniedInConstant | Kind::DeniedInNestedImport => "cannot access",
            Kind::DeniedThroughTrustedHelper | Kind::DeniedThroughHandlerReference => "is reachable from sandboxed code",
            Kind::TopLevelStatement => "may only declare functions, structs, enums, and constants",
            Kind::SelfImport | Kind::MutualImport => "Circular import detected",
            Kind::BothCapabilities => "is imported both with import() and import_dangerous()",
            Kind::DangerousInsideSandbox => "import_dangerous is not allowed inside a sandboxed import",
            Kind::ShadowedDeclaration => "is already defined",
            _ => "",
        }
    }
}

/// One file of a case, named the way the case's imports name it.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub name: String,
    pub source: String,
}

/// A whole case: the files, and the answer the compiler owes for them.
#[derive(Debug, Clone)]
pub struct ImportCase {
    pub seed: u64,
    pub kind: Kind,
    pub files: Vec<GeneratedFile>,
}

impl ImportCase {
    /// The file the compiler is pointed at. It is always the first one.
    pub fn main(&self) -> &GeneratedFile {
        &self.files[0]
    }

    pub fn verdict(&self) -> Verdict {
        self.kind.verdict()
    }

    pub fn origin(&self) -> String {
        format!("imports seed {} ({}, must be {})", self.seed, self.kind.name(), self.verdict().name())
    }

    /// Every file as one text, each under a banner naming it, so a finding is
    /// a single readable file even though the case is several.
    pub fn written(&self) -> String {
        let mut out = String::new();
        for (index, file) in self.files.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str(&format!("// ==== {} ====\n", file.name));
            out.push_str(&file.source);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// The library, as the registry describes it
// ---------------------------------------------------------------------------

/// A type a generated call can be handed and can hand back.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Int,
    Float,
    Text,
    Bool,
    Array(Box<Ty>),
}

impl Ty {
    fn written(&self) -> String {
        match self {
            Ty::Int => "i".to_string(),
            Ty::Float => "f".to_string(),
            Ty::Text => "s".to_string(),
            Ty::Bool => "b".to_string(),
            Ty::Array(inner) => format!("a:{}", inner.written()),
        }
    }
}

/// What a call hands back, which decides how it can be written down.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Return {
    /// Nothing at all, so the call can only stand alone as a statement.
    Nothing,
    /// Nothing, or an error, so it stands alone inside danger().
    FallibleNothing,
    /// A value.
    Value(Ty),
    /// A value or an error, so the error has to be dealt with.
    Fallible(Ty),
}

/// One library function a case knows how to call.
#[derive(Debug, Clone)]
struct Call {
    name: &'static str,
    params: Vec<Ty>,
    returns: Return,
}

/// Every library function whose sandbox verdict is the one asked for and
/// whose arguments a case can supply. The registry answers both halves: which
/// functions exist and which of them sandboxed code may call.
fn pool(sandbox_safe: bool) -> &'static [Call] {
    static ALLOWED: OnceLock<Vec<Call>> = OnceLock::new();
    static DENIED: OnceLock<Vec<Call>> = OnceLock::new();
    let build = || {
        let mut calls: Vec<Call> = crate::stdlib_registry::STDLIB_FUNCTIONS
            .iter()
            .filter(|(name, function)| crate::stdlib_registry::is_sandbox_safe(name) == sandbox_safe && !function.diverging)
            .filter_map(|(name, function)| callable(name, function))
            .collect();
        // The registry is a hash map, so its order is not the same twice, and
        // a seed has to mean the same case on every machine.
        calls.sort_by(|left, right| left.name.cmp(right.name));
        calls
    };
    if sandbox_safe {
        ALLOWED.get_or_init(build)
    } else {
        DENIED.get_or_init(build)
    }
}

/// One registry entry as a call a case can write, or nothing when its
/// arguments or its result are shapes a case cannot build a value of.
fn callable(name: &'static str, function: &crate::stdlib_registry::StdlibFunction) -> Option<Call> {
    // A registry name the lexer reads as one of the language's own words is
    // not written as a call at all. `spawn` is the block form `spawn { ... }`
    // and is in the registry only so the transpiler knows what it costs, so a
    // case that wrote `spawn()` would be refused by the parser rather than by
    // the sandbox. Asking the lexer keeps that list out of here.
    if !reads_as_a_name(name) {
        return None;
    }
    // A call that dispatches into the program needs the program to define the
    // function it dispatches to, with a signature the registry fixes. Writing
    // those is a generator of its own, and a case that left one out would be
    // refused for the missing handler rather than for the sandbox.
    if let Some(callbacks) = crate::stdlib_registry::get_handler_callbacks(name) {
        if callbacks.iter().any(|callback| callback.optional_stand_in.is_none()) {
            return None;
        }
    }
    // A type variable used twice in one signature has to be the same type in
    // both places, so what each one stands for is decided once for the whole
    // call rather than once per argument.
    let mut chosen: Vec<(String, Ty)> = Vec::new();
    for parameter in &function.parameters {
        choose_type_variables(&parameter.param_type, &mut chosen);
    }
    let params: Option<Vec<Ty>> = function.parameters.iter().map(|parameter| argument_type(&parameter.param_type, &chosen)).collect();
    let returns = match &function.return_type {
        NailDataTypeDescriptor::Void => Return::Nothing,
        NailDataTypeDescriptor::Result(inner) if **inner == NailDataTypeDescriptor::Void => Return::FallibleNothing,
        NailDataTypeDescriptor::Result(inner) => Return::Fallible(plain_type(inner)?),
        other => Return::Value(plain_type(other)?),
    };
    Some(Call { name, params: params?, returns })
}

/// Whether the lexer reads this name as an ordinary name, which is what a
/// call has to be written with.
fn reads_as_a_name(name: &str) -> bool {
    let tokens = crate::lexer::lexer_without_imports(name);
    matches!(tokens.first().map(|token| &token.token_type), Some(crate::lexer::TokenType::Identifier(_)))
}

/// Pick a concrete type for every type variable in a parameter, once each.
/// A bounded variable takes its first bound, an unbounded one takes text,
/// which every function that accepts anything accepts.
fn choose_type_variables(descriptor: &NailDataTypeDescriptor, chosen: &mut Vec<(String, Ty)>) {
    match descriptor {
        NailDataTypeDescriptor::TypeVar(name, bounds) => {
            if chosen.iter().any(|(existing, _)| existing == name) {
                return;
            }
            let ty = bounds.iter().find_map(plain_type).unwrap_or(Ty::Text);
            chosen.push((name.clone(), ty));
        }
        NailDataTypeDescriptor::Array(inner) => choose_type_variables(inner, chosen),
        _ => {}
    }
}

/// The type a case writes an argument as, with type variables resolved.
fn argument_type(descriptor: &NailDataTypeDescriptor, chosen: &[(String, Ty)]) -> Option<Ty> {
    match descriptor {
        NailDataTypeDescriptor::TypeVar(name, _) => chosen.iter().find(|(existing, _)| existing == name).map(|(_, ty)| ty.clone()),
        NailDataTypeDescriptor::Array(inner) => argument_type(inner, chosen).map(|inner| Ty::Array(Box::new(inner))),
        other => plain_type(other),
    }
}

/// A library type as a case thinks of it, or nothing for the shapes a case
/// cannot build: hashmaps, structs, functions and bare type variables.
fn plain_type(descriptor: &NailDataTypeDescriptor) -> Option<Ty> {
    match descriptor {
        NailDataTypeDescriptor::Int => Some(Ty::Int),
        NailDataTypeDescriptor::Float => Some(Ty::Float),
        NailDataTypeDescriptor::String => Some(Ty::Text),
        NailDataTypeDescriptor::Boolean => Some(Ty::Bool),
        NailDataTypeDescriptor::Array(inner) => plain_type(inner).map(|inner| Ty::Array(Box::new(inner))),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Writing a case
// ---------------------------------------------------------------------------

/// The case a seed writes. The same seed writes the same files forever, which
/// is what makes a finding a single number.
pub fn case(seed: u64) -> ImportCase {
    let kind = Kind::ALL[(seed % Kind::ALL.len() as u64) as usize];
    let mut writer = Writer::new(seed);
    let files = writer.files(kind);
    ImportCase { seed, kind, files }
}

struct Writer {
    seed: u64,
    rng: StdRng,
    /// Every name a case has used, so nothing is declared twice and nothing
    /// shadows anything, which Nail refuses on purpose.
    counter: usize,
}

impl Writer {
    fn new(seed: u64) -> Writer {
        Writer { seed, rng: StdRng::seed_from_u64(seed ^ 0x5720_1d0e), counter: 0 }
    }

    // -- names ------------------------------------------------------------

    fn fresh(&mut self, stem: &str) -> String {
        self.counter += 1;
        format!("{}_{}", stem, self.counter)
    }

    fn fresh_type_name(&mut self, stem: &str) -> String {
        self.counter += 1;
        format!("{}{}", stem, self.counter)
    }

    /// File names carry the seed, so the files of one case never collide with
    /// the files of another in the scratch directory they share.
    fn main_name(&self) -> String {
        format!("fuzz_main_{}.nail", self.seed)
    }

    fn helper_name(&self) -> String {
        format!("fuzz_helper_{}.nail", self.seed)
    }

    fn deep_name(&self) -> String {
        format!("fuzz_deep_{}.nail", self.seed)
    }

    fn second_helper_name(&self) -> String {
        format!("fuzz_second_{}.nail", self.seed)
    }

    // -- pieces -----------------------------------------------------------

    fn literal(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Int => format!("{}", self.rng.gen_range(1..40)),
            Ty::Float => format!("{}.{}", self.rng.gen_range(1..40), self.rng.gen_range(1..9)),
            Ty::Text => {
                let words = ["alpha", "beta gamma", "42", "a,b,c", "one two three"];
                format!("`{}`", words[self.rng.gen_range(0..words.len())])
            }
            Ty::Bool => format!("{}", self.rng.gen_bool(0.5)),
            Ty::Array(inner) => {
                let first = self.literal(inner);
                let second = self.literal(inner);
                format!("[{}, {}]", first, second)
            }
        }
    }

    /// One library call, with arguments of the types it asked for.
    fn written_call(&mut self, call: &Call) -> String {
        let mut arguments = Vec::new();
        for param in call.params.clone() {
            arguments.push(self.literal(&param));
        }
        format!("{}({})", call.name, arguments.join(", "))
    }

    /// A library call as a statement inside a function body, written so the
    /// program type checks whatever the call hands back.
    fn statement_calling(&mut self, call: &Call) -> String {
        let written = self.written_call(call);
        match &call.returns {
            Return::Nothing => format!("{};", written),
            Return::FallibleNothing => format!("danger({});", written),
            Return::Value(ty) => {
                let name = self.fresh("taken");
                format!("{}:{} = {};", name, ty.written(), written)
            }
            Return::Fallible(ty) => {
                let name = self.fresh("taken");
                format!("{}:{} = danger({});", name, ty.written(), written)
            }
        }
    }

    /// A library call as the value of a constant, for the kinds that need the
    /// call to run before the program does. Nothing when the call hands back
    /// nothing at all, since a constant has to hold something.
    fn constant_calling(&mut self, call: &Call) -> Option<String> {
        let written = self.written_call(call);
        let name = self.fresh("held");
        match &call.returns {
            Return::Value(ty) => Some(format!("{}:{} = {};", name, ty.written(), written)),
            Return::Fallible(ty) => Some(format!("{}:{} = danger({});", name, ty.written(), written)),
            _ => None,
        }
    }

    /// A call from one of the two pools, chosen by the seed.
    fn pick(&mut self, sandbox_safe: bool) -> Option<Call> {
        let calls = pool(sandbox_safe);
        if calls.is_empty() {
            return None;
        }
        Some(calls[self.rng.gen_range(0..calls.len())].clone())
    }

    /// A call from a pool that can stand as the value of a constant.
    fn pick_returning_value(&mut self, sandbox_safe: bool) -> Option<Call> {
        let calls: Vec<&Call> = pool(sandbox_safe).iter().filter(|call| matches!(call.returns, Return::Value(_) | Return::Fallible(_))).collect();
        if calls.is_empty() {
            return None;
        }
        Some(calls[self.rng.gen_range(0..calls.len())].clone())
    }

    /// A call from a pool that can stand alone as a statement, for the kinds
    /// that need a statement rather than a declaration.
    fn pick_standalone(&mut self, sandbox_safe: bool) -> Option<Call> {
        let calls: Vec<&Call> = pool(sandbox_safe).iter().filter(|call| matches!(call.returns, Return::Nothing | Return::FallibleNothing)).collect();
        if calls.is_empty() {
            return None;
        }
        Some(calls[self.rng.gen_range(0..calls.len())].clone())
    }

    /// A function that only computes: a few library calls that are allowed
    /// inside a sandbox, then a value. The name is returned so the importing
    /// program can call it.
    fn pure_function(&mut self) -> (String, String) {
        let name = self.fresh("compute");
        let mut body = Vec::new();
        for _ in 0..self.rng.gen_range(1..4) {
            if let Some(call) = self.pick(true) {
                let statement = self.statement_calling(&call);
                // Buried in a block as often as not, because a sandbox that
                // refused an honest call for sitting inside a loop would be
                // just as broken as one that let a dishonest one through.
                body.extend(self.buried(statement, false));
            }
        }
        let answer = self.literal(&Ty::Int);
        body.push(format!("    r {};", answer));
        (name.clone(), format!("f {}():i {{\n{}\n}}", name, body.join("\n")))
    }

    /// A function that reaches the world, whichever way the registry says a
    /// function can. Used both where that must be refused and, under
    /// import_dangerous, where it must be allowed.
    fn reaching_function(&mut self) -> (String, String) {
        let name = self.fresh("reach");
        let mut body = Vec::new();
        if let Some(call) = self.pick(false) {
            let statement = self.statement_calling(&call);
            body.extend(self.buried(statement, true));
        }
        let answer = self.literal(&Ty::Int);
        body.push(format!("    r {};", answer));
        (name.clone(), format!("f {}():i {{\n{}\n}}", name, body.join("\n")))
    }

    /// The same statement, sometimes buried inside a block. A call the
    /// sandbox has to see is no less a call for being inside a loop, and the
    /// walk that finds it has to go all the way down. Nothing is added to the
    /// block but the statement itself, so a case that is refused is refused
    /// for the call it was written around.
    ///
    /// `may_spawn` says whether a background block is one of the wrappings.
    /// Sandboxed code may not spawn at all, since a spawned block keeps a
    /// piece of the program after the answer is handed back, so only a case
    /// that is meant to be refused may use one.
    fn buried(&mut self, statement: String, may_spawn: bool) -> Vec<String> {
        match self.rng.gen_range(0..3) {
            0 => {
                let item = self.fresh("item");
                vec![format!("    for {} in [1, 2] {{", item), format!("        {}", statement), "    }".to_string()]
            }
            1 if may_spawn => vec!["    spawn {".to_string(), format!("        {}", statement), "    }".to_string()],
            _ => vec![format!("    {}", statement)],
        }
    }

    fn file(&self, name: String, lines: Vec<String>) -> GeneratedFile {
        let mut source = String::from(VERSION_LINE);
        source.push('\n');
        source.push_str(&format!("// written by the import fuzzer, seed {}\n", self.seed));
        for line in lines {
            source.push_str(&line);
            source.push('\n');
        }
        GeneratedFile { name, source }
    }

    // -- the kinds --------------------------------------------------------

    fn files(&mut self, kind: Kind) -> Vec<GeneratedFile> {
        match kind {
            Kind::PureHelper => self.pure_helper(),
            Kind::PureChain => self.pure_chain(),
            Kind::PureNestedImport => self.pure_nested_import(),
            Kind::PureStructHelper => self.pure_struct_helper(),
            Kind::Diamond => self.diamond(),
            Kind::DangerousHelper => self.dangerous_helper(),
            Kind::DeniedCall => self.denied_call(),
            Kind::DeniedThroughOwnChain => self.denied_through_own_chain(),
            Kind::DeniedThroughTrustedHelper => self.denied_through_trusted_helper(),
            Kind::DeniedThroughHandlerReference => self.denied_through_handler_reference(),
            Kind::DeniedInConstant => self.denied_in_constant(),
            Kind::TopLevelStatement => self.top_level_statement(),
            Kind::SelfImport => self.self_import(),
            Kind::MutualImport => self.mutual_import(),
            Kind::BothCapabilities => self.both_capabilities(),
            Kind::DangerousInsideSandbox => self.dangerous_inside_sandbox(),
            Kind::DeniedInNestedImport => self.denied_in_nested_import(),
            Kind::ShadowedDeclaration => self.shadowed_declaration(),
        }
    }

    fn pure_helper(&mut self) -> Vec<GeneratedFile> {
        let mut helper = Vec::new();
        let constant = self.fresh("prefix");
        let value = self.literal(&Ty::Text);
        helper.push(format!("{}:s = {};", constant, value));
        let mut names = Vec::new();
        for _ in 0..self.rng.gen_range(1..4) {
            let (name, text) = self.pure_function();
            helper.push(text);
            names.push(name);
        }

        let mut main = vec![format!("import(`{}`)", self.helper_name()), String::new()];
        for name in names {
            main.push(format!("print({}());", name));
        }
        main.push(format!("print({});", constant));

        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn pure_chain(&mut self) -> Vec<GeneratedFile> {
        let (inner, inner_text) = self.pure_function();
        let middle = self.fresh("middle");
        let outer = self.fresh("outer");
        let helper = vec![inner_text, format!("f {}():i {{\n    r {}();\n}}", middle, inner), format!("f {}():i {{\n    r {}();\n}}", outer, middle)];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", outer)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn pure_nested_import(&mut self) -> Vec<GeneratedFile> {
        let (deep_name, deep_text) = self.pure_function();
        let deep = vec![deep_text];
        let bridge = self.fresh("bridge");
        let helper = vec![format!("import(`{}`)", self.deep_name()), String::new(), format!("f {}():i {{\n    r {}();\n}}", bridge, deep_name)];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", bridge)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper), self.file(self.deep_name(), deep)]
    }

    fn pure_struct_helper(&mut self) -> Vec<GeneratedFile> {
        let shape = self.fresh_type_name("Shape");
        let number_field = self.fresh("field");
        let text_field = self.fresh("field");
        let mode = self.fresh_type_name("Mode");
        let first_choice = self.fresh_type_name("Choice");
        let second_choice = self.fresh_type_name("Choice");
        let reader = self.fresh("read");
        let parameter = self.fresh("shape");
        let helper = vec![
            format!("struct {} {{\n    {}:i,\n    {}:s\n}}", shape, number_field, text_field),
            format!("enum {} {{\n    {},\n    {}\n}}", mode, first_choice, second_choice),
            format!("f {}({}:{}):i {{\n    r {}.{};\n}}", reader, parameter, shape, parameter, number_field),
        ];

        let made = self.fresh("made");
        let number = self.literal(&Ty::Int);
        let text = self.literal(&Ty::Text);
        let chosen = self.fresh("chosen");
        let main = vec![
            format!("import(`{}`)", self.helper_name()),
            String::new(),
            format!("{}:{} = {} {{ {} = {}, {} = {} }};", made, shape, shape, number_field, number, text_field, text),
            format!("{}:{} = {}::{};", chosen, mode, mode, first_choice),
            format!("print({}({}));", reader, made),
            format!("print({});", chosen),
        ];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn diamond(&mut self) -> Vec<GeneratedFile> {
        let (shared_name, shared_text) = self.pure_function();
        let shared = vec![shared_text];
        let left = self.fresh("left");
        let right = self.fresh("right");
        let first = vec![format!("import(`{}`)", self.deep_name()), String::new(), format!("f {}():i {{\n    r {}();\n}}", left, shared_name)];
        let second = vec![format!("import(`{}`)", self.deep_name()), String::new(), format!("f {}():i {{\n    r {}();\n}}", right, shared_name)];
        let main = vec![
            format!("import(`{}`)", self.helper_name()),
            format!("import(`{}`)", self.second_helper_name()),
            String::new(),
            format!("print({}());", left),
            format!("print({}());", right),
        ];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), first), self.file(self.second_helper_name(), second), self.file(self.deep_name(), shared)]
    }

    fn dangerous_helper(&mut self) -> Vec<GeneratedFile> {
        let (name, text) = self.reaching_function();
        let helper = vec![text];
        let main = vec![format!("import_dangerous(`{}`)", self.helper_name()), String::new(), format!("print({}());", name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn denied_call(&mut self) -> Vec<GeneratedFile> {
        let (name, text) = self.reaching_function();
        let helper = vec![text];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn denied_through_own_chain(&mut self) -> Vec<GeneratedFile> {
        let (inner, inner_text) = self.reaching_function();
        let outer = self.fresh("outer");
        let helper = vec![inner_text, format!("f {}():i {{\n    r {}();\n}}", outer, inner)];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", outer)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn denied_through_trusted_helper(&mut self) -> Vec<GeneratedFile> {
        let (trusted, trusted_text) = self.reaching_function();
        let launder = self.fresh("launder");
        let helper = vec![format!("f {}():i {{\n    r {}();\n}}", launder, trusted)];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), trusted_text, format!("print({}());", launder)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn denied_through_handler_reference(&mut self) -> Vec<GeneratedFile> {
        let might_fail = self.fresh("might_fail");
        let user = self.fresh("use_handler");
        let handler = self.fresh("handle");
        let failure = self.fresh("failure");
        let flag = self.rng.gen_bool(0.5);
        // The sandboxed file never calls the handler, it only names it, which
        // reaches it just as surely.
        let helper = vec![
            format!("f {}(flag:b):s!e {{\n    if {{\n        flag -> {{ r e(`no answer`); }},\n        else -> {{ r `an answer`; }}\n    }}\n}}", might_fail),
            format!("f {}():s {{\n    r safe({}({}), {});\n}}", user, might_fail, flag, handler),
        ];

        let mut handler_body = Vec::new();
        if let Some(call) = self.pick(false) {
            let statement = self.statement_calling(&call);
            handler_body.push(format!("    {}", statement));
        }
        handler_body.push("    r `fallback`;".to_string());
        let main = vec![
            format!("import(`{}`)", self.helper_name()),
            String::new(),
            format!("f {}({}:e):s {{\n{}\n}}", handler, failure, handler_body.join("\n")),
            format!("print({}());", user),
        ];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn denied_in_constant(&mut self) -> Vec<GeneratedFile> {
        let call = self.pick_returning_value(false);
        let mut helper = Vec::new();
        let mut main = vec![format!("import(`{}`)", self.helper_name()), String::new()];
        let declaration = call.and_then(|call| self.constant_calling(&call));
        match declaration {
            Some(declaration) => {
                // The name is what stands before the colon, which is how the
                // importing program then names it.
                let held = declaration.split(':').next().unwrap_or("held").to_string();
                helper.push(declaration);
                main.push(format!("print({});", held));
            }
            // Nothing the registry denies can hold a value, which cannot
            // happen while a denied call hands one back. A function that
            // reaches the world is refused for the same reason, so the case
            // still tests what it says it tests.
            None => {
                let (name, text) = self.reaching_function();
                helper.push(text);
                main.push(format!("print({}());", name));
            }
        }
        let (name, text) = self.pure_function();
        helper.push(text);
        main.push(format!("print({}());", name));
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn top_level_statement(&mut self) -> Vec<GeneratedFile> {
        let (name, text) = self.pure_function();
        // A call that is allowed inside a sandbox, so the only thing wrong
        // with the file is where the call sits.
        let statement = match self.pick_standalone(true) {
            Some(call) => self.statement_calling(&call),
            None => "print(`this runs at the top level`);".to_string(),
        };
        let helper = vec![text, statement];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn self_import(&mut self) -> Vec<GeneratedFile> {
        let (name, text) = self.pure_function();
        let helper = vec![format!("import(`{}`)", self.helper_name()), String::new(), text];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn mutual_import(&mut self) -> Vec<GeneratedFile> {
        let (name, text) = self.pure_function();
        let (deep_name, deep_text) = self.pure_function();
        let helper = vec![format!("import(`{}`)", self.deep_name()), String::new(), text];
        let deep = vec![format!("import(`{}`)", self.helper_name()), String::new(), deep_text];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", name), format!("print({}());", deep_name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper), self.file(self.deep_name(), deep)]
    }

    fn both_capabilities(&mut self) -> Vec<GeneratedFile> {
        let (name, text) = self.pure_function();
        let helper = vec![text];
        // Either order is the same mistake, and both are worth writing: the
        // second import is the one that finds the file already spliced.
        let (first, second) = if self.rng.gen_bool(0.5) {
            (format!("import(`{}`)", self.helper_name()), format!("import_dangerous(`{}`)", self.helper_name()))
        } else {
            (format!("import_dangerous(`{}`)", self.helper_name()), format!("import(`{}`)", self.helper_name()))
        };
        let main = vec![first, second, String::new(), format!("print({}());", name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn dangerous_inside_sandbox(&mut self) -> Vec<GeneratedFile> {
        let (deep_name, deep_text) = self.pure_function();
        let deep = vec![deep_text];
        let bridge = self.fresh("bridge");
        let helper = vec![format!("import_dangerous(`{}`)", self.deep_name()), String::new(), format!("f {}():i {{\n    r {}();\n}}", bridge, deep_name)];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", bridge)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper), self.file(self.deep_name(), deep)]
    }

    /// The importing program and the sandboxed file both declare a function
    /// of the same name, one of them reaching the world. Which one a later
    /// stage would keep is the question, and a program that declares a name
    /// twice has to be refused before that question can be asked.
    fn shadowed_declaration(&mut self) -> Vec<GeneratedFile> {
        let (name, reaching_text) = self.reaching_function();
        let answer = self.literal(&Ty::Int);
        let harmless = format!("f {}():i {{\n    r {};\n}}", name, answer);
        let import_line = format!("import(`{}`)", self.helper_name());
        // Either file can be the one written first, and the order decides
        // which declaration a first-one-wins rule would keep.
        let (helper, main_declaration) = if self.rng.gen_bool(0.5) { (vec![reaching_text], harmless) } else { (vec![harmless], reaching_text) };
        let main = vec![import_line, String::new(), main_declaration, format!("print({}());", name)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper)]
    }

    fn denied_in_nested_import(&mut self) -> Vec<GeneratedFile> {
        let (deep_name, deep_text) = self.reaching_function();
        let deep = vec![deep_text];
        let bridge = self.fresh("bridge");
        let helper = vec![format!("import(`{}`)", self.deep_name()), String::new(), format!("f {}():i {{\n    r {}();\n}}", bridge, deep_name)];
        let main = vec![format!("import(`{}`)", self.helper_name()), String::new(), format!("print({}());", bridge)];
        vec![self.file(self.main_name(), main), self.file(self.helper_name(), helper), self.file(self.deep_name(), deep)]
    }
}

// ---------------------------------------------------------------------------
// Asking the compiler
// ---------------------------------------------------------------------------

/// What the compiler said about a case.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Lexed, parsed and type checked.
    Accepted,
    /// Refused, with everything it said and the stage that said it first.
    Refused { stage: &'static str, messages: Vec<String> },
    /// The case never reached the compiler, because its files could not be
    /// written. Not a finding: that is this machine's problem, not Nail's.
    NotRun(String),
}

impl Answer {
    pub fn describe(&self) -> String {
        match self {
            Answer::Accepted => "accepted".to_string(),
            Answer::Refused { stage, messages } => format!("refused at {}: {}", stage, messages.first().cloned().unwrap_or_default()),
            Answer::NotRun(reason) => format!("not run: {}", reason),
        }
    }
}

/// Write a case's files into a directory and hand back the path of the file
/// the compiler is pointed at. Imports resolve against the file that holds
/// them, so a multi-file case has to exist on disk to mean anything.
pub fn write_files(case: &ImportCase, scratch: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(scratch)?;
    for file in &case.files {
        fs::write(scratch.join(&file.name), &file.source)?;
    }
    Ok(scratch.join(&case.main().name))
}

/// Take a case's files away again, so a run of thousands leaves the scratch
/// directory as it found it.
pub fn remove_files(case: &ImportCase, scratch: &Path) {
    for file in &case.files {
        let _ = fs::remove_file(scratch.join(&file.name));
    }
}

/// Write one case out, compile it, and say whether the compiler's answer was
/// the one the case was built to expect.
///
/// The serious finding is a case built to be refused that was accepted: that
/// is sandboxed code reaching the world with the compiler's blessing. A case
/// built to be accepted that was refused is a finding too, of a different
/// class, because a sandbox nobody can pass is a feature nobody can use.
pub fn examine(case: &ImportCase, scratch: &Path) -> (Option<Finding>, Answer) {
    // On the compiler's own stack, the same one every other entry into the
    // compiler uses, so a case is judged by the depth limits rather than by
    // which thread the fuzzer happened to call from.
    crate::common::with_compiler_stack(|| examine_here(case, scratch))
}

fn examine_here(case: &ImportCase, scratch: &Path) -> (Option<Finding>, Answer) {
    let path = match write_files(case, scratch) {
        Ok(path) => path,
        Err(error) => return (None, Answer::NotRun(error.to_string())),
    };
    let (answer, finding) = compile(&case.main().source, &path);
    if let Some(finding) = finding {
        return (Some(finding), answer);
    }
    (judge(case, &answer), answer)
}

/// Run the compiler over the main file, catching a panic at every stage. A
/// panic is a finding whatever the case was built to expect: no input, however
/// broken, may crash the compiler.
fn compile(source: &str, path: &Path) -> (Answer, Option<Finding>) {
    let lexed = match guard("lex", || lex_program(source, Some(path))) {
        Ok(lexed) => lexed,
        Err(finding) => return (Answer::NotRun("the lexer panicked".to_string()), Some(finding)),
    };
    let lexer_errors = match guard("lex", || collect_lexer_errors(&lexed.tokens)) {
        Ok(errors) => errors,
        Err(finding) => return (Answer::NotRun("the lexer panicked".to_string()), Some(finding)),
    };
    if !lexer_errors.is_empty() {
        return (Answer::Refused { stage: "lex", messages: lexer_errors.iter().map(|error| error.message.clone()).collect() }, None);
    }

    let parsed = match guard("parse", || parse(lexed.tokens)) {
        Ok(parsed) => parsed,
        Err(finding) => return (Answer::NotRun("the parser panicked".to_string()), Some(finding)),
    };
    let mut ast = match parsed {
        Ok(ast) => ast,
        Err(error) => return (Answer::Refused { stage: "parse", messages: vec![error.message] }, None),
    };

    let checked = match guard("check", || {
        let result = checker(&mut ast);
        (ast, result)
    }) {
        Ok((walked, result)) => {
            ast = walked;
            result
        }
        Err(finding) => return (Answer::NotRun("the checker panicked".to_string()), Some(finding)),
    };
    if let Err(errors) = checked {
        return (Answer::Refused { stage: "check", messages: errors.iter().map(|error| error.message.clone()).collect() }, None);
    }

    // Past here the compiler said yes, so a later failure is the compiler
    // contradicting itself rather than the case being wrong.
    let transpiled = match guard("transpile", || {
        let mut transpiler = Transpiler::new();
        // Profiling writes a file beside the program and reads the clock,
        // neither of which says anything about the sandbox.
        transpiler.profile = false;
        transpiler.transpile(&ast)
    }) {
        Ok(result) => result,
        Err(finding) => return (Answer::Accepted, Some(finding)),
    };
    if let Err(error) = transpiled {
        return (
            Answer::Accepted,
            Some(Finding {
                property: Property::CheckImpliesTranspile,
                stage: "transpile",
                detail: error.message,
                site: None,
                class: "the transpiler refused a program the checker accepted",
            }),
        );
    }
    (Answer::Accepted, None)
}

/// Compare the compiler's answer with the one the case was built to expect.
fn judge(case: &ImportCase, answer: &Answer) -> Option<Finding> {
    match (case.verdict(), answer) {
        (_, Answer::NotRun(_)) => None,

        // The serious one. Somebody else's code was allowed to touch this
        // machine, and the compiler said that was fine.
        (Verdict::Refused, Answer::Accepted) => Some(Finding {
            property: Property::SandboxHolds,
            stage: "check",
            detail: format!("the compiler accepted a case it had to refuse, where {}", case.kind.reason()),
            site: None,
            class: "sandboxed code reached the world and was accepted",
        }),

        // Refused, but for something other than the sandbox. The case did not
        // test what it was written to test, which hides whatever it would
        // have found.
        (Verdict::Refused, Answer::Refused { stage, messages }) => {
            if messages.iter().any(|message| message.contains(case.kind.marker())) {
                return None;
            }
            Some(Finding {
                property: Property::SandboxHolds,
                stage,
                detail: format!("refused, but for something else: expected a refusal saying '{}', and it said '{}'", case.kind.marker(), messages.first().cloned().unwrap_or_default()),
                site: None,
                class: "refused, but not by the sandbox",
            })
        }

        // The other half of the promise. Code that only computes has to get
        // through, or nobody can import anything.
        (Verdict::Accepted, Answer::Refused { stage, messages }) => {
            let first = messages.first().cloned().unwrap_or_default();
            let blames_the_sandbox = messages.iter().any(|message| message.contains("sandbox") || message.contains("import"));
            Some(Finding {
                property: Property::SandboxHolds,
                stage,
                detail: format!("the compiler refused a case it had to accept, where {}: {}", case.kind.reason(), first),
                site: None,
                class: if blames_the_sandbox { "the sandbox refused code that only computes" } else { "a case built to be accepted was refused" },
            })
        }

        (Verdict::Accepted, Answer::Accepted) => None,
    }
}

/// Run one stage, turning a panic into a finding. The fuzzer's own panic hook
/// keeps the backtrace banner off the screen, so this only has to name the
/// stage and what the panic said.
fn guard<T>(stage: &'static str, work: impl FnOnce() -> T) -> Result<T, Finding> {
    match panic::catch_unwind(AssertUnwindSafe(work)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            let message = if let Some(text) = payload.downcast_ref::<&str>() {
                (*text).to_string()
            } else if let Some(text) = payload.downcast_ref::<String>() {
                text.clone()
            } else {
                "panicked with a value that is not text".to_string()
            };
            Err(Finding { property: Property::NoPanic, stage, detail: message, site: None, class: "panicked" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of this test's own, since two test runs at once would
    /// otherwise write each other's files.
    fn scratch() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("fuzz").join(format!("imports_tests_{}", std::process::id()))
    }

    #[test]
    fn the_registry_supplies_both_pools() {
        assert!(pool(true).len() > 100, "only {} library calls are allowed in a sandbox, which cannot be right", pool(true).len());
        assert!(pool(false).len() > 20, "only {} library calls are denied to a sandbox, which cannot be right", pool(false).len());
        // Nothing may be in both, or a case would be built to expect two
        // different answers for the same call.
        for allowed in pool(true) {
            assert!(!pool(false).iter().any(|denied| denied.name == allowed.name), "'{}' is in both pools", allowed.name);
        }
    }

    #[test]
    fn a_seed_always_writes_the_same_case() {
        for seed in [1, 2, 17, 4321] {
            let first = case(seed);
            let second = case(seed);
            assert_eq!(first.kind, second.kind);
            assert_eq!(first.written(), second.written(), "seed {} wrote two different cases", seed);
        }
    }

    #[test]
    fn a_run_of_seeds_covers_every_kind() {
        let mut seen: Vec<Kind> = Vec::new();
        for seed in 1..=(Kind::ALL.len() as u64 * 2) {
            let kind = case(seed).kind;
            if !seen.contains(&kind) {
                seen.push(kind);
            }
        }
        assert_eq!(seen.len(), Kind::ALL.len(), "only {} of {} kinds were written", seen.len(), Kind::ALL.len());
    }

    /// The whole point. Every case knows the answer it is owed, and the
    /// compiler has to give that answer.
    #[test]
    fn the_sandbox_answers_every_case_the_way_it_must() {
        let scratch = scratch();
        let mut findings = Vec::new();
        for seed in 1..=(Kind::ALL.len() as u64 * 3) {
            let built = case(seed);
            let (finding, answer) = examine(&built, &scratch);
            remove_files(&built, &scratch);
            if let Some(finding) = finding {
                findings.push(format!("seed {} ({}): {} [{}], the compiler {}", seed, built.kind.name(), finding.detail, finding.class, answer.describe()));
            }
        }
        let _ = fs::remove_dir_all(&scratch);
        assert!(findings.is_empty(), "the import fuzzer found bugs: {:#?}", findings);
    }

    /// The oracle has to be able to see a hole, not only to pass. This hands
    /// it a case whose files break the sandbox while claiming to be pure, and
    /// checks that it says so.
    #[test]
    fn the_oracle_can_see_a_case_answered_the_wrong_way() {
        let scratch = scratch().join("wrong_way");
        let denied = pool(false).first().expect("the registry denies something to sandboxed code").clone();
        let mut writer = Writer::new(7);
        let statement = writer.statement_calling(&denied);
        let helper = GeneratedFile { name: "fuzz_helper_wrong.nail".to_string(), source: format!("{}\nf reach_one():i {{\n    {}\n    r 1;\n}}\n", VERSION_LINE, statement) };
        let main = GeneratedFile {
            name: "fuzz_main_wrong.nail".to_string(),
            source: format!("{}\nimport(`fuzz_helper_wrong.nail`)\n\nprint(reach_one());\n", VERSION_LINE),
        };
        // Claimed pure, and it is not, so the oracle owes a finding.
        let lying = ImportCase { seed: 7, kind: Kind::PureHelper, files: vec![main, helper] };
        let (finding, _) = examine(&lying, &scratch);
        let _ = fs::remove_dir_all(&scratch);
        let finding = finding.expect("a helper that reaches the world is not a pure helper");
        assert_eq!(finding.property, Property::SandboxHolds);
        assert_eq!(finding.class, "the sandbox refused code that only computes");
    }
}
