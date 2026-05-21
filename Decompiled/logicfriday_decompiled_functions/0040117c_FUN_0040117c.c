/* 0040117c FUN_0040117c */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void __cdecl FUN_0040117c(HWND param_1)

{
  int c;
  LONG LVar1;
  uint unaff_retaddr;
  tagSIZE *psizl;
  tagSIZE local_5c;
  HWND local_54;
  uint local_50;
  LOGFONTA local_4c;
  uint local_10;
  HDC local_c;
  HWND local_8;
  
  local_10 = DAT_00451a00 ^ unaff_retaddr;
  local_8 = GetDlgItem(param_1,DAT_00451060);
  local_c = GetDC(local_8);
  _memset(&local_4c,0,0x3c);
  DAT_0046c9f0 = (HFONT)SendMessageA(local_8,0x31,0,0);
  DAT_0046c9e8 = GetSystemMetrics(0x2d);
  DAT_0046c9ec = GetSystemMetrics(0x2e);
  if (DAT_0046c9f0 != (HFONT)0x0) {
    GetObjectA(DAT_0046c9f0,0x3c,&local_4c);
    local_4c.lfUnderline = '\x01';
    DAT_0046c9f0 = CreateFontIndirectA(&local_4c);
    if (DAT_0046c9f0 != (HFONT)0x0) {
      SelectObject(local_c,DAT_0046c9f0);
      _DAT_0046c9e4 = 0;
      for (local_50 = 0; local_50 < 2; local_50 = local_50 + 1) {
        local_54 = GetDlgItem(param_1,(&DAT_00451060)[local_50 * 0xc6]);
        if (local_54 != (HWND)0x0) {
          SendMessageA(local_54,0x30,(WPARAM)DAT_0046c9f0,1);
          psizl = &local_5c;
          c = lstrlenA(s_logic_friday_sontrak_com_0045106c + local_50 * 0x318);
          GetTextExtentPoint32A
                    (local_c,s_logic_friday_sontrak_com_0045106c + local_50 * 0x318,c,psizl);
          local_5c.cx = local_5c.cx + DAT_0046c9e8 * 2;
          local_5c.cy = local_5c.cy + DAT_0046c9ec * 2;
          SetWindowPos(local_54,param_1,0,0,local_5c.cx,local_5c.cy,0x46);
          LVar1 = SetWindowLongA(local_54,-4,0x401133);
          *(LONG *)(&DAT_00451068 + local_50 * 0x318) = LVar1;
        }
      }
    }
  }
  if ((local_c != (HDC)0x0) && (local_8 != (HWND)0x0)) {
    ReleaseDC(local_8,local_c);
  }
  return;
}
