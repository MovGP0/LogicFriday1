/* 0042fa9d FUN_0042fa9d */

int __thiscall FUN_0042fa9d(void *this,int param_1,int param_2)

{
  int iVar1;
  int local_8;
  
  local_8 = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x18);
  while( true ) {
    local_8 = local_8 + -1;
    if (local_8 < 0) {
      return 0;
    }
    iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x1c + local_8 * 4);
    if (iVar1 == param_2) break;
    if ((((**(int **)(*(int *)((int)this + 0x16cc) + iVar1 * 4) != 10) &&
         (**(int **)(*(int *)((int)this + 0x16cc) + iVar1 * 4) != 0xb)) &&
        (**(int **)(*(int *)((int)this + 0x16cc) + iVar1 * 4) != 8)) &&
       (iVar1 = FUN_0042fa9d(this,iVar1,param_2), iVar1 != 0)) {
      return iVar1;
    }
  }
  return param_2;
}
