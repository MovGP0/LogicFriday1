/* 0043ca9b FUN_0043ca9b */

undefined4 __thiscall FUN_0043ca9b(void *this,int param_1,int param_2,int param_3)

{
  bool bVar1;
  undefined4 uVar2;
  int iVar3;
  int local_10;
  int local_c;
  int local_8;
  
  bVar1 = false;
  local_10 = -100;
  local_c = -100;
  if (*(int *)((int)this + 0x30) == 0) {
    uVar2 = 0xffffffff;
  }
  else {
    if (param_2 == 0) {
      if ((*(int *)this == 1) && (*(int *)((int)this + 4) == param_1)) {
        local_10 = **(int **)((int)this + 0x2c);
        local_c = (*(int **)((int)this + 0x2c))[1];
      }
      else if ((*(int *)((int)this + 0x14) == 1) && (*(int *)((int)this + 0x18) == param_1)) {
        iVar3 = (*(int *)((int)this + 0x28) + -1) * 0x14;
        local_10 = *(int *)(*(int *)((int)this + 0x2c) + iVar3);
        local_c = *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar3);
      }
    }
    else if (((*(int *)this == 0) && (*(int *)((int)this + 4) == param_1)) &&
            (*(int *)((int)this + 8) == param_3)) {
      local_10 = **(int **)((int)this + 0x2c);
      local_c = (*(int **)((int)this + 0x2c))[1];
    }
    else if (((*(int *)((int)this + 0x14) == 0) && (*(int *)((int)this + 0x18) == param_1)) &&
            (*(int *)((int)this + 0x1c) == param_3)) {
      iVar3 = (*(int *)((int)this + 0x28) + -1) * 0x14;
      local_10 = *(int *)(*(int *)((int)this + 0x2c) + iVar3);
      local_c = *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar3);
    }
    if (local_10 == -100) {
      uVar2 = 0xffffffff;
    }
    else {
      for (local_8 = 0; local_8 < *(int *)((int)this + 0x30); local_8 = local_8 + 1) {
        if ((*(int *)(local_8 * 0x14 + *(int *)((int)this + 0x34)) == local_10) &&
           (*(int *)(*(int *)((int)this + 0x34) + 4 + local_8 * 0x14) == local_c)) {
          bVar1 = true;
          break;
        }
      }
      if (bVar1) {
        uVar2 = *(undefined4 *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14);
      }
      else {
        uVar2 = 0xffffffff;
      }
    }
  }
  return uVar2;
}
