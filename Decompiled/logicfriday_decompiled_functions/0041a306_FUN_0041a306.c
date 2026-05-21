/* 0041a306 FUN_0041a306 */

bool __thiscall FUN_0041a306(void *this,WPARAM param_1)

{
  LRESULT LVar1;
  
  LVar1 = SendMessageA(*(HWND *)((int)this + 0x14),0x102c,param_1,2);
  return LVar1 == 2;
}
