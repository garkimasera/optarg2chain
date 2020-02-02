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
