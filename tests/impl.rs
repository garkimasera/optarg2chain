use optarg2chain::*;

#[derive(PartialEq, Debug)]
struct Integer(i32);

#[optarg_impl]
impl Integer {
    #[optarg_method(AddBuilder, exec)]
    fn add<'a>(&'a self, #[optarg(20)] a: i32) -> i32 {
        self.0 + a
    }

    #[optarg_method(AssignBuilder, exec)]
    fn assign<'a>(&'a mut self, #[optarg(0)] a: i32) {
        self.0 = a;
    }

    #[optarg_method(TakeOwnerShipBuilder, exec)]
    fn add_and_take(mut self, #[optarg(0)] a: i32) -> Integer {
        self.0 += a;
        self
    }
}

#[test]
fn integer_test() {
    let integer = Integer(22);
    assert_eq!(integer.add().a(11).exec(), 33);
    assert_eq!(integer.add().exec(), 42);
    let mut integer = Integer(10);
    integer.assign().exec();
    assert_eq!(integer.0, 0);
    integer.assign().a(100).exec();
    assert_eq!(integer.0, 100);
    assert_eq!(integer.add_and_take().a(27).exec(), Integer(127));
}

#[derive(PartialEq, Debug)]
struct Wrap<T>(T);

#[optarg_impl]
impl<T: core::ops::Add<Output = T> + Default> Wrap<T> {
    #[optarg_method(WrapAddBuilder, exec)]
    fn add<'a>(&'a self, #[optarg_default] a: T) -> T
    where
        T: Copy,
    {
        self.0 + a
    }
}

#[test]
fn wrap_test() {
    let wrapped_int = Wrap(5i32);
    assert_eq!(wrapped_int.add().exec(), 5);
    assert_eq!(wrapped_int.add().a(4).exec(), 9);
}

#[derive(Clone)]
struct MyVec<T> {
    data: Vec<T>,
}

#[optarg_impl]
impl<T: Clone> MyVec<T> {
    #[optarg_method(CloneOr, get)]
    fn clone_or<'a>(&'a self, #[optarg_default] other: Option<Self>) -> Self {
        other.unwrap_or(self.clone())
    }
}

#[test]
fn myvec_test() {
    let myvec = MyVec {
        data: vec![2, 4, 6],
    };
    assert_eq!(myvec.clone_or().get().data, [2, 4, 6]);
    assert_eq!(
        myvec
            .clone_or()
            .other(MyVec {
                data: vec![1, 3, 5]
            })
            .get()
            .data,
        [1, 3, 5]
    );
}

#[derive(PartialEq, Debug)]
struct TwoStr<'a, 'b> {
    a: &'a str,
    b: &'b str,
}

#[optarg_impl]
impl<'a, 'b> TwoStr<'a, 'b> {
    #[optarg_method(TwoStrNew, build)]
    fn new(a: &'a str, #[optarg("")] b: &'b str) -> TwoStr<'a, 'b> {
        TwoStr { a, b }
    }

    #[optarg_method(TwoStrNewStatic, build)]
    fn new_static(a: &'static str, #[optarg("")] b: &'b str) -> TwoStr<'static, 'b> {
        TwoStr { a, b }
    }

    #[optarg_method(TwoStrReplace, exec)]
    fn replace<'s, 'c>(&'s self, #[optarg("ccc")] b: &'c str) -> TwoStr<'a, 'c> {
        TwoStr { a: self.a, b }
    }

    #[optarg_method(TakeBox, exec)]
    fn take_box(self: Box<Self>, #[optarg(())] _dummy: ()) -> Box<Self> {
        self
    }
}

#[test]
fn twostr_test() {
    assert_eq!(TwoStr::new("x").b("y").build(), TwoStr { a: "x", b: "y" });
    assert_eq!(TwoStr::new("x").build(), TwoStr { a: "x", b: "" });
    assert_eq!(
        TwoStr::new_static("x").b("y").build(),
        TwoStr { a: "x", b: "y" }
    );
    assert_eq!(TwoStr::new_static("x").build(), TwoStr { a: "x", b: "" });
    let two_str = TwoStr { a: "aaa", b: "bbb" };
    assert_eq!(
        two_str.replace().b("yyy").exec(),
        TwoStr { a: "aaa", b: "yyy" }
    );
    assert_eq!(two_str.replace().exec(), TwoStr { a: "aaa", b: "ccc" });
    assert_eq!(
        Box::new(two_str).take_box().exec(),
        Box::new(TwoStr { a: "aaa", b: "bbb" })
    );
}
