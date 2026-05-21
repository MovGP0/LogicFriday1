/* 00417769 FUN_00417769 */

int __thiscall FUN_00417769(void *this,int param_1,undefined4 param_2)

{
  bool bVar1;
  int iVar2;
  int local_34;
  int local_30;
  int local_20;
  int local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_c;
  
  if ((*(int *)((int)this + 0x14) < 2) && (-4 < *(int *)((int)this + 0x14))) {
    if (*(int *)((int)this + 0x14) == -3) {
      for (local_10 = 0; local_10 < *(int *)((int)this + 0x18); local_10 = local_10 + 1) {
        if (*(int *)((int)this + local_10 * 4 + 0x1c) == -2) {
          *(undefined4 *)((int)this + local_10 * 4 + 0x2c) = 0;
        }
        else if (*(int *)((int)this + local_10 * 4 + 0x1c) == -1) {
          *(undefined4 *)((int)this + local_10 * 4 + 0x2c) = 1;
        }
        else {
          iVar2 = FUN_00417769((void *)(param_1 + *(int *)((int)this + local_10 * 4 + 0x1c) * 0xfc),
                               param_1,param_2);
          *(int *)((int)this + local_10 * 4 + 0x2c) = iVar2;
          if (*(int *)((int)this + local_10 * 4 + 0x2c) == -3) {
            return -3;
          }
        }
      }
      if (*(int *)this == 9) {
        *(undefined4 *)((int)this + 0x14) = *(undefined4 *)((int)this + 0x2c);
      }
      else if (*(int *)this == 0) {
        *(uint *)((int)this + 0x14) = (uint)(*(int *)((int)this + 0x2c) == 0);
      }
      else if (*(int *)this == 1) {
        bVar1 = true;
        for (local_14 = 0; local_14 < *(int *)((int)this + 0x18); local_14 = local_14 + 1) {
          if ((bVar1) && (*(int *)((int)this + local_14 * 4 + 0x2c) != 0)) {
            bVar1 = true;
          }
          else {
            bVar1 = false;
          }
        }
        *(uint *)((int)this + 0x14) = (uint)!bVar1;
      }
      else if (*(int *)this == 2) {
        bVar1 = false;
        for (local_18 = 0; local_18 < *(int *)((int)this + 0x18); local_18 = local_18 + 1) {
          if ((bVar1) || (*(int *)((int)this + local_18 * 4 + 0x2c) != 0)) {
            bVar1 = true;
          }
          else {
            bVar1 = false;
          }
        }
        *(uint *)((int)this + 0x14) = (uint)!bVar1;
      }
      else if (*(int *)this == 6) {
        local_c = 1;
        for (local_1c = 0; local_1c < *(int *)((int)this + 0x18); local_1c = local_1c + 1) {
          if ((local_c == 0) || (*(int *)((int)this + local_1c * 4 + 0x2c) == 0)) {
            local_30 = 0;
          }
          else {
            local_30 = 1;
          }
          local_c = local_30;
        }
        *(int *)((int)this + 0x14) = local_c;
      }
      else if (*(int *)this == 7) {
        local_c = 0;
        for (local_20 = 0; local_20 < *(int *)((int)this + 0x18); local_20 = local_20 + 1) {
          if ((local_c == 0) && (*(int *)((int)this + local_20 * 4 + 0x2c) == 0)) {
            local_34 = 0;
          }
          else {
            local_34 = 1;
          }
          local_c = local_34;
        }
        *(int *)((int)this + 0x14) = local_c;
      }
      else if (*(int *)this == 3) {
        if (((*(int *)((int)this + 0x2c) == 0) || (*(int *)((int)this + 0x30) != 0)) &&
           ((*(int *)((int)this + 0x2c) != 0 || (*(int *)((int)this + 0x30) == 0)))) {
          *(undefined4 *)((int)this + 0x14) = 0;
        }
        else {
          *(undefined4 *)((int)this + 0x14) = 1;
        }
      }
      else if (*(int *)this == 5) {
        if (((*(int *)((int)this + 0x2c) == 0) || (*(int *)((int)this + 0x34) != 0)) &&
           ((*(int *)((int)this + 0x30) == 0 || (*(int *)((int)this + 0x34) == 0)))) {
          *(undefined4 *)((int)this + 0x14) = 0;
        }
        else {
          *(undefined4 *)((int)this + 0x14) = 1;
        }
      }
      iVar2 = *(int *)((int)this + 0x14);
    }
    else {
      iVar2 = *(int *)((int)this + 0x14);
    }
  }
  else {
    iVar2 = -3;
  }
  return iVar2;
}
