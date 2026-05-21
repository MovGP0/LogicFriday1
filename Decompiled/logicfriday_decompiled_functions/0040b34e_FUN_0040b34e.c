/* 0040b34e FUN_0040b34e */

LRESULT FUN_0040b34e(HWND param_1,UINT param_2,WPARAM param_3,LPARAM param_4)

{
  HCURSOR hCursor;
  LRESULT LVar1;
  
  if ((param_2 == 0x20) &&
     ((((DAT_00452e7c != 0 || (DAT_00452eb0 != 0)) || (DAT_00452eb4 != 0)) ||
      ((DAT_00452ed0 != 0 || (DAT_00452ed4 != 0)))))) {
    hCursor = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f02);
    SetCursor(hCursor);
    LVar1 = 1;
  }
  else {
    LVar1 = CallWindowProcA(DAT_00452aa4,param_1,param_2,param_3,param_4);
  }
  return LVar1;
}
