/* 0043983d FUN_0043983d */

void __thiscall FUN_0043983d(void *this,LPARAM param_1)

{
  SendMessageA(*(HWND *)((int)this + 4),0xb1,0xffffffff,-1);
  SendMessageA(*(HWND *)((int)this + 4),0xc2,0,param_1);
  *(undefined4 *)((int)this + 0x28) = 0;
  *(undefined4 *)((int)this + 0x18) = 0x40000000;
  SendMessageA(*(HWND *)((int)this + 4),0x444,4,(int)this + 0x14);
  SendMessageA(*(HWND *)((int)this + 4),0xcf,1,0);
  FUN_004398bb((int)this);
  return;
}
