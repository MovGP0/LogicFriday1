/* 0041def5 FUN_0041def5 */

void __thiscall FUN_0041def5(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x16f0) = param_1;
  SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x8007,(int)this + 0x17e4);
  FUN_0043ed39((char *)((int)this + 0x21e8),(byte *)"%s\\user.genlib");
  return;
}
