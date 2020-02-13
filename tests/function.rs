use optarg2chain::*;

#[optarg_fn(JoinStringBuilder, exec)]
fn join_strings(
    mut a: String,
    #[optarg_default] b: String,
    #[optarg("ccc".to_owned())] c: String,
) -> String {
    a.push_str(&b);
    a.push_str(&c);
    a
}

#[test]
fn join_strings_test() {
    assert_eq!(join_strings("aaa".to_owned()).exec(), "aaaccc");
    assert_eq!(
        join_strings("xxx".to_owned())
            .b("yyy".to_owned())
            .c("zzz".to_owned())
            .exec(),
        "xxxyyyzzz"
    );
}

#[optarg_fn(JoinVecBuilder, exec)]
fn join_vecs<T>(
    mut a: Vec<T>,
    #[optarg_default] mut b: Vec<T>,
    #[optarg(vec![T::default()])] mut c: Vec<T>,
) -> Vec<T>
where
    T: Default,
{
    a.append(&mut b);
    a.append(&mut c);
    a
}

#[test]
fn join_vec_test() {
    assert_eq!(join_vecs(vec![3, 2, 1]).exec(), [3, 2, 1, 0]);
    assert_eq!(
        join_vecs(vec![2, 3, 5])
            .b(vec![7, 9])
            .c(vec![11, 13])
            .exec(),
        [2, 3, 5, 7, 9, 11, 13]
    );
}

#[optarg_fn(ConvertBuilder, exec)]
fn add_and_convert<T: Default, R>(a: T, #[optarg_default] b: T) -> R
where
    T: core::ops::Add<Output = T>,
    R: From<T>,
{
    (a + b).into()
}

#[test]
fn add_and_convert_test() {
    assert_eq!(add_and_convert::<i8, i32>(1).b(2).exec(), 3i32);
    assert_eq!(add_and_convert::<u8, u32>(42).exec(), 42u32);
}

#[optarg_fn(EmptyArg, exec)]
fn empty_arg() -> i32 {
    42
}

#[test]
fn empty_arg_test() {
    assert_eq!(empty_arg().exec(), 42);
}

#[optarg_fn(Convert, get)]
fn convert<T: Into<U> + Default, U>(#[optarg_default] target: T) -> U {
    target.into()
}

#[test]
fn convert_test() {
    assert_eq!(convert::<i8, i32>().get(), 0i32);
    assert_eq!(convert::<i8, i32>().target(42i8).get(), 42i32);
}

#[optarg_fn(IterImplTrait, iter)]
fn iter_impl_trait<T: Default>(
    #[optarg_default] a: T,
    #[optarg_default] b: T,
    #[optarg_default] c: T,
) -> impl Iterator<Item = T> {
    vec![a, b, c].into_iter()
}

#[test]
fn convert_impl_trait_test() {
    let iter = iter_impl_trait::<i32>().iter();
    assert_eq!(iter.collect::<Vec<i32>>(), vec![0, 0, 0]);
    let iter = iter_impl_trait::<i32>().a(1).b(2).c(3).iter();
    assert_eq!(iter.collect::<Vec<i32>>(), vec![1, 2, 3]);
}

#[optarg_fn(Async, exec)]
async fn async_fn<'a>(#[optarg("foo")] a: &'a str) -> &'a str {
    a
}

#[test]
fn async_test() {
    use futures::executor::block_on;
    assert_eq!(block_on(async_fn().exec()), "foo");
    assert_eq!(block_on(async_fn().a("bar").exec()), "bar");
}
