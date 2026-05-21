/* 00435e82 FUN_00435e82 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_00435e82(void *this,HWND param_1,int param_2,HDC param_3,HWND param_4)

{
  HBRUSH pHVar1;
  HWND pHVar2;
  size_t sVar3;
  int iVar4;
  char *pcVar5;
  char *pcVar6;
  uint unaff_retaddr;
  UINT Msg;
  WPARAM wParam;
  LPARAM lParam;
  LOGBRUSH local_144;
  int local_138;
  char local_134 [12];
  char local_128 [28];
  uint local_10c [64];
  uint local_c;
  int local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  pcVar5 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  pcVar6 = local_128;
  for (iVar4 = 6; iVar4 != 0; iVar4 = iVar4 + -1) {
    *(undefined4 *)pcVar6 = *(undefined4 *)pcVar5;
    pcVar5 = pcVar5 + 4;
    pcVar6 = pcVar6 + 4;
  }
  *(undefined2 *)pcVar6 = *(undefined2 *)pcVar5;
  pcVar6[2] = pcVar5[2];
  if (param_2 == 0x110) {
    *(undefined4 *)((int)this + 0x25b4) = 0;
    *(HWND *)((int)this + 0x25b0) = param_4;
    lParam = 0;
    wParam = 8;
    Msg = 0xc5;
    pHVar2 = GetDlgItem(param_1,0x43e);
    SendMessageA(pHVar2,Msg,wParam,lParam);
    local_144.lbColor = GetSysColor(0xf);
    local_144.lbStyle = 0;
    local_144.lbHatch = 0;
    pHVar1 = CreateBrushIndirect(&local_144);
    *(HBRUSH *)((int)this + 0x25b4) = pHVar1;
    if (*(int *)((int)this + 0x25b0) == 0x438) {
      local_8 = 0;
      while ((local_8 < 0x1a && (*(int *)((int)this + local_8 * 4 + 0x25ec) != 0))) {
        local_8 = local_8 + 1;
      }
      FUN_0043ed39(local_134,&DAT_0044c980);
      SetDlgItemTextA(param_1,0x43e,local_134);
    }
    else {
      local_138 = FUN_0040be0b();
      FUN_0043ed39(local_134,&DAT_0044a700);
      SetDlgItemTextA(param_1,0x43e,local_134);
    }
    return 1;
  }
  if (param_2 == 0x111) {
    if (((uint)param_3 & 0xffff) == 1) {
      GetDlgItemTextA(param_1,0x43e,local_134,9);
      local_138 = FUN_00436863(this,local_134);
      if (local_138 == 0) {
        if (*(int *)((int)this + 0x25b0) == 0x438) {
          FUN_0043ebd0((uint *)((int)this + *(int *)((int)this + 0xc4) * 9 + 0x160),
                       (uint *)local_134);
          *(int *)((int)this + 0xc4) = *(int *)((int)this + 0xc4) + 1;
          sVar3 = _strlen(local_134);
          if ((sVar3 == 1) && (iVar4 = _isupper((int)local_134[0]), iVar4 != 0)) {
            for (local_8 = 0; (local_8 < 0x1a && (local_134[0] != local_128[local_8]));
                local_8 = local_8 + 1) {
            }
            *(undefined4 *)((int)this + local_8 * 4 + 0x25ec) = 1;
          }
        }
        else {
          FUN_0043ebd0((uint *)((int)this + *(int *)((int)this + 200) * 9 + 0xd0),(uint *)local_134)
          ;
          *(int *)((int)this + 200) = *(int *)((int)this + 200) + 1;
        }
        DeleteObject(*(HGDIOBJ *)((int)this + 0x25b4));
        EndDialog(param_1,1);
        return 1;
      }
      if (local_138 == 1) {
        FUN_0043ed39((char *)local_10c,(byte *)"Error: Name must contain 1 to %d characters.");
      }
      else if (local_138 == 2) {
        FUN_0043ebd0(local_10c,(uint *)"Error: Name already in use.");
      }
      else if (local_138 == 3) {
        FUN_0043ebd0(local_10c,(uint *)"Error: Name must begin with a letter.");
      }
      else if (100 < local_138) {
        FUN_0043ed39((char *)local_10c,(byte *)"Illegal character: \'%c\'");
      }
      SetDlgItemTextA(param_1,0x43f,(LPCSTR)local_10c);
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
