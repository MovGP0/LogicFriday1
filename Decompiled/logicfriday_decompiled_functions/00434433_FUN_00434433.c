/* 00434433 FUN_00434433 */

int __fastcall FUN_00434433(void *param_1)

{
  bool bVar1;
  int iVar2;
  int local_1c;
  int local_14;
  int local_c;
  
  local_14 = 0;
  local_c = 0;
  do {
    if (*(int *)((int)param_1 + 0x16c8) <= local_c) {
      for (local_c = 0; local_c < *(int *)((int)param_1 + 0x16c8); local_c = local_c + 1) {
        if ((*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x40) == 0) &&
           (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) == -3)) {
          local_14 = local_14 + 1;
        }
      }
      if (local_14 != 0) {
        do {
          bVar1 = false;
          for (local_c = 0; local_c < *(int *)((int)param_1 + 0x16c8); local_c = local_c + 1) {
            if ((*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x40) == 0) &&
               (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) == -3)) {
              if ((**(int **)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) == 2) &&
                 (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                   *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4)
                                           + 0xc) * 4) + 0x38) != -3)) {
                *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) =
                     *(undefined4 *)
                      (*(int *)(*(int *)((int)param_1 + 0x16d0) +
                               *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) +
                                       0xc) * 4) + 0x38);
                bVar1 = true;
                local_14 = local_14 + -1;
              }
              if ((*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x14) == 2) &&
                 (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                   *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4)
                                           + 0x20) * 4) + 0x38) != -3)) {
                if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) == -3)
                {
                  *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) =
                       *(undefined4 *)
                        (*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                 *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) +
                                         0x20) * 4) + 0x38);
                  bVar1 = true;
                  local_14 = local_14 + -1;
                }
                else if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) !=
                         *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                          *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                                           local_c * 4) + 0x20) * 4) + 0x38)) {
                  return *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) +
                         0x3eb0000;
                }
              }
            }
          }
        } while ((local_14 != 0) && (bVar1));
      }
      for (local_c = 0; local_c < *(int *)((int)param_1 + 0x16c4); local_c = local_c + 1) {
        if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) + 0x48) == 0) {
          for (local_1c = 0;
              local_1c < *(int *)(*(int *)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) + 0x18);
              local_1c = local_1c + 1) {
            iVar2 = *(int *)(*(int *)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) + 0xe4 +
                            local_1c * 4);
            if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + iVar2 * 4) + 0x38) == -3) {
              return 0x290000;
            }
            *(undefined4 *)
             (*(int *)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) + 0x1c + local_1c * 4) =
                 *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + iVar2 * 4) + 0x38);
          }
        }
      }
      local_c = 0;
      while( true ) {
        if (*(int *)((int)param_1 + 0x16c4) <= local_c) {
          return 0;
        }
        if ((((**(int **)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) != 8) &&
             (**(int **)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) != 9)) &&
            (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) + 0x48) == 0)) &&
           (((**(int **)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) != 10 &&
             (**(int **)(*(int *)((int)param_1 + 0x16cc) + local_c * 4) != 0xb)) &&
            (iVar2 = FUN_0042fa9d(param_1,local_c,local_c), iVar2 != 0)))) break;
        local_c = local_c + 1;
      }
      return iVar2 + 0x3f20000;
    }
    if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x40) == 0) {
      if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x44) != 0) {
        *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x44) = 0;
        for (local_1c = 0;
            local_1c < *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x28) + -1
            ; local_1c = local_1c + 1) {
          *(undefined4 *)
           (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x2c) + 8 +
           local_1c * 0x14) = 0;
        }
      }
      if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38) != -3) {
        if (**(int **)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) == 2) {
          if ((*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) +
                                        0xc) * 4) + 0x38) != -3) &&
             (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                               *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) +
                                       0xc) * 4) + 0x38) !=
              *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38))) {
            return local_c + 0x3eb0000;
          }
          *(undefined4 *)
           (*(int *)(*(int *)((int)param_1 + 0x16d0) +
                    *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0xc) * 4) +
           0x38) = *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38);
        }
        if (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x14) == 2) {
          if ((*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                                *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) +
                                        0x20) * 4) + 0x38) != -3) &&
             (*(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) +
                               *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) +
                                       0x20) * 4) + 0x38) !=
              *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38))) {
            return local_c + 0x3eb0000;
          }
          *(undefined4 *)
           (*(int *)(*(int *)((int)param_1 + 0x16d0) +
                    *(int *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x20) * 4) +
           0x38) = *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x16d0) + local_c * 4) + 0x38);
        }
      }
    }
    local_c = local_c + 1;
  } while( true );
}
