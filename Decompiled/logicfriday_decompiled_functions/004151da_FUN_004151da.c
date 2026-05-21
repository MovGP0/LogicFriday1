/* 004151da FUN_004151da */

undefined4 __thiscall FUN_004151da(void *this,void *param_1)

{
  HDC hdc;
  HGDIOBJ h;
  HBRUSH hbr;
  HENHMETAFILE local_a0;
  tagENHMETAHEADER local_9c;
  tagRECT local_2c;
  HBITMAP local_1c;
  RECT local_18;
  HDC local_8;
  
  local_8 = (HDC)0x0;
  local_1c = (HBITMAP)0x0;
  local_a0 = (HENHMETAFILE)0x0;
  FUN_00436925(param_1,&local_a0,1);
  GetEnhMetaFileHeader(local_a0,0x6c,&local_9c);
  local_2c.bottom = local_9c.rclBounds.bottom - local_9c.rclBounds.top;
  local_2c.right = local_9c.rclBounds.right - local_9c.rclBounds.left;
  local_2c.left = 0;
  local_2c.top = 0;
  if (local_2c.bottom * local_2c.right < 0x2000000) {
    hdc = GetDC(*(HWND *)((int)this + 0x26c));
    local_8 = CreateCompatibleDC(hdc);
    ReleaseDC(*(HWND *)((int)this + 0x26c),hdc);
    local_1c = CreateBitmap(local_2c.right + 1,local_2c.bottom + 1,1,1,(void *)0x0);
    SelectObject(local_8,local_1c);
    h = GetStockObject(0);
    SelectObject(local_8,h);
    local_18.left = local_2c.left;
    local_18.top = local_2c.top;
    local_18.right = local_2c.right + 1;
    local_18.bottom = local_2c.bottom + 1;
    hbr = GetStockObject(0);
    FillRect(local_8,&local_18,hbr);
    OffsetRect(&local_2c,1,1);
    PlayEnhMetaFile(local_8,local_a0,&local_2c);
  }
  OpenClipboard(*(HWND *)((int)this + 0x26c));
  EmptyClipboard();
  SetClipboardData(0xe,local_a0);
  if (local_1c != (HBITMAP)0x0) {
    SetClipboardData(2,local_1c);
  }
  CloseClipboard();
  return 0;
}
