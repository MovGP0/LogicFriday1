/* 004373f1 FUN_004373f1 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __thiscall FUN_004373f1(void *this,int param_1)

{
  int iVar1;
  HENHMETAFILE pHVar2;
  uint unaff_retaddr;
  int nDenominator;
  int local_d8;
  tagENHMETAHEADER local_d4;
  HPEN local_64;
  HPEN local_60;
  int local_5c;
  LOGFONTA local_58;
  uint local_1c;
  HFONT local_18;
  HPEN local_14;
  int local_10;
  HDC local_c;
  int local_8;
  
  local_1c = DAT_00451a00 ^ unaff_retaddr;
  local_60 = CreatePen(0,1,0);
  local_64 = CreatePen(0,1,0xdf);
  local_14 = CreatePen(0,1,0xdf0000);
  if (*(int *)((int)this + 0x16b0) != 0) {
    DeleteEnhMetaFile(*(HENHMETAFILE *)((int)this + 0x16b0));
  }
  local_c = CreateEnhMetaFileA((HDC)0x0,(LPCSTR)0x0,(RECT *)0x0,(LPCSTR)0x0);
  if (param_1 == 0) {
    SetPixel(local_c,0,0,0xffffff);
  }
  _memset(&local_58,0,0x3c);
  nDenominator = 0x48;
  iVar1 = GetDeviceCaps(local_c,0x5a);
  local_58.lfHeight = MulDiv(0xc,iVar1,nDenominator);
  local_58.lfHeight = -local_58.lfHeight;
  local_58.lfCharSet = '\0';
  local_58.lfWeight = 100;
  FUN_0043ed39(local_58.lfFaceName,(byte *)"COURIER NEW");
  local_18 = CreateFontIndirectA(&local_58);
  SelectObject(local_c,local_18);
  SetTextColor(local_c,0);
  SetBkColor(local_c,0xffffff);
  SetBkMode(local_c,1);
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x1650); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)((int)this + 0x3a4) + 0x48 + local_8 * 0xfc) == 0) {
      if ((*(int *)((int)this + 0x16b8) == 0) || (*(int *)((int)this + 0x16bc) != 0)) {
        SelectObject(local_c,local_60);
      }
      else if (*(int *)(*(int *)((int)this + 0x3a4) + 0x14 + local_8 * 0xfc) == 1) {
        SelectObject(local_c,local_64);
      }
      else {
        SelectObject(local_c,local_14);
      }
      FUN_00425f03(local_c,(int *)(local_8 * 0xfc + *(int *)((int)this + 0x3a4)),
                   *(int *)(*(int *)((int)this + 0x3a4) + 0xc0 + local_8 * 0xfc),
                   *(int *)(*(int *)((int)this + 0x3a4) + 0xc4 + local_8 * 0xfc),1);
    }
  }
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x40) == 0) {
      local_10 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x38);
      if ((*(int *)((int)this + 0x16b8) == 0) || (*(int *)((int)this + 0x16bc) != 0)) {
        SelectObject(local_c,local_60);
        *(undefined4 *)((int)this + 0x26ec) = 0;
      }
      else if (*(int *)(*(int *)((int)this + 0x3a4) + 0x14 + local_10 * 0xfc) == 1) {
        SelectObject(local_c,local_64);
        *(undefined4 *)((int)this + 0x26ec) = 0xff;
      }
      else {
        SelectObject(local_c,local_14);
        *(undefined4 *)((int)this + 0x26ec) = 0xff0000;
      }
      if (**(int **)(*(int *)((int)this + 0x16d0) + local_8 * 4) == 2) {
        FUN_004287c6(this,local_c,
                     **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c),
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)
                             + 4));
      }
      local_5c = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28) + -1;
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x14) == 2) {
        FUN_004287c6(this,local_c,
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)
                             + local_5c * 0x14),
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c)
                              + 4 + local_5c * 0x14));
      }
      MoveToEx(local_c,**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c),
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4),
               (LPPOINT)0x0);
      for (local_d8 = 1;
          local_d8 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28);
          local_d8 = local_d8 + 1) {
        LineTo(local_c,*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c
                                        ) + local_d8 * 0x14),
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4 +
                       local_d8 * 0x14));
      }
      if (**(int **)(*(int *)((int)this + 0x16d0) + local_8 * 4) == 0) {
        iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 4);
        local_5c = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 8);
        MoveToEx(local_c,*(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x6c + local_5c * 8),
                 *(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x70 + local_5c * 8),
                 (LPPOINT)0x0);
        LineTo(local_c,*(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x8c + local_5c * 8),
               *(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x90 + local_5c * 8));
      }
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x14) == 0) {
        iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x18);
        local_5c = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x1c);
        MoveToEx(local_c,*(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x6c + local_5c * 8),
                 *(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x70 + local_5c * 8),
                 (LPPOINT)0x0);
        LineTo(local_c,*(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x8c + local_5c * 8),
               *(int *)(*(int *)((int)this + 0x3a4) + iVar1 * 0xfc + 0x90 + local_5c * 8));
      }
    }
  }
  pHVar2 = CloseEnhMetaFile(local_c);
  *(HENHMETAFILE *)((int)this + 0x16b0) = pHVar2;
  DeleteObject(local_18);
  DeleteObject(local_60);
  DeleteObject(local_64);
  DeleteObject(local_14);
  *(undefined4 *)((int)this + 0x1688) = *(undefined4 *)((int)this + 0x16b0);
  GetEnhMetaFileHeader(*(HENHMETAFILE *)((int)this + 0x16b0),0x6c,&local_d4);
  *(LONG *)((int)this + 0x16a8) = local_d4.rclBounds.bottom - local_d4.rclBounds.top;
  *(LONG *)((int)this + 0x16a4) = local_d4.rclBounds.right - local_d4.rclBounds.left;
  *(undefined4 *)((int)this + 0x169c) = 0;
  *(undefined4 *)((int)this + 0x16a0) = 0;
  *(undefined4 *)((int)this + 0x267c) = 1;
  return;
}
