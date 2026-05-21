/* 004349ff FUN_004349ff */

void __fastcall FUN_004349ff(int param_1)

{
  int iVar1;
  int local_18;
  int local_14;
  int local_c;
  int local_8;
  
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x16c8); local_8 = local_8 + 1) {
    *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) = 0xfffffffd;
  }
  local_14 = *(int *)(param_1 + 0x16c8);
  local_c = local_14 + 1;
  while ((0 < local_14 && (local_14 < local_c))) {
    for (local_8 = 0; local_8 < *(int *)(param_1 + 0x16c8); local_8 = local_8 + 1) {
      if ((*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x40) == 0) &&
         (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) == -3)) {
        if (**(int **)(*(int *)(param_1 + 0x16d0) + local_8 * 4) == 1) {
          *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) =
               *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 4);
          if (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x14) == 2) {
            *(undefined4 *)
             (*(int *)(*(int *)(param_1 + 0x16d0) +
                      *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x20) * 4) +
             0x38) = *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38);
          }
        }
        else if (**(int **)(*(int *)(param_1 + 0x16d0) + local_8 * 4) == 2) {
          iVar1 = *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0xc);
          if ((*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + iVar1 * 4) + 0x38) != -3) &&
             (*(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) =
                   *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + iVar1 * 4) + 0x38),
             *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x14) == 2)) {
            *(undefined4 *)
             (*(int *)(*(int *)(param_1 + 0x16d0) +
                      *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x20) * 4) +
             0x38) = *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38);
          }
        }
        else if (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x14) == 1) {
          *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) =
               *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x18);
          if (**(int **)(*(int *)(param_1 + 0x16d0) + local_8 * 4) == 2) {
            *(undefined4 *)
             (*(int *)(*(int *)(param_1 + 0x16d0) +
                      *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0xc) * 4) + 0x38
             ) = *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38);
          }
        }
        else if (((*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x14) == 2) &&
                 (iVar1 = *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x20),
                 *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + iVar1 * 4) + 0x38) != -3)) &&
                (*(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) =
                      *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + iVar1 * 4) + 0x38),
                **(int **)(*(int *)(param_1 + 0x16d0) + local_8 * 4) == 2)) {
          *(undefined4 *)
           (*(int *)(*(int *)(param_1 + 0x16d0) +
                    *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0xc) * 4) + 0x38)
               = *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38);
        }
        if (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) != -3) {
          for (local_18 = 0;
              local_18 < *(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x30);
              local_18 = local_18 + 1) {
            *(undefined4 *)
             (*(int *)(*(int *)(param_1 + 0x16d0) +
                      *(int *)(*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x34) +
                               0xc + local_18 * 0x14) * 4) + 0x38) =
                 *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38);
          }
        }
      }
    }
    local_c = local_14;
    local_14 = 0;
    for (local_8 = 0; local_8 < *(int *)(param_1 + 0x16c8); local_8 = local_8 + 1) {
      if (*(int *)(*(int *)(*(int *)(param_1 + 0x16d0) + local_8 * 4) + 0x38) == -3) {
        local_14 = local_14 + 1;
      }
    }
  }
  return;
}
