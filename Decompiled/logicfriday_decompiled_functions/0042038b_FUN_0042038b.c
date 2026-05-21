/* 0042038b FUN_0042038b */

int __fastcall FUN_0042038b(int param_1)

{
  int iVar1;
  int iVar2;
  int local_24;
  int local_20;
  int local_18;
  int local_14;
  int local_10;
  int local_8;
  
  local_18 = 0;
  for (local_8 = *(int *)(param_1 + 0x1654); local_8 < *(int *)(param_1 + 0x1658);
      local_8 = local_8 + 1) {
    iVar2 = local_18;
    if ((*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4)) == 0) &&
       (*(int *)(*(int *)(*(int *)(param_1 + 0x3a4) + 0x1c + local_8 * 0xfc) * 0xfc +
                *(int *)(param_1 + 0x3a4)) == 0)) {
      *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) = 1;
      iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x1c + local_8 * 0xfc);
      *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x4c + local_8 * 0xfc) =
           *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x1c + iVar1 * 0xfc);
      *(int *)(*(int *)(param_1 + 0x1668) +
              *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + local_8 * 0xfc) * 4) =
           *(int *)(*(int *)(param_1 + 0x1668) +
                   *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + local_8 * 0xfc) * 4) + -1;
      iVar2 = local_18 + -1;
      if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + iVar1 * 0xfc) == 0) {
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x48 + iVar1 * 0xfc) = 1;
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x4c + iVar1 * 0xfc) = 0xffffffff;
        *(int *)(*(int *)(param_1 + 0x1668) +
                *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + iVar1 * 0xfc) * 4) =
             *(int *)(*(int *)(param_1 + 0x1668) +
                     *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + iVar1 * 0xfc) * 4) + -1;
        iVar2 = local_18;
      }
    }
    local_18 = iVar2;
  }
  do {
    for (local_8 = *(int *)(param_1 + 0x1654); local_8 < *(int *)(param_1 + 0x1658) + -1;
        local_8 = local_8 + 1) {
      if ((*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) == 0) ||
         (*(int *)(*(int *)(param_1 + 0x3a4) + 0x4c + local_8 * 0xfc) == -1)) {
        iVar2 = *(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4));
        local_24 = local_8;
        while (local_24 = local_24 + 1, local_24 < *(int *)(param_1 + 0x1658)) {
          if (((*(int *)(local_24 * 0xfc + *(int *)(param_1 + 0x3a4)) == iVar2) &&
              (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_24 * 0xfc) == 0)) &&
             (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_24 * 0xfc) ==
              *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc))) {
            for (local_10 = 0;
                local_10 < *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
                local_10 = local_10 + 1) {
              local_14 = 0;
              for (local_20 = 0;
                  local_20 < *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_24 * 0xfc);
                  local_20 = local_20 + 1) {
                if (*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_10 * 4) ==
                    *(int *)(local_24 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_20 * 4)) {
                  local_14 = 1;
                }
              }
              if (local_14 == 0) break;
            }
            if (local_14 != 0) {
              *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_24 * 0xfc) = 1;
              *(int *)(*(int *)(param_1 + 0x3a4) + 0x4c + local_24 * 0xfc) = local_8;
              *(int *)(*(int *)(param_1 + 0x1668) +
                      *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + local_24 * 0xfc) * 4) =
                   *(int *)(*(int *)(param_1 + 0x1668) +
                           *(int *)(*(int *)(param_1 + 0x3a4) + 0x3c + local_24 * 0xfc) * 4) + -1;
              local_18 = local_18 + 1;
            }
          }
        }
      }
    }
    iVar2 = FUN_004207ae(param_1);
    if (iVar2 == 0) {
      for (local_8 = 0; local_8 < 0x40; local_8 = local_8 + 1) {
      }
      return local_18;
    }
  } while( true );
}
