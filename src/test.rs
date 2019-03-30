#[cfg(test)]
use crate::compiler::compiler::Compiler;
#[cfg(test)]
use crate::compiler::compiler::CompilerInput;
#[cfg(test)]
use crate::error::Error;
#[cfg(test)]
use crate::parser::error::LexerError;

#[cfg(test)]
fn create_source(name: &str, source: &str) -> CompilerInput {
    CompilerInput::Memory {
        name: name.to_string(),
        content: source.to_string(),
    }
}

#[cfg(test)]
fn single(source: &str) -> Vec<CompilerInput> {
    vec![create_source("source", source)]
}

#[cfg(test)]
fn ok(inputs: Vec<CompilerInput>) {
    let mut compiler = Compiler::new(false);
    assert!(compiler.compile(inputs).is_ok());
}

#[cfg(test)]
fn compile_err(source: &str) -> Error {
    let mut compiler = Compiler::new(false);
    compiler
        .compile(single(source))
        .err()
        .expect("Error not found")
}

#[test]
fn minimal_success() {
    let source = "
module Main where
main = ()
";
    ok(single(source));
}

#[test]
fn invalid_identifier_double_dot() {
    let source = "module Da..ta";
    if let LexerError::InvalidIdentifier(id, _) = compile_err(source).get_single_lexer() {
        assert_eq!(id, "Da..ta");
    } else {
        unreachable!()
    }
}

#[test]
fn invalid_identifier_ends_with_dot() {
    let source = "module Data. ";
    if let LexerError::InvalidIdentifier(id, _) = compile_err(source).get_single_lexer() {
        assert_eq!(id, "Data.");
    } else {
        unreachable!()
    }
}

#[test]
fn invalid_identifier_stars_with_number() {
    let source = "module 9Data ";
    if let LexerError::InvalidIdentifier(id, _) = compile_err(source).get_single_lexer() {
        assert_eq!(id, "9Data");
    } else {
        unreachable!()
    }
}
