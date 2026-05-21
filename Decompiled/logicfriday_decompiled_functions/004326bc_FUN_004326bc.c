/* 004326bc FUN_004326bc */

int __thiscall FUN_004326bc(void *this,HWND param_1,int param_2,int param_3)

{
  int iVar1;
  int iVar2;
  int iVar3;
  undefined4 uVar4;
  int local_6c;
  int local_68;
  tagRECT local_64;
  int local_54;
  int local_50;
  int local_4c;
  int local_48;
  int local_44;
  int local_40;
  int local_3c;
  int local_38;
  int local_34;
  int local_30;
  int local_2c;
  int local_28;
  int local_24;
  int local_20;
  int local_1c;
  LONG local_18;
  int local_14;
  LONG LStack_10;
  int local_c;
  int local_8;
  
  local_48 = 0;
  for (local_24 = 0; local_24 < *(int *)((int)this + 0x16c4); local_24 = local_24 + 1) {
    if ((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xd8) != 0) &&
       (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0x48) == 0)) {
      iVar1 = *(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4);
      local_64.left = *(LONG *)(iVar1 + 200);
      local_64.top = *(int *)(iVar1 + 0xcc);
      local_64.right = *(LONG *)(iVar1 + 0xd0);
      local_64.bottom = *(int *)(iVar1 + 0xd4) * 2 - local_64.top;
      local_18 = local_64.left;
      local_14 = local_64.top;
      LStack_10 = local_64.right;
      local_c = local_64.bottom;
      OffsetRect(&local_64,param_2,param_3);
      *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xc0) =
           *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xc0) + param_2;
      *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xc4) =
           *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xc4) + param_3;
      OffsetRect((LPRECT)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 200),param_2,
                 param_3);
      local_48 = local_48 + 1;
      if (param_3 != 0) {
        for (local_68 = 0;
            local_68 < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0x18);
            local_68 = local_68 + 1) {
          iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xe4 +
                          local_68 * 4);
          if ((((iVar1 != -3) &&
               (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x28) == 2)) &&
              (**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x2c) ==
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x2c) + 0x14))
              ) && (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x2c) + 4
                            ) ==
                    *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x2c) +
                            0x18))) {
            *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x44) = 1;
            *(undefined4 *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar1 * 4) + 0x2c) + 8)
                 = 1;
          }
        }
        local_30 = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_24 * 4) + 0xe0);
        if (((local_30 != -3) &&
            (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x28) == 2)) &&
           ((**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x2c) ==
             *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x2c) + 0x14)
            && (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x2c) + 4)
                == *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x2c) +
                           0x18))))) {
          *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x44) = 1;
          *(undefined4 *)
           (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_30 * 4) + 0x2c) + 8) = 1;
        }
      }
    }
  }
  for (local_24 = 0; local_24 < *(int *)((int)this + 0x16c8); local_24 = local_24 + 1) {
    if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x44) != 0) &&
       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x40) == 0)) {
      *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x24) = 0;
      *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x10) = 0;
      for (local_68 = 0;
          local_68 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x28) + -1;
          local_68 = local_68 + 1) {
        if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                    local_68 * 0x14) != 0) {
          if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 0xc
                      + local_68 * 0x14) == 0) {
            *(int *)(local_68 * 0x14 +
                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                 *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                         local_68 * 0x14) + param_2;
            *(int *)((local_68 + 1) * 0x14 +
                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                 *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                         (local_68 + 1) * 0x14) + param_2;
            local_48 = local_48 + 1;
            FUN_0043b7be(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_68,param_2,
                         param_3);
          }
          else {
            *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4 +
                    local_68 * 0x14) =
                 *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                         + local_68 * 0x14) + param_3;
            *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4 +
                    (local_68 + 1) * 0x14) =
                 *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                         + (local_68 + 1) * 0x14) + param_3;
            local_48 = local_48 + 1;
            FUN_0043b7be(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_68,param_2,
                         param_3);
          }
        }
      }
      FUN_0043b3ae(*(int **)(*(int *)((int)this + 0x16d0) + local_24 * 4));
    }
  }
  FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
  local_44 = 1;
  while (local_44 != 0) {
    local_44 = 0;
    for (local_24 = 0; local_24 < *(int *)((int)this + 0x16c8); local_24 = local_24 + 1) {
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x40) == 0) {
        if ((**(int **)(*(int *)((int)this + 0x16d0) + local_24 * 4) == 1) ||
           (**(int **)(*(int *)((int)this + 0x16d0) + local_24 * 4) == 0)) {
          local_1c = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                                      *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4)
                                              + 4) * 4) + 0xd8);
          if (**(int **)(*(int *)((int)this + 0x16d0) + local_24 * 4) == 1) {
            iVar1 = *(int *)(*(int *)((int)this + 0x16cc) +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 4) * 4)
            ;
            local_2c = *(int *)(iVar1 + 0xac);
            local_28 = *(int *)(iVar1 + 0xb0);
          }
          else {
            iVar1 = *(int *)(*(int *)((int)this + 0x16cc) +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 4) * 4)
            ;
            iVar2 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 8);
            local_2c = *(int *)(iVar1 + 0x6c + iVar2 * 8);
            local_28 = *(int *)(iVar1 + 0x70 + iVar2 * 8);
          }
          if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 0xc)
              == 0) {
            if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4)
                != local_28) {
              *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4) =
                   local_28;
            }
            if (**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) != local_2c
               ) {
              if (((local_1c == 0) ||
                  (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                           0x20) != 1)) ||
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x28) < 3)) {
                local_50 = local_2c;
                local_4c = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) +
                                            0x2c) + 4);
                FUN_0043b10a(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_2c,
                             local_4c);
              }
              else {
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                        0x14) = local_2c;
                **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) = local_2c;
                FUN_0043b7be(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),0,param_2,
                             param_3);
              }
            }
          }
          else {
            if (**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) != local_2c
               ) {
              **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) = local_2c;
            }
            if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4)
                != local_28) {
              if (((local_1c == 0) ||
                  (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x28) < 3)) ||
                 (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                          0x20) != 0)) {
                local_4c = local_28;
                local_50 = **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c);
                FUN_0043b10a(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_50,
                             local_28);
              }
              else {
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                        0x18) = local_28;
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4)
                     = local_28;
                FUN_0043b7be(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),0,param_2,
                             param_3);
              }
            }
          }
        }
        if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x14) == 1) ||
           (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x14) == 0)) {
          local_1c = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                                      *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4)
                                              + 0x18) * 4) + 0xd8);
          iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x28);
          local_40 = iVar1 + -2;
          if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x14) == 1) {
            iVar2 = *(int *)(*(int *)((int)this + 0x16cc) +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x18) *
                            4);
            local_2c = *(int *)(iVar2 + 0xac);
            local_28 = *(int *)(iVar2 + 0xb0);
          }
          else {
            iVar2 = *(int *)(*(int *)((int)this + 0x16cc) +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x18) *
                            4);
            iVar3 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x1c);
            local_2c = *(int *)(iVar2 + 0x6c + iVar3 * 8);
            local_28 = *(int *)(iVar2 + 0x70 + iVar3 * 8);
          }
          if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 0xc
                      + local_40 * 0x14) == 0) {
            if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + (iVar1 + -1) * 0x14) != local_28) {
              *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4 +
                      (iVar1 + -1) * 0x14) = local_28;
            }
            if (*(int *)((iVar1 + -1) * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) !=
                local_2c) {
              if (((local_1c == 0) ||
                  (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                            0xc + (iVar1 + -3) * 0x14) != 1)) ||
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x28) < 3)) {
                local_50 = local_2c;
                local_4c = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) +
                                            0x2c) + 4 + (iVar1 + -1) * 0x14);
                FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_2c,
                             local_4c);
              }
              else {
                *(int *)(local_40 * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                     local_2c;
                *(int *)((iVar1 + -1) * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                     local_2c;
                FUN_0043b7be(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_40,
                             param_2,param_3);
              }
            }
          }
          else {
            if (*(int *)((iVar1 + -1) * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) !=
                local_2c) {
              *(int *)((iVar1 + -1) * 0x14 +
                      *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                   local_2c;
            }
            if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + (iVar1 + -1) * 0x14) != local_28) {
              if (((local_1c == 0) ||
                  (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                            0xc + (iVar1 + -3) * 0x14) != 0)) ||
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x28) < 3)) {
                local_4c = local_28;
                local_50 = *(int *)((iVar1 + -1) * 0x14 +
                                   *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) +
                                           0x2c));
                FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_50,
                             local_28);
              }
              else {
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14) = local_28;
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + (iVar1 + -1) * 0x14) = local_28;
                FUN_0043b7be(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_40,
                             param_2,param_3);
              }
            }
          }
        }
        for (local_68 = 0;
            local_68 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x30);
            local_68 = local_68 + 1) {
          FUN_0043b877(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4));
          local_8 = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)
                             + 0xc + local_68 * 0x14);
          if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 0x10
                      + local_68 * 0x14) == 0) {
            local_6c = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c
                                        ) + 0xc);
            local_54 = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c
                                        ) + 8);
            local_3c = 0;
          }
          else {
            iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28);
            local_3c = iVar1 + -1;
            local_6c = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c
                                        ) + 0xc + (iVar1 + -2) * 0x14);
            local_54 = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c
                                        ) + 8 + (iVar1 + -2) * 0x14);
          }
          iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c);
          local_38 = *(int *)(iVar1 + local_3c * 0x14);
          local_34 = *(int *)(iVar1 + 4 + local_3c * 0x14);
          if (local_6c == 0) {
            if (local_38 <
                *(int *)(local_68 * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34))) {
              local_40 = FUN_0043c5dc(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),
                                      local_68);
              if (local_38 <
                  *(int *)(local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c))) {
                if (local_54 == 0) {
                  if (local_3c == 0) {
                    *(undefined4 *)
                     (local_68 * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    **(undefined4 **)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 0x14)
                         = *(undefined4 *)
                            (local_40 * 0x14 +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                  }
                  else {
                    *(undefined4 *)
                     (local_68 * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    *(undefined4 *)
                     (local_3c * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    *(undefined4 *)
                     ((local_3c + -1) * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                  }
                }
                else {
                  *(int *)(local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                       local_38;
                  *(int *)(local_68 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                       local_38;
                  local_20 = FUN_0043c7b8(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),
                                          local_40,0);
                  if (local_20 != -3) {
                    *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x44) =
                         1;
                    if (local_20 == local_40 + -1) {
                      *(undefined4 *)
                       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                       local_20 * 0x14) = 1;
                    }
                    else {
                      *(undefined4 *)
                       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                       local_40 * 0x14) = 1;
                    }
                    *(int *)(local_20 * 0x14 +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c))
                         = local_38;
                    if (1 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x30))
                    {
                      local_44 = 1;
                    }
                  }
                }
              }
              else {
                *(int *)(local_68 * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                     local_38;
              }
            }
            else if (*(int *)(local_68 * 0x14 +
                             *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34))
                     < local_38) {
              local_40 = FUN_0043c652(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),
                                      local_68);
              if (*(int *)(local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) <
                  local_38) {
                if (local_54 == 0) {
                  if (local_3c == 0) {
                    *(undefined4 *)
                     (local_68 * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    **(undefined4 **)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 0x14)
                         = *(undefined4 *)
                            (local_40 * 0x14 +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                  }
                  else {
                    *(undefined4 *)
                     (local_68 * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    *(undefined4 *)
                     (local_3c * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                    *(undefined4 *)
                     ((local_3c + -1) * 0x14 +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)) =
                         *(undefined4 *)
                          (local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c));
                  }
                }
                else {
                  *(int *)(local_40 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c)) =
                       local_38;
                  *(int *)(local_68 * 0x14 +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                       local_38;
                  local_20 = FUN_0043c7b8(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),
                                          local_40,0);
                  if (local_20 != -3) {
                    *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x44) =
                         1;
                    if (local_20 == local_40 + -1) {
                      *(undefined4 *)
                       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                       local_20 * 0x14) = 1;
                    }
                    else {
                      *(undefined4 *)
                       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                       local_40 * 0x14) = 1;
                    }
                    *(int *)(local_20 * 0x14 +
                            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c))
                         = local_38;
                    if (1 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x30))
                    {
                      local_44 = 1;
                    }
                  }
                }
              }
              else {
                *(int *)(local_68 * 0x14 +
                        *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)) =
                     local_38;
              }
            }
            else if (local_34 !=
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)
                              + 4 + local_68 * 0x14)) {
              iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34);
              uVar4 = *(undefined4 *)(iVar1 + 4 + local_68 * 0x14);
              iVar2 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c);
              *(undefined4 *)(iVar2 + local_3c * 0x14) = *(undefined4 *)(iVar1 + local_68 * 0x14);
              *(undefined4 *)(iVar2 + 4 + local_3c * 0x14) = uVar4;
            }
          }
          else if (local_34 <
                   *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) +
                            4 + local_68 * 0x14)) {
            local_40 = FUN_0043c6c8(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_68
                                   );
            if (local_34 <
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14)) {
              if (local_54 == 0) {
                if (local_3c == 0) {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4 +
                   local_68 * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 0x18) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                }
                else {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4 +
                   local_68 * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4 +
                   local_3c * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4 +
                   (local_3c + -1) * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                }
              }
              else {
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14) = local_34;
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4
                        + local_68 * 0x14) = local_34;
                local_20 = FUN_0043c7b8(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),
                                        local_40,1);
                if (local_20 != -3) {
                  *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x44) = 1;
                  if (local_20 == local_40 + -1) {
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                     local_20 * 0x14) = 1;
                  }
                  else {
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                     local_40 * 0x14) = 1;
                  }
                  *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                           4 + local_20 * 0x14) = local_34;
                  if (1 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x30)) {
                    local_44 = 1;
                  }
                }
              }
            }
            else {
              *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4 +
                      local_68 * 0x14) = local_34;
            }
          }
          else if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) +
                            4 + local_68 * 0x14) < local_34) {
            local_40 = FUN_0043c740(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),local_68
                                   );
            if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14) < local_34) {
              if (local_54 == 0) {
                if (local_3c == 0) {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4 +
                   local_68 * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 0x18) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                }
                else {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4 +
                   local_68 * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4 +
                   local_3c * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4 +
                   (local_3c + -1) * 0x14) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14);
                }
              }
              else {
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 4
                        + local_40 * 0x14) = local_34;
                *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4
                        + local_68 * 0x14) = local_34;
                local_20 = FUN_0043c7b8(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),
                                        local_40,1);
                if (local_20 != -3) {
                  *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x44) = 1;
                  if (local_20 == local_40 + -1) {
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                     local_20 * 0x14) = 1;
                  }
                  else {
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) + 8 +
                     local_40 * 0x14) = 1;
                  }
                  *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x2c) +
                           4 + local_20 * 0x14) = local_34;
                  if (1 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x30)) {
                    local_44 = 1;
                  }
                }
              }
            }
            else {
              *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34) + 4 +
                      local_68 * 0x14) = local_34;
            }
          }
          else if (local_38 !=
                   *(int *)(local_68 * 0x14 +
                           *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34)))
          {
            iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x34);
            uVar4 = *(undefined4 *)(iVar1 + 4 + local_68 * 0x14);
            iVar2 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c);
            *(undefined4 *)(iVar2 + local_3c * 0x14) = *(undefined4 *)(iVar1 + local_68 * 0x14);
            *(undefined4 *)(iVar2 + 4 + local_3c * 0x14) = uVar4;
          }
        }
        FUN_0043b3ae(*(int **)(*(int *)((int)this + 0x16d0) + local_24 * 4));
      }
    }
  }
  for (local_24 = 0; local_24 < *(int *)((int)this + 0x16c8); local_24 = local_24 + 1) {
    if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x3c) != 0) &&
       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_24 * 4) + 0x40) == 0)) {
      FUN_0043c8a6(*(void **)(*(int *)((int)this + 0x16d0) + local_24 * 4),param_1);
    }
  }
  FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
  return local_48;
}
