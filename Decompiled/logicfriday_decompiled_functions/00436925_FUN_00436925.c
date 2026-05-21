/* 00436925 FUN_00436925 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_00436925(void *this,undefined4 *param_1,int param_2)

{
  undefined4 uVar1;
  int nNumerator;
  HGDIOBJ pvVar2;
  HGDIOBJ ho;
  HENHMETAFILE pHVar3;
  uint unaff_retaddr;
  int nDenominator;
  tagENHMETAHEADER local_2cc;
  HGDIOBJ local_25c;
  LOGFONTA local_258;
  HFONT local_21c;
  HDC local_218;
  uint local_214 [65];
  HPEN local_110;
  char local_10c [260];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_218 = (HDC)0x0;
  if (*(int *)((int)this + 0x16c4) == 0) {
    uVar1 = 0;
  }
  else {
    if ((param_2 == 0) && (*(int *)((int)this + 0x16b0) != 0)) {
      DeleteEnhMetaFile(*(HENHMETAFILE *)((int)this + 0x16b0));
    }
    FUN_0043ebd0(local_214,(uint *)((int)this + 0xd0));
    if (1 < *(uint *)((int)this + 200)) {
      FUN_0043ebe0(local_214,(uint *)&DAT_0044ac88);
      FUN_0043ebe0(local_214,(uint *)((int)this + (*(int *)((int)this + 200) + -1) * 9 + 0xd0));
    }
    FUN_0043ed39(local_10c,(byte *)"Logic Friday Diagram");
    local_218 = CreateEnhMetaFileA((HDC)0x0,(LPCSTR)0x0,(RECT *)0x0,local_10c);
    _memset(&local_258,0,0x3c);
    nDenominator = 0x48;
    nNumerator = GetDeviceCaps(local_218,0x5a);
    local_258.lfHeight = MulDiv(0xc,nNumerator,nDenominator);
    local_258.lfHeight = -local_258.lfHeight;
    local_258.lfCharSet = '\0';
    local_258.lfWeight = 100;
    FUN_0043ed39(local_258.lfFaceName,(byte *)"COURIER NEW");
    local_21c = CreateFontIndirectA(&local_258);
    local_25c = SelectObject(local_218,local_21c);
    local_110 = CreatePen(0,1,0);
    pvVar2 = SelectObject(local_218,local_110);
    SetTextColor(local_218,0);
    SetBkColor(local_218,0xffffff);
    SetBkMode(local_218,1);
    FUN_00431daa(this,local_218,-*(int *)((int)this + 0x238c),-*(int *)((int)this + 0x2390),1,
                 param_2);
    ho = SelectObject(local_218,local_25c);
    DeleteObject(ho);
    pvVar2 = SelectObject(local_218,pvVar2);
    DeleteObject(pvVar2);
    if (param_2 == 0) {
      pHVar3 = CloseEnhMetaFile(local_218);
      *(HENHMETAFILE *)((int)this + 0x16b0) = pHVar3;
      *(undefined4 *)((int)this + 0x1688) = *(undefined4 *)((int)this + 0x16b0);
      GetEnhMetaFileHeader(*(HENHMETAFILE *)((int)this + 0x16b0),0x6c,&local_2cc);
      *(LONG *)((int)this + 0x16a8) = local_2cc.rclBounds.bottom - local_2cc.rclBounds.top;
      *(LONG *)((int)this + 0x16a4) = local_2cc.rclBounds.right - local_2cc.rclBounds.left;
      *(undefined4 *)((int)this + 0x169c) = 0;
      *(undefined4 *)((int)this + 0x16a0) = 0;
      *(undefined4 *)((int)this + 0x267c) = 1;
    }
    else {
      pHVar3 = CloseEnhMetaFile(local_218);
      *param_1 = pHVar3;
    }
    uVar1 = 1;
  }
  return uVar1;
}
