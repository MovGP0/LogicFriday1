/* 0043b7be FUN_0043b7be */

undefined4 __thiscall FUN_0043b7be(void *this,int param_1,int param_2,int param_3)

{
  undefined4 local_c;
  undefined4 local_8;
  
  local_c = 0;
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x30); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)((int)this + 0x34) + 8 + local_8 * 0x14) ==
        *(int *)(*(int *)((int)this + 0x2c) + 0x10 + param_1 * 0x14)) {
      if (*(int *)(*(int *)((int)this + 0x2c) + 0xc + param_1 * 0x14) == 0) {
        *(int *)(local_8 * 0x14 + *(int *)((int)this + 0x34)) =
             *(int *)(*(int *)((int)this + 0x34) + local_8 * 0x14) + param_2;
      }
      else {
        *(int *)(*(int *)((int)this + 0x34) + 4 + local_8 * 0x14) =
             *(int *)(*(int *)((int)this + 0x34) + 4 + local_8 * 0x14) + param_3;
      }
      local_c = 1;
    }
  }
  return local_c;
}
