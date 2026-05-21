/* 0042f2a2 FUN_0042f2a2 */

/* WARNING: Unable to track spacebase fully for stack */

undefined4 FUN_0042f2a2(int param_1,int param_2,int *param_3,int *param_4,int *param_5)

{
  void *this;
  int iVar1;
  BOOL BVar2;
  int *piVar3;
  int *piVar4;
  undefined1 *puVar5;
  undefined1 *puVar6;
  int *piVar7;
  int local_3c;
  int local_38;
  int local_34;
  int local_30;
  int local_2c;
  int local_28;
  int local_24;
  undefined4 local_20;
  undefined4 local_1c;
  undefined4 local_18;
  undefined4 local_14;
  int local_10;
  int local_c;
  int local_8;
  
  local_30 = 0;
  local_c = 0;
  local_34 = 0;
  iVar1 = FUN_0043f3b8(*(int *)param_4[0xb] - param_1);
  if ((iVar1 < 7) && (iVar1 = FUN_0043f3b8(*(int *)(param_4[0xb] + 4) - param_2), iVar1 < 7)) {
    return 0xffffffff;
  }
  piVar7 = &local_3c;
  if (*(int *)(local_3c + 0x16c4) == 0) {
    return 0;
  }
  for (local_10 = 0; local_10 < *(int *)(local_3c + 0x16c4); local_10 = local_10 + 1) {
    if (*(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0x48) == 0) {
      piVar4 = piVar7;
      if (**(int **)(*(int *)(local_3c + 0x16cc) + local_10 * 4) != 9) {
        local_18 = *(undefined4 *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xac);
        local_20 = local_18;
        local_14 = *(undefined4 *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xb0);
        local_1c = local_14;
        piVar7[-1] = 6;
        piVar7[-2] = 6;
        piVar7[-3] = (int)&local_20;
        piVar3 = piVar7 + -4;
        piVar7[-4] = 0x42f399;
        InflateRect((LPRECT)piVar7[-3],piVar7[-2],piVar7[-1]);
        *(int *)((int)piVar3 + -4) = param_2;
        *(int *)((int)piVar3 + -8) = param_1;
        *(undefined4 **)((int)piVar3 + -0xc) = &local_20;
        piVar4 = (int *)((int)piVar3 + -0x10);
        *(undefined4 *)((int)piVar3 + -0x10) = 0x42f3a9;
        BVar2 = PtInRect(*(RECT **)((int)piVar3 + -0xc),*(POINT *)((int)piVar3 + -8));
        if (BVar2 != 0) {
          if (param_4[0xe] != -3) {
            return 0xffffffff;
          }
          if ((*param_4 == 0) && (param_4[1] == local_10)) {
            return 0xffffffff;
          }
          if (*param_4 == 2) {
            local_2c = param_4[3];
            if ((**(int **)(*(int *)(local_3c + 0x16d0) + local_2c * 4) == 0) &&
               (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_2c * 4) + 4) == local_10)) {
              return 0xffffffff;
            }
            if ((*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_2c * 4) + 0x14) == 0) &&
               (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_2c * 4) + 0x18) == local_10))
            {
              return 0xffffffff;
            }
          }
          iVar1 = *(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4);
          local_28 = *(int *)(iVar1 + 0xac);
          local_24 = *(int *)(iVar1 + 0xb0);
          if (*(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xe0) == -3) {
            param_4[5] = 1;
            param_4[6] = local_10;
            param_4[0xe] = local_10;
          }
          else {
            param_4[5] = 2;
            param_4[8] = *(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xe0);
            param_4[0xe] = local_10;
          }
          if (*(int *)(local_3c + 0x2348) == 0) {
            param_3[1] = local_24;
          }
          else {
            *param_3 = local_28;
          }
          *(int *)((int)piVar4 + -4) = local_24;
          *(int *)((int)piVar4 + -8) = local_28;
          *(int *)((int)piVar4 + -0xc) = param_3[1];
          *(int *)((int)piVar4 + -0x10) = *param_3;
          *(undefined4 *)((int)piVar4 + -0x14) = 0x42f50f;
          FUN_0043b238(param_4,*(int *)((int)piVar4 + -0x10),*(int *)((int)piVar4 + -0xc),
                       *(int *)((int)piVar4 + -8),*(int *)((int)piVar4 + -4));
          return 1;
        }
      }
      for (local_38 = 0;
          local_38 < *(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0x18);
          local_38 = local_38 + 1) {
        local_18 = *(undefined4 *)
                    (*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0x6c + local_38 * 8);
        local_20 = local_18;
        local_14 = *(undefined4 *)
                    (*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0x70 + local_38 * 8);
        local_1c = local_14;
        *(undefined4 *)((int)piVar4 + -4) = 6;
        *(undefined4 *)((int)piVar4 + -8) = 6;
        *(undefined4 **)((int)piVar4 + -0xc) = &local_20;
        puVar5 = (undefined1 *)((int)piVar4 + -0x10);
        *(undefined4 *)((int)piVar4 + -0x10) = 0x42f58b;
        InflateRect(*(LPRECT *)((int)piVar4 + -0xc),*(int *)((int)piVar4 + -8),
                    *(int *)((int)piVar4 + -4));
        *(int *)(puVar5 + -4) = param_2;
        *(int *)(puVar5 + -8) = param_1;
        *(undefined4 **)(puVar5 + -0xc) = &local_20;
        piVar4 = (int *)(puVar5 + -0x10);
        *(undefined4 *)(puVar5 + -0x10) = 0x42f59b;
        BVar2 = PtInRect(*(RECT **)(puVar5 + -0xc),*(POINT *)(puVar5 + -8));
        if (BVar2 != 0) {
          if (param_4[0xe] == local_10) {
            return 0xffffffff;
          }
          iVar1 = *(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4);
          local_28 = *(int *)(iVar1 + 0x6c + local_38 * 8);
          local_24 = *(int *)(iVar1 + 0x70 + local_38 * 8);
          if (*(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xe4 + local_38 * 4)
              == -3) {
            param_4[5] = 0;
            param_4[6] = local_10;
            param_4[7] = local_38;
          }
          else {
            if ((*param_4 == 2) &&
               (param_4[3] ==
                *(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xe4 + local_38 * 4)
               )) {
              return 0xffffffff;
            }
            param_4[5] = 2;
            param_4[8] = *(int *)(*(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 0xe4 +
                                 local_38 * 4);
          }
          if (*(int *)(local_3c + 0x2348) == 0) {
            param_3[1] = local_24;
          }
          else {
            *param_3 = local_28;
          }
          *(int *)((int)piVar4 + -4) = local_24;
          *(int *)((int)piVar4 + -8) = local_28;
          *(int *)((int)piVar4 + -0xc) = param_3[1];
          *(int *)((int)piVar4 + -0x10) = *param_3;
          *(undefined4 *)((int)piVar4 + -0x14) = 0x42f69c;
          FUN_0043b238(param_4,*(int *)((int)piVar4 + -0x10),*(int *)((int)piVar4 + -0xc),
                       *(int *)((int)piVar4 + -8),*(int *)((int)piVar4 + -4));
          return 1;
        }
      }
      *(int *)((int)piVar4 + -4) = param_2;
      *(int *)((int)piVar4 + -8) = param_1;
      *(int *)((int)piVar4 + -0xc) = *(int *)(*(int *)(local_3c + 0x16cc) + local_10 * 4) + 200;
      piVar7 = (int *)((int)piVar4 + -0x10);
      *(undefined4 *)((int)piVar4 + -0x10) = 0x42f6ca;
      BVar2 = PtInRect(*(RECT **)((int)piVar4 + -0xc),*(POINT *)((int)piVar4 + -8));
      if (BVar2 != 0) {
        return 0xffffffff;
      }
    }
  }
  for (local_10 = 0; local_10 < *(int *)(local_3c + 0x16c8); local_10 = local_10 + 1) {
    if ((*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x3c) != 0) &&
       (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x40) == 0)) {
      if (**(int **)(*(int *)(local_3c + 0x16d0) + local_10 * 4) == -3) {
        local_8 = 0;
      }
      else {
        local_8 = *(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x28) + -1;
      }
      local_18 = *(undefined4 *)
                  (local_8 * 0x14 +
                  *(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x2c));
      local_20 = local_18;
      local_14 = *(undefined4 *)
                  (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x2c) + 4 +
                  local_8 * 0x14);
      local_1c = local_14;
      *(undefined4 *)((int)piVar7 + -4) = 6;
      *(undefined4 *)((int)piVar7 + -8) = 6;
      *(undefined4 **)((int)piVar7 + -0xc) = &local_20;
      puVar6 = (undefined1 *)((int)piVar7 + -0x10);
      *(undefined4 *)((int)piVar7 + -0x10) = 0x42f7ad;
      InflateRect(*(LPRECT *)((int)piVar7 + -0xc),*(int *)((int)piVar7 + -8),
                  *(int *)((int)piVar7 + -4));
      *(int *)(puVar6 + -4) = param_2;
      *(int *)(puVar6 + -8) = param_1;
      *(undefined4 **)(puVar6 + -0xc) = &local_20;
      piVar7 = (int *)(puVar6 + -0x10);
      *(undefined4 *)(puVar6 + -0x10) = 0x42f7bd;
      BVar2 = PtInRect(*(RECT **)(puVar6 + -0xc),*(POINT *)(puVar6 + -8));
      if (BVar2 != 0) {
        if ((*param_4 == 2) && (param_4[3] == local_10)) {
          return 0xffffffff;
        }
        if ((param_4[0xe] != -3) &&
           (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x38) != -3)) {
          return 0xffffffff;
        }
        param_5[0xd] = 1;
        param_5[6] = local_10;
        param_4[5] = 2;
        param_4[8] = local_10;
        if (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x38) != -3) {
          param_4[0xe] = *(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x38);
        }
        iVar1 = *(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x2c);
        local_28 = *(int *)(iVar1 + local_8 * 0x14);
        local_24 = *(int *)(iVar1 + 4 + local_8 * 0x14);
        if (*(int *)(local_3c + 0x2348) == 0) {
          param_3[1] = local_24;
        }
        else {
          *param_3 = local_28;
        }
        *(int *)((int)piVar7 + -4) = local_24;
        *(int *)((int)piVar7 + -8) = local_28;
        *(int *)((int)piVar7 + -0xc) = param_3[1];
        *(int *)((int)piVar7 + -0x10) = *param_3;
        *(undefined4 *)((int)piVar7 + -0x14) = 0x42f8b3;
        FUN_0043b238(param_4,*(int *)((int)piVar7 + -0x10),*(int *)((int)piVar7 + -0xc),
                     *(int *)((int)piVar7 + -8),*(int *)((int)piVar7 + -4));
        return 1;
      }
    }
  }
  if (*(int *)(local_3c + 0x16c8) == 0) {
    return 0;
  }
  *(int *)((int)piVar7 + -4) = param_1 - *param_3;
  *(undefined4 *)((int)piVar7 + -8) = 0x42f8de;
  iVar1 = FUN_0043f3b8(*(int *)((int)piVar7 + -4));
  if (iVar1 < 0xb) {
    *(int *)((int)piVar7 + -4) = param_2 - param_3[1];
    *(undefined4 *)((int)piVar7 + -8) = 0x42f8f3;
    iVar1 = FUN_0043f3b8(*(int *)((int)piVar7 + -4));
    if (iVar1 < 0xb) {
      local_c = 1;
      local_28 = *param_3;
      local_24 = param_3[1];
      if (param_3[1] == *(int *)(param_4[0xb] + 4 + (param_4[10] + -1) * 0x14)) {
        local_30 = 1;
      }
      goto LAB_0042f952;
    }
  }
  local_28 = param_1;
  local_24 = param_2;
  if (*(int *)(local_3c + 0x2348) == 0) {
    local_30 = 1;
  }
LAB_0042f952:
  *param_5 = local_28;
  param_5[1] = local_24;
  if (local_30 == 0) {
    param_5[4] = 1;
  }
  else {
    param_5[4] = 0;
  }
  for (local_10 = 0; local_10 < *(int *)(local_3c + 0x16c8); local_10 = local_10 + 1) {
    if (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x40) == 0) {
      param_5[6] = local_10;
      *(int **)((int)piVar7 + -4) = param_5;
      this = *(void **)(*(int *)(local_3c + 0x16d0) + local_10 * 4);
      *(undefined4 *)((int)piVar7 + -8) = 0x42f9cb;
      local_34 = FUN_0043c34c(this,*(int **)((int)piVar7 + -4));
      if (local_34 != 0) break;
    }
  }
  if (local_34 == 0) {
    return 0;
  }
  if ((*param_4 == 2) && (param_4[3] == local_10)) {
    return 0xffffffff;
  }
  if ((param_4[0xe] != -3) &&
     (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x38) != -3)) {
    return 0xffffffff;
  }
  if (local_c == 0) {
    *(int *)((int)piVar7 + -4) = param_5[3];
    *(int *)((int)piVar7 + -8) = param_5[2];
    *(int *)((int)piVar7 + -0xc) = param_3[1];
    *(int *)((int)piVar7 + -0x10) = *param_3;
    *(undefined4 *)((int)piVar7 + -0x14) = 0x42fa52;
    FUN_0043b238(param_4,*(int *)((int)piVar7 + -0x10),*(int *)((int)piVar7 + -0xc),
                 *(int *)((int)piVar7 + -8),*(int *)((int)piVar7 + -4));
  }
  else {
    *(int *)((int)piVar7 + -4) = param_5[3];
    *(int *)((int)piVar7 + -8) = param_5[2];
    *(undefined4 *)((int)piVar7 + -0xc) = 0x42fa37;
    FUN_0043ac51(param_4,*(int *)((int)piVar7 + -8),*(int *)((int)piVar7 + -4));
  }
  param_4[5] = 2;
  param_4[8] = local_10;
  if (*(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x38) != -3) {
    param_4[0xe] = *(int *)(*(int *)(*(int *)(local_3c + 0x16d0) + local_10 * 4) + 0x38);
  }
  return 1;
}
