/* 0043a5a9 FUN_0043a5a9 */

void __thiscall FUN_0043a5a9(void *this,int param_1,int param_2,uint param_3)

{
  HBITMAP hbm;
  HBRUSH h;
  HDC hdc;
  HGDIOBJ h_00;
  
  hbm = CreateBitmap(8,8,1,1,&DAT_004519f0);
  h = CreatePatternBrush(hbm);
  hdc = GetDC(*(HWND *)this);
  h_00 = SelectObject(hdc,h);
  if (((*(int *)((int)this + 0x6c) == 0) && (param_2 != *(int *)((int)this + 0x3c))) &&
     ((param_3 & 1) != 0)) {
    PatBlt(hdc,1,*(int *)((int)this + 0x3c),*(int *)((int)this + 100) + -2,4,0x5a0049);
    PatBlt(hdc,1,param_2,*(int *)((int)this + 100) + -2,4,0x5a0049);
    *(int *)((int)this + 0x3c) = param_2;
  }
  else if (((*(int *)((int)this + 0x6c) == 1) && (param_1 != *(int *)((int)this + 0x38))) &&
          ((param_3 & 1) != 0)) {
    PatBlt(hdc,*(int *)((int)this + 0x38),*(int *)((int)this + 0x28) + *(int *)((int)this + 0x34),4,
           (*(int *)((int)this + 0x68) - *(int *)((int)this + 0x28)) - *(int *)((int)this + 0x34),
           0x5a0049);
    PatBlt(hdc,param_1,*(int *)((int)this + 0x28) + *(int *)((int)this + 0x34),4,
           (*(int *)((int)this + 0x68) - *(int *)((int)this + 0x28)) - *(int *)((int)this + 0x34),
           0x5a0049);
    *(int *)((int)this + 0x38) = param_1;
  }
  else if (((*(int *)((int)this + 0x6c) == 2) && (param_2 != *(int *)((int)this + 0x40))) &&
          ((param_3 & 1) != 0)) {
    PatBlt(hdc,*(int *)((int)this + 0x2c) + *(int *)((int)this + 0x34),*(int *)((int)this + 0x40),
           *(int *)((int)this + 100) + -2,4,0x5a0049);
    PatBlt(hdc,*(int *)((int)this + 0x2c) + *(int *)((int)this + 0x34),param_2,
           *(int *)((int)this + 100) + -2,4,0x5a0049);
    *(int *)((int)this + 0x40) = param_2;
  }
  ReleaseDC(*(HWND *)this,hdc);
  SelectObject(hdc,h_00);
  DeleteObject(h);
  DeleteObject(hbm);
  return;
}
