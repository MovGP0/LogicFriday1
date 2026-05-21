/* 0041a18a FUN_0041a18a */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void FUN_0041a18a(HWND param_1,int param_2)

{
  uint unaff_retaddr;
  uint local_40c [257];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 1) {
    FUN_0043ed39((char *)local_40c,(byte *)"Variable names must have between 1 and %d characters.");
  }
  else if (param_2 == 2) {
    FUN_0043ebd0(local_40c,(uint *)"Variable name already in use.");
  }
  else if (param_2 == 3) {
    FUN_0043ebd0(local_40c,
                 (uint *)
                 "Variable names may have only letters, digits, underscores, periods,\nand brackets. The name must begin with a letter or underscore."
                );
  }
  MessageBoxA(param_1,(LPCSTR)local_40c,"Logic Friday",0);
  return;
}
