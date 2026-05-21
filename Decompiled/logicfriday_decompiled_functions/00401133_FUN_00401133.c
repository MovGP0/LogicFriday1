/* 00401133 FUN_00401133 */

LRESULT FUN_00401133(HWND param_1,UINT param_2,WPARAM param_3,LPARAM param_4)

{
  HCURSOR hCursor;
  LRESULT LVar1;
  int iVar2;
  WNDPROC lpPrevWndFunc;
  
  if (param_2 == 0x20) {
    hCursor = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f89);
    SetCursor(hCursor);
    LVar1 = 1;
  }
  else {
    iVar2 = GetDlgCtrlID(param_1);
    lpPrevWndFunc = (WNDPROC)FUN_00401078(iVar2);
    LVar1 = CallWindowProcA(lpPrevWndFunc,param_1,param_2,param_3,param_4);
  }
  return LVar1;
}
