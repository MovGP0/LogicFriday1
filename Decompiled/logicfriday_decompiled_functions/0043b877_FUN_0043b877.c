/* 0043b877 FUN_0043b877 */

undefined4 __fastcall FUN_0043b877(int param_1)

{
  undefined4 local_14;
  undefined4 local_10;
  undefined4 local_c;
  undefined4 local_8;
  
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x30); local_8 = local_8 + 1) {
    for (local_14 = 0; local_14 < *(int *)(param_1 + 0x28); local_14 = local_14 + 1) {
      if (*(int *)(*(int *)(param_1 + 0x34) + 8 + local_8 * 0x14) ==
          *(int *)(*(int *)(param_1 + 0x2c) + 0x10 + local_14 * 0x14)) {
        if (*(int *)(*(int *)(param_1 + 0x2c) + 0xc + local_14 * 0x14) == 0) {
          *(undefined4 *)(local_8 * 0x14 + *(int *)(param_1 + 0x34)) =
               *(undefined4 *)(local_14 * 0x14 + *(int *)(param_1 + 0x2c));
          if (*(int *)(*(int *)(param_1 + 0x2c) + 4 + (local_14 + 1) * 0x14) <
              *(int *)(*(int *)(param_1 + 0x2c) + 4 + local_14 * 0x14)) {
            local_c = local_14 + 1;
            local_10 = local_14;
          }
          else {
            local_c = local_14;
            local_10 = local_14 + 1;
          }
          if (*(int *)(*(int *)(param_1 + 0x2c) + 4 + local_10 * 0x14) <
              *(int *)(*(int *)(param_1 + 0x34) + 4 + local_8 * 0x14)) {
            *(undefined4 *)(*(int *)(param_1 + 0x34) + 4 + local_8 * 0x14) =
                 *(undefined4 *)(*(int *)(param_1 + 0x2c) + 4 + local_10 * 0x14);
          }
          if (*(int *)(*(int *)(param_1 + 0x34) + 4 + local_8 * 0x14) <
              *(int *)(*(int *)(param_1 + 0x2c) + 4 + local_c * 0x14)) {
            *(undefined4 *)(*(int *)(param_1 + 0x34) + 4 + local_8 * 0x14) =
                 *(undefined4 *)(*(int *)(param_1 + 0x2c) + 4 + local_c * 0x14);
          }
        }
        else {
          *(undefined4 *)(*(int *)(param_1 + 0x34) + 4 + local_8 * 0x14) =
               *(undefined4 *)(*(int *)(param_1 + 0x2c) + 4 + local_14 * 0x14);
          if (*(int *)((local_14 + 1) * 0x14 + *(int *)(param_1 + 0x2c)) <
              *(int *)(local_14 * 0x14 + *(int *)(param_1 + 0x2c))) {
            local_c = local_14 + 1;
            local_10 = local_14;
          }
          else {
            local_c = local_14;
            local_10 = local_14 + 1;
          }
          if (*(int *)(local_10 * 0x14 + *(int *)(param_1 + 0x2c)) <
              *(int *)(local_8 * 0x14 + *(int *)(param_1 + 0x34))) {
            *(undefined4 *)(local_8 * 0x14 + *(int *)(param_1 + 0x34)) =
                 *(undefined4 *)(local_10 * 0x14 + *(int *)(param_1 + 0x2c));
          }
          if (*(int *)(local_8 * 0x14 + *(int *)(param_1 + 0x34)) <
              *(int *)(local_c * 0x14 + *(int *)(param_1 + 0x2c))) {
            *(undefined4 *)(local_8 * 0x14 + *(int *)(param_1 + 0x34)) =
                 *(undefined4 *)(local_c * 0x14 + *(int *)(param_1 + 0x2c));
          }
        }
      }
    }
  }
  return 1;
}
