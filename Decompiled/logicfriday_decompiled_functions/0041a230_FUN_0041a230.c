/* 0041a230 FUN_0041a230 */

void __thiscall FUN_0041a230(void *this,LRESULT *param_1)

{
  LRESULT LVar1;
  
  LVar1 = SendMessageA(*(HWND *)((int)this + 0x10),0x1027,0,0);
  *param_1 = LVar1;
  return;
}
