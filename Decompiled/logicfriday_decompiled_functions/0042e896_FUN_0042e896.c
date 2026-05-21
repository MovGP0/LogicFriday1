/* 0042e896 FUN_0042e896 */

undefined4 __thiscall FUN_0042e896(void *this,int param_1)

{
  undefined4 uVar1;
  int local_c;
  int local_8;
  
  if ((*(int *)(param_1 + 0xc4) == *(int *)((int)this + 0xc4)) &&
     (*(int *)(param_1 + 200) == *(int *)((int)this + 200))) {
    for (local_c = 0; local_c < *(int *)((int)this + 200); local_c = local_c + 1) {
      for (local_8 = 0; local_8 < *(int *)this; local_8 = local_8 + 1) {
        if ((*(int *)(*(int *)((int)this + local_c * 4 + 0x84) + local_8 * 4) !=
             *(int *)(*(int *)(param_1 + 0x84 + local_c * 4) + local_8 * 4)) &&
           (*(int *)(*(int *)(param_1 + 0x84 + local_c * 4) + local_8 * 4) != 2)) {
          return 0;
        }
      }
    }
    uVar1 = 1;
  }
  else {
    uVar1 = 0;
  }
  return uVar1;
}
