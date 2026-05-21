/* 004207ae FUN_004207ae */

undefined4 __fastcall FUN_004207ae(int param_1)

{
  undefined4 local_14;
  undefined4 local_10;
  undefined4 local_c;
  undefined4 local_8;
  
  local_c = 0;
  local_8 = *(int *)(param_1 + 0x1654);
  do {
    if (*(int *)(param_1 + 0x1650) <= local_8) {
      return local_c;
    }
    if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) == 0) {
      for (local_14 = 0; local_14 < *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
          local_14 = local_14 + 1) {
        local_10 = *(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_14 * 4);
        while (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_10 * 0xfc) != 0) {
          if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x4c + local_10 * 0xfc) == -1) {
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_10 * 0xfc) = 0;
            *(int *)(*(int *)(param_1 + 0x1668) +
                    *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + local_10 * 0xfc) * 4) =
                 *(int *)(*(int *)(param_1 + 0x1668) +
                         *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + local_10 * 0xfc) * 4) + 1;
            break;
          }
          local_10 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x4c + local_10 * 0xfc);
          local_c = 1;
        }
        *(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_14 * 4) = local_10;
      }
    }
    local_8 = local_8 + 1;
  } while( true );
}
