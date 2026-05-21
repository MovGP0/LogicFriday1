/* 004305a5 FUN_004305a5 */

undefined4 __thiscall FUN_004305a5(void *this,int *param_1,int param_2)

{
  int local_8;
  
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c4); local_8 = local_8 + 1) {
    if (param_1[2] < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd0) + param_2
       ) {
      param_1[2] = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd0) + param_2;
    }
    if (param_1[3] < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd4) + param_2
       ) {
      param_1[3] = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd4) + param_2;
    }
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 200) - param_2 < *param_1) {
      *param_1 = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 200) - param_2;
    }
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xcc) - param_2 < param_1[1]
       ) {
      param_1[1] = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xcc) - param_2;
    }
  }
  return 0;
}
