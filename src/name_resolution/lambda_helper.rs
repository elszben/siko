use crate::name_resolution::environment::NamedRef;
use crate::util::Counter;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct LambdaHelper {
    captures: Vec<NamedRef>,
    level: usize,
    host_function: String,
    counter: Rc<RefCell<Counter>>,
}

impl LambdaHelper {
    pub fn new(level: usize, host_function: String, counter: Rc<RefCell<Counter>>) -> LambdaHelper {
        LambdaHelper {
            captures: Vec::new(),
            level: level,
            host_function: host_function,
            counter: counter,
        }
    }

    pub fn process_named_ref(&mut self, r: NamedRef, level: usize) -> NamedRef {
        if level < self.level {
            let arg_index = self.captures.len();
            let updated_ref = match &r {
                NamedRef::ExprValue(id) => NamedRef::LambdaCapturedExprValue(*id, arg_index),
                NamedRef::FunctionArg(arg_ref) => {
                    NamedRef::LambdaCapturedFunctionArg(arg_ref.clone(), arg_index)
                }
                _ => panic!("Unexpected name ref {:?}", r),
            };
            println!("Captured variable {:?}", updated_ref);
            self.captures.push(r);
            updated_ref
        } else {
            r
        }
    }

    pub fn captures(&self) -> Vec<NamedRef> {
        self.captures.clone()
    }

    pub fn host_function(&self) -> String {
        self.host_function.clone()
    }

    pub fn get_lambda_index(&self) -> usize {
        let index = self.counter.borrow_mut().next();
        index
    }

    pub fn new_counter() -> Rc<RefCell<Counter>> {
        Rc::new(RefCell::new(Counter::new()))
    }

    pub fn clone_counter(&self) -> Rc<RefCell<Counter>> {
        self.counter.clone()
    }
}
