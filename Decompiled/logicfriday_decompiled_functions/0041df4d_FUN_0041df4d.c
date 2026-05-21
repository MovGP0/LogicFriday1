/* 0041df4d FUN_0041df4d */

void __thiscall FUN_0041df4d(void *this,undefined4 param_1,undefined4 param_2)

{
  *(undefined4 *)((int)this + 0x16f0) = param_1;
  *(undefined4 *)((int)this + 0x16f4) = param_2;
  SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x8007,(int)this + 0x17e4);
  FUN_0043ed39((char *)((int)this + 0x21e8),(byte *)"%s\\user.genlib");
  *(undefined4 *)((int)this + 0x17e0) = 0;
  *(undefined4 *)((int)this + 0x17dc) = 0;
  *(undefined4 *)((int)this + 0x17e0) = 0;
  *(undefined4 *)((int)this + 0x2318) = 0;
  *(undefined4 *)((int)this + 0x23cc) = 0;
  return;
}
