/* 0041b260 FUN_0041b260 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __fastcall FUN_0041b260(undefined4 *param_1)

{
  int iVar1;
  int iVar2;
  HWND pHVar3;
  int iVar4;
  undefined4 uVar5;
  size_t sVar6;
  size_t sVar7;
  ulonglong uVar8;
  uint unaff_retaddr;
  char *lpString;
  tagSIZE local_180;
  int local_178;
  int local_174;
  tagRECT local_170;
  HRGN local_160;
  tagRECT local_15c;
  tagRECT local_14c;
  int local_13c;
  int local_138;
  int local_134;
  HFONT local_130;
  LOGFONTA local_12c;
  int local_f0;
  tagRECT local_ec;
  int local_dc;
  int local_d8;
  HFONT local_d4;
  RECT local_d0;
  int local_c0;
  tagTEXTMETRICA local_bc;
  int local_84;
  int local_80;
  RECT local_7c;
  tagRECT local_6c;
  int local_5c;
  char local_58 [16];
  int local_48;
  int local_44;
  char local_40 [32];
  uint local_20;
  HPEN local_1c;
  int local_18;
  HRGN local_14;
  int local_10;
  int local_c;
  HFONT local_8;
  
  local_20 = DAT_00451a00 ^ unaff_retaddr;
  local_174 = 1;
  local_10 = 0;
  builtin_strncpy(local_58,"Logic Friday",0xd);
  local_13c = 0;
  local_18 = 0;
  local_5c = 0;
  local_c0 = 0;
  local_14 = (HRGN)0x0;
  local_160 = (HRGN)0x0;
  DAT_00452ed0 = 1;
  _memset(&local_12c,0,0x3c);
  FUN_0043ed39(local_12c.lfFaceName,(byte *)"COURIER NEW");
  local_12c.lfHeight = MulDiv(10,param_1[0x4f],0x48);
  local_12c.lfHeight = -local_12c.lfHeight;
  local_8 = CreateFontIndirectA(&local_12c);
  local_12c.lfEscapement = 900;
  local_d4 = CreateFontIndirectA(&local_12c);
  local_12c.lfEscapement = 0xa8c;
  local_130 = CreateFontIndirectA(&local_12c);
  SelectObject((HDC)param_1[0x3a],local_8);
  GetTextMetricsA((HDC)param_1[0x3a],&local_bc);
  param_1[0x48] = local_bc.tmHeight + local_bc.tmExternalLeading;
  param_1[0x47] = local_bc.tmMaxCharWidth;
  param_1[0x49] = local_bc.tmAveCharWidth;
  InvalidateRect((HWND)*param_1,(RECT *)0x0,1);
  UpdateWindow((HWND)*param_1);
  EnableWindow((HWND)*param_1,0);
  param_1[0x2d] = 0;
  pHVar3 = CreateDialogParamA((HINSTANCE)param_1[2],"PRNCNCLDLG",(HWND)*param_1,FUN_0040af1b,0);
  param_1[1] = pHVar3;
  SetAbortProc((HDC)param_1[0x3a],FUN_0040af38);
  local_7c.top = 0;
  local_7c.left = 0;
  local_7c.right = GetDeviceCaps((HDC)param_1[0x3a],0x6e);
  local_7c.bottom = GetDeviceCaps((HDC)param_1[0x3a],0x6f);
  local_160 = CreateRectRgnIndirect(&local_7c);
  local_15c.top = 0;
  local_15c.left = 0;
  uVar8 = FUN_0043ee30();
  local_15c.right = (LONG)uVar8;
  uVar8 = FUN_0043ee30();
  local_15c.bottom = (LONG)uVar8;
  if ((((param_1[0x4a] - param_1[8]) - param_1[6]) - (int)param_1[0x4e] / 10 < local_15c.right) ||
     ((((param_1[0x4b] - param_1[9]) - param_1[7]) - param_1[0x48]) - (int)param_1[0x4e] / 10 <
      local_15c.bottom)) {
    if (((param_1[0x4a] - param_1[8]) - param_1[6]) - (int)param_1[0x4e] / 10 < local_15c.right) {
      local_d0.left = param_1[6] + param_1[0x48] * 2;
      local_d0.right = (param_1[0x4a] - param_1[8]) + param_1[0x48] * -2;
    }
    else {
      local_c = ((((param_1[0x4a] - param_1[8]) - param_1[6]) - (int)param_1[0x4e] / 10) -
                local_15c.right) / 2;
      local_d0.left = param_1[6] + local_c;
      local_d0.right = (param_1[0x4a] - param_1[8]) - local_c;
    }
    if ((((param_1[0x4b] - param_1[9]) - param_1[7]) - param_1[0x48]) - (int)param_1[0x4e] / 10 <
        local_15c.bottom) {
      local_d0.top = param_1[0x48] * 3 + param_1[7];
      local_d0.bottom = (param_1[0x4b] - param_1[9]) + param_1[0x48] * -2;
    }
    else {
      local_c = (((((param_1[0x4b] - param_1[9]) - param_1[7]) - param_1[0x48]) -
                 (int)param_1[0x4e] / 10) - local_15c.bottom) / 2;
      local_d0.top = param_1[7] + param_1[0x48] + local_c;
      local_d0.bottom = (param_1[0x4b] - param_1[9]) - local_c;
    }
    local_14 = CreateRectRgnIndirect(&local_d0);
    SelectClipRgn((HDC)param_1[0x3a],local_14);
    local_5c = (local_d0.bottom - local_d0.top) - param_1[0x4f];
    local_c0 = (local_d0.right - local_d0.left) - param_1[0x4e];
    local_13c = local_15c.bottom / local_5c;
    if (local_13c == 0) {
      local_13c = 1;
    }
    else if (((local_13c + -1) * local_5c + local_d0.bottom) - local_d0.top < local_15c.bottom) {
      local_13c = local_13c + 1;
    }
    local_18 = local_15c.right / local_c0;
    if (local_18 == 0) {
      local_18 = 1;
    }
    else if (((local_18 + -1) * local_c0 + local_d0.right) - local_d0.left < local_15c.right) {
      local_18 = local_18 + 1;
    }
    local_f0 = local_13c * local_18;
    OffsetRect(&local_15c,local_d0.left,local_d0.top);
    local_dc = param_1[6];
    local_d8 = (int)(((param_1[0x4b] - param_1[9]) - param_1[7]) - param_1[0x48]) / 2 + param_1[7];
    local_138 = (int)((param_1[0x4a] - param_1[8]) - param_1[6]) / 2 + param_1[6];
    local_80 = param_1[7] + param_1[0x48];
    local_48 = param_1[0x4a] - param_1[8];
    local_134 = param_1[0x4b] - param_1[9];
    local_84 = local_138;
    local_44 = local_d8;
    GetTextExtentPoint32A((HDC)param_1[0x3a],"To Page 999",0xb,&local_180);
    SetRect(&local_6c,0,0,local_180.cx + (int)param_1[0x4e] / 0x28,
            local_180.cy + (int)param_1[0x4f] / 0x14);
    SetRect(&local_170,0,0,local_180.cy + (int)param_1[0x4f] / 0x14,
            local_180.cx + (int)param_1[0x4e] / 0x28);
    local_14c.left = local_6c.left;
    local_14c.top = local_6c.top;
    local_14c.right = local_6c.right;
    local_14c.bottom = local_6c.bottom;
    local_ec.left = local_170.left;
    local_ec.top = local_170.top;
    local_ec.right = local_170.right;
    local_ec.bottom = local_170.bottom;
    OffsetRect(&local_6c,local_84 - (local_6c.right - local_6c.left) / 2,local_80);
    OffsetRect(&local_14c,local_138 - (local_14c.right - local_14c.left) / 2,
               local_134 - local_14c.bottom);
    OffsetRect(&local_170,local_dc,local_d8 - (local_170.bottom - local_170.top) / 2);
    OffsetRect(&local_ec,local_48 - local_ec.right,local_44 - (local_ec.bottom - local_ec.top) / 2);
  }
  else {
    local_f0 = 1;
    local_18 = 1;
    local_13c = 1;
    OffsetRect(&local_15c,param_1[8],param_1[7] + param_1[0x48]);
    local_15c.left = local_15c.left + ((param_1[0x4a] - param_1[8]) - local_15c.right) / 2;
    local_15c.right = local_15c.right + ((param_1[0x4a] - param_1[8]) - local_15c.right) / 2;
    local_15c.top = local_15c.top + ((param_1[0x4b] - param_1[9]) - local_15c.bottom) / 2;
    local_15c.bottom = local_15c.bottom + ((param_1[0x4b] - param_1[9]) - local_15c.bottom) / 2;
    local_d0.left = local_15c.left;
    local_d0.top = local_15c.top;
    local_d0.right = local_15c.right;
    local_d0.bottom = local_15c.bottom;
  }
  iVar4 = StartDocA((HDC)param_1[0x3a],(DOCINFOA *)(param_1 + 0x31));
  if (iVar4 < 1) {
    DeleteDC((HDC)param_1[0x3a]);
    DAT_00452ed0 = 0;
    if (param_1[0x2d] == 0) {
      EnableWindow((HWND)*param_1,1);
      DestroyWindow((HWND)param_1[1]);
      DeleteDC((HDC)param_1[0x3a]);
      DeleteObject(local_8);
      DeleteObject(local_d4);
      DeleteObject(local_130);
      DeleteObject(local_14);
      DeleteObject(local_160);
      uVar5 = 0x270000;
    }
    else {
      uVar5 = 0;
    }
  }
  else {
    local_10 = 1;
    local_1c = CreatePen(0,(int)param_1[0x4e] / 0x60,0);
    SelectObject((HDC)param_1[0x3a],local_1c);
    for (local_c = 0; local_c < local_18; local_c = local_c + 1) {
      for (local_178 = 0; local_178 < local_13c; local_178 = local_178 + 1) {
        iVar4 = StartPage((HDC)param_1[0x3a]);
        if (iVar4 < 1) {
          EnableWindow((HWND)*param_1,1);
          DeleteDC((HDC)param_1[0x3a]);
          DeleteObject(local_8);
          DeleteObject(local_d4);
          DeleteObject(local_130);
          DeleteObject(local_1c);
          DeleteObject(local_14);
          DeleteObject(local_160);
          DAT_00452ed0 = 0;
          return 0x270000;
        }
        SelectClipRgn((HDC)param_1[0x3a],local_160);
        if (local_174 != 0) {
          SelectObject((HDC)param_1[0x3a],local_8);
          SetTextAlign((HDC)param_1[0x3a],0);
          sVar6 = _strlen(local_58);
          TextOutA((HDC)param_1[0x3a],param_1[6],param_1[7],local_58,sVar6);
          FUN_0043ed39(local_40,(byte *)"Page %d of %d");
          sVar6 = _strlen(local_40);
          lpString = local_40;
          iVar4 = param_1[7];
          iVar1 = param_1[0x4a];
          iVar2 = param_1[8];
          sVar7 = _strlen(local_40);
          TextOutA((HDC)param_1[0x3a],(iVar1 - iVar2) - param_1[0x47] * sVar7,iVar4,lpString,sVar6);
          Rectangle((HDC)param_1[0x3a],param_1[6],param_1[7] + param_1[0x48],
                    param_1[0x4a] - param_1[8],param_1[0x4b] - param_1[9]);
        }
        if (local_c != 0) {
          Rectangle((HDC)param_1[0x3a],local_170.left,local_170.top,local_170.right,local_170.bottom
                   );
          SelectObject((HDC)param_1[0x3a],local_d4);
          SetTextAlign((HDC)param_1[0x3a],6);
          FUN_0043ed39(local_40,(byte *)"To Page %d");
          sVar6 = _strlen(local_40);
          TextOutA((HDC)param_1[0x3a],local_dc,local_d8,local_40,sVar6);
        }
        if (local_c != local_18 + -1) {
          Rectangle((HDC)param_1[0x3a],local_ec.left,local_ec.top,local_ec.right,local_ec.bottom);
          SelectObject((HDC)param_1[0x3a],local_130);
          SetTextAlign((HDC)param_1[0x3a],6);
          FUN_0043ed39(local_40,(byte *)"To Page %d");
          sVar6 = _strlen(local_40);
          TextOutA((HDC)param_1[0x3a],local_48,local_44,local_40,sVar6);
        }
        if (local_178 != 0) {
          Rectangle((HDC)param_1[0x3a],local_6c.left,local_6c.top,local_6c.right,local_6c.bottom);
          SelectObject((HDC)param_1[0x3a],local_8);
          SetTextAlign((HDC)param_1[0x3a],6);
          FUN_0043ed39(local_40,(byte *)"To Page %d");
          sVar6 = _strlen(local_40);
          TextOutA((HDC)param_1[0x3a],local_84,local_80,local_40,sVar6);
        }
        if (local_178 != local_13c + -1) {
          Rectangle((HDC)param_1[0x3a],local_14c.left,local_14c.top,local_14c.right,local_14c.bottom
                   );
          SelectObject((HDC)param_1[0x3a],local_8);
          SetTextAlign((HDC)param_1[0x3a],0xe);
          FUN_0043ed39(local_40,(byte *)"To Page %d");
          sVar6 = _strlen(local_40);
          TextOutA((HDC)param_1[0x3a],local_138,local_134,local_40,sVar6);
        }
        local_10 = local_10 + 1;
        if (1 < local_f0) {
          SelectClipRgn((HDC)param_1[0x3a],local_14);
        }
        PlayEnhMetaFile((HDC)param_1[0x3a],*(HENHMETAFILE *)(param_1[3] + 0x1688),&local_15c);
        iVar4 = EndPage((HDC)param_1[0x3a]);
        if (iVar4 < 1) {
          EnableWindow((HWND)*param_1,1);
          DeleteDC((HDC)param_1[0x3a]);
          DeleteObject(local_8);
          DeleteObject(local_d4);
          DeleteObject(local_130);
          DeleteObject(local_1c);
          DeleteObject(local_14);
          DeleteObject(local_160);
          DAT_00452ed0 = 0;
          return 0x270000;
        }
        OffsetRect(&local_15c,0,-local_5c);
      }
      local_15c.bottom = local_15c.bottom - local_15c.top;
      local_15c.top = 0;
      OffsetRect(&local_15c,-local_c0,local_d0.top);
    }
    EndDoc((HDC)param_1[0x3a]);
    if (param_1[0x2d] == 0) {
      EnableWindow((HWND)*param_1,1);
      DestroyWindow((HWND)param_1[1]);
    }
    DeleteDC((HDC)param_1[0x3a]);
    DeleteObject(local_8);
    DeleteObject(local_d4);
    DeleteObject(local_130);
    DeleteObject(local_1c);
    DeleteObject(local_14);
    DeleteObject(local_160);
    uVar5 = 0;
  }
  DAT_00452ed0 = 0;
  return uVar5;
}
