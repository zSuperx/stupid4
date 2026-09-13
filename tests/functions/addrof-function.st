fn foo() {}

fn bar() {}

fn main() {
  let x = &foo;
  let y = &x;
  *y = bar;
}
