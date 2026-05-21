/* 0043b3ae FUN_0043b3ae */

void __fastcall FUN_0043b3ae(int *param_1)

{
  bool bVar1;
  bool bVar2;
  int local_14;
  int local_8;
  
  bVar1 = true;
  if (param_1[10] < 3) {
    return;
  }
  do {
    if ((!bVar1) || (param_1[10] < 3)) {
      if (DAT_00452ef4 == 0) {
        return;
      }
      for (local_8 = 0; local_8 < param_1[0xc]; local_8 = local_8 + 1) {
        for (local_14 = 0;
            (local_14 < param_1[10] &&
            (*(int *)(param_1[0xd] + 8 + local_8 * 0x14) !=
             *(int *)(param_1[0xb] + 0x10 + local_14 * 0x14))); local_14 = local_14 + 1) {
        }
      }
      return;
    }
    bVar1 = false;
    for (local_8 = 0; local_8 < param_1[10] + -1; local_8 = local_8 + 1) {
      if (((local_8 != 0) || (*param_1 != 2)) &&
         ((local_8 != param_1[10] + -2 || (param_1[5] != 2)))) {
        bVar2 = false;
        for (local_14 = 0; local_14 < param_1[0xc]; local_14 = local_14 + 1) {
          if (*(int *)(param_1[0xd] + 8 + local_14 * 0x14) ==
              *(int *)(param_1[0xb] + 0x10 + local_8 * 0x14)) {
            bVar2 = true;
            break;
          }
        }
        if (!bVar2) {
          if (*(int *)(param_1[0xb] + 0xc + local_8 * 0x14) == 0) {
            if (*(int *)(param_1[0xb] + 4 + local_8 * 0x14) ==
                *(int *)(param_1[0xb] + 4 + (local_8 + 1) * 0x14)) {
              if (local_8 == 0) {
                FUN_0043c82b(param_1,0);
                bVar1 = true;
              }
              else if (local_8 == param_1[10] + -2) {
                FUN_0043c82b(param_1,param_1[10] + -1);
                bVar1 = true;
              }
              else {
                for (local_14 = 0; local_14 < param_1[0xc]; local_14 = local_14 + 1) {
                  if (*(int *)(param_1[0xd] + 8 + local_14 * 0x14) ==
                      *(int *)(param_1[0xb] + 0x10 + (local_8 + 1) * 0x14)) {
                    *(undefined4 *)(param_1[0xd] + 8 + local_14 * 0x14) =
                         *(undefined4 *)(param_1[0xb] + 0x10 + (local_8 + -1) * 0x14);
                  }
                }
                if (*(int *)(param_1[0xb] + 8 + (local_8 + 1) * 0x14) != 0) {
                  *(undefined4 *)(param_1[0xb] + 8 + (local_8 + -1) * 0x14) = 1;
                }
                if (local_8 != 0) {
                  FUN_0043c82b(param_1,local_8);
                }
                if (local_8 != param_1[10] + -1) {
                  FUN_0043c82b(param_1,local_8);
                }
                bVar1 = true;
              }
              break;
            }
            if ((local_8 != 0) && (*(int *)(param_1[0xb] + 0xc + (local_8 + -1) * 0x14) == 0)) {
              FUN_0043c82b(param_1,local_8);
              bVar1 = true;
              break;
            }
          }
          else {
            if (*(int *)(local_8 * 0x14 + param_1[0xb]) ==
                *(int *)((local_8 + 1) * 0x14 + param_1[0xb])) {
              if (local_8 == 0) {
                FUN_0043c82b(param_1,0);
                bVar1 = true;
              }
              else if (local_8 == param_1[10] + -2) {
                FUN_0043c82b(param_1,param_1[10] + -1);
                bVar1 = true;
              }
              else {
                for (local_14 = 0; local_14 < param_1[0xc]; local_14 = local_14 + 1) {
                  if (*(int *)(param_1[0xd] + 8 + local_14 * 0x14) ==
                      *(int *)(param_1[0xb] + 0x10 + (local_8 + 1) * 0x14)) {
                    *(undefined4 *)(param_1[0xd] + 8 + local_14 * 0x14) =
                         *(undefined4 *)(param_1[0xb] + 0x10 + (local_8 + -1) * 0x14);
                  }
                }
                if (*(int *)(param_1[0xb] + 8 + (local_8 + 1) * 0x14) != 0) {
                  *(undefined4 *)(param_1[0xb] + 8 + (local_8 + -1) * 0x14) = 1;
                }
                if (local_8 != 0) {
                  FUN_0043c82b(param_1,local_8);
                }
                if (local_8 != param_1[10] + -1) {
                  FUN_0043c82b(param_1,local_8);
                }
                bVar1 = true;
              }
              break;
            }
            if ((local_8 != 0) && (*(int *)(param_1[0xb] + 0xc + (local_8 + -1) * 0x14) == 1)) {
              FUN_0043c82b(param_1,local_8);
              bVar1 = true;
              break;
            }
          }
        }
      }
    }
  } while( true );
}
