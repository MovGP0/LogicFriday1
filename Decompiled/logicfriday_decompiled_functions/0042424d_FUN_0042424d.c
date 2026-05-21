/* 0042424d FUN_0042424d */

undefined4 __thiscall FUN_0042424d(void *this,HWND param_1,int param_2,ushort param_3)

{
  undefined4 uVar1;
  undefined4 local_8;
  
  local_8 = 0x4b;
  if (param_2 == 0x110) {
    SendDlgItemMessageA(param_1,0x43e,0xcb,1,(LPARAM)&local_8);
    SetDlgItemTextA(param_1,0x43e,*(LPCSTR *)((int)this + 0x274));
    uVar1 = 1;
  }
  else if (((param_2 == 0x111) && (param_3 != 0)) && (param_3 < 3)) {
    EndDialog(param_1,0);
    DeleteObject(*(HGDIOBJ *)((int)this + 0x2334));
    uVar1 = 1;
  }
  else {
    uVar1 = 0;
  }
  return uVar1;
}
