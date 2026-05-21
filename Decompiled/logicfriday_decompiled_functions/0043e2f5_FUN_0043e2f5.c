/* 0043e2f5 FUN_0043e2f5 */

undefined4 __thiscall FUN_0043e2f5(void *this,int param_1,undefined4 param_2)

{
  int local_8;
  
  local_8 = 0;
  while( true ) {
    if (*(int *)((int)this + 0x30) <= local_8) {
      return 0;
    }
    if (*(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) == param_1) break;
    local_8 = local_8 + 1;
  }
  *(undefined4 *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) = param_2;
  return 1;
}
