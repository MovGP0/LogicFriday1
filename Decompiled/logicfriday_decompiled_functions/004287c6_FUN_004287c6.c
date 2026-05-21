/* 004287c6 FUN_004287c6 */

void __thiscall FUN_004287c6(void *this,HDC param_1,int param_2,int param_3)

{
  HGDIOBJ ho;
  LOGBRUSH local_18;
  HGDIOBJ local_c;
  HBRUSH local_8;
  
  local_18.lbColor = *(COLORREF *)((int)this + 0x26ec);
  local_18.lbStyle = 0;
  local_8 = CreateBrushIndirect(&local_18);
  local_c = SelectObject(param_1,local_8);
  Rectangle(param_1,param_2 + -3,param_3 + -3,param_2 + 4,param_3 + 4);
  ho = SelectObject(param_1,local_c);
  DeleteObject(ho);
  return;
}
