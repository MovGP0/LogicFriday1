/* 0041509f FUN_0041509f */

undefined4 __thiscall FUN_0041509f(void *this,void *param_1)

{
  HDC hdc;
  HGDIOBJ h;
  HBRUSH hbr;
  RECT local_30;
  HBITMAP local_20;
  HENHMETAFILE local_1c;
  RECT local_18;
  HDC local_8;
  
  local_8 = (HDC)0x0;
  local_20 = (HBITMAP)0x0;
  FUN_004373f1(param_1,1);
  local_30.left = *(LONG *)((int)param_1 + 0x169c);
  local_30.top = *(LONG *)((int)param_1 + 0x16a0);
  local_30.right = *(int *)((int)param_1 + 0x16a4);
  local_30.bottom = *(int *)((int)param_1 + 0x16a8);
  local_1c = CopyEnhMetaFileA(*(HENHMETAFILE *)((int)param_1 + 0x1688),(LPCSTR)0x0);
  FUN_004373f1(param_1,0);
  OpenClipboard(*(HWND *)((int)this + 0x26c));
  EmptyClipboard();
  SetClipboardData(0xe,local_1c);
  if (local_30.bottom * local_30.right < 0x2000000) {
    hdc = GetDC(*(HWND *)((int)this + 0x26c));
    local_8 = CreateCompatibleDC(hdc);
    ReleaseDC(*(HWND *)((int)this + 0x26c),hdc);
    local_20 = CreateBitmap(local_30.right + 1,local_30.bottom + 1,1,1,(void *)0x0);
    SelectObject(local_8,local_20);
    h = GetStockObject(0);
    SelectObject(local_8,h);
    local_18.left = local_30.left;
    local_18.top = local_30.top;
    local_18.right = local_30.right + 1;
    local_18.bottom = local_30.bottom + 1;
    hbr = GetStockObject(0);
    FillRect(local_8,&local_18,hbr);
    PlayEnhMetaFile(local_8,local_1c,&local_30);
    SetClipboardData(2,local_20);
  }
  CloseClipboard();
  return 0;
}
