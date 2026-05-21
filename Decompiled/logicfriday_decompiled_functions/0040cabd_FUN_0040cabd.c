/* 0040cabd FUN_0040cabd */

undefined4 FUN_0040cabd(HWND param_1,int param_2,uint param_3,LPARAM param_4)

{
  HDC hdc;
  uint uVar1;
  undefined4 uVar2;
  tagRECT local_2c;
  tagRECT local_1c;
  int local_c;
  HANDLE local_8;
  
  local_8 = (HANDLE)0x0;
  if (param_2 == 0x110) {
    GetWindowRect(DAT_00452aac,&local_1c);
    GetWindowRect(param_1,&local_2c);
    hdc = GetDC(DAT_00452aac);
    local_c = GetDeviceCaps(hdc,8);
    ReleaseDC(DAT_00452aac,hdc);
    OffsetRect(&local_2c,-local_2c.left,-local_2c.top);
    local_2c.top = local_1c.bottom - local_2c.bottom;
    if (local_c < local_1c.right + local_2c.right) {
      local_2c.left = local_c - local_2c.right;
    }
    else {
      local_2c.left = local_1c.right;
    }
    MoveWindow(param_1,local_2c.left,local_2c.top,local_2c.right,local_2c.bottom,1);
    local_8 = LoadImageA(DAT_00452914,(LPCSTR)0xd7,1,0x10,0x10,0);
    SendDlgItemMessageA(param_1,0x42c,0xf7,1,(LPARAM)local_8);
    local_8 = LoadImageA(DAT_00452914,(LPCSTR)0x133,1,0x10,0x10,0);
    SendDlgItemMessageA(param_1,0x42b,0xf7,1,(LPARAM)local_8);
    uVar2 = 1;
  }
  else if (param_2 == 0x111) {
    uVar1 = param_3 & 0xffff;
    if (uVar1 != 0) {
      if (uVar1 < 3) {
        PostMessageA(DAT_00452a98,0x100,0x1b,0);
        return 1;
      }
      if (uVar1 == 0x428) {
        SendMessageA(DAT_00452aac,0x111,0x801b,0);
        return 1;
      }
      if (uVar1 == 0x42b) {
        SendMessageA(DAT_00452a98,0x111,param_3,param_4);
        return 1;
      }
      if (uVar1 == 0x42c) {
        SendMessageA(DAT_00452a98,0x111,param_3,param_4);
        return 1;
      }
      if (uVar1 == 0x458) {
        SendMessageA(DAT_00452a98,0x100,0xd,0);
        return 1;
      }
      if (uVar1 == 0x45a) {
        PostMessageA(DAT_00452a98,0x100,0x1b,0);
        return 1;
      }
    }
    SendMessageA(DAT_00452a98,0x111,param_3,param_4);
    uVar2 = 1;
  }
  else {
    uVar2 = 0;
  }
  return uVar2;
}
