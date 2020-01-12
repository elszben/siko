#[derive(Clone)]
struct Foo {}

#[derive(Clone)]
struct Bar {}

#[derive(Clone)]
struct Boo {}

#[derive(Clone)]
struct Data {
    foo: Option<Foo>,
    bar: Option<Bar>,
}

impl Function<Foo, Box<dyn Function<Bar, Boo>>> for Data {
    fn call(&self, foo: Foo) -> Box<dyn Function<Bar, Boo>> {
        let mut clone = self.clone();
        clone.foo = Some(foo);
        Box::new(clone)
    }
}

impl Function<Bar, Boo> for Data {
    fn call(&self, bar: Bar) -> Boo {
        foo_fn(self.foo.as_ref().expect("empty Foo").clone(), bar)
    }
}

trait Function<A, B> {
    fn call(&self, a: A) -> B;
}

fn foo_fn(foo: Foo, bar: Bar) -> Boo {
    println!("Ha!");
    Boo {}
}

fn func1() {
    let dyn_fn = Data {
        foo: None,
        bar: None,
    };
    let dyn_fn = Box::new(dyn_fn);
    func2(dyn_fn);
}

fn func2(dyn_fn: Box<dyn Function<Foo, Box<dyn Function<Bar, Boo>>>>) {
    let foo = Foo {};
    let dyn_fn = dyn_fn.call(foo);
    func3(dyn_fn)
}

fn func3(dyn_fn: Box<dyn Function<Bar, Boo>>) {
    let bar = Bar {};
    dyn_fn.call(bar);
}

fn main() {
    println!("Dynamic fn experiment");
    func1();
}
