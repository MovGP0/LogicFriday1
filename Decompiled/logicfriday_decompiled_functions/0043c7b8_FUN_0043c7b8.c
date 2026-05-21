/* 0043c7b8 FUN_0043c7b8 */

int __thiscall FUN_0043c7b8(void *this,int param_1,int param_2)

{
  int iVar1;
  
  if ((*(int *)(*(int *)((int)this + 0x2c) + 0xc + param_1 * 0x14) == 0) ||
     (*(int *)(*(int *)((int)this + 0x2c) + 0xc + param_1 * 0x14) == 1)) {
    if (*(int *)(*(int *)((int)this + 0x2c) + 0xc + param_1 * 0x14) == param_2) {
      if (param_1 + 1 < *(int *)((int)this + 0x28)) {
        iVar1 = param_1 + 1;
      }
      else {
        iVar1 = -3;
      }
    }
    else if (param_1 + -1 < 0) {
      iVar1 = -3;
    }
    else {
      iVar1 = param_1 + -1;
    }
  }
  else {
    iVar1 = -3;
  }
  return iVar1;
}
