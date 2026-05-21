/* 0043c5dc FUN_0043c5dc */

int __thiscall FUN_0043c5dc(void *this,int param_1)

{
  int local_8;
  
  for (local_8 = 0;
      (local_8 < *(int *)((int)this + 0x28) &&
      (*(int *)(*(int *)((int)this + 0x2c) + 0x10 + local_8 * 0x14) !=
       *(int *)(*(int *)((int)this + 0x34) + 8 + param_1 * 0x14))); local_8 = local_8 + 1) {
  }
  if (*(int *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x2c)) <
      *(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c))) {
    local_8 = local_8 + 1;
  }
  return local_8;
}
