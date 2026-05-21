/* 0041770d FUN_0041770d */

void __fastcall FUN_0041770d(int *param_1)

{
  int local_8;
  
  if (*param_1 == 10) {
    param_1[5] = 1;
  }
  else if (*param_1 == 0xb) {
    param_1[5] = 0;
  }
  else {
    param_1[5] = -3;
    for (local_8 = 0; local_8 < 4; local_8 = local_8 + 1) {
      param_1[local_8 + 0xb] = -3;
    }
  }
  return;
}
