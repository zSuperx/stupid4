fn no_frame(): i32 {
  return 69;
}

fn yes_frame(): i32 {
  let x = 69;
  let y = &x;
  return *y;
}
