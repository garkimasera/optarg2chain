# optarg2chain
Converts optional arguments to chaining style

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
}
impl JoinStringBuilder {
    fn b(mut self, value: String) -> Self {
        self.b = Some(value);
        self
    }
    fn c(mut self, value: String) -> Self {
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
    }
}
```

`optarg_fn` generates builder struct, optional argument setter and terminal methods. You can use above `join_strings` like this:

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

## License

MIT
