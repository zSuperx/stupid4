fn main() : u64 {
  let x: u64 = 5;
  let y = &x + 1;
  *y = 69;
  return x;
}
