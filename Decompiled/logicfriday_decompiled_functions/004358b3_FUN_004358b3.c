/* 004358b3 FUN_004358b3 */

undefined4 __fastcall FUN_004358b3(uint *param_1)

{
  int iVar1;
  uint local_20;
  uint local_18;
  int local_14;
  int local_10;
  uint local_c;
  
  local_20 = param_1[0x596];
  do {
    if ((int)param_1[0x594] <= (int)local_20) {
      return 0;
    }
    if (*(int *)(param_1[0xe9] + 0x48 + local_20 * 0xfc) == 0) {
      local_18 = 0;
      while ((local_18 < param_1[0x32] &&
             (iVar1 = _strcmp((char *)((int)param_1 + local_18 * 9 + 0xd0),
                              (char *)(param_1[0xe9] + 4 + local_20 * 0xfc)), iVar1 != 0))) {
        local_18 = local_18 + 1;
      }
      if (param_1[0x32] <= local_18) {
        return 0x1a0000;
      }
      for (local_10 = 0; local_10 < (int)param_1[0x594]; local_10 = local_10 + 1) {
        FUN_0041770d((int *)(local_10 * 0xfc + param_1[0xe9]));
      }
      for (local_c = 0; local_c < *param_1; local_c = local_c + 1) {
        for (local_14 = 0; local_14 < (int)param_1[0x595]; local_14 = local_14 + 1) {
          if ((1 << (((char)param_1[0x595] + -1) - (char)local_14 & 0x1fU) & local_c) == 0) {
            *(undefined4 *)(param_1[0xe9] + 0x14 + local_14 * 0xfc) = 0;
          }
          else {
            *(undefined4 *)(param_1[0xe9] + 0x14 + local_14 * 0xfc) = 1;
          }
        }
        iVar1 = FUN_00417769((void *)(local_20 * 0xfc + param_1[0xe9]),param_1[0xe9],local_c);
        if (iVar1 != 0) {
          *(undefined4 *)(param_1[local_18 + 0x21] + local_c * 4) = 1;
          param_1[local_18 + 1] = param_1[local_18 + 1] + 1;
        }
        for (local_10 = 0; local_10 < (int)param_1[0x594]; local_10 = local_10 + 1) {
          FUN_0041770d((int *)(local_10 * 0xfc + param_1[0xe9]));
        }
      }
    }
    local_20 = local_20 + 1;
  } while( true );
}
