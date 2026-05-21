/* 00416a26 FUN_00416a26 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __thiscall
FUN_00416a26(void *this,undefined4 param_1,undefined4 param_2,undefined4 *param_3,int param_4,
            undefined4 *param_5)

{
  uint uVar1;
  int iVar2;
  INT_PTR IVar3;
  void *pvVar4;
  uint unaff_retaddr;
  int local_28;
  uint local_20 [3];
  uint local_14;
  int local_10;
  int local_c;
  int local_8;
  
  local_14 = DAT_00451a00 ^ unaff_retaddr;
  local_c = 0;
  *(undefined4 *)((int)this + 0x218) = 0;
  *(undefined4 *)((int)this + 0x214) = 0;
  local_8 = 0;
  local_28 = 0;
  *(undefined4 *)((int)this + 8) = param_1;
  *(undefined4 *)((int)this + 0xc) = param_2;
  iVar2 = FUN_004160ec((int)this);
  if (iVar2 == 0) {
    MessageBoxA(*(HWND *)this,
                "The two functions must have the same number of inputs and only one output.",
                "Function Operation",0);
    iVar2 = 0;
  }
  else {
    iVar2 = FUN_00416134((int)this);
    if (iVar2 == 2) {
      iVar2 = MessageBoxA(*(HWND *)this,
                          "The two functions have different input variable names. Do you want to carry out the operation on the basis of the\ntruth table ordering of input variables?"
                          ,"Function Operation",3);
      if (iVar2 != 6) {
        return 0;
      }
      *(undefined4 *)((int)this + 0x218) = 1;
    }
    else if (iVar2 == 1) {
      IVar3 = DialogBoxParamA(*(HINSTANCE *)((int)this + 4),"NAMESORDERDLG",*(HWND *)this,
                              FUN_0040af4f,0);
      if (IVar3 == 0x40b) {
        local_c = 1;
      }
      else {
        if (IVar3 != 0x40c) {
          return 0;
        }
        *(undefined4 *)((int)this + 0x218) = 1;
      }
    }
    IVar3 = DialogBoxParamA(*(HINSTANCE *)((int)this + 4),"NEWFCNNAME",*(HWND *)this,FUN_0040af89,
                            (LPARAM)local_20);
    if (IVar3 == 2) {
      *param_5 = 0;
      iVar2 = 0;
    }
    else {
      *param_5 = 1;
      iVar2 = FUN_004161ea(this,local_c);
      if (iVar2 == 0) {
        for (local_10 = 0; local_10 < 0x10; local_10 = local_10 + 1) {
          if (param_3[local_10 + 0x21] != 0) {
            _free((void *)param_3[local_10 + 0x21]);
            param_3[local_10 + 0x21] = 0;
          }
        }
        pvVar4 = _malloc(*(int *)((int)this + 0x10) << 2);
        param_3[0x21] = pvVar4;
        if (param_3[0x21] == 0) {
          iVar2 = 0x40011;
        }
        else {
          *param_3 = *(undefined4 *)((int)this + 0x10);
          param_3[0x31] = *(undefined4 *)((int)this + 0xd4);
          param_3[0x32] = *(undefined4 *)((int)this + 0xd8);
          param_3[0x33] = *(undefined4 *)(*(int *)((int)this + 8) + 0xcc);
          FUN_0043ebd0(param_3 + 0x34,local_20);
          _memcpy(param_3 + 0x58,(void *)(*(int *)((int)this + 8) + 0x160),0x80);
          for (local_10 = 0; local_10 < *(int *)((int)this + 0x10); local_10 = local_10 + 1) {
            iVar2 = *(int *)(*(int *)(*(int *)((int)this + 8) + 0x84) + local_10 * 4);
            uVar1 = *(uint *)(*(int *)((int)this + 0x94) + local_10 * 4);
            if (param_4 == 0xbc) {
              if (iVar2 == 0) {
                if (uVar1 == 0) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 0;
                }
                else if (uVar1 == 1) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 1;
                  local_28 = local_28 + 1;
                }
                else if (uVar1 == 2) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                  local_8 = local_8 + 1;
                }
              }
              else if (iVar2 == 1) {
                *(undefined4 *)(param_3[0x21] + local_10 * 4) = 1;
                local_28 = local_28 + 1;
              }
              else if (iVar2 == 2) {
                if (uVar1 == 0) {
LAB_00416e8c:
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                  local_8 = local_8 + 1;
                }
                else if (uVar1 == 1) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 1;
                  local_28 = local_28 + 1;
                }
                else if (uVar1 == 2) goto LAB_00416e8c;
              }
            }
            else if (param_4 == 0xbd) {
              if (iVar2 == 0) {
                *(undefined4 *)(param_3[0x21] + local_10 * 4) = 0;
              }
              else if (iVar2 == 1) {
                if (uVar1 == 0) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 0;
                }
                else if (uVar1 == 1) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 1;
                  local_28 = local_28 + 1;
                }
                else if (uVar1 == 2) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                  local_8 = local_8 + 1;
                }
              }
              else if (iVar2 == 2) {
                if (uVar1 == 0) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 0;
                }
                else if ((uVar1 != 0) && (uVar1 < 3)) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                  local_8 = local_8 + 1;
                }
              }
            }
            else if (param_4 == 0xc0) {
              if (iVar2 == 0) {
                if (uVar1 == 0) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 0;
                }
                else if (uVar1 == 1) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 1;
                  local_28 = local_28 + 1;
                }
                else if (uVar1 == 2) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                  local_8 = local_8 + 1;
                }
              }
              else if (iVar2 == 1) {
                if (uVar1 == 0) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 1;
                  local_28 = local_28 + 1;
                }
                else if (uVar1 == 1) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 0;
                }
                else if (uVar1 == 2) {
                  *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                  local_8 = local_8 + 1;
                }
              }
              else if (iVar2 == 2) {
                *(undefined4 *)(param_3[0x21] + local_10 * 4) = 2;
                local_8 = local_8 + 1;
              }
            }
          }
          param_3[0x11] = local_8;
          param_3[1] = local_28;
          iVar2 = 0;
        }
      }
    }
  }
  return iVar2;
}
