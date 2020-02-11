# optarg2chain ![Rust](https://github.com/garkimasera/optarg2chain/workflows/Rust/badge.svg)
Converts optional arguments to chaining style.

Rust doesn't have optional or named arguments. This crate provide macros to convert optional arguments given by attributes to method chaining style instead.

## Example

```Rust
use optarg2chain::optarg_fn;

// specify optarg_fn attribute with builder struct name and terminal method name
#[optarg_fn(JoinStringBuilder, exec)]
fn join_strings(
    mut a: String,                         // Required argument, no default value
    #[optarg_default] b: String,           // String::default() is the default value to b
    #[optarg("ccc".to_owned())] c: String, // "ccc".to_owned() is the default value to c
) -> String {
    a.push_str(&b);
    a.push_str(&c);
    a
}
```

This code is expand to like this:

```Rust
struct JoinStringBuilder {
    a: String,
    b: core::option::Option<String>,
    c: core::option::Option<String>,
    _result_marker: core::marker::PhantomData<fn() -> String>,
}
impl JoinStringBuilder {
    fn b<_OPTARG_VALUE: core::convert::Into<String>>(mut self, value: _OPTARG_VALUE) -> Self {
        let value = <_OPTARG_VALUE as core::convert::Into<String>>::into(value);
        self.b = Some(value);
        self
    }
    fn c<_OPTARG_VALUE: core::convert::Into<String>>(mut self, value: _OPTARG_VALUE) -> Self {
        let value = <_OPTARG_VALUE as core::convert::Into<String>>::into(value);
        self.c = Some(value);
        self
    }
    fn exec(self) -> String {
        fn _optarg_inner_func(mut a: String, b: String, c: String) -> String {
            a.push_str(&b);
            a.push_str(&c);
            a
        }
        let a: String = self.a;
        let b: String = self
            .b
            .unwrap_or_else(|| <String as core::default::Default>::default());
        let c: String = self.c.unwrap_or_else(|| "ccc".to_owned());
        _optarg_inner_func(a, b, c)
    }
}
fn join_strings(a: String) -> JoinStringBuilder {
    JoinStringBuilder {
        a,
        b: core::option::Option::None,
        c: core::option::Option::None,
        _result_marker: core::marker::PhantomData,
    }
}
```

`optarg_fn` generates builder struct, optional argument setter and terminal methods. You can use above `join_strings` as below:

```Rust
assert_eq!(join_strings("aaa".to_owned()).exec(), "aaaccc");
assert_eq!(
    join_strings("xxx".to_owned())
        .b("yyy".to_owned())
        .c("zzz".to_owned())
        .exec(),
    "xxxyyyzzz"
);
```

`optarg_impl` and `optarg_method` attributes are prepared for methods.

```Rust
use optarg2chain::optarg_impl;

#[derive(Clone)]
struct MyVec<T> {
    data: Vec<T>,
}

#[optarg_impl]
impl<T: Default + Copy> MyVec<T> {
    #[optarg_method(MyVecGetOr, get)]
    fn get_or<'a>(&'a self, i: usize, #[optarg_default] other: T) -> T { // Lifetimes need to be given explicitly
        self.data.get(i).copied().unwrap_or(other)
    }
}
```

You can use this as below:

```Rust
let myvec = MyVec { data: vec![2, 4, 6] };
assert_eq!(myvec.get_or(1).get(), 4);
assert_eq!(myvec.get_or(10).get(), 0);
assert_eq!(myvec.get_or(10).other(42).get(), 42);
```

## Limitations

### References in argument types need to be given explicitly

Correct:

```Rust
#[optarg_impl]
impl Foo {
    #[optarg_method(DoSomething, exec)]
    fn do_something<'a, 'b>(&'a self, s: &'b str, ...) { ... }
}
```

Incorrect:

```Rust
#[optarg_impl]
impl Foo {
    #[optarg_method(DoSomething, exec)]
    fn do_something(&self, s: &str, ...) { ... }
}
```

### impl Trait is not supported

Explicit type generics is a replacement of impl Trait in argument position.

Correct:

```Rust
#[optarg_fn(PrintWith, exec)]
fn print_with<'b, T: std::fmt::Display>(a: T, #[optarg_default] b: &'b str) {
    println!("{}\n{}", b, a);
}
```

Incorrect:

```Rust
#[optarg_fn(PrintWith, exec)]
fn print_with<'b>(a: impl std::fmt::Display, #[optarg_default] b: &'b str) {
    println!("{}\n{}", b, a);
}
```

### Argument pattern

Patterns like `(a, b): (i32, i8)` or `Foo { x }: Foo` in argument position are not allowd.

## License

MIT
