/* 00439623 FUN_00439623 */

bool __thiscall FUN_00439623(void *this,undefined4 param_1,undefined4 *param_2)

{
  HWND pHVar1;
  bool bVar2;
  void *local_8;
  
  *(undefined4 *)this = param_1;
  local_8 = this;
  if (DAT_00452e70 == 0) {
    pHVar1 = CreateWindowExA(0x200,"RichEdit",(LPCSTR)0x0,0x54200004,0,0,0,0,*(HWND *)this,
                             (HMENU)0x64,DAT_00452914,(LPVOID)0x0);
    *(HWND *)((int)this + 4) = pHVar1;
  }
  else {
    pHVar1 = CreateWindowExA(0x200,"RichEdit20A",(LPCSTR)0x0,0x54200004,-0x80000000,-0x80000000,
                             -0x80000000,-0x80000000,*(HWND *)this,(HMENU)0x64,DAT_00452914,
                             (LPVOID)0x0);
    *(HWND *)((int)this + 4) = pHVar1;
  }
  bVar2 = *(int *)((int)this + 4) != 0;
  if (bVar2) {
    SendMessageA(*(HWND *)((int)this + 4),0x445,0,0x80000);
    FUN_0043ebd0((uint *)((int)this + 0x5c),(uint *)&DAT_0044ad26);
    *(undefined4 *)((int)this + 0x14) = 0x3c;
    *(undefined4 *)((int)this + 0x18) = 0xa8000001;
    *(undefined4 *)((int)this + 0x20) = 200;
    *(undefined1 *)((int)this + 0x2c) = 1;
    FUN_0043ebd0((uint *)((int)this + 0x2e),(uint *)"Courier New");
    SendMessageA(*(HWND *)((int)this + 4),0x444,4,(int)this + 0x14);
    SendMessageA(*(HWND *)((int)this + 4),0xcf,1,0);
    SendMessageA(*(HWND *)((int)this + 4),0xc5,0x7ffe,0);
    local_8 = (void *)0x28;
    SendMessageA(*(HWND *)((int)this + 4),0xcb,1,(LPARAM)&local_8);
    SendMessageA(*(HWND *)((int)this + 4),0x459,1,0);
    *param_2 = *(undefined4 *)((int)this + 4);
  }
  return bVar2;
}
