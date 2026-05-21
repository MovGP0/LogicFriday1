/* 0041ac36 FUN_0041ac36 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __fastcall FUN_0041ac36(undefined4 *param_1)

{
  int iVar1;
  int iVar2;
  HWND pHVar3;
  undefined4 uVar4;
  size_t sVar5;
  size_t sVar6;
  HPEN h;
  uint unaff_retaddr;
  char *lpString;
  int iVar7;
  int local_d0;
  LOGFONTA local_cc;
  size_t local_90;
  tagTEXTMETRICA local_8c;
  char local_54 [16];
  uint local_44;
  char local_40 [32];
  uint local_20;
  int local_1c;
  int local_18;
  LPSTR local_14;
  undefined4 local_10;
  char *local_c;
  HFONT local_8;
  
  local_20 = DAT_00451a00 ^ unaff_retaddr;
  local_10 = 0;
  local_1c = 0;
  builtin_strncpy(local_54,"Logic Friday",0xd);
  DAT_00452ed0 = 1;
  _memset(&local_cc,0,0x3c);
  FUN_0043ed39(local_cc.lfFaceName,(byte *)"COURIER NEW");
  iVar7 = 0x48;
  iVar2 = GetDeviceCaps((HDC)param_1[0x3a],0x5a);
  local_cc.lfHeight = MulDiv(10,iVar2,iVar7);
  local_cc.lfHeight = -local_cc.lfHeight;
  local_8 = CreateFontIndirectA(&local_cc);
  SelectObject((HDC)param_1[0x3a],local_8);
  SetTextAlign((HDC)param_1[0x3a],0x18);
  GetTextMetricsA((HDC)param_1[0x3a],&local_8c);
  param_1[0x48] = local_8c.tmHeight + local_8c.tmExternalLeading;
  param_1[0x47] = local_8c.tmMaxCharWidth;
  param_1[0x49] = local_8c.tmAveCharWidth;
  local_44 = (int)((param_1[0x4a] - param_1[6]) - param_1[8]) / (int)param_1[0x49];
  local_18 = (int)((param_1[0x4b] - param_1[7]) - param_1[9]) / (int)param_1[0x48];
  local_14 = _malloc(local_44 + 1);
  InvalidateRect((HWND)*param_1,(RECT *)0x0,1);
  UpdateWindow((HWND)*param_1);
  EnableWindow((HWND)*param_1,0);
  param_1[0x2d] = 0;
  pHVar3 = CreateDialogParamA((HINSTANCE)param_1[2],"PRNCNCLDLG",(HWND)*param_1,FUN_0040af1b,0);
  param_1[1] = pHVar3;
  SetAbortProc((HDC)param_1[0x3a],FUN_0040af38);
  iVar2 = StartDocA((HDC)param_1[0x3a],(DOCINFOA *)(param_1 + 0x31));
  if (iVar2 < 1) {
    DeleteDC((HDC)param_1[0x3a]);
    DeleteObject(local_8);
    DAT_00452ed0 = 0;
    if (param_1[0x2d] == 0) {
      EnableWindow((HWND)*param_1,1);
      DestroyWindow((HWND)param_1[1]);
      _free(local_14);
      uVar4 = 0x270000;
    }
    else {
      uVar4 = 0;
    }
  }
  else {
    if (param_1[0x2e] == 0) {
      local_c = *(char **)(param_1[3] + 0x268);
    }
    else {
      local_c = *(char **)(param_1[3] + 0x26c);
    }
    local_90 = _strlen(local_c);
    do {
      if ((int)local_90 < 1) {
        EndDoc((HDC)param_1[0x3a]);
        if (param_1[0x2d] == 0) {
          EnableWindow((HWND)*param_1,1);
          DestroyWindow((HWND)param_1[1]);
        }
        DeleteDC((HDC)param_1[0x3a]);
        DeleteObject(local_8);
        _free(local_14);
        DAT_00452ed0 = 0;
        return 0;
      }
      iVar2 = StartPage((HDC)param_1[0x3a]);
      if (iVar2 < 1) {
        EnableWindow((HWND)*param_1,1);
        DeleteDC((HDC)param_1[0x3a]);
        DeleteObject(local_8);
        _free(local_14);
        DAT_00452ed0 = 0;
        return 0x270000;
      }
      local_1c = local_1c + 1;
      sVar5 = _strlen(local_54);
      TextOutA((HDC)param_1[0x3a],param_1[6],param_1[7],local_54,sVar5);
      FUN_0043ed39(local_40,(byte *)"Page %d");
      sVar5 = _strlen(local_40);
      lpString = local_40;
      iVar2 = param_1[7];
      iVar7 = param_1[0x4a];
      iVar1 = param_1[8];
      sVar6 = _strlen(local_40);
      TextOutA((HDC)param_1[0x3a],(iVar7 - iVar1) - param_1[0x47] * sVar6,iVar2,lpString,sVar5);
      h = CreatePen(0,(int)param_1[0x4e] / 300 + 1,0);
      SelectObject((HDC)param_1[0x3a],h);
      MoveToEx((HDC)param_1[0x3a],param_1[6],(int)param_1[0x4f] / 0x14 + param_1[7],(LPPOINT)0x0);
      LineTo((HDC)param_1[0x3a],param_1[0x4a] - param_1[8],(int)param_1[0x4f] / 0x14 + param_1[7]);
      DeleteObject(h);
      local_d0 = 2;
      while ((local_d0 < local_18 && (0 < (int)local_90))) {
        FUN_0041c423(local_14,(int *)&local_c,(int *)&local_90,local_44);
        sVar5 = _strlen(local_14);
        TextOutA((HDC)param_1[0x3a],param_1[6],param_1[0x48] * local_d0 + param_1[7],local_14,sVar5)
        ;
        local_d0 = local_d0 + 1;
      }
      iVar2 = EndPage((HDC)param_1[0x3a]);
    } while (0 < iVar2);
    EnableWindow((HWND)*param_1,1);
    DeleteDC((HDC)param_1[0x3a]);
    DeleteObject(local_8);
    _free(local_14);
    DAT_00452ed0 = 0;
    uVar4 = 0x270000;
  }
  return uVar4;
}
