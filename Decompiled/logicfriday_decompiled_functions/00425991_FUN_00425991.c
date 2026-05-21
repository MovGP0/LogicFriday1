/* 00425991 FUN_00425991 */

void __fastcall FUN_00425991(int param_1)

{
  int iVar1;
  undefined4 local_14;
  undefined4 local_10;
  undefined4 local_8;
  
  for (local_14 = 0; local_14 < *(int *)(param_1 + 0x2674); local_14 = local_14 + 1) {
    *(undefined4 *)(**(int **)(param_1 + 0x2678) + 8 + local_14 * 0x48) = 0;
  }
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x2670); local_8 = local_8 + 1) {
    *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x14) = 0;
    for (local_14 = 0; local_14 < *(int *)(param_1 + 0x2674); local_14 = local_14 + 1) {
      iVar1 = *(int *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x10 + local_14 * 0x48)
              + 1 + *(int *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0xc +
                            local_14 * 0x48);
      if (*(int *)(**(int **)(param_1 + 0x2678) + 8 + local_14 * 0x48) < iVar1) {
        *(int *)(**(int **)(param_1 + 0x2678) + 8 + local_14 * 0x48) = iVar1;
      }
      if (*(int *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x14) <
          *(int *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x18 + local_14 * 0x48)) {
        *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x14) =
             *(undefined4 *)
              (*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x18 + local_14 * 0x48);
      }
      *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x18 + local_14 * 0x48) =
           0;
      *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x10 + local_14 * 0x48) =
           0;
      *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0xc + local_14 * 0x48) =
           0;
      for (local_10 = 0; local_10 < 4; local_10 = local_10 + 1) {
        *(undefined4 *)
         (local_14 * 0x48 + *(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x28 + local_10 * 4
         ) = 0;
        *(undefined4 *)
         (local_14 * 0x48 + *(int *)(*(int *)(param_1 + 0x2678) + local_8 * 4) + 0x38 + local_10 * 4
         ) = 0;
      }
    }
  }
  return;
}
