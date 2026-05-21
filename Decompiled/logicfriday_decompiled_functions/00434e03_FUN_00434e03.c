/* 00434e03 FUN_00434e03 */

undefined4 __fastcall FUN_00434e03(void *param_1)

{
  int local_14;
  int local_10;
  int local_c;
  int local_8;
  
  for (local_c = 0; local_c < *(int *)((int)param_1 + 0x1650); local_c = local_c + 1) {
    *(undefined4 *)(*(int *)((int)param_1 + 0x3a4) + 0xbc + local_c * 0xfc) = 0;
  }
  for (local_c = 0; local_c < *(int *)((int)param_1 + 0x1650); local_c = local_c + 1) {
    if ((*(int *)(*(int *)((int)param_1 + 0x3a4) + 0x48 + local_c * 0xfc) == 0) &&
       ((*(int *)(local_c * 0xfc + *(int *)((int)param_1 + 0x3a4)) == 10 ||
        (*(int *)(local_c * 0xfc + *(int *)((int)param_1 + 0x3a4)) == 0xb)))) {
      *(undefined4 *)(*(int *)((int)param_1 + 0x3a4) + 0x48 + local_c * 0xfc) = 1;
      *(undefined4 *)
       (*(int *)(*(int *)((int)param_1 + 0x16d0) +
                *(int *)(*(int *)((int)param_1 + 0x3a4) + 0xe0 + local_c * 0xfc) * 4) + 0x40) = 1;
      for (local_14 = 0; local_14 < *(int *)((int)param_1 + 0x1650); local_14 = local_14 + 1) {
        if ((((*(int *)(*(int *)((int)param_1 + 0x3a4) + 0x48 + local_14 * 0xfc) == 0) &&
             (*(int *)(local_14 * 0xfc + *(int *)((int)param_1 + 0x3a4)) != 10)) &&
            (*(int *)(local_14 * 0xfc + *(int *)((int)param_1 + 0x3a4)) != 0xb)) &&
           (*(int *)(local_14 * 0xfc + *(int *)((int)param_1 + 0x3a4)) != 8)) {
          for (local_10 = 0;
              local_10 < *(int *)(*(int *)((int)param_1 + 0x3a4) + 0x18 + local_14 * 0xfc);
              local_10 = local_10 + 1) {
            if (*(int *)(local_14 * 0xfc + *(int *)((int)param_1 + 0x3a4) + 0x1c + local_10 * 4) ==
                local_c) {
              if (*(int *)(local_c * 0xfc + *(int *)((int)param_1 + 0x3a4)) == 10) {
                *(undefined4 *)
                 (local_14 * 0xfc + *(int *)((int)param_1 + 0x3a4) + 0x1c + local_10 * 4) =
                     0xffffffff;
              }
              else {
                *(undefined4 *)
                 (local_14 * 0xfc + *(int *)((int)param_1 + 0x3a4) + 0x1c + local_10 * 4) =
                     0xfffffffe;
              }
              *(int *)(*(int *)((int)param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) =
                   *(int *)(*(int *)((int)param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) + 1;
            }
          }
        }
      }
    }
  }
  local_8 = 0;
  for (local_c = 0; local_c < *(int *)((int)param_1 + 0x1650); local_c = local_c + 1) {
    if (((*(int *)(*(int *)((int)param_1 + 0x3a4) + 0x48 + local_c * 0xfc) == 0) &&
        (*(int *)(local_c * 0xfc + *(int *)((int)param_1 + 0x3a4)) != 8)) &&
       (*(int *)(local_c * 0xfc + *(int *)((int)param_1 + 0x3a4)) != 9)) {
      local_8 = local_8 + 1;
      *(undefined4 *)(*(int *)((int)param_1 + 0x3a4) + 0x3c + local_c * 0xfc) = 0xfffffffd;
    }
  }
  FUN_0043510c(param_1,local_8);
  FUN_00424347();
  *(undefined4 *)((int)param_1 + 0x16b4) = 0;
  return 1;
}
