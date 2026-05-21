/* 00417acd FUN_00417acd */

undefined4 __thiscall FUN_00417acd(void *this,HWND param_1,undefined4 *param_2,undefined4 *param_3)

{
  HWND pHVar1;
  undefined4 uVar2;
  
  *(HWND *)((int)this + 0xc) = param_1;
  pHVar1 = CreateWindowExA(0x200,"SysListView32","",0x40001001,0,0,0,0,param_1,(HMENU)0x66,
                           DAT_00452914,(LPVOID)0x0);
  *(HWND *)((int)this + 0x10) = pHVar1;
  pHVar1 = CreateWindowExA(0x200,"SysListView32","",0x40001001,0,0,0,0,param_1,(HMENU)0x65,
                           DAT_00452914,(LPVOID)0x0);
  *(HWND *)((int)this + 0x14) = pHVar1;
  if ((*(int *)((int)this + 0x10) == 0) || (*(int *)((int)this + 0x14) == 0)) {
    uVar2 = 0;
  }
  else {
    *param_2 = *(undefined4 *)((int)this + 0x10);
    *param_3 = *(undefined4 *)((int)this + 0x14);
    *(undefined4 *)((int)this + 0x74) = 0;
    *(undefined4 *)((int)this + 0x7c) = 0;
    *(undefined4 *)((int)this + 0x80) = 0;
    _memset((void *)((int)this + 0x20),0,0x20);
    *(undefined4 *)((int)this + 0x20) = 2;
    *(undefined4 *)((int)this + 0x28) = 0x19;
    SendMessageA(*(HWND *)((int)this + 0x10),0x101b,0,(int)this + 0x20);
    SendMessageA(*(HWND *)((int)this + 0x14),0x101b,0,(int)this + 0x20);
    *(undefined4 *)((int)this + 0x68) = 1;
    *(undefined4 *)((int)this + 0x6c) = 1;
    SendMessageA(*(HWND *)((int)this + 0x14),0x1036,0,0x21);
    SendMessageA(*(HWND *)((int)this + 0x10),0x1036,0,1);
    *(undefined4 *)((int)this + 0x8c) = 0;
    *(undefined4 *)((int)this + 0x90) = 0;
    *(undefined4 *)((int)this + 0x94) = 0;
    *(undefined4 *)((int)this + 0x98) = 0;
    uVar2 = 1;
  }
  return uVar2;
}
