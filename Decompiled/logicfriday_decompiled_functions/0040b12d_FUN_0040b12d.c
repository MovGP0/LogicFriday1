/* 0040b12d FUN_0040b12d */

LRESULT FUN_0040b12d(HWND param_1,UINT param_2,WPARAM param_3,LPARAM param_4)

{
  SHORT SVar1;
  HCURSOR hCursor;
  LRESULT LVar2;
  
  if (param_2 == 0x20) {
    if ((((DAT_00452e7c != 0) || (DAT_00452eb0 != 0)) || (DAT_00452eb4 != 0)) ||
       ((DAT_00452ed0 != 0 || (DAT_00452ed4 != 0)))) {
      hCursor = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f02);
      SetCursor(hCursor);
      return 1;
    }
  }
  else if ((param_2 == 0x100) && (DAT_00452e98 != 0)) {
    SVar1 = GetKeyState(0x12);
    if (((int)SVar1 & 0x8000U) != 0) {
      return 1;
    }
    if (param_3 == 0xd) {
      SVar1 = GetKeyState(0x10);
      if (((int)SVar1 & 0x8000U) != 0) {
        return 1;
      }
      SendMessageA(DAT_00452ab0,0xb1,0xffffffff,-1);
      SVar1 = GetKeyState(0x11);
      if (((int)SVar1 & 0x8000U) != 0) {
        SendMessageA(DAT_00452ab0,0xc2,0,0x44b734);
        return 1;
      }
      PostMessageA(DAT_00452aac,0x111,0x8000,0);
    }
    else {
      if (param_3 == 0x1b) {
        SendMessageA(DAT_00452aac,0x111,0x150,0);
        return 1;
      }
      if (((param_3 == 0x56) || (param_3 == 0x76)) &&
         (SVar1 = GetKeyState(0x11), ((int)SVar1 & 0x8000U) != 0)) {
        PostMessageA(DAT_00452aac,0x111,0xae,0);
        return 1;
      }
    }
  }
  LVar2 = CallWindowProcA(DAT_00452918,param_1,param_2,param_3,param_4);
  return LVar2;
}
