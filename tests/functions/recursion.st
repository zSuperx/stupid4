fn foo(count: i32) {
  if count > 0 {
    foo(count - 1);
  }
}

fn main() {
  foo(10);
}
