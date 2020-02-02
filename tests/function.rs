use optarg2chain::*;

#[optarg_func(JoinStringBuilder, exec)]
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

#[optarg_func(JoinVecBuilder, exec)]
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

#[optarg_func(ConvertBuilder, exec)]
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
