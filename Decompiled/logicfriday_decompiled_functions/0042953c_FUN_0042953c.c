/* 0042953c FUN_0042953c */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __thiscall FUN_0042953c(void *this,HWND param_1)

{
  HDC pHVar1;
  HPEN pHVar2;
  HBRUSH pHVar3;
  int nNumerator;
  HFONT pHVar4;
  HDC pHVar5;
  HBITMAP pHVar6;
  HGDIOBJ h;
  uint unaff_retaddr;
  int nDenominator;
  LOGBRUSH local_58;
  LOGFONTA local_4c;
  uint local_10;
  int local_c;
  int local_8;
  
  local_10 = DAT_00451a00 ^ unaff_retaddr;
  if (*(int *)((int)this + 0x2318) != 0) {
    FUN_004297b5((int)this);
  }
  pHVar1 = GetDC(*(HWND *)((int)this + 0x16f0));
  local_c = GetDeviceCaps(pHVar1,8);
  local_8 = GetDeviceCaps(pHVar1,10);
  SetRect((LPRECT)((int)this + 0x2338),0,0,local_c,local_8);
  pHVar2 = CreatePen(0,1,0xdf0000);
  *(HPEN *)((int)this + 0x2320) = pHVar2;
  pHVar2 = CreatePen(0,1,0xdf0000);
  *(HPEN *)((int)this + 9000) = pHVar2;
  pHVar2 = CreatePen(2,1,0);
  *(HPEN *)((int)this + 0x2324) = pHVar2;
  local_58.lbColor = 0xff0000;
  local_58.lbStyle = 0;
  pHVar3 = CreateBrushIndirect(&local_58);
  *(HBRUSH *)((int)this + 0x2330) = pHVar3;
  pHVar2 = CreatePen(0,5,0xc0c0c0);
  *(HPEN *)((int)this + 0x232c) = pHVar2;
  _memset(&local_4c,0,0x3c);
  nDenominator = 0x48;
  nNumerator = GetDeviceCaps(pHVar1,0x5a);
  local_4c.lfHeight = MulDiv(0xc,nNumerator,nDenominator);
  local_4c.lfHeight = -local_4c.lfHeight;
  local_4c.lfCharSet = '\0';
  local_4c.lfWeight = 100;
  FUN_0043ed39(local_4c.lfFaceName,(byte *)"COURIER NEW");
  pHVar4 = CreateFontIndirectA(&local_4c);
  *(HFONT *)((int)this + 0x2334) = pHVar4;
  pHVar5 = CreateCompatibleDC(pHVar1);
  *(HDC *)((int)this + 0x2318) = pHVar5;
  pHVar6 = CreateCompatibleBitmap(pHVar1,local_c,local_8);
  *(HBITMAP *)((int)this + 0x2314) = pHVar6;
  ReleaseDC(*(HWND *)((int)this + 0x16f0),pHVar1);
  SelectObject(*(HDC *)((int)this + 0x2318),*(HGDIOBJ *)((int)this + 0x2314));
  pHVar3 = GetStockObject(0);
  FillRect(*(HDC *)((int)this + 0x2318),(RECT *)((int)this + 0x2338),pHVar3);
  SelectObject(*(HDC *)((int)this + 0x2318),*(HGDIOBJ *)((int)this + 0x2320));
  h = GetStockObject(0);
  SelectObject(*(HDC *)((int)this + 0x2318),h);
  SelectObject(*(HDC *)((int)this + 0x2318),*(HGDIOBJ *)((int)this + 0x2334));
  SetTextColor(*(HDC *)((int)this + 0x2318),0xdf0000);
  pHVar1 = GetDC(param_1);
  *(HDC *)((int)this + 0x231c) = pHVar1;
  SelectObject(*(HDC *)((int)this + 0x231c),*(HGDIOBJ *)((int)this + 0x2320));
  SelectObject(*(HDC *)((int)this + 0x231c),*(HGDIOBJ *)((int)this + 0x2334));
  SetTextColor(*(HDC *)((int)this + 0x231c),0xdf0000);
  return;
}
