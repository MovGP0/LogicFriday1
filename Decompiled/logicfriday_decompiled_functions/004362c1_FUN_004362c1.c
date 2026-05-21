/* 004362c1 FUN_004362c1 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004362c1(void *this,HWND param_1,int param_2,HDC param_3,HWND param_4)

{
  HBRUSH pHVar1;
  HWND pHVar2;
  int iVar3;
  uint unaff_retaddr;
  UINT Msg;
  WPARAM wParam;
  LPARAM lParam;
  LOGBRUSH local_128;
  int local_11c;
  uint local_118 [3];
  uint local_10c [64];
  uint local_c;
  int local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 0x110) {
    *(undefined4 *)((int)this + 0x25b4) = 0;
    *(HWND *)((int)this + 0x23c4) = param_4;
    lParam = 0;
    wParam = 8;
    Msg = 0xc5;
    pHVar2 = GetDlgItem(param_1,0x43e);
    SendMessageA(pHVar2,Msg,wParam,lParam);
    local_128.lbColor = GetSysColor(0xf);
    local_128.lbStyle = 0;
    local_128.lbHatch = 0;
    pHVar1 = CreateBrushIndirect(&local_128);
    *(HBRUSH *)((int)this + 0x25b4) = pHVar1;
    FUN_0043ebd0((uint *)((int)this + 0x2654),
                 (uint *)(*(int *)(*(int *)((int)this + 0x16cc) + *(int *)((int)this + 0x23c4) * 4)
                         + 0x50));
    SetDlgItemTextA(param_1,0x43e,(LPCSTR)((int)this + 0x2654));
    return 1;
  }
  if (param_2 == 0x111) {
    if (((uint)param_3 & 0xffff) == 1) {
      GetDlgItemTextA(param_1,0x43e,(LPSTR)local_118,8);
      local_11c = FUN_00436863(this,(char *)local_118);
      if (local_11c != 0) {
        if (local_11c == 1) {
          FUN_0043ed39((char *)local_10c,(byte *)"Error: Name must contain 1 to %d characters.");
        }
        else if (local_11c == 2) {
          FUN_0043ebd0(local_10c,(uint *)"Error: Name already in use.");
        }
        else if (local_11c == 3) {
          FUN_0043ebd0(local_10c,(uint *)"Error: Name must begin with a letter or underscore.");
        }
        else if (100 < local_11c) {
          FUN_0043ed39((char *)local_10c,(byte *)"Illegal character: \'%c\'");
        }
        SetDlgItemTextA(param_1,0x43f,(LPCSTR)local_10c);
        return 1;
      }
      if (**(int **)(*(int *)((int)this + 0x16cc) + *(int *)((int)this + 0x23c4) * 4) == 8) {
        local_11c = 0;
        for (local_8 = 0; local_8 < *(int *)((int)this + 0xc4); local_8 = local_8 + 1) {
          iVar3 = _strcmp((char *)((int)this + local_8 * 9 + 0x160),(char *)((int)this + 0x2654));
          if (iVar3 == 0) {
            FUN_0043ebd0((uint *)((int)this + local_8 * 9 + 0x160),local_118);
            iVar3 = _strcmp((char *)local_118,(char *)((int)this + 0x2654));
            if (iVar3 != 0) {
              *(undefined4 *)((int)this + 0x2668) = 1;
            }
            break;
          }
        }
      }
      else {
        local_11c = 0;
        for (local_8 = 0; local_8 < *(int *)((int)this + 200); local_8 = local_8 + 1) {
          iVar3 = _strcmp((char *)((int)this + local_8 * 9 + 0xd0),(char *)((int)this + 0x2654));
          if (iVar3 == 0) {
            FUN_0043ebd0((uint *)((int)this + local_8 * 9 + 0xd0),local_118);
            FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) +
                                          *(int *)((int)this + 0x23c4) * 4) + 4),local_118);
            iVar3 = _strcmp((char *)local_118,(char *)((int)this + 0x2654));
            if (iVar3 != 0) {
              *(undefined4 *)((int)this + 0x2668) = 1;
            }
            break;
          }
        }
      }
      FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) + *(int *)((int)this + 0x23c4) * 4
                                    ) + 0x50),local_118);
      DeleteObject(*(HGDIOBJ *)((int)this + 0x25b4));
      EndDialog(param_1,1);
      return 1;
    }
    if (((uint)param_3 & 0xffff) == 2) {
      DeleteObject(*(HGDIOBJ *)((int)this + 0x25b4));
      EndDialog(param_1,0);
      return 1;
    }
  }
  else if ((param_2 == 0x138) && (pHVar2 = GetDlgItem(param_1,0x43f), param_4 == pHVar2)) {
    SetTextColor(param_3,0xff);
    SetBkMode(param_3,1);
    return *(undefined4 *)((int)this + 0x25b4);
  }
  return 0;
}
