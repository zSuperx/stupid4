fn foo(): i32 {
  return 69;
}

fn main(): i32 {
  let x = foo;
  return x();
}
