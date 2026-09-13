fn add_one(x: *i32) {
  *x = *x + 1;
}

fn main() {
  let x = 5;
  add_one(&x);
}
