fn side_effect(x: i32): i32 {
  return x;
}

fn full(): i32 {
  let dead_a = 10;
  let dead_b = dead_a + 20;
  let dead_c = dead_b * 3;

  let input = 7;
  let observed = side_effect(input);

  let dead_d = observed + 100;
  let dead_e = dead_d * dead_c;

  let result = observed + 1;

  if result > 5 {
    let branch_dead_a = 40;
    let branch_dead_b = branch_dead_a + 2;

    let branch_live = result * 2;
    side_effect(branch_live);

    let branch_dead_c = branch_live + branch_dead_b;
  } else {
    let other_dead_a = 50;
    let other_dead_b = other_dead_a + 3;

    side_effect(result);

    let other_dead_c = other_dead_b * 4;
  }

  let after_branch_dead = result + 999;
  let after_branch_dead_2 = after_branch_dead * 2;

  return result;

  let unreachable_a = 123;
  let unreachable_b = unreachable_a + 1;
}

fn minimal(): i32 {
  let input = 7;
  let observed = side_effect(input);
  let result = observed + 1;

  if result > 5 {
    let branch_live = result * 2;
    side_effect(branch_live);
  } else {
    side_effect(result);
  }

  return result;
}
