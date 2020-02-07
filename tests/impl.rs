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
