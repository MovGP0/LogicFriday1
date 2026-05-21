/* 004198c3 FUN_004198c3 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004198c3(void *this,int param_1,int param_2)

{
  int iVar1;
  uint unaff_retaddr;
  RECT *local_60;
  RECT *local_5c;
  char local_48 [8];
  uint local_40;
  int local_3c;
  int local_38;
  RECT local_34;
  int local_24;
  int local_1c;
  WPARAM local_18;
  uint local_14;
  int local_10;
  uint local_c;
  int local_8;
  
  local_40 = DAT_00451a00 ^ unaff_retaddr;
  local_8 = param_1;
  local_38 = param_1;
  iVar1 = *(int *)(param_1 + 8);
  if (iVar1 == -0x96) {
    local_18 = *(uint *)(param_1 + 0x10);
    if (local_18 == 0xffffffff) {
      return 0;
    }
    if ((*(uint *)(param_1 + 0xc) & 1) != 0) {
      local_1c = *(int *)(param_1 + 0x14);
      if (local_1c == 0) {
        FUN_0043ed39(local_48,&DAT_0044b960);
        *(char **)(local_8 + 0x20) = local_48;
      }
      else if (*(int *)(param_2 + 0xc4) < local_1c) {
        if ((local_1c == *(int *)(param_2 + 0xc4) + 1) ||
           (*(int *)((int)this + 0x68) + -1 < local_1c)) {
          return 0;
        }
        iVar1 = (local_1c - *(int *)(param_2 + 0xc4)) + -2;
        if (*(int *)(*(int *)(param_2 + 0x84 + iVar1 * 4) + local_18 * 4) == 0) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044bbb0;
        }
        else if (*(int *)(*(int *)(param_2 + 0x84 + iVar1 * 4) + local_18 * 4) == 1) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
        }
        else if (*(int *)(*(int *)(param_2 + 0x84 + iVar1 * 4) + local_18 * 4) == 2) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044add0;
        }
      }
      else if ((local_18 & 1 << ((char)*(undefined4 *)(param_2 + 0xc4) - (char)local_1c & 0x1fU)) ==
               0) {
        *(undefined **)(param_1 + 0x20) = &DAT_0044bbb0;
      }
      else {
        *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
      }
    }
  }
  else {
    if (iVar1 == -0x6c) {
      if (*(int *)(param_1 + 0x10) == *(int *)(param_2 + 0xc4) + 1) {
        return 0;
      }
      if (*(int *)(param_1 + 0x10) == 0) {
        return 0;
      }
      if (*(int *)(param_1 + 0x10) < *(int *)(param_2 + 0xc4) + 1) {
        local_18 = SendMessageA(*(HWND *)((int)this + 0x14),0x1027,0,0);
        if (&stack0x00000000 == (undefined1 *)0x34) {
          local_60 = (RECT *)0x0;
        }
        else {
          local_34.left = 0;
          local_60 = &local_34;
        }
        SendMessageA(*(HWND *)((int)this + 0x14),0x100e,local_18,(LPARAM)local_60);
        local_14 = 1 << ((char)*(undefined4 *)(param_2 + 0xc4) -
                         (char)*(undefined4 *)(local_38 + 0x10) & 0x1fU);
        if ((local_18 & local_14) == 0) {
          local_c = local_18 | local_14;
        }
        else {
          local_c = local_18 & ~local_14;
        }
        local_10 = (local_c - local_18) * (local_34.bottom - local_34.top);
        SendMessageA(*(HWND *)((int)this + 0x14),0x1014,0,local_10);
      }
      else if (1 < *(uint *)(param_2 + 200)) {
        local_3c = (*(int *)(param_1 + 0x10) - *(int *)(param_2 + 0xc4)) + -2;
        *(uint *)((int)this + local_3c * 4 + 0x9c) =
             (uint)(*(int *)((int)this + local_3c * 4 + 0x9c) == 0);
        InvalidateRect(*(HWND *)((int)this + 0x14),(RECT *)0x0,0);
        UpdateWindow(*(HWND *)((int)this + 0x14));
      }
      return 0;
    }
    if (iVar1 == -0xc) {
      iVar1 = *(int *)(param_1 + 0xc);
      if (iVar1 == 1) {
        return 0x20;
      }
      if (iVar1 == 0x10001) {
        return 0x20;
      }
      if (iVar1 != 0x30001) {
        return 0;
      }
      if (*(uint *)(param_1 + 0x38) < *(int *)(param_2 + 0xc4) + 2U) {
        return 0;
      }
      if (*(int *)((int)this + (*(uint *)(param_1 + 0x38) - *(int *)(param_2 + 0xc4)) * 4 + 0x94) !=
          0) {
        *(undefined4 *)(param_1 + 0x30) = 0x99a8ac;
        return 2;
      }
      *(undefined4 *)(param_1 + 0x30) = 0xdf0000;
      return 2;
    }
    if (iVar1 == -4) {
      SendMessageA(*(HWND *)((int)this + 0xc),0x111,0x8002,0);
      return 0;
    }
    if (iVar1 == -3) {
      local_18 = *(WPARAM *)(param_1 + 0xc);
      local_24 = *(int *)(param_1 + 0x10);
      if ((-1 < (int)local_18) && (*(int *)(param_2 + 0xc4) + 2 <= local_24)) {
        local_3c = (local_24 - *(int *)(param_2 + 0xc4)) + -2;
        if (*(int *)((int)this + local_3c * 4 + 0x9c) != 0) {
          return 0;
        }
        if (*(int *)(*(int *)(param_2 + 0x84 + local_3c * 4) + local_18 * 4) == 0) {
          *(undefined4 *)(*(int *)(param_2 + 0x84 + local_3c * 4) + local_18 * 4) = 1;
        }
        else if (*(int *)(*(int *)(param_2 + 0x84 + local_3c * 4) + local_18 * 4) == 1) {
          *(undefined4 *)(*(int *)(param_2 + 0x84 + local_3c * 4) + local_18 * 4) = 2;
        }
        else {
          *(undefined4 *)(*(int *)(param_2 + 0x84 + local_3c * 4) + local_18 * 4) = 0;
        }
        *(undefined4 *)((int)this + 8) = 0;
        *(undefined4 *)((int)this + 4) = 0;
        *(undefined4 *)this = 0;
        if (&stack0x00000000 == (undefined1 *)0x34) {
          local_5c = (RECT *)0x0;
        }
        else {
          local_34.left = 0;
          local_5c = &local_34;
        }
        SendMessageA(*(HWND *)((int)this + 0x14),0x100e,local_18,(LPARAM)local_5c);
        InvalidateRect(*(HWND *)((int)this + 0x14),&local_34,0);
        UpdateWindow(*(HWND *)((int)this + 0x14));
        return 0;
      }
      return 0;
    }
  }
  return 0;
}
