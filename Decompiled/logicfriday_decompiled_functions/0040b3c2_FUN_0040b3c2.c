/* 0040b3c2 FUN_0040b3c2 */

LRESULT FUN_0040b3c2(HWND param_1,UINT param_2,WPARAM param_3,LPARAM param_4)

{
  LRESULT LVar1;
  
  if ((param_2 == 0x100) && (param_3 == 0x1b)) {
    SendMessageA(DAT_00452aac,0x111,0xe6,0);
    LVar1 = 1;
  }
  else {
    LVar1 = CallWindowProcA(DAT_00452aa0,param_1,param_2,param_3,param_4);
  }
  return LVar1;
}
