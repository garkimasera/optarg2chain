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