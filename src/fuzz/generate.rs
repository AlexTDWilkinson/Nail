//! Writing Nail programs from nothing.
//!
//! The mutation engine finds what breaks when a program is malformed. This
//! one finds what breaks when a program is fine: it keeps a type environment
//! as it writes, so nearly everything it produces type checks, which is the
//! only way to reach the transpiler and rustc in volume. A generator that
//! wrote text at random would spend its whole life being refused by the
//! lexer.
//!
//! Everything it knows how to write is in the language primer, and nothing
//! else: this file is a second, executable statement of the grammar, and a
//! construct it cannot produce is a construct the fuzzer does not cover.

use std::sync::OnceLock;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::common::NailDataTypeDescriptor;

/// A type as the generator thinks of it. Results, hashmaps and void are not
/// here: a result has to be handled where it appears, and the handling forms
/// are worth generating on their own rather than as an afterthought inside
/// expression generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    Text,
    Bool,
    Array(Box<Ty>),
    Struct(String),
    Enum(String),
}

impl Ty {
    /// The type as Nail writes it.
    fn written(&self) -> String {
        match self {
            Ty::Int => "i".to_string(),
            Ty::Float => "f".to_string(),
            Ty::Text => "s".to_string(),
            Ty::Bool => "b".to_string(),
            Ty::Array(inner) => format!("a:{}", inner.written()),
            Ty::Struct(name) | Ty::Enum(name) => name.clone(),
        }
    }
}

/// What a standard library call hands back, which decides how a generated
/// program is allowed to write the call.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Returns {
    /// A plain value, usable as an expression of that type.
    Value(Ty),
    /// A value or an error, so the call has to be handled: the generator
    /// wraps it in safe(...) with a handler that supplies a default.
    Fallible(Ty),
    /// Nothing, so the call can only stand alone as a statement.
    Nothing,
}

/// One standard library function the generator knows how to call.
#[derive(Debug, Clone)]
struct StdlibCall {
    name: &'static str,
    params: Vec<Ty>,
    returns: Returns,
}

/// The standard library functions a generated program may call.
///
/// The registry is the source of the list, filtered by the registry's own
/// policy question: a sandbox safe function is one that can only compute,
/// which is exactly what a fuzzed program is allowed to do. Nothing here
/// names a function, so a function added to the library tomorrow is fuzzed
/// tomorrow.
///
/// Only calls whose parameters and result are plain values are kept, because
/// those are the ones the generator can supply arguments for. Hashmaps,
/// structs and type variables are left to the parts of the generator that
/// know how to build them.
fn stdlib_calls() -> &'static [StdlibCall] {
    static CALLS: OnceLock<Vec<StdlibCall>> = OnceLock::new();
    CALLS.get_or_init(|| {
        let mut calls: Vec<StdlibCall> = crate::stdlib_registry::STDLIB_FUNCTIONS
            .iter()
            .filter(|(name, function)| crate::stdlib_registry::is_sandbox_safe(name) && !function.diverging)
            .filter_map(|(name, function)| {
                let params: Option<Vec<Ty>> = function.parameters.iter().map(|parameter| plain_type(&parameter.param_type)).collect();
                let returns = match &function.return_type {
                    NailDataTypeDescriptor::Void => Returns::Nothing,
                    NailDataTypeDescriptor::Result(inner) => Returns::Fallible(plain_type(inner)?),
                    other => Returns::Value(plain_type(other)?),
                };
                Some(StdlibCall { name, params: params?, returns })
            })
            .collect();
        // The registry is a hash map, so its order is not the same twice.
        // Sorting is what makes a seed mean the same program on every run.
        calls.sort_by(|left, right| left.name.cmp(right.name));
        calls
    })
}

/// The generator's version of a library type, or nothing when the type is one
/// it cannot build a value of.
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

struct StructDef {
    name: String,
    fields: Vec<(String, Ty)>,
}

struct EnumDef {
    name: String,
    variants: Vec<String>,
}

struct FunctionDef {
    name: String,
    params: Vec<(String, Ty)>,
    returns: Ty,
    /// Whether the function is declared as `T!e`, so that every call to it
    /// has to deal with the error rather than ignore it.
    fallible: bool,
}

#[derive(Clone)]
struct Binding {
    name: String,
    ty: Ty,
}

pub fn case(seed: u64) -> (String, String) {
    let mut generator = Generator::new(seed);
    (generator.program(), format!("generate seed {}", seed))
}

struct Generator {
    rng: StdRng,
    /// Every name the program has ever used, so nothing is ever declared
    /// twice and nothing ever shadows anything. Nail refuses a name that
    /// shadows one from an enclosing scope, and a generator that tripped on
    /// that rule would spend its whole run reporting it.
    counter: usize,
    structs: Vec<StructDef>,
    enums: Vec<EnumDef>,
    functions: Vec<FunctionDef>,
    /// The bindings in scope, innermost last.
    scopes: Vec<Vec<Binding>>,
    lines: Vec<String>,
    indent: usize,
    /// Error handlers written so far, one per type, so a fallible call can be
    /// handled without writing a new handler every time.
    fallbacks: Vec<(Ty, String)>,
    /// Declarations to append once the body is written. Function declarations
    /// are hoisted, so a handler can be written after everything that uses it.
    trailing: Vec<String>,
}

impl Generator {
    fn new(seed: u64) -> Generator {
        Generator {
            rng: StdRng::seed_from_u64(seed),
            counter: 0,
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            scopes: vec![Vec::new()],
            lines: Vec::new(),
            indent: 0,
            fallbacks: Vec::new(),
            trailing: Vec::new(),
        }
    }

    fn program(&mut self) -> String {
        self.emit("nail latest");

        for _ in 0..self.rng.gen_range(0..3) {
            self.declare_struct();
        }
        for _ in 0..self.rng.gen_range(0..2) {
            self.declare_enum();
        }
        for _ in 0..self.rng.gen_range(0..4) {
            self.declare_function();
        }

        let statements = self.rng.gen_range(3..12);
        for _ in 0..statements {
            self.statement(0);
        }

        // A program that computes and prints nothing is a program whose
        // whole body the transpiler is free to throw away, so every one ends
        // by printing something it built.
        if let Some(binding) = self.pick_binding_of_any_type() {
            self.emit(&format!("print({});", binding.name));
        } else {
            self.emit("print(`done`);");
        }

        // The error handlers the body asked for, written last because a
        // function may be used above where it is declared.
        let trailing = std::mem::take(&mut self.trailing);
        for line in trailing {
            self.emit(&line);
        }

        let mut source = self.lines.join("\n");
        source.push('\n');
        source
    }

    // -- names ------------------------------------------------------------

    /// A fresh lowercase name. Nail refuses single letters other than the
    /// four coordinate names, so every one of these is a word and a number.
    fn fresh(&mut self, stem: &str) -> String {
        self.counter += 1;
        format!("{}_{}", stem, self.counter)
    }

    /// A fresh capitalized name, for a struct or an enum.
    fn fresh_type_name(&mut self, stem: &str) -> String {
        self.counter += 1;
        format!("{}{}", stem, self.counter)
    }

    // -- declarations -----------------------------------------------------

    fn declare_struct(&mut self) {
        let name = self.fresh_type_name("Shape");
        let field_count = self.rng.gen_range(1..4);
        let mut fields = Vec::new();
        for _ in 0..field_count {
            let field_name = self.fresh("field");
            // A struct field holds a plain value. Arrays of structs and
            // structs inside structs are legal Nail, and are left to the
            // mutation engine to build out of these.
            let ty = self.scalar_type();
            fields.push((field_name, ty));
        }
        self.emit(&format!("struct {} {{", name));
        let rendered: Vec<String> = fields.iter().map(|(field, ty)| format!("    {}:{}", field, ty.written())).collect();
        self.emit(&rendered.join(",\n"));
        self.emit("}");
        self.structs.push(StructDef { name, fields });
    }

    fn declare_enum(&mut self) {
        let name = self.fresh_type_name("Mode");
        let variant_count = self.rng.gen_range(2..4);
        let variants: Vec<String> = (0..variant_count)
            .map(|_| {
                self.counter += 1;
                format!("Choice{}", self.counter)
            })
            .collect();
        self.emit(&format!("enum {} {{", name));
        self.emit(&variants.iter().map(|variant| format!("    {}", variant)).collect::<Vec<_>>().join(",\n"));
        self.emit("}");
        self.enums.push(EnumDef { name, variants });
    }

    fn declare_function(&mut self) {
        let name = self.fresh("compute");
        let param_count = self.rng.gen_range(0..3);
        let mut params = Vec::new();
        for _ in 0..param_count {
            let param_name = self.fresh("input");
            params.push((param_name, self.scalar_type()));
        }
        let returns = self.scalar_type();
        // A quarter of functions can fail, because how a program deals with
        // an error is a whole side of the language: every call to one of
        // these has to be wrapped in safe, danger or expect.
        let fallible = self.rng.gen_bool(0.25);
        let written_params: Vec<String> = params.iter().map(|(param, ty)| format!("{}:{}", param, ty.written())).collect();
        self.emit(&format!("f {}({}):{}{} {{", name, written_params.join(", "), returns.written(), if fallible { "!e" } else { "" }));

        // A function body sees its parameters and nothing else, so the scope
        // stack is replaced rather than pushed for the length of the body.
        let outer = std::mem::replace(&mut self.scopes, vec![params.iter().map(|(param, ty)| Binding { name: param.clone(), ty: ty.clone() }).collect()]);
        self.indent += 1;
        for _ in 0..self.rng.gen_range(0..3) {
            self.statement(1);
        }
        // A fallible function is allowed to hand back a result it did not
        // make itself, and that shape is worth writing on its own: a body of
        // `r <call that can fail>;` is already a result, so a transpiler that
        // wraps a return in Ok produces Ok(Result<..>), which type checks in
        // Nail and then fails to build as Rust.
        let passed_through = if fallible && self.rng.gen_bool(0.35) { self.raw_fallible_call(&returns, 0) } else { None };
        if let Some(call) = passed_through {
            self.emit(&format!("r {};", call));
            self.indent -= 1;
            self.scopes = outer;
            self.emit("}");
            self.functions.push(FunctionDef { name, params, returns, fallible });
            return;
        }

        let value = self.expression(&returns, 0);
        if fallible {
            // One way out with a value and one with an error, which is what a
            // fallible function looks like when a person writes one.
            let condition = self.expression(&Ty::Bool, 1);
            self.emit("if {");
            self.indent += 1;
            self.emit(&format!("{} -> {{", condition));
            self.emit(&format!("    r e(`{} could not answer`);", name));
            self.emit("},");
            self.emit("else -> {");
            self.emit(&format!("    r {};", value));
            self.emit("}");
            self.indent -= 1;
            self.emit("}");
        } else {
            self.emit(&format!("r {};", value));
        }
        self.indent -= 1;
        self.scopes = outer;

        self.emit("}");
        self.functions.push(FunctionDef { name, params, returns, fallible });
    }

    // -- statements -------------------------------------------------------

    fn statement(&mut self, depth: usize) {
        // Deeper statements stay simple, so a program's size stays in the
        // hundreds of lines rather than exploding.
        let choice = if depth >= 2 { self.rng.gen_range(0..3) } else { self.rng.gen_range(0..12) };
        match choice {
            0 | 1 => self.declaration(),
            2 => {
                let Some(binding) = self.pick_binding_of_any_type() else { return self.declaration() };
                self.emit(&format!("print({});", binding.name));
            }
            3 => self.each_statement(depth),
            4 => self.if_statement(depth),
            5 => self.if_statement(depth),
            6 => self.each_statement(depth),
            7 => self.parallel_block(),
            8 => self.hashmap_statements(),
            9 => self.parallel_block(),
            10 => self.stdlib_statement(),
            _ => self.declaration(),
        }
    }

    /// A hashmap built, filled and read back. Hashmaps are their own corner of
    /// the type system, so they are written as a small script rather than
    /// threaded through expression generation.
    fn hashmap_statements(&mut self) {
        let value_type = match self.rng.gen_range(0..4) {
            0 => Ty::Int,
            1 => Ty::Float,
            2 => Ty::Text,
            _ => Ty::Bool,
        };
        let map_name = self.fresh("lookup");
        self.emit(&format!("{}:h<s,{}> = hashmap_new();", map_name, value_type.written()));
        let key = self.fresh("key");
        let value = self.leaf(&value_type);
        self.emit(&format!("hashmap_set({}, `{}`, {});", map_name, key, value));
        // The key was just written, so reading it back cannot fail, which is
        // what makes danger the honest form here.
        let read_name = self.fresh(stem_for(&value_type));
        self.emit(&format!("{}:{} = danger(hashmap_get({}, `{}`));", read_name, value_type.written(), map_name, key));
        self.bind(read_name, value_type);
    }

    /// A library call that returns nothing, which can stand on its own as a
    /// statement. Calls that return a value cannot: Nail has no bare
    /// expression statements.
    fn stdlib_statement(&mut self) {
        let candidates: Vec<&StdlibCall> = stdlib_calls().iter().filter(|call| call.returns == Returns::Nothing).collect();
        if candidates.is_empty() {
            return self.declaration();
        }
        let call = candidates[self.rng.gen_range(0..candidates.len())].clone();
        let arguments: Vec<String> = call.params.iter().map(|param| self.expression(param, 2)).collect();
        self.emit(&format!("{}({});", call.name, arguments.join(", ")));
    }

    fn declaration(&mut self) {
        let ty = self.any_type();
        let name = self.fresh(stem_for(&ty));
        let value = self.expression(&ty, 0);
        self.emit(&format!("{}:{} = {};", name, ty.written(), value));
        self.bind(name, ty);
    }

    fn each_statement(&mut self, depth: usize) {
        let element = self.scalar_type();
        let array = self.expression(&Ty::Array(Box::new(element.clone())), 0);
        let item = self.fresh("item");
        self.emit(&format!("each {} in {} {{", item, array));
        self.indent += 1;
        self.scopes.push(vec![Binding { name: item.clone(), ty: element }]);
        self.emit(&format!("print({});", item));
        for _ in 0..self.rng.gen_range(0..2) {
            self.statement(depth + 1);
        }
        self.scopes.pop();
        self.indent -= 1;
        self.emit("}");
    }

    fn if_statement(&mut self, depth: usize) {
        let condition = self.expression(&Ty::Bool, 0);
        self.emit(&format!("if {{"));
        self.indent += 1;
        self.emit(&format!("{} -> {{", condition));
        self.indent += 1;
        self.scopes.push(Vec::new());
        self.statement(depth + 1);
        self.scopes.pop();
        self.indent -= 1;
        self.emit("},");
        self.emit("else -> {");
        self.indent += 1;
        self.scopes.push(Vec::new());
        self.statement(depth + 1);
        self.scopes.pop();
        self.indent -= 1;
        self.emit("}");
        self.indent -= 1;
        self.emit("}");
    }

    /// Two declarations on real threads at once. Whatever is declared inside
    /// is visible after the block, which is the part worth testing: the
    /// transpiler has to hand every one of those values back out.
    fn parallel_block(&mut self) {
        let concurrent = self.rng.gen_bool(0.5);
        self.emit(if concurrent { "c" } else { "p" });
        self.indent += 1;
        let mut declared = Vec::new();
        for _ in 0..self.rng.gen_range(2..4) {
            let ty = self.scalar_type();
            let name = self.fresh(stem_for(&ty));
            let value = self.expression(&ty, 0);
            self.emit(&format!("{}:{} = {};", name, ty.written(), value));
            declared.push(Binding { name, ty });
        }
        self.indent -= 1;
        self.emit(if concurrent { "/c" } else { "/p" });
        for binding in declared {
            self.bind(binding.name, binding.ty);
        }
    }

    // -- expressions ------------------------------------------------------

    /// An expression of exactly the given type. Depth is what keeps this from
    /// recursing forever: past a couple of levels only leaves are produced.
    fn expression(&mut self, ty: &Ty, depth: usize) -> String {
        // A name already in scope is always the cheapest correct answer, and
        // using existing values is what makes generated programs look like
        // programs rather than like a list of literals.
        if depth > 0 && self.rng.gen_bool(0.35) {
            if let Some(binding) = self.pick_binding(ty) {
                return binding.name;
            }
        }
        if depth >= 3 {
            return self.leaf(ty);
        }
        // A library call, which is where most of the surface area of a real
        // program is, and where the transpiler has the most to get right.
        if self.rng.gen_bool(0.3) {
            if let Some(call) = self.stdlib_expression(ty, depth) {
                return call;
            }
        }
        match ty {
            Ty::Int => self.int_expression(depth),
            Ty::Float => self.float_expression(depth),
            Ty::Text => self.text_expression(depth),
            Ty::Bool => self.bool_expression(depth),
            Ty::Array(inner) => self.array_expression(inner, depth),
            Ty::Struct(_) | Ty::Enum(_) => self.leaf(ty),
        }
    }

    fn int_expression(&mut self, depth: usize) -> String {
        match self.rng.gen_range(0..8) {
            0 => self.leaf(&Ty::Int),
            1 => {
                // Small operands only. Arithmetic that overflows is a real
                // error the checker now reports, and a generator that
                // produced it constantly would drown out everything else.
                let left = self.expression(&Ty::Int, depth + 1);
                let right = self.expression(&Ty::Int, depth + 1);
                let operator = ["+", "-", "*"][self.rng.gen_range(0..3)];
                format!("({} {} {})", left, operator, right)
            }
            2 => {
                // A divisor written as a literal that is never zero, because
                // dividing by a zero the compiler can see is refused.
                let left = self.expression(&Ty::Int, depth + 1);
                let divisor = self.rng.gen_range(1..20);
                let operator = if self.rng.gen_bool(0.5) { "/" } else { "%" };
                format!("({} {} {})", left, operator, divisor)
            }
            3 => self.call_returning(&Ty::Int, depth).unwrap_or_else(|| self.leaf(&Ty::Int)),
            4 => self.if_expression(&Ty::Int, depth),
            5 => self.field_access(&Ty::Int).unwrap_or_else(|| self.leaf(&Ty::Int)),
            6 => self.find_expression(&Ty::Int, depth),
            _ => self.reduce_expression(depth),
        }
    }

    fn float_expression(&mut self, depth: usize) -> String {
        match self.rng.gen_range(0..4) {
            0 | 1 => self.leaf(&Ty::Float),
            2 => {
                let left = self.expression(&Ty::Float, depth + 1);
                let right = self.expression(&Ty::Float, depth + 1);
                let operator = ["+", "-", "*"][self.rng.gen_range(0..3)];
                format!("({} {} {})", left, operator, right)
            }
            _ => self.call_returning(&Ty::Float, depth).unwrap_or_else(|| self.leaf(&Ty::Float)),
        }
    }

    fn text_expression(&mut self, depth: usize) -> String {
        match self.rng.gen_range(0..3) {
            0 | 1 => self.leaf(&Ty::Text),
            _ => self.call_returning(&Ty::Text, depth).unwrap_or_else(|| self.leaf(&Ty::Text)),
        }
    }

    fn bool_expression(&mut self, depth: usize) -> String {
        match self.rng.gen_range(0..6) {
            0 => self.leaf(&Ty::Bool),
            1 => {
                let left = self.expression(&Ty::Int, depth + 1);
                let right = self.expression(&Ty::Int, depth + 1);
                let operator = ["==", "!=", "<", ">", "<=", ">="][self.rng.gen_range(0..6)];
                format!("({} {} {})", left, operator, right)
            }
            2 => {
                let left = self.expression(&Ty::Bool, depth + 1);
                let right = self.expression(&Ty::Bool, depth + 1);
                let operator = if self.rng.gen_bool(0.5) { "&&" } else { "||" };
                format!("({} {} {})", left, operator, right)
            }
            3 => format!("!{}", self.expression(&Ty::Bool, depth + 1)),
            4 => self.call_returning(&Ty::Bool, depth).unwrap_or_else(|| self.leaf(&Ty::Bool)),
            _ => {
                let element = self.scalar_type();
                let array = self.expression(&Ty::Array(Box::new(element.clone())), depth + 1);
                let item = self.fresh("item");
                let keyword = if self.rng.gen_bool(0.5) { "all" } else { "any" };
                let condition = self.comparison_on(&item, &element);
                format!("{} {} in {} {{ y {}; }}", keyword, item, array, condition)
            }
        }
    }

    fn array_expression(&mut self, element: &Ty, depth: usize) -> String {
        if *element == Ty::Int && self.rng.gen_bool(0.15) {
            return self.scan_expression(depth);
        }
        match self.rng.gen_range(0..4) {
            0 | 1 => {
                let count = self.rng.gen_range(1..5);
                let items: Vec<String> = (0..count).map(|_| self.expression(element, depth + 1)).collect();
                format!("[{}]", items.join(", "))
            }
            2 => {
                // map from an array of one type to an array of another, which
                // is where the transpiler has to get closures and types right
                let source_element = self.scalar_type();
                let array = self.expression(&Ty::Array(Box::new(source_element.clone())), depth + 1);
                let item = self.fresh("item");
                self.scopes.push(vec![Binding { name: item.clone(), ty: source_element }]);
                let body = self.expression(element, depth + 1);
                self.scopes.pop();
                format!("map {} in {} {{ y {}; }}", item, array, body)
            }
            _ => {
                let array = self.expression(&Ty::Array(Box::new(element.clone())), depth + 1);
                let item = self.fresh("item");
                let condition = self.comparison_on(&item, element);
                format!("filter {} in {} {{ y {}; }}", item, array, condition)
            }
        }
    }

    /// The first item of a collection that answers a question, with a handler
    /// for the collection that has none.
    fn find_expression(&mut self, ty: &Ty, depth: usize) -> String {
        let array = self.expression(&Ty::Array(Box::new(ty.clone())), depth + 1);
        let item = self.fresh("item");
        let condition = self.comparison_on(&item, ty);
        let handler = self.fallback_for(ty);
        format!("safe(find {} in {} {{ y {}; }}, {})", item, array, condition, handler)
    }

    /// Every step of a running total, which is reduce keeping its work.
    fn scan_expression(&mut self, depth: usize) -> String {
        let array = self.expression(&Ty::Array(Box::new(Ty::Int)), depth + 1);
        let accumulator = self.fresh("running");
        let item = self.fresh("item");
        format!("scan {} {} in {} from 0 {{ y {} + {}; }}", accumulator, item, array, accumulator, item)
    }

    fn reduce_expression(&mut self, depth: usize) -> String {
        let array = self.expression(&Ty::Array(Box::new(Ty::Int)), depth + 1);
        let accumulator = self.fresh("total");
        let item = self.fresh("item");
        format!("reduce {} {} in {} from 0 {{ y {} + {}; }}", accumulator, item, array, accumulator, item)
    }

    fn if_expression(&mut self, ty: &Ty, depth: usize) -> String {
        let condition = self.expression(&Ty::Bool, depth + 1);
        let yes = self.expression(ty, depth + 1);
        let no = self.expression(ty, depth + 1);
        format!("if {{ {} -> {{ r {}; }}, else -> {{ r {}; }} }}", condition, yes, no)
    }

    /// A call to a standard library function that produces the wanted type. A
    /// fallible one is wrapped in safe(...) with a handler, which is how Nail
    /// says "use this default when it fails" and keeps the program from
    /// crashing on a value the generator could not predict.
    fn stdlib_expression(&mut self, ty: &Ty, depth: usize) -> Option<String> {
        let candidates: Vec<StdlibCall> =
            stdlib_calls().iter().filter(|call| call.returns == Returns::Value(ty.clone()) || call.returns == Returns::Fallible(ty.clone())).cloned().collect();
        if candidates.is_empty() {
            return None;
        }
        let call = candidates[self.rng.gen_range(0..candidates.len())].clone();
        let arguments: Vec<String> = call.params.iter().map(|param| self.expression(param, depth + 1)).collect();
        let written = format!("{}({})", call.name, arguments.join(", "));
        match call.returns {
            Returns::Fallible(_) => {
                let handler = self.fallback_for(ty);
                Some(format!("safe({}, {})", written, handler))
            }
            _ => Some(written),
        }
    }

    /// The name of a handler that answers with a default value of this type.
    /// One per type per program, declared at the end.
    fn fallback_for(&mut self, ty: &Ty) -> String {
        if let Some((_, name)) = self.fallbacks.iter().find(|(candidate, _)| candidate == ty) {
            return name.clone();
        }
        let name = self.fresh("fallback");
        let parameter = self.fresh("failure");
        let default = self.leaf(ty);
        self.trailing.push(format!("f {}({}:e):{} {{\n    r {};\n}}", name, parameter, ty.written(), default));
        self.fallbacks.push((ty.clone(), name.clone()));
        name
    }

    /// A call to a function already declared, if one returns the wanted type
    /// and every argument can be produced.
    fn call_returning(&mut self, ty: &Ty, depth: usize) -> Option<String> {
        let candidates: Vec<usize> = self.functions.iter().enumerate().filter(|(_, function)| function.returns == *ty).map(|(index, _)| index).collect();
        if candidates.is_empty() {
            return None;
        }
        let index = candidates[self.rng.gen_range(0..candidates.len())];
        let name = self.functions[index].name.clone();
        let fallible = self.functions[index].fallible;
        let param_types: Vec<Ty> = self.functions[index].params.iter().map(|(_, param_type)| param_type.clone()).collect();
        let arguments: Vec<String> = param_types.iter().map(|param_type| self.expression(param_type, depth + 1)).collect();
        let written = format!("{}({})", name, arguments.join(", "));
        if !fallible {
            return Some(written);
        }
        // The three ways out of an error, all of them worth generating: a
        // handler, a crash, and an insistence that it cannot fail.
        Some(match self.rng.gen_range(0..4) {
            0 => format!("danger({})", written),
            1 => format!("expect({})", written),
            _ => {
                let handler = self.fallback_for(ty);
                format!("safe({}, {})", written, handler)
            }
        })
    }

    /// A call that can fail, written bare: no safe, no danger, no expect. It
    /// is only legal where a result is what is wanted, which in this
    /// generator means the body of a function declared to return one.
    fn raw_fallible_call(&mut self, ty: &Ty, depth: usize) -> Option<String> {
        let mine: Vec<usize> = self.functions.iter().enumerate().filter(|(_, function)| function.fallible && function.returns == *ty).map(|(index, _)| index).collect();
        let library: Vec<StdlibCall> = stdlib_calls().iter().filter(|call| call.returns == Returns::Fallible(ty.clone())).cloned().collect();
        let total = mine.len() + library.len();
        if total == 0 {
            return None;
        }
        let choice = self.rng.gen_range(0..total);
        if choice < mine.len() {
            let index = mine[choice];
            let name = self.functions[index].name.clone();
            let param_types: Vec<Ty> = self.functions[index].params.iter().map(|(_, param_type)| param_type.clone()).collect();
            let arguments: Vec<String> = param_types.iter().map(|param_type| self.expression(param_type, depth + 1)).collect();
            return Some(format!("{}({})", name, arguments.join(", ")));
        }
        let call = library[choice - mine.len()].clone();
        let arguments: Vec<String> = call.params.iter().map(|param| self.expression(param, depth + 1)).collect();
        Some(format!("{}({})", call.name, arguments.join(", ")))
    }

    /// Reading a field off a struct already in scope.
    fn field_access(&mut self, ty: &Ty) -> Option<String> {
        let bindings: Vec<Binding> = self.scopes.iter().flatten().cloned().collect();
        let mut options = Vec::new();
        for binding in bindings {
            if let Ty::Struct(struct_name) = &binding.ty {
                if let Some(definition) = self.structs.iter().find(|definition| definition.name == *struct_name) {
                    for (field, field_type) in &definition.fields {
                        if field_type == ty {
                            options.push(format!("{}.{}", binding.name, field));
                        }
                    }
                }
            }
        }
        if options.is_empty() {
            return None;
        }
        let index = self.rng.gen_range(0..options.len());
        Some(options[index].clone())
    }

    /// A literal, or the simplest possible value of a type that has no
    /// literal form of its own.
    fn leaf(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Int => format!("{}", self.rng.gen_range(-100..100)),
            Ty::Float => format!("{}.{}", self.rng.gen_range(-100..100), self.rng.gen_range(0..100)),
            Ty::Text => {
                let words = ["alpha", "beta", "gamma", "a b c", "", "line one", "quotes \" and \\ backslash", "unicode ünïcödé", "brace { and } close", "percent % and hash #"];
                let word = words[self.rng.gen_range(0..words.len())];
                // A tagged string carries a language marker for highlighters,
                // and is otherwise an ordinary string, so it belongs in the
                // pool of ordinary strings.
                match self.rng.gen_range(0..8) {
                    0 => format!("html`<b>{}</b>`", word),
                    1 => format!("sql`select `"),
                    _ => format!("`{}`", word),
                }
            }
            Ty::Bool => format!("{}", self.rng.gen_bool(0.5)),
            Ty::Array(inner) => {
                let count = self.rng.gen_range(1..4);
                let items: Vec<String> = (0..count).map(|_| self.leaf(inner)).collect();
                format!("[{}]", items.join(", "))
            }
            Ty::Struct(name) => {
                let Some(index) = self.structs.iter().position(|definition| definition.name == *name) else {
                    return "0".to_string();
                };
                let fields: Vec<(String, Ty)> = self.structs[index].fields.clone();
                let written: Vec<String> = fields.iter().map(|(field, field_type)| format!("{} = {}", field, self.leaf(field_type))).collect();
                format!("{} {{ {} }}", name, written.join(", "))
            }
            Ty::Enum(name) => {
                let Some(definition) = self.enums.iter().find(|definition| definition.name == *name) else {
                    return "0".to_string();
                };
                let variant = definition.variants[self.rng.gen_range(0..definition.variants.len())].clone();
                format!("{}::{}", name, variant)
            }
        }
    }

    /// A condition about one item of a collection, for the clauses that need
    /// one: `when`, `filter`, `all`, `any`.
    fn comparison_on(&mut self, item: &str, ty: &Ty) -> String {
        match ty {
            Ty::Int => format!("{} > {}", item, self.rng.gen_range(-50..50)),
            Ty::Float => format!("{} > {}.0", item, self.rng.gen_range(-50..50)),
            Ty::Bool => format!("{} == {}", item, self.rng.gen_bool(0.5)),
            Ty::Text => {
                let literal = self.leaf(&Ty::Text);
                format!("{} == {}", item, literal)
            }
            other => {
                let literal = self.leaf(other);
                format!("{} == {}", item, literal)
            }
        }
    }

    // -- environment ------------------------------------------------------

    /// A scalar type, including the structs and enums this program declared.
    fn scalar_type(&mut self) -> Ty {
        let named = self.structs.len() + self.enums.len();
        let choice = self.rng.gen_range(0..(4 + named.min(2)));
        match choice {
            0 => Ty::Int,
            1 => Ty::Float,
            2 => Ty::Text,
            3 => Ty::Bool,
            _ => {
                if !self.structs.is_empty() && (self.enums.is_empty() || self.rng.gen_bool(0.5)) {
                    let index = self.rng.gen_range(0..self.structs.len());
                    Ty::Struct(self.structs[index].name.clone())
                } else if !self.enums.is_empty() {
                    let index = self.rng.gen_range(0..self.enums.len());
                    Ty::Enum(self.enums[index].name.clone())
                } else {
                    Ty::Int
                }
            }
        }
    }

    /// Any type at all, arrays included.
    fn any_type(&mut self) -> Ty {
        if self.rng.gen_bool(0.25) {
            let element = self.scalar_type();
            // Arrays of structs work, arrays of arrays work, and both are
            // worth reaching, but only one level deep: nothing is learned
            // from a:a:a:a:i that a:a:i does not already say.
            if self.rng.gen_bool(0.2) {
                return Ty::Array(Box::new(Ty::Array(Box::new(element))));
            }
            return Ty::Array(Box::new(element));
        }
        self.scalar_type()
    }

    fn bind(&mut self, name: String, ty: Ty) {
        self.scopes.last_mut().expect("there is always a scope").push(Binding { name, ty });
    }

    fn pick_binding(&mut self, ty: &Ty) -> Option<Binding> {
        let matching: Vec<Binding> = self.scopes.iter().flatten().filter(|binding| binding.ty == *ty).cloned().collect();
        if matching.is_empty() {
            return None;
        }
        let index = self.rng.gen_range(0..matching.len());
        Some(matching[index].clone())
    }

    fn pick_binding_of_any_type(&mut self) -> Option<Binding> {
        let all: Vec<Binding> = self.scopes.iter().flatten().cloned().collect();
        if all.is_empty() {
            return None;
        }
        let index = self.rng.gen_range(0..all.len());
        Some(all[index].clone())
    }

    fn emit(&mut self, text: &str) {
        for line in text.lines() {
            if line.is_empty() {
                self.lines.push(String::new());
            } else {
                self.lines.push(format!("{}{}", "    ".repeat(self.indent), line));
            }
        }
    }
}

/// The word a variable of this type is named after, so a generated program
/// reads like one a person wrote and a finding is legible when it is read.
fn stem_for(ty: &Ty) -> &'static str {
    match ty {
        Ty::Int => "count",
        Ty::Float => "ratio",
        Ty::Text => "label",
        Ty::Bool => "flag",
        Ty::Array(_) => "items",
        Ty::Struct(_) => "shape",
        Ty::Enum(_) => "mode",
    }
}

// ---------------------------------------------------------------------------
// The run tier: programs that come with the answer
// ---------------------------------------------------------------------------
//
// Everything above writes programs to find out whether the compiler accepts
// them. This part writes programs to find out whether they are right. It
// keeps the value of every expression as it writes, so it knows exactly what
// the finished program prints, and a program that builds and then prints
// something else is a compiler that quietly changed the meaning of the code.
//
// Knowing the answer costs breadth. Nothing here calls the standard library,
// because what a library function returns is the library's to know, and
// nothing here depends on the clock, the machine or a random number. What is
// left is arithmetic, comparison, the collection forms, structs and calls to
// the program's own functions, which is where the transpiler does the work
// that a wrong answer would come from.

/// A value the predicting generator has worked out.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    List(Vec<Value>),
    Shape { name: String, fields: Vec<(String, Value)> },
}

impl Value {
    /// The value as Rust's Debug writes it, which is where print starts from:
    /// the library formats with {:?} and then unwraps a string's quotes.
    fn debug_form(&self) -> String {
        match self {
            Value::Int(number) => format!("{:?}", number),
            Value::Float(number) => format!("{:?}", number),
            Value::Text(text) => format!("{:?}", text),
            Value::Bool(flag) => format!("{:?}", flag),
            Value::List(items) => format!("[{}]", items.iter().map(Value::debug_form).collect::<Vec<_>>().join(", ")),
            Value::Shape { name, fields } => {
                let written: Vec<String> = fields.iter().map(|(field, value)| format!("{}: {}", field, value.debug_form())).collect();
                format!("{} {{ {} }}", name, written.join(", "))
            }
        }
    }

    /// The value written as Nail source. Only values the generator made up
    /// itself are ever written, so a float here is always a plain decimal
    /// with digits on both sides of the point.
    fn written(&self) -> Option<String> {
        match self {
            Value::Int(number) => Some(format!("{}", number)),
            Value::Float(number) => {
                let text = format!("{:?}", number);
                if !text.contains('.') || text.contains('e') || text.contains('E') {
                    return None;
                }
                Some(text)
            }
            Value::Text(text) => Some(format!("`{}`", text)),
            Value::Bool(flag) => Some(format!("{}", flag)),
            Value::List(items) => {
                let mut written = Vec::new();
                for item in items {
                    written.push(item.written()?);
                }
                Some(format!("[{}]", written.join(", ")))
            }
            Value::Shape { name, fields } => {
                let mut written = Vec::new();
                for (field, value) in fields {
                    written.push(format!("{} = {}", field, value.written()?));
                }
                Some(format!("{} {{ {} }}", name, written.join(", ")))
            }
        }
    }
}

/// The line `print` writes for a value: the Debug form, and then the same
/// unwrapping the library's print does to a string, so that `hello` prints as
/// hello rather than as "hello". Everything inside a collection keeps its
/// quotes, which is why an array of strings prints them and a bare string
/// does not.
fn printed_line(value: &Value) -> String {
    let formatted = value.debug_form();
    if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() > 1 {
        return crate::std_lib::print::unescape_debug_string(&formatted[1..formatted.len() - 1]);
    }
    formatted.replace("\\n", "\n")
}

/// Whether a value is one the generator is willing to carry. Small numbers
/// keep the arithmetic around them clear of the overflow the checker refuses,
/// and a float that stays in the ordinary range is one Rust writes back as a
/// plain decimal rather than in exponent form.
fn sane(value: &Value) -> bool {
    match value {
        Value::Int(number) => number.abs() <= 1_000_000,
        Value::Float(number) => number.is_finite() && number.abs() <= 1_000_000.0 && (*number == 0.0 || number.abs() >= 0.000_001),
        Value::Text(text) => text.chars().count() <= 200,
        Value::Bool(_) => true,
        Value::List(items) => items.len() <= 32 && items.iter().all(sane),
        Value::Shape { fields, .. } => fields.iter().all(|(_, value)| sane(value)),
    }
}

/// The value an item stands for while a loop body is being written, when the
/// collection it walks turns out to be empty. Nothing is recorded from a body
/// that never runs, so this only has to be a value of the right type.
fn stand_in(ty: &Ty) -> Value {
    match ty {
        Ty::Float => Value::Float(0.0),
        Ty::Text => Value::Text(String::new()),
        Ty::Bool => Value::Bool(false),
        _ => Value::Int(0),
    }
}

/// One expression, kept as a tree rather than as text, because the generator
/// has to do two things with it: write it out, and work out what it comes to.
/// A function's body is the reason it is a tree at all, since a call has to be
/// worked out again for every set of arguments it is given.
#[derive(Debug, Clone)]
enum Expr {
    Lit(Value),
    Name(String),
    Binary { operator: &'static str, left: Box<Expr>, right: Box<Expr> },
    Negate(Box<Expr>),
    Choice { condition: Box<Expr>, yes: Box<Expr>, no: Box<Expr> },
    ListOf(Vec<Expr>),
    ShapeOf { name: String, fields: Vec<(String, Expr)> },
    Field { shape: String, field: String },
    Map { item: String, source: Box<Expr>, body: Box<Expr> },
    Keep { item: String, source: Box<Expr>, body: Box<Expr> },
    /// reduce and scan, which are the same walk: one keeps only the last
    /// answer and the other keeps every step of it.
    Fold { accumulator: String, item: String, source: Box<Expr>, start: Box<Expr>, body: Box<Expr>, keeps_steps: bool },
    Call { name: String, arguments: Vec<Expr> },
}

impl Expr {
    /// The expression as Nail source.
    fn written(&self) -> Option<String> {
        Some(match self {
            Expr::Lit(value) => value.written()?,
            Expr::Name(name) => name.clone(),
            Expr::Binary { operator, left, right } => format!("({} {} {})", left.written()?, operator, right.written()?),
            Expr::Negate(inner) => format!("!{}", inner.written()?),
            Expr::Choice { condition, yes, no } => format!("if {{ {} -> {{ r {}; }}, else -> {{ r {}; }} }}", condition.written()?, yes.written()?, no.written()?),
            Expr::ListOf(items) => {
                let mut written = Vec::new();
                for item in items {
                    written.push(item.written()?);
                }
                format!("[{}]", written.join(", "))
            }
            Expr::ShapeOf { name, fields } => {
                let mut written = Vec::new();
                for (field, value) in fields {
                    written.push(format!("{} = {}", field, value.written()?));
                }
                format!("{} {{ {} }}", name, written.join(", "))
            }
            Expr::Field { shape, field } => format!("{}.{}", shape, field),
            Expr::Map { item, source, body } => format!("map {} in {} {{ y {}; }}", item, source.written()?, body.written()?),
            Expr::Keep { item, source, body } => format!("filter {} in {} {{ y {}; }}", item, source.written()?, body.written()?),
            Expr::Fold { accumulator, item, source, start, body, keeps_steps } => {
                format!("{} {} {} in {} from {} {{ y {}; }}", if *keeps_steps { "scan" } else { "reduce" }, accumulator, item, source.written()?, start.written()?, body.written()?)
            }
            Expr::Call { name, arguments } => {
                let mut written = Vec::new();
                for argument in arguments {
                    written.push(argument.written()?);
                }
                format!("{}({})", name, written.join(", "))
            }
        })
    }

    /// What the expression comes to. Nothing here can fail at runtime, so a
    /// None is the generator admitting it does not know, and a case it does
    /// not know the answer to is one it never writes.
    fn evaluate(&self, bindings: &mut Vec<(String, Value)>, functions: &[PredictedFunction]) -> Option<Value> {
        match self {
            Expr::Lit(value) => Some(value.clone()),
            Expr::Name(name) => look_up(bindings, name),
            Expr::Binary { operator, left, right } => {
                let left = left.evaluate(bindings, functions)?;
                let right = right.evaluate(bindings, functions)?;
                binary_value(operator, &left, &right)
            }
            Expr::Negate(inner) => match inner.evaluate(bindings, functions)? {
                Value::Bool(flag) => Some(Value::Bool(!flag)),
                _ => None,
            },
            // Only the branch that runs is worked out, because only the
            // branch that runs is what the program computes.
            Expr::Choice { condition, yes, no } => match condition.evaluate(bindings, functions)? {
                Value::Bool(true) => yes.evaluate(bindings, functions),
                Value::Bool(false) => no.evaluate(bindings, functions),
                _ => None,
            },
            Expr::ListOf(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(item.evaluate(bindings, functions)?);
                }
                Some(Value::List(values))
            }
            Expr::ShapeOf { name, fields } => {
                let mut values = Vec::new();
                for (field, value) in fields {
                    values.push((field.clone(), value.evaluate(bindings, functions)?));
                }
                Some(Value::Shape { name: name.clone(), fields: values })
            }
            Expr::Field { shape, field } => match look_up(bindings, shape)? {
                Value::Shape { fields, .. } => fields.iter().find(|(candidate, _)| candidate == field).map(|(_, value)| value.clone()),
                _ => None,
            },
            Expr::Map { item, source, body } => {
                let Value::List(items) = source.evaluate(bindings, functions)? else { return None };
                let mut mapped = Vec::new();
                for element in items {
                    bindings.push((item.clone(), element));
                    let value = body.evaluate(bindings, functions);
                    bindings.pop();
                    mapped.push(value?);
                }
                Some(Value::List(mapped))
            }
            Expr::Keep { item, source, body } => {
                let Value::List(items) = source.evaluate(bindings, functions)? else { return None };
                let mut kept = Vec::new();
                for element in items {
                    bindings.push((item.clone(), element.clone()));
                    let value = body.evaluate(bindings, functions);
                    bindings.pop();
                    match value? {
                        Value::Bool(true) => kept.push(element),
                        Value::Bool(false) => {}
                        _ => return None,
                    }
                }
                Some(Value::List(kept))
            }
            Expr::Fold { accumulator, item, source, start, body, keeps_steps } => {
                let Value::List(items) = source.evaluate(bindings, functions)? else { return None };
                let mut carried = start.evaluate(bindings, functions)?;
                let mut steps = Vec::new();
                for element in items {
                    bindings.push((accumulator.clone(), carried.clone()));
                    bindings.push((item.clone(), element));
                    let next = body.evaluate(bindings, functions);
                    bindings.pop();
                    bindings.pop();
                    carried = next?;
                    if *keeps_steps {
                        steps.push(carried.clone());
                    }
                }
                if *keeps_steps {
                    return Some(Value::List(steps));
                }
                Some(carried)
            }
            Expr::Call { name, arguments } => {
                let function = functions.iter().find(|candidate| candidate.name == *name)?;
                let mut values = Vec::new();
                for argument in arguments {
                    values.push(argument.evaluate(bindings, functions)?);
                }
                if values.len() != function.parameters.len() {
                    return None;
                }
                // A function body sees its parameters and nothing else, which
                // is Nail's rule and also what makes a call worth predicting:
                // the same arguments always give the same answer.
                let mut inner: Vec<(String, Value)> = function.parameters.iter().map(|(parameter, _)| parameter.clone()).zip(values).collect();
                function.body.evaluate(&mut inner, functions)
            }
        }
    }
}

/// The innermost binding of a name, since a loop's item shadows nothing but
/// is pushed on top while the body is being worked out.
fn look_up(bindings: &[(String, Value)], name: &str) -> Option<Value> {
    bindings.iter().rev().find(|(candidate, _)| candidate == name).map(|(_, value)| value.clone())
}

/// One operator applied to two values, with the same answers Rust gives:
/// whole number division truncates toward zero, and anything that would
/// overflow is refused rather than wrapped, because the checker refuses it
/// too.
fn binary_value(operator: &str, left: &Value, right: &Value) -> Option<Value> {
    match (left, right) {
        (Value::Int(left), Value::Int(right)) => match operator {
            "+" => left.checked_add(*right).map(Value::Int),
            "-" => left.checked_sub(*right).map(Value::Int),
            "*" => left.checked_mul(*right).map(Value::Int),
            "/" => left.checked_div(*right).map(Value::Int),
            "%" => left.checked_rem(*right).map(Value::Int),
            "==" => Some(Value::Bool(left == right)),
            "!=" => Some(Value::Bool(left != right)),
            "<" => Some(Value::Bool(left < right)),
            ">" => Some(Value::Bool(left > right)),
            "<=" => Some(Value::Bool(left <= right)),
            ">=" => Some(Value::Bool(left >= right)),
            _ => None,
        },
        (Value::Float(left), Value::Float(right)) => match operator {
            "+" => Some(Value::Float(left + right)),
            "-" => Some(Value::Float(left - right)),
            "*" => Some(Value::Float(left * right)),
            "==" => Some(Value::Bool(left == right)),
            "!=" => Some(Value::Bool(left != right)),
            "<" => Some(Value::Bool(left < right)),
            ">" => Some(Value::Bool(left > right)),
            "<=" => Some(Value::Bool(left <= right)),
            ">=" => Some(Value::Bool(left >= right)),
            _ => None,
        },
        (Value::Text(left), Value::Text(right)) => match operator {
            "==" => Some(Value::Bool(left == right)),
            "!=" => Some(Value::Bool(left != right)),
            _ => None,
        },
        (Value::Bool(left), Value::Bool(right)) => match operator {
            "==" => Some(Value::Bool(left == right)),
            "!=" => Some(Value::Bool(left != right)),
            "&&" => Some(Value::Bool(*left && *right)),
            "||" => Some(Value::Bool(*left || *right)),
            _ => None,
        },
        _ => None,
    }
}

struct PredictedShape {
    name: String,
    fields: Vec<(String, Ty)>,
}

struct PredictedFunction {
    name: String,
    parameters: Vec<(String, Ty)>,
    returns: Ty,
    body: Expr,
}

#[derive(Clone)]
struct Known {
    name: String,
    ty: Ty,
    value: Value,
}

/// A program, where it came from, and exactly what it prints. The third
/// string is the whole of the program's standard output, byte for byte, so
/// the run tier can compare rather than guess.
pub fn case_with_expected(seed: u64) -> Option<(String, String, String)> {
    let mut generator = Predicting::new(seed);
    let (source, expected) = generator.program()?;
    Some((source, format!("predict seed {}", seed), expected))
}

struct Predicting {
    rng: StdRng,
    counter: usize,
    shapes: Vec<PredictedShape>,
    functions: Vec<PredictedFunction>,
    scopes: Vec<Vec<Known>>,
    lines: Vec<String>,
    indent: usize,
    /// What the program prints, in order, one entry per line of output.
    printed: Vec<String>,
    /// Whether what is being written now is code that actually runs. The
    /// branch an if does not take is still written, and what it would have
    /// printed is not counted.
    running: bool,
}

impl Predicting {
    fn new(seed: u64) -> Predicting {
        Predicting {
            // A stream of its own, so that seed 7 here and seed 7 in the full
            // generator are two different programs rather than one being the
            // beginning of the other.
            rng: StdRng::seed_from_u64(seed ^ 0x9e37_79b9_7f4a_7c15),
            counter: 0,
            shapes: Vec::new(),
            functions: Vec::new(),
            scopes: vec![Vec::new()],
            lines: Vec::new(),
            indent: 0,
            printed: Vec::new(),
            running: true,
        }
    }

    fn program(&mut self) -> Option<(String, String)> {
        self.emit("nail latest");
        for _ in 0..self.rng.gen_range(0..3) {
            self.declare_shape();
        }
        for _ in 0..self.rng.gen_range(0..3) {
            self.declare_function();
        }
        let statements = self.rng.gen_range(3..9);
        for _ in 0..statements {
            self.statement(0)?;
        }
        self.final_print()?;

        let mut source = self.lines.join("\n");
        source.push('\n');
        let mut expected = String::new();
        for line in &self.printed {
            expected.push_str(line);
            expected.push('\n');
        }
        Some((source, expected))
    }

    // -- declarations -----------------------------------------------------

    fn declare_shape(&mut self) {
        self.counter += 1;
        let name = format!("Shape{}", self.counter);
        let count = self.rng.gen_range(1..4);
        let mut fields = Vec::new();
        for _ in 0..count {
            let field = self.fresh("field");
            fields.push((field, self.printable_scalar()));
        }
        self.emit(&format!("struct {} {{", name));
        let written: Vec<String> = fields.iter().map(|(field, ty)| format!("    {}:{}", field, ty.written())).collect();
        self.emit(&written.join(",\n"));
        self.emit("}");
        self.shapes.push(PredictedShape { name, fields });
    }

    /// A function whose body is one expression of its parameters. One
    /// expression rather than a body of statements is what keeps a call
    /// predictable: the same arguments always give the same answer, so the
    /// generator can work a call out wherever it appears.
    fn declare_function(&mut self) {
        let name = self.fresh("compute");
        let count = self.rng.gen_range(0..3);
        let mut parameters = Vec::new();
        for _ in 0..count {
            let parameter = self.fresh("input");
            let ty = self.printable_scalar();
            parameters.push((parameter, ty));
        }
        let returns = self.printable_scalar();

        let mut scope = Vec::new();
        for (parameter, ty) in &parameters {
            let value = self.leaf_value(ty);
            scope.push(Known { name: parameter.clone(), ty: ty.clone(), value });
        }
        let outer = std::mem::replace(&mut self.scopes, vec![scope]);
        let written = self.expression(&returns, 0);
        self.scopes = outer;

        let Some((body, _)) = written else { return };
        let Some(text) = body.written() else { return };
        let signature: Vec<String> = parameters.iter().map(|(parameter, ty)| format!("{}:{}", parameter, ty.written())).collect();
        self.emit(&format!("f {}({}):{} {{", name, signature.join(", "), returns.written()));
        self.indent += 1;
        self.emit(&format!("r {};", text));
        self.indent -= 1;
        self.emit("}");
        self.functions.push(PredictedFunction { name, parameters, returns, body });
    }

    // -- statements -------------------------------------------------------

    fn statement(&mut self, depth: usize) -> Option<()> {
        let choice = if depth >= 2 { self.rng.gen_range(0..3) } else { self.rng.gen_range(0..9) };
        match choice {
            0 | 1 => self.declaration(),
            2 => self.print_statement(),
            3 | 4 => self.walk(),
            5 => self.branch(depth),
            6 => self.branch(depth),
            7 => self.parallel_block(),
            _ => self.declaration(),
        }
    }

    fn declaration(&mut self) -> Option<()> {
        let ty = self.any_type();
        let (expr, value) = self.expression(&ty, 0)?;
        let text = expr.written()?;
        let name = self.fresh(stem_for(&ty));
        self.emit(&format!("{}:{} = {};", name, ty.written(), text));
        self.bind(name, ty, value);
        Some(())
    }

    fn print_statement(&mut self) -> Option<()> {
        let known: Vec<Known> = self.scopes.iter().flatten().cloned().collect();
        if known.is_empty() {
            return self.declaration();
        }
        let index = self.rng.gen_range(0..known.len());
        let chosen = known[index].clone();
        self.emit(&format!("print({});", chosen.name));
        if self.running {
            self.printed.push(printed_line(&chosen.value));
        }
        Some(())
    }

    /// An if whose condition the generator has already worked out, so it
    /// knows which arm runs. Both arms are written, because the compiler has
    /// to compile both, and only the one that runs is counted as output.
    fn branch(&mut self, depth: usize) -> Option<()> {
        let (condition, value) = self.expression(&Ty::Bool, 0)?;
        let Value::Bool(taken) = value else { return None };
        let text = condition.written()?;
        let outer = self.running;

        self.emit("if {");
        self.indent += 1;
        self.emit(&format!("{} -> {{", text));
        self.indent += 1;
        self.running = outer && taken;
        self.scopes.push(Vec::new());
        self.statement(depth + 1)?;
        self.scopes.pop();
        self.indent -= 1;
        self.emit("},");
        self.emit("else -> {");
        self.indent += 1;
        self.running = outer && !taken;
        self.scopes.push(Vec::new());
        self.statement(depth + 1)?;
        self.scopes.pop();
        self.indent -= 1;
        self.emit("}");
        self.indent -= 1;
        self.emit("}");
        self.running = outer;
        Some(())
    }

    fn parallel_block(&mut self) -> Option<()> {
        let concurrent = self.rng.gen_bool(0.5);
        self.emit(if concurrent { "c" } else { "p" });
        self.indent += 1;
        let count = self.rng.gen_range(2..4);
        let mut declared = Vec::new();
        for _ in 0..count {
            let ty = self.printable_scalar();
            let (expr, value) = self.expression(&ty, 0)?;
            let text = expr.written()?;
            let name = self.fresh(stem_for(&ty));
            self.emit(&format!("{}:{} = {};", name, ty.written(), text));
            declared.push(Known { name, ty, value });
        }
        self.indent -= 1;
        self.emit(if concurrent { "/c" } else { "/p" });
        for known in declared {
            self.bind(known.name, known.ty, known.value);
        }
        Some(())
    }

    /// A loop that prints each item, which is the one place a generated
    /// program's output has a shape rather than a length: the collection
    /// decides how many lines come out.
    fn walk(&mut self) -> Option<()> {
        let element = self.printable_scalar();
        let (source, source_value) = self.expression(&Ty::Array(Box::new(element.clone())), 0)?;
        let Value::List(items) = source_value else { return None };
        let source_text = source.written()?;

        let item = self.fresh("item");
        let representative = items.first().cloned().unwrap_or_else(|| stand_in(&element));
        self.scopes.push(vec![Known { name: item.clone(), ty: element.clone(), value: representative }]);
        let mut prints = vec![Expr::Name(item.clone())];
        if self.rng.gen_bool(0.5) {
            let ty = self.printable_scalar();
            if let Some((expr, _)) = self.expression(&ty, 1) {
                prints.push(expr);
            }
        }
        self.scopes.pop();

        // The body runs once per item, so every item has to be worked out,
        // not only the first. When one of them cannot be, the extra print
        // goes and the loop keeps the item print, which always can.
        let mut lines = self.walk_output(&items, &item, &prints);
        if lines.is_none() {
            prints.truncate(1);
            lines = self.walk_output(&items, &item, &prints);
        }
        let lines = lines?;

        let mut written = Vec::new();
        for expr in &prints {
            written.push(expr.written()?);
        }
        self.emit(&format!("each {} in {} {{", item, source_text));
        self.indent += 1;
        for text in written {
            self.emit(&format!("print({});", text));
        }
        self.indent -= 1;
        self.emit("}");
        if self.running {
            self.printed.extend(lines);
        }
        Some(())
    }

    /// What a loop prints, worked out item by item.
    fn walk_output(&self, items: &[Value], item: &str, prints: &[Expr]) -> Option<Vec<String>> {
        let base = self.environment();
        let mut lines = Vec::new();
        for element in items {
            let mut bindings = base.clone();
            bindings.push((item.to_string(), element.clone()));
            for expr in prints {
                let value = expr.evaluate(&mut bindings, &self.functions)?;
                if !sane(&value) {
                    return None;
                }
                lines.push(printed_line(&value));
            }
        }
        Some(lines)
    }

    /// Every program ends by printing something it built, so that the whole
    /// body is work the compiler has to keep.
    fn final_print(&mut self) -> Option<()> {
        let known: Vec<Known> = self.scopes.iter().flatten().cloned().collect();
        if known.is_empty() {
            self.emit("print(`done`);");
            self.printed.push("done".to_string());
            return Some(());
        }
        let index = self.rng.gen_range(0..known.len());
        let chosen = known[index].clone();
        self.emit(&format!("print({});", chosen.name));
        self.printed.push(printed_line(&chosen.value));
        Some(())
    }

    // -- expressions ------------------------------------------------------

    /// An expression of exactly this type, and the value it comes to. Every
    /// form is tried and then worked out: one that cannot be worked out, or
    /// that comes to a number too big to be safe, is dropped for a literal,
    /// so a program is never written that the generator cannot predict.
    fn expression(&mut self, ty: &Ty, depth: usize) -> Option<(Expr, Value)> {
        if depth > 0 && self.rng.gen_bool(0.35) {
            if let Some(known) = self.pick_binding(ty) {
                return Some((Expr::Name(known.name), known.value));
            }
        }
        if depth >= 3 {
            return self.leaf(ty);
        }
        let candidate = match ty {
            Ty::Int => self.int_expression(depth),
            Ty::Float => self.float_expression(depth),
            Ty::Text => self.text_expression(depth),
            Ty::Bool => self.bool_expression(depth),
            Ty::Array(element) => {
                let element = element.as_ref().clone();
                self.array_expression(&element, depth)
            }
            Ty::Struct(name) => {
                let name = name.clone();
                self.shape_expression(&name, depth)
            }
            Ty::Enum(_) => None,
        };
        if let Some(expr) = candidate {
            if expr.written().is_some() {
                let mut bindings = self.environment();
                if let Some(value) = expr.evaluate(&mut bindings, &self.functions) {
                    if sane(&value) {
                        return Some((expr, value));
                    }
                }
            }
        }
        self.leaf(ty)
    }

    fn int_expression(&mut self, depth: usize) -> Option<Expr> {
        match self.rng.gen_range(0..8) {
            0 => None,
            1 => {
                let (left, _) = self.expression(&Ty::Int, depth + 1)?;
                let (right, _) = self.expression(&Ty::Int, depth + 1)?;
                let operator = ["+", "-", "*"][self.rng.gen_range(0..3)];
                Some(Expr::Binary { operator, left: Box::new(left), right: Box::new(right) })
            }
            2 => {
                // A divisor written as a literal that is never zero, because
                // dividing by a zero the compiler can see is refused.
                let (left, _) = self.expression(&Ty::Int, depth + 1)?;
                let divisor = self.rng.gen_range(1..20);
                let operator = if self.rng.gen_bool(0.5) { "/" } else { "%" };
                Some(Expr::Binary { operator, left: Box::new(left), right: Box::new(Expr::Lit(Value::Int(divisor))) })
            }
            3 => self.call_returning(&Ty::Int, depth),
            4 => self.choice_expression(&Ty::Int, depth),
            5 => self.field_expression(&Ty::Int),
            6 => self.fold_expression(depth, false),
            _ => None,
        }
    }

    fn float_expression(&mut self, depth: usize) -> Option<Expr> {
        match self.rng.gen_range(0..6) {
            0 | 1 => None,
            2 => {
                let (left, _) = self.expression(&Ty::Float, depth + 1)?;
                let (right, _) = self.expression(&Ty::Float, depth + 1)?;
                let operator = ["+", "-", "*"][self.rng.gen_range(0..3)];
                Some(Expr::Binary { operator, left: Box::new(left), right: Box::new(right) })
            }
            3 => self.call_returning(&Ty::Float, depth),
            4 => self.choice_expression(&Ty::Float, depth),
            _ => self.field_expression(&Ty::Float),
        }
    }

    fn text_expression(&mut self, depth: usize) -> Option<Expr> {
        match self.rng.gen_range(0..5) {
            0 | 1 => None,
            2 => self.call_returning(&Ty::Text, depth),
            3 => self.choice_expression(&Ty::Text, depth),
            _ => self.field_expression(&Ty::Text),
        }
    }

    fn bool_expression(&mut self, depth: usize) -> Option<Expr> {
        match self.rng.gen_range(0..7) {
            0 => None,
            1 | 2 => self.comparison(depth),
            3 => {
                let (left, _) = self.expression(&Ty::Bool, depth + 1)?;
                let (right, _) = self.expression(&Ty::Bool, depth + 1)?;
                let operator = if self.rng.gen_bool(0.5) { "&&" } else { "||" };
                Some(Expr::Binary { operator, left: Box::new(left), right: Box::new(right) })
            }
            4 => {
                let (inner, _) = self.expression(&Ty::Bool, depth + 1)?;
                Some(Expr::Negate(Box::new(inner)))
            }
            5 => self.call_returning(&Ty::Bool, depth),
            _ => self.choice_expression(&Ty::Bool, depth),
        }
    }

    /// Two values of one type, compared. Ordering is only asked of numbers,
    /// which is where Nail allows it.
    fn comparison(&mut self, depth: usize) -> Option<Expr> {
        let ty = self.printable_scalar();
        let (left, _) = self.expression(&ty, depth + 1)?;
        let (right, _) = self.expression(&ty, depth + 1)?;
        let operator = match ty {
            Ty::Text | Ty::Bool => ["==", "!="][self.rng.gen_range(0..2)],
            _ => ["==", "!=", "<", ">", "<=", ">="][self.rng.gen_range(0..6)],
        };
        Some(Expr::Binary { operator, left: Box::new(left), right: Box::new(right) })
    }

    fn array_expression(&mut self, element: &Ty, depth: usize) -> Option<Expr> {
        match self.rng.gen_range(0..6) {
            0 | 1 => {
                let count = self.rng.gen_range(1..5);
                let mut items = Vec::new();
                for _ in 0..count {
                    let (expr, _) = self.expression(element, depth + 1)?;
                    items.push(expr);
                }
                Some(Expr::ListOf(items))
            }
            2 => self.map_expression(element, depth),
            3 => self.keep_expression(element, depth),
            4 if *element == Ty::Int => self.fold_expression(depth, true),
            _ => None,
        }
    }

    /// map from an array of one type to an array of another, which is where
    /// the transpiler has to get closures and their types right.
    fn map_expression(&mut self, element: &Ty, depth: usize) -> Option<Expr> {
        let source_element = self.printable_scalar();
        let (source, source_value) = self.expression(&Ty::Array(Box::new(source_element.clone())), depth + 1)?;
        let Value::List(items) = source_value else { return None };
        let item = self.fresh("item");
        let representative = items.first().cloned().unwrap_or_else(|| stand_in(&source_element));
        self.scopes.push(vec![Known { name: item.clone(), ty: source_element, value: representative }]);
        let body = self.expression(element, depth + 1);
        self.scopes.pop();
        let (body, _) = body?;
        Some(Expr::Map { item, source: Box::new(source), body: Box::new(body) })
    }

    fn keep_expression(&mut self, element: &Ty, depth: usize) -> Option<Expr> {
        let (source, source_value) = self.expression(&Ty::Array(Box::new(element.clone())), depth + 1)?;
        let Value::List(_) = source_value else { return None };
        let item = self.fresh("item");
        let (comparison, _) = self.comparison_on(&item, element);
        Some(Expr::Keep { item, source: Box::new(source), body: Box::new(comparison) })
    }

    /// A running total over whole numbers, kept whole or kept step by step.
    /// The step is always addition on purpose: the transpiler turns a fold it
    /// recognizes as associative into a parallel one, and addition is the
    /// case where every order of the work gives the same answer.
    fn fold_expression(&mut self, depth: usize, keeps_steps: bool) -> Option<Expr> {
        let (source, source_value) = self.expression(&Ty::Array(Box::new(Ty::Int)), depth + 1)?;
        let Value::List(_) = source_value else { return None };
        let accumulator = self.fresh("total");
        let item = self.fresh("item");
        let body = Expr::Binary { operator: "+", left: Box::new(Expr::Name(accumulator.clone())), right: Box::new(Expr::Name(item.clone())) };
        Some(Expr::Fold { accumulator, item, source: Box::new(source), start: Box::new(Expr::Lit(Value::Int(0))), body: Box::new(body), keeps_steps })
    }

    fn choice_expression(&mut self, ty: &Ty, depth: usize) -> Option<Expr> {
        let (condition, _) = self.expression(&Ty::Bool, depth + 1)?;
        let (yes, _) = self.expression(ty, depth + 1)?;
        let (no, _) = self.expression(ty, depth + 1)?;
        Some(Expr::Choice { condition: Box::new(condition), yes: Box::new(yes), no: Box::new(no) })
    }

    /// A condition about one item of a collection, for the clause that filter
    /// needs.
    fn comparison_on(&mut self, item: &str, ty: &Ty) -> (Expr, Value) {
        let value = self.leaf_value(ty);
        let operator = match ty {
            Ty::Text | Ty::Bool => ["==", "!="][self.rng.gen_range(0..2)],
            _ => ["==", "!=", "<", ">", "<=", ">="][self.rng.gen_range(0..6)],
        };
        (Expr::Binary { operator, left: Box::new(Expr::Name(item.to_string())), right: Box::new(Expr::Lit(value.clone())) }, value)
    }

    fn call_returning(&mut self, ty: &Ty, depth: usize) -> Option<Expr> {
        let candidates: Vec<usize> = self.functions.iter().enumerate().filter(|(_, function)| function.returns == *ty).map(|(index, _)| index).collect();
        if candidates.is_empty() {
            return None;
        }
        let index = candidates[self.rng.gen_range(0..candidates.len())];
        let name = self.functions[index].name.clone();
        let parameter_types: Vec<Ty> = self.functions[index].parameters.iter().map(|(_, parameter_type)| parameter_type.clone()).collect();
        let mut arguments = Vec::new();
        for parameter_type in &parameter_types {
            let (expr, _) = self.expression(parameter_type, depth + 1)?;
            arguments.push(expr);
        }
        Some(Expr::Call { name, arguments })
    }

    fn shape_expression(&mut self, name: &str, depth: usize) -> Option<Expr> {
        let fields: Vec<(String, Ty)> = self.shapes.iter().find(|shape| shape.name == name)?.fields.clone();
        let mut written = Vec::new();
        for (field, ty) in fields {
            let (expr, _) = self.expression(&ty, depth + 1)?;
            written.push((field, expr));
        }
        Some(Expr::ShapeOf { name: name.to_string(), fields: written })
    }

    /// Reading a field off a struct already in scope.
    fn field_expression(&mut self, ty: &Ty) -> Option<Expr> {
        let mut options = Vec::new();
        for known in self.scopes.iter().flatten() {
            let Ty::Struct(shape_name) = &known.ty else { continue };
            let Some(shape) = self.shapes.iter().find(|shape| shape.name == *shape_name) else { continue };
            for (field, field_type) in &shape.fields {
                if field_type == ty {
                    options.push((known.name.clone(), field.clone()));
                }
            }
        }
        if options.is_empty() {
            return None;
        }
        let index = self.rng.gen_range(0..options.len());
        let (shape, field) = options[index].clone();
        Some(Expr::Field { shape, field })
    }

    fn leaf(&mut self, ty: &Ty) -> Option<(Expr, Value)> {
        let value = self.leaf_value(ty);
        if !sane(&value) || value.written().is_none() {
            return None;
        }
        Some((Expr::Lit(value.clone()), value))
    }

    fn leaf_value(&mut self, ty: &Ty) -> Value {
        match ty {
            Ty::Int => Value::Int(self.rng.gen_range(-50..50)),
            Ty::Float => {
                let whole: i64 = self.rng.gen_range(-50..50);
                let fraction: i64 = self.rng.gen_range(0..100);
                // Built as text and read back, so the literal in the program
                // and the number the generator carries are the same number.
                Value::Float(format!("{}.{}", whole, fraction).parse().unwrap_or(0.0))
            }
            Ty::Text => {
                // Plain words on purpose. What print does to a quote or a
                // backslash is the full generator's business, and a run tier
                // that argued about escaping would be testing the escaping
                // rather than the program.
                let words = ["alpha", "beta", "gamma", "delta", "a b c", "", "line one", "north", "seven", "x y z"];
                Value::Text(words[self.rng.gen_range(0..words.len())].to_string())
            }
            Ty::Bool => Value::Bool(self.rng.gen_bool(0.5)),
            Ty::Array(element) => {
                let element = element.as_ref().clone();
                let count = self.rng.gen_range(1..5);
                let mut items = Vec::new();
                for _ in 0..count {
                    items.push(self.leaf_value(&element));
                }
                Value::List(items)
            }
            Ty::Struct(name) => {
                let Some(fields) = self.shapes.iter().find(|shape| shape.name == *name).map(|shape| shape.fields.clone()) else {
                    return Value::Int(0);
                };
                let mut values = Vec::new();
                for (field, field_type) in fields {
                    values.push((field, self.leaf_value(&field_type)));
                }
                Value::Shape { name: name.clone(), fields: values }
            }
            Ty::Enum(_) => Value::Int(0),
        }
    }

    // -- environment ------------------------------------------------------

    /// The four types every part of the predicting mode can handle: they
    /// print the same way everywhere, and they are what a struct field, a
    /// parameter and a loop item are allowed to be.
    fn printable_scalar(&mut self) -> Ty {
        match self.rng.gen_range(0..4) {
            0 => Ty::Int,
            1 => Ty::Float,
            2 => Ty::Text,
            _ => Ty::Bool,
        }
    }

    fn any_type(&mut self) -> Ty {
        if !self.shapes.is_empty() && self.rng.gen_bool(0.2) {
            let index = self.rng.gen_range(0..self.shapes.len());
            return Ty::Struct(self.shapes[index].name.clone());
        }
        if self.rng.gen_bool(0.3) {
            let element = self.printable_scalar();
            return Ty::Array(Box::new(element));
        }
        self.printable_scalar()
    }

    fn environment(&self) -> Vec<(String, Value)> {
        self.scopes.iter().flatten().map(|known| (known.name.clone(), known.value.clone())).collect()
    }

    fn bind(&mut self, name: String, ty: Ty, value: Value) {
        self.scopes.last_mut().expect("there is always a scope").push(Known { name, ty, value });
    }

    fn pick_binding(&mut self, ty: &Ty) -> Option<Known> {
        let matching: Vec<Known> = self.scopes.iter().flatten().filter(|known| known.ty == *ty).cloned().collect();
        if matching.is_empty() {
            return None;
        }
        let index = self.rng.gen_range(0..matching.len());
        Some(matching[index].clone())
    }

    fn fresh(&mut self, stem: &str) -> String {
        self.counter += 1;
        format!("{}_{}", stem, self.counter)
    }

    fn emit(&mut self, text: &str) {
        for line in text.lines() {
            if line.is_empty() {
                self.lines.push(String::new());
            } else {
                self.lines.push(format!("{}{}", "    ".repeat(self.indent), line));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzz::oracle::{examine, Outcome};
    use std::path::PathBuf;

    /// Nothing on disk: a predicted program never imports anything, so where
    /// it would live only matters to the parts of the oracle that resolve
    /// imports.
    fn nowhere() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("fuzz").join("predicting_unit_test.nail")
    }

    #[test]
    fn a_seed_predicts_the_same_program_and_the_same_answer() {
        for seed in [1, 2, 77, 12345] {
            let first = case_with_expected(seed).expect("a seed writes a program");
            let second = case_with_expected(seed).expect("a seed writes a program");
            assert_eq!(first.0, second.0, "seed {} wrote two different programs", seed);
            assert_eq!(first.2, second.2, "seed {} predicted two different answers", seed);
        }
    }

    #[test]
    fn different_seeds_predict_different_programs() {
        let programs: std::collections::HashSet<String> = (1..40).filter_map(|seed| case_with_expected(seed).map(|(source, _, _)| source)).collect();
        assert!(programs.len() > 30, "40 seeds produced only {} distinct programs", programs.len());
    }

    /// The run tier only earns its cost if what it writes reaches rustc. A
    /// predicted program that the checker refuses is a program that never
    /// gets to have its answer compared with anything.
    #[test]
    fn predicted_programs_compile_and_break_no_invariant() {
        let path = nowhere();
        let mut built = 0;
        let mut written = 0;
        let mut findings = Vec::new();
        for seed in 1..60 {
            let Some((source, _, expected)) = case_with_expected(seed) else { continue };
            written += 1;
            assert!(!expected.is_empty(), "seed {} predicted a program that prints nothing", seed);
            assert!(expected.ends_with('\n'), "seed {} predicted output that does not end a line", seed);
            let (finding, outcome) = examine(&source, &path);
            if let Some(finding) = finding {
                findings.push(format!("seed {}: {} in {} ({})", seed, finding.property.name(), finding.stage, finding.detail));
            }
            if matches!(outcome, Outcome::Built(_)) {
                built += 1;
            }
        }
        assert!(findings.is_empty(), "the predicting generator found bugs: {:?}", findings);
        assert!(written > 55, "only {} of 59 seeds produced a predicted program", written);
        assert!(built > 55, "only {} of {} predicted programs made it through the compiler", built, written);
    }

    /// The three renderings the run tier lives or dies by, pinned here so a
    /// change to how print formats a value fails a fast test rather than a
    /// thousand slow ones.
    #[test]
    fn a_value_is_printed_the_way_the_library_prints_it() {
        assert_eq!(printed_line(&Value::Int(7)), "7");
        assert_eq!(printed_line(&Value::Float(3.0)), "3.0");
        assert_eq!(printed_line(&Value::Bool(true)), "true");
        assert_eq!(printed_line(&Value::Text("a b".to_string())), "a b");
        assert_eq!(printed_line(&Value::List(vec![Value::Int(3), Value::Int(7)])), "[3, 7]");
        assert_eq!(printed_line(&Value::List(vec![Value::Float(1.5), Value::Float(2.0)])), "[1.5, 2.0]");
        assert_eq!(printed_line(&Value::List(vec![Value::Text("few".to_string()), Value::Text("some".to_string())])), "[\"few\", \"some\"]");
        assert_eq!(
            printed_line(&Value::Shape { name: "Point".to_string(), fields: vec![("x_pos".to_string(), Value::Int(1)), ("y_pos".to_string(), Value::Text("here".to_string()))] }),
            "Point { x_pos: 1, y_pos: \"here\" }"
        );
    }
}
