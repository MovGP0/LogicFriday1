/* 00431d3b FUN_00431d3b */

undefined4 __thiscall FUN_00431d3b(void *this,int param_1,int param_2,int param_3,int param_4)

{
  undefined4 local_c;
  undefined4 local_8;
  
  local_c = 0;
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x40) == 0) {
      local_c = FUN_0043c146(*(void **)(*(int *)((int)this + 0x16d0) + local_8 * 4),param_1,param_2,
                             param_3,param_4);
    }
  }
  return local_c;
}
