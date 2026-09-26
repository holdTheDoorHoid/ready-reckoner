//! A parser for the subset of TypeScript that `web/src/engine/types.ts` uses, and a checker that
//! walks a JSON value produced by the Rust types against the TypeScript declarations.
//!
//! Supported declarations: `export const X = [...] as const;`, `export const X = <number>;`,
//! `export type X = <type>;`, `export interface X { field: type; other?: type; }` (optionally
//! generic). Supported types: `string`, `number`, `boolean`, `unknown`, string literals, `true`,
//! `false`, named types (with type arguments ignored), `X[]`, `[A, B]`, unions `A | B`,
//! `(typeof X)[number]`, and single-letter type parameters.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Str,
    Num,
    Bool,
    Unknown,
    StrLit(String),
    BoolLit(bool),
    Named(String),
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    Union(Vec<Ty>),
    Param,
    ConstUnion(String),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub optional: bool,
    pub ty: Ty,
}

#[derive(Debug, Default)]
pub struct TsFile {
    /// `export const X = [...] as const;` values (strings or numbers).
    pub arrays: BTreeMap<String, Vec<Value>>,
    /// `export const X = <number>;`
    pub numbers: BTreeMap<String, f64>,
    /// `export type X = ...;` right-hand sides, unparsed (parsed on use).
    pub aliases: BTreeMap<String, String>,
    /// `export interface X { ... }`
    pub interfaces: BTreeMap<String, Vec<Field>>,
}

/// Removes `/* */` and `//` comments, leaving string literals alone.
fn strip_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let mut quote: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            let mut j = i + 2;
            while j + 1 < chars.len() && !(chars[j] == '*' && chars[j + 1] == '/') {
                j += 1;
            }
            i = j + 2;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else {
            if c == '\'' || c == '"' || c == '`' {
                quote = Some(c);
            }
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Splits `s` at `sep` where it is not nested inside brackets, braces, parentheses or angles.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for c in s.chars() {
        if let Some(q) = quote {
            current.push(c);
            if c == q {
                quote = None;
            }
            prev = c;
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            // `=>` is an arrow, not a closing angle.
            '>' if prev != '=' => depth -= 1,
            _ => {}
        }
        if c == sep && depth == 0 {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
        prev = c;
    }
    parts.push(current);
    parts
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parses a TypeScript type expression.
pub fn parse_type(src: &str) -> Ty {
    let s = src.trim();
    let s = s.strip_prefix('|').map(str::trim).unwrap_or(s);
    let parts = split_top(s, '|');
    if parts.len() > 1 {
        return Ty::Union(parts.iter().map(|p| parse_type(p)).collect());
    }
    match s {
        "string" => return Ty::Str,
        "number" => return Ty::Num,
        "boolean" => return Ty::Bool,
        "unknown" => return Ty::Unknown,
        "true" => return Ty::BoolLit(true),
        "false" => return Ty::BoolLit(false),
        _ => {}
    }
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        return Ty::StrLit(s[1..s.len() - 1].to_owned());
    }
    if let Some(rest) = s.strip_prefix("(typeof ") {
        if let Some(name) = rest.strip_suffix(")[number]") {
            return Ty::ConstUnion(name.trim().to_owned());
        }
    }
    if let Some(inner) = s.strip_suffix("[]") {
        return Ty::Array(Box::new(parse_type(inner)));
    }
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        return Ty::Tuple(
            split_top(inner, ',')
                .iter()
                .filter(|p| !p.trim().is_empty())
                .map(|p| parse_type(p))
                .collect(),
        );
    }
    if s.starts_with('(') && s.ends_with(')') {
        return parse_type(&s[1..s.len() - 1]);
    }
    if let Some(open) = s.find('<') {
        if s.ends_with('>') && is_ident(&s[..open]) {
            return Ty::Named(s[..open].to_owned());
        }
    }
    if s.len() == 1 && s.chars().all(|c| c.is_ascii_uppercase()) {
        return Ty::Param;
    }
    assert!(
        is_ident(s),
        "types.ts uses a type the test parser does not understand: {s:?}"
    );
    Ty::Named(s.to_owned())
}

/// Reads the name after a keyword, stopping at a generic list, `=`, `{` or whitespace.
fn decl_name(s: &str) -> (&str, &str) {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// Returns the text up to the matching close of the bracket that `s` starts with.
fn take_balanced(s: &str, open: char, close: char) -> (&str, &str) {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return (&s[1..i], &s[i + 1..]);
            }
        }
    }
    panic!("unbalanced {open}{close} in types.ts");
}

/// Parses `web/src/engine/types.ts`.
pub fn parse_file(src: &str) -> TsFile {
    let text = strip_comments(src);
    let mut file = TsFile::default();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("export ") {
        rest = &rest[at + "export ".len()..];
        if let Some(r) = rest.strip_prefix("const ") {
            let (name, r) = decl_name(r);
            let r = r
                .trim_start()
                .strip_prefix('=')
                .expect("const =")
                .trim_start();
            if r.starts_with('[') {
                let (body, after) = take_balanced(r, '[', ']');
                assert!(
                    after.trim_start().starts_with("as const"),
                    "{name} must be `as const`"
                );
                let values = split_top(body, ',')
                    .iter()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| {
                        if let Some(s) = v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
                            Value::String(s.to_owned())
                        } else {
                            let n: f64 = v
                                .parse()
                                .unwrap_or_else(|_| panic!("{name}: bad value {v}"));
                            serde_json::json!(n)
                        }
                    })
                    .collect();
                file.arrays.insert(name.to_owned(), values);
                rest = after;
            } else {
                let end = r.find(';').expect("const ;");
                let n: f64 = r[..end]
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("{name} is not a number"));
                file.numbers.insert(name.to_owned(), n);
                rest = &r[end..];
            }
        } else if let Some(r) = rest.strip_prefix("type ") {
            let (name, r) = decl_name(r);
            let eq = r.find('=').expect("type =");
            let rhs = &r[eq + 1..];
            let end = split_top(rhs, ';')[0].len();
            file.aliases
                .insert(name.to_owned(), rhs[..end].trim().to_owned());
            rest = &rhs[end..];
        } else if let Some(r) = rest.strip_prefix("interface ") {
            let (name, r) = decl_name(r);
            let brace = r.find('{').expect("interface {");
            let (body, after) = take_balanced(&r[brace..], '{', '}');
            let fields = split_top(body, ';')
                .iter()
                .map(|f| f.trim())
                .filter(|f| !f.is_empty())
                .map(|f| {
                    let colon = f
                        .find(':')
                        .unwrap_or_else(|| panic!("{name}: field without type: {f}"));
                    let (key, ty) = (f[..colon].trim(), &f[colon + 1..]);
                    let (key, optional) = match key.strip_suffix('?') {
                        Some(k) => (k.trim(), true),
                        None => (key, false),
                    };
                    Field {
                        name: key.to_owned(),
                        optional,
                        ty: parse_type(ty),
                    }
                })
                .collect();
            file.interfaces.insert(name.to_owned(), fields);
            rest = after;
        }
    }
    file
}

/// Walks JSON values produced by the Rust types against the TypeScript declarations, recording
/// which interfaces and fields the samples exercised and what the Rust side accepts as optional.
pub struct Checker<'a> {
    pub ts: &'a TsFile,
    pub visited: BTreeSet<String>,
    pub seen: BTreeMap<String, BTreeSet<String>>,
    pub rust_optional: BTreeMap<(String, String), bool>,
    pub errors: Vec<String>,
}

impl<'a> Checker<'a> {
    pub fn new(ts: &'a TsFile) -> Self {
        Self {
            ts,
            visited: BTreeSet::new(),
            seen: BTreeMap::new(),
            rust_optional: BTreeMap::new(),
            errors: Vec::new(),
        }
    }

    /// Checks `root`, a serialised Rust value, against the TypeScript type named `ts_type`.
    /// `parses` says whether an edited copy of `root` still deserialises into the Rust type; it
    /// decides which fields Rust treats as optional.
    pub fn check_root(&mut self, ts_type: &str, root: &Value, parses: &dyn Fn(&Value) -> bool) {
        let ty = Ty::Named(ts_type.to_owned());
        self.check(&ty, root, "", root, parses);
    }

    fn resolve(&self, name: &str) -> Option<Ty> {
        self.ts.aliases.get(name).map(|rhs| parse_type(rhs))
    }

    /// Structural match with no side effects, used to pick a union member.
    fn matches(&self, ty: &Ty, v: &Value) -> bool {
        match ty {
            Ty::Str => v.is_string(),
            Ty::Num => v.is_number(),
            Ty::Bool => v.is_boolean(),
            Ty::Unknown | Ty::Param => true,
            Ty::StrLit(s) => v.as_str() == Some(s),
            Ty::BoolLit(b) => v.as_bool() == Some(*b),
            Ty::ConstUnion(name) => self
                .ts
                .arrays
                .get(name)
                .is_some_and(|vals| vals.contains(v)),
            Ty::Array(inner) => v
                .as_array()
                .is_some_and(|a| a.iter().all(|e| self.matches(inner, e))),
            Ty::Tuple(items) => v.as_array().is_some_and(|a| {
                a.len() == items.len() && a.iter().zip(items).all(|(e, t)| self.matches(t, e))
            }),
            Ty::Union(members) => members.iter().any(|m| self.matches(m, v)),
            Ty::Named(name) => {
                if let Some(fields) = self.ts.interfaces.get(name) {
                    let Some(obj) = v.as_object() else {
                        return false;
                    };
                    obj.keys().all(|k| fields.iter().any(|f| &f.name == k))
                        && fields.iter().all(|f| match obj.get(&f.name) {
                            Some(fv) => self.matches(&f.ty, fv),
                            None => f.optional,
                        })
                } else if let Some(t) = self.resolve(name) {
                    self.matches(&t, v)
                } else {
                    false
                }
            }
        }
    }

    fn check(
        &mut self,
        ty: &Ty,
        v: &Value,
        ptr: &str,
        root: &Value,
        parses: &dyn Fn(&Value) -> bool,
    ) {
        let ok = match ty {
            Ty::Union(members) => match members.iter().find(|m| self.matches(m, v)) {
                Some(m) => {
                    let m = m.clone();
                    self.check(&m, v, ptr, root, parses);
                    true
                }
                None => false,
            },
            Ty::Array(inner) => match v.as_array() {
                Some(items) => {
                    for (i, e) in items.iter().enumerate() {
                        self.check(inner, e, &format!("{ptr}/{i}"), root, parses);
                    }
                    true
                }
                None => false,
            },
            Ty::Tuple(items) => match v.as_array() {
                Some(a) if a.len() == items.len() => {
                    for (i, (e, t)) in a.iter().zip(items).enumerate() {
                        self.check(t, e, &format!("{ptr}/{i}"), root, parses);
                    }
                    true
                }
                _ => false,
            },
            Ty::Named(name) => {
                if self.ts.interfaces.contains_key(name) {
                    self.check_interface(name, v, ptr, root, parses);
                    true
                } else if let Some(t) = self.resolve(name) {
                    self.check(&t, v, ptr, root, parses);
                    true
                } else {
                    self.errors
                        .push(format!("{ptr}: types.ts has no type named {name}"));
                    true
                }
            }
            other => self.matches(other, v),
        };
        if !ok {
            self.errors.push(format!(
                "{ptr}: Rust produced {v} where types.ts expects {ty:?}"
            ));
        }
    }

    fn check_interface(
        &mut self,
        name: &str,
        v: &Value,
        ptr: &str,
        root: &Value,
        parses: &dyn Fn(&Value) -> bool,
    ) {
        self.visited.insert(name.to_owned());
        let fields = self.ts.interfaces[name].clone();
        let Some(obj) = v.as_object() else {
            self.errors.push(format!(
                "{ptr}: {name} must be an object, Rust produced {v}"
            ));
            return;
        };
        for key in obj.keys() {
            if !fields.iter().any(|f| &f.name == key) {
                self.errors.push(format!(
                    "{ptr}: Rust emits `{key}`, which interface {name} lacks"
                ));
            }
        }
        for f in &fields {
            match obj.get(&f.name) {
                None if !f.optional => self.errors.push(format!(
                    "{ptr}: interface {name} requires `{}`, which Rust did not emit",
                    f.name
                )),
                None => {}
                Some(fv) => {
                    self.seen
                        .entry(name.to_owned())
                        .or_default()
                        .insert(f.name.clone());
                    // Does the Rust type still parse with this field removed?
                    let mut edited = root.clone();
                    let target = if ptr.is_empty() {
                        Some(&mut edited)
                    } else {
                        edited.pointer_mut(ptr)
                    };
                    let optional_in_rust = match target.and_then(Value::as_object_mut) {
                        Some(o) => {
                            o.remove(&f.name);
                            parses(&edited)
                        }
                        None => {
                            self.errors
                                .push(format!("{ptr}: cannot follow the pointer"));
                            false
                        }
                    };
                    let key = (name.to_owned(), f.name.clone());
                    if let Some(prev) = self.rust_optional.insert(key, optional_in_rust) {
                        if prev != optional_in_rust {
                            self.errors.push(format!(
                                "{name}.{}: Rust optionality differs between samples",
                                f.name
                            ));
                        }
                    }
                    if optional_in_rust != f.optional {
                        self.errors.push(format!(
                            "{name}.{}: Rust {} it but types.ts marks it {}",
                            f.name,
                            if optional_in_rust {
                                "accepts JSON without"
                            } else {
                                "requires"
                            },
                            if f.optional {
                                "optional (`?`)"
                            } else {
                                "required"
                            },
                        ));
                    }
                    let fty = f.ty.clone();
                    self.check(&fty, fv, &format!("{ptr}/{}", f.name), root, parses);
                }
            }
        }
    }
}
