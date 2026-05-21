/* 0040b055 FUN_0040b055 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_0040b055(HWND param_1,int param_2,short param_3)

{
  uint unaff_retaddr;
  char local_114 [268];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 0x110) {
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,2);
      return 1;
    }
    if (param_3 == 0x490) {
      FUN_0043ed39(local_114,(byte *)"%s\\lf.chm::/Syntax.htm#VarOrder");
      FUN_0043e36f(DAT_00452aac,local_114,0,0);
      return 1;
    }
  }
  return 0;
}
