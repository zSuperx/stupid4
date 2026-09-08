fn main(): u8 {
  let base: u64 = 65536;
  let index = 1;
  let array = @(*u8)&base;
  return array[index];
}
