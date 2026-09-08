fn main() : i32 {
  let input: i32 = 8;
  let fallback: i32 = 20;
  let base: i32 = 5;
  let multiplier: i32 = 3;
  let unused: i32 = 999;

  let score = input + base;
  let scaled = score * multiplier;

  if input > base {
    let bonus = scaled + 10;
    let unused_branch_value = 123;

    if bonus > 50 {
      let result = bonus + input;

      if result > 100 {
        return result;
      } else {
        return result + 1;
      }
    } else {
      let penalty = base * 2;
      return bonus - penalty;
    }
  } else {
    let fallback_score = fallback + base;

    if fallback_score > 30 {
      let reduced = fallback_score - input;
      return reduced;
    } else {
      let replacement = fallback_score * 2;
      return replacement;
    }
  }
}
