fn foo(): i32 {
  return 69;
}

fn bar(): i32 {
  return 5;
}

fn main(): i32 {
  let x = foo;

  if true {
    x = bar;
  }

  return x();
}
