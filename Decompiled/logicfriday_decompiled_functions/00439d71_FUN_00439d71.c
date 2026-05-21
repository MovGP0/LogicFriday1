/* 00439d71 FUN_00439d71 */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void __thiscall FUN_00439d71(void *this,int param_1,undefined4 param_2)

{
  uint uVar1;
  int iVar2;
  tagRECT local_34;
  tagRECT local_24;
  tagRECT local_14;
  
  GetClientRect(*(HWND *)this,&local_34);
  if (*(int *)((int)this + 0x10) == 0) {
    SetRect(&local_24,0,0,0,0);
  }
  else {
    GetClientRect(*(HWND *)((int)this + 0x10),&local_24);
    MoveWindow(*(HWND *)((int)this + 0x10),0,local_34.bottom - local_24.bottom,local_34.right,
               local_24.bottom,1);
    _DAT_004519e8 = local_34.right + -100;
    SendMessageA(*(HWND *)((int)this + 0x10),0x404,2,0x4519e8);
  }
  if (*(int *)((int)this + 0x20) == 0) {
    SetRect(&local_14,0,0,0,0);
  }
  else {
    SendMessageA(*(HWND *)this,0x421,0,0);
    GetClientRect(*(HWND *)((int)this + 0x20),&local_14);
  }
  SetRect((LPRECT)((int)this + 0x4c),local_34.left,local_34.top + local_14.bottom,local_34.right,
          local_34.bottom - local_24.bottom);
  if (param_1 != 0) {
    *(undefined4 *)((int)this + 0x40) = 0xfffffffc;
    *(undefined4 *)((int)this + 0x3c) = 0xfffffffc;
    *(undefined4 *)((int)this + 0x38) = 0xfffffffc;
    *(undefined4 *)((int)this + 0x70) = param_2;
    if (*(int *)(*(int *)((int)this + 0x70) + 0x167c) == -1) {
      FUN_0043ab21((int)this);
      uVar1 = SendMessageA(*(HWND *)((int)this + 8),0x1040,0xffffffff,-1);
      *(uint *)((int)this + 0x2c) = (uVar1 & 0xffff) + 4;
      if ((*(int *)((int)this + 0x58) - *(int *)((int)this + 0x28)) - *(int *)((int)this + 0x34) <
          (int)(uVar1 >> 0x10)) {
        iVar2 = GetSystemMetrics(2);
        *(int *)((int)this + 0x2c) = iVar2 + 2 + *(int *)((int)this + 0x2c);
      }
      if (local_34.right / 2 < *(int *)((int)this + 0x2c)) {
        *(int *)((int)this + 0x2c) = local_34.right / 2;
      }
      *(undefined4 *)(*(int *)((int)this + 0x70) + 0x167c) = *(undefined4 *)((int)this + 0x28);
      *(undefined4 *)(*(int *)((int)this + 0x70) + 0x1680) = *(undefined4 *)((int)this + 0x2c);
      *(undefined4 *)(*(int *)((int)this + 0x70) + 0x1684) = *(undefined4 *)((int)this + 0x30);
    }
    else {
      *(undefined4 *)((int)this + 0x2c) = *(undefined4 *)(*(int *)((int)this + 0x70) + 0x1680);
      *(undefined4 *)((int)this + 0x30) = *(undefined4 *)(*(int *)((int)this + 0x70) + 0x1684);
    }
  }
  if (DAT_00452ea4 == 0) {
    DAT_0046c518 = 0;
  }
  else {
    if ((((DAT_00452eec == 0) && (DAT_00452e90 == 0)) && (DAT_00452e98 == 0)) && (DAT_00452e94 == 0)
       ) {
      *(undefined4 *)((int)this + 0x70) = 0;
      DAT_0046c518 = *(int *)((int)this + 0x58);
      ShowWindow(*(HWND *)((int)this + 4),0);
      ShowWindow(*(HWND *)((int)this + 8),0);
      ShowWindow(*(HWND *)((int)this + 0xc),0);
      ShowWindow(*(HWND *)((int)this + 0x18),0);
      if (*(int *)((int)this + 0x1c) != 0) {
        ShowWindow(*(HWND *)((int)this + 0x1c),0);
      }
    }
    else {
      DAT_0046c518 = *(int *)((int)this + 0x28);
    }
    ShowWindow(*(HWND *)((int)this + 0x14),5);
    MoveWindow(*(HWND *)((int)this + 0x14),*(int *)((int)this + 0x4c),*(int *)((int)this + 0x50),
               *(int *)((int)this + 0x54) - *(int *)((int)this + 0x4c),
               DAT_0046c518 - *(int *)((int)this + 0x50),1);
    SetRect((LPRECT)((int)this + 0x5c),*(int *)((int)this + 0x4c),
            DAT_0046c518 + *(int *)((int)this + 0x34),*(int *)((int)this + 0x54),
            *(int *)((int)this + 0x58));
  }
  if (DAT_00452e90 == 0) {
    if ((DAT_00452e94 == 0) || (*(int *)((int)this + 0x1c) == 0)) {
      if (DAT_00452e98 == 0) {
        if (DAT_00452eec != 0) {
          ShowWindow(*(HWND *)((int)this + 8),5);
          MoveWindow(*(HWND *)((int)this + 8),*(int *)((int)this + 0x5c),*(int *)((int)this + 0x60),
                     *(int *)((int)this + 0x2c),
                     *(int *)((int)this + 0x68) - *(int *)((int)this + 0x60),1);
          ShowWindow(*(HWND *)((int)this + 0xc),0);
          if (*(int *)((int)this + 0x1c) != 0) {
            ShowWindow(*(HWND *)((int)this + 0x1c),0);
          }
          InvalidateRect(*(HWND *)((int)this + 8),(RECT *)0x0,0);
          UpdateWindow(*(HWND *)((int)this + 8));
          ShowWindow(*(HWND *)((int)this + 4),5);
          if (DAT_00452ef0 == 0) {
            MoveWindow(*(HWND *)((int)this + 4),
                       *(int *)((int)this + 0x2c) + *(int *)((int)this + 0x34),
                       *(int *)((int)this + 0x60),
                       (*(int *)((int)this + 100) - *(int *)((int)this + 0x2c)) -
                       *(int *)((int)this + 0x34),
                       *(int *)((int)this + 0x68) - *(int *)((int)this + 0x60),1);
            ShowWindow(*(HWND *)((int)this + 0x18),0);
          }
          else {
            MoveWindow(*(HWND *)((int)this + 4),
                       *(int *)((int)this + 0x2c) + *(int *)((int)this + 0x34),
                       *(int *)((int)this + 0x60),
                       (*(int *)((int)this + 100) - *(int *)((int)this + 0x2c)) -
                       *(int *)((int)this + 0x34),
                       *(int *)((int)this + 0x30) - *(int *)((int)this + 0x60),1);
            ShowWindow(*(HWND *)((int)this + 0x18),5);
            if (*(int *)((int)this + 0x1c) != 0) {
              ShowWindow(*(HWND *)((int)this + 0x1c),0);
            }
            MoveWindow(*(HWND *)((int)this + 0x18),
                       *(int *)((int)this + 0x2c) + *(int *)((int)this + 0x34),
                       *(int *)((int)this + 0x30) + *(int *)((int)this + 0x34),
                       (*(int *)((int)this + 100) - *(int *)((int)this + 0x2c)) -
                       *(int *)((int)this + 0x34),
                       (*(int *)((int)this + 0x68) - *(int *)((int)this + 0x30)) -
                       *(int *)((int)this + 0x34),1);
            InvalidateRect(*(HWND *)((int)this + 0x18),(RECT *)0x0,0);
            UpdateWindow(*(HWND *)((int)this + 0x18));
          }
          InvalidateRect(*(HWND *)((int)this + 4),(RECT *)0x0,1);
          UpdateWindow(*(HWND *)((int)this + 4));
          FUN_0043a4d3(this);
        }
      }
      else {
        ShowWindow(*(HWND *)((int)this + 4),5);
        SetFocus(*(HWND *)((int)this + 4));
        ShowWindow(*(HWND *)((int)this + 8),0);
        ShowWindow(*(HWND *)((int)this + 0xc),0);
        ShowWindow(*(HWND *)((int)this + 0x18),0);
        if (*(int *)((int)this + 0x1c) != 0) {
          ShowWindow(*(HWND *)((int)this + 0x1c),0);
        }
        if (DAT_00452e98 != 0) {
          MoveWindow(*(HWND *)((int)this + 4),*(int *)((int)this + 0x4c),
                     *(int *)((int)this + 0x28) + *(int *)((int)this + 0x34),
                     *(int *)((int)this + 0x54) - *(int *)((int)this + 0x4c),
                     (*(int *)((int)this + 0x58) - *(int *)((int)this + 0x28)) -
                     *(int *)((int)this + 0x34),1);
        }
      }
    }
    else {
      ShowWindow(*(HWND *)((int)this + 0x1c),5);
      SetFocus(*(HWND *)((int)this + 0x1c));
      ShowWindow(*(HWND *)((int)this + 4),0);
      ShowWindow(*(HWND *)((int)this + 0xc),0);
      ShowWindow(*(HWND *)((int)this + 8),0);
      ShowWindow(*(HWND *)((int)this + 0x18),0);
      MoveWindow(*(HWND *)((int)this + 0x1c),*(int *)((int)this + 0x4c),
                 *(int *)((int)this + 0x28) + *(int *)((int)this + 0x34),
                 *(int *)((int)this + 0x54) - *(int *)((int)this + 0x4c),
                 (*(int *)((int)this + 0x58) - *(int *)((int)this + 0x28)) -
                 *(int *)((int)this + 0x34),1);
      InvalidateRect(*(HWND *)((int)this + 0x1c),(RECT *)0x0,0);
      UpdateWindow(*(HWND *)((int)this + 0x1c));
    }
  }
  else {
    ShowWindow(*(HWND *)((int)this + 0xc),5);
    SetFocus(*(HWND *)((int)this + 0xc));
    ShowWindow(*(HWND *)((int)this + 8),0);
    ShowWindow(*(HWND *)((int)this + 4),0);
    ShowWindow(*(HWND *)((int)this + 0x18),0);
    if (*(int *)((int)this + 0x1c) != 0) {
      ShowWindow(*(HWND *)((int)this + 0x1c),0);
    }
    MoveWindow(*(HWND *)((int)this + 0xc),*(int *)((int)this + 0x4c),
               *(int *)((int)this + 0x28) + *(int *)((int)this + 0x34),
               *(int *)((int)this + 0x54) - *(int *)((int)this + 0x4c),
               (*(int *)((int)this + 0x58) - *(int *)((int)this + 0x28)) -
               *(int *)((int)this + 0x34),1);
    InvalidateRect(*(HWND *)((int)this + 0xc),(RECT *)0x0,0);
    UpdateWindow(*(HWND *)((int)this + 0xc));
  }
  return;
}
