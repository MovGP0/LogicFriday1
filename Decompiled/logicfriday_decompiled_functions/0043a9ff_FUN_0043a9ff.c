/* 0043a9ff FUN_0043a9ff */

void __thiscall FUN_0043a9ff(void *this,int *param_1)

{
  int local_c;
  int local_8;
  
  local_c = *param_1;
  local_8 = param_1[1];
  if (*(int *)((int)this + 0x6c) == 0) {
    if (local_8 < *(int *)((int)this + 0x50) + 2) {
      local_8 = *(int *)((int)this + 0x50) + 2;
    }
    else if (DAT_00452ef0 == 0) {
      if (*(int *)((int)this + 0x68) + -2 < local_8) {
        local_8 = *(int *)((int)this + 0x68) + -2;
      }
    }
    else if ((*(int *)((int)this + 0x30) - *(int *)((int)this + 0x34)) + -4 < local_8) {
      local_8 = (*(int *)((int)this + 0x30) - *(int *)((int)this + 0x34)) + -4;
    }
  }
  else if (*(int *)((int)this + 0x6c) == 1) {
    if (local_c < *(int *)((int)this + 0x5c) + 2) {
      local_c = *(int *)((int)this + 0x5c) + 2;
    }
    else if (*(int *)((int)this + 100) + -2 < local_c) {
      local_c = *(int *)((int)this + 100) + -2;
    }
  }
  else if (*(int *)((int)this + 0x6c) == 2) {
    if (local_8 < *(int *)((int)this + 0x28) + 4 + *(int *)((int)this + 0x34)) {
      local_8 = *(int *)((int)this + 0x28) + 4 + *(int *)((int)this + 0x34);
    }
    else if (*(int *)((int)this + 0x68) + -2 < local_8) {
      local_8 = *(int *)((int)this + 0x68) + -2;
    }
  }
  *param_1 = local_c;
  param_1[1] = local_8;
  return;
}
