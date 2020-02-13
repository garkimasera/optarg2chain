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

    #[optarg_method(IntegerNew, build)]
    fn new() -> Integer {
        Integer(42)
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
    assert_eq!(Integer::new().build(), Integer(42));
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

    #[optarg_method(WrapConvertBuilder, exec)]
    fn convert<U: From<T>>(self) -> Wrap<U> {
        Wrap(self.0.into())
    }
}

#[test]
fn wrap_test() {
    let wrapped_int = Wrap(5i32);
    assert_eq!(wrapped_int.add().exec(), 5);
    assert_eq!(wrapped_int.add().a(4).exec(), 9);
    assert_eq!(wrapped_int.convert::<i64>().exec().0, 5i64);
}

#[derive(Clone)]
struct MyVec<T> {
    data: Vec<T>,
}

#[optarg_impl]
impl<T: Default + Copy> MyVec<T> {
    #[optarg_method(MyVecGetOr, get)]
    fn get_or<'a>(&'a self, i: usize, #[optarg_default] other: T) -> T {
        self.data.get(i).copied().unwrap_or(other)
    }
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
    assert_eq!(myvec.get_or(1).get(), 4);
    assert_eq!(myvec.get_or(10).get(), 0);
    assert_eq!(myvec.get_or(10).other(42).get(), 42);
    assert_eq!(myvec.clone_or().get().data, [2, 4, 6]);
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
    fn take_box(self: Box<Self>) -> Box<Self> {
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

struct AsyncTest;

#[optarg_impl]
impl AsyncTest {
    #[optarg_method(AsyncFn, exec)]
    async fn async_fn<'a>(&'a self, #[optarg(3)] a: i32) -> i32 {
        a
    }
}

#[test]
fn async_test() {
    use futures::executor::block_on;
    let a = AsyncTest;
    assert_eq!(block_on(a.async_fn().exec()), 3);
    assert_eq!(block_on(a.async_fn().a(6).exec()), 6);
}
