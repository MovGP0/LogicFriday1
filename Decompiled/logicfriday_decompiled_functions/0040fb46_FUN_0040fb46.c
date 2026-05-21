/* 0040fb46 FUN_0040fb46 */

undefined4 __thiscall FUN_0040fb46(void *this,int param_1,undefined4 *param_2)

{
  LRESULT LVar1;
  
  *param_2 = 0;
  if ((0xfffffff9 < *(uint *)(param_1 + 8)) && (*(uint *)(param_1 + 8) != 0xffffffff)) {
    LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x1032,0,0);
    if (LVar1 == 0) {
      if (*(int *)((int)this + 0x50) != 0) {
        FUN_0040f300(this,*(WPARAM *)((int)this + 0x54));
      }
      *param_2 = 0;
    }
    else if (LVar1 == 1) {
      *param_2 = 0xfffffffe;
    }
  }
  return 0;
}
