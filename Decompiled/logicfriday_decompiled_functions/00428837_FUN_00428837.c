/* 00428837 FUN_00428837 */

undefined4 __thiscall
FUN_00428837(void *this,int param_1,int param_2,int param_3,int param_4,int param_5,int *param_6,
            int *param_7)

{
  int iVar1;
  int iVar2;
  bool bVar3;
  bool bVar4;
  int iVar5;
  int iVar6;
  undefined4 uVar7;
  int local_54;
  int local_48;
  int local_40;
  int local_3c;
  int local_34;
  int local_24;
  int local_20;
  int local_1c;
  int local_10;
  int local_c;
  int local_8;
  
  local_24 = 0x40000000;
  local_20 = -1;
  bVar4 = false;
  iVar6 = *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_1 * 0xfc);
  local_1c = 0;
  do {
    if (*(int *)((int)this + 0x16c8) <= local_1c) {
      if ((param_2 == *(int *)(**(int **)((int)this + 0x2678) + 0x1c + (param_5 + 1) * 0x48)) &&
         (param_3 != *(int *)(*(int *)((int)this + 0x3a4) + 0xb0 + param_1 * 0xfc))) {
        if (param_4 < iVar6) {
          local_c = *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_1 * 0xfc);
          local_48 = FUN_00428eb1(this,param_4,local_c,param_5,1,0);
          local_3c = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_4 * 4) + 0x20);
          local_8 = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_c * 4) + 0x14) * 0xf +
                    *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_c * 4) + 0x24);
        }
        else if (*(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_1 * 0xfc) == 0) {
          local_c = *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_1 * 0xfc);
          local_48 = FUN_00428eb1(this,param_4,local_c,param_5,1,0);
          local_3c = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_c * 4) + 0x20);
          local_8 = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_c * 4) + 0x14) * 0xf +
                    *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_4 * 4) + 0x24);
        }
        else {
          local_c = *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_1 * 0xfc) + -1;
          local_48 = FUN_00428eb1(this,param_4,local_c,param_5,1,0);
          local_3c = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_c * 4) + 0x20);
          local_8 = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_4 * 4) + 0x14) * 0xf +
                    *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_4 * 4) + 0x24);
        }
        if ((local_48 != 0) && ((local_20 == -1 || ((local_8 - local_3c) / 2 < local_24)))) {
          local_34 = param_2 + local_48 * -0xf;
          for (local_1c = 0; local_1c < *(int *)((int)this + 0x16c8); local_1c = local_1c + 1) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x38) == param_1) {
              for (local_40 = 0;
                  local_40 <
                  *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x28) + -1;
                  local_40 = local_40 + 1) {
                if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x2c)
                             + 0xc + local_40 * 0x14) == 1) {
                  bVar3 = false;
                  iVar6 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x2c);
                  iVar5 = *(int *)(iVar6 + local_40 * 0x14);
                  iVar6 = *(int *)(iVar6 + 4 + local_40 * 0x14);
                  if ((local_3c <= iVar6) && (iVar6 <= local_8)) {
                    iVar1 = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4)
                                             + 0x2c) + (local_40 + 1) * 0x14);
                    if ((iVar5 == local_34) || (iVar1 == local_34)) {
                      bVar3 = true;
                    }
                    else if ((iVar5 < local_34) && (local_34 < iVar1)) {
                      bVar3 = true;
                    }
                    else if ((local_34 < iVar5) && (iVar1 < local_34)) {
                      bVar3 = true;
                    }
                    if (bVar3) {
                      iVar6 = FUN_0043f3b8(param_3 - iVar6);
                      iVar6 = iVar6 + local_48 * 0xf;
                      if (iVar6 < local_24) {
                        bVar4 = false;
                        local_20 = local_1c;
                        local_10 = local_40;
                        local_24 = iVar6;
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
      if (local_20 == -1) {
        uVar7 = 0;
      }
      else {
        if (bVar4) {
          *param_6 = *(int *)(local_10 * 0x14 +
                             *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_20 * 4) + 0x2c))
          ;
          if (((param_5 != 0) &&
              (*param_6 < *(int *)(**(int **)((int)this + 0x2678) + 0x1c + (param_5 + -1) * 0x48)))
             && (param_3 != *(int *)(*(int *)((int)this + 0x3a4) + 0xb0 + param_1 * 0xfc))) {
            return 0;
          }
          param_6[1] = param_3;
        }
        else {
          FUN_00428eb1(this,param_4,local_c,param_5,1,1);
          *param_6 = local_34;
          param_6[1] = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_20 * 4) +
                                        0x2c) + 4 + local_10 * 0x14);
        }
        *param_7 = local_20;
        uVar7 = 1;
      }
      return uVar7;
    }
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x38) == param_1) {
      for (local_40 = 0;
          local_40 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x28) + -1;
          local_40 = local_40 + 1) {
        if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x2c) + 0xc +
                    local_40 * 0x14) == 0) {
          bVar3 = false;
          iVar5 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x2c);
          iVar1 = *(int *)(iVar5 + local_40 * 0x14);
          iVar5 = *(int *)(iVar5 + 4 + local_40 * 0x14);
          if (iVar1 < param_2) {
            iVar2 = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_1c * 4) + 0x2c)
                             + 4 + (local_40 + 1) * 0x14);
            if ((iVar5 == param_3) || (iVar2 == param_3)) {
              bVar3 = true;
            }
            else if ((iVar5 < param_3) && (param_3 < iVar2)) {
              bVar3 = true;
            }
            else if ((param_3 < iVar5) && (iVar2 < param_3)) {
              bVar3 = true;
            }
            if ((((bVar3) &&
                 (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_4 * 4) + 0x20) < param_3))
                && (param_3 < *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_4 * 4) + 0x24))
                ) && (iVar1 < *(int *)(**(int **)((int)this + 0x2678) + 0x1c + param_5 * 0x48))) {
              for (local_54 = param_5;
                  iVar1 < *(int *)(**(int **)((int)this + 0x2678) + 0x1c + local_54 * 0x48);
                  local_54 = local_54 + -1) {
                if (*(int *)(local_54 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + param_4 * 4))
                    != -1) {
                  bVar3 = false;
                  break;
                }
              }
            }
            if ((bVar3) && (iVar5 = FUN_0043f3b8(param_2 - iVar1), iVar5 < local_24)) {
              bVar4 = true;
              local_20 = local_1c;
              local_10 = local_40;
              local_24 = iVar5;
            }
          }
        }
      }
    }
    local_1c = local_1c + 1;
  } while( true );
}
