#[cfg(test)]
use crate::compiler::compiler::Compiler;
#[cfg(test)]
use crate::compiler::compiler::CompilerInput;

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

#[test]
fn minimal_success() {
    let source = "
module Main where
main = ()
";
    ok(single(source));
}
