/* 0043bfa3 FUN_0043bfa3 */

undefined4 __thiscall FUN_0043bfa3(void *this,int param_1,int param_2)

{
  int iVar1;
  int local_c;
  
  if (((param_1 != **(int **)((int)this + 0x2c)) ||
      (param_2 != *(int *)(*(int *)((int)this + 0x2c) + 4))) &&
     ((param_1 != *(int *)((*(int *)((int)this + 0x28) + -1) * 0x14 + *(int *)((int)this + 0x2c)) ||
      (param_2 !=
       *(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -1) * 0x14))))) {
    for (local_c = 0; local_c < *(int *)((int)this + 0x28) + -1; local_c = local_c + 1) {
      iVar1 = local_c + 1;
      if (*(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) ==
          *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14)) {
        if (((*(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) == param_2) &&
            ((*(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c)) <= param_1 ||
             (*(int *)(iVar1 * 0x14 + *(int *)((int)this + 0x2c)) <= param_1)))) &&
           ((param_1 <= *(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c)) ||
            (param_1 <= *(int *)(iVar1 * 0x14 + *(int *)((int)this + 0x2c)))))) {
          return 1;
        }
      }
      else if (((*(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c)) == param_1) &&
               ((*(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) <= param_2 ||
                (*(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14) <= param_2)))) &&
              ((param_2 <= *(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) ||
               (param_2 <= *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14))))) {
        return 1;
      }
    }
  }
  return 0;
}
