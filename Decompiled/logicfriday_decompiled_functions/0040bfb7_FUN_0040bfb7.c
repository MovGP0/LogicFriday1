/* 0040bfb7 FUN_0040bfb7 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_0040bfb7(HWND param_1,int param_2,uint param_3,int param_4)

{
  int iVar1;
  uint unaff_retaddr;
  char local_4c [68];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 2) {
    FUN_00401445();
  }
  else {
    if (param_2 == 0x2b) {
      FUN_0040133c(param_1,param_3,param_4);
      return 1;
    }
    if (param_2 == 0x110) {
      FUN_0043ed39(local_4c,(byte *)"Logic Friday v. %d.%d.%d");
      SetDlgItemTextA(param_1,0x47d,local_4c);
      FUN_0040117c(param_1);
      return 1;
    }
    if (param_2 == 0x111) {
      if (((param_3 & 0xffff) == 1) || ((param_3 & 0xffff) == 2)) {
        EndDialog(param_1,param_3 & 0xffff);
        return 1;
      }
      if (((param_3 & 0xffff) == 0x493) || ((param_3 & 0xffff) == 0x494)) {
        iVar1 = FUN_004013e5(param_1,param_3 & 0xffff);
        if (iVar1 == 0) {
          if ((param_3 & 0xffff) == 0x493) {
            MessageBoxA(DAT_00452aac,"Could not find a registered email application.","Logic Friday"
                        ,0);
          }
          else {
            MessageBoxA(DAT_00452aac,"Could not find a registered web browser.","Logic Friday",0);
          }
        }
        return 1;
      }
    }
  }
  return 0;
}
