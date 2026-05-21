/* 00436c32 FUN_00436c32 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_00436c32(void *this,HWND param_1,int param_2,ushort param_3)

{
  HDC hdc;
  undefined4 uVar1;
  uint unaff_retaddr;
  tagRECT local_c0;
  tagRECT local_b0;
  HWND local_a0;
  char local_9c [32];
  uint local_7c;
  tagRECT local_78;
  int local_68;
  tagRECT local_64;
  uint local_54 [17];
  int local_10;
  int local_c;
  HWND local_8;
  
  local_7c = DAT_00451a00 ^ unaff_retaddr;
  local_54[0] = 0x45d;
  local_54[1] = 0x45e;
  local_54[2] = 0x45f;
  local_54[3] = 0x460;
  local_54[4] = 0x461;
  local_54[5] = 0x462;
  local_54[6] = 0x463;
  local_54[7] = 0x464;
  local_54[8] = 0x465;
  local_54[9] = 0x466;
  local_54[10] = 0x468;
  local_54[0xb] = 0x467;
  local_54[0xc] = 0x469;
  local_54[0xd] = 0x46a;
  local_54[0xe] = 0x46b;
  local_54[0xf] = 0x46c;
  if (param_2 == 2) {
    GetWindowRect(param_1,(LPRECT)((int)this + 0x26f0));
    uVar1 = 1;
  }
  else if (param_2 == 0x110) {
    if (*(uint *)((int)this + 0xc4) < 0x10) {
      for (local_68 = *(int *)((int)this + 0xc4); local_68 < 0x10; local_68 = local_68 + 1) {
        local_8 = GetDlgItem(param_1,local_54[local_68]);
        DestroyWindow(local_8);
      }
      local_8 = GetDlgItem(param_1,0x46e);
      local_a0 = GetDlgItem(param_1,0x428);
      GetWindowRect(local_8,&local_c0);
      GetWindowRect(local_a0,&local_78);
      local_a0 = GetDlgItem(param_1,local_54[*(int *)((int)this + 0xc4) + -1]);
      GetWindowRect(local_a0,&local_78);
      local_10 = (local_c0.top - local_78.bottom) - (local_78.top - local_c0.bottom);
      FUN_004372d9(param_1,0x46e,local_10);
      FUN_004372d9(param_1,0x428,local_10);
      FUN_004372d9(param_1,1,local_10);
      FUN_004372d9(param_1,0x48f,local_10);
      GetWindowRect(param_1,&local_64);
      MoveWindow(param_1,local_64.left,local_64.top,local_64.right - local_64.left,
                 (local_64.bottom - local_64.top) - local_10,1);
    }
    if (*(int *)((int)this + 0x26f0) == -10000) {
      GetWindowRect(*(HWND *)((int)this + 0x16f0),&local_b0);
      GetWindowRect(param_1,&local_64);
      hdc = GetDC(*(HWND *)((int)this + 0x16f0));
      local_c = GetDeviceCaps(hdc,8);
      ReleaseDC(*(HWND *)((int)this + 0x16f0),hdc);
      OffsetRect(&local_64,-local_64.left,-local_64.top);
      local_64.top = local_b0.bottom - local_64.bottom;
      if (local_c < local_b0.right + local_64.right) {
        local_64.left = local_c - local_64.right;
      }
      else {
        local_64.left = local_b0.right;
      }
      MoveWindow(param_1,local_64.left,local_64.top,local_64.right,local_64.bottom,1);
      *(LONG *)((int)this + 0x26f0) = local_64.left;
      *(LONG *)((int)this + 0x26f4) = local_64.top;
      *(LONG *)((int)this + 0x26f8) = local_64.right;
      *(LONG *)((int)this + 0x26fc) = local_64.bottom;
    }
    else {
      MoveWindow(param_1,*(int *)((int)this + 0x26f0),*(int *)((int)this + 0x26f4),
                 *(int *)((int)this + 0x26f8) - *(int *)((int)this + 0x26f0),
                 *(int *)((int)this + 0x26fc) - *(int *)((int)this + 0x26f4),1);
    }
    if (*(int *)(*(int *)((int)this + 0x3a4) + 0x14) == -3) {
      for (local_68 = 0; local_68 < *(int *)((int)this + 0x1654); local_68 = local_68 + 1) {
        *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0x14 + local_68 * 0xfc) = 0;
      }
    }
    for (local_68 = 0; local_68 < *(int *)((int)this + 0x1654); local_68 = local_68 + 1) {
      FUN_0043ed39(local_9c,(byte *)"%s = %d");
      SetDlgItemTextA(param_1,local_54[local_68],local_9c);
    }
    *(undefined4 *)((int)this + 0x16bc) = 0;
    PostMessageA(param_1,0x111,0x428,0);
    uVar1 = 1;
  }
  else {
    if (param_2 == 0x111) {
      if (param_3 != 0) {
        if (param_3 < 3) {
          PostMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x152,0);
          return 1;
        }
        if (param_3 == 0x428) {
          *(undefined4 *)((int)this + 0x16bc) = 0;
          FUN_00437348((int)this);
          FUN_004373f1(this,0);
          InvalidateRect(*(HWND *)((int)this + 0x16f4),(RECT *)0x0,1);
          UpdateWindow(*(HWND *)((int)this + 0x16f4));
          return 1;
        }
        if (param_3 == 0x48f) {
          SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x801c,0);
          return 1;
        }
      }
      for (local_68 = 0; local_68 < *(int *)((int)this + 0x1654); local_68 = local_68 + 1) {
        if ((uint)param_3 == local_54[local_68]) {
          *(uint *)(*(int *)((int)this + 0x3a4) + 0x14 + local_68 * 0xfc) =
               (uint)(*(int *)(*(int *)((int)this + 0x3a4) + 0x14 + local_68 * 0xfc) == 0);
          FUN_0043ed39(local_9c,(byte *)"%s = %d");
          SetDlgItemTextA(param_1,local_54[local_68],local_9c);
          if (*(int *)((int)this + 0x16bc) == 0) {
            *(undefined4 *)((int)this + 0x16bc) = 1;
            FUN_004373f1(this,0);
            InvalidateRect(*(HWND *)((int)this + 0x16f4),(RECT *)0x0,1);
            UpdateWindow(*(HWND *)((int)this + 0x16f4));
          }
          return 1;
        }
      }
    }
    uVar1 = 0;
  }
  return uVar1;
}
