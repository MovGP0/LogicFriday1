/* 0043a4d3 FUN_0043a4d3 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __fastcall FUN_0043a4d3(undefined4 *param_1)

{
  HDC hdc;
  uint unaff_retaddr;
  tagRECT local_5c;
  tagPAINTSTRUCT local_4c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  hdc = BeginPaint((HWND)*param_1,&local_4c);
  GetClientRect((HWND)*param_1,&local_5c);
  SelectObject(hdc,(HGDIOBJ)param_1[9]);
  PatBlt(hdc,0,param_1[10],local_5c.right,param_1[0xd],0xf00021);
  PatBlt(hdc,param_1[0xb],param_1[10],param_1[0xd],param_1[0x1a] - param_1[10],0xf00021);
  PatBlt(hdc,param_1[0xb],param_1[0xc],local_5c.right - param_1[0xb],param_1[0xd],0xf00021);
  EndPaint((HWND)*param_1,&local_4c);
  return;
}
